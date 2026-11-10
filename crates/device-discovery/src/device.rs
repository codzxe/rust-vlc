//! The device vocabulary of the services-discovery layer (doc 01
//! §2.4's interface sketch, materialized).
//!
//! [`Device`] is data, not UI copy: labels come from the OS or the
//! network and are localized at render time (doc 08 layering); they
//! are untrusted text (V7 — never interpolate them into markup, which
//! is the UI panel's contract, not this crate's).
//!
//! Relationship to `plugin-api`'s `Device`: that type is the HOST
//! surface for sandboxed plugins (stringly `kind`, its own id
//! scheme); this one is the domain vocabulary. The conversion is the
//! plugin host's composition job (a plugin's devices enter the engine
//! through its own contract), not a dependency edge here — the two
//! crates deliberately share no types beyond media-types.

use std::fmt;

use media_types::{EngineError, SourceUri};

use crate::context::DiscoveryContext;
use crate::volume::VolumeKind;

/// The domain port from the doc 01 §2.4 sketch: one bounded discovery
/// pass over a provider's device set.
///
/// `plugin-api`'s `ServiceDiscovery` (doc 06 §1.2 row 16) is the HOST
/// surface for sandboxed plugins — snapshot + drained changes on the
/// engine's cadence; this trait is the domain vocabulary that sketch
/// names this crate as the home of. Implementations must be bounded
/// and non-blocking beyond `ctx`'s budget: absence of devices is
/// `Ok(vec![])`, never an error (MP §8).
pub trait ServiceDiscovery {
    /// One bounded enumeration pass.
    ///
    /// # Errors
    /// [`EngineError`] (stage
    /// [`media_types::PipelineStage::Discovery`]) — the provider was
    /// asked and could not answer; "no devices" is NOT an error.
    fn discover(&self, ctx: &DiscoveryContext) -> Result<Vec<Device>, EngineError>;
}

/// One discovered device (doc 01 §2.4 sketch: uri + label + kind).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// Where to reach it (validated by [`SourceUri`]; for volumes, a
    /// percent-encoded `file://` URI of the mount root — see
    /// [`Volume::to_device`]).
    pub uri: SourceUri,
    /// Display label as the OS/network advertised it (data, not UI
    /// copy; untrusted text, V7).
    pub label: String,
    /// What kind of thing this is (sorting/filtering vocabulary).
    pub kind: DeviceKind,
}

/// The device-kind vocabulary: the local-volume lane plus the network
/// service protocols with a recorded verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceKind {
    /// A local volume (fixed disk, removable medium, optical disc,
    /// network mount).
    Volume(VolumeKind),
    /// A network service (SSDP/UPnP today; mDNS and friends join with
    /// their own cards and verdicts).
    Service(ServiceProtocol),
}

/// Network service protocols the discovery layer speaks (or has a
/// recorded verdict for).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceProtocol {
    /// UPnP (SSDP discovery; DLNA devices ride the same exchange —
    /// see [`crate::ssdp`]'s verdict for what is implemented versus
    /// recorded).
    Upnp,
}

impl ServiceProtocol {
    /// The stable wire-ish name (manifest `discovers` vocabulary,
    /// doc 06 §7.2 — matches plugin-api's examples).
    #[must_use = "pure accessor"]
    pub fn as_str(self) -> &'static str {
        match self {
            ServiceProtocol::Upnp => "upnp",
        }
    }
}

impl fmt::Display for ServiceProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uri(s: &str) -> SourceUri {
        match SourceUri::parse(s) {
            Ok(u) => u,
            Err(e) => unreachable!("fixture uri must parse: {e}"),
        }
    }

    #[test]
    fn device_is_plain_data() {
        let device = Device {
            uri: uri("file:///media/backup"),
            label: "backup".to_string(),
            kind: DeviceKind::Volume(VolumeKind::Removable),
        };
        assert_eq!(device.uri.scheme().as_str(), "file");
        assert_eq!(device.label, "backup");
        assert_eq!(device.kind, DeviceKind::Volume(VolumeKind::Removable));
    }

    #[test]
    fn service_protocol_names_match_the_manifest_vocabulary() {
        assert_eq!(ServiceProtocol::Upnp.as_str(), "upnp");
        assert_eq!(ServiceProtocol::Upnp.to_string(), "upnp");
    }
}
