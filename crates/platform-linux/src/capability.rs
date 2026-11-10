//! Capability negotiation: what this build can actually do, honestly
//! (card 7.A2; MP §13 "no invented crate capabilities").
//!
//! A [`Capability::Unavailable`] is a *real* implementation of
//! graceful degradation — the adapter says what is missing and why;
//! it never fakes success. The report is computed from live probes
//! (library presence, service reachability), not from compile-time
//! assumptions alone.

use crate::error::PlatformError;

/// The per-feature availability verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capability {
    /// The integration is compiled in and its runtime dependencies
    /// were found. Individual calls may still fail with typed errors
    /// (e.g. the server went away between probe and call).
    Available,
    /// The integration cannot work here. The reason is typed, never a
    /// bare string guess.
    Unavailable(UnavailableReason),
}

impl Capability {
    /// `true` only for [`Capability::Available`].
    #[must_use = "pure classification"]
    pub fn is_available(&self) -> bool {
        matches!(self, Capability::Available)
    }

    /// Map a typed error into the capability verdict the probe that
    /// produced it implies.
    #[must_use = "pure conversion from probe outcomes"]
    pub fn from_probe_error(err: &PlatformError) -> Capability {
        Capability::Unavailable(UnavailableReason::from_error(err))
    }
}

/// Why a capability is unavailable — the typed core of honest
/// degradation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnavailableReason {
    /// The OS integration this crate carries is not for the running
    /// OS (e.g. `platform-linux` on a macOS host).
    WrongOs {
        /// The OS the real integration targets.
        crate_os: &'static str,
    },
    /// A runtime library the integration needs is not installed
    /// (names + loader diagnosis).
    LibraryNotFound {
        /// The candidate names that were tried.
        names: Vec<String>,
        /// The loader's diagnosis.
        detail: String,
    },
    /// The backing OS service is not reachable (no server / no bus /
    /// no device nodes).
    ServiceNotReachable {
        /// The service name.
        service: &'static str,
        /// Why it was unreachable.
        detail: String,
    },
    /// Not implemented in this card — an honest gap with a recorded
    /// follow-up, never a silent stub that pretends.
    NotYetImplemented {
        /// What is missing and where it is tracked.
        detail: &'static str,
    },
}

impl UnavailableReason {
    /// Collapse a probe error into the reason class it proves.
    #[must_use = "pure conversion"]
    pub fn from_error(err: &PlatformError) -> UnavailableReason {
        match err {
            PlatformError::NotCompiledForThisOs { crate_os } => {
                UnavailableReason::WrongOs { crate_os }
            }
            PlatformError::LibraryUnavailable { names, detail } => {
                UnavailableReason::LibraryNotFound {
                    names: names.clone(),
                    detail: detail.clone(),
                }
            }
            PlatformError::ServiceUnavailable { service, detail } => {
                UnavailableReason::ServiceNotReachable {
                    service,
                    detail: detail.clone(),
                }
            }
            // A missing symbol is a version-skew form of "library not
            // usable"; keep the symbol visible in the detail.
            PlatformError::SymbolMissing { library, symbol } => {
                UnavailableReason::LibraryNotFound {
                    names: vec![library.clone()],
                    detail: format!("required symbol {symbol} is missing (version skew?)"),
                }
            }
            other => UnavailableReason::ServiceNotReachable {
                service: "platform integration",
                detail: other.to_string(),
            },
        }
    }
}

impl std::fmt::Display for UnavailableReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnavailableReason::WrongOs { crate_os } => {
                write!(f, "this adapter carries the {crate_os} integration only")
            }
            UnavailableReason::LibraryNotFound { names, detail } => write!(
                f,
                "runtime library not found (tried {}): {}",
                names.join(", "),
                detail
            ),
            UnavailableReason::ServiceNotReachable { service, detail } => {
                write!(f, "{service} not reachable: {detail}")
            }
            UnavailableReason::NotYetImplemented { detail } => write!(f, "{detail}"),
        }
    }
}

/// The addressable platform features (doc 01 §2.7 lanes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feature {
    /// Audio device enumeration.
    AudioEnumeration,
    /// Audio device change notifications (hotplug).
    AudioNotifications,
    /// GPU adapter enumeration.
    GpuEnumeration,
    /// File open/save dialogs.
    FileDialogs,
}

impl Feature {
    /// The stable dotted key (feature-matrix vocabulary).
    #[must_use = "pure accessor"]
    pub fn key(self) -> &'static str {
        match self {
            Feature::AudioEnumeration => "platform.audio.enumeration",
            Feature::AudioNotifications => "platform.audio.notifications",
            Feature::GpuEnumeration => "platform.gpu.enumeration",
            Feature::FileDialogs => "platform.dialogs",
        }
    }
}

/// The per-feature capability table a UI or engine can query before
/// attempting a call.
#[derive(Debug, Clone, Default)]
pub struct PlatformReport {
    entries: Vec<(Feature, Capability)>,
}

impl PlatformReport {
    /// An empty report (features default to absent — callers add what
    /// they probed).
    pub fn new() -> PlatformReport {
        PlatformReport::default()
    }

    /// Record one feature's verdict.
    pub fn set(&mut self, feature: Feature, capability: Capability) {
        // Replace in place if the feature already has a row — the
        // report is a table, not a log.
        for (f, c) in &mut self.entries {
            if *f == feature {
                *c = capability;
                return;
            }
        }
        self.entries.push((feature, capability));
    }

    /// One feature's verdict; unprobed features are honestly
    /// `NotYetImplemented` ("not probed"), never assumed available.
    #[must_use = "pure lookup"]
    pub fn get(&self, feature: Feature) -> Capability {
        for (f, c) in &self.entries {
            if *f == feature {
                return c.clone();
            }
        }
        Capability::Unavailable(UnavailableReason::NotYetImplemented {
            detail: "feature was not probed by this adapter",
        })
    }

    /// Iterate the recorded rows.
    pub fn iter(&self) -> impl Iterator<Item = (Feature, &Capability)> {
        self.entries.iter().map(|(f, c)| (*f, c))
    }
}
