//! Capability negotiation: what this crate can actually do, honestly
//! (card 7.D1; MP §13 "no invented crate capabilities").
//!
//! The shape deliberately mirrors the wave-22 `platform-*` crates'
//! capability tables ([`Feature`]/[`Capability`]/[`DiscoveryReport`]
//! versus their `Feature`/`Capability`/`PlatformReport`]) so the
//! Lead's recorded seam-unification [PROPOSAL] collapses the two
//! vocabularies mechanically. A [`Capability::Unavailable`] is a
//! *real* implementation of graceful degradation: the lane says what
//! is missing and why; it never fakes success and never fabricates
//! events.

use crate::error::UnavailableReason;

/// The per-feature availability verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capability {
    /// The integration is compiled in and its runtime dependencies
    /// were found. Individual calls may still fail with typed errors
    /// (a search can time out honestly).
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

    /// Map a typed probe error into the capability verdict the probe
    /// that produced it implies.
    #[must_use = "pure conversion from probe outcomes"]
    pub fn from_probe_error(err: &crate::error::DiscoveryError) -> Capability {
        Capability::Unavailable(UnavailableReason::from_error(err))
    }
}

/// The addressable discovery features (doc 01 §2.4 lanes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feature {
    /// Enumerating local volumes / removable media.
    VolumeEnumeration,
    /// Volume add/remove notifications (the watcher lane).
    VolumeNotifications,
    /// Network service discovery (SSDP/UPnP; DLNA rides the same
    /// exchange — see [`crate::ssdp`]'s verdict).
    NetworkServiceDiscovery,
}

impl Feature {
    /// The stable dotted key (feature-matrix vocabulary).
    #[must_use = "pure accessor"]
    pub fn key(self) -> &'static str {
        match self {
            Feature::VolumeEnumeration => "discovery.volumes.enumeration",
            Feature::VolumeNotifications => "discovery.volumes.notifications",
            Feature::NetworkServiceDiscovery => "discovery.network.services",
        }
    }
}

/// The per-feature capability table a UI or engine can query before
/// attempting a call.
#[derive(Debug, Clone, Default)]
pub struct DiscoveryReport {
    entries: Vec<(Feature, Capability)>,
}

impl DiscoveryReport {
    /// An empty report (features default to absent — callers add what
    /// they probed).
    pub fn new() -> DiscoveryReport {
        DiscoveryReport::default()
    }

    /// Record one feature's verdict (replaces an existing row — the
    /// report is a table, not a log).
    pub fn set(&mut self, feature: Feature, capability: Capability) {
        for (f, c) in &mut self.entries {
            if *f == feature {
                *c = capability;
                return;
            }
        }
        self.entries.push((feature, capability));
    }

    /// One feature's verdict; unprobed features are honestly
    /// unavailable ("not probed"), never assumed available.
    #[must_use = "pure lookup"]
    pub fn get(&self, feature: Feature) -> Capability {
        for (f, c) in &self.entries {
            if *f == feature {
                return c.clone();
            }
        }
        Capability::Unavailable(UnavailableReason::NotYetImplemented {
            detail: "feature was not probed by this assembly",
        })
    }

    /// Iterate the recorded rows.
    pub fn iter(&self) -> impl Iterator<Item = (Feature, &Capability)> {
        self.entries.iter().map(|(f, c)| (*f, c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unprobed_features_are_honestly_unavailable() {
        let report = DiscoveryReport::new();
        let verdict = report.get(Feature::NetworkServiceDiscovery);
        assert!(!verdict.is_available());
        match verdict {
            Capability::Unavailable(UnavailableReason::NotYetImplemented { detail }) => {
                assert!(detail.contains("not probed"), "{detail}");
            }
            other => unreachable!("unprobed is NotYetImplemented, got {other:?}"),
        }
    }

    #[test]
    fn set_replaces_in_place_not_appends() {
        let mut report = DiscoveryReport::new();
        report.set(
            Feature::VolumeEnumeration,
            Capability::Unavailable(UnavailableReason::NotYetImplemented { detail: "first" }),
        );
        report.set(Feature::VolumeEnumeration, Capability::Available);
        assert_eq!(report.iter().count(), 1);
        assert!(report.get(Feature::VolumeEnumeration).is_available());
    }

    #[test]
    fn feature_keys_are_stable_dotted_ids() {
        assert_eq!(
            Feature::VolumeEnumeration.key(),
            "discovery.volumes.enumeration"
        );
        assert_eq!(
            Feature::VolumeNotifications.key(),
            "discovery.volumes.notifications"
        );
        assert_eq!(
            Feature::NetworkServiceDiscovery.key(),
            "discovery.network.services"
        );
    }
}
