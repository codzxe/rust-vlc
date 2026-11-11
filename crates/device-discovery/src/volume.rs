//! The local-volume lane: the seam the `platform-*` adapters will
//! implement (card 7.D1; issue #42).
//!
//! # Why this crate only *mints* the port
//!
//! Doc 01 §2.7's registry lists `device-discovery` as a dependency of
//! all three L5 platform adapters ("OS impls behind plugin traits").
//! The layering therefore runs **adapter → port**: Linux
//! (mount-table diffs / udisks2), Windows (volume GUID notifications)
//! and macOS (DiskArbitration) integrations implement
//! [`VolumeSource`] inside their own crates, where the wave-22 crates
//! keep their audited OS-service exceptions. Until those cards land,
//! [`UnavailableVolumes`] is the only implementation — the typed
//! fallback whose every method names the exact reason. It never
//! fabricates a volume and never fabricates an event; the UI's honest
//! empty state is the truth.
//!
//! # The wire mapping (catalog row contract)
//!
//! The command catalog's `devices.volume-added` payload is
//! `{ id: String, label: String, volumeKind: "fixed" | "removable" |
//! "optical" | "network" }` and `devices.volume-removed` is
//! `{ id: String }` (doc 05 §6.2, the 7.C1 additive rows). A
//! [`VolumeEvent`] maps 1:1 onto those payloads — `VolumeKind::as_str`
//! IS the wire spelling — so the engine-side relay the emission
//! ledger assigns to host wiring is a mechanical field copy, never a
//! re-derivation.

use std::fmt;

use media_types::{EngineError, SourceUri};

use crate::context::DiscoveryContext;
use crate::device::{Device, DeviceKind, ServiceDiscovery};
use crate::error::DiscoveryError;

/// Stable id of one volume (opaque string; stable while mounted —
/// OS-minted by the platform integration, e.g. a Windows volume GUID
/// or a Linux `major:minor` + uuid pair).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VolumeId(String);

impl VolumeId {
    /// Wrap an OS-minted id string.
    #[must_use = "constructs the id"]
    pub fn new(id: impl Into<String>) -> VolumeId {
        VolumeId(id.into())
    }

    /// The id string (already opaque data; fine to log — it carries
    /// no user path by construction, the label does).
    #[must_use = "pure accessor"]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VolumeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The kind of a discovered volume — the catalog's `volumeKind`
/// vocabulary verbatim (commands `VolumeKind`, doc 05 §6.2; kebab-case
/// wire names via [`VolumeKind::as_str`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VolumeKind {
    /// A filesystem on an internal fixed disk.
    Fixed,
    /// A removable medium (USB stick, SD card, …).
    Removable,
    /// An optical disc (CD/DVD/BD).
    Optical,
    /// A network mount (NFS/SMB/…).
    Network,
}

impl VolumeKind {
    /// The wire spelling (doc 05 §6.2 `volumeKind`; kebab-case).
    #[must_use = "pure accessor"]
    pub fn as_str(self) -> &'static str {
        match self {
            VolumeKind::Fixed => "fixed",
            VolumeKind::Removable => "removable",
            VolumeKind::Optical => "optical",
            VolumeKind::Network => "network",
        }
    }
}

impl fmt::Display for VolumeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One local volume, as a platform integration reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volume {
    /// Stable-while-mounted id.
    pub id: VolumeId,
    /// Display label (untrusted text, V7 — OS/filesystem-provided).
    pub label: String,
    /// The volume's kind.
    pub kind: VolumeKind,
    /// The mount root (or drive root on Windows) — where the
    /// filesystem is addressable.
    pub root: String,
}

impl Volume {
    /// The volume as a browsable [`Device`] (doc 01 §2.4 vocabulary):
    /// a percent-encoded `file://` URI of the mount root.
    ///
    /// Encoding: mount roots can contain spaces and non-ASCII (a
    /// Linux automounter derives directories from labels); a raw
    /// `file://{root}` would fail [`SourceUri::parse`]'s printable-
    /// remainder rule and produce an unopenable URI. Bytes outside
    /// the conservative path set (`ALPHA / DIGIT / '-' / '.' / '_' /
    /// '~' / '/'`) are percent-encoded per RFC 3986 §2.1.
    ///
    /// # Errors
    /// [`DiscoveryError::MalformedResponse`] — the root failed to
    /// encode into a parseable URI (the platform integration handed
    /// us something that is not a path).
    pub fn to_device(&self) -> Result<Device, DiscoveryError> {
        let encoded = percent_encode_path(&self.root);
        let uri_text = format!("file://{encoded}");
        let uri = SourceUri::parse(&uri_text).map_err(|err| DiscoveryError::MalformedResponse {
            protocol: "volume-root",
            detail: format!("mount root did not encode into a URI: {err}"),
        })?;
        Ok(Device {
            uri,
            label: self.label.clone(),
            kind: DeviceKind::Volume(self.kind),
        })
    }
}

/// Percent-encode a path for a URI path slot (RFC 3986 §2.1 / §3.3;
/// conservative: only `unreserved` + `/` survive verbatim, so a
/// space-bearing or non-ASCII mount root round-trips through
/// [`SourceUri`] and back).
#[must_use = "pure encoding"]
pub fn percent_encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        let keep = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/');
        if keep {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// The volume seam: enumerate the volumes visible right now.
///
/// Implementations: the OS integration behind `cfg(target_os)` in the
/// L5 `platform-*` crates (their cards), and [`UnavailableVolumes`]
/// — the typed fallback used until then. The fallback is a *real*
/// implementation of graceful degradation: every method answers a
/// typed error naming the reason, never a fake success.
///
/// Polling posture (doc 06 §1.3's engine-driven model, adapted): the
/// integration does NOT own a thread — [`crate::monitor::VolumeMonitor`]
/// polls `enumerate` on a bounded cadence and diffs snapshots into
/// events. A push-style adapter (one that can report changes the
/// moment the OS does) lands as its own seam extension with its card.
pub trait VolumeSource: Send {
    /// Volumes visible right now. An empty list is honest ("no
    /// volumes"), matching the engine's `enumerate` contracts; a
    /// source that cannot answer returns the typed error.
    ///
    /// # Errors
    /// Typed [`DiscoveryError`] — integration absent, backing service
    /// unreachable, or the pass exceeded its bounds.
    fn enumerate(&mut self, ctx: &DiscoveryContext) -> Result<Vec<Volume>, DiscoveryError>;
}

/// The typed fallback: no OS integration behind this instance.
///
/// Constructed with the exact reason (follow-up owner, blocked
/// target, …) so callers and the capability report agree on the
/// truth. Mirrors the wave-22 `UnavailableAudioDevices` shape.
#[derive(Debug, Clone)]
pub struct UnavailableVolumes {
    reason: DiscoveryError,
}

impl UnavailableVolumes {
    /// Wrap the reason every method will return.
    #[must_use = "constructs the typed fallback"]
    pub fn because(reason: DiscoveryError) -> UnavailableVolumes {
        UnavailableVolumes { reason }
    }

    /// The reason this integration is unavailable.
    #[must_use = "pure accessor"]
    pub fn reason(&self) -> &DiscoveryError {
        &self.reason
    }
}

impl VolumeSource for UnavailableVolumes {
    fn enumerate(&mut self, _ctx: &DiscoveryContext) -> Result<Vec<Volume>, DiscoveryError> {
        Err(self.reason.clone())
    }
}

/// The local-volumes lane as a doc 01 §2.4 [`ServiceDiscovery`]: one
/// bounded pass over the seam, answered in [`Device`] vocabulary.
///
/// Wraps the source in a mutex so the trait's `&self` discovery
/// method can share a `&mut self` seam with the monitor thread (a
/// host wires exactly one consumer in practice; the lock keeps the
/// seam sound if they don't, and no method holds it across a call
/// out).
pub struct VolumesService {
    source: std::sync::Mutex<Box<dyn VolumeSource>>,
}

impl VolumesService {
    /// Build the service over any volume seam (real integration or
    /// typed fallback).
    #[must_use = "constructs the service"]
    pub fn new(source: Box<dyn VolumeSource>) -> VolumesService {
        VolumesService {
            source: std::sync::Mutex::new(source),
        }
    }
}

impl ServiceDiscovery for VolumesService {
    fn discover(&self, ctx: &DiscoveryContext) -> Result<Vec<Device>, EngineError> {
        let mut source = match self.source.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let volumes = source
            .enumerate(ctx)
            .map_err(DiscoveryError::into_engine_error)?;
        let mut devices = Vec::with_capacity(volumes.len());
        for volume in &volumes {
            // Per-volume conversion failure fails the pass: a source
            // that reports un-URI-able roots is violating its
            // contract, and silently dropping volumes would hide it.
            let device = volume
                .to_device()
                .map_err(DiscoveryError::into_engine_error)?;
            devices.push(device);
        }
        if devices.len() > ctx.max_devices {
            return Err(DiscoveryError::CapacityExceeded {
                what: "device list",
                limit: ctx.max_devices,
                attempted: devices.len(),
            }
            .into_engine_error());
        }
        Ok(devices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn removable(id: &str, label: &str, root: &str) -> Volume {
        Volume {
            id: VolumeId::new(id),
            label: label.to_string(),
            kind: VolumeKind::Removable,
            root: root.to_string(),
        }
    }

    #[test]
    fn kind_wire_names_are_the_catalog_spelling() {
        assert_eq!(VolumeKind::Fixed.as_str(), "fixed");
        assert_eq!(VolumeKind::Removable.as_str(), "removable");
        assert_eq!(VolumeKind::Optical.as_str(), "optical");
        assert_eq!(VolumeKind::Network.as_str(), "network");
    }

    #[test]
    fn to_device_percent_encodes_space_bearing_roots() {
        let volume = removable("usb-1", "My Volume", "/media/My Volume");
        let device = match volume.to_device() {
            Ok(d) => d,
            Err(e) => unreachable!("space-bearing root must encode: {e}"),
        };
        // The URI keeps the space percent-encoded (SourceUri rejects
        // literal spaces) and the label stays raw data.
        assert_eq!(
            device.uri.raw(),
            "file:///media/My%20Volume",
            "raw is the access-layer form; redacted() is the log form"
        );
        assert_eq!(device.label, "My Volume");
        assert_eq!(device.kind, DeviceKind::Volume(VolumeKind::Removable));
    }

    #[test]
    fn to_device_keeps_unreserved_bytes_verbatim() {
        let volume = removable("d1", "data", "/mnt/data-2026_disk");
        let device = match volume.to_device() {
            Ok(d) => d,
            Err(e) => unreachable!("unreserved root must parse: {e}"),
        };
        assert_eq!(device.uri.raw(), "file:///mnt/data-2026_disk");
    }

    #[test]
    fn percent_encoder_handles_non_ascii_and_windows_roots() {
        assert_eq!(percent_encode_path("/media/Üsb"), "/media/%C3%9Csb");
        // ':' and '\' are not in the conservative keep-set (RFC 3986
        // reserves ':' for the scheme separator) — a Windows drive
        // root encodes fully; still a valid, round-trippable URI path.
        assert_eq!(percent_encode_path("E:\\"), "E%3A%5C");
        assert_eq!(percent_encode_path("/plain"), "/plain");
    }

    #[test]
    fn unavailable_fallback_answers_typed_errors_only() {
        let reason = DiscoveryError::Unavailable {
            reason: crate::error::UnavailableReason::NotYetImplemented {
                detail: "platform cards own the follow-up",
            },
        };
        let mut fallback = UnavailableVolumes::because(reason);
        let outcome = fallback.enumerate(&DiscoveryContext::new());
        match outcome {
            Err(DiscoveryError::Unavailable { .. }) => {}
            other => unreachable!("fallback never fakes success, got {other:?}"),
        }
        // The reason is data: repeated calls return it verbatim.
        assert!(fallback.enumerate(&DiscoveryContext::new()).is_err());
    }

    #[test]
    fn volumes_service_translates_the_whole_pass() {
        let service = VolumesService::new(Box::new(UnavailableVolumes::because(
            DiscoveryError::Unavailable {
                reason: crate::error::UnavailableReason::NotYetImplemented { detail: "gap" },
            },
        )));
        let err = match service.discover(&DiscoveryContext::new()) {
            Err(e) => e,
            Ok(devices) => unreachable!("unavailable lane must not succeed: {devices:?}"),
        };
        assert_eq!(err.stage, media_types::PipelineStage::Discovery);
        assert_eq!(err.context.code, "discovery.unavailable");
    }
}
