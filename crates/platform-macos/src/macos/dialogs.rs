//! The AppKit open/save panels (card 7.A3): `NSOpenPanel` /
//! `NSSavePanel` — the macOS file-dialog seam's implementation,
//! driven through the Objective-C runtime (no `objc2` crate).
//!
//! # Runtime model (the honest part)
//!
//! - **Main-thread guard** (`pthread_main_np`): AppKit panels are
//!   main-thread-only UI. A request from any other thread gets the
//!   typed refusal naming the rule — never a cross-thread call that
//!   would be undefined AppKit behavior.
//! - Every dialog round runs inside its own
//!   [`AutoreleasePool`]: the panels, the bridged strings/URLs and
//!   the filter array are autoreleased objects whose lifetime ends
//!   with the round (the seam is a synchronous, scoped request).
//! - `runModal` is the OS's own modal loop — the calling (main)
//!   thread blocks in AppKit until the user decides. Cancel
//!   (`NSModalResponseCancel`) is a first-class
//!   [`FilePick::Cancelled`], never an error.
//! - File-type filters map `*.ext` patterns to
//!   `setAllowedFileTypes:` extension strings. That API is
//!   soft-deprecated in favor of UTTypes on modern macOS; it still
//!   functions and is the only shape reachable without the
//!   UniformTypeIdentifiers bridging surface — `[PROPOSAL]`: move to
//!   `setAllowedContentTypes:` when a Lead-ratified path exists.
//!
//! **Evidence status:** compile-verified via `cargo check --target
//! aarch64-apple-darwin` and `x86_64-apple-darwin`; runtime NOT
//! exercised in this sandbox (no macOS SDK) — `[VERIFY]`.

// SAFETY-SCOPED MODULE (doc 01 §2.7 / MP §9): the libobjc/AppKit
// C ABI is the audited OS-service exception this file exercises.
#![allow(unsafe_code)]

use std::path::PathBuf;

use crate::dialog::{FileDialogRequest, FileDialogs, FilePick};
use crate::error::PlatformError;
use crate::macos::objc::{
    AutoreleasePool, ObjcId, class, msg_send_i64, msg_send_id, msg_send_id_id, msg_send_id_ptr_u64,
    msg_send_id_u64, msg_send_u64, msg_send_void_bool, msg_send_void_id, nsstring_from_utf8,
    nsstring_to_string, release, sel,
};

/// `NSModalResponseOK`.
const NS_MODAL_RESPONSE_OK: i64 = 1;
/// `NSModalResponseCancel`.
const NS_MODAL_RESPONSE_CANCEL: i64 = 0;
/// Objective-C `YES` / `NO`.
const YES: u8 = 1;
const NO: u8 = 0;

// SAFETY: pthread_main_np(3) — the documented libSystem thread
// query. No arguments; reads only the calling thread's identity.
#[link(name = "System", kind = "dylib")]
unsafe extern "C" {
    fn pthread_main_np() -> i32;
}

/// Whether the calling thread is the process main thread.
fn is_main_thread() -> bool {
    // SAFETY: no arguments, no preconditions; the return value is a
    // plain integer flag.
    unsafe { pthread_main_np() != 0 }
}

// ---------------------------------------------------------------------------
// The dialog seam implementation
// ---------------------------------------------------------------------------

/// The AppKit panels adapter: stateless — every round is scoped to
/// its own pool and its own modal loop.
#[derive(Debug, Clone, Default)]
pub struct AppKitPanels;

impl AppKitPanels {
    /// A fresh adapter.
    #[must_use = "constructs the adapter"]
    pub fn new() -> AppKitPanels {
        AppKitPanels
    }

    /// The non-intrusive probe: the panel classes exist in the
    /// linked runtime. Verifies AppKit is present without showing
    /// any UI.
    ///
    /// # Errors
    /// Typed [`PlatformError`] when the classes are absent (the
    /// dialog integration then reports `Unavailable` honestly).
    pub fn probe() -> Result<AppKitPanels, PlatformError> {
        let _ = class("NSOpenPanel")?;
        let _ = class("NSSavePanel")?;
        Ok(AppKitPanels)
    }
}

impl FileDialogs for AppKitPanels {
    fn open_file(&mut self, request: &FileDialogRequest) -> Result<FilePick, PlatformError> {
        run_panel(false, request)
    }

    fn save_file(&mut self, request: &FileDialogRequest) -> Result<FilePick, PlatformError> {
        run_panel(true, request)
    }
}

/// One complete panel round (must run on the main thread).
fn run_panel(save: bool, request: &FileDialogRequest) -> Result<FilePick, PlatformError> {
    if !is_main_thread() {
        return Err(PlatformError::CallFailed {
            api: if save {
                "NSSavePanel runModal"
            } else {
                "NSOpenPanel runModal"
            },
            code: None,
            detail: "AppKit panels require the main thread (the caller is on another thread)"
                .to_string(),
        });
    }
    let _pool = AutoreleasePool::new()?;
    let panel = create_panel(save)?;
    configure_panel(panel, save, request)?;
    // The modal loop. Cancel is a first-class outcome, not an error.
    // SAFETY: panel is live; runModal takes no arguments and returns
    // the NSModalResponse (NSInteger).
    let response = unsafe { msg_send_i64(panel, sel("runModal")?) };
    if response == NS_MODAL_RESPONSE_CANCEL {
        return Ok(FilePick::Cancelled);
    }
    if response != NS_MODAL_RESPONSE_OK {
        return Err(PlatformError::CallFailed {
            api: "NSOpenPanel runModal",
            code: Some(response),
            detail: "the panel round ended with an unexpected response".to_string(),
        });
    }
    collect(panel, save, request)
}

/// Create the shared panel object for the round (`+openPanel` /
/// `+savePanel` — autoreleased, owned by the pool).
fn create_panel(save: bool) -> Result<ObjcId, PlatformError> {
    let cls = class(if save { "NSSavePanel" } else { "NSOpenPanel" })?;
    let factory = sel(if save { "savePanel" } else { "openPanel" })?;
    // SAFETY: cls is the live panel class; the factory method takes
    // no arguments and returns the autoreleased panel (alive for the
    // pool's scope — exactly the round).
    let panel = unsafe { msg_send_id(cls, factory) };
    if panel.is_null() {
        return Err(PlatformError::CallFailed {
            api: if save {
                "NSSavePanel savePanel"
            } else {
                "NSOpenPanel openPanel"
            },
            code: None,
            detail: "the panel factory returned no panel".to_string(),
        });
    }
    Ok(panel)
}

/// Apply the request's shape to the panel (title, directory,
/// filters, multi-select). Applicable setters only — an absent
/// option keeps the panel's own default (the honest OS behavior).
fn configure_panel(
    panel: ObjcId,
    save: bool,
    request: &FileDialogRequest,
) -> Result<(), PlatformError> {
    if let Some(title) = request.title.as_ref() {
        let title_string = nsstring_from_utf8(title)?;
        // SAFETY: panel is live; the title string is alive (pool
        // scope); setTitle: takes it and returns void.
        unsafe { msg_send_void_id(panel, sel("setTitle:")?, title_string) };
    }
    if let Some(dir) = request.default_dir.as_ref() {
        let dir_string = nsstring_from_utf8(&dir.display().to_string())?;
        let url_class = class("NSURL")?;
        // SAFETY: the NSURL class is live; fileURLWithPath: takes the
        // NSString (alive, pool scope) and returns an autoreleased
        // NSURL (alive for the pool's scope).
        let url = unsafe { msg_send_id_id(url_class, sel("fileURLWithPath:")?, dir_string) };
        if !url.is_null() {
            // SAFETY: panel and url are live; setDirectoryURL: takes
            // the URL and returns void.
            unsafe { msg_send_void_id(panel, sel("setDirectoryURL:")?, url) };
        }
        // An unparsable suggested directory is not an error: the
        // panel simply starts at its own default location.
    }
    if !save {
        // Open panels can choose files and/or directories; the seam
        // asks for files, optionally several.
        // SAFETY: panel is live; the BOOL argument is passed by
        // value per the pinned alias.
        unsafe { msg_send_void_bool(panel, sel("setCanChooseFiles:")?, YES) };
        // SAFETY: as above.
        unsafe { msg_send_void_bool(panel, sel("setCanChooseDirectories:")?, NO) };
        // SAFETY: as above.
        unsafe {
            msg_send_void_bool(
                panel,
                sel("setAllowsMultipleSelection:")?,
                if request.multiple { YES } else { NO },
            )
        };
    }
    apply_filters(panel, request)
}

/// Translate the request's `*.ext` patterns into
/// `setAllowedFileTypes:` (an NSArray of extension strings). A
/// pattern without a simple trailing extension (e.g. `*` or `a*b`)
/// disables filtering for the round — the honest reading of "any
/// file"; the label column is the UI's own concern (AppKit shows
/// extensions only).
fn apply_filters(panel: ObjcId, request: &FileDialogRequest) -> Result<(), PlatformError> {
    let extensions: Vec<String> = request
        .filters
        .iter()
        .flat_map(|filter| filter.patterns.iter().map(String::as_str))
        .filter_map(|pattern| pattern.strip_prefix("*."))
        .filter(|ext| !ext.is_empty() && !ext.contains('*'))
        .map(str::to_string)
        .collect();
    if extensions.is_empty() {
        // No expressible extension: leave the panel unfiltered (the
        // request asked for patterns AppKit cannot express — all
        // files is the honest superset, and the Linux lane's
        // zenity fallback documents the same reading).
        return Ok(());
    }
    let strings: Vec<ObjcId> = extensions
        .iter()
        .map(|ext| nsstring_from_utf8(ext))
        .collect::<Result<_, _>>()?;
    let array_class = class("NSArray")?;
    // SAFETY: the NSArray class is live; alloc returns an owned,
    // uninitialized instance.
    let allocated = unsafe { msg_send_id(array_class, sel("alloc")?) };
    if allocated.is_null() {
        return Err(PlatformError::CallFailed {
            api: "NSArray alloc",
            code: None,
            detail: "the runtime refused the array allocation".to_string(),
        });
    }
    // SAFETY: allocated is live and owned by this call;
    // initWithObjects:count: reads `strings.len()` live NSString
    // pointers (pool scope) and returns the initialized, still-owned
    // array.
    let array = unsafe {
        msg_send_id_ptr_u64(
            allocated,
            sel("initWithObjects:count:")?,
            strings.as_ptr(),
            strings.len() as u64,
        )
    };
    if array.is_null() {
        // SAFETY: alloc succeeded but init failed — release the
        // un-initialized memory before refusing.
        unsafe {
            release(allocated);
        }
        return Err(PlatformError::CallFailed {
            api: "NSArray initWithObjects:count:",
            code: None,
            detail: "the filter array could not be initialized".to_string(),
        });
    }
    // SAFETY: panel and the owned array are live; the
    // setAllowedFileTypes: call copies what it needs.
    unsafe { msg_send_void_id(panel, sel("setAllowedFileTypes:")?, array) };
    // SAFETY: the single release of the array this function owns
    // (alloc + init), after its last use above.
    unsafe {
        release(array);
    }
    Ok(())
}

/// Collect the round's outcome: `URL` for single picks and save
/// rounds, `URLs` for multi-select open rounds.
fn collect(
    panel: ObjcId,
    save: bool,
    request: &FileDialogRequest,
) -> Result<FilePick, PlatformError> {
    if save || !request.multiple {
        // SAFETY: panel is live and the round completed OK; URL
        // takes no arguments and returns the chosen NSURL (or NULL).
        let url = unsafe { msg_send_id(panel, sel("URL")?) };
        if url.is_null() {
            return Err(PlatformError::CallFailed {
                api: "NSSavePanel URL",
                code: None,
                detail: "the panel confirmed a pick but returned no URL".to_string(),
            });
        }
        return Ok(FilePick::Paths(vec![url_path(url)?]));
    }
    // SAFETY: panel is a live NSOpenPanel (the save branch returned)
    // whose round completed OK; URLs returns the autoreleased
    // NSArray of picks.
    let urls = unsafe { msg_send_id(panel, sel("URLs")?) };
    if urls.is_null() {
        return Err(PlatformError::CallFailed {
            api: "NSOpenPanel URLs",
            code: None,
            detail: "the panel confirmed a pick but returned no URLs".to_string(),
        });
    }
    // SAFETY: urls is live (pool scope); count is the documented
    // accessor.
    let count = unsafe { msg_send_u64(urls, sel("count")?) };
    if count == 0 {
        return Err(PlatformError::CallFailed {
            api: "NSOpenPanel URLs",
            code: None,
            detail: "the panel confirmed a pick but the selection was empty".to_string(),
        });
    }
    let at_sel = sel("objectAtIndex:")?;
    let mut paths = Vec::new();
    for index in 0..count {
        // SAFETY: urls is live and index < count (just read).
        let url = unsafe { msg_send_id_u64(urls, at_sel, index) };
        if url.is_null() {
            continue; // a vanishing item is not fatal to the rest
        }
        paths.push(url_path(url)?);
    }
    if paths.is_empty() {
        // OK with an unreadable selection is still a completed
        // round; reporting "no paths" would be a lie about what the
        // OS said (the Windows-lane precedent).
        return Err(PlatformError::CallFailed {
            api: "NSOpenPanel URLs",
            code: None,
            detail: "the selection existed but no item could be read".to_string(),
        });
    }
    Ok(FilePick::Paths(paths))
}

/// `NSURL.path` → [`PathBuf`].
fn url_path(url: ObjcId) -> Result<PathBuf, PlatformError> {
    // SAFETY: url is a live NSURL; path takes no arguments and
    // returns the NSString (alive for the pool's scope — copied out
    // below within the round).
    let path_string = unsafe { msg_send_id(url, sel("path")?) };
    if path_string.is_null() {
        return Err(PlatformError::CallFailed {
            api: "NSURL path",
            code: None,
            detail: "the chosen URL has no filesystem path".to_string(),
        });
    }
    // SAFETY: the NSString is live for this call (pool scope).
    let path = unsafe { nsstring_to_string(path_string)? };
    if path.is_empty() {
        return Err(PlatformError::CallFailed {
            api: "NSURL path",
            code: None,
            detail: "the chosen URL's path is empty".to_string(),
        });
    }
    Ok(PathBuf::from(path))
}
