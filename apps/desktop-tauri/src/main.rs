//! The ADR-004 admission binary (issue #67). This crate is
//! workspace-EXCLUDED (root Cargo.toml `exclude`) — it only compiles on
//! webview-equipped hosts, in its dedicated CI lanes:
//! webkit2gtk on Linux, WebView2 on Windows, WKWebView on macOS.

fn main() {
    if let Err(error) = lumen_desktop_tauri::app::run() {
        // lumen-allow-literal: argv0/diagnostic notice — the honest
        // failure face; exit 1 (never a fake window, 2.C3's rule).
        eprintln!("lumen-tauri: admission shell failed: {error:#}");
        std::process::exit(1);
    }
}
