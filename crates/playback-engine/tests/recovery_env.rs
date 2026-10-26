//! The environment-level recovery ACs (card 3.C3, doc 13 wave 3.C):
//! the three environment policies of `playback-engine/src/recovery.rs`
//! proven first as **policy** (pure virtual-time schedules, exact
//! nanosecond bounds, typed terminals) and then **through the real
//! engine** (the doc 03 §8 scenario rows, driven end-to-end with the
//! guards wired into fake stage objects — deterministic, no sleeps).
//!
//! # What is proven here
//!
//! **Interrupted network source** (doc 03 §4.1(d), §8 row
//! "interrupted network"):
//! - the retry window's backoff schedule is bounded and exact
//!   (base, doubling, cap, saturating);
//! - a transient loss recovers: the buffering episode closes with
//!   `1.0`, a later loss opens a *fresh* window;
//! - exhaustion answers the **typed network error**
//!   (`Network { ReconnectExhausted { attempts } }`, stage `open`,
//!   context `source.retry.exhausted`, last failure chained) — never
//!   an infinite retry, never a panic, and the terminal answer is
//!   idempotent;
//! - the window can expire **by time** with attempts unspent;
//! - TLS is never retried (a downgrade is not a recovery) and
//!   non-network failures pass through as typed fatals;
//! - end-to-end: a mid-stream interruption drives the engine
//!   `playing → buffering → playing` (the doc 03 §8 "Buffering then
//!   resume" pass condition) with **zero fabricated packets**, and a
//!   permanent interruption lands the typed fatal on the wire as
//!   `playback.error { stage: input, code: network, fatal: true }`
//!   followed by `state-changed → error`.
//!
//! **Audio device loss** (doc 03 §4.9(d) suspend + reopen):
//! - loss emits the catalog `audio.device-lost` and suspends at the
//!   remembered position;
//! - reopen success emits `audio.device-changed` and resumes **from
//!   the remembered position** (clock continuity, never a restart);
//! - reopen failure backs off inside the window and terminates in
//!   exactly one **non-fatal** notice (suspend-and-notify — the doc's
//!   `[PROPOSAL]` default, implemented);
//! - repeated loss is tolerated (fresh window per episode);
//! - end-to-end through the **command layer**: the suspend is
//!   `playback.pause` and the resume `playback.play` (typed
//!   `state-changed` rows, engine-frozen clock, position preserved),
//!   and an exhausted reopen leaves the session `paused` — not
//!   `error`.
//!
//! **Renderer surface loss** (doc 03 §4.11(d)):
//! - loss marks the surface dirty, requests a recreate, and emits the
//!   non-fatal audio-only-degradation `playback.error`;
//! - reattachment resumes compositing with **no catalog event** (the
//!   recorded v1 gap) and **no engine restart**;
//! - resize is non-fatal by construction;
//! - end-to-end: the session stays `playing` through the whole
//!   loss/recreate cycle (same session id, same generation, video
//!   presents suppressed while audio frames keep flowing).
//!
//! # The fakes
//!
//! The testkit crate carries no synthetic fault sources yet (cards
//! 3.D1/3.D2, W5-03/W5-04 own `crates/testkit/src/faults.rs`); per
//! the card, every fault primitive this suite needs is a **local
//! fake** (the `recovery_media.rs` doctrine — local fakes so waves
//! cannot collide): the interrupted-network demuxer (guard-gated
//! reads over a shared virtual clock), the kind-aware pass-through
//! decoder, and the reopen-result scripts. The shared builders
//! (`track`/`packet`/queues/throttle) come from `tests/common`.
//!
//! # The stall stand-in (recorded honestly)
//!
//! A stalled source must answer `Demuxer::next_event` *something*
//! without ending the stream or failing: the real input thread
//! **blocks** on its byte-stream read (doc 03 §4.1(c)) — the
//! synchronous test engine has no blocking seam and no stall event
//! kind. The fake answers
//! `DemuxEvent::Recovered { skipped: 0, … }`, which is observably
//! inert in the engine (`demux_step` adds zero to the damage
//! counters, so no `DamagedData`/`RecoveredDamage` is ever emitted —
//! asserted below). `[PROPOSAL: a first-class stall signal for the
//! demux seam; until then this stand-in is the test-side shape of
//! the blocked read.]`

use std::collections::VecDeque;
use std::error::Error;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
use commands::envelope::RequestId;
use commands::{ErrorCode, EventKind, PlaybackState, Stage};
use media_clock::{MediaClock, MonotonicSource, PlaybackClock};
use media_types::error::{
    CorruptDetail, DeviceCause, DeviceSlot, ErrorContext, ErrorKind, NetworkCause,
};
use media_types::frame::{ColorDescription, FrameStorage, PixelFormat};
use media_types::packet::SharedBytes;
use media_types::{
    EngineError, Frame, GenerationId, MediaKind, Nanoseconds, Packet, PipelineStage, Rate,
    SourceUri, TimeSpan, Timestamp, Track, TrackId,
};
use playback_engine::recovery::{
    AudioDeviceGuard, ReadOutcome, RecoveryAction, RetryWindow, SourceRetryGuard, SurfaceGuard,
    is_retryable_network, retry_exhausted_error,
};
use playback_engine::state::SessionState;
use playback_engine::{
    DecoderProvider, Engine, EngineEventKind, EngineOutput, FrameConsumer, SessionConfig,
    SessionHandle, wire_error_code, wire_stage,
};
use plugin_api::{DemuxEvent, Demuxer};
use testkit::Must;

mod common;

use common::{
    Collector, FakeDemuxer, SharedNow, base, frozen_throttle, packet, tiny_budget, tiny_queues,
    track,
};

// ---------------------------------------------------------------------------
// Local fakes (the card's fault primitives; testkit owns none yet)
// ---------------------------------------------------------------------------

/// Advance the virtual clock by `ns` nanoseconds.
fn advance_now(now: &SharedNow, ns: u64) {
    let current = now.0.load(Ordering::Relaxed);
    now.0.store(current.saturating_add(ns), Ordering::Relaxed);
}

/// The clock's current position in nanoseconds (the
/// `clock_position_ns` doctrine from `session.rs`: the `Err` arm is
/// the i64-exhaustion corner only; 0 is the honest floor there,
/// never a guess presented as truth).
fn clock_ns(clock: &MediaClock) -> i64 {
    match clock.now().as_nanos() {
        Ok(Nanoseconds(ns)) => i64::try_from(ns).unwrap_or(i64::MAX),
        Err(_) => 0,
    }
}

/// The network link's shared state: the source's remaining data, the
/// delivered-packet counter (the no-fabrication proof surface), the
/// interruption flag, and the retry-guard the demuxer consults.
struct NetworkLink {
    remaining: AtomicU64,
    delivered: AtomicU64,
    interrupted: AtomicBool,
    guard: Mutex<SourceRetryGuard>,
}

impl NetworkLink {
    fn new(packets: u64, policy: RetryWindow) -> Arc<NetworkLink> {
        Arc::new(NetworkLink {
            remaining: AtomicU64::new(packets),
            delivered: AtomicU64::new(0),
            interrupted: AtomicBool::new(false),
            guard: Mutex::new(SourceRetryGuard::new(policy)),
        })
    }
}

/// The observably-inert stall token (see the module docs): the
/// stand-in for the input thread's blocked read.
fn stall_tick() -> DemuxEvent {
    DemuxEvent::Recovered {
        skipped: 0,
        reason: "network read pending (the blocked-read stand-in)",
    }
}

/// The interrupted-network source (doc 03 §8's `ScriptSource` shape,
/// built on the retry guard): reads are **gated by the guard** — a
/// failing read opens/advances the retry window, a retry is only
/// performed when the guard's deadline passed, and the window's
/// exhaustion surfaces as the typed network error (a fatal the engine
/// turns into `playback.error` + `error`).
struct NetworkDemuxer {
    tracks: Vec<Track>,
    track_id: TrackId,
    link: Arc<NetworkLink>,
    now: Arc<SharedNow>,
}

impl Demuxer for NetworkDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        Ok(self.tracks.clone())
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        let now = self.now.now_ns();
        let mut guard = self
            .link
            .guard
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if self.link.interrupted.load(Ordering::Relaxed) {
            // Time-expiry first: the window may have ended between
            // polls.
            if let RecoveryAction::FailSource(error) = guard.poll(now).action {
                return Err(error);
            }
            // A read is due when the guard is idle (first failure) or
            // its retry deadline passed.
            let due = !guard.retrying()
                || guard
                    .next_retry_ns()
                    .is_some_and(|deadline| now >= deadline);
            if due {
                let outcome = ReadOutcome::Interrupted(network_down());
                if let RecoveryAction::FailSource(error) = guard.read(now, outcome).action {
                    return Err(error);
                }
            }
            return Ok(stall_tick());
        }
        // Feeding: the guard still gates reads until the scheduled
        // retry is due (the source recovered only counts when the
        // policy says a read may happen).
        if guard.retrying() && guard.next_retry_ns().is_some_and(|deadline| now < deadline) {
            return Ok(stall_tick());
        }
        let left = self.link.remaining.load(Ordering::Relaxed);
        if left == 0 {
            return Ok(DemuxEvent::EndOfStream);
        }
        self.link.remaining.store(left - 1, Ordering::Relaxed);
        let index = self.link.delivered.fetch_add(1, Ordering::Relaxed);
        // The read succeeded. The guard's buffering bookends are the
        // input-layer's wire view — the ENGINE owns the episode in
        // this integration (the module-docs ownership rule), so the
        // step's events are deliberately dropped here.
        guard.read(now, ReadOutcome::Data { units: 1 });
        Ok(DemuxEvent::Packet(packet(self.track_id, index)))
    }

    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
        Err(EngineError::new(
            PipelineStage::Demux,
            ErrorKind::Unsupported { format: None },
            ErrorContext::new("demux.seek.not-implemented"),
        ))
    }
}

/// The kind-aware pass-through decoder: one frame per packet — audio
/// frames for audio limbs, video frames for video limbs (the surface
/// suite must distinguish them at the sink).
struct PassDecoder {
    kind: MediaKind,
    generation: GenerationId,
}

impl Decoder for PassDecoder {
    fn configure(&mut self, config: &DecoderConfig) -> Result<(), EngineError> {
        self.generation = config.generation;
        Ok(())
    }

    fn decode(
        &mut self,
        packet: Packet,
        out: &mut dyn FrameSink,
    ) -> Result<DecodeOutcome, EngineError> {
        if packet.is_stale(self.generation) {
            return Ok(DecodeOutcome::NeedMoreInput);
        }
        let frame = match self.kind {
            MediaKind::Video => video_frame(packet.pts, self.generation),
            _ => common::audio_frame(packet.pts, self.generation),
        };
        out.accept(frame)?;
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        Ok(DecodeOutcome::Drained)
    }
}

/// The decoder seam: one [`PassDecoder`] per track, kind taken from
/// the track.
struct PassProvider;

impl DecoderProvider for PassProvider {
    fn decoder_for(
        &self,
        track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        Ok(Box::new(PassDecoder {
            kind: track.kind,
            generation: config.generation,
        }))
    }
}

fn video_frame(pts: media_types::Stamp, generation: GenerationId) -> Frame {
    Frame::Video(media_types::frame::VideoFrame {
        format: PixelFormat::Nv12,
        size: (16, 16),
        data: FrameStorage::Cpu(SharedBytes::from_static(&[0u8; 4])),
        pts,
        color: ColorDescription::BT709_LIMITED,
        interlaced: false,
        generation,
    })
}

// ---------------------------------------------------------------------------
// Error shapes the fakes and tests arm
// ---------------------------------------------------------------------------

/// The transient network weather the source fails with.
fn network_down() -> EngineError {
    EngineError::new(
        PipelineStage::Open,
        ErrorKind::Network {
            cause: NetworkCause::ConnectionLost,
        },
        ErrorContext::new("source.read.connection-lost"),
    )
}

/// A TLS verification failure (never retried).
fn tls_failed() -> EngineError {
    EngineError::new(
        PipelineStage::Open,
        ErrorKind::Network {
            cause: NetworkCause::Tls,
        },
        ErrorContext::new("source.read.tls"),
    )
}

/// A media-level failure (out of the source guard's scope).
fn corrupt_media() -> EngineError {
    EngineError::new(
        PipelineStage::Demux,
        ErrorKind::Corrupt {
            detail: CorruptDetail::BadIndex,
        },
        ErrorContext::new("demux.index.corrupt"),
    )
}

/// A failed audio-device reopen attempt.
fn device_gone() -> EngineError {
    EngineError::new(
        PipelineStage::AudioOutput,
        ErrorKind::Device {
            which: DeviceSlot::AudioOutput,
            cause: DeviceCause::Disconnected,
        },
        ErrorContext::new("audio.device.reopen"),
    )
}

// ---------------------------------------------------------------------------
// The engine fixture (clock the test owns, optional network link)
// ---------------------------------------------------------------------------

const NS_PER_MS: u64 = 1_000_000;
const NS_PER_S: u64 = 1_000_000_000;

/// Everything one engine-level test drives: the engine and its
/// command handle, the collecting sink, the **shared virtual clock**
/// (the same `SharedNow` drives the media clock, the network link,
/// and every `now_ns` the guards are asked with), and the network
/// link when the test is about the source.
struct EnvFixture {
    engine: Engine,
    handle: SessionHandle,
    collector: Arc<Collector>,
    clock: Arc<MediaClock>,
    now: Arc<SharedNow>,
    link: Option<Arc<NetworkLink>>,
}

impl EnvFixture {
    /// The network link (panic-free: tests that built one ask for it).
    fn link(&self) -> Arc<NetworkLink> {
        match &self.link {
            Some(link) => Arc::clone(link),
            None => unreachable!("fixture has no network link"),
        }
    }

    /// Frames of one kind delivered so far.
    fn frames_of_kind(&self, kind: MediaKind) -> usize {
        self.collector
            .frames
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .filter(|(_, frame)| frame_kind(frame) == kind)
            .count()
    }

    /// Frames of every kind delivered so far.
    fn frames_total(&self) -> usize {
        self.collector
            .frames
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }
}

fn frame_kind(frame: &Frame) -> MediaKind {
    match frame {
        Frame::Audio(_) => MediaKind::Audio,
        Frame::Video(_) => MediaKind::Video,
        Frame::Subtitle(_) => MediaKind::Subtitle,
    }
}

/// Open an engine session over the given demuxer with the tiny
/// deterministic queues, the shared virtual clock, and the frozen
/// throttle (the session_flow doctrine: emission windows never fire
/// mid-test).
fn open_env(
    demuxer: Box<dyn Demuxer>,
    now: Arc<SharedNow>,
    link: Option<Arc<NetworkLink>>,
) -> EnvFixture {
    let collector = Arc::new(Collector::default());
    // Two-step: the unsizing coercion lands at the binding (an
    // expected type on the `Arc::clone` call itself would pollute its
    // method inference).
    let clock_now = Arc::clone(&now);
    let clock_source: Arc<dyn MonotonicSource> = clock_now;
    let clock = Arc::new(
        MediaClock::system_monotonic(clock_source, Timestamp::new(0, base()), Rate::ONE).must(),
    );
    let mut config = SessionConfig::new(
        SourceUri::parse("https://media.lumen.test/live/env-recovery").must(),
        demuxer,
        Arc::new(PassProvider),
        Arc::clone(&collector) as Arc<dyn FrameConsumer>,
        Arc::clone(&clock),
    );
    config.queues = tiny_queues();
    config.budget = tiny_budget();
    config.throttle = frozen_throttle();
    let mut engine = Engine::new();
    let handle = engine.open(config).must();
    EnvFixture {
        engine,
        handle,
        collector,
        clock,
        now,
        link,
    }
}

/// The interrupted-network fixture: one audio track, `packets` in the
/// source, the guard armed with `policy`.
fn open_network(packets: u64, policy: RetryWindow, start_ns: u64) -> EnvFixture {
    let now = Arc::new(SharedNow(std::sync::atomic::AtomicU64::new(start_ns)));
    let link = NetworkLink::new(packets, policy);
    let tracks = vec![track(1, MediaKind::Audio)];
    let demuxer = Box::new(NetworkDemuxer {
        track_id: tracks[0].id,
        tracks,
        link: Arc::clone(&link),
        now: Arc::clone(&now),
    });
    open_env(demuxer, now, Some(link))
}

/// A static audio script (`len` packets then end-of-stream) over the
/// shared clock — the device/surface fixtures.
fn open_script_audio(len: u64, start_ns: u64) -> EnvFixture {
    let now = Arc::new(SharedNow(std::sync::atomic::AtomicU64::new(start_ns)));
    let mut script: Vec<DemuxEvent> = (0..len)
        .map(|index| DemuxEvent::Packet(packet(TrackId::new(1), index)))
        .collect();
    script.push(DemuxEvent::EndOfStream);
    open_env(
        Box::new(FakeDemuxer {
            tracks: vec![track(1, MediaKind::Audio)],
            script: Arc::new(Mutex::new(VecDeque::from(script))),
            stall: Arc::new(Mutex::new(false)),
            fail_start: None,
        }),
        now,
        None,
    )
}

/// The interleaved audio+video script (the surface fixture): `len`
/// packets per kind, alternating, then end-of-stream.
fn open_script_av(len: u64, start_ns: u64) -> EnvFixture {
    let now = Arc::new(SharedNow(std::sync::atomic::AtomicU64::new(start_ns)));
    let mut script: Vec<DemuxEvent> = Vec::new();
    for index in 0..len {
        script.push(DemuxEvent::Packet(packet(TrackId::new(1), index * 2)));
        script.push(DemuxEvent::Packet(packet(TrackId::new(2), index * 2 + 1)));
    }
    script.push(DemuxEvent::EndOfStream);
    let tracks = vec![track(1, MediaKind::Audio), track(2, MediaKind::Video)];
    open_env(
        Box::new(FakeDemuxer {
            tracks,
            script: Arc::new(Mutex::new(VecDeque::from(script))),
            stall: Arc::new(Mutex::new(false)),
            fail_start: None,
        }),
        now,
        None,
    )
}

// ---------------------------------------------------------------------------
// Wire collection and assertions
// ---------------------------------------------------------------------------

/// One poll, collecting the engine's outputs into the wire log (the
/// catalog form of every engine event) and the raw log (engine-private
/// kinds — the `Fatal` payload assertions).
fn poll_round(fixture: &mut EnvFixture, wire: &mut Vec<EventKind>, raw: &mut Vec<EngineEventKind>) {
    fixture.engine.poll();
    for output in fixture.engine.take_outputs() {
        match output {
            EngineOutput::Event(event) => {
                if let Some(kind) = event.kind.as_wire() {
                    wire.push(kind);
                }
                raw.push(event.kind);
            }
            EngineOutput::Receipt(_) => {}
        }
    }
}

/// Drive polls until `predicate` holds on the engine state (the
/// bounded never-hang driver), collecting as it goes.
fn poll_engine_until(
    fixture: &mut EnvFixture,
    wire: &mut Vec<EventKind>,
    raw: &mut Vec<EngineEventKind>,
    predicate: impl Fn(SessionState) -> bool,
    limit: usize,
    context: &str,
) {
    for _ in 0..limit {
        if predicate(fixture.engine.state()) {
            return;
        }
        poll_round(fixture, wire, raw);
    }
    assert!(
        predicate(fixture.engine.state()),
        "{context}: state is {} after {limit} polls",
        fixture.engine.state()
    );
}

/// Every `(from, to)` state change on the wire, in order.
fn state_changes(wire: &[EventKind]) -> Vec<(PlaybackState, PlaybackState)> {
    wire.iter()
        .filter_map(|event| match event {
            EventKind::SessionStateChanged { from, to } => Some((*from, *to)),
            _ => None,
        })
        .collect()
}

/// Every `session.buffering` progress value, in order.
fn buffering_values(wire: &[EventKind]) -> Vec<f32> {
    wire.iter()
        .filter_map(|event| match event {
            EventKind::SessionBuffering { progress } => Some(*progress),
            _ => None,
        })
        .collect()
}

/// The position of an exact wire event, if present.
fn wire_index(wire: &[EventKind], wanted: &EventKind) -> Option<usize> {
    wire.iter().position(|event| event == wanted)
}

/// The fatal errors the engine emitted (raw, pre-wire).
fn fatals(raw: &[EngineEventKind]) -> Vec<&EngineError> {
    raw.iter()
        .filter_map(|kind| match kind {
            EngineEventKind::Fatal { error } => Some(error),
            _ => None,
        })
        .collect()
}

/// Assert a step is the typed-source-fatal and hand it back.
fn take_fail_source(step: playback_engine::recovery::RecoveryStep) -> EngineError {
    match step.action {
        RecoveryAction::FailSource(error) => error,
        other => unreachable!("expected FailSource, got {other:?}"),
    }
}

// ===========================================================================
// Policy: the retry window (interrupted network source)
// ===========================================================================

/// The backoff schedule: base, doubling, cap — saturating, exact.
#[test]
fn source_retry_backoff_schedule_is_bounded_and_deterministic() {
    let window = RetryWindow {
        max_attempts: 6,
        window_ns: 10 * NS_PER_S,
        base_backoff_ns: 100 * NS_PER_MS,
        max_backoff_ns: 400 * NS_PER_MS,
    };
    assert_eq!(window.backoff_ns(1), 100 * NS_PER_MS);
    assert_eq!(window.backoff_ns(2), 200 * NS_PER_MS);
    assert_eq!(window.backoff_ns(3), 400 * NS_PER_MS, "the cap binds");
    assert_eq!(window.backoff_ns(4), 400 * NS_PER_MS, "the cap holds");
    assert_eq!(
        window.backoff_ns(u32::MAX),
        400 * NS_PER_MS,
        "saturating: no overflow, no wrap"
    );
    assert_eq!(RetryWindow::DEFAULT.max_attempts, 6);
}

/// A transient loss: the window opens with the `0.0` bookend, the
/// retries follow the exact schedule, recovery closes the episode
/// with `1.0` — and a later loss opens a FRESH window (fresh
/// budget).
#[test]
fn source_retry_recovers_transient_loss_with_exact_schedule() {
    let window = RetryWindow {
        max_attempts: 6,
        window_ns: 10 * NS_PER_S,
        base_backoff_ns: 100 * NS_PER_MS,
        max_backoff_ns: 400 * NS_PER_MS,
    };
    let mut guard = SourceRetryGuard::new(window);

    // First failure at t=1s: the episode opens.
    let step = guard.read(NS_PER_S, ReadOutcome::Interrupted(network_down()));
    assert_eq!(
        step.events,
        vec![EventKind::SessionBuffering { progress: 0.0 }]
    );
    assert!(guard.retrying());
    assert_eq!(guard.retries_used(), Some(0));
    assert_eq!(guard.next_retry_ns(), Some(NS_PER_S + 100 * NS_PER_MS));

    // Before the deadline: nothing due.
    let step = guard.poll(NS_PER_S + 50 * NS_PER_MS);
    assert!(matches!(step.action, RecoveryAction::None));

    // At the deadline: retry 1 is permitted.
    let step = guard.poll(NS_PER_S + 100 * NS_PER_MS);
    match step.action {
        RecoveryAction::RetryRead { attempt } => assert_eq!(attempt, 1),
        other => unreachable!("expected RetryRead, got {other:?}"),
    }

    // Retry 1 fails: backoff doubles.
    let step = guard.read(
        NS_PER_S + 100 * NS_PER_MS,
        ReadOutcome::Interrupted(network_down()),
    );
    assert!(matches!(step.action, RecoveryAction::None));
    assert_eq!(guard.retries_used(), Some(1));
    assert_eq!(guard.next_retry_ns(), Some(NS_PER_S + 300 * NS_PER_MS));

    // Retry 2 fails at the capped backoff.
    let step = guard.poll(NS_PER_S + 300 * NS_PER_MS);
    match step.action {
        RecoveryAction::RetryRead { attempt } => assert_eq!(attempt, 2),
        other => unreachable!("expected RetryRead, got {other:?}"),
    }
    guard.read(
        NS_PER_S + 300 * NS_PER_MS,
        ReadOutcome::Interrupted(network_down()),
    );
    assert_eq!(guard.next_retry_ns(), Some(NS_PER_S + 700 * NS_PER_MS));

    // Retry 3 delivers data: the episode closes with the 1.0 bookend.
    let step = guard.read(
        NS_PER_S + 700 * NS_PER_MS,
        ReadOutcome::Data { units: 1024 },
    );
    assert_eq!(
        step.events,
        vec![EventKind::SessionBuffering { progress: 1.0 }]
    );
    assert!(!guard.retrying());
    assert_eq!(guard.retries_used(), None, "recovered: no standing count");
    assert_eq!(guard.windows_opened(), 1);

    // A later loss opens a fresh window with a fresh budget.
    let step = guard.read(4 * NS_PER_S, ReadOutcome::Interrupted(network_down()));
    assert_eq!(
        step.events,
        vec![EventKind::SessionBuffering { progress: 0.0 }]
    );
    assert_eq!(guard.retries_used(), Some(0), "the attempt budget reset");
    assert_eq!(guard.windows_opened(), 2);
}

/// Exhaustion by attempts: the typed network error, the bookend
/// close, the chained last failure, and the idempotent terminal.
#[test]
fn source_retry_exhaustion_answers_the_typed_network_error() {
    let window = RetryWindow {
        max_attempts: 2,
        window_ns: 60 * NS_PER_S,
        base_backoff_ns: 100 * NS_PER_MS,
        max_backoff_ns: 200 * NS_PER_MS,
    };
    let mut guard = SourceRetryGuard::new(window);

    guard.read(NS_PER_S, ReadOutcome::Interrupted(network_down()));
    guard.read(
        NS_PER_S + 100 * NS_PER_MS,
        ReadOutcome::Interrupted(network_down()),
    );
    assert_eq!(guard.retries_used(), Some(1));
    // Retry 2 fails: the budget (2) is spent.
    let step = guard.read(
        NS_PER_S + 300 * NS_PER_MS,
        ReadOutcome::Interrupted(network_down()),
    );
    let error = take_fail_source(step);
    assert_eq!(
        error.kind,
        ErrorKind::Network {
            cause: NetworkCause::ReconnectExhausted { attempts: 2 }
        },
        "the typed network error, attempts = the retries actually made"
    );
    assert_eq!(error.stage, PipelineStage::Open);
    assert_eq!(error.context.code, "source.retry.exhausted");
    // The last observed failure rides the source chain.
    let chained = error
        .source()
        .and_then(|source| source.downcast_ref::<EngineError>())
        .must();
    assert_eq!(
        chained.kind,
        ErrorKind::Network {
            cause: NetworkCause::ConnectionLost
        }
    );

    // The terminal is standing and idempotent: a dead source keeps
    // answering the typed error (never a fabricated read).
    for now_ns in [
        NS_PER_S + 400 * NS_PER_MS,
        NS_PER_S + 10 * NS_PER_S,
        NS_PER_S + 400 * NS_PER_S,
    ] {
        let error = take_fail_source(guard.read(now_ns, ReadOutcome::Interrupted(network_down())));
        assert_eq!(
            error.kind,
            ErrorKind::Network {
                cause: NetworkCause::ReconnectExhausted { attempts: 2 }
            }
        );
        let error = take_fail_source(guard.poll(now_ns));
        assert_eq!(
            error.kind,
            ErrorKind::Network {
                cause: NetworkCause::ReconnectExhausted { attempts: 2 }
            }
        );
    }
    assert_eq!(guard.windows_opened(), 1, "one episode, one window");
}

/// The window is the SECOND bound: it can expire by time with
/// attempts unspent (never an infinite retry by either axis).
#[test]
fn source_retry_window_expires_by_time_with_attempts_left() {
    let window = RetryWindow {
        max_attempts: 10,
        window_ns: NS_PER_S,
        base_backoff_ns: 300 * NS_PER_MS,
        max_backoff_ns: 600 * NS_PER_MS,
    };
    let mut guard = SourceRetryGuard::new(window);

    guard.read(0, ReadOutcome::Interrupted(network_down()));
    assert_eq!(guard.next_retry_ns(), Some(300 * NS_PER_MS));
    guard.read(300 * NS_PER_MS, ReadOutcome::Interrupted(network_down()));
    assert_eq!(guard.retries_used(), Some(1));
    // The next retry would land at 300ms + 600ms = 900ms — inside
    // the 1s window, so it is scheduled…
    assert_eq!(guard.next_retry_ns(), Some(900 * NS_PER_MS));
    guard.read(900 * NS_PER_MS, ReadOutcome::Interrupted(network_down()));
    // …but the retry after that (900ms + 600ms = 1.5s) is beyond the
    // window's end: exhaustion BY TIME with only 2 of 10 attempts
    // spent.
    let error =
        take_fail_source(guard.read(900 * NS_PER_MS, ReadOutcome::Interrupted(network_down())));
    assert_eq!(
        error.kind,
        ErrorKind::Network {
            cause: NetworkCause::ReconnectExhausted { attempts: 2 }
        }
    );

    // The pure time-expiry path (no fresh failure at hand): a window
    // left open past its end answers the typed error at poll time.
    let mut guard = SourceRetryGuard::new(window);
    guard.read(0, ReadOutcome::Interrupted(network_down()));
    let error = take_fail_source(guard.poll(2 * NS_PER_S));
    assert_eq!(
        error.kind,
        ErrorKind::Network {
            cause: NetworkCause::ReconnectExhausted { attempts: 0 }
        },
        "the window expired before any retry was made"
    );
}

/// TLS failures are never retried (a weaker-path retry would be a
/// downgrade, not a recovery): the original typed error is the
/// immediate fatal, and no window opens.
#[test]
fn source_tls_failure_is_never_retried() {
    let mut guard = SourceRetryGuard::new(RetryWindow::DEFAULT);
    let error = take_fail_source(guard.read(0, ReadOutcome::Interrupted(tls_failed())));
    assert_eq!(
        error.kind,
        ErrorKind::Network {
            cause: NetworkCause::Tls
        }
    );
    assert!(!guard.retrying());
    assert_eq!(guard.next_retry_ns(), None, "no retry was scheduled");
    assert_eq!(guard.windows_opened(), 0);
}

/// Non-network failures are out of the source guard's scope: they
/// pass through as typed fatals, unchanged.
#[test]
fn source_non_network_failure_passes_through_fatal() {
    let mut guard = SourceRetryGuard::new(RetryWindow::DEFAULT);
    let error = take_fail_source(guard.read(0, ReadOutcome::Interrupted(corrupt_media())));
    assert_eq!(
        error.kind,
        ErrorKind::Corrupt {
            detail: CorruptDetail::BadIndex
        }
    );
    assert_eq!(error.stage, PipelineStage::Demux);
    assert_eq!(guard.windows_opened(), 0);

    // A mid-window non-network failure is also immediate (and closes
    // the open episode bookend).
    let mut guard = SourceRetryGuard::new(RetryWindow::DEFAULT);
    guard.read(0, ReadOutcome::Interrupted(network_down()));
    let step = guard.read(500 * NS_PER_MS, ReadOutcome::Interrupted(corrupt_media()));
    assert_eq!(
        step.events,
        vec![EventKind::SessionBuffering { progress: 1.0 }],
        "the open episode closes before the fatal (no stranded 0.0)"
    );
    let error = take_fail_source(step);
    assert_eq!(
        error.kind,
        ErrorKind::Corrupt {
            detail: CorruptDetail::BadIndex
        }
    );
}

/// The retryability classification: exactly the transient network
/// weather set.
#[test]
fn retryability_classification_pins_the_network_weather_set() {
    let net = |cause| {
        EngineError::new(
            PipelineStage::Open,
            ErrorKind::Network { cause },
            ErrorContext::new("source.read"),
        )
    };
    assert!(is_retryable_network(&net(NetworkCause::Timeout)));
    assert!(is_retryable_network(&net(NetworkCause::ConnectionLost)));
    assert!(is_retryable_network(&net(NetworkCause::Dns)));
    assert!(!is_retryable_network(&net(NetworkCause::Tls)));
    assert!(!is_retryable_network(&net(
        NetworkCause::ReconnectExhausted { attempts: 3 }
    )));
    assert!(!is_retryable_network(&corrupt_media()));
    assert!(!is_retryable_network(&device_gone()));
}

/// The typed exhaustion error lands on the catalog wire row exactly:
/// `playback.error` code `network`, stage `input` (the seam the
/// engine's `Failed` input emits through `event.rs`'s mapping).
#[test]
fn exhausted_error_maps_onto_the_catalog_wire_row() {
    let error = retry_exhausted_error(4, None);
    assert_eq!(wire_error_code(&error.kind), ErrorCode::Network);
    assert_eq!(wire_stage(error.stage), Stage::Input);
}

// ===========================================================================
// Policy: audio device loss (suspend + reopen)
// ===========================================================================

/// Loss suspends at the remembered position with the catalog event,
/// and the first reopen attempt is due immediately.
#[test]
fn device_loss_suspends_at_position_with_catalog_event() {
    let mut guard = AudioDeviceGuard::new(RetryWindow::DEFAULT);
    let step = guard.observe_device_lost(NS_PER_S, "dev0", 123_456);
    assert_eq!(
        step.events,
        vec![EventKind::AudioDeviceLost {
            device_id: "dev0".to_string()
        }]
    );
    match step.action {
        RecoveryAction::SuspendClock { position_ns } => assert_eq!(position_ns, 123_456),
        other => unreachable!("expected SuspendClock, got {other:?}"),
    }
    assert!(guard.suspended());
    assert_eq!(guard.suspended_position_ns(), Some(123_456));
    assert_eq!(guard.active_device(), None);
    // The immediate first attempt: a re-plugged device answers now.
    let step = guard.poll(NS_PER_S);
    assert!(matches!(step.action, RecoveryAction::ReopenDevice));
    assert_eq!(guard.losses(), 1);
}

/// Reopen success emits `audio.device-changed` and resumes from the
/// remembered position — never a restart.
#[test]
fn device_reopen_success_resumes_from_remembered_position() {
    let mut guard = AudioDeviceGuard::new(RetryWindow::DEFAULT);
    guard.observe_device_lost(NS_PER_S, "dev0", 777_000);
    let step = guard.observe_reopen_result(NS_PER_S + 2 * NS_PER_MS, Ok("dev1".to_string()));
    assert_eq!(
        step.events,
        vec![EventKind::AudioDeviceChanged {
            device_id: "dev1".to_string()
        }]
    );
    match step.action {
        RecoveryAction::ResumeClock { position_ns } => {
            assert_eq!(
                position_ns, 777_000,
                "resume continues from the loss position"
            );
        }
        other => unreachable!("expected ResumeClock, got {other:?}"),
    }
    assert!(!guard.suspended());
    assert_eq!(guard.active_device(), Some("dev1"));
    assert_eq!(guard.reopens(), 1);
}

/// Reopen failures back off inside the window and terminate in
/// exactly ONE non-fatal notice (suspend-and-notify); afterwards no
/// further attempts are granted (bounded).
#[test]
fn device_reopen_failure_backs_off_then_notifies_once_non_fatal() {
    let policy = RetryWindow {
        max_attempts: 2,
        window_ns: 10 * NS_PER_S,
        base_backoff_ns: NS_PER_S,
        max_backoff_ns: 4 * NS_PER_S,
    };
    let mut guard = AudioDeviceGuard::new(policy);
    guard.observe_device_lost(10 * NS_PER_S, "dev0", 5_000);

    // First attempt: immediate, fails, backoff to +2s.
    assert!(matches!(
        guard.poll(10 * NS_PER_S).action,
        RecoveryAction::ReopenDevice
    ));
    let step = guard.observe_reopen_result(10 * NS_PER_S, Err(device_gone()));
    assert!(matches!(step.action, RecoveryAction::None));
    assert!(matches!(
        guard.poll(11 * NS_PER_S).action,
        RecoveryAction::None
    ));
    assert!(matches!(
        guard.poll(12 * NS_PER_S).action,
        RecoveryAction::ReopenDevice
    ));

    // Second failure: the budget is spent — the one-time notice.
    let step = guard.observe_reopen_result(12 * NS_PER_S, Err(device_gone()));
    assert!(matches!(step.action, RecoveryAction::None));
    match step.events.as_slice() {
        [
            EventKind::PlaybackError {
                stage,
                code,
                fatal,
                details,
                ..
            },
        ] => {
            assert_eq!(
                (*stage, *code, *fatal),
                (Stage::AudioOutput, ErrorCode::Device, false)
            );
            assert_eq!(
                details.get("reopen_attempts").map(String::as_str),
                Some("2")
            );
            assert_eq!(details.get("device").map(String::as_str), Some("dev0"));
        }
        other => unreachable!("expected one non-fatal notice, got {other:?}"),
    }
    // Still suspended at the remembered position; no more grants.
    assert!(guard.suspended());
    assert_eq!(guard.suspended_position_ns(), Some(5_000));
    for now_ns in [13 * NS_PER_S, 30 * NS_PER_S, 400 * NS_PER_S] {
        let step = guard.poll(now_ns);
        assert!(
            matches!(step.action, RecoveryAction::None),
            "no reopen after the notice (bounded) at {now_ns}"
        );
    }
}

/// Repeated loss is tolerated: every episode gets a fresh window and
/// a fresh budget, and the active device tracks the replacements.
#[test]
fn device_repeated_loss_opens_fresh_windows_and_is_tolerated() {
    let policy = RetryWindow {
        max_attempts: 1,
        window_ns: NS_PER_S,
        base_backoff_ns: 100 * NS_PER_MS,
        max_backoff_ns: 100 * NS_PER_MS,
    };
    let mut guard = AudioDeviceGuard::new(policy);
    for cycle in 0..3u64 {
        let position: i64 = 1_000 * (cycle as i64 + 1);
        let lost = format!("dev{cycle}");
        let replacement = format!("dev{}-new", cycle);
        let step = guard.observe_device_lost(NS_PER_S * (cycle + 1), &lost, position);
        assert_eq!(
            step.events,
            vec![EventKind::AudioDeviceLost {
                device_id: lost.clone()
            }]
        );
        // The fresh window grants the first attempt immediately —
        // even though the previous episode exhausted its budget.
        assert!(matches!(
            guard.poll(NS_PER_S * (cycle + 1)).action,
            RecoveryAction::ReopenDevice
        ));
        let step = guard.observe_reopen_result(NS_PER_S * (cycle + 1), Ok(replacement.clone()));
        assert_eq!(
            step.events,
            vec![EventKind::AudioDeviceChanged {
                device_id: replacement.clone()
            }]
        );
        match step.action {
            RecoveryAction::ResumeClock { position_ns } => assert_eq!(position_ns, position),
            other => unreachable!("expected ResumeClock, got {other:?}"),
        }
    }
    assert_eq!(guard.losses(), 3);
    assert_eq!(guard.reopens(), 3);
    assert_eq!(guard.active_device(), Some("dev2-new"));
}

// ===========================================================================
// Policy: renderer surface loss
// ===========================================================================

/// Loss marks the surface dirty, requests the recreate, and emits the
/// non-fatal audio-only-degradation event.
#[test]
fn surface_loss_marks_dirty_requests_recreate_and_degrades_audio_only() {
    let mut guard = SurfaceGuard::new();
    assert!(guard.compositing());
    let step = guard.observe_surface_lost();
    match step.events.as_slice() {
        [
            EventKind::PlaybackError {
                stage,
                code,
                fatal,
                details,
                ..
            },
        ] => {
            assert_eq!(
                (*stage, *code, *fatal),
                (Stage::Renderer, ErrorCode::Device, false)
            );
            assert_eq!(
                details.get("context").map(String::as_str),
                Some("renderer.surface.lost")
            );
        }
        other => unreachable!("expected the degradation event, got {other:?}"),
    }
    assert!(matches!(
        step.action,
        RecoveryAction::RequestSurfaceRecreate
    ));
    assert!(!guard.compositing(), "compositing holds while dirty");
    assert!(!guard.gate_present(), "video presents are suppressed");
    assert!(!guard.gate_present());
    assert_eq!(guard.suppressed_presents(), 2);
    assert_eq!(guard.losses(), 1);
}

/// Recreate resumes compositing with NO catalog event — the recorded
/// v1 gap (there is no `video.surface-*` row; see the module docs).
#[test]
fn surface_recreate_resumes_compositing_without_events() {
    let mut guard = SurfaceGuard::new();
    guard.observe_surface_lost();
    let step = guard.observe_surface_recreated();
    assert!(matches!(step.action, RecoveryAction::None));
    assert!(
        step.events.is_empty(),
        "no catalog event exists for reattachment (the gap)"
    );
    assert!(guard.compositing());
    assert!(guard.gate_present(), "presents flow again");
    assert_eq!(
        guard.suppressed_presents(),
        0,
        "nothing suppressed after the recreate"
    );
    assert_eq!(guard.recreations(), 1);
}

/// Resize is non-fatal by construction (doc 03 §4.11(d)): no dirty
/// flag, no events, compositing continues.
#[test]
fn surface_resize_is_non_fatal_by_construction() {
    let mut guard = SurfaceGuard::new();
    let step = guard.observe_resize();
    assert!(matches!(step.action, RecoveryAction::None));
    assert!(step.events.is_empty());
    assert!(guard.compositing());
    assert_eq!(guard.resizes(), 1);
    assert_eq!(guard.losses(), 0);
    // A resize while dirty does not clear the loss either.
    guard.observe_surface_lost();
    guard.observe_resize();
    assert!(!guard.compositing(), "resize never restores a lost surface");
}

/// A repeated loss before the recreate is the same loss: idempotent,
/// no duplicate events or requests.
#[test]
fn surface_repeated_loss_is_idempotent() {
    let mut guard = SurfaceGuard::new();
    guard.observe_surface_lost();
    let step = guard.observe_surface_lost();
    assert!(matches!(step.action, RecoveryAction::None));
    assert!(step.events.is_empty());
    assert_eq!(guard.losses(), 1, "the second loss is the same episode");
    guard.observe_surface_recreated();
    // A genuinely NEW loss after the recreate is a new episode.
    let step = guard.observe_surface_lost();
    assert!(matches!(
        step.action,
        RecoveryAction::RequestSurfaceRecreate
    ));
    assert_eq!(step.events.len(), 1);
    assert_eq!(guard.losses(), 2);
}

// ===========================================================================
// Engine: interrupted network source
// ===========================================================================

/// **The doc 03 §8 row, end-to-end**: "interrupted network |
/// `ScriptSource` hang/reset scripts | `Buffering` then resume or
/// typed error; no hang." A mid-stream interruption drives the engine
/// `playing → buffering → playing`; the recovery respects the retry
/// window's schedule; the media finishes at `ended` with EVERY
/// packet delivered exactly once (no fabrication, no loss).
#[test]
fn engine_interrupted_network_rebuffers_then_resumes() {
    let policy = RetryWindow {
        max_attempts: 3,
        window_ns: 3 * NS_PER_S,
        base_backoff_ns: 100 * NS_PER_MS,
        max_backoff_ns: 200 * NS_PER_MS,
    };
    let start = NS_PER_S;
    let mut fixture = open_network(40, policy, start);
    let mut wire: Vec<EventKind> = Vec::new();
    let mut raw: Vec<EngineEventKind> = Vec::new();

    // Open and reach playing (the queue primes at its high watermark).
    poll_engine_until(
        &mut fixture,
        &mut wire,
        &mut raw,
        |state| state == SessionState::Playing,
        16,
        "initial priming",
    );
    let frozen_at = clock_ns(&fixture.clock);

    // The link drops mid-stream.
    fixture.link().interrupted.store(true, Ordering::Relaxed);
    // The buffered data plays out, the queue drains, the engine
    // rebuffers (the starve path — the ENGINE's buffering episode).
    poll_engine_until(
        &mut fixture,
        &mut wire,
        &mut raw,
        |state| state == SessionState::Buffering,
        16,
        "underrun after the interruption",
    );

    // Retry 1 becomes due at +100ms; it fails (the link is still
    // down), scheduling retry 2 at +200ms more (the capped backoff).
    // A fixed round count, not an until-driver: the state predicate
    // is already true here, and the point is to run engine ticks
    // while the retry deadline passes.
    advance_now(&fixture.now, 150 * NS_PER_MS);
    for _ in 0..4 {
        poll_round(&mut fixture, &mut wire, &mut raw);
        assert_eq!(
            fixture.engine.state(),
            SessionState::Buffering,
            "still buffering while retrying"
        );
    }
    let link = fixture.link();
    let guard = link.guard.lock().unwrap_or_else(|p| p.into_inner());
    assert_eq!(guard.retries_used(), Some(1), "retry 1 was made and failed");
    drop(guard);

    // The link heals; retry 2 is due at +350ms total — advance past
    // it and let the engine refill and resume.
    fixture.link().interrupted.store(false, Ordering::Relaxed);
    advance_now(&fixture.now, 250 * NS_PER_MS);
    poll_engine_until(
        &mut fixture,
        &mut wire,
        &mut raw,
        |state| state == SessionState::Playing,
        16,
        "resume after the retry succeeds",
    );

    // Clock continuity across the rebuffer: the engine froze the
    // clock at the underrun and resumed from the frozen position —
    // the stall itself consumed no media time.
    let after_resume = clock_ns(&fixture.clock);
    assert_eq!(
        after_resume, frozen_at,
        "the clock resumed at its frozen position"
    );

    // The media finishes: every packet delivered exactly once.
    poll_engine_until(
        &mut fixture,
        &mut wire,
        &mut raw,
        |state| state == SessionState::Ended,
        64,
        "drain to ended",
    );
    assert_eq!(
        fixture.frames_total(),
        40,
        "no fabricated and no lost packets"
    );
    assert_eq!(
        fixture.link().delivered.load(Ordering::Relaxed),
        40,
        "the source delivered exactly its script"
    );

    // The wire: the full state trace with the rebuffer cycle in the
    // middle, and both buffering episodes bookended 0.0 → 1.0.
    assert_eq!(
        state_changes(&wire),
        vec![
            (PlaybackState::Idle, PlaybackState::Opening),
            (PlaybackState::Opening, PlaybackState::Buffering),
            (PlaybackState::Buffering, PlaybackState::Playing),
            (PlaybackState::Playing, PlaybackState::Buffering),
            (PlaybackState::Buffering, PlaybackState::Playing),
            (PlaybackState::Playing, PlaybackState::Ended),
        ]
    );
    assert_eq!(buffering_values(&wire), vec![0.0, 1.0, 0.0, 1.0]);

    // The stall stand-in is observationally inert: no damage ever
    // surfaced (no `playback.error` of ANY kind in this scenario).
    assert!(
        !wire
            .iter()
            .any(|event| matches!(event, EventKind::PlaybackError { .. })),
        "the inert stall token must never surface as damage"
    );

    // The guard's own accounting: one window, two failed retries, the
    // third succeeded.
    let link = fixture.link();
    let guard = link.guard.lock().unwrap_or_else(|p| p.into_inner());
    assert_eq!(guard.windows_opened(), 1);
    assert!(!guard.retrying());
}

/// **The typed-error AC, end-to-end**: a permanent interruption
/// exhausts the window and the engine answers the typed network
/// error — `playback.error { stage: input, code: network, fatal:
/// true }` BEFORE `state-changed → error` (doc 05 §3.3 ordering) —
/// with no panic, no hang, and no fabricated frames.
#[test]
fn engine_permanent_network_interruption_is_the_typed_fatal() {
    let policy = RetryWindow {
        max_attempts: 2,
        window_ns: 5 * NS_PER_S,
        base_backoff_ns: 100 * NS_PER_MS,
        max_backoff_ns: 300 * NS_PER_MS,
    };
    let mut fixture = open_network(40, policy, NS_PER_S);
    let mut wire: Vec<EventKind> = Vec::new();
    let mut raw: Vec<EngineEventKind> = Vec::new();

    poll_engine_until(
        &mut fixture,
        &mut wire,
        &mut raw,
        |state| state == SessionState::Playing,
        16,
        "initial priming",
    );

    // The link drops for good. The buffered data plays out and the
    // queue drains to the underrun — the no-fabrication baseline is
    // this drained point, not the loss moment (the buffered tail
    // legitimately finishes first).
    fixture.link().interrupted.store(true, Ordering::Relaxed);
    poll_engine_until(
        &mut fixture,
        &mut wire,
        &mut raw,
        |state| state == SessionState::Buffering,
        16,
        "underrun after the permanent interruption",
    );
    // Flush the buffered tail completely: the underrun fires below
    // the low watermark, so a few packets (and the demuxer's one
    // held packet) are still in flight — the no-fabrication baseline
    // is the fully drained point (virtual time still frozen: no
    // retry can fire during the flush).
    for _ in 0..4 {
        poll_round(&mut fixture, &mut wire, &mut raw);
        assert_eq!(
            fixture.engine.state(),
            SessionState::Buffering,
            "draining the buffered tail"
        );
    }
    let frames_at_loss = fixture.frames_total();
    let delivered_at_loss = fixture.link().delivered.load(Ordering::Relaxed);

    // Advance virtual time in retry-sized steps until the window is
    // spent (bounded: the guard cannot retry forever, so this loop
    // MUST terminate — the never-hang term).
    for _ in 0..64 {
        if fixture.engine.state() == SessionState::Error {
            break;
        }
        advance_now(&fixture.now, 100 * NS_PER_MS);
        poll_round(&mut fixture, &mut wire, &mut raw);
    }
    assert_eq!(
        fixture.engine.state(),
        SessionState::Error,
        "the typed fatal landed"
    );

    // The fatal is the typed network error, with the honest attempt
    // count, and its wire form is the catalog row.
    let fatal_errors = fatals(&raw);
    assert_eq!(fatal_errors.len(), 1, "exactly one fatal");
    let error = fatal_errors[0];
    assert_eq!(
        error.kind,
        ErrorKind::Network {
            cause: NetworkCause::ReconnectExhausted { attempts: 2 }
        }
    );
    assert_eq!(error.stage, PipelineStage::Open);
    assert_eq!(error.context.code, "source.retry.exhausted");

    // Wire ordering (doc 05 §3.3): the fatal `playback.error` first…
    let fatal_wire = EventKind::PlaybackError {
        stage: Stage::Input,
        code: ErrorCode::Network,
        // The F4 fix: the wire message drops Display's [stage] prefix.
        message: error.message_summary(),
        details: {
            let mut details = std::collections::BTreeMap::new();
            details.insert("context".to_string(), error.context.code.clone());
            details.insert("stage".to_string(), "open".to_string());
            details
        },
        fatal: true,
    };
    let fatal_at = wire_index(&wire, &fatal_wire).must();
    // …then the settling state change to error…
    let error_change = EventKind::SessionStateChanged {
        from: PlaybackState::Buffering,
        to: PlaybackState::Error,
    };
    let error_at = wire_index(&wire, &error_change).must();
    assert!(
        fatal_at < error_at,
        "playback.error precedes state-changed → error"
    );
    // …and the open rebuffer episode was closed (no stranded 0.0).
    assert_eq!(
        buffering_values(&wire),
        vec![0.0, 1.0, 0.0, 1.0],
        "initial episode + the rebuffer the fatal settled"
    );

    // No fabrication: nothing decoded past the buffered data, and the
    // source delivered nothing new after the loss.
    assert_eq!(fixture.frames_total(), frames_at_loss);
    for _ in 0..8 {
        poll_round(&mut fixture, &mut wire, &mut raw);
    }
    assert_eq!(
        fixture.engine.state(),
        SessionState::Error,
        "terminal, no flapping"
    );
    assert_eq!(
        fixture.link().delivered.load(Ordering::Relaxed),
        delivered_at_loss,
        "no reads were fabricated after the fatal"
    );
    assert_eq!(fixture.frames_total(), frames_at_loss);

    // The guard's accounting: one window, both retries spent.
    let link = fixture.link();
    let guard = link.guard.lock().unwrap_or_else(|p| p.into_inner());
    assert_eq!(guard.windows_opened(), 1);
    assert_eq!(guard.retries_used(), Some(2));
}

// ===========================================================================
// Engine: audio device loss
// ===========================================================================

/// **Suspend + reopen through the command layer** (doc 03 §4.9(d)):
/// the loss emits `audio.device-lost` and the suspend action is
/// realized as `playback.pause` (the engine freezes the clock — the
/// position is preserved BY the engine); the reopen emits
/// `audio.device-changed` and the resume as `playback.play`
/// (continuity from the frozen position, never a restart).
#[test]
fn engine_device_loss_suspends_and_reopen_resumes_through_the_command_layer() {
    let mut fixture = open_script_audio(200, NS_PER_S);
    let mut wire: Vec<EventKind> = Vec::new();
    let mut raw: Vec<EngineEventKind> = Vec::new();
    let mut guard = AudioDeviceGuard::new(RetryWindow::DEFAULT);

    poll_engine_until(
        &mut fixture,
        &mut wire,
        &mut raw,
        |state| state == SessionState::Playing,
        16,
        "initial priming",
    );
    let session = fixture.engine.session_id().must();
    let generation = fixture.engine.generation().must();
    let frames_at_loss = fixture.frames_total();

    // The device dies at the current position.
    let position = clock_ns(&fixture.clock);
    let step = guard.observe_device_lost(fixture.now.now_ns(), "dev0", position);
    wire.extend(step.events);
    match step.action {
        RecoveryAction::SuspendClock { position_ns } => assert_eq!(position_ns, position),
        other => unreachable!("expected SuspendClock, got {other:?}"),
    }

    // The suspend flows through the command layer: `playback.pause`.
    fixture.handle.pause(RequestId(100)).must();
    poll_engine_until(
        &mut fixture,
        &mut wire,
        &mut raw,
        |state| state == SessionState::Paused,
        4,
        "the suspend pause applies",
    );

    // Five seconds of virtual time pass while suspended: the clock
    // stays frozen at the loss position (the engine froze it — the
    // position survives by construction, not by copy).
    advance_now(&fixture.now, 5 * NS_PER_S);
    poll_round(&mut fixture, &mut wire, &mut raw);
    assert_eq!(fixture.engine.state(), SessionState::Paused);
    assert_eq!(
        clock_ns(&fixture.clock),
        position,
        "the media clock is frozen at the remembered position"
    );

    // The reopen window grants the attempt; the (fake) host
    // re-acquires the device.
    let step = guard.poll(fixture.now.now_ns());
    assert!(matches!(step.action, RecoveryAction::ReopenDevice));
    let step = guard.observe_reopen_result(fixture.now.now_ns(), Ok("dev1".to_string()));
    wire.extend(step.events);
    match step.action {
        RecoveryAction::ResumeClock { position_ns } => assert_eq!(
            position_ns, position,
            "the resume continues from the remembered position"
        ),
        other => unreachable!("expected ResumeClock, got {other:?}"),
    }

    // The resume flows through the command layer: `playback.play`.
    fixture.handle.play(RequestId(101)).must();
    poll_engine_until(
        &mut fixture,
        &mut wire,
        &mut raw,
        |state| state == SessionState::Playing,
        4,
        "the resume play applies",
    );

    // Clock continuity: one more second passes and the position is
    // remembered+1s — NOT wall-clock, NOT zero (no restart).
    advance_now(&fixture.now, NS_PER_S);
    poll_round(&mut fixture, &mut wire, &mut raw);
    assert_eq!(
        clock_ns(&fixture.clock),
        position + NS_PER_S as i64,
        "resumed from the frozen position with no discontinuity"
    );

    // Playback continues: new frames after the resume (the bounded
    // never-hang driver, specialized past the state predicate — the
    // signal here is the sink, not the machine).
    for _ in 0..16 {
        if fixture.frames_total() > frames_at_loss {
            break;
        }
        poll_round(&mut fixture, &mut wire, &mut raw);
    }
    assert!(
        fixture.frames_total() > frames_at_loss,
        "frames flow again after the resume"
    );

    // The wire: the loss event precedes its pause state change, the
    // device-changed event precedes its resume state change (the
    // cause precedes the reaction — doc 05 §4.3 causal ordering).
    let lost_at = wire_index(
        &wire,
        &EventKind::AudioDeviceLost {
            device_id: "dev0".to_string(),
        },
    )
    .must();
    let paused_at = wire_index(
        &wire,
        &EventKind::SessionStateChanged {
            from: PlaybackState::Playing,
            to: PlaybackState::Paused,
        },
    )
    .must();
    let changed_at = wire_index(
        &wire,
        &EventKind::AudioDeviceChanged {
            device_id: "dev1".to_string(),
        },
    )
    .must();
    let playing_at = wire_index(
        &wire,
        &EventKind::SessionStateChanged {
            from: PlaybackState::Paused,
            to: PlaybackState::Playing,
        },
    )
    .must();
    assert!(
        lost_at < paused_at,
        "audio.device-lost precedes the suspend"
    );
    assert!(
        changed_at < playing_at,
        "audio.device-changed precedes the resume"
    );

    // No engine restart: the same session, the same generation.
    assert_eq!(fixture.engine.session_id(), Some(session));
    assert_eq!(fixture.engine.generation(), Some(generation));
    assert_eq!(guard.losses(), 1);
    assert_eq!(guard.reopens(), 1);
}

/// **Suspend-and-notify, end-to-end** (the doc's `[PROPOSAL]`
/// default): an exhausted reopen window leaves the session `paused`
/// — NOT `error` — with exactly one non-fatal notice on the wire,
/// the clock still frozen at the remembered position, and no further
/// reopen attempts granted.
#[test]
fn engine_device_reopen_exhaustion_suspends_and_notifies_non_fatal() {
    let policy = RetryWindow {
        max_attempts: 2,
        window_ns: 10 * NS_PER_S,
        base_backoff_ns: NS_PER_S,
        max_backoff_ns: 2 * NS_PER_S,
    };
    let mut fixture = open_script_audio(200, NS_PER_S);
    let mut wire: Vec<EventKind> = Vec::new();
    let mut raw: Vec<EngineEventKind> = Vec::new();
    let mut guard = AudioDeviceGuard::new(policy);

    poll_engine_until(
        &mut fixture,
        &mut wire,
        &mut raw,
        |state| state == SessionState::Playing,
        16,
        "initial priming",
    );
    let position = clock_ns(&fixture.clock);

    // Loss → suspend (through the command layer, as above).
    let step = guard.observe_device_lost(fixture.now.now_ns(), "dev0", position);
    wire.extend(step.events);
    fixture.handle.pause(RequestId(200)).must();
    poll_engine_until(
        &mut fixture,
        &mut wire,
        &mut raw,
        |state| state == SessionState::Paused,
        4,
        "the suspend pause applies",
    );

    // Both reopen attempts fail (the second spends the budget).
    assert!(matches!(
        guard.poll(fixture.now.now_ns()).action,
        RecoveryAction::ReopenDevice
    ));
    let step = guard.observe_reopen_result(fixture.now.now_ns(), Err(device_gone()));
    assert!(
        step.events.is_empty(),
        "a failed attempt inside the window is silent"
    );
    advance_now(&fixture.now, 2 * NS_PER_S);
    assert!(matches!(
        guard.poll(fixture.now.now_ns()).action,
        RecoveryAction::ReopenDevice
    ));
    let step = guard.observe_reopen_result(fixture.now.now_ns(), Err(device_gone()));
    match step.events.as_slice() {
        [
            EventKind::PlaybackError {
                stage, code, fatal, ..
            },
        ] => {
            assert_eq!(
                (*stage, *code, *fatal),
                (Stage::AudioOutput, ErrorCode::Device, false)
            );
        }
        other => unreachable!("expected the one non-fatal notice, got {other:?}"),
    }
    wire.extend(step.events);

    // The session stays suspended — the notice is NON-fatal: no error
    // state, no teardown, the clock still frozen at the position.
    for _ in 0..4 {
        advance_now(&fixture.now, NS_PER_S);
        poll_round(&mut fixture, &mut wire, &mut raw);
        assert_eq!(
            fixture.engine.state(),
            SessionState::Paused,
            "suspended, not errored"
        );
    }
    assert_eq!(clock_ns(&fixture.clock), position);
    assert!(guard.suspended());
    assert_eq!(guard.suspended_position_ns(), Some(position));

    // Bounded: no reopen is granted after the notice, however far
    // virtual time runs.
    advance_now(&fixture.now, 60 * NS_PER_S);
    let step = guard.poll(fixture.now.now_ns());
    assert!(
        matches!(step.action, RecoveryAction::None),
        "the suspend is terminal until a new loss or a host reopen"
    );

    // Exactly one device-lost and one notice on the wire — and no
    // fatal error anywhere.
    assert_eq!(
        wire.iter()
            .filter(|event| matches!(event, EventKind::AudioDeviceLost { .. }))
            .count(),
        1
    );
    assert_eq!(
        wire.iter()
            .filter(|event| matches!(event, EventKind::PlaybackError { fatal: true, .. }))
            .count(),
        0,
        "a failed device is not a fatal media error"
    );
    assert!(fatals(&raw).is_empty());
}

// ===========================================================================
// Engine: renderer surface loss
// ===========================================================================

/// **Degradation + reattachment without engine restart** (doc 03
/// §4.11(d)): the loss emits the non-fatal audio-only event and
/// requests a recreate; the session stays `playing` (same session,
/// same generation — no teardown), video presents are suppressed
/// while audio frames keep flowing; the recreate resumes compositing
/// and the media finishes normally.
#[test]
fn engine_surface_loss_degrades_to_audio_only_then_reattaches_without_restart() {
    let mut fixture = open_script_av(60, NS_PER_S);
    let mut wire: Vec<EventKind> = Vec::new();
    let mut raw: Vec<EngineEventKind> = Vec::new();
    let mut guard = SurfaceGuard::new();

    poll_engine_until(
        &mut fixture,
        &mut wire,
        &mut raw,
        |state| state == SessionState::Playing,
        16,
        "initial priming",
    );
    let session = fixture.engine.session_id().must();
    let generation = fixture.engine.generation().must();
    let audio_at_loss = fixture.frames_of_kind(MediaKind::Audio);

    // The surface dies: the loss is a renderer-level fact — the
    // ENGINE neither knows nor cares (no state change, no teardown).
    let wire_len_at_loss = wire.len();
    let step = guard.observe_surface_lost();
    wire.extend(step.events);
    assert!(matches!(
        step.action,
        RecoveryAction::RequestSurfaceRecreate
    ));

    // Audio-only degradation: polls continue, audio frames keep
    // flowing, every video present is suppressed at the gate.
    let mut presented = 0usize;
    for _ in 0..6 {
        poll_round(&mut fixture, &mut wire, &mut raw);
        let video_now = fixture.frames_of_kind(MediaKind::Video);
        for _ in 0..video_now.saturating_sub(guard.suppressed_presents() as usize + presented) {
            // One gate consult per newly decoded video frame (the
            // renderer's present decision).
            if guard.gate_present() {
                presented += 1;
            }
        }
        assert_eq!(
            fixture.engine.state(),
            SessionState::Playing,
            "still playing"
        );
    }
    assert!(
        fixture.frames_of_kind(MediaKind::Audio) > audio_at_loss,
        "audio continues through the degradation"
    );
    assert!(
        guard.suppressed_presents() > 0,
        "video presents were suppressed"
    );
    assert_eq!(presented, 0, "nothing presented while dirty");

    // No state change, no error, no teardown crossed the wire during
    // the loss window (the engine stream is untouched).
    assert!(
        !wire[wire_len_at_loss..]
            .iter()
            .any(|event| matches!(event, EventKind::SessionStateChanged { .. })),
        "the session never left playing"
    );
    assert!(fatals(&raw).is_empty());

    // The host provides a fresh surface: compositing resumes — no
    // engine restart (same session id, same generation).
    let step = guard.observe_surface_recreated();
    assert!(matches!(step.action, RecoveryAction::None));
    assert!(guard.compositing());
    assert_eq!(fixture.engine.session_id(), Some(session));
    assert_eq!(fixture.engine.generation(), Some(generation));

    // The media finishes normally; video presents flow again.
    poll_engine_until(
        &mut fixture,
        &mut wire,
        &mut raw,
        |state| state == SessionState::Ended,
        128,
        "drain to ended after reattachment",
    );
    let suppressed_at_recreate = guard.suppressed_presents();
    assert!(guard.gate_present(), "presents are allowed again");
    assert_eq!(guard.suppressed_presents(), suppressed_at_recreate);
    assert_eq!(guard.recreations(), 1);
    assert_eq!(
        fixture.frames_total(),
        120,
        "every frame of both limbs delivered"
    );
    // The only state changes on the wire are the session's own
    // lifecycle: idle→opening→buffering→playing→ended — the loss
    // never moved the machine.
    assert_eq!(
        state_changes(&wire),
        vec![
            (PlaybackState::Idle, PlaybackState::Opening),
            (PlaybackState::Opening, PlaybackState::Buffering),
            (PlaybackState::Buffering, PlaybackState::Playing),
            (PlaybackState::Playing, PlaybackState::Ended),
        ]
    );
}
