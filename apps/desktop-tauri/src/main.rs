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

fn main() {
    if let Err(error) = lumen_desktop_tauri::app::run() {
        // lumen-allow-literal: argv0/diagnostic notice — the honest failure face; exit 1 (never a fake window, 2.C3's rule). The marker must sit directly above the flagged line: the lint reads adjacency, and a wrapped two-line comment breaks it (C1, run 43400610890).
        eprintln!("lumen-tauri: admission shell failed: {error:#}");
        std::process::exit(1);
    }
}
