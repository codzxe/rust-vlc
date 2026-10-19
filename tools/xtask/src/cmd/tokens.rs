//! `xtask tokens` — theme-token pipeline (doc 08 §3).
//!
//! Owned by card **1.B4** — completed by the Lead after agent 1.B4-R's
//! context deadline (salvage per doc 13 §1.6; bridge wiring by Lead).
//!
//! Subprocess bridge (Lead-ratified [PROPOSAL], first recorded in
//! `crates/theme-tokens/Cargo.toml`): `tools/xtask` cannot take a path
//! dependency on `crates/theme-tokens` without editing its frozen
//! manifest, so this subcommand shells out to the crate's `tokens-gen`
//! binary, which reads `themes/tokens.json` and (re)generates the two
//! committed outputs:
//!
//! * `themes/generated/tokens.css`            — CSS custom properties
//! * `crates/theme-tokens/src/generated/tokens.rs` — Rust constants
//!
//! Inherited stdio keeps the generator's actionable diagnostics visible;
//! the generator's exit status is forwarded verbatim (a failed
//! regeneration must fail the gate).

use std::path::Path;
use std::process::Command;
use std::process::ExitCode;

use crate::XtaskError;

/// Workspace root, derived from this crate's manifest location
/// (`<root>/tools/xtask`). Compile-time constant: the xtask binary only
/// ever runs from inside the workspace it was built by.
const WORKSPACE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// The `cargo` driver that invoked us, when available (`cargo xtask`),
/// falling back to a plain `cargo` lookup on PATH.
fn cargo_driver() -> String {
    match std::env::var("CARGO") {
        Ok(path) => path,
        Err(_) => String::from("cargo"),
    }
}

/// Workspace root at runtime: `<root>/tools/xtask` minus two components.
fn workspace_root() -> &'static Path {
    Path::new(WORKSPACE_ROOT)
        .parent()
        .and_then(Path::parent)
        .unwrap_or(Path::new(WORKSPACE_ROOT))
}

/// Run the `tokens-gen` binary through cargo and forward its status.
///
/// `--quiet` suppresses cargo's own compilation noise so the only output
/// is the generator's report; compilation errors still surface verbatim.
pub fn run(_args: &[String]) -> Result<ExitCode, XtaskError> {
    let program = cargo_driver();
    let mut child = Command::new(&program)
        .args([
            "run",
            "--quiet",
            "-p",
            "theme-tokens",
            "--bin",
            "tokens-gen",
        ])
        .current_dir(workspace_root())
        .spawn()
        .map_err(|source| XtaskError::Io {
            context: format!("spawning `{program} run -p theme-tokens --bin tokens-gen`"),
            source,
        })?;

    let status = child.wait().map_err(|source| XtaskError::Io {
        context: String::from("waiting for `tokens-gen` to finish"),
        source,
    })?;

    match status.code() {
        Some(code) => Ok(ExitCode::from(u8::try_from(code).unwrap_or(2))),
        // Terminated by a signal: report failure rather than success.
        None => Ok(ExitCode::from(2)),
    }
}
