//! `xtask codegen` — command-catalog codegen (doc 05 §2-3, §5.1).
//!
//! Owned by card **1.D4**. Registered by card 1.A3 (extension-first
//! dispatch); implemented by 1.D4 (the stub that lived here until then
//! only reported that the owning card had not landed).
//!
//! Subprocess bridge (the Lead-ratified [PROPOSAL] pattern first recorded
//! by card 1.B4 / theme-tokens, mirrored from `schema_gen.rs`):
//! `tools/xtask` cannot take a path dependency on `crates/commands`
//! without editing its frozen manifest, so this subcommand shells out to
//! the crate's `catalog-gen` binary, which derives the machine
//! description of the whole catalog from the wire types (schemars) and
//! (re)generates the three committed outputs:
//!
//! * `schemas/commands.schema.json`            — the JSON Schema (§5.1a)
//! * `ui/src/generated/commands.ts`            — the TS types (§5.1b)
//! * `schemas/generated/commands.checksum.txt` — the catalog checksum
//!
//! Extra args are forwarded verbatim (e.g. `cargo xtask codegen --check`
//! for the CI drift mode; the generator's exit codes are 0 ok / 1 drift /
//! 2 error). Inherited stdio keeps the generator's actionable diagnostics
//! visible; its exit status is forwarded as-is (a failed regeneration
//! must fail the gate).

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

/// Run the `catalog-gen` binary through cargo and forward its status.
///
/// `--quiet` suppresses cargo's own compilation noise so the only output
/// is the generator's report; compilation errors still surface verbatim.
pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
    let program = cargo_driver();
    let mut command = Command::new(&program);
    command
        .args([
            "run",
            "--quiet",
            "-p",
            "commands",
            "--bin",
            "catalog-gen",
            "--",
        ])
        .args(args)
        .current_dir(workspace_root());
    let mut child = command.spawn().map_err(|source| XtaskError::Io {
        context: format!("spawning `{program} run -p commands --bin catalog-gen`"),
        source,
    })?;

    let status = child.wait().map_err(|source| XtaskError::Io {
        context: String::from("waiting for `catalog-gen` to finish"),
        source,
    })?;

    match status.code() {
        Some(code) => Ok(ExitCode::from(u8::try_from(code).unwrap_or(2))),
        // Terminated by a signal: report failure rather than success.
        None => Ok(ExitCode::from(2)),
    }
}
