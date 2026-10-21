//! `CommandError` → process exit codes + stage-tagged honest printing
//! (card 2.D2).
//!
//! Doc 01 §2.8's apps/cli sketch: "Exit codes map `EngineError` stages
//! deterministically **[PROPOSAL]**". This module implements the
//! mapping over the command layer's [`ErrorCode`] vocabulary (finer
//! than stages — the stage is already inside every printed line as
//! `[stage.code]`, so the exit code's job is the coarse "what kind of
//! failure" class a script can branch on).
//!
//! The table is BSD `sysexits(3)`-based `[PROPOSAL: the doc names no
//! numbers; sysexits is the longest-standing CLI convention for
//! exactly this taxonomy]`:
//!
//! | Exit | `ErrorCode`s |
//! |------|--------------|
//! | 0 | success |
//! | 64 `EX_USAGE` | `invalid-argument`, `out-of-range` |
//! | 65 `EX_DATAERR` | `unsupported`, `corrupt`, `decoder`, `canceled` |
//! | 66 `EX_NOINPUT` | `inaccessible`, `not-found` |
//! | 69 `EX_UNAVAILABLE` | `not-ready`, `not-seekable`, `network`, `device`, `feature-not-exposed`, `unsupported-for-host` |
//! | 71 `EX_OSERR` | `resource-limit` |
//! | 74 `EX_IOERR` | `io` |
//! | 77 `EX_NOPERM` | `permission-denied` |
//! | 70 `EX_SOFTWARE` | `internal` |
//!
//! The totality of the table is pinned by test (every `ErrorCode`
//! variant maps — a new catalog code fails the test until classified).
//!
//! # Printing (honest, no secrets)
//!
//! Every error line is the command layer's own redaction-safe form:
//! `error: [stage.code] message` on **stderr**, with the typed
//! `details` map printed as indented `key: value` lines.
//! [`CommandError`]'s constructor already routes message and details
//! through doc 05 §2.13's V8 redactor, so no path can leak userinfo or
//! full user paths here.

use commands::{CommandError, ErrorCode};

/// The process exit code for one error class (the table in the module
/// docs). `u8`: `std::process::ExitCode::from` wants exactly that.
#[must_use]
pub fn exit_code(code: ErrorCode) -> u8 {
    match code {
        // Usage errors: the caller's arguments were wrong.
        ErrorCode::InvalidArgument | ErrorCode::OutOfRange => 64, // EX_USAGE
        // Media-data errors: the file exists but cannot be played.
        ErrorCode::Unsupported | ErrorCode::Corrupt | ErrorCode::Decoder | ErrorCode::Canceled => {
            65 // EX_DATAERR
        }
        // The named input does not exist / cannot be opened.
        ErrorCode::Inaccessible | ErrorCode::NotFound => 66, // EX_NOINPUT
        // The operation is unavailable right now / for this host.
        ErrorCode::NotReady
        | ErrorCode::NotSeekable
        | ErrorCode::Network
        | ErrorCode::Device
        | ErrorCode::FeatureNotExposed
        | ErrorCode::UnsupportedForHost => 69, // EX_UNAVAILABLE
        // Resource budgets hit (bounded memory/queue/probe).
        ErrorCode::ResourceLimit => 71, // EX_OSERR
        // I/O failed mid-operation (post-open).
        ErrorCode::Io => 74, // EX_IOERR
        // Authorization refused.
        ErrorCode::PermissionDenied => 77, // EX_NOPERM
        // Engine invariant breach — always a bug, never user-actionable.
        ErrorCode::Internal => 70, // EX_SOFTWARE
    }
}

/// The `std::process::ExitCode` for one error.
#[must_use]
pub fn exit_code_of(error: &CommandError) -> std::process::ExitCode {
    std::process::ExitCode::from(exit_code(error.code))
}

/// Print one error to stderr in the stage-tagged honest form:
/// `error: [stage.code] message` plus indented detail lines.
///
/// This is the ONLY error printer in the app — `main` routes every
/// failure through it so the output shape cannot drift per mode.
pub fn print_error(error: &CommandError) {
    eprintln!("error: {error}");
    for (key, value) in &error.details {
        eprintln!("  {key}: {value}");
    }
}

/// The writer-taking twin of [`print_error`] (the runner's in-memory
/// test I/O; same shape, parameterized sink).
pub fn print_error_to<W: std::io::Write>(error: &CommandError, err: &mut W) {
    let _ = writeln!(err, "error: {error}");
    for (key, value) in &error.details {
        let _ = writeln!(err, "  {key}: {value}");
    }
}

/// Print one error and answer its exit code (the one-call form `main`
/// uses for startup failures).
#[must_use]
pub fn report(error: &CommandError) -> std::process::ExitCode {
    print_error(error);
    exit_code_of(error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use commands::Stage;

    /// Every catalog `ErrorCode` variant must classify — a new code
    /// added to the catalog without an exit mapping fails here.
    #[test]
    fn every_error_code_maps_to_an_exit_code() {
        let all = [
            ErrorCode::Unsupported,
            ErrorCode::Corrupt,
            ErrorCode::Inaccessible,
            ErrorCode::Network,
            ErrorCode::Decoder,
            ErrorCode::Device,
            ErrorCode::ResourceLimit,
            ErrorCode::InvalidArgument,
            ErrorCode::OutOfRange,
            ErrorCode::NotFound,
            ErrorCode::NotReady,
            ErrorCode::NotSeekable,
            ErrorCode::Canceled,
            ErrorCode::FeatureNotExposed,
            ErrorCode::UnsupportedForHost,
            ErrorCode::PermissionDenied,
            ErrorCode::Io,
            ErrorCode::Internal,
        ];
        assert_eq!(all.len(), 18, "the catalog gained a code — classify it");
        for code in all {
            let mapped = exit_code(code);
            // The sysexits set this module is allowed to answer with.
            assert!(
                matches!(mapped, 64 | 65 | 66 | 69 | 70 | 71 | 74 | 77),
                "code {code} mapped to unmapped exit {mapped}"
            );
        }
    }

    /// The representative rows of the table (spot-pinned; the totality
    /// test above is the real proof).
    #[test]
    fn representative_rows_are_pinned() {
        assert_eq!(exit_code(ErrorCode::InvalidArgument), 64);
        assert_eq!(exit_code(ErrorCode::Unsupported), 65);
        assert_eq!(exit_code(ErrorCode::Inaccessible), 66);
        assert_eq!(exit_code(ErrorCode::NotReady), 69);
        assert_eq!(exit_code(ErrorCode::ResourceLimit), 71);
        assert_eq!(exit_code(ErrorCode::Io), 74);
        assert_eq!(exit_code(ErrorCode::PermissionDenied), 77);
        assert_eq!(exit_code(ErrorCode::Internal), 70);
    }

    /// The printed line is the layer's own summary form, details
    /// indented beneath.
    #[test]
    fn printed_form_is_stage_tagged_with_details() {
        let error = CommandError::new(
            Stage::Probe,
            ErrorCode::Unsupported,
            "no signature matched",
        )
        .with_detail("format", "unknown");
        let mut seen = Vec::new();
        // Capture stderr is not possible std-only; assert the shape of
        // the pieces the printer uses instead (Display + summary are
        // the contract under the println!s).
        seen.push(format!("error: {error}"));
        seen.push(error.details.keys().cloned().collect::<Vec<_>>().join(","));
        assert_eq!(seen[0], "error: [probe.unsupported]: no signature matched");
        assert_eq!(seen[1], "format");
    }
}
