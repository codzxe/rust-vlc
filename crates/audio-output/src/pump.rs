//! The PCM pump: decoded frames → the bounded device queue → the
//! sink, paced by the playback clock (wave 14, issue #1 — the PCM
//! pipeline proof behind the ADR-005 seam; std-only, no `unsafe`, no
//! new dependencies).
//!
//! # The pipeline this proves
//!
//! ```text
//! decoded AudioBuffers ─▶ PcmPump (high-water pacing + policy) ─▶
//!   AudioSink::write ─▶ [device ring] ─▶ render_into (RT callback) ─▶
//!   SampleCounter ─▶ MediaClock::audio_master
//! ```
//!
//! The pump is the producer half of the SPSC discipline: exactly one
//! producer (this), exactly one consumer (the device callback). Its
//! **clock-sync** is the playback-clock integration: the pump paces
//! against the same [`SampleCounterSource`] the media clock reads —
//! `in_flight = written − consumed` — so the device queue is bounded
//! by *media time*, not by retries: a buffer is only handed to the
//! sink while the device is less than `high_water` frames behind
//! (MP §8 bounded queues; doc 03 §4.9's pump-keeps-device-fed shape).
//!
//! # Bounded by construction, policy explicit
//!
//! The high-water check runs **before** the sink is touched, so the
//! queue can never exceed the pacing budget whatever the sink's own
//! capacity. When the budget would be exceeded the configured
//! [`DropPolicy`] decides — a typed backpressure refusal (lossless)
//! or a counted drop (bounded memory, honest loss ledger; the sink
//! still receives an exact in-order prefix). Nothing is ever silently
//! discarded or repeated.
//!
//! # Real-time discipline
//!
//! The pump runs on the **write side** (it may compute, allocate for
//! its own bookkeeping is still avoided — D-1: the push path moves
//! only integers and a `Copy` outcome); it never touches the render
//! callback, which stays [`crate::render::render_into`] alone.
//!
//! # Never
//!
//! Threads (the session thread is Phase-3 wiring, doc 03 §9 — the
//! caller drives), mixing (audio-engine's job), device semantics
//! (the backend's job).

use std::sync::Arc;

use audio_engine::AudioSink;
use audio_engine::buffer::AudioBuffer;
use media_clock::SampleCounterSource;
use media_types::EngineError;

use crate::error;

/// What the pump does when the device is already `high_water` frames
/// behind (the bounded-queue policy — MP §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPolicy {
    /// Refuse the whole buffer with a typed backpressure error
    /// (`audio.pump_backpressure`, [`ErrorKind::ResourceLimit`]); the
    /// caller waits for the playback clock to advance and retries.
    /// The lossless default.
    Backpressure,
    /// Drop the over-water buffer and count it in
    /// [`PcmPump::dropped_frames`] — bounded memory with honest loss
    /// accounting (live/preview paths). The sink still receives an
    /// exact in-order prefix of the decoded stream.
    Drop,
}

/// One [`PcmPump::push`] outcome: frames accepted by the sink, frames
/// dropped by policy, and the device's in-flight depth after the call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PushOutcome {
    /// Whole frames the sink accepted (0 when the policy dropped).
    pub accepted: u64,
    /// Whole frames dropped by policy (0 on every accepted push).
    pub dropped: u64,
    /// Frames written but not yet consumed by the device — always
    /// ≤ the configured high-water mark.
    pub in_flight: u64,
}

/// The pump half of the output pipeline (module docs).
pub struct PcmPump<S: AudioSink> {
    sink: S,
    clock: Option<Arc<dyn SampleCounterSource>>,
    written: u64,
    dropped: u64,
    high_water: u64,
    policy: DropPolicy,
}

impl<S: AudioSink> std::fmt::Debug for PcmPump<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The clock handle is a dyn trait object — record its presence,
        // not its identity.
        f.debug_struct("PcmPump")
            .field("written", &self.written)
            .field("dropped", &self.dropped)
            .field("high_water", &self.high_water)
            .field("policy", &self.policy)
            .field("clock_attached", &self.clock.is_some())
            .finish()
    }
}

impl<S: AudioSink> PcmPump<S> {
    /// A pump over `sink` with the given over-water `policy` and a
    /// `high_water` pacing budget in whole frames (≥ 1).
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.pump_high_water_invalid`: a zero budget could never
    /// accept a frame — refuse the wiring, not the buffers.
    pub fn new(sink: S, policy: DropPolicy, high_water: u64) -> Result<PcmPump<S>, EngineError> {
        if high_water == 0 {
            return Err(error::unsupported("audio.pump_high_water_invalid"));
        }
        Ok(PcmPump {
            sink,
            clock: None,
            written: 0,
            dropped: 0,
            high_water,
            policy,
        })
    }

    /// Attach the playback clock — the open session's
    /// [`SampleCounterSource`](media_clock::SampleCounterSource), the
    /// same handle [`MediaClock::audio_master`](media_clock::MediaClock::audio_master)
    /// reads. Without one the pump assumes the device consumes
    /// nothing (every push past `high_water` hits the policy).
    pub fn attach_clock(&mut self, clock: Arc<dyn SampleCounterSource>) {
        self.clock = Some(clock);
    }

    fn consumed(&self) -> u64 {
        self.clock.as_ref().map_or(0, |c| c.sample_count())
    }

    /// Frames written but not yet consumed by the device
    /// (`written − consumed`; saturating by construction — see the
    /// mismatch guard in [`push`](PcmPump::push)).
    pub fn in_flight(&self) -> u64 {
        self.written.saturating_sub(self.consumed())
    }

    /// Whether a `frames`-frame buffer fits the pacing budget right
    /// now (the drive loop's ready test).
    pub fn fits(&self, frames: u64) -> bool {
        self.in_flight() + frames <= self.high_water
    }

    /// Frames accepted by the sink so far (monotonic).
    pub fn written_frames(&self) -> u64 {
        self.written
    }

    /// Frames dropped by policy so far (monotonic — the honest loss
    /// ledger; only [`DropPolicy::Drop`] ever increments it).
    pub fn dropped_frames(&self) -> u64 {
        self.dropped
    }

    /// The configured high-water mark (whole frames).
    pub fn high_water(&self) -> u64 {
        self.high_water
    }

    /// Push one decoded buffer (whole frames only; the sink's own
    /// format/partial-frame/backpressure rules still apply and
    /// propagate unchanged).
    ///
    /// # Errors
    /// - `audio.partial_frame` — the payload is not whole frames;
    /// - `audio.pump_clock_mismatch` — the attached clock has consumed
    ///   **more** than this pump wrote (a clock from another session:
    ///   refuse the wiring, never pace on a lie);
    /// - `audio.pump_backpressure` — over budget under
    ///   [`DropPolicy::Backpressure`] (retry after the clock advances);
    /// - anything the sink itself refuses (e.g. `audio.ring_full`
    ///   when `high_water` exceeds the sink's own capacity — a wiring
    ///   bug surfaced honestly).
    pub fn push(&mut self, buf: &AudioBuffer) -> Result<PushOutcome, EngineError> {
        let frame_bytes = buf.format.frame_bytes().max(1) as usize;
        let data = buf.data.as_slice();
        if !data.len().is_multiple_of(frame_bytes) {
            return Err(error::partial_frame(
                frame_bytes as u64,
                (data.len() % frame_bytes) as u64,
            ));
        }
        let frames = (data.len() / frame_bytes) as u64;
        let consumed = self.consumed();
        if consumed > self.written {
            // The clock advanced past this pump's writes: it is not
            // this session's truth source.
            return Err(error::unsupported("audio.pump_clock_mismatch"));
        }
        let before = self.written - consumed;
        if before + frames > self.high_water {
            return match self.policy {
                DropPolicy::Backpressure => {
                    Err(error::pump_backpressure(self.high_water, before + frames))
                }
                DropPolicy::Drop => {
                    self.dropped += frames;
                    Ok(PushOutcome {
                        accepted: 0,
                        dropped: frames,
                        in_flight: before,
                    })
                }
            };
        }
        self.sink.write(buf)?;
        self.written += frames;
        Ok(PushOutcome {
            accepted: frames,
            dropped: 0,
            in_flight: self.written - self.consumed(),
        })
    }

    /// The sink (latency queries, drain gates).
    pub fn sink(&self) -> &S {
        &self.sink
    }

    /// The sink, mutably (pause-side integration).
    pub fn sink_mut(&mut self) -> &mut S {
        &mut self.sink
    }

    /// Recover the sink (session teardown).
    pub fn into_sink(self) -> S {
        self.sink
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counter::SampleCounter;
    use crate::null::{NullBackend, NullConfig};
    use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
    use media_types::packet::SharedBytes;
    use media_types::time::Stamp;
    use testkit::Must;

    fn stereo() -> AudioFormat {
        AudioFormat {
            sample: SampleFormat::F32,
            channels: 2,
            layout: AudioLayout::Interleaved,
            rate: 44_100,
        }
    }

    fn frames_buf(format: AudioFormat, frames: usize) -> AudioBuffer {
        AudioBuffer {
            format,
            data: SharedBytes::from_vec(vec![0.25f32.to_le_bytes(); frames * format.channels as usize].concat()),
            pts: Stamp::Missing,
        }
    }

    #[test]
    fn zero_high_water_is_refused_at_construction() {
        let stream = NullBackend::open_stream(&stereo(), 8, NullConfig { capture: false }).must();
        let err = PcmPump::new(stream.sink, DropPolicy::Drop, 0).err().must();
        assert_eq!(err.context.code, "audio.pump_high_water_invalid");
    }

    #[test]
    fn partial_frames_are_refused_before_the_sink_is_touched() {
        let stream = NullBackend::open_stream(&stereo(), 8, NullConfig { capture: false }).must();
        let mut pump = PcmPump::new(stream.sink, DropPolicy::Backpressure, 8).must();
        // 3 samples of stereo f32 = 1.5 frames.
        let buf = AudioBuffer {
            format: stereo(),
            data: SharedBytes::from_vec(vec![0u8; 3 * 4]),
            pts: Stamp::Missing,
        };
        let err = pump.push(&buf).err().must();
        assert_eq!(err.context.code, "audio.partial_frame");
        assert_eq!(pump.written_frames(), 0);
    }

    #[test]
    fn a_foreign_clock_is_a_typed_mismatch() {
        let stream = NullBackend::open_stream(&stereo(), 8, NullConfig { capture: false }).must();
        let mut pump = PcmPump::new(stream.sink, DropPolicy::Backpressure, 8).must();
        // A counter that already consumed 100 instants this pump never
        // wrote: wiring bug, refused by name.
        let foreign: Arc<dyn SampleCounterSource> =
            Arc::new(SampleCounter::with_base(100, 44_100).must());
        pump.attach_clock(foreign);
        let err = pump.push(&frames_buf(stereo(), 1)).err().must();
        assert_eq!(err.context.code, "audio.pump_clock_mismatch");
    }

    #[test]
    fn drop_policy_counts_and_backpressure_is_typed() {
        let format = stereo();
        let stream = NullBackend::open_stream(&format, 8, NullConfig { capture: false }).must();
        let mut pump = PcmPump::new(stream.sink, DropPolicy::Drop, 4).must();
        // No clock, no renders: the device consumes nothing.
        assert!(pump.push(&frames_buf(format, 2)).must().accepted == 2);
        assert!(pump.push(&frames_buf(format, 2)).must().accepted == 2);
        assert_eq!(pump.in_flight(), 4, "at the high-water mark");
        let outcome = pump.push(&frames_buf(format, 2)).must();
        assert_eq!(
            (outcome.accepted, outcome.dropped, outcome.in_flight),
            (0, 2, 4),
            "over-water buffers drop, counted, never silently"
        );
        assert_eq!(pump.dropped_frames(), 2);
        assert_eq!(pump.written_frames(), 4);
        assert!(!pump.fits(1));

        // The same state under Backpressure is the typed refusal.
        let stream = NullBackend::open_stream(&format, 8, NullConfig { capture: false }).must();
        let mut pump = PcmPump::new(stream.sink, DropPolicy::Backpressure, 4).must();
        pump.push(&frames_buf(format, 4)).must();
        let err = pump.push(&frames_buf(format, 2)).err().must();
        assert_eq!(err.context.code, "audio.pump_backpressure");
        assert!(matches!(
            err.kind,
            media_types::ErrorKind::ResourceLimit { .. }
        ));
        assert_eq!(pump.written_frames(), 4, "the refused buffer never counted");
    }
}
