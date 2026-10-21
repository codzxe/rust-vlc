//! The seek acceptance suite (card 3.A1, doc 03 §6): the seven-step
//! flow driven end-to-end through [`Engine::poll`] with fake stage
//! objects — a **seekable** scripted demuxer (compute-then-commit
//! reanchor honoring D4), scripted decoders, the collecting sink, and
//! a virtual clock.
//!
//! What is proven here (the pure-model proofs live in `table.rs`,
//! the resolution/clamping unit tests in `seek.rs`):
//!
//! - the **seven steps** from `Playing`: accept (class-A receipt with
//!   the bumped generation), the buffering episode, the flush +
//!   re-anchor, the pre-roll, the clock re-anchor on the **audio
//!   frame's own timestamp** (not the request), and the
//!   `Seeked{requested, actual}` terminal with its origin;
//! - **boundary seeks**: zero (lands at the start), past-end (clamps
//!   to the declared duration — clamping is truth, doc 05 §2.4 — and
//!   the media then ends cleanly);
//! - **contract D4**: an unseekable source refuses typed
//!   (`SeekFailed{NotSeekable}`) with playback continuing at the old
//!   position, and a failed reanchor leaves the session playable;
//! - **supersession**: a second seek mid-flight retires the older
//!   generation silently — exactly one terminal, for the newest
//!   generation (doc 05 §4.2/§4.3);
//! - **stale purge**: the decoder's old-generation reorder buffer is
//!   dropped and counted at the seek flush, and no old-generation
//!   frame or pre-landing timestamp ever reaches the sink after the
//!   seek completes (doc 03 §5 checking point 5);
//! - the **seek-terminal drain exemption**: a `Seeked` already
//!   emitted for a completed generation survives a later mint in the
//!   outbox (doc 05 §4.3 invariant 4);
//! - the **Paused decode carve-out** [PROPOSAL: 3.HARD-AUDIT D-a]:
//!   a seek accepted while `Paused` completes without the wire state
//!   ever leaving `paused`.

use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
use commands::envelope::{Generation, RequestId};
use commands::{EventKind, SessionId};
use media_clock::MonotonicSource;
use media_types::error::ErrorKind;
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
    FrameConsumer, QueuePlan, ReceiptResult, SeekFailure, SeekTarget, SessionCommandKind,
    SessionConfig, SessionError, SessionHandle, TickBudget,
};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};
use testkit::Must;

// ---------------------------------------------------------------------------
// Fakes
// ---------------------------------------------------------------------------

/// Shared virtual monotonic time (doc 03 §8: no sleeps). It never
/// advances unless a test says so, so clock-position assertions are
/// deterministic.
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

/// The scripted media: the full packet list in demux order plus the
/// cursor, shared with the test so failure-arming is possible
/// mid-run. Packets are all keyframes (the MJPEG-in-AVI shape).
#[derive(Default)]
struct Media {
    packets: Vec<Packet>,
    cursor: usize,
    fail_reanchor: Option<EngineError>,
}

fn lock_media(media: &Arc<Mutex<Media>>) -> std::sync::MutexGuard<'_, Media> {
    media
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The seekable scripted demux stage: emits the media in order, then
/// end-of-stream. `reanchor` is compute-then-commit (D4): a landing is
/// computed from the packet list and the cursor moves only on success
/// — an armed failure leaves the stream exactly where it stood.
struct SeekDemuxer {
    tracks: Vec<Track>,
    media: Arc<Mutex<Media>>,
    seekable: bool,
}

impl Demuxer for SeekDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        Ok(self.tracks.clone())
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        let mut media = lock_media(&self.media);
        if let Some(packet) = media.packets.get(media.cursor) {
            let packet = packet.clone();
            media.cursor += 1;
            return Ok(DemuxEvent::Packet(packet));
        }
        Ok(DemuxEvent::EndOfStream)
    }

    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
        // The pure query is not the engine's seek path (doc 03 §6
        // step 4 goes through `reanchor`); the honest refusal.
        Err(EngineError::new(
            PipelineStage::Demux,
            ErrorKind::Unsupported { format: None },
            media_types::error::ErrorContext::new("demux.seek.not-the-reanchor-path"),
        ))
    }

    fn seekability(&self) -> SeekSupport {
        if self.seekable {
            SeekSupport::Fast { indexed: true }
        } else {
            SeekSupport::None
        }
    }

    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        let mut media = lock_media(&self.media);
        // D4: an armed failure refuses before anything moves.
        if let Some(error) = media.fail_reanchor.take() {
            return Err(error);
        }
        let Some(target_ns) = stamp_ns(&target.start) else {
            return Err(reanchor_refusal("demux.reanchor.target-missing"));
        };
        // The keyframe-safe position ≤ target (all packets are
        // keyframes in this fake): the last packet at or before it.
        let landing = media
            .packets
            .iter()
            .rposition(|packet| stamp_ns(&packet.pts) <= Some(target_ns))
            .unwrap_or(0);
        media.cursor = landing;
        let start = media
            .packets
            .get(landing)
            .map_or(Stamp::Missing, |packet| packet.pts);
        Ok(TimeSpan {
            start,
            end: Stamp::Missing,
        })
    }
}

/// A present stamp's nanoseconds.
fn stamp_ns(stamp: &Stamp) -> Option<i64> {
    let timestamp = stamp.timestamp().ok()?;
    match timestamp.as_nanos() {
        Ok(Nanoseconds(ns)) => Some(i64::try_from(ns).unwrap_or(i64::MAX)),
        Err(_) => None,
    }
}

fn reanchor_refusal(code: &'static str) -> EngineError {
    EngineError::new(
        PipelineStage::Demux,
        ErrorKind::Corrupt {
            detail: media_types::error::CorruptDetail::InvalidStructure { offset: None },
        },
        media_types::error::ErrorContext::new(code),
    )
}

/// The scripted decode stage: one frame out per packet (the frame
/// carries the packet's own pts and generation — the honest shape).
struct FakeDecoder {
    generation: GenerationId,
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
        if packet.is_stale(self.generation) {
            return Ok(DecodeOutcome::NeedMoreInput);
        }
        out.accept(audio_frame(packet.pts, packet.generation))?;
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        Ok(DecodeOutcome::Drained)
    }
}

/// The reorder-buffer decode stage (the stale-purge shape): holds one
/// packet back and emits the *previous* packet's frame — so at a seek
/// flush it still owes the pipeline one old-generation frame, which
/// the generation-checking sink must drop and count (doc 03 §5
/// checking point 5).
struct ReorderDecoder {
    generation: GenerationId,
    held: Option<Packet>,
}

impl Decoder for ReorderDecoder {
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
        let Some(held) = self.held.take() else {
            self.held = Some(packet);
            return Ok(DecodeOutcome::NeedMoreInput);
        };
        // The frame carries the *held* packet's truth: its own pts and
        // its own generation.
        out.accept(audio_frame(held.pts, held.generation))?;
        self.held = Some(packet);
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }

    fn flush(&mut self, out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        if let Some(held) = self.held.take() {
            out.accept(audio_frame(held.pts, held.generation))?;
            return Ok(DecodeOutcome::Emitted { frames: 1 });
        }
        Ok(DecodeOutcome::Drained)
    }
}

/// The decoder seam: hands every track the scripted decoder the test
/// chose.
enum ProviderKind {
    Immediate,
    Reorder,
}

struct FakeProvider {
    kind: ProviderKind,
}

impl DecoderProvider for FakeProvider {
    fn decoder_for(
        &self,
        _track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        Ok(match self.kind {
            ProviderKind::Immediate => Box::new(FakeDecoder {
                generation: config.generation,
            }),
            ProviderKind::Reorder => Box::new(ReorderDecoder {
                generation: config.generation,
                held: None,
            }),
        })
    }
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

/// A track whose declared extent ends at `end_us` (`None` leaves the
/// span unknown — the duration-less shape).
fn track(id: u32, kind: MediaKind, end_us: Option<i64>) -> Track {
    Track {
        id: TrackId::new(id),
        stream: media_types::StreamId::new(id),
        kind,
        language: None,
        codec: media_types::ids::CodecDescriptor::new("fake-pcm").must(),
        title: None,
        flags: TrackFlags::NONE,
        timing: TrackTiming {
            span: match end_us {
                Some(end) => TimeSpan {
                    start: at_micros(0),
                    end: at_micros(end),
                },
                None => TimeSpan::UNKNOWN,
            },
            base: base(),
            gaps: Vec::new(),
        },
    }
}

fn packet(track: TrackId, pts_us: i64) -> Packet {
    Packet {
        track,
        pts: at_micros(pts_us),
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

/// Small deterministic queue caps: high watermark at 12 resident, low
/// at 4 (the session_flow numbers — the re-prime math relies on
/// them).
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

/// Everything one seek test needs.
struct Fixture {
    engine: Engine,
    handle: SessionHandle,
    collector: Arc<Collector>,
    clock: Arc<media_clock::MediaClock>,
    media: Arc<Mutex<Media>>,
}

/// Open a session over a single audio track of `count` packets at
/// 1 ms strides (packet k has pts k ms), declaring the extent when
/// `declared_end` is `Some`.
fn open_audio(count: u64, declared_end: Option<i64>, seekable: bool) -> Fixture {
    let packets: Vec<Packet> = (0..count)
        .map(|index| packet(TrackId::new(1), index as i64))
        .collect();
    open(
        vec![track(1, MediaKind::Audio, declared_end)],
        packets,
        seekable,
        ProviderKind::Immediate,
    )
}

/// Open a session over an interleaved audio+video media: video (id 1)
/// at even milliseconds, audio (id 2) at odd milliseconds, in
/// presentation order.
fn open_audio_video(count_each: u64) -> Fixture {
    let mut packets = Vec::new();
    for step in 0..count_each {
        packets.push(packet(TrackId::new(1), step as i64 * 2));
        packets.push(packet(TrackId::new(2), step as i64 * 2 + 1));
    }
    open(
        vec![
            track(1, MediaKind::Video, Some(count_each as i64 * 2)),
            track(2, MediaKind::Audio, Some(count_each as i64 * 2)),
        ],
        packets,
        true,
        ProviderKind::Immediate,
    )
}

fn open(
    tracks: Vec<Track>,
    packets: Vec<Packet>,
    seekable: bool,
    provider: ProviderKind,
) -> Fixture {
    let media = Arc::new(Mutex::new(Media {
        packets,
        cursor: 0,
        fail_reanchor: None,
    }));
    let collector = Arc::new(Collector::default());
    let clock = Arc::new(
        media_clock::MediaClock::system_monotonic(
            Arc::new(SharedNow(AtomicU64::new(0))),
            Timestamp::new(0, base()),
            media_types::Rate::ONE,
        )
        .must(),
    );
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-seek-fake.media").must(),
        Box::new(SeekDemuxer {
            tracks,
            media: Arc::clone(&media),
            seekable,
        }),
        Arc::new(FakeProvider { kind: provider }),
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
        media,
    }
}

/// Drive polls until `predicate` holds on the engine state, or fail
/// loudly after `limit` polls (a hang is a failed test — doc 03
/// §4.6(f)).
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

/// The collector's frame list, snapshotted under one lock.
fn frames_snapshot(collector: &Collector) -> Vec<(TrackId, Frame)> {
    collector
        .frames
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
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

/// Every `Seeked` event in the output stream with its generation.
fn seeked_terms(outputs: &[EngineOutput]) -> Vec<(Generation, i64, i64)> {
    outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Event(EngineEvent {
                generation,
                kind: EngineEventKind::Seeked { requested, actual },
                ..
            }) => Some((*generation, *requested, *actual)),
            _ => None,
        })
        .collect()
}

/// Every `SeekFailed` event in the output stream.
fn seek_failed_terms(outputs: &[EngineOutput]) -> Vec<(Generation, SeekFailure, i64)> {
    outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Event(EngineEvent {
                generation,
                kind: EngineEventKind::SeekFailed { reason, position },
                ..
            }) => Some((*generation, reason.clone(), *position)),
            _ => None,
        })
        .collect()
}

/// The index of the first event matching the kind predicate.
fn event_index(outputs: &[EngineOutput], matches: impl Fn(&EngineEventKind) -> bool) -> Option<usize> {
    outputs.iter().position(|output| match output {
        EngineOutput::Event(event) => matches(&event.kind),
        EngineOutput::Receipt(_) => false,
    })
}

/// Milliseconds, in nanoseconds (the engine's interchange unit).
const MS: i64 = 1_000_000;

// ---------------------------------------------------------------------------
// The seven steps, happy path
// ---------------------------------------------------------------------------

#[test]
fn a_happy_seek_from_playing_runs_the_seven_steps() {
    let mut fixture = open_audio(400, Some(400), true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    let frames_before = frames_of(&fixture.collector);

    // Step 1 — accept: the class-A receipt carries the NEW generation
    // (doc 05 §4.2: the wire gen moves at accept time).
    fixture
        .handle
        .seek(RequestId(50), SeekTarget::Absolute { position_ns: 200 * MS })
        .must();
    fixture.engine.poll();
    // Steps 2–3: the row's mint + flush opened the seeking episode.
    assert_eq!(fixture.engine.state(), SessionState::Buffering);
    // Steps 4–5: the re-anchor landed; the pre-roll refills and the
    // watermark re-enters Playing (the completion fires in the same
    // tick as the priming — the pre-roll only needs one frame per
    // limb, the priming needs the queue high).
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "re-prime");

    let outputs = take_events(&mut fixture);
    let receipt = receipt_for(&outputs, RequestId(50));
    assert_eq!(
        receipt.result,
        ReceiptResult::Accepted {
            session: SessionId(1),
            generation: Generation(1),
        }
    );
    // Step 7 — the terminal: requested vs actual, under the seek's
    // generation, with the causing request as origin.
    assert_eq!(
        seeked_terms(&outputs),
        vec![(Generation(1), 200 * MS, 200 * MS)]
    );
    let seeked_index = event_index(&outputs, |kind| {
        matches!(kind, EngineEventKind::Seeked { .. })
    })
    .must();
    match &outputs[seeked_index] {
        EngineOutput::Event(event) => assert_eq!(event.origin, Some(RequestId(50))),
        EngineOutput::Receipt(_) => unreachable!("positioned an event"),
    }
    // The terminal precedes the episode's close and the resume (the
    // seek completes, then the session resumes).
    let exit_index = event_index(&outputs, |kind| {
        matches!(kind, EngineEventKind::Buffering { progress: 1.0 })
    })
    .must();
    assert!(seeked_index < exit_index);
    // The wire form (doc 05 §3.2 timing.seek-completed, µs).
    let wire = match &outputs[seeked_index] {
        EngineOutput::Event(event) => event.as_wire_event().must(),
        EngineOutput::Receipt(_) => unreachable!("positioned an event"),
    };
    assert_eq!(
        wire.kind,
        EventKind::TimingSeekCompleted {
            position: commands::catalog::types::Timestamp(200_000),
            seek_kind: commands::catalog::types::SeekKind::Command,
        }
    );
    // Step 6 — the clock re-anchored on the first audio frame's own
    // timestamp (the landing packet's pts: 200 ms).
    let anchor = fixture.clock.anchor();
    assert_eq!(anchor.media.ticks, 200 * MS);
    // The media continues from the landing and drains to Ended —
    // with exactly one terminal, ever.
    poll_until(&mut fixture, |s| s == SessionState::Ended, 400, "drain");
    let outputs = take_events(&mut fixture);
    assert!(
        seeked_terms(&outputs).is_empty(),
        "no second terminal for the completed generation"
    );
    // Every post-seek frame is at or after the landing; the pre-seek
    // delivery is untouched history below it.
    let frames = frames_snapshot(&fixture.collector);
    assert!(frames.len() > frames_before);
    for (index, (_, frame)) in frames.iter().enumerate() {
        let pts_ns = stamp_ns(&frame.pts()).must();
        if index < frames_before {
            assert!(pts_ns < 200 * MS, "pre-seek history is below the landing");
        } else {
            assert!(
                pts_ns >= 200 * MS,
                "a pre-landing timestamp reappeared post-seek: {pts_ns} ns"
            );
        }
    }
}

#[test]
fn the_audio_frame_timestamp_is_the_landing_truth_not_the_request() {
    // Video at even ms, audio at odd ms: a 100 ms target lands on the
    // video keyframe at 100 ms, but the first *audio* frame is
    // 101 ms — and that is what the clock and the terminal report
    // (doc 03 §6 step 6: anchor on truth, not on request).
    let mut fixture = open_audio_video(200);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    fixture
        .handle
        .seek(RequestId(51), SeekTarget::Absolute { position_ns: 100 * MS })
        .must();
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "re-prime");

    let outputs = take_events(&mut fixture);
    assert_eq!(
        seeked_terms(&outputs),
        vec![(Generation(1), 100 * MS, 101 * MS)],
        "actual is the first audio frame's own timestamp"
    );
    let anchor = fixture.clock.anchor();
    assert_eq!(anchor.media.ticks, 101 * MS);
    // Both limbs delivered post-seek content.
    let frames = frames_snapshot(&fixture.collector);
    let post_landing: Vec<u32> = frames
        .iter()
        .filter(|(_, frame)| stamp_ns(&frame.pts()).must() >= 100 * MS)
        .map(|(track, _)| track.handle())
        .collect();
    assert!(post_landing.contains(&1), "the video limb delivered");
    assert!(post_landing.contains(&2), "the audio limb delivered");
    // The first audio-limb frame post-landing is the 101 ms one.
    let first_audio_ns = frames
        .iter()
        .filter(|(track, _)| track.handle() == 2)
        .map(|(_, frame)| stamp_ns(&frame.pts()).must())
        .filter(|pts| *pts >= 100 * MS)
        .min()
        .must();
    assert_eq!(first_audio_ns, 101 * MS);
}

// ---------------------------------------------------------------------------
// Paused seeks — the decode carve-out [PROPOSAL: 3.HARD-AUDIT D-a]
// ---------------------------------------------------------------------------

#[test]
fn a_seek_from_paused_completes_without_leaving_paused() {
    let mut fixture = open_audio(400, Some(400), true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    fixture.handle.pause(RequestId(60)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Paused);
    // Drain the pre-pause history so the later ordering proof sees
    // only the paused era.
    let _ = take_events(&mut fixture);
    let frames_at_pause = frames_of(&fixture.collector);
    assert!(frames_at_pause > 0);

    // The seek runs while Paused: the wire state never moves.
    fixture
        .handle
        .seek(RequestId(61), SeekTarget::Absolute { position_ns: 150 * MS })
        .must();
    let mut saw_terminal = None;
    for _ in 0..10 {
        fixture.engine.poll();
        assert_eq!(
            fixture.engine.state(),
            SessionState::Paused,
            "the wire-visible state stays paused"
        );
        if let Some(term) = seeked_terms(&take_events(&mut fixture)).first().copied() {
            saw_terminal = Some(term);
            break;
        }
    }
    let term = match saw_terminal {
        Some(term) => term,
        None => unreachable!("the paused seek never completed"),
    };
    assert_eq!(term, (Generation(1), 150 * MS, 150 * MS));
    // The clock is still frozen — but re-anchored at the landing
    // (the freeze moves with the re-anchor, the state does not).
    assert!(fixture.clock.is_paused());
    assert_eq!(fixture.clock.anchor().media.ticks, 150 * MS);
    // The carve-out decoded the landing (a bounded slice: one decode
    // budget's worth), then stopped — paused sinks stay frozen.
    let frames_at_completion = frames_of(&fixture.collector);
    let grown = frames_at_completion - frames_at_pause;
    assert!(
        grown > 0 && grown <= 4,
        "the landing frame may present, bounded by one decode slice: {grown}"
    );
    let mut quiet = true;
    for _ in 0..4 {
        fixture.engine.poll();
        quiet = quiet && frames_of(&fixture.collector) == frames_at_completion;
    }
    assert!(quiet, "post-completion paused decode stays frozen");
    // The state-changed events of the paused era show only the
    // self-confirmation — no leaving of paused.
    let outputs = take_events(&mut fixture);
    let left_paused = outputs.iter().any(|output| {
        matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::StateChanged { to, .. },
                ..
            }) if *to != SessionState::Paused
        )
    });
    assert!(!left_paused, "no state-changed out of paused");
    // And the session is still a functional paused session: resume
    // continues from the landing.
    fixture.handle.play(RequestId(62)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Playing);
    poll_until(&mut fixture, |s| s == SessionState::Ended, 400, "drain");
}

// ---------------------------------------------------------------------------
// Boundary seeks
// ---------------------------------------------------------------------------

#[test]
fn a_seek_to_zero_lands_at_the_start() {
    let mut fixture = open_audio(400, Some(400), true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    let frames_before = frames_of(&fixture.collector);
    fixture
        .handle
        .seek(RequestId(63), SeekTarget::Absolute { position_ns: 0 })
        .must();
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "re-prime");
    let outputs = take_events(&mut fixture);
    assert_eq!(seeked_terms(&outputs), vec![(Generation(1), 0, 0)]);
    // The first post-seek frame is the media's first frame again.
    let frames = frames_snapshot(&fixture.collector);
    let first_post = stamp_ns(&frames[frames_before].1.pts()).must();
    assert_eq!(first_post, 0, "packet 0 re-delivered after the boundary seek");
    // A negative absolute target clamps to zero (truth, not failure).
    fixture
        .handle
        .seek(RequestId(64), SeekTarget::Absolute { position_ns: -5 * MS })
        .must();
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "re-prime");
    let outputs = take_events(&mut fixture);
    assert_eq!(
        seeked_terms(&outputs),
        vec![(Generation(2), 0, 0)],
        "negative clamps to zero"
    );
}

#[test]
fn a_past_end_seek_clamps_to_the_duration_then_ends_cleanly() {
    // 40 packets of 1 ms with a declared 40 ms extent: a far-past-end
    // target clamps to the duration (doc 05 §2.4 — clamping is
    // truth), lands on the last keyframe (39 ms), and the media then
    // ends cleanly.
    let mut fixture = open_audio(40, Some(40), true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    let frames_before = frames_of(&fixture.collector);
    fixture
        .handle
        .seek(
            RequestId(65),
            SeekTarget::Absolute {
                position_ns: 999_999 * MS,
            },
        )
        .must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Buffering);
    // The clamped target is 40 ms; the landing is the last keyframe
    // at or before it: packet 39. The short tail starts, the drain
    // ends the media.
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "short start");
    poll_until(&mut fixture, |s| s == SessionState::Ended, 40, "clean end");
    let outputs = take_events(&mut fixture);
    assert_eq!(
        seeked_terms(&outputs),
        vec![(Generation(1), 40 * MS, 39 * MS)],
        "requested is the clamped target; actual is the last keyframe"
    );
    // The end arrived with the documented event chain.
    let has_end = event_index(&outputs, |kind| {
        matches!(kind, EngineEventKind::EndOfMedia)
    })
    .is_some();
    assert!(has_end);
    // Exactly the landing frame was new (packet 39); the pre-seek
    // delivery is untouched history.
    assert_eq!(frames_of(&fixture.collector), frames_before + 1);
}

// ---------------------------------------------------------------------------
// Contract D4 — failed seeks
// ---------------------------------------------------------------------------

#[test]
fn an_unseekable_source_refuses_typed_and_playback_continues() {
    let mut fixture = open_audio(400, Some(400), false);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    let frames_before = frames_of(&fixture.collector);

    fixture
        .handle
        .seek(RequestId(70), SeekTarget::Absolute { position_ns: 200 * MS })
        .must();
    fixture.engine.poll();
    // The accept is still class A with the bumped generation (doc 05
    // §2.4/§4.2) — the refusal is the terminal, not the receipt.
    let outputs = take_events(&mut fixture);
    let receipt = receipt_for(&outputs, RequestId(70));
    assert_eq!(
        receipt.result,
        ReceiptResult::Accepted {
            session: SessionId(1),
            generation: Generation(1),
        }
    );
    // Exactly one typed refusal, never a fake success.
    let failures = seek_failed_terms(&outputs);
    assert_eq!(failures.len(), 1, "exactly one SeekFailed");
    assert_eq!(failures[0].1, SeekFailure::NotSeekable);
    assert_eq!(failures[0].0, Generation(1));
    // The wire form (doc 05 §3.2 timing.seek-failed).
    let failed_index = event_index(&outputs, |kind| {
        matches!(kind, EngineEventKind::SeekFailed { .. })
    })
    .must();
    let wire = match &outputs[failed_index] {
        EngineOutput::Event(event) => event.as_wire_event().must(),
        EngineOutput::Receipt(_) => unreachable!("positioned an event"),
    };
    let error = match wire.kind {
        EventKind::TimingSeekFailed { error } => error,
        other => unreachable!("SeekFailed maps to TimingSeekFailed, got {other:?}"),
    };
    assert_eq!(error.stage, commands::Stage::Control);
    assert_eq!(error.code, commands::ErrorCode::Unsupported);
    assert_eq!(
        error.details.get("reason").map(String::as_str),
        Some("not-seekable")
    );
    // Playback continues at the old position: the row's buffering
    // episode re-primes (nothing was flushed), the media plays on.
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "re-prime");
    assert!(frames_of(&fixture.collector) > frames_before);
    poll_until(&mut fixture, |s| s == SessionState::Ended, 400, "drain");
    let outputs = take_events(&mut fixture);
    assert!(
        seeked_terms(&outputs).is_empty(),
        "never a fake success for an unseekable source"
    );
    // No content was dropped either: the unseekable refusal never
    // touched the pipeline (all 400 frames delivered).
    assert_eq!(frames_of(&fixture.collector), 400);
}

#[test]
fn a_failed_reanchor_rolls_back_d4_and_stays_playable() {
    let mut fixture = open_audio(400, Some(400), true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    // Arm the reanchor failure (compute-then-commit: the armed refusal
    // never moves the cursor).
    lock_media(&fixture.media).fail_reanchor = Some(reanchor_refusal("demux.reanchor.armed"));

    fixture
        .handle
        .seek(RequestId(71), SeekTarget::Absolute { position_ns: 200 * MS })
        .must();
    fixture.engine.poll();
    // The failure surfaces in the same poll (steps 3–4 are
    // synchronous); the state still took the row's buffering episode
    // and now rebuffers — the demuxer never moved.
    let outputs = take_events(&mut fixture);
    let failures = seek_failed_terms(&outputs);
    assert_eq!(failures.len(), 1);
    match &failures[0].1 {
        SeekFailure::Demux(error) => {
            assert_eq!(error.context.code, "demux.reanchor.armed");
        }
        other => unreachable!("the armed demuxer refusal, got {other:?}"),
    }
    assert_eq!(
        failures[0].2, 0,
        "the reported position is the old (never-moved) one"
    );
    // D4: playable — the rebuffer refills from where the demuxer
    // stands and the media runs to its end.
    poll_until(&mut fixture, |s| s == SessionState::Playing, 40, "D4 rebuffer");
    poll_until(&mut fixture, |s| s == SessionState::Ended, 400, "D4 drain");
    let outputs = take_events(&mut fixture);
    assert!(
        seeked_terms(&outputs).is_empty(),
        "a failed seek never reports success"
    );
    // Content integrity: every delivered timestamp is distinct, and
    // the loss is bounded by what the flush dropped (the queue slice
    // at the accept — never a backward jump, never a repeat).
    let frames = frames_snapshot(&fixture.collector);
    let mut pts: Vec<i64> = frames
        .iter()
        .map(|(_, frame)| stamp_ns(&frame.pts()).must())
        .collect();
    let delivered = pts.len();
    pts.sort_unstable();
    pts.dedup();
    assert_eq!(pts.len(), delivered, "no timestamp ever repeats");
    assert!(
        delivered >= 400 - 20,
        "the flush loss is bounded by the queue slice: {delivered}"
    );
}

#[test]
fn percent_without_a_duration_is_the_typed_out_of_range() {
    // No declared extent: percent targets are unresolvable (doc 05
    // §2.4) — typed refusal, playback untouched.
    let mut fixture = open_audio(400, None, true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    fixture
        .handle
        .seek(RequestId(72), SeekTarget::Percent { fraction: 0.5 })
        .must();
    fixture.engine.poll();
    let outputs = take_events(&mut fixture);
    let receipt = receipt_for(&outputs, RequestId(72));
    assert!(receipt.result.accepted_generation().is_some());
    let failures = seek_failed_terms(&outputs);
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].1, SeekFailure::OutOfRange);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "re-prime");
    poll_until(&mut fixture, |s| s == SessionState::Ended, 400, "drain");
    let outputs = take_events(&mut fixture);
    assert!(seeked_terms(&outputs).is_empty());
    assert_eq!(frames_of(&fixture.collector), 400, "nothing was dropped");
}

// ---------------------------------------------------------------------------
// Supersession (doc 05 §4.2/§4.3)
// ---------------------------------------------------------------------------

#[test]
fn a_superseded_seek_is_silent_only_the_newest_terminal_fires() {
    // Two seeks accepted before a single tick (the repeated-seek
    // storm shape, doc 03 §6): the first is superseded in-flight and
    // emits no terminal; the second completes and reports.
    let mut fixture = open_audio(400, Some(400), true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    fixture
        .handle
        .seek(RequestId(80), SeekTarget::Absolute { position_ns: 100 * MS })
        .must();
    fixture
        .handle
        .seek(RequestId(81), SeekTarget::Absolute { position_ns: 250 * MS })
        .must();
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "re-prime");
    poll_until(&mut fixture, |s| s == SessionState::Ended, 400, "drain");

    let outputs = take_events(&mut fixture);
    // Both accepts answer class A — each under its own mint.
    let first = receipt_for(&outputs, RequestId(80));
    let second = receipt_for(&outputs, RequestId(81));
    assert_eq!(first.result.accepted_generation(), Some(Generation(1)));
    assert_eq!(second.result.accepted_generation(), Some(Generation(2)));
    // Exactly one terminal: the newest generation's, at its landing.
    assert_eq!(
        seeked_terms(&outputs),
        vec![(Generation(2), 250 * MS, 250 * MS)],
        "the superseded generation is silent"
    );
    assert!(seek_failed_terms(&outputs).is_empty());
}

#[test]
fn a_resolution_failure_does_not_retire_an_in_flight_seek() {
    // The point of no return is the flush (see seek.rs): a second
    // seek that fails *resolution* leaves the older in-flight seek
    // running — both generations get their exactly-one terminal.
    let mut fixture = open_audio(400, None, true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    fixture
        .handle
        .seek(RequestId(82), SeekTarget::Absolute { position_ns: 100 * MS })
        .must();
    // Percent without a declared duration: unresolvable.
    fixture
        .handle
        .seek(RequestId(83), SeekTarget::Percent { fraction: 0.5 })
        .must();
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "re-prime");

    let outputs = take_events(&mut fixture);
    assert_eq!(
        seeked_terms(&outputs),
        vec![(Generation(1), 100 * MS, 100 * MS)],
        "the older seek still completes"
    );
    let failures = seek_failed_terms(&outputs);
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].1, SeekFailure::OutOfRange);
    assert_eq!(failures[0].0, Generation(2));
}

// ---------------------------------------------------------------------------
// The stale-purge proofs (doc 03 §5 checking point 5)
// ---------------------------------------------------------------------------

#[test]
fn stale_content_never_reappears_after_a_seek() {
    // The reorder decoder owes the pipeline one old-generation frame
    // at the seek boundary: the flush must drop it at the
    // generation-checking sink and count it — and no pre-landing
    // timestamp or old generation may ever reach the consumer again.
    let mut fixture = open(
        vec![track(1, MediaKind::Audio, Some(400))],
        (0..400)
            .map(|index| packet(TrackId::new(1), index))
            .collect(),
        true,
        ProviderKind::Reorder,
    );
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    let frames_before = frames_of(&fixture.collector);
    assert!(frames_before > 0);
    let old_generation = frames_snapshot(&fixture.collector)[0].1.generation();

    fixture
        .handle
        .seek(RequestId(90), SeekTarget::Absolute { position_ns: 200 * MS })
        .must();
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "re-prime");
    // The decoder-held old frame was purged and counted.
    let stats = fixture.engine.stats();
    assert!(
        stats.stale_frames_dropped >= 1,
        "the reorder buffer's old-generation frame is dropped and counted: {stats:?}"
    );
    // Nothing at or before the pre-seek frontier, and nothing of the
    // old generation, reappears after the seek.
    let frames = frames_snapshot(&fixture.collector);
    for (index, (_, frame)) in frames.iter().enumerate() {
        if index < frames_before {
            continue;
        }
        let pts_ns = stamp_ns(&frame.pts()).must();
        assert!(
            pts_ns >= 200 * MS,
            "a pre-landing timestamp reappeared post-seek: {pts_ns} ns"
        );
        assert!(
            frame.generation() > old_generation,
            "an old-generation frame reappeared post-seek"
        );
    }
    // And the seek still completes truthfully.
    let outputs = take_events(&mut fixture);
    assert_eq!(
        seeked_terms(&outputs),
        vec![(Generation(1), 200 * MS, 200 * MS)]
    );
}

// ---------------------------------------------------------------------------
// The drain exemption (doc 05 §4.3 invariant 4)
// ---------------------------------------------------------------------------

#[test]
fn a_seek_terminal_survives_a_later_generation_mint() {
    let mut fixture = open_audio(400, Some(400), true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    fixture
        .handle
        .seek(RequestId(95), SeekTarget::Absolute { position_ns: 200 * MS })
        .must();
    // The seek completes and the session re-primes — all without
    // draining (the terminal stays in the outbox).
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "re-prime");
    // A later mint lands before the host drains.
    fixture.handle.stop(RequestId(96)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Stopped);

    let outputs = take_events(&mut fixture);
    // The completed generation's terminal survives the suppression
    // that swallows its same-generation siblings.
    assert_eq!(
        seeked_terms(&outputs),
        vec![(Generation(1), 200 * MS, 200 * MS)]
    );
    assert!(
        fixture.engine.stats().suppressed_stale_events > 0,
        "the same-generation non-terminals were suppressed: {:?}",
        fixture.engine.stats()
    );
    // Receipts always survive.
    assert_eq!(receipt_for(&outputs, RequestId(95)).request, RequestId(95));
    assert_eq!(receipt_for(&outputs, RequestId(96)).request, RequestId(96));
}

// ---------------------------------------------------------------------------
// Command-boundary rejections
// ---------------------------------------------------------------------------

#[test]
fn a_seek_without_its_payload_is_rejected_typed() {
    let mut fixture = open_audio(400, Some(400), true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    // Raw submit with the Seek kind and no payload: the caller bug
    // surfaces typed at the command boundary.
    fixture
        .handle
        .submit(RequestId(97), SessionCommandKind::Seek)
        .must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Playing, "unchanged");
    let outputs = take_events(&mut fixture);
    let receipt = receipt_for(&outputs, RequestId(97));
    match receipt.result {
        ReceiptResult::Rejected { state, reason } => {
            assert_eq!(state, SessionState::Playing);
            assert_eq!(reason, RejectionReason::MissingPayload);
        }
        other => unreachable!("payload-less seek is rejected, got {other:?}"),
    }
    assert!(
        seeked_terms(&outputs).is_empty() && seek_failed_terms(&outputs).is_empty(),
        "no terminal for a rejected command"
    );
}

#[test]
fn a_seek_while_opening_is_rejected_typed() {
    let mut fixture = open_audio(400, Some(400), true);
    // Before the first poll the session is Opening: the pipeline is
    // not wired yet.
    assert_eq!(fixture.engine.state(), SessionState::Opening);
    fixture
        .handle
        .seek(RequestId(98), SeekTarget::Absolute { position_ns: 0 })
        .must();
    fixture.engine.poll();
    let outputs = take_events(&mut fixture);
    let receipt = receipt_for(&outputs, RequestId(98));
    match receipt.result {
        ReceiptResult::Rejected { state, reason } => {
            assert_eq!(state, SessionState::Opening);
            assert_eq!(reason, RejectionReason::NotReady);
        }
        other => unreachable!("seek while opening is rejected, got {other:?}"),
    }
    // The rejection changed nothing: the open proceeded.
    poll_until(&mut fixture, |s| s == SessionState::Playing, 10, "priming");
    assert!(
        seeked_terms(&take_events(&mut fixture)).is_empty(),
        "the rejected seek never ran"
    );
}

#[test]
fn a_stale_handle_cannot_seek_after_close() {
    let mut fixture = open_audio(40, Some(40), true);
    poll_until(&mut fixture, |s| s == SessionState::Ended, 100, "drain");
    fixture.handle.close(RequestId(99)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Idle);
    assert!(matches!(
        fixture
            .handle
            .seek(RequestId(100), SeekTarget::Absolute { position_ns: 0 }),
        Err(SessionError::SessionClosed)
    ));
}
