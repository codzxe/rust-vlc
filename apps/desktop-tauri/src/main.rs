//! The ADR-004 admission binary (issue #67). This crate is
//! workspace-EXCLUDED (root Cargo.toml `exclude`) — it only compiles on
//! webview-equipped hosts, in its dedicated CI lanes:
//! webkit2gtk on Linux, WebView2 on Windows, WKWebView on macOS.

// Windows subsystem (the "normal app" rule, w28 fix 1): a Rust binary
// defaults to the CONSOLE subsystem, so double-clicking the release exe
// allocated a terminal window next to the GUI — the user-facing bug
// this attribute removes. In RELEASE builds the exe is a pure GUI
// binary: no console is ever allocated. Debug builds KEEP the console
// (cfg_attr), so `cargo run` still shows the honest stderr face below
// — and even in release, stderr/stdout handles provided by a parent
// (the CI launch lane's Start-Process -RedirectStandardError, a shell
// pipe) are inherited and written normally: the subsystem bit only
// stops WINDOWS from auto-allocating a console, it never detaches
// explicit handles. When no console exists (Explorer double-click),
// Rust std maps the null stderr handle to a silent sink — the
// eprintln! below is a no-op there, not a panic.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// The visible failure face (w30.6, audit E §A.2/§F.2 —
// tmp/audit/E-desktop-tauri.md): the subsystem rule above made every
// startup failure INVISIBLE on a double-click launch — an eprintln!
// into a null sink plus exit(1) is exactly the v0.1.2 complaint shape
// ("brief loading then nothing"). main() now leaves THREE faces on
// every failure, in this order:
//   1. the crash log (desktop::diagnostics, std-only): product
//      version, platform, stage, the full error chain and a UTC
//      timestamp under <OS tmp>/lumen/lumen-tauri-crash.log — the
//      panic hook writes the same artifact, so exit-101 panics are no
//      quieter than startup errors;
//   2. the stderr receipt below (the CI greps stay
//      exact-compatible) plus the log's path, so redirected-handle
//      runs (the launch smokes' pipes) get a richer, still-grep-able
//      receipt;
//   3. a native MessageBox — Windows ONLY, and only when BOTH other
//      sinks are provably dead (no console attached AND no real
//      stderr handle: the true Explorer-double-click case). CI and
//      console launches already have faces 1+2 and never block on a
//      modal dialog.

use std::error::Error;
use std::path::Path;

fn main() {
    // First thing, before anything can fail: the process-global panic
    // hook — a panic now leaves the same crash log a startup error
    // does, then the wrapped (std) hook keeps the classic stderr face.
    desktop::diagnostics::install_panic_hook();
    if let Err(error) = lumen_desktop_tauri::app::run() {
        // Face 1: the diagnosable artifact, before anything else can
        // go wrong on this path.
        let log = desktop::diagnostics::write_crash_log("startup", &error);
        // Face 2: the stderr receipt. Line 1 is byte-identical to the
        // pre-w30.6 face (tauri-launch.yml and release smokes grep
        // "admission shell failed" — richer must not mean renamed);
        // line 2 names the log so a captured stderr is self-sufficient.
        // lumen-allow-literal: argv0/diagnostic notice — the honest failure face; exit 1 (never a fake window, 2.C3's rule). The marker must sit directly above the flagged line: the lint reads adjacency, and a wrapped two-line comment breaks it (C1, run 43400610890).
        eprintln!("lumen-tauri: admission shell failed: {error:#}");
        // lumen-allow-literal: argv0/diagnostic notice — the crash-log path receipt; same argv0 prefix family, so CI receipts and support triage share one vocabulary
        eprintln!("lumen-tauri: crash log: {}", log.display());
        // Face 3: the GUI dialog, gated inside on "nothing else can
        // show this failure" (never blocks CI/console/redirected runs).
        fatal_dialog(&error, &log);
        std::process::exit(1);
    }
}

/// Compose the face-3 dialog text and hand it to the platform sink:
/// the product name, the one-line error chain, the crash-log path —
/// and, for webview-class failures, the actionable WebView2 hint (the
/// top Windows early-exit candidate, audit E §A.4 #1). The text is
/// built on every platform; only the sink is Windows-conditional.
fn fatal_dialog(error: &dyn Error, log: &Path) {
    let chain = format!("{error:#}");
    let mut text = String::new();
    // lumen-allow-literal: dialog copy — the one human sentence of the GUI failure face; this excluded std-only crate has no config-layer edge to read [identity] from, so the display name is annotated per the C1 discipline
    text.push_str("Lumen could not start.\n\n");
    text.push_str(&chain);
    text.push_str("\n\nDetails: ");
    text.push_str(&log.display().to_string());
    if is_webview_class(&chain) {
        text.push_str("\n\nThe Microsoft Edge WebView2 Runtime may be missing. Install it from:\n");
        // lumen-allow-literal: the Microsoft Edge WebView2 Evergreen bootstrapper link — Microsoft's shipped product URL (go.microsoft.com/fwlink, not a test double), the actionable hint audit E §F.3 asks for; a first-run failure dialog is not a config surface
        text.push_str("https://go.microsoft.com/fwlink/p/?LinkId=2124703");
    }
    // lumen-allow-literal: dialog caption — the failing app's window title; a display surface is exactly where the identity belongs (same C1 annotation rationale as the copy line above)
    show_message_box(&text, "Lumen");
}

/// Heuristic webview-class detection for the hint: the WebView2/wry
/// creation failures name the webview in their error chain. A false
/// positive only shows an unneeded install hint; a miss loses nothing
/// — the crash log carries the full chain either way.
fn is_webview_class(chain: &str) -> bool {
    chain.to_ascii_lowercase().contains("webview")
}

/// Face 3's Windows sink: the native modal dialog, gated on "no other
/// face can display this" (the gate + rationale live in `win32`).
#[cfg(windows)]
fn show_message_box(text: &str, caption: &str) {
    win32::show_message_box_when_no_other_face(text, caption);
}

/// Face 3's non-Windows sink: a deliberate no-op. Faces 1+2 have
/// already run; a GTK/OS-native dialog would drag a toolkit
/// dependency into this std-only lane (out of w30.6's scope).
#[cfg(not(windows))]
fn show_message_box(_text: &str, _caption: &str) {}

/// The Windows native sink: user32/kernel32 FFI, std-only — the same
/// hand-rolled posture boundaries.toml pins for the L5 adapters
/// (`extern "system"` declarations, NO -sys crates). The crate-level
/// `unsafe_code = "deny"` stands everywhere else; it is allowed on
/// THIS module only, with the invariant per block.
#[cfg(windows)]
#[allow(unsafe_code)]
mod win32 {
    use std::ffi::c_void;

    // Invariants: signatures mirror the SDK headers exactly
    // (MessageBoxW takes LPCWSTR, not LPSTR; HWND is a handle-sized
    // pointer); nothing here frees, closes or writes through a handle.
    unsafe extern "system" {
        /// The native modal message dialog.
        fn MessageBoxW(hwnd: *mut c_void, text: *const u16, caption: *const u16, utype: u32)
        -> i32;
        /// Non-null iff a console is attached to this process.
        fn GetConsoleWindow() -> *mut c_void;
        /// The process's standard-handle slot, or null when the
        /// parent passed none.
        fn GetStdHandle(nstdhandle: u32) -> *mut c_void;
        /// 0 (FILE_TYPE_UNKNOWN) when the handle is no real sink.
        fn GetFileType(hfile: *mut c_void) -> u32;
    }

    /// SDK constants (the values winapi would export; no crate here).
    const STD_ERROR_HANDLE: u32 = 0xFFFF_FFF4; // SDK: (DWORD)-12
    const FILE_TYPE_UNKNOWN: u32 = 0;
    const MB_ICONERROR: u32 = 0x0000_0010;
    const MB_OK: u32 = 0x0000_0000;
    const MB_SETFOREGROUND: u32 = 0x0001_0000;

    /// `text` as a NUL-terminated UTF-16 buffer (MessageBoxW's
    /// LPCWSTR; encode_utf16 is the std half of the no-winapi rule).
    fn utf16(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// The gated dialog: shows ONLY when BOTH provable sinks are dead
    /// — no console attached AND no real stderr handle — which is
    /// exactly the Explorer-double-click case the w28 subsystem rule
    /// created. A console-parent launch (debug `cargo run`, a shell)
    /// and a REDIRECTED launch (the CI smokes'
    /// Start-Process -RedirectStandardError pipes) both skip the
    /// dialog and keep the stderr face: a modal box there would block
    /// the smoke's HasExited poll and fake a "survived" verdict.
    /// GetConsoleWindow alone cannot detect the CI case — a
    /// GUI-subsystem child of a redirecting parent has no console
    /// either; the stderr-handle probe is what proves the pipe.
    pub fn show_message_box_when_no_other_face(text: &str, caption: &str) {
        if console_attached() || stderr_is_live() {
            return;
        }
        let wide_text = utf16(text);
        let wide_caption = utf16(caption);
        // SAFETY: both pointers are NUL-terminated UTF-16 buffers that
        // outlive the call; the null owner HWND means "no owner
        // window". The return value (which button was pressed) is
        // irrelevant — the process exits right after.
        let _pressed = unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                wide_text.as_ptr(),
                wide_caption.as_ptr(),
                MB_ICONERROR | MB_OK | MB_SETFOREGROUND,
            )
        };
    }

    /// A console is attached to this process.
    fn console_attached() -> bool {
        // SAFETY: argument-less handle query, no invariants.
        let console = unsafe { GetConsoleWindow() };
        !console.is_null()
    }

    /// The stderr handle points at a real sink (console, pipe or
    /// file) — i.e. face 2 above has somewhere to land.
    fn stderr_is_live() -> bool {
        // SAFETY: reads the process's own handle slot; SDK constant.
        let handle = unsafe { GetStdHandle(STD_ERROR_HANDLE) };
        if handle.is_null() || handle as isize == -1 {
            return false; // no inherited handle: the null sink
        }
        // SAFETY: a read-only probe of a handle we never invalidate.
        let kind = unsafe { GetFileType(handle) };
        kind != FILE_TYPE_UNKNOWN
    }
}
