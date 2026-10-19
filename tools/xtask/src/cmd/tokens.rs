//! `xtask tokens` — theme-token pipeline (doc 08 §3).
//!
//! Owned by card **1.B4** — do not edit outside that card.
//! Registered by card 1.A3 (extension-first dispatch); this stub only
//! reports that the owning card has not landed yet.

use std::process::ExitCode;

use crate::XtaskError;

pub fn run(_args: &[String]) -> Result<ExitCode, XtaskError> {
    Err(XtaskError::NotYetImplemented {
        subcommand: "tokens",
        card: "1.B4",
    })
}
