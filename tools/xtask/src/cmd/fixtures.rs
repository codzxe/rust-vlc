//! `xtask fixtures` — deterministic demo-fixture generation (doc 03 §7/§8).
//!
//! Owned by card **2.A3** — do not edit outside that card.
//! Registered by the Lead (frozen-registry ownership grant, worklog 1.F1-V);
//! this stub only reports that the owning card has not landed yet.

use std::process::ExitCode;

use crate::XtaskError;

pub fn run(_args: &[String]) -> Result<ExitCode, XtaskError> {
    Err(XtaskError::NotYetImplemented {
        subcommand: "fixtures",
        card: "2.A3",
    })
}
