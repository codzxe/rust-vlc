//! The cpal adapter seam: **ADR-005's admission flip point** (wave 14,
//! issue #1). This module is where the real cpal-backed backend will be
//! constructed when (and only when) the Lead admits it — mirroring the
//! streaming TLS seam (`HTTPS_ADR011_ADMITTED`,
//! `crates/streaming/src/schemes.rs`, ADR-011), whose discipline this
//! file copies exactly:
//!
//! - a single named const is the gate ([`AUDIO_ADR005_ADMITTED`]);
//! - the not-admitted state is a **typed marker**
//!   ([`BackendNotAdmitted`]), never a panic, never a fake success;
//! - a compile-time assertion makes the gate impossible to open by
//!   accident — flipping the const without editing this file breaks
//!   the build;
//! - the admission change is **one change**: flip the const, land the
//!   `platform-*` adapter behind [`cpal_backend`], amend ADR-005, add
//!   the allowlist row, delete the gate + refusal arm + marker.
//!
//! The evidence base for that decision:
//! `docs/dependencies/records/audio-output-cpal.md` (the wave-14
//! admission spike: real `cargo tree` graphs for all three desktop
//! targets, the C-FFI unsafe counts, the sandbox build failure) on top
//! of `records/cpal-0.18.2.md` (the card-2.C3 P0 record). Both verdicts:
//! **conditional-E2, NOT admitted.**

use std::fmt;

use crate::device::AudioBackend;

/// The ADR-005 admission flip point (module docs — the
/// `HTTPS_ADR011_ADMITTED` pattern).
///
/// `false` means: no OS audio library is imported anywhere in the
/// workspace; the only backends are the null backend and the typed
/// not-supported stubs.
///
/// TODO(admitted-impl): the Lead's admission change flips this to
/// `true` **in the same change** that (a) amends ADR-005 with the
/// substrate choice + the doc 11 §1.2 E2 safety review, (b) adds the
/// dependency-record allowlist row at a pinned version, and (c) lands
/// the real `platform-*` cpal adapter behind [`cpal_backend`] —
/// deleting the compile-time gate below, the refusal arm, and the
/// [`BackendNotAdmitted`] marker with it.
pub const AUDIO_ADR005_ADMITTED: bool = false;

/// Compile-time flip-point discipline: the gate cannot silently open.
/// Deleting this assertion without landing the adapter (and the ADR
/// amendment) is the accident it exists to prevent.
const _: () = assert!(
    !AUDIO_ADR005_ADMITTED,
    "AUDIO_ADR005_ADMITTED may only flip together with the admission change \
     (ADR-005 amendment + platform-* cpal adapter + allowlist row)"
);

/// Typed marker: the named backend's admission is pending its ADR
/// (Lead-gated — AGENTS §3.2/§3.3; dependency admission is never an
/// implementation agent's call). Returned by [`cpal_backend`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendNotAdmitted {
    /// The substrate awaiting admission (e.g. `"cpal"`).
    pub backend: &'static str,
    /// The admission vehicle (e.g. `"ADR-005"`).
    pub adr: &'static str,
}

impl BackendNotAdmitted {
    /// The cpal marker — the substrate the wave-14 spike audited
    /// (`docs/dependencies/records/audio-output-cpal.md`).
    pub const CPAL: BackendNotAdmitted = BackendNotAdmitted {
        backend: "cpal",
        adr: "ADR-005",
    };
}

impl fmt::Display for BackendNotAdmitted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "audio backend `{}` is not admitted: {} is pending Lead sign-off",
            self.backend, self.adr
        )
    }
}

impl std::error::Error for BackendNotAdmitted {}

/// The cpal adapter's construction gate — the single point platform
/// wiring will call to obtain the real backend.
///
/// While [`AUDIO_ADR005_ADMITTED`] is `false` this is the typed
/// refusal marker ([`BackendNotAdmitted::CPAL`]) — exactly the
/// streaming TLS seam's shape: the not-admitted state is a value the
/// caller can match on, log, and surface, never a panic and never a
/// fake success.
///
/// # Errors
/// [`BackendNotAdmitted::CPAL`] while admission is pending (today:
/// always — see the compile-time gate above).
pub fn cpal_backend() -> Result<Box<dyn AudioBackend>, BackendNotAdmitted> {
    if !AUDIO_ADR005_ADMITTED {
        // The refusal arm (the flip point): delete together with the
        // gate and the marker when the admission change lands.
        return Err(BackendNotAdmitted::CPAL);
    }
    // Unreachable while the compile-time gate holds: the admission
    // change replaces this arm with the real adapter's construction.
    unreachable!("the ADR-005 admission change replaces this arm")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gate_is_recorded_closed() {
        // The flip-point discipline: flipping the const without the
        // admission change breaks the compile-time gate (and this
        // test) — the same-change rule, enforced.
        assert!(!AUDIO_ADR005_ADMITTED);
    }

    #[test]
    fn the_constructor_refuses_with_the_typed_marker() {
        let marker = cpal_backend().err().expect("not admitted today");
        assert_eq!(marker, BackendNotAdmitted::CPAL);
        assert_eq!(marker.backend, "cpal");
        assert_eq!(marker.adr, "ADR-005");
        let rendered = marker.to_string();
        assert!(rendered.contains("cpal"), "{rendered}");
        assert!(rendered.contains("ADR-005"), "{rendered}");
        assert!(rendered.contains("Lead"), "{rendered}");
    }
}
