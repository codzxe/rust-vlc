//! Lumen workspace task runner — binary entry point (card 1.A3, xtask v0).
//!
//! # FROZEN INTERFACE — read before editing
//!
//! `main.rs` and `cmd/mod.rs` are **FROZEN after card 1.A3**. The subcommand
//! table below is the extension-first dispatch surface: every subcommand that
//! later wave-1 cards need (`schema-gen` 1.B2, `tokens` 1.B4, `codegen` 1.D4,
//! `check-architecture` 1.E1, `rebrand` 1.E4) is registered NOW and dispatched
//! to its own module `src/cmd/<name>.rs`. A later card owns ONLY that one
//! module file; it must not edit this file, `cmd/mod.rs`, or another card's
//! module. Adding a NEW subcommand after the freeze requires Lead-granted
//! temporary ownership of both files (AGENTS.md §3.7).
//!
//! # Exit codes (stable contract — CI and scripts depend on these)
//!
//! * `0` — success (for `gate`: every executed command passed).
//! * `1` — `gate` ran to a verdict and at least one gate command failed
//!   (doc 10 §2.4 `result: FAILURE`).
//! * `2` — xtask error: unknown subcommand, bad usage, unimplemented stub,
//!   or internal failure. Distinct from `1` so callers can tell "the tree is
//!   broken" from "the tool is broken".
//!
//! # Lint policy
//!
//! Workspace lints (doc 10 §3.1) deny `unwrap`/`expect`/`panic!`/`todo!`/
//! `unimplemented!`/`dbg!` — this crate propagates typed errors instead.

use std::fmt;
use std::process::ExitCode;

mod cmd;

/// Typed error for every xtask subcommand. No panics, no unwraps.
#[derive(Debug)]
pub enum XtaskError {
    /// Bad command-line usage (unknown subcommand/flag). Exit code 2.
    Usage(String),
    /// Pre-registered subcommand whose owning card has not landed yet.
    /// Exit code 2.
    NotYetImplemented {
        /// Subcommand name as typed by the user.
        subcommand: &'static str,
        /// Owning card in `docs/13-agent-wave-program.md`.
        card: &'static str,
    },
    /// Internal failure (workspace root not found, report unwritable, ...).
    /// Exit code 2.
    Internal(String),
    /// An I/O failure with the operation that caused it. Exit code 2.
    Io {
        /// What we were doing when the error occurred.
        context: String,
        /// The underlying error.
        source: std::io::Error,
    },
}

impl XtaskError {
    /// Build an [`XtaskError::Internal`] from any string-like message.
    pub fn internal(msg: impl Into<String>) -> Self {
        XtaskError::Internal(msg.into())
    }

    /// Attach a what-was-happening context to an I/O error.
    pub fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        XtaskError::Io {
            context: context.into(),
            source,
        }
    }
}

impl fmt::Display for XtaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XtaskError::Usage(msg) => write!(f, "{msg}"),
            XtaskError::NotYetImplemented { subcommand, card } => {
                write!(
                    f,
                    "subcommand `{subcommand}` is not yet implemented (card {card})"
                )
            }
            XtaskError::Internal(msg) => write!(f, "internal error: {msg}"),
            XtaskError::Io { context, source } => write!(f, "{context}: {source}"),
        }
    }
}

impl std::error::Error for XtaskError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            XtaskError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match dispatch(&args) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("xtask error: {err}");
            ExitCode::from(2)
        }
    }
}

/// Parse argv and route it to the registered subcommand.
fn dispatch(args: &[String]) -> Result<ExitCode, XtaskError> {
    let Some(first) = args.first() else {
        eprint_usage();
        return Err(XtaskError::Usage(
            "no subcommand given — try `cargo xtask help`".to_string(),
        ));
    };
    match first.as_str() {
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(ExitCode::SUCCESS)
        }
        name => match cmd::lookup(name) {
            Some(sub) => (sub.run)(&args[1..]),
            None => Err(XtaskError::Usage(format!(
                "unknown subcommand `{name}` — known subcommands: {}",
                cmd::names().join(", ")
            ))),
        },
    }
}

/// Print the subcommand table to stdout (`help`).
fn print_usage() {
    println!("Lumen workspace task runner (xtask v0, card 1.A3)");
    println!();
    println!("Usage: cargo xtask <subcommand> [args]");
    println!();
    println!("Subcommands:");
    for sub in cmd::SUBCOMMANDS {
        let state = if sub.implemented {
            "implemented"
        } else {
            "not yet implemented"
        };
        println!(
            "  {:<20} {} [card {} — {state}]",
            sub.name, sub.summary, sub.card
        );
    }
    println!();
    println!("Exit codes: 0 success | 1 gate FAILURE (doc 10 §2.4) | 2 xtask error.");
}

/// Print the subcommand table to stderr (invocation mistake).
fn eprint_usage() {
    eprintln!("Usage: cargo xtask <subcommand> [args] — try `cargo xtask help`");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_yet_implemented_message_names_subcommand_and_card() {
        let err = XtaskError::NotYetImplemented {
            subcommand: "schema-gen",
            card: "1.B2",
        };
        let msg = err.to_string();
        assert_eq!(
            msg,
            "subcommand `schema-gen` is not yet implemented (card 1.B2)"
        );
    }

    #[test]
    fn usage_error_renders_message() {
        let err = XtaskError::Usage("no subcommand given".to_string());
        assert_eq!(err.to_string(), "no subcommand given");
    }

    #[test]
    fn io_error_includes_context_and_source() {
        let err = XtaskError::io(
            "writing gate report",
            std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"),
        );
        let msg = err.to_string();
        assert_eq!(msg, "writing gate report: no such file");
    }
}
