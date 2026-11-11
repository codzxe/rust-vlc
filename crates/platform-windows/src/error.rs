//! The typed platform-error vocabulary (card 7.A1).
//!
//! Every failure a platform seam can produce is one of these variants
//! — never a bare string, never a fake success (MP §8: "errors are
//! data, and the error path is a first-class outcome"). The same
//! shape is mirrored in `platform-windows`/`platform-macos` so the
//! Lead can later unify them into a shared L5 vocabulary crate
//! `[PROPOSAL]`.

use std::fmt;

/// Why a platform call failed. `Display` renders the human form; the
/// enum is the machine-checkable truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformError {
    /// This build does not contain the OS integration (e.g. the Linux
    /// adapter running on a non-Linux host).
    NotCompiledForThisOs {
        /// The OS this crate's real integration targets.
        crate_os: &'static str,
    },
    /// A required runtime library could not be dlopen'd / linked.
    LibraryUnavailable {
        /// The candidate names that were tried.
        names: Vec<String>,
        /// The loader's diagnosis (dlerror text, errno, …).
        detail: String,
    },
    /// A required symbol is missing from an otherwise-loadable library
    /// (version skew, stripped build).
    SymbolMissing {
        /// The library the symbol was expected in.
        library: String,
        /// The symbol name.
        symbol: &'static str,
    },
    /// The OS service exists but refused the connection (no server, no
    /// session bus, device node absent).
    ServiceUnavailable {
        /// The service name (e.g. "PulseAudio server").
        service: &'static str,
        /// Why it was unreachable.
        detail: String,
    },
    /// A call into an OS API returned a failure code.
    CallFailed {
        /// The API/function that failed.
        api: &'static str,
        /// The raw OS status code, when the API is numeric.
        code: Option<i64>,
        /// Context for the failure.
        detail: String,
    },
    /// The operation exceeded its bounded deadline (engineering
    /// directive D-2: no unbounded waits on foreign event loops).
    TimedOut {
        /// The API that was waited on.
        api: &'static str,
        /// The deadline that expired, in milliseconds.
        deadline_ms: u64,
    },
    /// The OS returned a string that is not valid UTF-8/UTF-16.
    InvalidEncoding {
        /// The API that produced the string.
        api: &'static str,
    },
    /// A helper program the seam shells out to could not be executed.
    SpawnFailed {
        /// The program that was attempted.
        program: String,
        /// Why the spawn failed.
        detail: String,
    },
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlatformError::NotCompiledForThisOs { crate_os } => write!(
                f,
                "this build carries no {crate_os} integration (compiled for another OS)"
            ),
            PlatformError::LibraryUnavailable { names, detail } => write!(
                f,
                "required runtime library unavailable (tried {}): {detail}",
                names.join(", ")
            ),
            PlatformError::SymbolMissing { library, symbol } => {
                write!(f, "symbol {symbol:?} missing from {library}")
            }
            PlatformError::ServiceUnavailable { service, detail } => {
                write!(f, "{service} unavailable: {detail}")
            }
            PlatformError::CallFailed { api, code, detail } => match code {
                Some(code) => write!(f, "{api} failed with status {code}: {detail}"),
                None => write!(f, "{api} failed: {detail}"),
            },
            PlatformError::TimedOut { api, deadline_ms } => {
                write!(
                    f,
                    "{api} did not answer within the {deadline_ms} ms deadline"
                )
            }
            PlatformError::InvalidEncoding { api } => {
                write!(f, "{api} returned a string that is not valid Unicode")
            }
            PlatformError::SpawnFailed { program, detail } => {
                write!(f, "could not execute {program}: {detail}")
            }
        }
    }
}

impl std::error::Error for PlatformError {}
