//! The device abstraction: [`AudioBackend`] — the audio-output plugin
//! kind's domain trait (doc 01 §2.6; doc 03 §4.9; plugin-api kind 8
//! "final audio sink + master clock").
//!
//! [`AudioBackend`] extends [`audio_engine::AudioSink`] (the mixer's
//! write/latency target) with everything a device owns: enumeration,
//! open, pause/resume, the **sample counter** (via a
//! [`media_clock::SampleCounterSource`] handle — the audio-master
//! clock's truth source) and the **underrun count**. OS types never
//! appear (ARCH-6): devices are [`DeviceId`]s + [`DeviceDesc`]s, our
//! own vocabulary.
//!
//! v0 implementations: [`crate::NullBackend`] (complete) and
//! [`crate::UnsupportedBackend`] (typed not-supported stubs — the
//! real-device adapters' placeholders until ADR-005 lands them in
//! `platform-*`, doc 01 §2.7).

use std::sync::Arc;

use audio_engine::AudioSink;
use media_clock::SampleCounterSource;
use media_types::frame::AudioFormat;
use media_types::EngineError;

/// A device's stable identifier (opaque; minted by
/// [`AudioBackend::enumerate`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceId(u64);

impl DeviceId {
    /// Wrap a raw handle (backends mint these from their own tables).
    pub const fn new(handle: u64) -> DeviceId {
        DeviceId(handle)
    }

    /// The raw handle (stable within the backend that minted it).
    pub fn handle(self) -> u64 {
        self.0
    }
}

/// One enumerable device (doc 03 §4.9 `DeviceDesc`): a name and the
/// shape it prefers. Negotiation happens at [`AudioBackend::open`] —
/// the desc is a suggestion, not a contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceDesc {
    /// The device's identifier.
    pub id: DeviceId,
    /// Human-readable name (never an OS handle string — redacted
    /// vocabulary only, doc 01 §3.4).
    pub name: String,
    /// The format this device prefers.
    pub preferred: AudioFormat,
}

/// Events a sink can report (doc 03 §4.9 b, vocabulary). v0 counts
/// underruns atomically ([`AudioBackend::underruns`]); the event
/// *stream* joins the session event bus in the 3.B cards — the enum
/// is the recorded vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinkEvent {
    /// The device went away mid-playback (unplug, server restart).
    DeviceLost,
    /// The session switched to another device (counter offset
    /// carried — doc 03 §4.9 e).
    DeviceChanged,
    /// Underrun count observed (cumulative; one event per callback
    /// that could not fill entirely from the ring).
    Underrun(u64),
}

/// The audio-output backend trait (doc 01 §2.6: "extends
/// `audio-engine::AudioSink` with device enumeration/open"; doc 03
/// §4.9's device surface).
///
/// # Contract
///
/// - [`open`](AudioBackend::open) is the only transition into the
///   playing state; it validates the requested shape and reserves the
///   output ring (open-time allocation only — the callback path never
///   allocates).
/// - [`write`](AudioSink::write) runs on the **pump side** (it may
///   block for backpressure); the device callback runs elsewhere and
///   never holds a lock the producer takes.
/// - The sample counter is monotonic while the device runs, frozen
///   while paused, and never rewinds across device switches
///   (offset-carried — doc 03 §4.9 e).
pub trait AudioBackend: AudioSink {
    /// Devices this backend can see right now. Empty is honest ("no
    /// devices"), never a placeholder entry.
    fn enumerate(&self) -> Vec<DeviceDesc>;

    /// Open `device` at `format` with an output ring of
    /// `buffer_frames` (whole frames; power-of-two where the backend
    /// requires it). On success this backend **is** the open sink
    /// (doc 01 §2.6's shape) and [`clock_source`](AudioBackend::clock_source)
    /// becomes `Some`.
    ///
    /// # Errors
    /// Typed [`Device`](media_types::ErrorKind::Device) or
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) engine
    /// errors: unknown device, unsupported shape, backend unavailable.
    fn open(
        &mut self,
        device: DeviceId,
        format: &AudioFormat,
        buffer_frames: usize,
    ) -> Result<(), EngineError>;

    /// Pause the device (the sample counter freezes — doc 03 §4.7 e).
    ///
    /// # Errors
    /// Typed engine errors when not open or the device refuses.
    fn pause(&mut self) -> Result<(), EngineError>;

    /// Resume the device (continues from the same media time; the
    /// counter carries on).
    ///
    /// # Errors
    /// As [`AudioBackend::pause`].
    fn resume(&mut self) -> Result<(), EngineError>;

    /// The clock's truth source for this open session (`None` before
    /// open, or when the backend has no counter to expose). The
    /// session hands this to
    /// [`MediaClock::audio_master`](media_clock::MediaClock::audio_master).
    fn clock_source(&self) -> Option<Arc<dyn SampleCounterSource>>;

    /// Underrun events so far (one per callback that could not fill
    /// entirely from the ring; atomic, monotonic).
    fn underruns(&self) -> u64;
}
