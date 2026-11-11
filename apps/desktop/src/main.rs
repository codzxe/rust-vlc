//! The `lumen-desktop` binary — the honest skeleton face (card 2.D3).
//!
//! The composition root's real `main` (product-config → registry →
//! `EngineSession` → Tauri IPC bridge, doc 01 §2.7 b) joins with the
//! Phase-4 packaging card (4.H1). Until ADR-004 ratifies the
//! webview import, this binary reports the typed pending state and
//! the ADR-003 landing, then exits [`WEBVIEW_PENDING`] — the 2.C3
//! `UnsupportedBackend` precedent: enumerate nothing, never fake a
//! window, never block a script waiting on one.
//!
//! Exit codes: `0` never (there is no window to serve); [`WEBVIEW_PENDING`]
//! (86) is the documented "host layer not compiled" state.

use std::process::ExitCode;

use desktop::{DesktopHost, TargetOs, WebviewHostKind};
use desktop::{WindowTitle, parse_window_table};

/// The documented exit code for "the webview host layer is not
/// compiled into this build" (ADR-004 pending).
pub const WEBVIEW_PENDING: u8 = 86;

fn main() -> ExitCode {
    println!("lumen-desktop (skeleton, card 2.D3)"); // lumen-allow-literal: argv0/skeleton notice — identity wiring pending (see apps/cli help.rs [PROPOSAL])
    println!();
    // Issue #3 (4.H1 handoff c): the [window] table is CONSUMED at
    // startup — the build spec the ADR-004 wrapper applies verbatim at
    // window build (inner/min sizes, maximized, geometry policy, idle
    // title key). A parse failure is loud (the honest-parse rule:
    // config drift never invents a window).
    match parse_window_table(include_str!("../../../product.toml")) {
        Ok(config) => {
            let spec = config.build_spec();
            println!(
                "window build spec (product.toml [window]): {}x{} (min {}x{}){} — title {}",
                spec.inner_size.0,
                spec.inner_size.1,
                spec.min_inner_size.0,
                spec.min_inner_size.1,
                if spec.maximized { ", maximized" } else { "" },
                match &spec.title {
                    WindowTitle::Idle { key } => format!("idle key {key}"),
                    WindowTitle::Media { rendered } => {
                        format!("media {rendered:?}")
                    }
                },
            );
            if spec.remember_geometry {
                println!(
                    "window geometry: remembered across runs (the persisted-geometry store joins with the settings-backed persistence card)"
                );
            }
        }
        Err(error) => {
            eprintln!("window config: {error}");
            return ExitCode::from(WEBVIEW_PENDING);
        }
    }
    println!();
    println!(
        "presentation path: {}",
        DesktopHost::active_presentation_path()
    );
    println!("webview hosts:");
    for host in [
        WebviewHostKind::TauriChannel,
        WebviewHostKind::CustomProtocol,
        WebviewHostKind::SharedMemory,
    ] {
        println!("  {:<16} {}", host_label(host), host.status());
    }
    println!("ADR-003 per-OS landing (accepted; per-OS confirmation pending prototypes):");
    for os in [TargetOs::Windows, TargetOs::Macos, TargetOs::Linux] {
        let report = desktop::strategy_for(os);
        println!(
            "  {:<8} {:<28} via {} — {}",
            os_name(report.os),
            report.variant.label(),
            report.os.webview(),
            report.status
        );
    }
    println!();
    println!("frame protocol: v1 (LUF1) — see docs/prototypes/video-in-webview.md");
    eprintln!("no webview host compiled: ADR-004 pending (records/tauri-wry-2.x.md)");
    ExitCode::from(WEBVIEW_PENDING)
}

/// Stable lowercase names for the report lines (tiny helper — the
/// `TargetOs` Debug form is not a stable surface).
fn os_name(os: TargetOs) -> &'static str {
    match os {
        TargetOs::Windows => "windows",
        TargetOs::Macos => "macos",
        TargetOs::Linux => "linux",
    }
}

/// Stable labels for the webview-host kinds (same reasoning).
fn host_label(host: WebviewHostKind) -> &'static str {
    match host {
        WebviewHostKind::TauriChannel => "tauri-ipc",
        WebviewHostKind::CustomProtocol => "lumen-proto", // lumen-allow-literal: custom-protocol scheme label — protocol vocabulary, not display identity
        WebviewHostKind::SharedMemory => "shared-mem",
    }
}
