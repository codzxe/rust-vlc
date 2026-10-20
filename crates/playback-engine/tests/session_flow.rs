//! The engine-level session suite (card 2.C2): the state machine
//! driven end-to-end through [`Engine::poll`] with fake stage objects
//! (scripted demuxer, scripted decoder, collecting sink, virtual
//! clock) — the doc 03 §4.6(f) model exercised through the real
//! executor, not a mock of it.
//!
//! What is proven here (the table-level proofs live in `table.rs`):
//!
//! - the **lifetimes**: happy path to `Ended`, pause/resume, the
//!   rebuffer cycle, short media, the `stop` exit, the failure exits
//!   (open failure, armed decode failure, nothing-decodable), and the
//!   reopen cycle;
//! - **output ordering** (doc 05 §4.3): receipts before the events
//!   they cause, `playback.error` before `state-changed → error`,
//!   `playback.end-of-media` before `state-changed → ended`,
//!   `tracks.list-ready` before the first `state-changed → playing`,
//!   and engine-global strictly-increasing `seq`;
//! - **session epochs** (doc 03 §5 / doc 05 §4.2): generation bump at
//!   stop accept time with the NEW gen in the receipt, stale-handle
//!   commands rejected typed, superseded pipeline events suppressed
//!   at drain, cross-epoch reopen with fresh epochs;
//! - **pipeline wiring facts** (demux → queues → decoders → sinks):
//!   priming by the queue watermark, bounded per-tick decode, the
//!   end-of-stream flush-to-drain, backpressure parking that never
//!   drops fresh data, discontinuity/damage observability, and the
//!   doc 03 §5 point-5 checkpoint never dropping a live-generation
//!   frame.

use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
use commands::envelope::{Generation, RequestId};
use commands::{EventKind, SessionId};
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
use playback_engine::state::{RejectionReason, SessionState};
use playback_engine::{
    CommandReceipt, DecoderProvider, Engine, EngineEvent, EngineEventKind, EngineOutput,
    FrameConsumer, QueuePlan, ReceiptResult, SessionCommandKind, SessionConfig, SessionError,
    SessionHandle, TickBudget,
};
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

// ---------------------------------------------------------------------------
// Fakes
// ---------------------------------------------------------------------------

/// Shared virtual monotonic time (doc 03 §8: no sleeps).
struct SharedNow(AtomicU64);

impl MonotonicSource for SharedNow {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// The collecting sink: every frame the engine delivered, with its
/// track, in delivery order.
#[derive(Default)]
struct Collector {
    frames: Mutex<Vec<(TrackId, Frame)>>,
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
struct FakeDemuxer {
    tracks: Vec<Track>,
    script: Arc<Mutex<VecDeque<DemuxEvent>>>,
    stall: Arc<Mutex<bool>>,
    fail_start: Option<EngineError>,
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
        // Seek is Phase 3 (card 3.A1): the v0 engine never calls this.
        Err(EngineError::new(
            PipelineStage::Demux,
            ErrorKind::Unsupported { format: None },
            media_types::error::ErrorContext::new("demux.seek.not-implemented"),
        ))
    }
}

/// The scripted decode stage: one frame out per packet, the codec-api
/// stale-packet guard, and an optional one-shot decode failure.
struct FakeDecoder {
    generation: GenerationId,
    fail_decode: Option<EngineError>,
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
struct FakeProvider {
    fail_select: Option<EngineError>,
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
fn shallow_clone(error: &EngineError) -> EngineError {
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

fn base() -> TimeBase {
    TimeBase::new(1, 1_000_000).must()
}

fn at_micros(us: i64) -> Stamp {
    Stamp::Present(Timestamp::new(us, base()))
}

fn track(id: u32, kind: MediaKind) -> Track {
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

fn packet(track: TrackId, index: u64) -> Packet {
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

fn audio_frame(pts: Stamp, generation: GenerationId) -> Frame {
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

fn inaccessible(code: &'static str) -> EngineError {
    EngineError::new(
        PipelineStage::Open,
        ErrorKind::Inaccessible {
            reason: AccessDenied::NotFound,
        },
        media_types::error::ErrorContext::new(code),
    )
}

fn decoder_error(code: &'static str) -> EngineError {
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
fn tiny_queues() -> QueuePlan {
    let limits = QueueLimits {
        max_packets: NonZeroUsize::new(16).must(),
        max_bytes: NonZeroUsize::new(1 << 20).must(),
        max_duration: Nanoseconds(1_000_000_000),
    };
    QueuePlan::new(limits, limits)
}

fn tiny_budget() -> TickBudget {
    TickBudget {
        demux_events: 64,
        packets_per_limb: 4,
        flush_calls_per_limb: 8,
    }
}

fn virtual_clock() -> Arc<media_clock::MediaClock> {
    Arc::new(
        media_clock::MediaClock::system_monotonic(
            Arc::new(SharedNow(AtomicU64::new(0))),
            Timestamp::new(0, base()),
            media_types::Rate::ONE,
        )
        .must(),
    )
}

fn config_with(
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
    config
}

/// Everything one test needs to drive a session.
struct Fixture {
    engine: Engine,
    handle: SessionHandle,
    collector: Arc<Collector>,
    clock: Arc<media_clock::MediaClock>,
    script: Arc<Mutex<VecDeque<DemuxEvent>>>,
    stall: Arc<Mutex<bool>>,
}

fn open_with(
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

fn open_audio(script_len: u64) -> Fixture {
    let mut script: Vec<DemuxEvent> = (0..script_len)
        .map(|index| DemuxEvent::Packet(packet(TrackId::new(1), index)))
        .collect();
    script.push(DemuxEvent::EndOfStream);
    open_with(vec![track(1, MediaKind::Audio)], script, None, None)
}

/// `preload` packets then a producer stall: the underrun shape (the
/// demuxer can later refill — the stream has NOT ended).
fn open_stalled_audio(preload: u64) -> Fixture {
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
fn poll_until(
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

fn take_events(fixture: &mut Fixture) -> Vec<EngineOutput> {
    fixture.engine.take_outputs()
}

fn frames_of(collector: &Collector) -> usize {
    collector
        .frames
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .len()
}

fn receipt_for(outputs: &[EngineOutput], request: RequestId) -> CommandReceipt {
    outputs
        .iter()
        .find_map(|output| match output {
            EngineOutput::Receipt(receipt) if receipt.request == request => Some(*receipt),
            _ => None,
        })
        .must()
}

// ---------------------------------------------------------------------------
// Lifetimes
// ---------------------------------------------------------------------------

#[test]
fn the_happy_path_reaches_ended_with_the_documented_event_chain() {
    let mut fixture = open_audio(40);
    // poll 1: the open step (probe + wiring) moves us to Buffering.
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Buffering);
    // polls: priming (12 resident packets) moves us to Playing.
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    // polls: the script exhausts, queues drain, decoders flush.
    poll_until(&mut fixture, |s| s == SessionState::Ended, 40, "drain");

    let outputs = take_events(&mut fixture);
    let wire: Vec<EventKind> = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Event(event) => event.as_wire_event().map(|wire| wire.kind),
            EngineOutput::Receipt(_) => None,
        })
        .collect();
    let state_changes: Vec<(commands::PlaybackState, commands::PlaybackState)> = wire
        .iter()
        .filter_map(|kind| match kind {
            EventKind::SessionStateChanged { from, to } => Some((*from, *to)),
            _ => None,
        })
        .collect();
    assert_eq!(
        state_changes,
        vec![
            (
                commands::PlaybackState::Idle,
                commands::PlaybackState::Opening
            ),
            (
                commands::PlaybackState::Opening,
                commands::PlaybackState::Buffering
            ),
            (
                commands::PlaybackState::Buffering,
                commands::PlaybackState::Playing
            ),
            (
                commands::PlaybackState::Playing,
                commands::PlaybackState::Ended
            ),
        ]
    );
    // end-of-media precedes state-changed -> ended (doc 05 §3.2).
    let end_index = wire
        .iter()
        .position(|kind| matches!(kind, EventKind::PlaybackEndOfMedia))
        .must();
    let ended_index = wire
        .iter()
        .position(|kind| {
            matches!(
                kind,
                EventKind::SessionStateChanged {
                    to: commands::PlaybackState::Ended,
                    ..
                }
            )
        })
        .must();
    assert!(end_index < ended_index);
    // tracks.list-ready precedes the first state-changed -> playing
    // (doc 05 §4.3 terminal invariant 4).
    let tracks_index = wire
        .iter()
        .position(|kind| matches!(kind, EventKind::TracksListReady { .. }))
        .must();
    let playing_index = wire
        .iter()
        .position(|kind| {
            matches!(
                kind,
                EventKind::SessionStateChanged {
                    to: commands::PlaybackState::Playing,
                    ..
                }
            )
        })
        .must();
    assert!(tracks_index < playing_index);
    // Buffering episodes bracket with 0.0 and 1.0 (doc 05 §3.2).
    let mut progress: Vec<f32> = wire
        .iter()
        .filter_map(|kind| match kind {
            EventKind::SessionBuffering { progress } => Some(*progress),
            _ => None,
        })
        .collect();
    assert_eq!(progress.len(), 2, "enter + exit only in v0");
    assert_eq!(progress.remove(0), 0.0);
    assert_eq!(progress.remove(0), 1.0);
    // The media's frames all reached the sink: 40 decoded, 0 dropped.
    assert_eq!(frames_of(&fixture.collector), 40);
    let stats = fixture.engine.stats();
    assert_eq!(stats.frames_delivered, 40);
    assert_eq!(stats.stale_frames_dropped, 0, "no live frame is ever stale");
    // The clock was resumed for playing and stopped at the end.
    assert!(fixture.clock.is_paused(), "ended freezes the clock");
}

#[test]
fn pause_and_resume_freeze_and_thaw_the_clock() {
    let mut fixture = open_audio(200);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    let delivered_at_pause = frames_of(&fixture.collector);
    assert!(delivered_at_pause > 0, "buffering ticks already decode");

    fixture.handle.pause(RequestId(1)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Paused);
    assert!(fixture.clock.is_paused());

    // While paused the sinks are frozen: polling does not present.
    let mut quiet = true;
    for _ in 0..4 {
        fixture.engine.poll();
        quiet = quiet && frames_of(&fixture.collector) == delivered_at_pause;
    }
    assert!(quiet, "paused sessions do not present frames");

    fixture.handle.play(RequestId(2)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Playing);
    assert!(!fixture.clock.is_paused());
    poll_until(&mut fixture, |s| s != SessionState::Playing, 400, "resume");
}

#[test]
fn an_underrun_rebuffers_and_recovery_primes_again() {
    // Preload 20 packets, then stall the producer: the queue runs low
    // while the demuxer can (later) refill — the rebuffer shape.
    let mut fixture = open_stalled_audio(20);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    // The stall bites: decode drains the queue below the low
    // watermark while eof is still false -> Underrun -> Buffering.
    poll_until(
        &mut fixture,
        |s| s == SessionState::Buffering,
        200,
        "underrun",
    );
    // Release the stall: the script refills, priming resumes.
    let mut script = fixture
        .script
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for index in 20..120 {
        script.push_back(DemuxEvent::Packet(packet(TrackId::new(1), index)));
    }
    script.push_back(DemuxEvent::EndOfStream);
    drop(script);
    *fixture
        .stall
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = false;
    poll_until(&mut fixture, |s| s == SessionState::Playing, 200, "reprime");
    poll_until(&mut fixture, |s| s == SessionState::Ended, 400, "drain");
    // Two buffering episodes: initial + rebuffer.
    let outputs = take_events(&mut fixture);
    let episodes = outputs
        .iter()
        .filter(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::Buffering { progress: 0.0 },
                    ..
                })
            )
        })
        .count();
    assert_eq!(episodes, 2, "initial + rebuffer episodes");
}

#[test]
fn short_media_starts_with_what_exists_and_drains_to_ended() {
    // 3 packets then eof: below the 12-packet high watermark, so the
    // start is the short-media row (buffering -> playing on
    // EofReached), then a quick drain to ended.
    let mut fixture = open_audio(3);
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Buffering);
    poll_until(
        &mut fixture,
        |s| s == SessionState::Playing,
        5,
        "short start",
    );
    poll_until(
        &mut fixture,
        |s| s == SessionState::Ended,
        10,
        "quick drain",
    );
    assert_eq!(frames_of(&fixture.collector), 3);
    // And the buffering episode still closed with 1.0.
    let outputs = take_events(&mut fixture);
    let closed = outputs.iter().any(|output| {
        matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::Buffering { progress: 1.0 },
                ..
            })
        )
    });
    assert!(closed, "the short-media episode still closes with 1.0");
}

#[test]
fn stop_while_playing_tears_down_and_bumps_the_generation() {
    let mut fixture = open_audio(200);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    let frames_before = frames_of(&fixture.collector);

    fixture.handle.stop(RequestId(7)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Stopped);
    // The stop is gen-bumping: the Accepted receipt carries the NEW
    // generation (doc 05 §4.2) — generation 1 after the open's 0.
    let outputs = take_events(&mut fixture);
    let receipt = receipt_for(&outputs, RequestId(7));
    assert_eq!(
        receipt.result,
        ReceiptResult::Accepted {
            session: SessionId(1),
            generation: Generation(1),
        }
    );
    // No further frames are decoded after the stop.
    let mut quiet = true;
    for _ in 0..4 {
        fixture.engine.poll();
        quiet = quiet && frames_of(&fixture.collector) == frames_before;
    }
    assert!(quiet, "stopped sessions are inert");
    // Close from Stopped returns to Idle (class S completion).
    fixture.handle.close(RequestId(8)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Idle);
    let outputs = take_events(&mut fixture);
    let receipt = receipt_for(&outputs, RequestId(8));
    assert_eq!(
        receipt.result,
        ReceiptResult::Completed {
            session: SessionId(1)
        }
    );
}

#[test]
fn close_while_opening_returns_to_idle() {
    // doc 05 §3.3's `opening -> idle` edge: a close racing the open.
    let mut fixture = open_with(
        vec![track(1, MediaKind::Audio)],
        vec![DemuxEvent::EndOfStream],
        None,
        None,
    );
    // The session is Opening before the first poll runs the open step.
    assert_eq!(fixture.engine.state(), SessionState::Opening);
    fixture.handle.close(RequestId(1)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Idle);
}

#[test]
fn an_open_failure_lands_in_error_with_the_fatal_first() {
    let mut fixture = open_with(
        vec![track(1, MediaKind::Audio)],
        vec![],
        Some(inaccessible("open.header.missing")),
        None,
    );
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Error);
    let outputs = take_events(&mut fixture);
    // playback.error precedes state-changed -> error (doc 05 §3.3).
    let fatal_index = outputs
        .iter()
        .position(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::Fatal { .. },
                    ..
                })
            )
        })
        .must();
    let error_index = outputs
        .iter()
        .position(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::StateChanged {
                        to: SessionState::Error,
                        ..
                    },
                    ..
                })
            )
        })
        .must();
    assert!(fatal_index < error_index);
    // The wire form is a fatal playback.error with the typed facts.
    match &outputs[fatal_index] {
        EngineOutput::Event(event) => {
            let wire = event.as_wire_event().must();
            match &wire.kind {
                EventKind::PlaybackError {
                    stage,
                    code,
                    details,
                    fatal,
                    ..
                } => {
                    assert_eq!(*stage, commands::Stage::Input);
                    assert_eq!(*code, commands::ErrorCode::Inaccessible);
                    assert!(matches!(
                        details.get("context").map(String::as_str),
                        Some("open.header.missing")
                    ));
                    assert!(*fatal);
                }
                other => unreachable!("fatal maps to PlaybackError, got {other:?}"),
            }
        }
        EngineOutput::Receipt(_) => unreachable!("the fatal is an event"),
    }
    // Error is inert until close.
    fixture.handle.play(RequestId(2)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Error);
    fixture.handle.close(RequestId(3)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Idle);
}

#[test]
fn an_armed_decode_failure_transitions_to_error_from_playing() {
    // The provider arms every decoder it hands out with a one-shot
    // failure; the first decode call turns fatal from Playing (the
    // selection promised, the decoding did not deliver).
    struct ArmedProvider;
    impl DecoderProvider for ArmedProvider {
        fn decoder_for(
            &self,
            _track: &Track,
            config: &DecoderConfig,
        ) -> Result<Box<dyn Decoder>, EngineError> {
            Ok(Box::new(FakeDecoder {
                generation: config.generation,
                fail_decode: Some(decoder_error("decode.frame.bad")),
            }))
        }
    }
    let mut script: Vec<DemuxEvent> = (0..40)
        .map(|index| DemuxEvent::Packet(packet(TrackId::new(1), index)))
        .collect();
    script.push(DemuxEvent::EndOfStream);
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-armed.media").must(),
        Box::new(FakeDemuxer {
            tracks: vec![track(1, MediaKind::Audio)],
            script: Arc::new(Mutex::new(VecDeque::from(script))),
            stall: Arc::new(Mutex::new(false)),
            fail_start: None,
        }),
        Arc::new(ArmedProvider),
        Arc::new(Collector::default()),
        virtual_clock(),
    );
    config.queues = tiny_queues();
    config.budget = tiny_budget();
    let mut engine = Engine::new();
    let handle = engine.open(config).must();
    // The failure fires during the first decode (Buffering tick): the
    // session lands in Error with the typed cause on the stream.
    let mut state = engine.state();
    for _ in 0..10 {
        if state == SessionState::Error {
            break;
        }
        engine.poll();
        state = engine.state();
    }
    assert_eq!(state, SessionState::Error, "the armed decode failure");
    let _ = handle;
    let outputs = engine.take_outputs();
    let has_fatal = outputs.iter().any(|output| {
        matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::Fatal { error },
                ..
            }) if error.context.code == "decode.frame.bad"
        )
    });
    assert!(has_fatal, "the typed decode failure is on the stream");
}

#[test]
fn nothing_decodable_is_a_typed_config_failure() {
    // A subtitle-only source: no audio/video limb is wireable, and the
    // engine refuses to fabricate playback (typed, not a crash).
    let mut fixture = open_with(
        vec![track(9, MediaKind::Subtitle)],
        vec![DemuxEvent::EndOfStream],
        None,
        None,
    );
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Error);
    let outputs = take_events(&mut fixture);
    let code = outputs
        .iter()
        .find_map(|output| match output {
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::Fatal { error },
                ..
            }) => Some(error.context.code.clone()),
            _ => None,
        })
        .must();
    assert_eq!(code, "session.no-decodable-track");
    // The wire error is Unsupported (the config_error mapping).
    let wire_error = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Event(event) => event.as_wire_event(),
            _ => None,
        })
        .find_map(|wire| match wire.kind {
            EventKind::PlaybackError { code, .. } => Some(code),
            _ => None,
        });
    assert_eq!(wire_error, Some(commands::ErrorCode::Unsupported));
}

// ---------------------------------------------------------------------------
// Epochs and staleness (doc 03 §5 / doc 05 §4.2)
// ---------------------------------------------------------------------------

#[test]
fn a_stale_handle_is_rejected_typed_after_close() {
    let mut fixture = open_audio(40);
    poll_until(&mut fixture, |s| s == SessionState::Ended, 100, "drain");
    let stale = &fixture.handle;
    stale.close(RequestId(11)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Idle);
    // The handle outlived its epoch: submitting is a typed rejection,
    // never a silent drop.
    assert!(matches!(
        stale.play(RequestId(12)),
        Err(SessionError::SessionClosed)
    ));
    assert!(matches!(
        stale.submit(RequestId(13), SessionCommandKind::Pause),
        Err(SessionError::SessionClosed)
    ));
}

#[test]
fn a_second_open_on_a_live_session_is_refused() {
    let mut fixture = open_audio(40);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    let config = config_with(
        vec![track(1, MediaKind::Audio)],
        vec![],
        None,
        None,
        virtual_clock(),
        Arc::new(Collector::default()),
    );
    assert!(matches!(
        fixture.engine.open(config),
        Err(SessionError::SessionExists {
            session: SessionId(1)
        })
    ));
    // The refusal changed nothing.
    assert_eq!(fixture.engine.state(), SessionState::Playing);
}

#[test]
fn reopening_mints_a_fresh_epoch_and_continues_the_seq() {
    let mut fixture = open_audio(10);
    poll_until(&mut fixture, |s| s == SessionState::Ended, 100, "drain");
    let first_seqs: Vec<u64> = take_events(&mut fixture)
        .into_iter()
        .filter_map(|output| match output {
            EngineOutput::Event(event) => Some(event.seq),
            EngineOutput::Receipt(_) => None,
        })
        .collect();
    assert!(!first_seqs.is_empty());
    for pair in first_seqs.windows(2) {
        assert!(
            pair[0] < pair[1],
            "engine-global seq is strictly increasing"
        );
    }
    fixture.handle.close(RequestId(20)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Idle);

    // Reopen ON THE SAME ENGINE: a fresh epoch (session id 2), seq
    // strictly continues engine-globally.
    let mut script: Vec<DemuxEvent> = (0..10)
        .map(|index| DemuxEvent::Packet(packet(TrackId::new(1), index)))
        .collect();
    script.push(DemuxEvent::EndOfStream);
    let config = config_with(
        vec![track(1, MediaKind::Audio)],
        script,
        None,
        None,
        virtual_clock(),
        Arc::new(Collector::default()),
    );
    let second_handle = fixture.engine.open(config).must();
    assert_eq!(
        second_handle.session_id(),
        SessionId(2),
        "the second session is a fresh epoch"
    );
    for _ in 0..100 {
        if fixture.engine.state() == SessionState::Ended {
            break;
        }
        fixture.engine.poll();
    }
    assert_eq!(fixture.engine.state(), SessionState::Ended);
    let second_seqs: Vec<u64> = take_events(&mut fixture)
        .into_iter()
        .filter_map(|output| match output {
            EngineOutput::Event(event) => Some(event.seq),
            EngineOutput::Receipt(_) => None,
        })
        .collect();
    assert!(second_seqs[0] > first_seqs[first_seqs.len() - 1]);
    for pair in second_seqs.windows(2) {
        assert!(pair[0] < pair[1]);
    }
}

#[test]
fn superseded_events_are_suppressed_but_receipts_survive() {
    // Play, pause, stop — all without draining: the not-yet-drained
    // pipeline events of generation 0 are suppressed at drain (doc 05
    // §4.2 engine-side rule); the receipts survive.
    let mut fixture = open_audio(200);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    for _ in 0..3 {
        fixture.engine.poll();
    }
    fixture.handle.pause(RequestId(30)).must();
    fixture.engine.poll();
    fixture.handle.stop(RequestId(31)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Stopped);

    let outputs = take_events(&mut fixture);
    let receipts: Vec<RequestId> = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Receipt(receipt) => Some(receipt.request),
            _ => None,
        })
        .collect();
    assert!(receipts.contains(&RequestId(30)), "receipts always survive");
    assert!(receipts.contains(&RequestId(31)));
    // No generation-0 event survived the bump to generation 1.
    for output in &outputs {
        if let EngineOutput::Event(event) = output {
            assert!(
                event.generation >= Generation(1),
                "stale event leaked: seq {} gen {}",
                event.seq,
                event.generation.0
            );
        }
    }
    assert!(
        fixture.engine.stats().suppressed_stale_events > 0,
        "the suppression is observable"
    );
}

#[test]
fn receipts_precede_the_events_they_cause() {
    let mut fixture = open_audio(200);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    fixture.handle.pause(RequestId(41)).must();
    fixture.engine.poll();
    let outputs = take_events(&mut fixture);
    let receipt_index = outputs
        .iter()
        .position(|output| {
            matches!(
                output,
                EngineOutput::Receipt(CommandReceipt {
                    request: RequestId(41),
                    ..
                })
            )
        })
        .must();
    let paused_index = outputs
        .iter()
        .position(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::StateChanged {
                        to: SessionState::Paused,
                        ..
                    },
                    ..
                })
            )
        })
        .must();
    assert!(
        receipt_index < paused_index,
        "doc 05 §4.3 guarantee 2: the receipt precedes its events"
    );
    // The paused event carries the originating request as origin.
    match &outputs[paused_index] {
        EngineOutput::Event(event) => assert_eq!(event.origin, Some(RequestId(41))),
        EngineOutput::Receipt(_) => unreachable!("positioned an event"),
    }
}

#[test]
fn rejected_commands_answer_typed_receipts_and_change_nothing() {
    let mut fixture = open_audio(200);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    // `close` from Playing is the v1 StopFirst rejection.
    fixture.handle.close(RequestId(50)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Playing, "unchanged");
    let outputs = take_events(&mut fixture);
    let receipt = receipt_for(&outputs, RequestId(50));
    match receipt.result {
        ReceiptResult::Rejected { state, reason } => {
            assert_eq!(state, SessionState::Playing);
            assert_eq!(reason, RejectionReason::StopFirst);
        }
        other => unreachable!("close in playing is rejected, got {other:?}"),
    }
    // `play` while playing is the C8 idempotent confirmation.
    fixture.handle.play(RequestId(51)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Playing);
    let outputs = take_events(&mut fixture);
    let confirmed = outputs.iter().any(|output| {
        matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::StateChanged {
                    from: SessionState::Playing,
                    to: SessionState::Playing,
                },
                ..
            })
        )
    });
    assert!(confirmed, "the self-transition confirms idempotently");
}

// ---------------------------------------------------------------------------
// Pipeline wiring facts
// ---------------------------------------------------------------------------

#[test]
fn demux_marks_are_counted_and_damage_is_reported() {
    let mut script: Vec<DemuxEvent> = vec![
        DemuxEvent::Packet(packet(TrackId::new(1), 0)),
        DemuxEvent::Discontinuity,
        DemuxEvent::Recovered {
            skipped: 2,
            reason: "test",
        },
    ];
    for index in 1..20 {
        script.push(DemuxEvent::Packet(packet(TrackId::new(1), index)));
    }
    script.push(DemuxEvent::EndOfStream);
    let mut fixture = open_with(vec![track(1, MediaKind::Audio)], script, None, None);
    poll_until(&mut fixture, |s| s == SessionState::Ended, 200, "drain");
    let stats = fixture.engine.stats();
    assert_eq!(
        stats.demux_discontinuities, 1,
        "the marked splice is counted"
    );
    // The damage report is engine-private (no wire row) but observable.
    let outputs = take_events(&mut fixture);
    let damaged: Vec<u64> = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::DamagedData { count },
                ..
            }) => Some(*count),
            _ => None,
        })
        .collect();
    assert_eq!(damaged, vec![2], "bounded recovery is observable data");
}

#[test]
fn first_frame_is_reported_and_stays_engine_private() {
    let mut fixture = open_audio(40);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    let outputs = take_events(&mut fixture);
    let mut saw_first_frame = false;
    for output in &outputs {
        if let EngineOutput::Event(EngineEvent {
            kind: EngineEventKind::FirstFrame { position },
            ..
        }) = output
        {
            saw_first_frame = true;
            // The first decoded frame's pts is the first packet's pts.
            match position {
                Stamp::Present(timestamp) => {
                    assert_eq!(timestamp.ticks, 0, "the first frame's presentation time");
                }
                other => unreachable!("first frame reported with a stamp, got {other:?}"),
            }
        }
    }
    assert!(saw_first_frame, "the first frame is reported");
    // Engine-private: every event that reaches the wire is a wire kind
    // — FirstFrame never leaks (doc 05 §4.2 mapping).
    for output in &outputs {
        if let EngineOutput::Event(event) = output {
            let is_first_frame = matches!(event.kind, EngineEventKind::FirstFrame { .. });
            assert!(
                !is_first_frame || event.as_wire_event().is_none(),
                "FirstFrame must not leak onto the wire"
            );
        }
    }
}

#[test]
fn a_full_queue_holds_the_packet_and_never_drops_fresh_data() {
    // 200 packets with a 16-packet queue and a 64-event demux budget:
    // backpressure parks the overflow in `pending` — nothing is lost
    // (doc 03 §4.4 Q3: fresh data is never dropped).
    let mut fixture = open_audio(200);
    poll_until(&mut fixture, |s| s == SessionState::Ended, 2_000, "drain");
    assert_eq!(frames_of(&fixture.collector), 200);
    assert_eq!(fixture.engine.stats().frames_delivered, 200);
}

#[test]
fn taking_outputs_twice_yields_nothing_the_second_time() {
    let mut fixture = open_audio(40);
    fixture.engine.poll();
    let first = take_events(&mut fixture);
    assert!(!first.is_empty());
    assert!(take_events(&mut fixture).is_empty());
}

#[test]
fn the_outbox_drops_oldest_events_at_capacity_and_counts_them() {
    // A tiny outbox: events overflow (oldest dropped, counted); the
    // receipts are pinned and survive.
    let mut script: Vec<DemuxEvent> = (0..200)
        .map(|index| DemuxEvent::Packet(packet(TrackId::new(1), index)))
        .collect();
    script.push(DemuxEvent::EndOfStream);
    let mut config = config_with(
        vec![track(1, MediaKind::Audio)],
        script,
        None,
        None,
        virtual_clock(),
        Arc::new(Collector::default()),
    );
    config.queues = tiny_queues();
    config.budget = tiny_budget();
    let mut engine = playback_engine::Engine::with_outbox_capacity(4);
    let handle = engine.open(config).must();
    // Flood: many polls without draining, until the media settles.
    for _ in 0..400 {
        if engine.state() == SessionState::Ended {
            break;
        }
        engine.poll();
    }
    assert_eq!(engine.state(), SessionState::Ended, "the flood settles");
    handle.close(RequestId(60)).must();
    engine.poll();
    assert_eq!(engine.state(), SessionState::Idle);
    let outputs = engine.take_outputs();
    let has_close_receipt = outputs.iter().any(|output| {
        matches!(
            output,
            EngineOutput::Receipt(CommandReceipt {
                request: RequestId(60),
                ..
            })
        )
    });
    assert!(has_close_receipt, "receipts are never dropped");
    assert!(
        engine.stats().outbox_dropped_events > 0,
        "the overflow is counted: {:?}",
        engine.stats()
    );
}

#[test]
fn the_engine_debug_renders_the_observable_surface() {
    let fixture = open_audio(40);
    let text = format!("{:?}", fixture.engine);
    assert!(text.contains("Engine"), "{text}");
    assert!(text.contains("Opening"), "{text}");
}
