//! The shared engine-test harness (card 3.C1, hoisted verbatim from
//! `tests/session_flow.rs` — the design's step 1: the fake stage
//! objects, builders, and poll drivers the session suite and the
//! model-walk suite both drive the real [`Engine`] with. No behavior
//! change; `session_flow.rs` now consumes it as `mod common`).
//!
//! The shape (doc 03 §4.6(f) — the model exercised through the real
//! executor, never a mock of it):
//!
//! - [`SharedNow`] — a virtual monotonic clock source (no sleeps);
//! - [`Collector`] — the collecting sink: every delivered frame, in
//!   delivery order;
//! - [`FakeDemuxer`] — the scripted demux stage (fixed track list,
//!   mutable event script, optional start failure, stall flag);
//! - [`FakeDecoder`]/[`FakeProvider`] — the scripted decode stage and
//!   its seam (one frame per packet, optional one-shot failures);
//! - [`open_with`]/[`open_audio`]/[`open_stalled_audio`] — the
//!   builders wiring a [`Fixture`] (engine + handle + script + stall);
//! - [`poll_until`] — the bounded hang-proof driver.
//!
//! Everything is `pub` and the module allows dead code: each
//! consuming test crate uses a different subset (the standard
//! `tests/common` discipline).

#![allow(dead_code)]

use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
use media_clock::MonotonicSource;
use media_types::error::{AccessDenied, ErrorKind};
use media_types::frame::{AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::{
    AudioFormat, Duration, EngineError, Frame, GenerationId, MediaKind, Nanoseconds, Packet,
    PipelineStage, SourceUri, Stamp, TimeBase, TimeSpan, Timestamp, Track, TrackFlags, TrackId,
    TrackTiming,
};
use packet_queue::QueueLimits;
use playback_engine::{
    CommandReceipt, DecoderProvider, Engine, EngineOutput, FrameConsumer, QueuePlan, SessionConfig,
    SessionHandle, SessionState, TickBudget,
};
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

// ---------------------------------------------------------------------------
// Fakes
// ---------------------------------------------------------------------------

/// Shared virtual monotonic time (doc 03 §8: no sleeps).
pub struct SharedNow(pub AtomicU64);

impl MonotonicSource for SharedNow {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// The collecting sink: every frame the engine delivered, with its
/// track, in delivery order.
#[derive(Default)]
pub struct Collector {
    pub frames: Mutex<Vec<(TrackId, Frame)>>,
}

impl FrameConsumer for Collector {
    fn accept(&self, frame: Frame, track: TrackId) -> Result<(), EngineError> {
        self.frames
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push((track, frame));
        Ok(())
    }
}

/// The scripted demux stage: a fixed track list, a mutable event
/// script (shared so a test can release a stall mid-run), and an
/// optional start failure. An exhausted script answers end-of-stream;
/// an armed stall answers discontinuity markers — the honest
/// non-blocking shape of "nothing new yet, stream not ended" (the
/// engine counts the marks; the re-anchor policy is Phase 3).
pub struct FakeDemuxer {
    pub tracks: Vec<Track>,
    pub script: Arc<Mutex<VecDeque<DemuxEvent>>>,
    pub stall: Arc<Mutex<bool>>,
    pub fail_start: Option<EngineError>,
}

impl Demuxer for FakeDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        match self.fail_start.take() {
            Some(error) => Err(error),
            None => Ok(self.tracks.clone()),
        }
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        let mut script = self
            .script
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(event) = script.pop_front() {
            return Ok(event);
        }
        drop(script);
        if *self
            .stall
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
        {
            return Ok(DemuxEvent::Discontinuity);
        }
        Ok(DemuxEvent::EndOfStream)
    }

    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
        // The base fake is unseekable (the seek-flow suites carry
        // their own seekable variants — see `tests/seek_flow.rs`).
        Err(EngineError::new(
            PipelineStage::Demux,
            ErrorKind::Unsupported { format: None },
            media_types::error::ErrorContext::new("demux.seek.not-implemented"),
        ))
    }
}

/// The scripted decode stage: one frame out per packet, the codec-api
/// stale-packet guard, and an optional one-shot decode failure.
pub struct FakeDecoder {
    pub generation: GenerationId,
    pub fail_decode: Option<EngineError>,
}

impl Decoder for FakeDecoder {
    fn configure(&mut self, config: &DecoderConfig) -> Result<(), EngineError> {
        self.generation = config.generation;
        Ok(())
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        if let Some(error) = self.fail_decode.take() {
            return Err(error);
        }
        if packet.is_stale(self.generation) {
            return Ok(DecodeOutcome::NeedMoreInput);
        }
        out.accept(audio_frame(packet.pts, self.generation))?;
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        Ok(DecodeOutcome::Drained)
    }
}

/// The decoder seam: answers a working decoder per track, unless the
/// test armed a selection failure (the registry-lied case).
pub struct FakeProvider {
    pub fail_select: Option<EngineError>,
}

impl DecoderProvider for FakeProvider {
    fn decoder_for(
        &self,
        _track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        match &self.fail_select {
            Some(error) => Err(shallow_clone(error)),
            None => Ok(Box::new(FakeDecoder {
                generation: config.generation,
                fail_decode: None,
            })),
        }
    }
}

/// Clone without the boxed `source` chain (the engine never clones
/// errors; the fake's shared provider needs per-call copies).
pub fn shallow_clone(error: &EngineError) -> EngineError {
    EngineError::new(
        error.stage,
        error.kind.clone(),
        media_types::error::ErrorContext {
            code: error.context.code.clone(),
        },
    )
}

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

pub fn base() -> TimeBase {
    TimeBase::new(1, 1_000_000).must()
}

pub fn at_micros(us: i64) -> Stamp {
    Stamp::Present(Timestamp::new(us, base()))
}

pub fn track(id: u32, kind: MediaKind) -> Track {
    Track {
        id: TrackId::new(id),
        stream: media_types::StreamId::new(id),
        kind,
        language: None,
        codec: media_types::ids::CodecDescriptor::new("fake-pcm").must(),
        title: None,
        flags: TrackFlags::NONE,
        timing: TrackTiming {
            span: TimeSpan::UNKNOWN,
            base: base(),
            gaps: Vec::new(),
        },
    }
}

pub fn packet(track: TrackId, index: u64) -> Packet {
    Packet {
        track,
        pts: at_micros((index * 1_000) as i64),
        dts: Stamp::Missing,
        duration: Some(Duration::Known(Nanoseconds(1_000_000))),
        data: SharedBytes::from_static(&[7u8; 64]),
        keyframe: true,
        discontinuity: false,
        generation: GenerationId::FIRST,
    }
}

pub fn audio_frame(pts: Stamp, generation: GenerationId) -> Frame {
    Frame::Audio(media_types::frame::AudioFrame {
        format: AudioFormat {
            sample: SampleFormat::S16,
            channels: 2,
            layout: AudioLayout::Interleaved,
            rate: 48_000,
        },
        samples: SharedBytes::from_static(&[0u8; 4]),
        pts,
        duration: None,
        generation,
    })
}

pub fn inaccessible(code: &'static str) -> EngineError {
    EngineError::new(
        PipelineStage::Open,
        ErrorKind::Inaccessible {
            reason: AccessDenied::NotFound,
        },
        media_types::error::ErrorContext::new(code),
    )
}

pub fn decoder_error(code: &'static str) -> EngineError {
    EngineError::new(
        PipelineStage::Decode,
        ErrorKind::Decoder {
            cause: media_types::error::DecodeCause::Bitstream,
        },
        media_types::error::ErrorContext::new(code),
    )
}

/// Small deterministic queue caps: the packets axis governs — high
/// watermark at 12 resident (cap 16 minus a quarter), low at 4. The
/// tick math below relies on these numbers.
pub fn tiny_queues() -> QueuePlan {
    let limits = QueueLimits {
        max_packets: NonZeroUsize::new(16).must(),
        max_bytes: NonZeroUsize::new(1 << 20).must(),
        max_duration: Nanoseconds(1_000_000_000),
    };
    QueuePlan::new(limits, limits)
}

pub fn tiny_budget() -> TickBudget {
    TickBudget {
        demux_events: 64,
        packets_per_limb: 4,
        flush_calls_per_limb: 8,
    }
}

pub fn virtual_clock() -> Arc<media_clock::MediaClock> {
    Arc::new(
        media_clock::MediaClock::system_monotonic(
            Arc::new(SharedNow(AtomicU64::new(0))),
            Timestamp::new(0, base()),
            media_types::Rate::ONE,
        )
        .must(),
    )
}

/// A frozen virtual throttle source (card 3.B4): every emission
/// window is deterministic — a never-advancing source permits
/// exactly one in-episode buffering emission and never windows a
/// damage flush mid-test (the terminal `flush_all` still fires).
pub fn frozen_throttle() -> playback_engine::EventPolicy {
    playback_engine::EventPolicy {
        tick: playback_engine::TimeTickPolicy::default_policy(),
        throttle: Arc::new(SharedNow(AtomicU64::new(0))),
    }
}

pub fn config_with(
    tracks: Vec<Track>,
    script: Vec<DemuxEvent>,
    fail_start: Option<EngineError>,
    fail_select: Option<EngineError>,
    clock: Arc<media_clock::MediaClock>,
    collector: Arc<Collector>,
) -> SessionConfig {
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-fake.media").must(),
        Box::new(FakeDemuxer {
            tracks,
            script: Arc::new(Mutex::new(VecDeque::from(script))),
            stall: Arc::new(Mutex::new(false)),
            fail_start,
        }),
        Arc::new(FakeProvider { fail_select }),
        collector,
        clock,
    );
    config.queues = tiny_queues();
    config.budget = tiny_budget();
    config.throttle = frozen_throttle();
    config
}

/// Everything one test needs to drive a session.
pub struct Fixture {
    pub engine: Engine,
    pub handle: SessionHandle,
    pub collector: Arc<Collector>,
    pub clock: Arc<media_clock::MediaClock>,
    pub script: Arc<Mutex<VecDeque<DemuxEvent>>>,
    pub stall: Arc<Mutex<bool>>,
}

pub fn open_with(
    tracks: Vec<Track>,
    script: Vec<DemuxEvent>,
    fail_start: Option<EngineError>,
    fail_select: Option<EngineError>,
) -> Fixture {
    let script = Arc::new(Mutex::new(VecDeque::from(script)));
    let stall = Arc::new(Mutex::new(false));
    let collector = Arc::new(Collector::default());
    let clock = virtual_clock();
    let demuxer = FakeDemuxer {
        tracks,
        script: Arc::clone(&script),
        stall: Arc::clone(&stall),
        fail_start,
    };
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-fake.media").must(),
        Box::new(demuxer),
        Arc::new(FakeProvider { fail_select }),
        Arc::clone(&collector) as Arc<dyn FrameConsumer>,
        clock.clone(),
    );
    config.queues = tiny_queues();
    config.budget = tiny_budget();
    config.throttle = frozen_throttle();
    let mut engine = Engine::new();
    let handle = engine.open(config).must();
    Fixture {
        engine,
        handle,
        collector,
        clock,
        script,
        stall,
    }
}

pub fn open_audio(script_len: u64) -> Fixture {
    let mut script: Vec<DemuxEvent> = (0..script_len)
        .map(|index| DemuxEvent::Packet(packet(TrackId::new(1), index)))
        .collect();
    script.push(DemuxEvent::EndOfStream);
    open_with(vec![track(1, MediaKind::Audio)], script, None, None)
}

/// `preload` packets then a producer stall: the underrun shape (the
/// demuxer can later refill — the stream has NOT ended).
pub fn open_stalled_audio(preload: u64) -> Fixture {
    let script: Vec<DemuxEvent> = (0..preload)
        .map(|index| DemuxEvent::Packet(packet(TrackId::new(1), index)))
        .collect();
    let fixture = open_with(vec![track(1, MediaKind::Audio)], script, None, None);
    *fixture
        .stall
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
    fixture
}

/// Drive polls until `predicate` holds on the engine state, or fail
/// loudly after `limit` polls (a hang is a failed test, never a
/// timeout mystery — doc 03 §4.6(f)).
pub fn poll_until(
    fixture: &mut Fixture,
    predicate: impl Fn(SessionState) -> bool,
    limit: usize,
    context: &str,
) {
    for _ in 0..limit {
        if predicate(fixture.engine.state()) {
            return;
        }
        fixture.engine.poll();
    }
    assert!(
        predicate(fixture.engine.state()),
        "{context}: state is {} after {limit} polls",
        fixture.engine.state()
    );
}

pub fn take_events(fixture: &mut Fixture) -> Vec<EngineOutput> {
    fixture.engine.take_outputs()
}

pub fn frames_of(collector: &Collector) -> usize {
    collector
        .frames
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .len()
}

pub fn receipt_for(
    outputs: &[EngineOutput],
    request: commands::envelope::RequestId,
) -> CommandReceipt {
    outputs
        .iter()
        .find_map(|output| match output {
            EngineOutput::Receipt(receipt) if receipt.request == request => Some(*receipt),
            _ => None,
        })
        .must()
}
