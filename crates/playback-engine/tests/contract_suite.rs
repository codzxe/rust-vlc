//! The doc 05 §7 contract suite, engine-side (card 3.B4): the
//! command-layer event contract proven against the real engine
//! through the **commands wire vocabulary** — every observable is a
//! [`WireEvent`]/[`EventKind`] (the surface a host binds to), never
//! an engine-internal type, except the receipts (the engine's half
//! of the one ordered channel).
//!
//! What is proven here (doc 05 §4.3 + §3.4 — the card's AC):
//!
//! - **ordering guarantees** (§4.3): `seq` engine-global strictly
//!   increasing; the receipt for request R precedes every event
//!   attributable to R (one ordered channel); the terminal
//!   invariants — `tracks.list-ready`, `duration-known` and
//!   `media.info-ready` before the first `state-changed → playing`,
//!   `duration-known` before any tick that assumes it, the fatal
//!   `playback.error` before `state-changed → error`,
//!   `end-of-media` before `state-changed → ended`;
//! - **C8 echoes**: idempotent self-transitions confirm on the wire
//!   (`from == to`, receipt first, origin echoed);
//! - **the §3.4 throttle rules on a virtual monotonic source**:
//!   `time-tick` fires on the configured media-position interval and
//!   carries the doc payload (µs time, clamped fraction); it is
//!   **never queued** — an undelivered older tick is replaced by a
//!   newer one (counted); the immediate ticks on pause entry and
//!   seek completion; `session.buffering` brackets with `0.0`/`1.0`,
//!   is monotonic in-episode and coalesced to its ≤ 5 Hz window;
//! - **staleness/supersession + the terminal invariant** (§4.2/§4.3
//!   invariant 4): a superseded seek is silent (exactly one
//!   terminal, for the newest generation); a seek terminal already
//!   emitted for a COMPLETED generation survives a later mint in
//!   the undrained outbox (the drain exemption) while the old
//!   generation's other pipeline-scoped events are suppressed and
//!   receipts always survive;
//! - **the non-fatal `playback.error`**: demux-stage bounded
//!   recovery coalesces over the 1 s window into one counted
//!   `fatal: false` wire event, flushed at the settling transition
//!   (the count is never lost).

use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
use commands::envelope::{Generation, RequestId};
use commands::{EventKind, PlaybackState, WireEvent};
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
use playback_engine::state::SessionState;
use playback_engine::{
    CommandReceipt, DecoderProvider, Engine, EngineOutput, EventPolicy, FrameConsumer, QueuePlan,
    ReceiptResult, SeekTarget, SessionConfig, SessionHandle, TickBudget, TimeTickPolicy,
};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};
use testkit::Must;

// ---------------------------------------------------------------------------
// Fakes
// ---------------------------------------------------------------------------

/// Shared virtual monotonic time (doc 03 §8: no sleeps). One source
/// drives BOTH the media clock and the event-policy throttle, so a
/// test advances "time" once and every window sees it.
struct SharedNow(AtomicU64);

impl MonotonicSource for SharedNow {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// Advance virtual time by `ms` milliseconds.
fn advance_ms(now: &SharedNow, ms: u64) {
    now.0.fetch_add(ms * 1_000_000, Ordering::Relaxed);
}

/// The collecting sink.
#[derive(Default)]
struct Collector {
    #[allow(dead_code)]
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

/// The scripted media: the packet list in demux order, the cursor,
/// and a producer stall (`Discontinuity` while armed — "nothing new
/// yet, stream not ended", the underrun shape).
#[derive(Default)]
struct Media {
    packets: Vec<Packet>,
    cursor: usize,
    stall: bool,
    fail_start: Option<EngineError>,
}

fn lock_media(media: &Arc<Mutex<Media>>) -> std::sync::MutexGuard<'_, Media> {
    media
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The seekable scripted demux stage: emits the media in order, then
/// a stall or end-of-stream. `reanchor` is compute-then-commit (D4):
/// the landing is computed from the packet list and the cursor moves
/// only on success.
struct ScriptDemuxer {
    tracks: Vec<Track>,
    media: Arc<Mutex<Media>>,
    seekable: bool,
}

impl Demuxer for ScriptDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        match lock_media(&self.media).fail_start.take() {
            Some(error) => Err(error),
            None => Ok(self.tracks.clone()),
        }
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        let mut media = lock_media(&self.media);
        if let Some(packet) = media.packets.get(media.cursor) {
            let packet = packet.clone();
            media.cursor += 1;
            return Ok(DemuxEvent::Packet(packet));
        }
        if media.stall {
            return Ok(DemuxEvent::Discontinuity);
        }
        Ok(DemuxEvent::EndOfStream)
    }

    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
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
        let target_ns = stamp_ns(&target.start)
            .ok_or_else(|| reanchor_refusal("demux.reanchor.target-missing"))?;
        // All packets are keyframes: the landing is the last packet
        // at or before the target.
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

/// The script demux stage for scenarios that need `Recovered` damage
/// marks interleaved with the packets: a fixed, ordered event script
/// (packets, marks, end-of-stream).
struct MarkDemuxer {
    tracks: Vec<Track>,
    script: Mutex<VecDeque<DemuxEvent>>,
}

impl Demuxer for MarkDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        Ok(self.tracks.clone())
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        Ok(self
            .script
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .pop_front()
            .unwrap_or(DemuxEvent::EndOfStream))
    }

    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
        Err(EngineError::new(
            PipelineStage::Demux,
            ErrorKind::Unsupported { format: None },
            media_types::error::ErrorContext::new("demux.seek.unneeded"),
        ))
    }

    fn seekability(&self) -> SeekSupport {
        SeekSupport::Fast { indexed: true }
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

fn stamp_ns(stamp: &Stamp) -> Option<i64> {
    let timestamp = stamp.timestamp().ok()?;
    match timestamp.as_nanos() {
        Ok(Nanoseconds(ns)) => Some(i64::try_from(ns).unwrap_or(i64::MAX)),
        Err(_) => None,
    }
}

/// The scripted decode stage: one frame out per packet.
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

/// The slow-flushing decode stage (the end-of-track shape): decodes
/// normally, but its end-of-stream flush needs more calls than one
/// tick's budget allows — a decoder with a deep reorder buffer. Its
/// limb reports `Drained` strictly LATER than an immediate limb, so
/// the two ends are observable as separate facts.
struct StallFlushDecoder {
    generation: GenerationId,
    flush_calls_left: u32,
}

impl Decoder for StallFlushDecoder {
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
        if self.flush_calls_left > 0 {
            self.flush_calls_left -= 1;
            // A reorder-buffer drain step: no frame this call (the
            // fake's frames were all delivered at decode time).
            return Ok(DecodeOutcome::Emitted { frames: 0 });
        }
        Ok(DecodeOutcome::Drained)
    }
}

/// The decoder seam: the video limb gets the slow-flushing decoder,
/// every other limb the immediate one (the end-of-track scenario's
/// asymmetric ends).
struct LimbProvider {
    stall_flush: bool,
}

impl DecoderProvider for LimbProvider {
    fn decoder_for(
        &self,
        track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        if self.stall_flush && track.kind == MediaKind::Video {
            Ok(Box::new(StallFlushDecoder {
                generation: config.generation,
                // One tick's flush budget is 8 calls: 9 forces the
                // drain onto the next tick.
                flush_calls_left: 9,
            }))
        } else {
            Ok(Box::new(FakeDecoder {
                generation: config.generation,
            }))
        }
    }
}

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

fn base() -> TimeBase {
    TimeBase::new(1, 1_000_000).must()
}

fn at_millis(ms: i64) -> Stamp {
    Stamp::Present(Timestamp::new(ms * 1_000, base()))
}

/// A track whose declared extent ends at `end_ms` milliseconds.
fn track(id: u32, kind: MediaKind, end_ms: i64) -> Track {
    Track {
        id: TrackId::new(id),
        stream: media_types::StreamId::new(id),
        kind,
        language: None,
        codec: media_types::ids::CodecDescriptor::new("fake-pcm").must(),
        title: None,
        flags: TrackFlags::NONE,
        timing: TrackTiming {
            span: TimeSpan {
                start: at_millis(0),
                end: at_millis(end_ms),
            },
            base: base(),
            gaps: Vec::new(),
        },
    }
}

fn packet(track: TrackId, pts_ms: i64) -> Packet {
    Packet {
        track,
        pts: at_millis(pts_ms),
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
/// at 4 (the session_flow/seek_flow numbers).
fn tiny_queues() -> QueuePlan {
    let limits = QueueLimits {
        max_packets: NonZeroUsize::new(16).must(),
        max_bytes: NonZeroUsize::new(1 << 20).must(),
        max_duration: Nanoseconds(1_000_000_000),
    };
    QueuePlan::new(limits, limits)
}

fn budget(demux_events: u32) -> TickBudget {
    TickBudget {
        demux_events,
        packets_per_limb: 4,
        flush_calls_per_limb: 8,
    }
}

/// Everything one contract scenario needs: the engine, the handle,
/// the shared virtual time, and the scripted media.
struct Fixture {
    engine: Engine,
    handle: SessionHandle,
    now: Arc<SharedNow>,
    media: Arc<Mutex<Media>>,
}

/// Open a session over the given media with the standard budget (12
/// demux events per tick), the default 250 ms tick policy, and the
/// immediate decoders.
fn open(tracks: Vec<Track>, packets: Vec<Packet>) -> Fixture {
    open_with(tracks, packets, budget(12), true, None, false)
}

/// Open with full control: the budget (the buffering-progress
/// scenarios want a smaller demux slice so the queue fills visibly),
/// seekability, an optional open failure, and the slow-flush video
/// decoder (the end-of-track scenario).
#[allow(clippy::too_many_arguments)]
fn open_with(
    tracks: Vec<Track>,
    packets: Vec<Packet>,
    tick_budget: TickBudget,
    seekable: bool,
    fail_start: Option<EngineError>,
    stall_flush: bool,
) -> Fixture {
    let media = Arc::new(Mutex::new(Media {
        packets,
        cursor: 0,
        stall: false,
        fail_start,
    }));
    let now = Arc::new(SharedNow(AtomicU64::new(0)));
    let collector = Arc::new(Collector::default());
    let clock = Arc::new(
        media_clock::MediaClock::system_monotonic(
            Arc::clone(&now) as Arc<dyn MonotonicSource>,
            Timestamp::new(0, base()),
            media_types::Rate::ONE,
        )
        .must(),
    );
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-contract-fake.media").must(),
        Box::new(ScriptDemuxer {
            tracks,
            media: Arc::clone(&media),
            seekable,
        }),
        Arc::new(LimbProvider { stall_flush }),
        Arc::clone(&collector) as Arc<dyn FrameConsumer>,
        clock,
    );
    config.queues = tiny_queues();
    config.budget = tick_budget;
    config.throttle = EventPolicy {
        tick: TimeTickPolicy::default_policy(),
        throttle: Arc::clone(&now) as Arc<dyn MonotonicSource>,
    };
    let mut engine = Engine::new();
    let handle = engine.open(config).must();
    Fixture {
        engine,
        handle,
        now,
        media,
    }
}

/// Open a session over a fixed event script (packets + `Recovered`
/// marks + end-of-stream) — the damage scenario's stage.
fn open_script(tracks: Vec<Track>, script: Vec<DemuxEvent>) -> Fixture {
    let now = Arc::new(SharedNow(AtomicU64::new(0)));
    let collector = Arc::new(Collector::default());
    let clock = Arc::new(
        media_clock::MediaClock::system_monotonic(
            Arc::clone(&now) as Arc<dyn MonotonicSource>,
            Timestamp::new(0, base()),
            media_types::Rate::ONE,
        )
        .must(),
    );
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-contract-damage.media").must(),
        Box::new(MarkDemuxer {
            tracks,
            script: Mutex::new(VecDeque::from(script)),
        }),
        Arc::new(LimbProvider { stall_flush: false }),
        Arc::clone(&collector) as Arc<dyn FrameConsumer>,
        clock,
    );
    config.queues = tiny_queues();
    config.budget = budget(12);
    config.throttle = EventPolicy {
        tick: TimeTickPolicy::default_policy(),
        throttle: Arc::clone(&now) as Arc<dyn MonotonicSource>,
    };
    let mut engine = Engine::new();
    let handle = engine.open(config).must();
    Fixture {
        engine,
        handle,
        now,
        media: Arc::new(Mutex::new(Media::default())),
    }
}

/// A single audio track of `count` packets at 1 ms strides, extent
/// `count` ms.
fn audio_media(count: i64) -> (Vec<Track>, Vec<Packet>) {
    (
        vec![track(1, MediaKind::Audio, count)],
        (0..count)
            .map(|index| packet(TrackId::new(1), index))
            .collect(),
    )
}

/// Drive polls until `predicate` holds on the engine state, or fail
/// loudly after `limit` polls (always polling at least once — a
/// command submitted right before the call needs its poll to apply).
fn poll_until(
    fixture: &mut Fixture,
    predicate: impl Fn(SessionState) -> bool,
    limit: usize,
    context: &str,
) {
    fixture.engine.poll();
    for _ in 1..limit {
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

// ---------------------------------------------------------------------------
// The wire-stream observation surface
// ---------------------------------------------------------------------------

/// One item of the single ordered output channel (doc 05 §4.3
/// guarantee 2): a receipt, or a wire event (engine-private kinds
/// never reach this surface — `as_wire_event` answers `None`).
enum WireItem {
    Receipt(CommandReceipt),
    Event(WireEvent),
}

/// Convert one output batch onto the wire surface, in order.
fn wire_stream(outputs: &[EngineOutput]) -> Vec<WireItem> {
    outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Receipt(receipt) => Some(WireItem::Receipt(*receipt)),
            EngineOutput::Event(event) => event.as_wire_event().map(WireItem::Event),
        })
        .collect()
}

/// Just the wire event kinds, in order.
fn kinds(stream: &[WireItem]) -> Vec<&EventKind> {
    stream
        .iter()
        .filter_map(|item| match item {
            WireItem::Event(event) => Some(&event.kind),
            _ => None,
        })
        .collect()
}

/// The receipt for one request.
fn receipt_for(stream: &[WireItem], request: RequestId) -> CommandReceipt {
    stream
        .iter()
        .find_map(|item| match item {
            WireItem::Receipt(receipt) if receipt.request == request => Some(*receipt),
            _ => None,
        })
        .must()
}

/// The index of the first event matching the predicate.
fn event_index(stream: &[WireItem], matches: impl Fn(&EventKind) -> bool) -> Option<usize> {
    stream.iter().position(|item| match item {
        WireItem::Event(event) => matches(&event.kind),
        WireItem::Receipt(_) => false,
    })
}

/// The event kind at a wire index, when the index is an event
/// (receipt positions answer `None` — the callers positioned the
/// index via [`event_index`], which only ever lands on events).
fn kind_at(stream: &[WireItem], index: usize) -> Option<&EventKind> {
    match &stream[index] {
        WireItem::Event(event) => Some(&event.kind),
        WireItem::Receipt(_) => None,
    }
}

/// Every `timing.time-tick` position (µs) in the stream.
fn tick_times(stream: &[WireItem]) -> Vec<i64> {
    kinds(stream)
        .iter()
        .filter_map(|kind| match kind {
            EventKind::TimingTimeTick { time, .. } => Some(time.0),
            _ => None,
        })
        .collect()
}

/// Every `session.buffering` progress value in the stream.
fn buffering_progress(stream: &[WireItem]) -> Vec<f32> {
    kinds(stream)
        .iter()
        .filter_map(|kind| match kind {
            EventKind::SessionBuffering { progress } => Some(*progress),
            _ => None,
        })
        .collect()
}

/// The `from → to` trail of every state-changed event.
fn state_trail(stream: &[WireItem]) -> Vec<(PlaybackState, PlaybackState)> {
    kinds(stream)
        .iter()
        .filter_map(|kind| match kind {
            EventKind::SessionStateChanged { from, to } => Some((*from, *to)),
            _ => None,
        })
        .collect()
}

/// The wire generation each seek terminal carries, with its kind.
fn seek_terminals(stream: &[WireItem]) -> Vec<(Generation, bool)> {
    stream
        .iter()
        .filter_map(|item| match item {
            WireItem::Event(event) => match &event.kind {
                EventKind::TimingSeekCompleted { .. } => Some((event.generation.must(), true)),
                EventKind::TimingSeekFailed { .. } => Some((event.generation.must(), false)),
                _ => None,
            },
            WireItem::Receipt(_) => None,
        })
        .collect()
}

/// Milliseconds, in nanoseconds.
const MS: i64 = 1_000_000;

// ---------------------------------------------------------------------------
// §4.3 ordering guarantees
// ---------------------------------------------------------------------------

/// The happy-path walk (open → play → pause → resume → end) proves
/// every doc 05 §4.3 ordering invariant on the wire surface at once:
/// strictly increasing engine-global `seq`, receipts causally before
/// the events they cause, the terminal invariants (list-ready,
/// duration-known and info-ready before the first `→ playing`;
/// duration-known before any tick that assumes it; end-of-media
/// before `→ ended`), the buffering bookends, and — because the
/// virtual clock advances only 200 ms (below the 250 ms interval) —
/// that the ONLY tick is the pause-entry immediate (doc 05 §3.2
/// lists no tick on play entry; the immediate cases are seek
/// completion, pause, rate change).
#[test]
fn the_wire_stream_honors_every_doc_05_ordering_invariant() {
    let (tracks, packets) = audio_media(600);
    let mut fixture = open(tracks, packets);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "priming");

    let mut stream = fixture.engine.take_outputs();
    // 200 ms of media time — below the 250 ms interval, so no
    // continuous tick fires before the pause.
    advance_ms(&fixture.now, 200);
    fixture.engine.poll();
    stream.append(&mut fixture.engine.take_outputs());

    fixture.handle.pause(RequestId(1)).must();
    fixture.engine.poll();
    stream.append(&mut fixture.engine.take_outputs());
    assert_eq!(fixture.engine.state(), SessionState::Paused);

    fixture.handle.play(RequestId(2)).must();
    fixture.engine.poll();
    stream.append(&mut fixture.engine.take_outputs());
    assert_eq!(fixture.engine.state(), SessionState::Playing);

    poll_until(&mut fixture, |s| s == SessionState::Ended, 400, "drain");
    stream.append(&mut fixture.engine.take_outputs());

    let wire = wire_stream(&stream);

    // Guarantee 1: seq is engine-global, strictly increasing.
    let mut last_seq = 0_u64;
    for item in &wire {
        if let WireItem::Event(event) = item {
            assert!(
                event.seq > last_seq,
                "seq {} did not increase past {}",
                event.seq,
                last_seq
            );
            last_seq = event.seq;
        }
    }

    // Guarantee 2: the receipt for request R precedes every event
    // attributable to R (one ordered channel, causal order).
    for (event_at, item) in wire.iter().enumerate() {
        if let WireItem::Event(event) = item
            && let Some(origin) = event.origin
        {
            let receipt_at = wire
                .iter()
                .position(|probe| {
                    matches!(probe, WireItem::Receipt(receipt) if receipt.request == origin)
                })
                .must();
            assert!(
                receipt_at < event_at,
                "the receipt for {origin:?} must precede its caused event"
            );
        }
    }

    // Invariant 4: tracks.list-ready, duration-known and
    // media.info-ready all precede the first `state-changed →
    // playing`.
    let first_playing = event_index(&wire, |kind| {
        matches!(
            kind,
            EventKind::SessionStateChanged {
                to: PlaybackState::Playing,
                ..
            }
        )
    })
    .must();
    let tracks_at = event_index(&wire, |kind| {
        matches!(kind, EventKind::TracksListReady { .. })
    })
    .must();
    let duration_at = event_index(&wire, |kind| {
        matches!(kind, EventKind::TimingDurationKnown { .. })
    })
    .must();
    let info_at = event_index(&wire, |kind| {
        matches!(kind, EventKind::MediaInfoReady { .. })
    })
    .must();
    assert!(tracks_at < first_playing);
    assert!(duration_at < first_playing);
    assert!(info_at < first_playing);

    // duration-known precedes every time-tick (the ticks assume it).
    for (index, item) in wire.iter().enumerate() {
        if let WireItem::Event(event) = item
            && matches!(event.kind, EventKind::TimingTimeTick { .. })
        {
            assert!(duration_at < index, "a tick preceded duration-known");
        }
    }

    // The duration payload: the declared extent, in µs.
    match kind_at(&wire, duration_at) {
        Some(EventKind::TimingDurationKnown { duration }) => {
            assert_eq!(duration.0, 600_000, "600 ms media → 600 000 µs");
        }
        _ => unreachable!("positioned duration-known"),
    }

    // The media.info-ready payload (the doc 05 §3.2 snapshot facts
    // the engine can honestly assemble at open).
    match kind_at(&wire, info_at) {
        Some(EventKind::MediaInfoReady { info }) => {
            assert!(matches!(
                info.source,
                commands::catalog::types::SourceSpec::File { .. }
            ));
            assert!(
                matches!(
                    info.duration,
                    commands::catalog::types::Duration::Known { time } if time.0 == 600_000
                ),
                "the snapshot carries the declared extent"
            );
            assert!(info.seekable, "the scripted source declares Fast seek");
            assert_eq!(info.tracks.len(), 1);
        }
        _ => unreachable!("positioned media.info-ready"),
    }

    // end-of-media precedes `state-changed → ended`.
    let eom_at = event_index(&wire, |kind| matches!(kind, EventKind::PlaybackEndOfMedia)).must();
    let ended_at = event_index(&wire, |kind| {
        matches!(
            kind,
            EventKind::SessionStateChanged {
                to: PlaybackState::Ended,
                ..
            }
        )
    })
    .must();
    assert!(eom_at < ended_at);

    // The buffering episode brackets with 0.0 and 1.0 (doc 05 §3.2),
    // with any in-episode samples strictly between and monotonic
    // (the ≤ 5 Hz progress stream this card adds — capped by the
    // frozen window to the first sample here).
    let progress = buffering_progress(&wire);
    assert_eq!(progress.first(), Some(&0.0), "the episode opens with 0.0");
    assert_eq!(progress.last(), Some(&1.0), "the episode closes with 1.0");
    let inside = &progress[1..progress.len() - 1];
    assert!(
        inside.windows(2).all(|pair| pair[0] < pair[1]),
        "monotonic inside the episode: {progress:?}"
    );

    // The state trail is the documented §3.3-legal walk.
    assert_eq!(
        state_trail(&wire),
        vec![
            (PlaybackState::Idle, PlaybackState::Opening),
            (PlaybackState::Opening, PlaybackState::Buffering),
            (PlaybackState::Buffering, PlaybackState::Playing),
            (PlaybackState::Playing, PlaybackState::Paused),
            (PlaybackState::Paused, PlaybackState::Playing),
            (PlaybackState::Playing, PlaybackState::Ended),
        ]
    );

    // The pause-entry immediate tick: exactly one tick in the whole
    // walk, at the frozen position (200 ms), BEFORE the
    // `state-changed → paused` it precedes.
    assert_eq!(
        tick_times(&wire),
        vec![200_000],
        "the pause-entry tick only"
    );
    let tick_at = event_index(&wire, |kind| {
        matches!(kind, EventKind::TimingTimeTick { .. })
    })
    .must();
    let paused_at = event_index(&wire, |kind| {
        matches!(
            kind,
            EventKind::SessionStateChanged {
                to: PlaybackState::Paused,
                ..
            }
        )
    })
    .must();
    assert!(
        tick_at < paused_at,
        "the tick is the last fact of the playing era"
    );
}

/// C8 (doc 05 §1.4): idempotent commands confirm with a
/// self-transition `state-changed` echo — receipt first, origin
/// echoed — and the C8 pause does NOT re-fire the immediate tick
/// (only the pause *entry* does).
#[test]
fn c8_idempotent_commands_confirm_with_self_transition_echoes() {
    let (tracks, packets) = audio_media(600);
    let mut fixture = open(tracks, packets);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "priming");
    let _ = fixture.engine.take_outputs();

    // Play while Playing: a self-transition confirmation.
    fixture.handle.play(RequestId(10)).must();
    fixture.engine.poll();
    let wire = wire_stream(&fixture.engine.take_outputs());
    assert_eq!(
        state_trail(&wire),
        vec![(PlaybackState::Playing, PlaybackState::Playing)],
        "the C8 echo"
    );
    let receipt = receipt_for(&wire, RequestId(10));
    assert_eq!(
        receipt.result,
        ReceiptResult::Accepted {
            session: fixture.handle.session_id(),
            generation: Generation(0),
        },
        "play is class A, non-bumping — the current gen"
    );
    let echo_at = event_index(&wire, |kind| {
        matches!(kind, EventKind::SessionStateChanged { .. })
    })
    .must();
    match &wire[echo_at] {
        WireItem::Event(event) => {
            assert_eq!(event.origin, Some(RequestId(10)));
            assert_eq!(event.generation, Some(Generation(0)));
        }
        WireItem::Receipt(_) => unreachable!("positioned an event"),
    }

    // Pause, then pause again: the second is a C8 echo, not an entry
    // — no second immediate tick.
    fixture.handle.pause(RequestId(11)).must();
    fixture.engine.poll();
    let _ = fixture.engine.take_outputs();
    advance_ms(&fixture.now, 100);
    fixture.handle.pause(RequestId(12)).must();
    fixture.engine.poll();
    let wire = wire_stream(&fixture.engine.take_outputs());
    assert_eq!(
        state_trail(&wire),
        vec![(PlaybackState::Paused, PlaybackState::Paused)],
        "the C8 pause echo"
    );
    assert!(
        tick_times(&wire).is_empty(),
        "a C8 pause confirmation is not a pause entry — no tick"
    );
}

/// A fatal failure's `playback.error` precedes its
/// `state-changed → error` (doc 05 §4.3 invariant 4, §3.3).
#[test]
fn a_fatal_error_precedes_its_state_changed_to_error() {
    let (tracks, packets) = audio_media(40);
    let failure = EngineError::new(
        PipelineStage::Open,
        ErrorKind::Inaccessible {
            reason: media_types::error::AccessDenied::NotFound,
        },
        media_types::error::ErrorContext::new("open.start.missing"),
    );
    let mut fixture = open_with(tracks, packets, budget(12), true, Some(failure), false);
    poll_until(&mut fixture, |s| s == SessionState::Error, 10, "fatal");
    let wire = wire_stream(&fixture.engine.take_outputs());
    let error_at = event_index(&wire, |kind| {
        matches!(kind, EventKind::PlaybackError { fatal: true, .. })
    })
    .must();
    let state_at = event_index(&wire, |kind| {
        matches!(
            kind,
            EventKind::SessionStateChanged {
                to: PlaybackState::Error,
                ..
            }
        )
    })
    .must();
    assert!(error_at < state_at);
    match kind_at(&wire, error_at) {
        Some(EventKind::PlaybackError { stage, code, .. }) => {
            assert_eq!(*stage, commands::Stage::Input);
            assert_eq!(*code, commands::ErrorCode::Inaccessible);
        }
        _ => unreachable!("positioned the fatal error"),
    }
}

// ---------------------------------------------------------------------------
// §3.4 rule 1 — the time-tick stream
// ---------------------------------------------------------------------------

/// The continuous stream fires on the configured interval, judged on
/// media position over the virtual monotonic source, and carries the
/// doc payload: µs time + the clamped fraction of the known
/// duration.
#[test]
fn time_ticks_fire_on_the_interval_and_carry_the_wire_payload() {
    let (tracks, packets) = audio_media(4_000);
    let mut fixture = open(tracks, packets);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "priming");
    let _ = fixture.engine.take_outputs();

    // 40 steps of 25 ms: positions 25..=1000 ms. The 250 ms default
    // policy admits a tick exactly at each 250 ms boundary — four
    // ticks over the second, none between.
    let mut stream = Vec::new();
    for _ in 0..40 {
        advance_ms(&fixture.now, 25);
        fixture.engine.poll();
        stream.append(&mut fixture.engine.take_outputs());
    }
    let wire = wire_stream(&stream);
    assert_eq!(
        tick_times(&wire),
        vec![250_000, 500_000, 750_000, 1_000_000],
        "one tick per 250 ms of media position"
    );
    for kind in kinds(&wire) {
        if let EventKind::TimingTimeTick { time, position } = kind {
            let expected = (time.0 as f64 / 4_000_000.0).clamp(0.0, 1.0) as f32;
            assert_eq!(*position, expected, "the fraction of {time:?}");
        }
    }
}

/// "Never queued" (doc 05 §3.4 rule 1): a host that does not drain
/// sees at most ONE time-tick — each newer tick replaces the
/// undelivered older one (counted) instead of queueing behind it.
#[test]
fn undelivered_time_ticks_are_never_queued() {
    let (tracks, packets) = audio_media(4_000);
    let mut fixture = open(tracks, packets);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "priming");
    let _ = fixture.engine.take_outputs();

    // Advance a full second WITHOUT draining: four ticks become due,
    // the hub keeps only the newest.
    for _ in 0..40 {
        advance_ms(&fixture.now, 25);
        fixture.engine.poll();
    }
    let wire = wire_stream(&fixture.engine.take_outputs());
    assert_eq!(
        tick_times(&wire),
        vec![1_000_000],
        "only the newest tick survives — at most one in flight"
    );
    assert_eq!(
        fixture.engine.stats().time_ticks_replaced,
        3,
        "250/500/750 ms replaced by the newer tick"
    );
}

/// The immediate tick after a seek terminal (doc 05 §3.2): the
/// landing truth, right where the clock re-anchored, immediately
/// after `timing.seek-completed`.
#[test]
fn a_seek_completion_emits_the_immediate_tick_at_the_landing() {
    let (tracks, packets) = audio_media(400);
    let mut fixture = open(tracks, packets);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "priming");
    let _ = fixture.engine.take_outputs();

    fixture
        .handle
        .seek(
            RequestId(20),
            SeekTarget::Absolute {
                position_ns: 200 * MS,
            },
        )
        .must();
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "re-prime");
    let wire = wire_stream(&fixture.engine.take_outputs());

    // The terminal is present, under the seek's own generation.
    assert_eq!(seek_terminals(&wire), vec![(Generation(1), true)]);
    let terminal_at = event_index(&wire, |kind| {
        matches!(kind, EventKind::TimingSeekCompleted { .. })
    })
    .must();
    // The immediate tick directly follows it, at the landing.
    let tick_at = event_index(&wire, |kind| {
        matches!(kind, EventKind::TimingTimeTick { .. })
    })
    .must();
    assert_eq!(
        tick_at,
        terminal_at + 1,
        "the tick is the terminal's next fact"
    );
    match kind_at(&wire, tick_at) {
        Some(EventKind::TimingTimeTick { time, .. }) => assert_eq!(time.0, 200_000),
        _ => unreachable!("positioned the tick"),
    }
}

// ---------------------------------------------------------------------------
// §4.2/§4.3 — staleness, supersession, terminal invariants
// ---------------------------------------------------------------------------

/// The drain exemption (doc 05 §4.3 invariant 4): a seek terminal
/// already emitted for a COMPLETED generation survives a later
/// generation mint sitting in the undriven outbox — while every
/// other pipeline-scoped event of the old generation is suppressed
/// (the engine-side staleness rule) and the receipts always survive.
#[test]
fn a_completed_seek_terminal_survives_a_later_generation_mint() {
    let (tracks, packets) = audio_media(400);
    let mut fixture = open(tracks, packets);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "priming");
    let _ = fixture.engine.take_outputs();

    // Seek to 200 ms and let it complete — WITHOUT draining.
    fixture
        .handle
        .seek(
            RequestId(30),
            SeekTarget::Absolute {
                position_ns: 200 * MS,
            },
        )
        .must();
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "re-prime");

    // A stop mints generation 2 before the host drains anything.
    fixture.handle.stop(RequestId(31)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Stopped);

    let wire = wire_stream(&fixture.engine.take_outputs());
    // The seek's receipt (gen 1) and the stop's (gen 2) both survive.
    assert_eq!(
        receipt_for(&wire, RequestId(30))
            .result
            .accepted_generation(),
        Some(Generation(1))
    );
    assert_eq!(
        receipt_for(&wire, RequestId(31))
            .result
            .accepted_generation(),
        Some(Generation(2))
    );
    // The terminal for the completed generation survives the newer
    // mint — exactly one terminal, stamped with its own generation.
    assert_eq!(seek_terminals(&wire), vec![(Generation(1), true)]);
    // The stop's state-changed carries the live generation 2.
    let stopped_at = event_index(&wire, |kind| {
        matches!(
            kind,
            EventKind::SessionStateChanged {
                to: PlaybackState::Stopped,
                ..
            }
        )
    })
    .must();
    match &wire[stopped_at] {
        WireItem::Event(event) => assert_eq!(event.generation, Some(Generation(2))),
        WireItem::Receipt(_) => unreachable!("positioned an event"),
    }
    // The old generation's other pipeline-scoped events (the seek's
    // immediate tick, the episode's in-episode sample and 1.0 exit,
    // the → playing) were suppressed at drain — the staleness rule,
    // counted — never delivered.
    assert!(
        fixture.engine.stats().suppressed_stale_events >= 3,
        "the gen-1 pipeline events were suppressed, not delivered"
    );
    assert!(tick_times(&wire).is_empty());
}

/// Supersession (doc 05 §4.2): seek A superseded mid-flight by seek
/// B is silent — no terminal for A's generation, exactly one for
/// B's — and the gen-A pipeline events sitting in the undrained
/// outbox (its 0.0 bookend, its `playing → buffering`) are
/// suppressed at drain while their receipts survive. Both seeks are
/// submitted before a single poll (the repeated-seek storm shape,
/// doc 03 §6): poll 1 applies A (mint gen 1, flush, episode) then B
/// (mint gen 2, flush, slot replacement) — A's generation is dead
/// before the tick could ever complete it, and every generation's
/// events sit in the outbox together until the one final drain.
#[test]
fn superseded_seeks_are_silent_and_stale_events_are_suppressed() {
    let (tracks, packets) = audio_media(400);
    let mut fixture = open(tracks, packets);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "priming");

    // Seek A, then seek B — both before any poll.
    fixture
        .handle
        .seek(
            RequestId(40),
            SeekTarget::Absolute {
                position_ns: 100 * MS,
            },
        )
        .must();
    fixture
        .handle
        .seek(
            RequestId(41),
            SeekTarget::Absolute {
                position_ns: 250 * MS,
            },
        )
        .must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Buffering);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 20, "re-prime");
    poll_until(&mut fixture, |s| s == SessionState::Ended, 400, "drain");

    let wire = wire_stream(&fixture.engine.take_outputs());
    // Both receipts survive (responses are never suppressed).
    assert_eq!(
        receipt_for(&wire, RequestId(40))
            .result
            .accepted_generation(),
        Some(Generation(1))
    );
    assert_eq!(
        receipt_for(&wire, RequestId(41))
            .result
            .accepted_generation(),
        Some(Generation(2))
    );
    // The superseded generation's buffered events (its 0.0 episode
    // bookend, its `playing → buffering`) were suppressed at drain —
    // counted, never delivered.
    assert!(
        fixture.engine.stats().suppressed_stale_events >= 2,
        "the superseded generation's buffered events were suppressed"
    );
    assert!(
        event_index(&wire, |kind| {
            matches!(kind, EventKind::SessionBuffering { progress: 0.0 })
        })
        .is_none(),
        "no 0.0 bookend was delivered — both older episodes were stale"
    );
    // Exactly one terminal in the whole stream: B's, for the newest
    // generation, at B's landing.
    assert_eq!(seek_terminals(&wire), vec![(Generation(2), true)]);
    let terminal_at = event_index(&wire, |kind| {
        matches!(kind, EventKind::TimingSeekCompleted { .. })
    })
    .must();
    match kind_at(&wire, terminal_at) {
        Some(EventKind::TimingSeekCompleted { position, .. }) => {
            assert_eq!(position.0, 250_000, "B's landing, in µs");
        }
        _ => unreachable!("positioned the terminal"),
    }
}

// ---------------------------------------------------------------------------
// §3.4 rule 2 — the buffering progress stream
// ---------------------------------------------------------------------------

/// The underrun → rebuffer cycle: the episode brackets with 0.0/1.0,
/// the in-episode progress is the monotonic queue fill, and the
/// ≤ 5 Hz window coalesces it — with the virtual clock frozen
/// between refills, exactly ONE in-episode sample escapes the
/// window.
#[test]
fn in_episode_buffering_progress_is_monotonic_and_window_capped() {
    // A slow producer slice (6 demux events vs 4 decoded per tick)
    // makes the refill visible: the queue climbs +2 per tick while
    // the episode is open. The preload (40 packets) is enough to
    // prime at that rate before the stall bites.
    let (tracks, packets) = audio_media(40);
    let mut fixture = open_with(tracks, packets, budget(6), true, None, false);
    lock_media(&fixture.media).stall = true;
    poll_until(&mut fixture, |s| s == SessionState::Playing, 60, "priming");
    let _ = fixture.engine.take_outputs();
    poll_until(
        &mut fixture,
        |s| s == SessionState::Buffering,
        200,
        "underrun",
    );

    // Release the stall with a long tail: each buffering tick adds 6
    // packets and decodes 4 — net +2 per tick.
    {
        let mut media = lock_media(&fixture.media);
        media.stall = false;
        let cursor = media.cursor as i64;
        for index in cursor..cursor + 120 {
            media.packets.push(packet(TrackId::new(1), index));
        }
    }

    // Refill WITHOUT advancing virtual time: the 200 ms window opens
    // on the first sample and swallows the rest of the climb.
    poll_until(&mut fixture, |s| s == SessionState::Playing, 60, "refill");
    let mut stream = fixture.engine.take_outputs();
    poll_until(&mut fixture, |s| s == SessionState::Ended, 400, "drain");
    stream.append(&mut fixture.engine.take_outputs());
    let wire = wire_stream(&stream);

    let progress = buffering_progress(&wire);
    // The bookends plus exactly one in-episode sample (the window
    // never elapsed — the ≤ 5 Hz cap coalesced the climb).
    assert_eq!(progress.len(), 3, "0.0 + one sample + 1.0: {progress:?}");
    assert_eq!(progress[0], 0.0, "the episode opens with 0.0");
    assert_eq!(progress[2], 1.0, "the episode closes with 1.0");
    let sample = progress[1];
    assert!(
        sample > 0.0 && sample < 1.0,
        "the in-episode sample is a real fill fraction: {sample}"
    );
}

/// The same refill with the window elapsing between ticks: the climb
/// is emitted step by step, strictly monotonic inside the episode
/// (doc 05 §3.4 rule 2).
#[test]
fn an_elapsed_window_emits_the_monotonic_climb() {
    let (tracks, packets) = audio_media(40);
    let mut fixture = open_with(tracks, packets, budget(6), true, None, false);
    lock_media(&fixture.media).stall = true;
    poll_until(&mut fixture, |s| s == SessionState::Playing, 60, "priming");
    let _ = fixture.engine.take_outputs();
    poll_until(
        &mut fixture,
        |s| s == SessionState::Buffering,
        200,
        "underrun",
    );
    {
        let mut media = lock_media(&fixture.media);
        media.stall = false;
        let cursor = media.cursor as i64;
        for index in cursor..cursor + 120 {
            media.packets.push(packet(TrackId::new(1), index));
        }
    }
    // Refill with the window elapsing (250 ms advances between the
    // polls that carry the climb).
    let mut stream = Vec::new();
    for _ in 0..10 {
        advance_ms(&fixture.now, 250);
        fixture.engine.poll();
        stream.append(&mut fixture.engine.take_outputs());
        if fixture.engine.state() == SessionState::Playing {
            break;
        }
    }
    assert_eq!(fixture.engine.state(), SessionState::Playing);
    poll_until(&mut fixture, |s| s == SessionState::Ended, 400, "drain");
    stream.append(&mut fixture.engine.take_outputs());
    let wire = wire_stream(&stream);

    let progress = buffering_progress(&wire);
    // Bookends + at least two strictly monotonic in-episode samples.
    assert_eq!(progress.first(), Some(&0.0));
    assert_eq!(progress.last(), Some(&1.0));
    let inside = &progress[1..progress.len() - 1];
    assert!(
        inside.len() >= 2,
        "the climb is observable when the window elapses: {progress:?}"
    );
    assert!(
        inside.windows(2).all(|pair| pair[0] < pair[1]),
        "strictly monotonic inside the episode: {progress:?}"
    );
}

// ---------------------------------------------------------------------------
// §3.2 — end-of-track and the non-fatal playback.error
// ---------------------------------------------------------------------------

/// One elementary stream ends while the media continues: the audio
/// limb's decoder flushes to `Drained` a tick before the video limb's
/// (the video decoder's reorder buffer outlives one flush budget),
/// so `playback.end-of-track` fires for the audio limb — and the
/// LAST limb's end is the media's end (`end-of-media` speaks; no
/// second end-of-track).
#[test]
fn end_of_track_fires_for_a_limb_that_ends_before_the_media() {
    let tracks = vec![
        track(1, MediaKind::Video, 60),
        track(2, MediaKind::Audio, 60),
    ];
    let mut packets = Vec::new();
    for step in 0..30 {
        packets.push(packet(TrackId::new(1), step * 2));
        packets.push(packet(TrackId::new(2), step * 2 + 1));
    }
    let mut fixture = open_with(tracks, packets, budget(12), true, None, true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 40, "priming");
    poll_until(&mut fixture, |s| s == SessionState::Ended, 400, "drain");
    let wire = wire_stream(&fixture.engine.take_outputs());

    // Exactly one end-of-track: the audio limb, with its kind and id.
    let ends: Vec<&EventKind> = kinds(&wire)
        .iter()
        .filter(|kind| matches!(kind, EventKind::PlaybackEndOfTrack { .. }))
        .copied()
        .collect();
    assert_eq!(ends.len(), 1, "the video limb's end is the media's end");
    match ends[0] {
        EventKind::PlaybackEndOfTrack { track_kind, id } => {
            assert_eq!(*track_kind, commands::catalog::types::TrackKind::Audio);
            assert_eq!(id.0, 2);
        }
        _ => unreachable!("filtered to end-of-track"),
    }
    // It precedes the media's end.
    let eot_at = event_index(&wire, |kind| {
        matches!(kind, EventKind::PlaybackEndOfTrack { .. })
    })
    .must();
    let eom_at = event_index(&wire, |kind| matches!(kind, EventKind::PlaybackEndOfMedia)).must();
    assert!(eot_at < eom_at);
}

/// Demux-stage bounded recovery coalesces into ONE non-fatal
/// `playback.error` per window with the count in its details (doc 05
/// §3.2) — and the terminal flush at the settling transition means
/// the count is never lost, even with the window frozen.
#[test]
fn non_fatal_damage_coalesces_into_one_counted_playback_error() {
    // 40 real packets with three Recovered marks (2 + 3 + 4 skipped
    // units) spread across the media. The virtual throttle is frozen
    // (the 1 s window never elapses mid-run), so everything must
    // flush at the terminal — one event, the full count.
    let mut script: Vec<DemuxEvent> = vec![DemuxEvent::Packet(packet(TrackId::new(1), 0))];
    script.push(DemuxEvent::Recovered {
        skipped: 2,
        reason: "resync",
    });
    for index in 1..12 {
        script.push(DemuxEvent::Packet(packet(TrackId::new(1), index)));
    }
    script.push(DemuxEvent::Recovered {
        skipped: 3,
        reason: "resync",
    });
    for index in 12..25 {
        script.push(DemuxEvent::Packet(packet(TrackId::new(1), index)));
    }
    script.push(DemuxEvent::Recovered {
        skipped: 4,
        reason: "resync",
    });
    for index in 25..40 {
        script.push(DemuxEvent::Packet(packet(TrackId::new(1), index)));
    }
    script.push(DemuxEvent::EndOfStream);

    let mut fixture = open_script(vec![track(1, MediaKind::Audio, 40)], script);
    poll_until(&mut fixture, |s| s == SessionState::Ended, 200, "drain");
    let wire = wire_stream(&fixture.engine.take_outputs());

    let errors: Vec<&EventKind> = kinds(&wire)
        .iter()
        .filter(|kind| matches!(kind, EventKind::PlaybackError { .. }))
        .copied()
        .collect();
    // Frozen window: ONE coalesced non-fatal error carries all nine
    // units, flushed at the settling transition.
    assert_eq!(errors.len(), 1, "coalesced: {errors:?}");
    match errors[0] {
        EventKind::PlaybackError {
            stage,
            code,
            details,
            fatal,
            ..
        } => {
            assert!(!*fatal, "bounded recovery is non-fatal");
            assert_eq!(*stage, commands::Stage::Demux);
            assert_eq!(*code, commands::ErrorCode::Corrupt);
            assert_eq!(
                details.get("count").map(String::as_str),
                Some("9"),
                "2 + 3 + 4 units, one count"
            );
        }
        _ => unreachable!("filtered to playback.error"),
    }
    // And the fatal half never fired: the media played to its end.
    assert!(
        event_index(&wire, |kind| {
            matches!(
                kind,
                EventKind::SessionStateChanged {
                    to: PlaybackState::Ended,
                    ..
                }
            )
        })
        .is_some(),
        "the damaged media still ends cleanly"
    );
}
