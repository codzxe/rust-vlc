//! The cpal adapter: **ADR-005's admitted backend** (wave FX3, Route A
//! of FIX-PLAN §3 F1.1). This module is where the admission flip ran —
//! the same point the streaming TLS seam held
//! (`HTTPS_ADR011_ADMITTED`, `crates/streaming/src/schemes.rs`,
//! ADR-011): the gate const flipped to `true`, and the compile-time
//! assertion, the refusal arm and the `BackendNotAdmitted`-shaped
//! marker vocabulary were **deleted in the same change** the real
//! adapter landed — the flip-point discipline's "one change" rule,
//! run to completion. (The marker no longer exists; these docs name
//! it as history, exactly the way schemes.rs documents its deleted
//! seam.)
//!
//! # The admission record (who decided what)
//!
//! Route A (FIX-PLAN §3 F1.1, Lead): **cpal =0.18.2** is admitted as
//! an **optional** dependency of this crate behind the
//! `backend-cpal` Cargo feature — the registry-row amendment the P0
//! record anticipated ("or in an audio-output feature if the Lead
//! explicitly amends the registry row",
//! `docs/dependencies/records/cpal-0.18.2.md` R10). The evidence
//! chain is that record (R1–R10) plus the wave-14 admission spike
//! (`docs/dependencies/records/audio-output-cpal.md`: real graphs
//! for all three desktop targets, the C-FFI unsafe counts, the
//! alternatives survey); FX3-A02 lands the ADR-005 amendment, the
//! dependency-allowlist rows and the deny.toml allowances in the
//! same wave. The E-class posture is unchanged from the records:
//! the native surface is the OS's own audio service (doc 11 §1.2
//! E2), all `unsafe` lives inside cpal's host layers (counted in the
//! spike record's R4) — the adapter code here is std-only and
//! unsafe-free.
//!
//! # Default builds: byte-compatible with the pre-admission crate
//!
//! The feature is **off by default**: the default dependency set
//! stays workspace-internal, the default `builtin_backends()` stays
//! null-then-stub, and [`cpal_backend`] answers the null backend —
//! headless/CI builds behave exactly as before the admission.
//! [`cpal_linked`] reports whether *this build* links the substrate
//! (the const above records the decision; this reports the build —
//! product surfaces read it instead of guessing, closing the audit
//! gap that the admission state was unqueryable).
//!
//! # The adapter's real-time discipline (doc 03 §4.9 c, binding)
//!
//! cpal hands the device callback a `&mut [f32]` buffer; the
//! callback body is the crate's own RT core, [`crate::render_into`],
//! **verbatim** — ring read (≤ 2 contiguous segments), silence fill
//! on underrun, one counter advance, one underrun count per short
//! callback. Nothing else runs there: no locks (the shared state is
//! single-word atomics), no allocation (open-time only), no
//! logging, no syscalls (the borrows-only signature is the
//! structural half of the contract; the op-count ledger is measured
//! by the null-adapter ACs). The device-error callback follows the
//! same discipline: it counts, never logs. The [`crate::SampleCounter`]
//! stays the clock's only truth source — cpal exposes no
//! sample-position API on all hosts (the P0 record's R6), so the
//! atomic fed by this callback is exactly the design the seam
//! shipped.
//!
//! # Honest limits (recorded, not hidden)
//!
//! This sandbox can neither compile the feature (no ALSA dev
//! package: the alsa-sys pkg-config probe fails, exit 101 — P0
//! record R5) nor exercise a device (none exists). The
//! `backend-cpal` compile oracle is FX3-A07's windows CI lane; the
//! on-device acceptance work is ADR-005's post-admission conformance
//! plan (format matrix, measured callback budget, pause/resume
//! counter discipline) — hardware-labeled evidence, never claimed
//! from here.

use crate::device::AudioBackend;
use media_types::EngineError;

/// The ADR-005 admission flip point, now recorded **open** (Route A,
/// FIX-PLAN §3 F1.1): the Lead's amendment admits cpal =0.18.2 as the
/// optional real-device substrate of this crate, behind the
/// `backend-cpal` feature.
///
/// `true` records the **decision** (ADR-005 amended, allowlist row
/// pinned) — not "this binary plays audio": whether *this build*
/// links the substrate is [`cpal_linked`] (the feature flag). The
/// pre-admission discipline — the compile-time `assert!` gate, the
/// refusal arm, the `BackendNotAdmitted` marker — is deleted with
/// the admission, exactly as this module's docs always promised; the
/// default (feature-off) build's honest answer is the null backend.
pub const AUDIO_ADR005_ADMITTED: bool = true;

/// Whether **this build** links the admitted substrate
/// (`cfg!(feature = "backend-cpal")`).
///
/// The pair with [`AUDIO_ADR005_ADMITTED`] is deliberate: the const
/// records the Lead's decision (repo truth, build-independent), this
/// reports the compiled artifact (build truth). Product surfaces
/// (engine-info, feature-matrix, `product.toml`'s audio row) read
/// this instead of asserting device capability the binary does not
/// carry.
pub const fn cpal_linked() -> bool {
    cfg!(feature = "backend-cpal")
}

// ---------------------------------------------------------------------------
// Feature ON: the real adapter.
// ---------------------------------------------------------------------------

#[cfg(feature = "backend-cpal")]
use std::fmt;
#[cfg(feature = "backend-cpal")]
use std::sync::Arc;
#[cfg(feature = "backend-cpal")]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[cfg(feature = "backend-cpal")]
use audio_engine::AudioSink;
#[cfg(feature = "backend-cpal")]
use audio_engine::buffer::AudioBuffer;
#[cfg(feature = "backend-cpal")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(feature = "backend-cpal")]
use media_clock::SampleCounterSource;
#[cfg(feature = "backend-cpal")]
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
#[cfg(feature = "backend-cpal")]
use media_types::time::{Duration, Nanoseconds};

#[cfg(feature = "backend-cpal")]
use crate::counter::SampleCounter;
#[cfg(feature = "backend-cpal")]
use crate::device::{DeviceDesc, DeviceId};
#[cfg(feature = "backend-cpal")]
use crate::error;
#[cfg(feature = "backend-cpal")]
use crate::render::render_into;
#[cfg(feature = "backend-cpal")]
use crate::ring::SpscRing;

/// The preferred-format suggestion when a device's config probe fails
/// (the null backend's own preference — a desc is a suggestion, not a
/// contract; open negotiates the truth against the device's table).
#[cfg(feature = "backend-cpal")]
const PREFERRED_SUGGESTION: AudioFormat = AudioFormat {
    sample: SampleFormat::F32,
    channels: 2,
    layout: AudioLayout::Interleaved,
    rate: 48_000,
};

/// State shared by the pump half (producer) and the device callback
/// (consumer) — the same shape [`crate::null`] splits into its two
/// halves: the ring, the clock counter, the underrun count, the pause
/// flag. All single-word atomics — no locks anywhere (the
/// no-blocking contract's structural half).
#[cfg(feature = "backend-cpal")]
#[derive(Debug)]
struct Shared {
    ring: SpscRing,
    counter: Arc<SampleCounter>,
    underruns: AtomicU64,
    paused: AtomicBool,
    /// Cumulative frames pushed into the ring (monotonic; the
    /// write-side twin of the counter's consumed count — parity with
    /// [`crate::null`]'s bookkeeping, the count the seek boundary's
    /// exact-prefix arithmetic reads when in-session seek lands).
    written: AtomicU64,
}

#[cfg(feature = "backend-cpal")]
impl Shared {
    fn open(format: &AudioFormat, buffer_frames: usize) -> Result<Shared, EngineError> {
        // v0 devices speak interleaved f32 (doc 03 §4.9's callback
        // payload type); anything else is a typed Device error — the
        // same gate every backend in this crate applies.
        if format.sample != SampleFormat::F32
            || format.layout != AudioLayout::Interleaved
            || format.channels == 0
            || format.rate == 0
        {
            return Err(error::format_unsupported(format));
        }
        Ok(Shared {
            ring: SpscRing::new(format.channels, buffer_frames)?,
            counter: Arc::new(SampleCounter::new(format.rate)?),
            underruns: AtomicU64::new(0),
            paused: AtomicBool::new(false),
            written: AtomicU64::new(0),
        })
    }
}

/// One open device session: the shared SPSC state plus the live cpal
/// stream. The stream must outlive playback (dropping a cpal `Stream`
/// stops it) — it parks inside the backend exactly the way
/// [`crate::NullBackend`] parks its driver, with the OS callback
/// thread in place of the manual [`crate::null::NullDevice::render`]
/// calls.
#[cfg(feature = "backend-cpal")]
struct Session {
    shared: Arc<Shared>,
    stream: cpal::Stream,
    /// The error-callback counter (device-side failures are counted,
    /// never logged — the callback path forbids logging).
    errors: Arc<AtomicU64>,
    format: AudioFormat,
}

/// One enumerated device (the constructor's snapshot): the cpal device
/// handle plus our redacted vocabulary for it. [`crate::DeviceId`]s are the
/// snapshot's index — stable within the backend that minted them (the
/// registry's contract).
#[cfg(feature = "backend-cpal")]
struct DeviceEntry {
    device: cpal::Device,
    desc: DeviceDesc,
}

/// Negotiate an interleaved-f32 stream config on `device` for `format`:
/// the device's own supported-config table decides — no silent
/// resampling, no channel remapping (rate conversion is FX3-A04's
/// separate work; a mismatch refuses with the typed
/// `audio.format_unsupported`).
#[cfg(feature = "backend-cpal")]
fn negotiate(
    device: &cpal::Device,
    format: &AudioFormat,
) -> Result<cpal::StreamConfig, EngineError> {
    let channels = u16::try_from(format.channels).map_err(|_| error::format_unsupported(format))?;
    let ranges = device
        .supported_output_configs()
        .map_err(|_| error::format_unsupported(format))?;
    for range in ranges {
        if range.channels() != channels {
            continue;
        }
        if !range.sample_formats().contains(&cpal::SampleFormat::F32) {
            continue;
        }
        if format.rate < range.min_sample_rate().0 || format.rate > range.max_sample_rate().0 {
            continue;
        }
        // Found: an f32 config at exactly the requested shape. The
        // device buffer size stays the host's default period — the
        // ring's `buffer_frames` capacity (power of two, ours) is the
        // backpressure budget, NOT the device period; coupling them
        // would drain the ring every callback and underrun by design.
        return Ok(cpal::StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(format.rate),
            buffer_size: cpal::BufferSize::Default,
        });
    }
    Err(error::format_unsupported(format))
}

/// The admitted cpal backend (feature `backend-cpal` only): a real
/// OS-device [`AudioBackend`] — enumerate, open, pause/resume, the
/// sample-counter clock source and underrun accounting — built on
/// this crate's own SPSC ring and [`crate::render_into`] callback core (the
/// ADR-005 conformance plan's clause 2: the adapter installs
/// `render_into` as the cpal callback unchanged).
///
/// Construct via [`cpal_backend`] or [`CpalBackend::new`]. A host with
/// no output devices enumerates **zero** devices honestly (empty is
/// honest, never a placeholder entry) and every open refuses with
/// the typed `audio.device_id_unknown`.
#[cfg(feature = "backend-cpal")]
pub struct CpalBackend {
    devices: Vec<DeviceEntry>,
    session: Option<Session>,
}

#[cfg(feature = "backend-cpal")]
impl CpalBackend {
    /// A backend over this host's output devices — a constructor-time
    /// snapshot; re-enumerate by constructing a new backend (device
    /// hotplug events are the session event bus's later work, doc 03
    /// §4.9 b).
    ///
    /// Devices whose name cannot be read are skipped honestly; a
    /// device whose config probe fails keeps the
    /// [`PREFERRED_SUGGESTION`] shape (48 kHz stereo f32 — the desc is
    /// a suggestion; open negotiates the truth).
    pub fn new() -> CpalBackend {
        let host = cpal::default_host();
        let mut devices = Vec::new();
        for (handle, device) in host.output_devices().enumerate() {
            let Ok(name) = device.name() else {
                continue;
            };
            let preferred = device
                .default_output_config()
                .ok()
                .map(|supported| {
                    let config = supported.config();
                    AudioFormat {
                        sample: SampleFormat::F32,
                        channels: u32::from(config.channels),
                        layout: AudioLayout::Interleaved,
                        rate: config.sample_rate.0,
                    }
                })
                .unwrap_or(PREFERRED_SUGGESTION);
            devices.push(DeviceEntry {
                device,
                desc: DeviceDesc {
                    id: DeviceId::new(handle as u64),
                    name,
                    preferred,
                },
            });
        }
        CpalBackend {
            devices,
            session: None,
        }
    }

    /// Whether a session is open.
    pub fn is_open(&self) -> bool {
        self.session.is_some()
    }

    /// Device-side stream errors so far (the error callback's
    /// counter; monotonic — surfaced here because logging is
    /// forbidden on the device thread).
    pub fn stream_errors(&self) -> u64 {
        self.session
            .as_ref()
            .map_or(0, |session| session.errors.load(Ordering::Relaxed))
    }

    /// Underrun events so far (monotonic; the sink-side view of the
    /// same atomic the callback increments).
    pub fn underruns(&self) -> u64 {
        self.session.as_ref().map_or(0, |session| {
            session.shared.underruns.load(Ordering::Relaxed)
        })
    }

    /// Sample instants consumed so far (monotonic while the device
    /// runs, frozen while paused — the clock's truth).
    pub fn sample_count(&self) -> u64 {
        self.session
            .as_ref()
            .map_or(0, |session| session.shared.counter.count())
    }

    /// Frames currently queued in the ring (the write-side view).
    pub fn buffered_frames(&self) -> usize {
        self.session
            .as_ref()
            .map_or(0, |session| session.shared.ring.available_frames())
    }

    /// Cumulative frames pushed into the ring (monotonic; the
    /// written/consumed pair bounds the ring's live content).
    pub fn written_frames(&self) -> u64 {
        self.session
            .as_ref()
            .map_or(0, |session| session.shared.written.load(Ordering::Relaxed))
    }

    fn session(&self) -> Result<&Session, EngineError> {
        self.session
            .as_ref()
            .ok_or_else(|| error::unsupported("audio.sink_not_open"))
    }

    /// Raw interleaved f32 **bytes** into the session ring — the null
    /// sink's zero-heap-traffic refill discipline, mirrored here
    /// (D-1/D-8: this is the pump's write path): whole-frame stack
    /// chunks, no per-quantum `Vec<f32>`, byte-identical samples in
    /// the same order, honest all-or-nothing backpressure
    /// (`audio.ring_full`). Callers validate whole-frame alignment
    /// first (the `AudioSink::write` path does).
    fn push_bytes(shared: &Shared, interleaved: &[u8]) -> Result<(), EngineError> {
        let frame_bytes = shared.ring.channels() as usize * 4; // f32 slots
        let frames = (interleaved.len() / frame_bytes) as u64;
        let free = shared.ring.free_frames() as u64;
        if free < frames {
            return Err(error::ring_full(
                shared.ring.capacity_frames() as u64,
                frames,
            ));
        }
        // The frame-aligned refill step: the largest whole number of
        // frames that fits the 256-sample stack chunk (one frame
        // minimum), so every `ring.push` sees a whole-frame slice.
        let frame_samples = frame_bytes / 4;
        let step = (256 / frame_samples).max(1) * frame_samples;
        let pushed_total = if step <= 256 {
            let mut chunk = [0f32; 256];
            Self::push_aligned(&shared.ring, interleaved, &mut chunk, step)
        } else {
            let mut chunk = vec![0f32; step];
            Self::push_aligned(&shared.ring, interleaved, &mut chunk, step)
        };
        if pushed_total as u64 != frames {
            // Cannot happen under the free-check above (single
            // producer); if it ever does, refuse honestly rather than
            // drop data.
            return Err(error::ring_full(
                shared.ring.capacity_frames() as u64,
                frames,
            ));
        }
        shared
            .written
            .fetch_add(pushed_total as u64, Ordering::Relaxed);
        Ok(())
    }

    /// Refill `chunk` from the byte view `step` samples at a time and
    /// push each whole-frame slice — the null sink's shared body,
    /// mirrored (the crate-wide sink-half helper lands with the
    /// instrumentation seam, FX3-A05's files this wave).
    fn push_aligned(ring: &SpscRing, interleaved: &[u8], chunk: &mut [f32], step: usize) -> usize {
        let mut pushed_total = 0usize;
        for part in interleaved.as_chunks::<4>().0.chunks(step) {
            for (dst, bytes) in chunk.iter_mut().zip(part) {
                *dst = f32::from_le_bytes(*bytes);
            }
            pushed_total += ring.push(&chunk[..part.len()]);
        }
        pushed_total
    }
}

#[cfg(feature = "backend-cpal")]
impl Default for CpalBackend {
    fn default() -> CpalBackend {
        CpalBackend::new()
    }
}

#[cfg(feature = "backend-cpal")]
impl fmt::Debug for CpalBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CpalBackend")
            .field("devices", &self.devices.len())
            .field("open", &self.session.is_some())
            .finish()
    }
}

#[cfg(feature = "backend-cpal")]
impl AudioSink for CpalBackend {
    fn write(&mut self, buf: &AudioBuffer) -> Result<(), EngineError> {
        let session = self.session()?;
        if buf.format != session.format {
            return Err(error::unsupported("audio.format_mismatch"));
        }
        let frame_bytes = session.format.frame_bytes() as usize;
        let data = buf.data.as_slice();
        if !data.len().is_multiple_of(frame_bytes) {
            return Err(error::partial_frame(
                frame_bytes as u64,
                (data.len() % frame_bytes) as u64,
            ));
        }
        Self::push_bytes(&session.shared, data)
    }

    fn latency(&self) -> Option<Duration> {
        let session = self.session().ok()?;
        let frames = session.shared.ring.available_frames() as u128;
        let nanos = frames * 1_000_000_000u128 / u128::from(session.format.rate);
        let nanos = u64::try_from(nanos).ok()?;
        Some(Duration::Known(Nanoseconds(nanos)))
    }
}

#[cfg(feature = "backend-cpal")]
impl AudioBackend for CpalBackend {
    fn enumerate(&self) -> Vec<DeviceDesc> {
        self.devices
            .iter()
            .map(|entry| entry.desc.clone())
            .collect()
    }

    fn open(
        &mut self,
        device: DeviceId,
        format: &AudioFormat,
        buffer_frames: usize,
    ) -> Result<(), EngineError> {
        let entry = self
            .devices
            .get(device.handle() as usize)
            .ok_or_else(|| error::unsupported("audio.device_id_unknown"))?;
        // Open-time allocation only (the callback path never
        // allocates): validate the session shape, negotiate against
        // the device's own table, then build.
        let config = negotiate(&entry.device, format)?;
        let shared = Arc::new(Shared::open(format, buffer_frames)?);

        // The device callback: the crate's RT core, verbatim — ring →
        // the device's buffer, silence on underrun (never old audio),
        // one counter advance, one underrun count per short callback.
        // Nothing else runs here (doc 03 §4.9 c: no locks, no
        // allocation, no logging, no syscalls — render_into's
        // borrows-only signature is the structural half of that).
        let callback_shared = shared.clone();
        let data_callback = move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
            if callback_shared.paused.load(Ordering::Acquire) {
                // The paused device pulls nothing in the null model;
                // a live OS stream may keep pulling in the window
                // between pause() and the host halting it — those
                // callbacks answer silence (never old audio) and
                // freeze the counter (doc 03 §4.7 e).
                data.fill(0.0);
                return;
            }
            let _stats = render_into(
                &callback_shared.ring,
                &callback_shared.counter,
                &callback_shared.underruns,
                data,
            );
        };

        // The error callback: count, never log (logging is forbidden
        // on the device thread; `stream_errors()` surfaces the count).
        let errors = Arc::new(AtomicU64::new(0));
        let error_shared = errors.clone();
        let error_callback = move |_err: cpal::StreamError| {
            let _ = error_shared.fetch_add(1, Ordering::Relaxed);
        };

        let stream = entry
            .device
            .build_output_stream(&config, data_callback, error_callback, None)
            .map_err(|_| error::unsupported("audio.device_build_failed"))?;
        // open IS the transition into the playing state (the trait's
        // contract) — the stream starts live, like the null driver.
        stream
            .play()
            .map_err(|_| error::unsupported("audio.device_start_failed"))?;
        self.session = Some(Session {
            shared,
            stream,
            errors,
            format: *format,
        });
        Ok(())
    }

    fn pause(&mut self) -> Result<(), EngineError> {
        let session = self.session()?;
        // Flag first: the instant the pause lands, in-flight callbacks
        // go silent and freeze the counter (doc 03 §4.7 e — pause
        // freezes it), then the stream itself stops pulling. A host
        // that refuses stream pause (ALSA has no native device pause —
        // a recorded platform limit) answers a typed error while the
        // session degrades to silence, never to stale audio.
        session.shared.paused.store(true, Ordering::Release);
        session
            .stream
            .pause()
            .map_err(|_| error::unsupported("audio.device_pause_failed"))
    }

    fn resume(&mut self) -> Result<(), EngineError> {
        let session = self.session()?;
        // Stream first, flag after: a callback racing the window
        // answers silence (the flag still says paused) rather than
        // advancing the clock on audio the host has not restarted.
        session
            .stream
            .play()
            .map_err(|_| error::unsupported("audio.device_resume_failed"))?;
        session.shared.paused.store(false, Ordering::Release);
        Ok(())
    }

    fn clock_source(&self) -> Option<Arc<dyn SampleCounterSource>> {
        let session = self.session().ok()?;
        Some(session.shared.counter.clone())
    }

    fn underruns(&self) -> u64 {
        self.session.as_ref().map_or(0, |session| {
            session.shared.underruns.load(Ordering::Relaxed)
        })
    }
}

#[cfg(feature = "backend-cpal")]
/// The cpal adapter's construction gate — now the real backend's
/// single entry point (the platform wiring FX4 calls under
/// `cfg(windows)`/`cfg(unix)` per FIX-PLAN §3 F1.1 Route A).
///
/// # Errors
/// Never for the construction itself (a host with no output devices
/// is an honest zero-device backend); the typed failures start at
/// `open` (`audio.device_id_unknown`, `audio.format_unsupported`,
/// `audio.device_build_failed`, `audio.device_start_failed`).
pub fn cpal_backend() -> Result<Box<dyn AudioBackend>, EngineError> {
    Ok(Box::new(CpalBackend::new()))
}

// ---------------------------------------------------------------------------
// Feature OFF: the null-backend fallback (default + headless builds,
// byte-compatible with the pre-admission crate).
// ---------------------------------------------------------------------------

#[cfg(not(feature = "backend-cpal"))]
use crate::null::NullBackend;

/// The cpal adapter's construction gate — the feature-off answer.
///
/// The admission is recorded ([`AUDIO_ADR005_ADMITTED`] is `true`),
/// but this build was compiled without `backend-cpal` (check
/// [`cpal_linked`]): the honest answer is the **null backend** —
/// exactly the pre-admission default behavior, a working
/// headless/CI sink, never a fake success and never a panic. Hosts
/// that want the real device path compile the feature (FX3-A07's CI
/// lane; the FX4 host wiring selects it).
///
/// # Errors
/// None at construction; the null backend's own typed errors apply
/// from `open` onward.
#[cfg(not(feature = "backend-cpal"))]
pub fn cpal_backend() -> Result<Box<dyn AudioBackend>, EngineError> {
    Ok(Box::new(NullBackend::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must as _;

    /// The admission is recorded open: the flip-point discipline's
    /// end state. The gate, the refusal arm and the marker are deleted
    /// WITH the admission (the same-change rule, completed): a drifted
    /// re-introduction of the refusal vocabulary fails loud here and
    /// in the wave report's compile-level pins.
    #[test]
    fn the_admission_is_recorded_open() {
        const {
            assert!(
                AUDIO_ADR005_ADMITTED,
                "the ADR-005 admission (Route A) is recorded"
            );
        };
    }

    /// Default builds: the seam answers a working null backend (the
    /// honest fallback — byte-compatible with the pre-admission
    /// crate).
    #[cfg(not(feature = "backend-cpal"))]
    #[test]
    fn the_default_build_falls_back_to_the_null_backend() {
        let backend = cpal_backend().must();
        let descs = backend.enumerate();
        assert_eq!(descs.len(), 1);
        assert_eq!(descs[0].id, crate::null::NULL_DEVICE_ID);
        assert!(descs[0].name.contains("null"), "{}", descs[0].name);
    }
}
