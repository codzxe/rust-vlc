//! The service-discovery host contract (doc 06 §1.2 kind 16):
//! background enumeration → device lists for UI browsing. Domain
//! trait: `device_discovery::ServiceDiscovery` (L2); this minimal
//! host surface is staged here [PROPOSAL: placement, doc 06 §10
//! reconciliation]. Allowed as a WASM extension in v1 (§1.4).
//!
//! Instance scope: app singleton (§1.3). Discovery is
//! **engine-driven polling**: the engine asks for a snapshot and
//! subscribes to change events on its own cadence (hotplug budgets
//! are product.toml limits, doc 06 §5.4's posture) — a plugin never
//! owns a background thread or socket loop. Network protocols a
//! discovery impl speaks (UPnP, mDNS) are manifest capabilities
//! (`discovers`), enforced by the sandbox for extensions.

use std::fmt;

use media_types::{EngineError, PipelineStage};

/// One discovered service/device (doc 06 §1.2 row 16: "Device lists
/// for UI browsing").
///
/// Data, not UI copy: names come from the network response and are
/// localized at render time (doc 08 layering), never here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// Stable id for the browsing session (the plugin's own scheme).
    pub id: String,
    /// Display name as advertised by the service (data, not UI copy).
    pub name: String,
    /// The kind of thing discovered (`upnp-media-server`,
    /// `mdns-airplay`, ...).
    pub kind: String,
    /// Where to reach it (a URI the Access layer can open, e.g.
    /// `http://192.0.2.7:8200/desc.xml`).
    pub uri: String,
}

/// A service-discovery provider (doc 06 §1.2 row 16).
///
/// # Calling context
///
/// [`Send`] + [`Sync`]: app-singleton state shared across the
/// discovery polling task and the UI query path. All methods are
/// bounded and non-blocking: a snapshot is one bounded enumeration
/// pass; long scans chunk across `changes` calls (each returns
/// promptly, `None` = nothing new yet).
pub trait ServiceDiscovery: Send + Sync {
    /// The protocols this provider enumerates (must match the
    /// manifest's `discovers` claim — doc 06 §7.2 capability
    /// honesty).
    fn discovers(&self) -> Vec<&'static str>;

    /// One bounded enumeration pass. Implementations may return a
    /// cached snapshot refreshed within the engine's polling budget;
    /// they must not block on the network beyond it.
    ///
    /// # Errors
    /// [`EngineError`] (stage [`PipelineStage::Discovery`]) — network
    /// causes for protocol failures; absence of devices is
    /// `Ok(vec![])`, not an error (MP §8).
    fn snapshot(&self) -> Result<Vec<Device>, EngineError>;

    /// Drain pending change events since the last call (`None` =
    /// none yet). The engine polls this on its own cadence; the
    /// plugin never pushes.
    ///
    /// # Errors
    /// [`EngineError`] (stage [`PipelineStage::Discovery`]).
    fn changes(&self) -> Result<Option<Vec<DiscoveryChange>>, EngineError>;
}

/// One device-list change since the previous drain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryChange {
    /// A device appeared.
    Added(Device),
    /// A device vanished (timeout, shutdown).
    Removed {
        /// The id that disappeared.
        id: String,
    },
}

/// Typed discovery error: a manifest `discovers` claim the provider
/// does not serve (doc 06 §7.2).
pub fn claim_mismatch(claimed: &str) -> EngineError {
    EngineError::new(
        PipelineStage::Discovery,
        media_types::ErrorKind::Corrupt {
            detail: media_types::error::CorruptDetail::InvalidStructure { offset: None },
        },
        media_types::error::ErrorContext::new("plugin.discovery.claim-mismatch"),
    )
    .with_source(Mismatch {
        claimed: claimed.to_owned(),
    })
}

#[derive(Debug)]
struct Mismatch {
    claimed: String,
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "manifest claims discovery of `{}` but the provider does not serve it",
            self.claimed
        )
    }
}
impl std::error::Error for Mismatch {}

#[cfg(test)]
mod tests {
    use super::*;

    struct UpnpMock {
        devices: Vec<Device>,
        events: std::sync::Mutex<Vec<DiscoveryChange>>,
    }

    impl ServiceDiscovery for UpnpMock {
        fn discovers(&self) -> Vec<&'static str> {
            vec!["upnp"]
        }
        fn snapshot(&self) -> Result<Vec<Device>, EngineError> {
            Ok(self.devices.clone())
        }
        fn changes(&self) -> Result<Option<Vec<DiscoveryChange>>, EngineError> {
            let mut guard = match self.events.lock() {
                Ok(g) => g,
                Err(poisoned) => poisoned.into_inner(),
            };
            if guard.is_empty() {
                return Ok(None);
            }
            Ok(Some(std::mem::take(&mut *guard)))
        }
    }

    #[test]
    fn snapshot_and_change_drain() {
        let provider = UpnpMock {
            devices: vec![Device {
                id: "upnp-1".to_string(),
                name: "Living Room".to_string(),
                kind: "upnp-media-server".to_string(),
                uri: "http://192.0.2.7:8200/".to_string(),
            }],
            events: std::sync::Mutex::new(vec![DiscoveryChange::Removed {
                id: "upnp-2".to_string(),
            }]),
        };
        assert_eq!(provider.discovers(), vec!["upnp"]);
        let devices = match provider.snapshot() {
            Ok(d) => d,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(devices.len(), 1);
        let changes = match provider.changes() {
            Ok(Some(c)) => c,
            Ok(None) => unreachable!("one pending event"),
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(changes.len(), 1);
        // Second drain: none yet.
        let again = match provider.changes() {
            Ok(v) => v,
            Err(e) => unreachable!("{e}"),
        };
        assert!(again.is_none());
    }

    #[test]
    fn absence_is_data() {
        let provider = UpnpMock {
            devices: Vec::new(),
            events: std::sync::Mutex::new(Vec::new()),
        };
        let devices = match provider.snapshot() {
            Ok(d) => d,
            Err(e) => unreachable!("{e}"),
        };
        assert!(devices.is_empty(), "no devices is Ok(vec![])");
    }

    #[test]
    fn claim_mismatch_error_shape() {
        let err = claim_mismatch("mdns");
        assert_eq!(err.stage, PipelineStage::Discovery);
        let src = match std::error::Error::source(&err) {
            Some(s) => s.to_string(),
            None => unreachable!("source carries the claim"),
        };
        assert!(src.contains("mdns"));
    }
}
