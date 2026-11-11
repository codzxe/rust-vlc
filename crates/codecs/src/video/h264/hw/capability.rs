//! Hardware-decode **capability negotiation** (cards 7.B1–7.B3) —
//! the typed layer every per-OS adapter lane plugs into, and the
//! only place the `hw-active` truth is decided (ADR-001: "truthful
//! reporting of when they are active"; R-25).
//!
//! # The three questions this module answers
//!
//! 1. **Is hardware decode present?** — a lane probe yields
//!    [`HwAvailability`] ([`Available`](HwAvailability::Available)
//!    with a device description, or [`Unavailable`](HwAvailability::Unavailable)
//!    with a typed [`HwUnavailableReason`]). Probes are *first*: no
//!    session is ever minted off an unprobed device, and no probe
//!    outcome is ever guessed (a lane that cannot run — wrong OS,
//!    missing device node, missing library, denied permission —
//!    reports exactly that).
//! 2. **What does policy say we should do about it?** —
//!    [`HwDecodePolicy`] mirrors the settings vocabulary
//!    (`playback.hw-decode-policy`, `schemas/settings.toml`:
//!    `software-only` / `prefer-hardware` / `hardware-only`, gated by
//!    the `hw-decode` product feature, shipped default
//!    `software-only` + `features.hw-decode = false` — rule F2).
//!    [`HwDecodePolicy::from_setting`] parses the exact setting
//!    string; anything else is a typed rejection, never a guess.
//! 3. **What actually happened?** — [`negotiate`] returns a
//!    [`Negotiation`] carrying the accelerator (when hardware won)
//!    plus the [`HwDecodeEvent`] trail for the truthful active-
//!    decoder report. The events are the `hw-active` surface the
//!    engine surfaces through its reporting seam
//!    `[VERIFY: engine-side wiring is the registry card's scope]`.
//!
//! # Honesty contract (never fake hardware decode)
//!
//! For the H.264 family there is **no software decoder** (ADR-001
//! §1: the conformance-gated software track is open but empty), so
//! every fallback here lands on
//! [`FallbackTarget::UnsupportedPending`] — the typed
//! `h264.hw.required` configure error of [`super::decode`], never a
//! fabricated frame. [`FallbackTarget::Software`] exists in the
//! vocabulary because the same negotiation will serve the
//! software-first families (MJPEG/AV1/…) when their optional hw
//! lanes land; this module keeps the vocabulary honest for both.

use std::fmt;
use std::sync::Arc;

use codec_api::HwAccelerator;

/// A hardware decode lane's backend identity (the
/// [`HwAccelerator::backend_id`] labels, one per per-OS adapter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HwBackend {
    /// macOS VideoToolbox (`VTDecompressionSession`, lane 7.B1).
    VideoToolbox,
    /// Windows Media Foundation (+ DXVA, lane 7.B2 — not yet
    /// implemented; the label is the negotiated vocabulary's truth
    /// either way).
    MediaFoundation,
    /// Linux VA-API (`vaCreate*`, lane 7.B3 — activation
    /// `[PROPOSAL]` pending ADR-013).
    VaApi,
}

impl HwBackend {
    /// The stable backend label (matches the accelerator's
    /// `backend_id` and the `hw-active` event trail).
    #[must_use = "pure accessor"]
    pub fn backend_id(self) -> &'static str {
        match self {
            HwBackend::VideoToolbox => "hw.videotoolbox",
            HwBackend::MediaFoundation => "hw.mediafoundation",
            HwBackend::VaApi => "hw.vaapi",
        }
    }
}

impl fmt::Display for HwBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.backend_id())
    }
}

/// Why a lane is unavailable — the typed, user-renderable truth a
/// probe failure produces (mirrors the platform crates'
/// `PlatformError` vocabulary: capability negotiation is a
/// first-class outcome, never a guessed success).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HwUnavailableReason {
    /// The lane is not compiled into this binary (wrong OS for the
    /// adapter — e.g. VA-API on a macOS build).
    WrongOs,
    /// The OS exposes no usable device node (Linux: no `/dev/dri`
    /// render or card node exists).
    NoDeviceNode,
    /// The device node exists but this process may not open it
    /// (`EACCES`/`EPERM` — the classic "user not in the `video`/
    /// `render` group" reality).
    PermissionDenied,
    /// The OS service library could not be loaded at runtime (the
    /// loader's diagnosis is carried verbatim).
    LibraryUnavailable(String),
    /// The library loaded and the device opened, but the driver
    /// rejected initialization or does not serve this codec (the
    /// API's own status text is carried verbatim).
    DriverIncompatible(String),
    /// The user policy disabled hardware decode entirely
    /// (`software-only`): not a failure, but the honest reason no
    /// probe ran.
    DisabledByPolicy,
}

impl fmt::Display for HwUnavailableReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HwUnavailableReason::WrongOs => f.write_str("adapter lane not compiled for this OS"),
            HwUnavailableReason::NoDeviceNode => {
                f.write_str("no hardware decode device node is present")
            }
            HwUnavailableReason::PermissionDenied => {
                f.write_str("device node present but permission denied")
            }
            HwUnavailableReason::LibraryUnavailable(d) => {
                write!(f, "OS decode library unavailable: {d}")
            }
            HwUnavailableReason::DriverIncompatible(d) => {
                write!(f, "driver rejected hardware decode: {d}")
            }
            HwUnavailableReason::DisabledByPolicy => {
                f.write_str("disabled by user policy (software-only)")
            }
        }
    }
}

/// One lane's probe outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HwAvailability {
    /// Hardware decode is usable: the backend label plus a
    /// human-renderable device/driver description (the truthful
    /// `hw-active` detail).
    Available {
        /// Which lane answered.
        backend: HwBackend,
        /// Device/driver description as the OS API reported it.
        detail: String,
    },
    /// Not usable, with the typed reason.
    Unavailable {
        /// Which lane was probed.
        backend: HwBackend,
        /// The typed reason (never a bare string).
        reason: HwUnavailableReason,
    },
}

impl HwAvailability {
    /// The lane this outcome belongs to.
    #[must_use = "pure accessor"]
    pub fn backend(&self) -> HwBackend {
        match self {
            HwAvailability::Available { backend, .. } => *backend,
            HwAvailability::Unavailable { backend, .. } => *backend,
        }
    }
}

/// The settings vocabulary (`playback.hw-decode-policy`,
/// `schemas/settings.toml` — shipped default `software-only`, so
/// hardware decode is strictly opt-in; rule F2 keeps
/// `features.hw-decode = false` until the audited integration
/// passes its gate).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HwDecodePolicy {
    /// `software-only` — no hardware attempt at all (the shipped
    /// default; zero behavior change without opt-in).
    #[default]
    SoftwareOnly,
    /// `prefer-hardware` — probe first, fall back when absent.
    PreferHardware,
    /// `hardware-only` — probe first; absence is a typed refusal.
    HardwareOnly,
}

impl HwDecodePolicy {
    /// Parse the exact settings string (`software-only` /
    /// `prefer-hardware` / `hardware-only`).
    ///
    /// # Errors
    /// [`HwPolicyError`] for any other spelling — a misconfigured
    /// policy is reported, never guessed.
    pub fn from_setting(value: &str) -> Result<HwDecodePolicy, HwPolicyError> {
        match value {
            "software-only" => Ok(HwDecodePolicy::SoftwareOnly),
            "prefer-hardware" => Ok(HwDecodePolicy::PreferHardware),
            "hardware-only" => Ok(HwDecodePolicy::HardwareOnly),
            other => Err(HwPolicyError {
                got: other.to_string(),
            }),
        }
    }

    /// The settings string for this policy (round-trips through
    /// [`HwDecodePolicy::from_setting`]).
    #[must_use = "pure accessor"]
    pub fn as_setting(self) -> &'static str {
        match self {
            HwDecodePolicy::SoftwareOnly => "software-only",
            HwDecodePolicy::PreferHardware => "prefer-hardware",
            HwDecodePolicy::HardwareOnly => "hardware-only",
        }
    }

    /// Whether a probe should run under this policy at all (the
    /// `software-only` default performs no device or library
    /// touches — OFF means off).
    #[must_use = "pure predicate"]
    pub fn probes(self) -> bool {
        !matches!(self, HwDecodePolicy::SoftwareOnly)
    }
}

/// A policy string the settings vocabulary does not define.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HwPolicyError {
    /// The rejected spelling.
    pub got: String,
}

impl fmt::Display for HwPolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown hw-decode policy {:?} (want software-only | prefer-hardware | hardware-only)",
            self.got
        )
    }
}

impl std::error::Error for HwPolicyError {}

/// Where control lands when hardware does not serve the stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackTarget {
    /// The codec family's software decoder takes over (the
    /// software-first families; not H.264 today — ADR-001).
    Software,
    /// No software decoder exists for the family (H.264/HEVC under
    /// ADR-001 hw-first): the typed `*.hw.required` `Unsupported`
    /// configure error is the honest outcome — never a fake frame.
    UnsupportedPending,
}

/// One entry of the truthful `hw-active` event trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HwDecodeEvent {
    /// A lane probe ran and produced this outcome (emitted even for
    /// the `software-only` default, where the "probe" is the
    /// policy refusal itself — the report then says why nothing
    /// was touched).
    Probed {
        /// The outcome verbatim.
        availability: HwAvailability,
    },
    /// Hardware decode is ACTIVE: a session was (or will be) opened
    /// off this accelerator (ADR-001's truthful active-decoder
    /// report; the engine's `system.get-engine-info` surface
    /// consumes this `[VERIFY: registry-card wiring]`).
    Active {
        /// Which backend is producing output.
        backend: HwBackend,
        /// The device/driver detail.
        detail: String,
    },
    /// Hardware decode is NOT active, with the reason and where
    /// control landed.
    Inactive {
        /// Why (typed).
        reason: HwUnavailableReason,
        /// Where control landed.
        fallback: FallbackTarget,
    },
}

/// The result of [`negotiate`]: the accelerator when hardware won,
/// plus the event trail for the truthful report.
pub struct Negotiation {
    /// The negotiated accelerator (`None` = hardware not serving;
    /// the codec layer then follows `fallback`).
    pub accelerator: Option<Arc<dyn HwAccelerator>>,
    /// Where control landed when `accelerator` is `None`.
    pub fallback: Option<FallbackTarget>,
    /// The event trail, in order (probe → decision).
    pub events: Vec<HwDecodeEvent>,
}

impl fmt::Debug for Negotiation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The accelerator handle is deliberately opaque (ADR-001:
        // the label is the report, not a pointer) — mirror the
        // H264HwPath Debug convention.
        f.debug_struct("Negotiation")
            .field(
                "accelerator",
                &self.accelerator.as_ref().map(|a| a.backend_id()),
            )
            .field("fallback", &self.fallback)
            .field("events", &self.events)
            .finish()
    }
}

/// Decide, under `policy`, whether `accelerator` (minted by a
/// successful lane probe) serves this stream family.
///
/// Pure: the probe has already happened by the time an accelerator
/// exists; this function applies policy to facts and records the
/// event trail. The H.264 caller passes
/// [`FallbackTarget::UnsupportedPending`] (ADR-001: no software
/// H.264 decoder exists — the honest refusal, never a fake frame).
#[must_use = "the event trail is the hw-active report; use it"]
pub fn negotiate(
    policy: HwDecodePolicy,
    availability: &HwAvailability,
    accelerator: Option<Arc<dyn HwAccelerator>>,
    on_absence: FallbackTarget,
) -> Negotiation {
    let mut events = Vec::with_capacity(2);
    events.push(HwDecodeEvent::Probed {
        availability: availability.clone(),
    });
    match (policy, availability, accelerator) {
        // Policy OFF: never activate hardware, whatever the probe
        // found. The event trail still records the probe outcome —
        // the user asked software, the report says so, and the
        // accelerator is dropped unused.
        (HwDecodePolicy::SoftwareOnly, availability, _) => {
            let reason = if matches!(availability, HwAvailability::Available { .. }) {
                HwUnavailableReason::DisabledByPolicy
            } else {
                unavailable_reason(availability)
            };
            events.push(HwDecodeEvent::Inactive {
                reason,
                fallback: on_absence,
            });
            Negotiation {
                accelerator: None,
                fallback: Some(on_absence),
                events,
            }
        }
        // Hardware usable and policy wants it: ACTIVE.
        (policy, HwAvailability::Available { backend, detail }, Some(accel)) if policy.probes() => {
            events.push(HwDecodeEvent::Active {
                backend: *backend,
                detail: detail.clone(),
            });
            Negotiation {
                accelerator: Some(accel),
                fallback: None,
                events,
            }
        }
        // Usable hardware but the caller could not mint an
        // accelerator off it: an internal lane failure — the
        // honest report is Inactive (typed DriverIncompatible
        // carrying the backend label), never a fake Active.
        (policy, HwAvailability::Available { backend, .. }, None) if policy.probes() => {
            events.push(HwDecodeEvent::Inactive {
                reason: HwUnavailableReason::DriverIncompatible(format!(
                    "probe succeeded for {backend} but no accelerator was minted"
                )),
                fallback: on_absence,
            });
            Negotiation {
                accelerator: None,
                fallback: Some(on_absence),
                events,
            }
        }
        // Not usable: fall back per the family's reality.
        (policy, availability, _) if policy.probes() => {
            events.push(HwDecodeEvent::Inactive {
                reason: unavailable_reason(availability),
                fallback: on_absence,
            });
            Negotiation {
                accelerator: None,
                fallback: Some(on_absence),
                events,
            }
        }
        // Unreachable arm shape (policy that does not probe handled
        // above); kept total rather than panicking.
        _ => Negotiation {
            accelerator: None,
            fallback: Some(on_absence),
            events,
        },
    }
}

/// The typed reason of an `Unavailable` outcome (`DisabledByPolicy`
/// stand-in for available lanes is decided by the caller's arm).
fn unavailable_reason(availability: &HwAvailability) -> HwUnavailableReason {
    match availability {
        HwAvailability::Unavailable { reason, .. } => reason.clone(),
        HwAvailability::Available { .. } => HwUnavailableReason::DisabledByPolicy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_accel() -> Arc<dyn HwAccelerator> {
        struct MockVa;
        impl HwAccelerator for MockVa {
            fn backend_id(&self) -> &str {
                "hw.vaapi"
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
        Arc::new(MockVa)
    }

    // Not a `const`: `Available` carries a heap `String` detail.
    fn va_available() -> HwAvailability {
        HwAvailability::Available {
            backend: HwBackend::VaApi,
            detail: "test driver".into(),
        }
    }
    const VA_NODE_MISSING: HwAvailability = HwAvailability::Unavailable {
        backend: HwBackend::VaApi,
        reason: HwUnavailableReason::NoDeviceNode,
    };

    #[test]
    fn policy_parses_the_settings_vocabulary_exactly() {
        for (s, p) in [
            ("software-only", HwDecodePolicy::SoftwareOnly),
            ("prefer-hardware", HwDecodePolicy::PreferHardware),
            ("hardware-only", HwDecodePolicy::HardwareOnly),
        ] {
            assert_eq!(HwDecodePolicy::from_setting(s), Ok(p), "{s}");
            assert_eq!(p.as_setting(), s, "round-trip");
        }
        for bad in ["", "auto", "prefer-hw", "Software-Only", "hardware"] {
            let err = match HwDecodePolicy::from_setting(bad) {
                Err(e) => e,
                Ok(_) => unreachable!("{bad:?} must be rejected"),
            };
            assert_eq!(err.got.as_str(), bad);
        }
        // Default is OFF: no probes under the shipped default.
        assert_eq!(HwDecodePolicy::default(), HwDecodePolicy::SoftwareOnly);
        assert!(!HwDecodePolicy::default().probes());
        assert!(HwDecodePolicy::PreferHardware.probes());
        assert!(HwDecodePolicy::HardwareOnly.probes());
    }

    #[test]
    fn software_only_never_activates_even_with_hardware_present() {
        let n = negotiate(
            HwDecodePolicy::SoftwareOnly,
            &va_available(),
            Some(mock_accel()),
            FallbackTarget::UnsupportedPending,
        );
        assert!(n.accelerator.is_none(), "policy OFF must not activate");
        assert_eq!(n.fallback, Some(FallbackTarget::UnsupportedPending));
        match &n.events[..] {
            [
                HwDecodeEvent::Probed { .. },
                HwDecodeEvent::Inactive { reason, fallback },
            ] => {
                assert_eq!(*reason, HwUnavailableReason::DisabledByPolicy);
                assert_eq!(*fallback, FallbackTarget::UnsupportedPending);
            }
            other => unreachable!("event trail shape: {other:?}"),
        }
    }

    #[test]
    fn prefer_hardware_activates_when_available() {
        let n = negotiate(
            HwDecodePolicy::PreferHardware,
            &va_available(),
            Some(mock_accel()),
            FallbackTarget::UnsupportedPending,
        );
        assert!(n.accelerator.is_some());
        assert_eq!(n.fallback, None);
        match &n.events[..] {
            [
                HwDecodeEvent::Probed { .. },
                HwDecodeEvent::Active { backend, detail },
            ] => {
                assert_eq!(*backend, HwBackend::VaApi);
                assert_eq!(detail, "test driver");
            }
            other => unreachable!("event trail shape: {other:?}"),
        }
    }

    #[test]
    fn unavailable_falls_back_with_the_typed_reason() {
        for policy in [HwDecodePolicy::PreferHardware, HwDecodePolicy::HardwareOnly] {
            let n = negotiate(
                policy,
                &VA_NODE_MISSING,
                None,
                FallbackTarget::UnsupportedPending,
            );
            assert!(n.accelerator.is_none());
            assert_eq!(n.fallback, Some(FallbackTarget::UnsupportedPending));
            match &n.events[..] {
                [
                    HwDecodeEvent::Probed { availability },
                    HwDecodeEvent::Inactive { reason, .. },
                ] => {
                    assert_eq!(*availability, VA_NODE_MISSING);
                    assert_eq!(*reason, HwUnavailableReason::NoDeviceNode);
                }
                other => unreachable!("event trail shape: {other:?}"),
            }
        }
    }

    #[test]
    fn available_without_an_accelerator_is_an_honest_inactive() {
        let n = negotiate(
            HwDecodePolicy::PreferHardware,
            &va_available(),
            None,
            FallbackTarget::UnsupportedPending,
        );
        assert!(n.accelerator.is_none());
        match n.events[1] {
            HwDecodeEvent::Inactive {
                reason: HwUnavailableReason::DriverIncompatible(_),
                ..
            } => {}
            ref other => unreachable!("must be a typed Inactive, got {other:?}"),
        }
    }

    #[test]
    fn backend_labels_are_stable() {
        assert_eq!(HwBackend::VideoToolbox.backend_id(), "hw.videotoolbox");
        assert_eq!(
            HwBackend::MediaFoundation.backend_id(),
            "hw.mediafoundation"
        );
        assert_eq!(HwBackend::VaApi.backend_id(), "hw.vaapi");
        assert_eq!(HwBackend::VaApi.to_string(), "hw.vaapi");
    }

    #[test]
    fn reasons_render_for_users() {
        let cases = [
            (HwUnavailableReason::WrongOs, "not compiled for this OS"),
            (
                HwUnavailableReason::NoDeviceNode,
                "no hardware decode device node",
            ),
            (HwUnavailableReason::PermissionDenied, "permission denied"),
            (HwUnavailableReason::DisabledByPolicy, "software-only"),
        ];
        for (reason, fragment) in cases {
            let rendered = reason.to_string();
            assert!(rendered.contains(fragment), "{reason:?} -> {rendered}");
        }
        assert!(
            HwUnavailableReason::LibraryUnavailable("libva.so.2: no".into())
                .to_string()
                .contains("libva.so.2: no")
        );
    }
}
