//! `device-discovery` — the services-discovery layer (card 7.D1,
//! issue #42; doc 01 §2.4 L2 registry row).
//!
//! # Responsibility
//!
//! Enumerate network services and local devices into [`device::Device`]
//! lists for the UI to browse (the doc 01 §2.4 sketch is this crate's
//! public trait, [`ServiceDiscovery`]), and watch local volumes /
//! removable media, translating real mount-table diffs into bounded
//! [`events::VolumeEvent`]s — the source the engine host relays into
//! the catalog's `devices.volume-added` / `devices.volume-removed`
//! rows (the relay itself is host wiring; the catalog emission ledger
//! records the split).
//!
//! # The typed-seam design (the wave-22 pattern, pointed the other way)
//!
//! The three wave-22 `platform-*` crates (L5) implement seams defined
//! by core crates; doc 01 §2.7's registry lists `device-discovery` as
//! a dependency of every adapter. This crate therefore **mints the
//! port**: [`volume::VolumeSource`] is the trait the platform adapters
//! will implement (Linux udisks2/mount-table diffs, Windows volume
//! GUID notifications, macOS disk-arbitration). Until those cards
//! land, [`Discovery::probe`] wires the typed fallback
//! ([`volume::UnavailableVolumes`]) whose every method returns the
//! typed [`error::DiscoveryError::Unavailable`] — honest capability
//! reporting via [`capability::DiscoveryReport`], never a fake
//! success and never a fabricated event. A dependency from here on
//! `platform-*` would point upward (L2→L5) and is forbidden by doc
//! 01 §1.4.
//!
//! # The UPnP/DLNA verdict
//!
//! The honest investigation verdict against the in-tree https client
//! (ADR-011, `crates/streaming`) is recorded in [`ssdp`] — SSDP is UDP
//! multicast, the TCP https client can never serve it, the std-only
//! native path is feasible (and partly proven: real loopback UDP
//! round-trips in `tests/ssdp.rs`), and the web build carries the
//! typed [`error::UnavailableReason::BlockedOnWeb`] verdict per doc
//! 09 §10. What is implemented is exactly what is provable; the rest
//! is recorded, not guessed.
//!
//! # Bounded everything (MP §12, directives D-2/D-3)
//!
//! The event queue is capacity-clamped drop-oldest with monotonic
//! dropped/pushed counters; the monitor is ONE bounded thread with a
//! responsive shutdown flag; enumerate passes are device-count
//! clamped ([`context::DiscoveryContext`]); SSDP searches are deadline
//! + response-count bounded with per-datagram size caps.

pub mod capability;
pub mod context;
pub mod device;
pub mod error;
pub mod events;
pub mod monitor;
pub mod ssdp;
pub mod volume;

use capability::{Capability, DiscoveryReport, Feature};
use error::{DiscoveryError, UnavailableReason};

pub use context::DiscoveryContext;
pub use device::{Device, DeviceKind, ServiceDiscovery, ServiceProtocol};
pub use events::{VolumeEvent, VolumeEventQueue, diff_volumes};
pub use monitor::{MonitorConfig, VolumeMonitor, VolumeMonitorHandle, poll_once};
pub use volume::{UnavailableVolumes, Volume, VolumeId, VolumeKind, VolumeSource, VolumesService};

// The domain trait `ServiceDiscovery` is re-exported above from its
// vocabulary module (`device`) because doc 01 §2.4's sketch names it
// as the crate's headline interface.

/// The assembled discovery service: one probe, two lanes, one honest
/// capability report (card 7.D1's entry point).
///
/// Construct via [`Discovery::probe`]. The volume lane's real OS
/// integrations are the `platform-*` follow-up cards (doc 01 §2.7
/// registry edges); until they land, the lane is the typed
/// [`UnavailableVolumes`] fallback and the report says
/// [`UnavailableReason::NotYetImplemented`] — the same honest
/// negotiation posture the wave-22 adapters carry.
pub struct Discovery {
    volumes: std::sync::Mutex<Box<dyn VolumeSource>>,
    ssdp: Option<ssdp::SsdpSearcher>,
    report: DiscoveryReport,
}

impl Discovery {
    /// Probe the host and assemble the best available lanes.
    ///
    /// Probes run: the volume seam (compile-time verdict today — no
    /// platform integration is linked into this crate, by layering)
    /// and the SSDP searcher (a live socket-creation probe on native
    /// targets; the typed web-blocked verdict on wasm). Each outcome
    /// lands verbatim in the returned [`DiscoveryReport`].
    #[must_use = "probing allocates OS-adjacent state; reuse the value"]
    pub fn probe() -> Discovery {
        let mut report = DiscoveryReport::new();

        // Volume lane: the platform adapters do not implement
        // VolumeSource yet. This is a compile-time fact (not a runtime
        // probe) — this crate cannot link L5 code — so the typed
        // verdict is minted here, once, and both volume features
        // share it (enumeration and notifications arrive together
        // from a mount-table watcher, like the audio lane's single
        // PulseAudio verdict in platform-linux).
        let volume_reason = DiscoveryError::Unavailable {
            reason: UnavailableReason::NotYetImplemented {
                detail: "no platform adapter implements VolumeSource yet — the \
                         platform-{windows,linux,macos} volume-watcher cards own the \
                         follow-up (doc 01 §2.7 registry edges; issue #42)",
            },
        };
        let volumes: Box<dyn VolumeSource> = Box::new(UnavailableVolumes::because(volume_reason));
        let volume_capability = Capability::Unavailable(UnavailableReason::NotYetImplemented {
            detail: "volume watching waits on the platform-* VolumeSource integrations",
        });
        report.set(Feature::VolumeEnumeration, volume_capability.clone());
        report.set(Feature::VolumeNotifications, volume_capability);

        // Network lane: the SSDP searcher probe (socket creation on
        // native; typed blocked verdict on wasm — src/ssdp.rs records
        // the full UPnP/DLNA feasibility verdict).
        let (ssdp, ssdp_capability) = ssdp::SsdpSearcher::probe();
        report.set(Feature::NetworkServiceDiscovery, ssdp_capability);

        Discovery {
            volumes: std::sync::Mutex::new(volumes),
            ssdp,
            report,
        }
    }

    /// The capability table computed at discovery time.
    #[must_use = "pure accessor"]
    pub fn report(&self) -> &DiscoveryReport {
        &self.report
    }

    /// The local-volume seam (real integration or typed fallback).
    ///
    /// Locking: one caller at a time (the monitor thread or the
    /// service path — a host wires exactly one consumer; the mutex
    /// keeps the seam sound if they don't).
    pub fn volumes(&self) -> std::sync::MutexGuard<'_, Box<dyn VolumeSource>> {
        match self.volumes.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// The SSDP/UPnP searcher, when the native socket probe passed.
    #[must_use = "pure accessor"]
    pub fn ssdp(&self) -> Option<&ssdp::SsdpSearcher> {
        self.ssdp.as_ref()
    }
}
