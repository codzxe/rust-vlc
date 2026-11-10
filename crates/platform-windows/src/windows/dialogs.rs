//! The Common Item Dialogs (card 7.A1): `IFileOpenDialog` /
//! `IFileSaveDialog` — the Windows file-dialog seam's implementation,
//! hand-declared over the ole32/shell32 COM ABI (no `windows-s`
//! crate).
//!
//! # Runtime model (the honest part)
//!
//! - Every dialog round runs on a **dedicated short-lived STA
//!   thread** (the same discipline as the WASAPI enumeration): the
//!   apartment, the dialog object and the result extraction live and
//!   die inside one call, so the caller's thread model never matters
//!   and the modal loop cannot strand references on a foreign thread.
//! - The dialog has no owner window (NULL `hwnd`): the OS centers it
//!   on the active monitor. That is the standard shape for a
//!   media-player integration that owns no window of its own yet (the
//!   ADR-004 webview shell owns windows later).
//! - User cancel is `ERROR_CANCELLED` from `Show` — a first-class
//!   [`FilePick::Cancelled`], never an error.
//!
//! **Evidence status:** compile-verified via `cargo check --target
//! x86_64-pc-windows-msvc`; runtime NOT exercised in this sandbox
//! (no Windows SDK) — `[VERIFY]` on real-device behavior.

// SAFETY-SCOPED MODULE (doc 01 §2.7 / MP §9): the ole32/shell32 COM
// ABI is the audited OS-service exception this file exercises.
#![allow(unsafe_code)]

use std::ffi::c_void;
use std::path::PathBuf;
use std::thread;

use crate::dialog::{FileDialogRequest, FileDialogs, FilePick};
use crate::error::PlatformError;
use crate::windows::com::{
    ComApartment, Guid, Hresult, PropertyKey, S_OK, co_create_instance, free_wide, from_wide,
    release_interface, to_wide,
};

// ---------------------------------------------------------------------------
// ABI constants (shobjidl_core.h / exdisp.h)
// ---------------------------------------------------------------------------

/// `CLSID_FileOpenDialog` {DC1C5A9C-E88A-4DDE-A5A1-60F82A20AEF7}.
const CLSID_FILE_OPEN_DIALOG: Guid = Guid {
    data1: 0xDC1C_5A9C,
    data2: 0xE88A,
    data3: 0x4DDE,
    data4: [0xA5, 0xA1, 0x60, 0xF8, 0x2A, 0x20, 0xAE, 0xF7],
};

/// `CLSID_FileSaveDialog` {C0B4E2F3-BA21-4773-8DCA-3357EB2EB9B1}.
const CLSID_FILE_SAVE_DIALOG: Guid = Guid {
    data1: 0xC0B4_E2F3,
    data2: 0xBA21,
    data3: 0x4773,
    data4: [0x8D, 0xCA, 0x33, 0x57, 0xEB, 0x2E, 0xB9, 0xB1],
};

/// `IID_IFileOpenDialog` {D57C7288-D4AD-4768-BE02-9D969532D960}.
const IID_FILE_OPEN_DIALOG: Guid = Guid {
    data1: 0xD57C_7288,
    data2: 0xD4AD,
    data3: 0x4768,
    data4: [0xBE, 0x02, 0x9D, 0x96, 0x95, 0x32, 0xD9, 0x60],
};

/// `IID_IFileSaveDialog` {84BCCD23-5FDE-4CDB-AEA4-AF64B83D78AB}.
const IID_FILE_SAVE_DIALOG: Guid = Guid {
    data1: 0x84BC_CD23,
    data2: 0x5FDE,
    data3: 0x4CDB,
    data4: [0xAE, 0xA4, 0xAF, 0x64, 0xB8, 0x3D, 0x78, 0xAB],
};

/// `ERROR_CANCELLED` (win32 1225) mapped into an `HRESULT`
/// (`HRESULT_FROM_WIN32`) — the user closed the dialog.
const ERROR_CANCELLED: Hresult = 0x8007_04C5u32 as i32;

/// `SIGDN_FILESYSPATH` — `GetDisplayName`'s "full filesystem path"
/// selector (shobjidl_core.h's `SIGDN` value, not a GUID).
const SIGDN_FILESYSPATH: i32 = 0x8005_8000u32 as i32;

/// `FOS_FORCEFILESYSTEM` — only real files.
const FOS_FORCEFILESYSTEM: u32 = 0x40;
/// `FOS_PATHMUSTEXIST` — the chosen path must exist.
const FOS_PATHMUSTEXIST: u32 = 0x800;
/// `FOS_FILEMUSTEXIST` — open rounds only: the file must exist.
const FOS_FILEMUSTEXIST: u32 = 0x1000;
/// `FOS_ALLOWMULTISELECT` — open rounds with `multiple: true`.
const FOS_ALLOWMULTISELECT: u32 = 0x200;
/// `FOS_OVERWRITEPROMPT` — save rounds: confirm overwrites.
const FOS_OVERWRITEPROMPT: u32 = 0x2;

// SAFETY: the declared ABI of shell32's SHCreateItemFromParsingName
// (used to turn a default-dir path into the IShellItem the dialog
// API takes). Invariants honored at the call site: the path is a
// NUL-terminated wide string, the out-slot is zero-initialized and
// the returned interface is released exactly once.
#[link(name = "shell32")]
unsafe extern "system" {
    fn SHCreateItemFromParsingName(
        pszpath: *const u16,
        pbc: *mut c_void,
        riid: *const Guid,
        ppv: *mut *mut c_void,
    ) -> Hresult;
}

/// `IID_IShellItem` {43826D1E-E718-42EE-BC55-A1E261C37BFE}.
const IID_SHELL_ITEM: Guid = Guid {
    data1: 0x4382_6D1E,
    data2: 0xE718,
    data3: 0x42EE,
    data4: [0xBC, 0x55, 0xA1, 0xE2, 0x61, 0xC3, 0x7B, 0xFE],
};

// ---------------------------------------------------------------------------
// Vtables (shobjidl_core.h order; unused slots underscore-prefixed)
// ---------------------------------------------------------------------------

/// One `COMDLG_FILTERSPEC` row (label + pattern string, both wide).
#[repr(C)]
struct FilterSpec {
    /// Human label.
    psz_name: *const u16,
    /// Pattern string ("*.mkv;*.mp4").
    psz_spec: *const u16,
}

/// `IModalWindow`'s slot (the IFileDialog prefix).
type ShowFn = unsafe extern "system" fn(*mut c_void, *mut c_void) -> Hresult;

/// `IFileDialog` vtable: IUnknown + IModalWindow::Show + the 23
/// IFileDialog slots, in shobjidl_core.h order.
#[repr(C)]
struct FileDialogVtbl {
    /// IUnknown.
    pub unknown: crate::windows::com::UnknownVtbl,
    /// `Show(this, hwndOwner)`.
    pub show: ShowFn,
    /// `SetFileTypes(this, count, specs)`.
    pub set_file_types: unsafe extern "system" fn(*mut c_void, u32, *const FilterSpec) -> Hresult,
    /// `SetFileTypeIndex(this, index)`.
    pub _set_file_type_index: unsafe extern "system" fn(*mut c_void, u32) -> Hresult,
    /// `GetFileTypeIndex(this, out)`.
    pub _get_file_type_index: unsafe extern "system" fn(*mut c_void, *mut u32) -> Hresult,
    /// `Advise(this, events, cookie)`.
    pub _advise: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut u32) -> Hresult,
    /// `Unadvise(this, cookie)`.
    pub _unadvise: unsafe extern "system" fn(*mut c_void, u32) -> Hresult,
    /// `SetOptions(this, fos)`.
    pub set_options: unsafe extern "system" fn(*mut c_void, u32) -> Hresult,
    /// `GetOptions(this, out)`.
    pub _get_options: unsafe extern "system" fn(*mut c_void, *mut u32) -> Hresult,
    /// `SetDefaultFolder(this, item)`.
    pub set_default_folder: unsafe extern "system" fn(*mut c_void, *mut c_void) -> Hresult,
    /// `SetFolder(this, item)`.
    pub _set_folder: unsafe extern "system" fn(*mut c_void, *mut c_void) -> Hresult,
    /// `GetFolder(this, out)`.
    pub _get_folder: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
    /// `GetCurrentSelection(this, out)`.
    pub _get_current_selection: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
    /// `SetFileName(this, name)`.
    pub _set_file_name: unsafe extern "system" fn(*mut c_void, *const u16) -> Hresult,
    /// `GetFileName(this, out)`.
    pub _get_file_name: unsafe extern "system" fn(*mut c_void, *mut *mut u16) -> Hresult,
    /// `SetTitle(this, title)`.
    pub set_title: unsafe extern "system" fn(*mut c_void, *const u16) -> Hresult,
    /// `SetOkButtonLabel(this, text)`.
    pub _set_ok_button_label: unsafe extern "system" fn(*mut c_void, *const u16) -> Hresult,
    /// `SetFileNameLabel(this, label)`.
    pub _set_file_name_label: unsafe extern "system" fn(*mut c_void, *const u16) -> Hresult,
    /// `GetResult(this, out)`.
    pub get_result: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
    /// `AddPlace(this, item, fdap)`.
    pub _add_place: unsafe extern "system" fn(*mut c_void, *mut c_void, u32) -> Hresult,
    /// `SetDefaultExtension(this, ext)`.
    pub _set_default_extension: unsafe extern "system" fn(*mut c_void, *const u16) -> Hresult,
    /// `Close(this, hr)`.
    pub _close: unsafe extern "system" fn(*mut c_void, Hresult) -> Hresult,
    /// `SetClientGuid(this, guid)`.
    pub _set_client_guid: unsafe extern "system" fn(*mut c_void, *const Guid) -> Hresult,
    /// `ClearClientData(this)`.
    pub _clear_client_data: unsafe extern "system" fn(*mut c_void) -> Hresult,
    /// `SetFilter(this, filter)`.
    pub _set_filter: unsafe extern "system" fn(*mut c_void, *mut c_void) -> Hresult,
    /// `GetResults(this, out)` — IFileOpenDialog slot 1.
    pub get_results: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
    /// `GetSelectedItems(this, out)` — IFileOpenDialog slot 2.
    pub _get_selected_items: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
}

/// `IShellItem` vtable (shobjidl_core.h order through
/// `GetDisplayName`; the trailing slots are unused).
#[repr(C)]
struct ShellItemVtbl {
    /// IUnknown.
    pub unknown: crate::windows::com::UnknownVtbl,
    /// `BindToHandler(this, …)`.
    pub _bind_to_handler: unsafe extern "system" fn(
        *mut c_void,
        *mut c_void,
        *const Guid,
        *const Guid,
        *mut *mut c_void,
    ) -> Hresult,
    /// `GetParent(this, out)`.
    pub _get_parent: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
    /// `GetDisplayName(this, sigdn, out)`.
    pub get_display_name: unsafe extern "system" fn(*mut c_void, i32, *mut *mut u16) -> Hresult,
    /// `GetAttributes(this, mask, out)`.
    pub _get_attributes: unsafe extern "system" fn(*mut c_void, u32, *mut u32) -> Hresult,
    /// `Compare(this, other, hint, out)`.
    pub _compare: unsafe extern "system" fn(*mut c_void, *mut c_void, i32, *mut i32) -> Hresult,
}

/// `IShellItemArray` vtable (shobjidl_core.h order through
/// `GetItemAt`; the trailing slot is unused).
#[repr(C)]
struct ShellItemArrayVtbl {
    /// IUnknown.
    pub unknown: crate::windows::com::UnknownVtbl,
    /// `BindToHandler(this, …)`.
    pub _bind_to_handler: unsafe extern "system" fn(
        *mut c_void,
        *mut c_void,
        *const Guid,
        *const Guid,
        *mut *mut c_void,
    ) -> Hresult,
    /// `GetPropertyStore(this, …)`.
    pub _get_property_store:
        unsafe extern "system" fn(*mut c_void, u32, *const Guid, *mut *mut c_void) -> Hresult,
    /// `GetPropertyDescriptionList(this, …)`.
    pub _get_property_description_list: unsafe extern "system" fn(
        *mut c_void,
        *const PropertyKey,
        *const Guid,
        *mut *mut c_void,
    ) -> Hresult,
    /// `GetAttributes(this, …)`.
    pub _get_attributes: unsafe extern "system" fn(*mut c_void, u32, u32, *mut u32) -> Hresult,
    /// `GetCount(this, out)`.
    pub get_count: unsafe extern "system" fn(*mut c_void, *mut u32) -> Hresult,
    /// `GetItemAt(this, index, out)`.
    pub get_item_at: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> Hresult,
    /// `EnumItems(this, out)`.
    pub _enum_items: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Hresult,
}

/// A COM object header for vtable access through a raw pointer.
#[repr(C)]
struct ObjectHeader {
    vtbl: *const (),
}

// ---------------------------------------------------------------------------
// The dialog seam implementation
// ---------------------------------------------------------------------------

/// The Common Item Dialogs adapter: stateless — every round runs on
/// its own short-lived STA thread.
#[derive(Debug, Clone, Default)]
pub struct CommonItemDialogs;

impl CommonItemDialogs {
    /// A fresh adapter.
    #[must_use = "constructs the adapter"]
    pub fn new() -> CommonItemDialogs {
        CommonItemDialogs
    }

    /// The non-intrusive probe: create an `IFileOpenDialog` and
    /// release it immediately, on its own STA thread. Verifies the
    /// shell classes are present without showing any UI.
    ///
    /// # Errors
    /// Typed [`PlatformError`] when the class cannot be created (the
    /// dialog integration then reports `Unavailable` honestly).
    pub fn probe() -> Result<CommonItemDialogs, PlatformError> {
        // lumen-allow-literal: diagnostic thread name (debugger/log
        // surface only, not a user-visible string).
        let outcome = spawn_sta("lumen-cid-probe", || {
            // SAFETY: created on this thread inside this closure and
            // released exactly once before it returns; the vtable
            // layout is the documented IFileOpenDialog order.
            let dialog =
                unsafe { co_create_instance(&CLSID_FILE_OPEN_DIALOG, &IID_FILE_OPEN_DIALOG)? };
            // SAFETY: the single release of the probe's reference.
            unsafe {
                release_interface(dialog);
            }
            Ok(())
        })?;
        outcome
    }
}

/// Run `body` on a dedicated short-lived STA thread and join it
/// (the apartment lives and dies inside the thread).
fn spawn_sta<T: Send + 'static>(
    name: &str,
    body: impl FnOnce() -> T + Send + 'static,
) -> Result<T, PlatformError> {
    thread::Builder::new()
        .name(name.to_string())
        .spawn(body)
        .map_err(|err| PlatformError::SpawnFailed {
            program: name.to_string(),
            detail: err.to_string(),
        })?
        .join()
        .map_err(|_| PlatformError::CallFailed {
            api: name,
            code: None,
            detail: "the dialog thread panicked".to_string(),
        })
}

impl FileDialogs for CommonItemDialogs {
    fn open_file(&mut self, request: &FileDialogRequest) -> Result<FilePick, PlatformError> {
        let request = request.clone();
        // lumen-allow-literal: diagnostic thread name (debugger/log
        // surface only, not a user-visible string).
        spawn_sta("lumen-cid-open", move || run_dialog(false, &request))
    }

    fn save_file(&mut self, request: &FileDialogRequest) -> Result<FilePick, PlatformError> {
        let request = request.clone();
        // lumen-allow-literal: diagnostic thread name (debugger/log
        // surface only, not a user-visible string).
        spawn_sta("lumen-cid-save", move || run_dialog(true, &request))
    }
}

/// One complete dialog round (runs on the dedicated STA thread).
fn run_dialog(save: bool, request: &FileDialogRequest) -> Result<FilePick, PlatformError> {
    let _apartment = ComApartment::new()?;
    let (clsid, iid) = if save {
        (&CLSID_FILE_SAVE_DIALOG, &IID_FILE_SAVE_DIALOG)
    } else {
        (&CLSID_FILE_OPEN_DIALOG, &IID_FILE_OPEN_DIALOG)
    };
    // SAFETY: created on this thread, used only here, and released
    // exactly once on every path below (the vtable layout is the
    // documented IFile{Open,Save}Dialog order above).
    let dialog = unsafe { co_create_instance(clsid, iid)? };
    let result = configure_and_show(dialog, save, request);
    // SAFETY: the single release of the created reference.
    unsafe {
        release_interface(dialog);
    }
    result
}

/// Configure, show and collect the results against a live dialog
/// object. Every intermediate COM reference is released on every path.
fn configure_and_show(
    dialog: *mut c_void,
    save: bool,
    request: &FileDialogRequest,
) -> Result<FilePick, PlatformError> {
    let vtbl = unsafe { (*(dialog as *mut ObjectHeader)).vtbl as *const FileDialogVtbl };

    // Options: filesystem items only; existence/overwrite semantics
    // per dialog kind; multi-select for open rounds that asked.
    let mut options = FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST;
    if save {
        options |= FOS_OVERWRITEPROMPT;
    } else {
        options |= FOS_FILEMUSTEXIST;
        if request.multiple {
            options |= FOS_ALLOWMULTISELECT;
        }
    }
    // SAFETY: dialog is live; the options are the documented
    // FOS_* mask for this round.
    let hr = unsafe { ((*vtbl).set_options)(dialog, options) };
    if hr != S_OK {
        return Err(PlatformError::CallFailed {
            api: "IFileDialog::SetOptions",
            code: Some(i64::from(hr)),
            detail: "the dialog refused the option mask".to_string(),
        });
    }

    if let Some(title) = request.title.as_ref() {
        let wide = to_wide(title)?;
        // SAFETY: dialog is live; the wide buffer is NUL-terminated
        // (to_wide) and outlives the call.
        let hr = unsafe { ((*vtbl).set_title)(dialog, wide.as_ptr()) };
        if hr != S_OK {
            return Err(PlatformError::CallFailed {
                api: "IFileDialog::SetTitle",
                code: Some(i64::from(hr)),
                detail: "the dialog refused the title".to_string(),
            });
        }
    }

    if let Some(dir) = request.default_dir.as_ref() {
        let wide = to_wide(&dir.display().to_string())?;
        // SAFETY: the path is NUL-terminated (to_wide), punkouter and
        // the bind context are NULL, the out-slot is zero-initialized
        // and the returned IShellItem is released exactly once below.
        let mut folder: *mut c_void = std::ptr::null_mut();
        let hr = unsafe {
            SHCreateItemFromParsingName(
                wide.as_ptr(),
                std::ptr::null_mut(),
                &IID_SHELL_ITEM,
                &mut folder,
            )
        };
        if hr == S_OK && !folder.is_null() {
            // SAFETY: dialog is live; folder is the freshly created
            // IShellItem (or NULL when the path did not parse — the
            // dialog then simply starts at its own default).
            let hr = unsafe { ((*vtbl).set_default_folder)(dialog, folder) };
            // SAFETY: the single release of the parsed item.
            unsafe {
                release_interface(folder);
            }
            if hr != S_OK {
                return Err(PlatformError::CallFailed {
                    api: "IFileDialog::SetDefaultFolder",
                    code: Some(i64::from(hr)),
                    detail: "the dialog refused the default folder".to_string(),
                });
            }
        }
        // An unparsable default dir is not an error: the OS default
        // location is an honest fallback for a suggestion.
    }

    // Filters: keep every wide buffer alive until the call returns.
    if !request.filters.is_empty() {
        let names: Vec<Vec<u16>> = request
            .filters
            .iter()
            .map(|f| to_wide(&f.label))
            .collect::<Result<_, _>>()?;
        let specs: Vec<Vec<u16>> = request
            .filters
            .iter()
            .map(|f| to_wide(&f.patterns.join(";")))
            .collect::<Result<_, _>>()?;
        let rows: Vec<FilterSpec> = names
            .iter()
            .zip(specs.iter())
            .map(|(name, spec)| FilterSpec {
                psz_name: name.as_ptr(),
                psz_spec: spec.as_ptr(),
            })
            .collect();
        // SAFETY: dialog is live; the rows point at the wide buffers
        // above, which outlive this call (both still in scope).
        let hr = unsafe { ((*vtbl).set_file_types)(dialog, rows.len() as u32, rows.as_ptr()) };
        if hr != S_OK {
            return Err(PlatformError::CallFailed {
                api: "IFileDialog::SetFileTypes",
                code: Some(i64::from(hr)),
                detail: "the dialog refused the filter table".to_string(),
            });
        }
    }

    // Show (modal). Cancel is a first-class outcome, not an error.
    // SAFETY: dialog is live; hwndOwner NULL is the documented
    // "no owner" shape.
    let hr = unsafe { ((*vtbl).show)(dialog, std::ptr::null_mut()) };
    if hr == ERROR_CANCELLED {
        return Ok(FilePick::Cancelled);
    }
    if hr != S_OK {
        return Err(PlatformError::CallFailed {
            api: "IFileDialog::Show",
            code: Some(i64::from(hr)),
            detail: "the dialog round failed".to_string(),
        });
    }

    // Collect. Save rounds (and single-select open rounds) read
    // GetResult; multi-select open rounds read GetResults.
    if save || !request.multiple {
        // SAFETY: dialog is live and the user pressed OK (Show
        // returned S_OK), so GetResult's out-slot is filled; the
        // returned item is released exactly once below.
        let mut item: *mut c_void = std::ptr::null_mut();
        let hr = unsafe { ((*vtbl).get_result)(dialog, &mut item) };
        if hr != S_OK || item.is_null() {
            return Err(PlatformError::CallFailed {
                api: "IFileDialog::GetResult",
                code: Some(i64::from(hr)),
                detail: "the chosen item could not be read".to_string(),
            });
        }
        let path = shell_item_path(item);
        // SAFETY: the single release of the result item.
        unsafe {
            release_interface(item);
        }
        return path.map(|p| FilePick::Paths(vec![p]));
    }
    // SAFETY: dialog is a live IFileOpenDialog (the save branch
    // returned above) whose Show returned S_OK; the out-slot is
    // filled and the array is released exactly once below.
    let mut array: *mut c_void = std::ptr::null_mut();
    let hr = unsafe { ((*vtbl).get_results)(dialog, &mut array) };
    if hr != S_OK || array.is_null() {
        return Err(PlatformError::CallFailed {
            api: "IFileOpenDialog::GetResults",
            code: Some(i64::from(hr)),
            detail: "the chosen items could not be read".to_string(),
        });
    }
    let paths = array_paths(array);
    // SAFETY: the single release of the array reference.
    unsafe {
        release_interface(array);
    }
    paths.map(FilePick::Paths)
}

/// `IShellItem::GetDisplayName(SIGDN_FILESYSPATH)` → [`PathBuf`].
fn shell_item_path(item: *mut c_void) -> Result<PathBuf, PlatformError> {
    let vtbl = unsafe { (*(item as *mut ObjectHeader)).vtbl as *const ShellItemVtbl };
    // SAFETY: item is live; the selector is the documented SIGDN and
    // the returned PWSTR is CoTaskMemAlloc'd by the API and freed
    // below (the only free).
    let mut wide_path: *mut u16 = std::ptr::null_mut();
    let hr = unsafe { ((*vtbl).get_display_name)(item, SIGDN_FILESYSPATH, &mut wide_path) };
    if hr != S_OK || wide_path.is_null() {
        return Err(PlatformError::CallFailed {
            api: "IShellItem::GetDisplayName",
            code: Some(i64::from(hr)),
            detail: "the item's filesystem path could not be read".to_string(),
        });
    }
    // SAFETY: the wide path is NUL-terminated and valid until freed
    // on the next line (from_wide copies out first).
    let path = unsafe { from_wide(wide_path) };
    // SAFETY: the API allocated it with CoTaskMemAlloc; this is the
    // only free.
    unsafe {
        free_wide(wide_path);
    }
    path.map(PathBuf::from)
}

/// Every item of an `IShellItemArray` as a [`PathBuf`].
fn array_paths(array: *mut c_void) -> Result<Vec<PathBuf>, PlatformError> {
    let vtbl = unsafe { (*(array as *mut ObjectHeader)).vtbl as *const ShellItemArrayVtbl };
    // SAFETY: array is live; the count slot is zero-initialized.
    let mut count: u32 = 0;
    let hr = unsafe { ((*vtbl).get_count)(array, &mut count) };
    if hr != S_OK {
        return Err(PlatformError::CallFailed {
            api: "IShellItemArray::GetCount",
            code: Some(i64::from(hr)),
            detail: "the selection count could not be read".to_string(),
        });
    }
    let mut paths = Vec::new();
    for index in 0..count {
        // SAFETY: array is live and index < count (just read); the
        // out-slot is zero-initialized and the item is released
        // exactly once per iteration below.
        let mut item: *mut c_void = std::ptr::null_mut();
        let hr = unsafe { ((*vtbl).get_item_at)(array, index, &mut item) };
        if hr != S_OK || item.is_null() {
            // An item vanishing mid-collection is not fatal to the
            // rest; skip it and keep the honest remainder.
            continue;
        }
        let path = shell_item_path(item);
        // SAFETY: the single release of this item's reference.
        unsafe {
            release_interface(item);
        }
        paths.push(path?);
    }
    if paths.is_empty() {
        // OK with an unreadable selection is still a completed round;
        // reporting "no paths" would be a lie about what the OS said.
        return Err(PlatformError::CallFailed {
            api: "IFileOpenDialog::GetResults",
            code: None,
            detail: "the selection existed but no item could be read".to_string(),
        });
    }
    Ok(paths)
}
