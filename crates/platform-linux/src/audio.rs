//! The audio seam (card 7.A2): device enumeration + change
//! notifications against the `audio-output` vocabulary (doc 01 §2.7
//! — platform crates implement traits defined in core crates;
//! ARCH-6: no OS types in public signatures).
//!
//! v0 scope is the **enumeration/notification half** of the adapter:
//! the streaming half (`AudioBackend::open`, the render callback)
//! stays behind the ADR-005 admission gate — the same gate that
//! kept cpal out of `audio-output` (records/cpal-0.18.2.md). When
//! that gate opens, the real backend consumes this seam's
//! [`DeviceDesc`](audio_output::DeviceDesc)s directly: same
//! [`DeviceId`] minting, same names.

use std::sync::Arc;

use audio_output::{DeviceDesc, DeviceId};

use crate::error::PlatformError;
use crate::events::DeviceEventQueue;

/// The audio device seam: enumerate, and subscribe to changes.
///
/// Implementations: the OS integration behind `cfg(target_os)` and
/// [`UnavailableAudioDevices`] — the typed fallback used when the
/// integration is not compiled in or its runtime dependencies are
/// absent. The fallback is a *real* implementation of graceful
/// degradation: every method answers a typed error naming the
/// reason, never a fake success.
pub trait AudioDevices: Send {
    /// Devices visible right now. An empty list is honest ("no
    /// devices"), matching [`AudioBackend::enumerate`]'s contract.
    ///
    /// # Errors
    /// Typed [`PlatformError`] — library absent, server unreachable,
    /// call failed.
    fn enumerate(&mut self) -> Result<Vec<DeviceDesc>, PlatformError>;

    /// Subscribe to device changes; the integration pushes
    /// [`PlatformEvent`]s into `queue` (bounded — the queue owns its
    /// memory bound, D-3). Re-subscribing replaces the previous
    /// queue.
    ///
    /// # Errors
    /// Typed [`PlatformError`] when notifications are not possible
    /// (integration absent, server unreachable).
    fn subscribe(&mut self, queue: Arc<DeviceEventQueue>) -> Result<(), PlatformError>;

    /// Pump the OS event loop once, non-blocking. Returns the number
    /// of loop iterations dispatched. Push-based integrations
    /// (Windows COM callbacks, CoreAudio listeners) return 0 —
    /// events already sit in the queue; poll-style integrations
    /// (PulseAudio mainloop) dispatch here.
    ///
    /// # Errors
    /// Typed [`PlatformError`] when the loop cannot be pumped.
    fn poll(&mut self) -> Result<u32, PlatformError>;
}

/// The typed fallback: no OS integration behind this instance.
///
/// Constructed with the exact reason (wrong OS, library missing, …)
/// so callers and the capability report agree on the truth.
#[derive(Debug, Clone)]
pub struct UnavailableAudioDevices {
    reason: PlatformError,
}

impl UnavailableAudioDevices {
    /// Wrap the reason every method will return.
    #[must_use = "constructs the typed fallback"]
    pub fn because(reason: PlatformError) -> UnavailableAudioDevices {
        UnavailableAudioDevices { reason }
    }

    /// The reason this integration is unavailable.
    #[must_use = "pure accessor"]
    pub fn reason(&self) -> &PlatformError {
        &self.reason
    }
}

impl AudioDevices for UnavailableAudioDevices {
    fn enumerate(&mut self) -> Result<Vec<DeviceDesc>, PlatformError> {
        Err(self.reason.clone())
    }

    fn subscribe(&mut self, _queue: Arc<DeviceEventQueue>) -> Result<(), PlatformError> {
        Err(self.reason.clone())
    }

    fn poll(&mut self) -> Result<u32, PlatformError> {
        Err(self.reason.clone())
    }
}

/// Mint a stable [`DeviceId`] from an OS device identifier string
/// (PulseAudio sink name, WASAPI endpoint id, CoreAudio UID).
///
/// The OS strings are long and stable across sessions; the engine's
/// id is `u64`, so the mapping is FNV-1a-64 — deterministic,
/// std-only, and documented as **not collision-free** (a hash clash
/// would alias two devices' ids; the birthday bound for 64-bit makes
/// that negligible at audio-device counts, and ids are only
/// meaningful within one backend instance — doc 03 §4.9).
#[must_use = "pure hashing"]
pub fn stable_device_id(os_identifier: &str) -> DeviceId {
    // FNV-1a 64-bit over the UTF-8 bytes.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in os_identifier.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    DeviceId::new(hash)
}
