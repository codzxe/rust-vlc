//! The null backend: the always-available, CI-safe audio output
//! (card 2.C3; doc 01 §2.6 "a null backend for tests/headless";
//! doc 03 §4.9 f "`NullSink` … for determinism").
//!
//! # Shape
//!
//! [`NullBackend::open_stream`] splits the device into its two
//! SPSC halves explicitly:
//!
//! - [`NullStream::sink`] — the [`audio_engine::AudioSink`] the pump
//!   writes through (producer side of the ring);
//! - [`NullStream::device`] — the driver the test/integration owns
//!   (consumer side): it invokes the real-time callback exactly as a
//!   device would ([`NullDevice::render`]), records underruns,
//!   advances the sample counter, and (configured with
//!   [`NullConfig::capture`]) records every byte "played" for the
//!   sink-buffer hash AC.
//!
//! The [`AudioBackend`] trait path (`open`) parks the driver inside
//! the backend — the sink is live (backpressure, counter, pause) but
//! nobody drains it, so an undriven session fills its ring and
//! refuses further writes with a typed `audio.ring_full` (bounded
//! memory, honest state). Code that needs driven audio extracts the
//! driver with [`NullBackend::take_device`]; code that only needs a
//! sink (headless discard) uses the trait as-is. The v0 thread model
//! is manual driving on the caller's thread (doc 03 §9 — the device
//! thread emulation joins Phase-3 session wiring, same call the 2.C4
//! scheduler made).
//!
//! A doc-03 deviation, recorded: the doc places `CapturingSink` in
//! testkit, but testkit is dev-only (MP §3.7) and the 2.E1
//! integration must capture through the **production** backend seam
//! — so capture is a null-device configuration instead `[PROPOSAL:
//! Lead ratifies the relocation]`.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use audio_engine::AudioSink;
use audio_engine::buffer::AudioBuffer;
use media_clock::SampleCounterSource;
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
use media_types::time::{Duration, Nanoseconds};

use crate::counter::SampleCounter;
use crate::device::{AudioBackend, DeviceDesc, DeviceId};
use crate::error;
use crate::render::{RenderStats, render_into};
use crate::ring::SpscRing;
use media_types::EngineError;

/// The null device's identifier (the only one it enumerates).
pub const NULL_DEVICE_ID: DeviceId = DeviceId::new(0);

/// State shared by the sink half (producer) and the device half
/// (consumer): the ring, the clock counters, the pause flag. All
/// single-word atomics — no locks anywhere (the no-blocking
/// contract's structural half).
#[derive(Debug)]
struct Shared {
    ring: SpscRing,
    counter: Arc<SampleCounter>,
    underruns: AtomicU64,
    paused: AtomicBool,
}

impl Shared {
    fn open(format: &AudioFormat, buffer_frames: usize) -> Result<Shared, EngineError> {
        // v0 devices speak interleaved f32 (doc 03 §4.9's callback
        // payload type); anything else is a typed Device error.
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
        })
    }
}

/// Null-stream configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NullConfig {
    /// Record every rendered byte (data **and** underrun silence) for
    /// retrieval via [`NullDevice::take_capture`] — the sink-buffer
    /// hash AC's observation point. Off for headless discard.
    pub capture: bool,
}

/// A driven null stream: the sink half and the device half of one
/// output session.
#[derive(Debug)]
pub struct NullStream {
    /// The pump's half (writes, latency, clock source).
    pub sink: NullSink,
    /// The device's half (renders, underruns, capture).
    pub device: NullDevice,
}

/// The producer half of a null session (an [`AudioSink`]).
#[derive(Debug)]
pub struct NullSink {
    shared: Arc<Shared>,
    format: AudioFormat,
}

impl NullSink {
    /// The clock's truth source for this session (the atomic device
    /// counter — doc 03 §4.9 "the clock's truth source").
    pub fn clock_source(&self) -> Arc<dyn SampleCounterSource> {
        self.shared.counter.clone()
    }

    /// Underrun events so far (monotonic).
    pub fn underruns(&self) -> u64 {
        self.shared.underruns.load(Ordering::Relaxed)
    }

    /// Sample instants consumed so far (monotonic while running,
    /// frozen while paused).
    pub fn sample_count(&self) -> u64 {
        self.shared.counter.count()
    }

    /// Write raw interleaved f32 **bytes** (the convenience the AC
    /// suites use; the same whole-frames/backpressure contract as
    /// [`AudioSink::write`]).
    ///
    /// # Errors
    /// As [`AudioSink::write`] (`audio.partial_frame`,
    /// `audio.ring_full`).
    pub fn write_bytes(&mut self, interleaved: &[u8]) -> Result<(), EngineError> {
        let frame_bytes = self.format.frame_bytes() as usize;
        if !interleaved.len().is_multiple_of(frame_bytes) {
            return Err(error::partial_frame(
                frame_bytes as u64,
                (interleaved.len() % frame_bytes) as u64,
            ));
        }
        let frames = (interleaved.len() / frame_bytes) as u64;
        let free = self.shared.ring.free_frames() as u64;
        if free < frames {
            return Err(error::ring_full(
                self.shared.ring.capacity_frames() as u64,
                frames,
            ));
        }
        let samples: Vec<f32> = interleaved
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c))
            .collect();
        let pushed = self.shared.ring.push(&samples);
        if pushed as u64 != frames {
            // Cannot happen under the free-check above (single
            // producer); if it ever does, refuse honestly rather
            // than drop data.
            return Err(error::ring_full(
                self.shared.ring.capacity_frames() as u64,
                frames,
            ));
        }
        Ok(())
    }
}

impl AudioSink for NullSink {
    fn write(&mut self, buf: &AudioBuffer) -> Result<(), EngineError> {
        if buf.format != self.format {
            return Err(error::unsupported("audio.format_mismatch"));
        }
        let frame_bytes = self.format.frame_bytes() as usize;
        let data = buf.data.as_slice();
        if !data.len().is_multiple_of(frame_bytes) {
            return Err(error::partial_frame(
                frame_bytes as u64,
                (data.len() % frame_bytes) as u64,
            ));
        }
        self.write_bytes(data)
    }

    fn latency(&self) -> Option<Duration> {
        let frames = self.shared.ring.available_frames() as u128;
        let nanos = frames * 1_000_000_000u128 / u128::from(self.format.rate);
        let nanos = u64::try_from(nanos).ok()?;
        Some(Duration::Known(Nanoseconds(nanos)))
    }
}

/// The consumer half of a null session: the device driver.
///
/// Driving is manual and synchronous in v0 — call
/// [`render`](NullDevice::render) with a device-sized buffer exactly
/// as an OS callback would. Pause ([`NullSink`]'s backend pause)
/// freezes the device: paused renders are no-ops (no callback, no
/// counter advance, no capture — a real paused device pulls nothing).
#[derive(Debug)]
pub struct NullDevice {
    shared: Arc<Shared>,
    channels: usize,
    capture: Option<Vec<u8>>,
}

impl NullDevice {
    /// Render one device buffer: the real-time callback runs
    /// ([`render_into`]), the counter advances by the frames
    /// consumed, underruns are counted, and the rendered bytes are
    /// captured when configured (driver-side, **after** the callback
    /// — capture never runs under the real-time rules).
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.render_buffer_unaligned`: `out` must be whole frames.
    pub fn render(&mut self, out: &mut [f32]) -> Result<RenderStats, EngineError> {
        if !out.len().is_multiple_of(self.channels) {
            return Err(error::unsupported("audio.render_buffer_unaligned"));
        }
        if self.shared.paused.load(Ordering::Acquire) {
            // The device is not pulling: no callback, no counter, no
            // capture (doc 03 §4.7 e — paused freezes the counter).
            return Ok(RenderStats::default());
        }
        let stats = render_into(
            &self.shared.ring,
            &self.shared.counter,
            &self.shared.underruns,
            out,
        );
        if let Some(capture) = self.capture.as_mut() {
            for sample in out.iter() {
                capture.extend_from_slice(&sample.to_le_bytes());
            }
        }
        Ok(stats)
    }

    /// Drain and return the captured bytes (empty when capture is
    /// off; cumulative otherwise — capture stays enabled after a
    /// take).
    pub fn take_capture(&mut self) -> Vec<u8> {
        match self.capture.as_mut() {
            Some(buffer) => std::mem::take(buffer),
            None => Vec::new(),
        }
    }

    /// Whether the device is paused.
    pub fn paused(&self) -> bool {
        self.shared.paused.load(Ordering::Acquire)
    }

    /// Underrun events so far.
    pub fn underruns(&self) -> u64 {
        self.shared.underruns.load(Ordering::Relaxed)
    }

    /// Sample instants consumed so far.
    pub fn sample_count(&self) -> u64 {
        self.shared.counter.count()
    }

    /// Frames currently sitting in the ring.
    pub fn buffered_frames(&self) -> usize {
        self.shared.ring.available_frames()
    }
}

/// The null backend: enumerates one virtual device, opens sessions
/// that discard or capture audio, and never touches a real OS
/// device. CI-safe by construction.
#[derive(Debug, Default)]
pub struct NullBackend {
    session: Option<(Arc<Shared>, AudioFormat)>,
    device: Option<NullDevice>,
}

impl NullBackend {
    /// A fresh null backend.
    pub fn new() -> NullBackend {
        NullBackend::default()
    }

    /// Open a **driven** null stream (both halves; see the module
    /// docs). This is the constructor the AC suites and the 2.E1
    /// integration use; the trait's `open` parks the driver instead.
    ///
    /// # Errors
    /// [`Device`](media_types::ErrorKind::Device) with code
    /// `audio.format_unsupported` (v0 devices speak interleaved f32
    /// only) or [`Unsupported`](media_types::ErrorKind::Unsupported)
    /// `audio.ring_shape_invalid` (non-power-of-two buffer).
    pub fn open_stream(
        format: &AudioFormat,
        buffer_frames: usize,
        config: NullConfig,
    ) -> Result<NullStream, EngineError> {
        let shared = Arc::new(Shared::open(format, buffer_frames)?);
        Ok(NullStream {
            sink: NullSink {
                shared: shared.clone(),
                format: *format,
            },
            device: NullDevice {
                shared,
                channels: format.channels as usize,
                capture: config.capture.then(Vec::new),
            },
        })
    }

    /// Extract the parked device driver (trait-`open` mode). `None`
    /// when not open or already taken.
    pub fn take_device(&mut self) -> Option<NullDevice> {
        self.device.take()
    }

    /// Whether a session is open.
    pub fn is_open(&self) -> bool {
        self.session.is_some()
    }

    fn shared(&self) -> Result<&Arc<Shared>, EngineError> {
        self.session
            .as_ref()
            .map(|(shared, _)| shared)
            .ok_or_else(|| error::unsupported("audio.sink_not_open"))
    }
}

impl AudioSink for NullBackend {
    fn write(&mut self, buf: &AudioBuffer) -> Result<(), EngineError> {
        let shared = self.shared()?;
        let format = self
            .session
            .as_ref()
            .map(|(_, f)| *f)
            .ok_or_else(|| error::unsupported("audio.sink_not_open"))?;
        let mut sink = NullSink {
            shared: shared.clone(),
            format,
        };
        sink.write(buf)
    }

    fn latency(&self) -> Option<Duration> {
        let (shared, format) = self.session.as_ref()?;
        let sink = NullSink {
            shared: shared.clone(),
            format: *format,
        };
        sink.latency()
    }
}

impl AudioBackend for NullBackend {
    fn enumerate(&self) -> Vec<DeviceDesc> {
        vec![DeviceDesc {
            id: NULL_DEVICE_ID,
            name: "null (headless discard)".to_string(),
            preferred: AudioFormat {
                sample: SampleFormat::F32,
                channels: 2,
                layout: AudioLayout::Interleaved,
                rate: 48_000,
            },
        }]
    }

    fn open(
        &mut self,
        device: DeviceId,
        format: &AudioFormat,
        buffer_frames: usize,
    ) -> Result<(), EngineError> {
        if device != NULL_DEVICE_ID {
            return Err(error::unsupported("audio.device_id_unknown"));
        }
        let shared = Arc::new(Shared::open(format, buffer_frames)?);
        // The driver is parked inside the backend (discard mode);
        // `take_device` hands it to whoever wants to drive.
        self.device = Some(NullDevice {
            shared: shared.clone(),
            channels: format.channels as usize,
            capture: None,
        });
        self.session = Some((shared, *format));
        Ok(())
    }

    fn pause(&mut self) -> Result<(), EngineError> {
        self.shared()?.paused.store(true, Ordering::Release);
        Ok(())
    }

    fn resume(&mut self) -> Result<(), EngineError> {
        self.shared()?.paused.store(false, Ordering::Release);
        Ok(())
    }

    fn clock_source(&self) -> Option<Arc<dyn SampleCounterSource>> {
        let (shared, _) = self.session.as_ref()?;
        Some(shared.counter.clone())
    }

    fn underruns(&self) -> u64 {
        self.session
            .as_ref()
            .map_or(0, |(shared, _)| shared.underruns.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use media_types::packet::SharedBytes;
    use media_types::time::Stamp;
    use testkit::Must;

    fn f32_format(channels: u32, rate: u32) -> AudioFormat {
        AudioFormat {
            sample: SampleFormat::F32,
            channels,
            layout: AudioLayout::Interleaved,
            rate,
        }
    }

    fn buffer_of(format: AudioFormat, samples: &[f32]) -> AudioBuffer {
        let mut data = Vec::new();
        for s in samples {
            data.extend_from_slice(&s.to_le_bytes());
        }
        AudioBuffer {
            format,
            data: SharedBytes::from_vec(data),
            pts: Stamp::Missing,
        }
    }

    #[test]
    fn open_stream_roundtrip_and_capture() {
        let format = f32_format(2, 44_100);
        let mut stream = NullBackend::open_stream(&format, 8, NullConfig { capture: true }).must();
        stream
            .sink
            .write(&buffer_of(format, &[0.5, -0.5, 0.25, -0.25]))
            .must();
        let mut out = vec![f32::NAN; 4];
        let stats = stream.device.render(&mut out).must();
        assert_eq!(stats.frames_read, 2);
        assert!(stats.fully_served());
        assert_eq!(out, vec![0.5, -0.5, 0.25, -0.25]);
        let captured = stream.device.take_capture();
        let mut expected = Vec::new();
        for s in [0.5f32, -0.5, 0.25, -0.25] {
            expected.extend_from_slice(&s.to_le_bytes());
        }
        assert_eq!(captured, expected);
        assert_eq!(stream.device.take_capture().len(), 0, "take drains");
    }

    #[test]
    fn underruns_are_counted_and_silence_captured() {
        let format = f32_format(1, 44_100);
        let mut stream = NullBackend::open_stream(&format, 8, NullConfig { capture: true }).must();
        stream.sink.write(&buffer_of(format, &[0.5])).must();
        let mut out = vec![f32::NAN; 3];
        let stats = stream.device.render(&mut out).must();
        assert_eq!(stats.silence_filled, 2);
        assert!(stats.underrun);
        assert_eq!(stream.device.underruns(), 1);
        assert_eq!(stream.sink.underruns(), 1);
        assert_eq!(out, vec![0.5, 0.0, 0.0]);
        // Silence is part of what "played" means — captured too.
        let mut expected = Vec::new();
        for s in [0.5f32, 0.0, 0.0] {
            expected.extend_from_slice(&s.to_le_bytes());
        }
        assert_eq!(stream.device.take_capture(), expected);
        // The counter counts data + silence.
        assert_eq!(stream.device.sample_count(), 3);
    }

    #[test]
    fn backpressure_is_typed_and_all_or_nothing() {
        let format = f32_format(1, 44_100);
        let mut stream = NullBackend::open_stream(&format, 4, NullConfig { capture: false }).must();
        stream
            .sink
            .write(&buffer_of(format, &[1.0, 2.0, 3.0, 4.0]))
            .must();
        let err = stream
            .sink
            .write(&buffer_of(format, &[9.0, 9.0, 9.0]))
            .err()
            .must();
        assert_eq!(err.context.code, "audio.ring_full");
        // Nothing was written by the failed attempt: FIFO intact.
        let mut out = vec![0.0; 4];
        let stats = stream.device.render(&mut out).must();
        assert!(stats.fully_served());
        assert_eq!(out, vec![1.0, 2.0, 3.0, 4.0]);
        // A too-big single buffer is refused whole.
        let err = stream
            .sink
            .write(&buffer_of(format, &[0.0; 8]))
            .err()
            .must();
        assert_eq!(err.context.code, "audio.ring_full");
    }

    #[test]
    fn pause_freezes_counter_and_renders() {
        // The trait path: write, pause (renders become no-ops — no
        // callback, no counter advance, no capture), resume, and the
        // queued frames play in order.
        let format = f32_format(2, 44_100);
        let mut backend = NullBackend::new();
        backend.open(NULL_DEVICE_ID, &format, 8).must();
        backend
            .write(&buffer_of(format, &[0.5, -0.5, 0.25, -0.25]))
            .must();
        backend.pause().must();
        let mut device = backend.take_device().must();
        assert!(device.paused());
        let mut out = vec![f32::NAN; 4];
        let stats = device.render(&mut out).must();
        assert_eq!(stats, RenderStats::default());
        assert!(out.iter().all(|v| v.is_nan()), "paused touches nothing");
        assert_eq!(device.sample_count(), 0);
        assert_eq!(backend.underruns(), 0);
        // Resume: the same frames are still queued — pause preserved
        // them (playback position preserved, doc 03 §4.9 d).
        backend.resume().must();
        let stats = device.render(&mut out).must();
        assert_eq!(stats.frames_read, 2);
        assert_eq!(out, vec![0.5, -0.5, 0.25, -0.25]);
        assert_eq!(device.sample_count(), 2);
        // The clock-source view (what media-clock reads) is the same
        // shared counter the device half advances.
        let clock = backend.clock_source().must();
        let source: &dyn SampleCounterSource = clock.as_ref();
        assert_eq!(source.sample_count(), 2);
        assert_eq!(source.sample_rate_hz(), 44_100);
    }

    #[test]
    fn stream_mode_shares_the_counter_between_the_halves() {
        // The split-halves (open_stream) mode: sink-side and
        // device-side views of the counter, underruns and clock source
        // are one shared state — the SPSC session, not three copies.
        let format = f32_format(2, 44_100);
        let mut stream = NullBackend::open_stream(&format, 8, NullConfig { capture: false }).must();
        let clock = stream.sink.clock_source();
        let source: &dyn SampleCounterSource = clock.as_ref();
        assert_eq!(source.sample_count(), 0);
        stream
            .sink
            .write(&buffer_of(format, &[0.5, -0.5, 0.25, -0.25]))
            .must();
        let mut out = vec![f32::NAN; 4];
        stream.device.render(&mut out).must();
        assert_eq!(stream.sink.sample_count(), 2);
        assert_eq!(stream.device.sample_count(), 2);
        assert_eq!(
            source.sample_count(),
            2,
            "the clock sees the device's truth"
        );
        assert_eq!(stream.sink.underruns(), stream.device.underruns());
    }

    #[test]
    fn format_and_shape_errors_are_typed() {
        // Non-f32 formats are refused at open.
        let s16 = AudioFormat {
            sample: SampleFormat::S16,
            channels: 2,
            layout: AudioLayout::Interleaved,
            rate: 44_100,
        };
        let err = NullBackend::open_stream(&s16, 8, NullConfig { capture: false })
            .err()
            .must();
        assert_eq!(err.context.code, "audio.format_unsupported");
        // Planar f32 too.
        let planar = AudioFormat {
            sample: SampleFormat::F32,
            channels: 2,
            layout: AudioLayout::Planar,
            rate: 44_100,
        };
        let err = NullBackend::open_stream(&planar, 8, NullConfig { capture: false })
            .err()
            .must();
        assert_eq!(err.context.code, "audio.format_unsupported");
        // Non-power-of-two ring.
        let err =
            NullBackend::open_stream(&f32_format(2, 44_100), 7, NullConfig { capture: false })
                .err()
                .must();
        assert_eq!(err.context.code, "audio.ring_shape_invalid");
        // Unknown device on the trait path.
        let mut backend = NullBackend::new();
        let err = backend
            .open(DeviceId::new(99), &f32_format(2, 44_100), 8)
            .err()
            .must();
        assert_eq!(err.context.code, "audio.device_id_unknown");
        // Not-open errors.
        let err = backend.pause().err().must();
        assert_eq!(err.context.code, "audio.sink_not_open");
        // A mismatched buffer format is refused.
        let mut backend = NullBackend::new();
        backend
            .open(NULL_DEVICE_ID, &f32_format(2, 44_100), 8)
            .must();
        let err = backend
            .write(&buffer_of(f32_format(2, 48_000), &[0.0, 0.0]))
            .err()
            .must();
        assert_eq!(err.context.code, "audio.format_mismatch");
        // Unaligned render buffer.
        let mut device = backend.take_device().must();
        let mut out = [0.0f32; 3];
        let err = device.render(&mut out).err().must();
        assert_eq!(err.context.code, "audio.render_buffer_unaligned");
        // Partial-frame buffer.
        let mut stream =
            NullBackend::open_stream(&f32_format(2, 44_100), 8, NullConfig { capture: false })
                .must();
        let err = stream.sink.write_bytes(&[0, 0, 0]).err().must();
        assert_eq!(err.context.code, "audio.partial_frame");
    }

    #[test]
    fn latency_reports_the_ring_fill() {
        let format = f32_format(1, 8_000);
        let mut backend = NullBackend::new();
        assert_eq!(backend.latency(), None, "not open yet");
        backend.open(NULL_DEVICE_ID, &format, 16).must();
        assert_eq!(backend.latency(), Some(Duration::Known(Nanoseconds(0))));
        // 16 frames at 8 kHz = exactly 2 ms.
        backend.write(&buffer_of(format, &[0.5; 16])).must();
        let latency = backend.latency().must();
        assert_eq!(latency, Duration::Known(Nanoseconds(2_000_000)));
        // The enumerate desc is present and redacted (no OS strings).
        let devices = backend.enumerate();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].id, NULL_DEVICE_ID);
        assert!(devices[0].name.contains("null"), "{}", devices[0].name);
    }

    #[test]
    fn renderer_callback_composes_over_the_shared_state() {
        // The production RenderCallback shape runs over the same
        // shared state the driver uses (doc 03 §4.9's `render`).
        let format = f32_format(2, 44_100);
        let mut stream = NullBackend::open_stream(&format, 8, NullConfig { capture: false }).must();
        let mut bytes = Vec::new();
        for s in [0.5f32, -0.5] {
            bytes.extend_from_slice(&s.to_le_bytes());
        }
        stream.sink.write_bytes(&bytes).must();
        let mut out = vec![0.0; 2];
        let stats = stream.device.render(&mut out).must();
        assert!(stats.fully_served());
        assert_eq!(out, vec![0.5, -0.5]);
    }
}
