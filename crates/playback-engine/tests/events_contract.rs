//! The doc 05 §7 contract suite, events side (card 3.B4, W5-05
//! rebuild): scripted engine sessions on **virtual time** asserting
//! exact event sequences and payloads through the commands wire
//! vocabulary, with the policy layer of
//! `playback-engine/src/events.rs` as the checking machinery.
//!
//! What this suite adds over its sibling `contract_suite.rs` (the
//! 2.C2/3.B4 salvage, untouched and still merge-blocking):
//!
//! - **the completeness map, proven both ways**: the 23 catalog rows
//!   (D-017-frozen; `EventId::ALL` — the generated catalog — is the
//!   source of truth) are classified by
//!   [`CATALOG_EMISSIONS`](playback_engine::events::CATALOG_EMISSIONS),
//!   the `EventKind` ↔ `EventId` bridge is a bijection over all 23
//!   sample payloads, and every engine `EngineEventKind` variant
//!   either maps onto an **emitted** row or is one of the two
//!   documented engine-private kinds (which must never leak onto the
//!   wire);
//! - **exact sequences**: the happy-path session's wire event id
//!   sequence, pinned row by row, with per-row payload assertions
//!   (the §3.2 *Payload* column), including the pause-entry immediate
//!   tick and the buffering bookends;
//! - **the ordering guard** (`OrderingGuard`): every scripted
//!   session's full stream — receipts and events, in channel order —
//!   is observed by the guard, which must accept all of it and
//!   `finish()` clean; a second guard runs the same stream through
//!   the `debug_assert` form (the release-cheap self-check API);
//!   planted violations (the PositionUpdate-after-Stopped class,
//!   error-after-state-change, bookend-less episodes, …) are caught
//!   and named against the **real** stream shape;
//! - **the policy pieces**: the sessions are collected through
//!   [`BoundedEventQueue`](playback_engine::events::BoundedEventQueue)
//!   (the D-2 bounded-collector policy twin — and its zero-drop
//!   accounting proves no scenario storms past the bound).
//!
//! Rows proven here: `session.state-changed`, `session.buffering`,
//! `timing.time-tick`, `timing.duration-known`,
//! `timing.seek-completed`, `timing.seek-failed`,
//! `tracks.list-ready`, `tracks.changed`, `playback.end-of-track`,
//! `playback.end-of-media`, `playback.error` (both halves),
//! `media.info-ready` — the map's twelve emitted rows. The eleven
//! `WiredLater` rows own their triggers to later cards (the map's
//! notes) and have no engine behavior to test yet.

use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
use commands::catalog::types::{
    Duration as WireDuration, LogLevel, MediaInfo as WireMediaInfo, PlaylistChange, PlaylistItemId,
    PluginAction, SeekKind, SettingValue, SourceSpec, Timestamp as WireTimestamp, TrackChangeKind,
    TrackId as WireTrackId, TrackInfo as WireTrackInfo, TrackKind as WireTrackKind, TrackOrigin,
};
use commands::envelope::{Generation, RequestId};
use commands::{CommandError, ErrorCode, EventId, EventKind, PlaybackState, Stage, WireEvent};
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
use playback_engine::events::{
    BoundedEventQueue, CATALOG_EMISSIONS, ENGINE_EMITTED_ROWS, OrderingGuard,
    debug_assert_wire_ordered, emission_status, is_engine_emitted, wire_event_id,
};
use playback_engine::state::SessionState;
use playback_engine::{
    CommandReceipt, DecoderProvider, Engine, EngineEventKind, EngineOutput, EventPolicy,
    FrameConsumer, QueuePlan, ReceiptResult, SeekTarget, SessionConfig, SessionHandle, TickBudget,
    TimeTickPolicy, TrackSelector,
};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};
use testkit::Must;

/// Milliseconds, in nanoseconds.
const MS: i64 = 1_000_000;

// ---------------------------------------------------------------------------
// Fakes (the scripted media, on shared virtual time — doc 03 §8)
// ---------------------------------------------------------------------------

/// Shared virtual monotonic time: one source drives BOTH the media
/// clock and the event-policy throttle, so advancing "time" once
/// moves every window.
struct SharedNow(AtomicU64);

impl MonotonicSource for SharedNow {
    fn now_ns(&self) -> u64 {
        self.0.load(AtomicOrdering::Relaxed)
    }
}

fn advance_ms(now: &SharedNow, ms: u64) {
    now.0.fetch_add(ms * 1_000_000, AtomicOrdering::Relaxed);
}

/// The collecting sink: frames counted, never inspected (the wire
/// event stream is the subject here). The counter is atomic — the
/// sink rides an `Arc` through the session, so it must be `Sync`.
#[derive(Default)]
struct CounterSink {
    frames: std::sync::atomic::AtomicU64,
}

impl FrameConsumer for CounterSink {
    fn accept(&self, _frame: Frame, _track: TrackId) -> Result<(), EngineError> {
        self.frames
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

/// The scripted media: packets in demux order, the cursor, and an
/// optional open failure.
#[derive(Default)]
struct Media {
    packets: Vec<Packet>,
    cursor: usize,
    fail_start: Option<EngineError>,
}

fn lock_media(media: &Arc<Mutex<Media>>) -> std::sync::MutexGuard<'_, Media> {
    media
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The seekable scripted demux stage (the contract-suite shape).
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

/// The typed D4 refusal for an unresolvable re-anchor target (the
/// contract-suite shape).
fn reanchor_refusal(code: &'static str) -> EngineError {
    EngineError::new(
        PipelineStage::Demux,
        ErrorKind::Corrupt {
            detail: media_types::error::CorruptDetail::InvalidStructure { offset: None },
        },
        media_types::error::ErrorContext::new(code),
    )
}

/// The scripted demux stage for damage scenarios: a fixed event
/// script (packets + `Recovered` marks + end-of-stream).
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

/// The slow-flushing decode stage (the end-of-track shape): its
/// end-of-stream flush needs more calls than one tick's budget — its
/// limb reports `Drained` strictly later than an immediate limb's.
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
            return Ok(DecodeOutcome::Emitted { frames: 0 });
        }
        Ok(DecodeOutcome::Drained)
    }
}

/// The decoder seam: the video limb gets the slow-flushing decoder
/// when asked (the end-of-track scenario's asymmetric ends).
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

fn packet(track_id: TrackId, pts_ms: i64) -> Packet {
    Packet {
        track: track_id,
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

// ---------------------------------------------------------------------------
// The recording harness (the guard + the bounded collector)
// ---------------------------------------------------------------------------

/// One item of the single ordered output channel (doc 05 §4.3
/// guarantee 2): a receipt, or a wire event.
enum WireItem {
    Receipt(CommandReceipt),
    Event(WireEvent),
}

/// The scripted session plus its recording machinery: everything the
/// engine emits is converted to the wire surface, collected through
/// the D-2 bounded queue, and observed by both guard forms.
struct Session {
    engine: Engine,
    handle: SessionHandle,
    now: Arc<SharedNow>,
    /// The ordered channel, as recorded.
    items: Vec<WireItem>,
    /// The strict guard: every observation must be `Ok`.
    guard: OrderingGuard,
    /// The debug_assert form over the same stream (a second guard —
    /// observing twice on one guard would double-count the state).
    debug_guard: OrderingGuard,
    /// The bounded collector (the D-2 policy twin): every wire event
    /// rides it, so the policy is exercised by every scenario.
    collector: BoundedEventQueue<WireEvent>,
}

impl Session {
    /// Record one output batch: receipts position causally, events
    /// convert to the wire surface, both guard forms observe, the
    /// bounded queue collects.
    fn record(&mut self, outputs: &[EngineOutput]) {
        for output in outputs {
            match output {
                EngineOutput::Receipt(receipt) => {
                    self.guard.observe_receipt(receipt.request);
                    self.debug_guard.observe_receipt(receipt.request);
                    self.items.push(WireItem::Receipt(*receipt));
                }
                EngineOutput::Event(event) => {
                    if let Some(wire_event) = event.as_wire_event() {
                        self.guard.observe(&wire_event).must();
                        // The debug form: same stream, second guard —
                        // in a test profile this is the live proof the
                        // API compiles and passes on real sessions.
                        debug_assert_wire_ordered(&mut self.debug_guard, &wire_event);
                        self.collector.push(wire_event.clone());
                        self.items.push(WireItem::Event(wire_event));
                    }
                }
            }
        }
    }

    /// Poll once and record.
    fn poll(&mut self) {
        self.engine.poll();
        let outputs = self.engine.take_outputs();
        self.record(&outputs);
    }

    /// Poll until `predicate` holds on the engine state.
    fn poll_until(&mut self, predicate: impl Fn(SessionState) -> bool, limit: usize, what: &str) {
        self.poll();
        for _ in 1..limit {
            if predicate(self.engine.state()) {
                return;
            }
            self.poll();
        }
        assert!(
            predicate(self.engine.state()),
            "{what}: state is {} after {limit} polls",
            self.engine.state()
        );
    }

    /// The wire event kinds, in order.
    fn kinds(&self) -> Vec<&EventKind> {
        self.items
            .iter()
            .filter_map(|item| match item {
                WireItem::Event(event) => Some(&event.kind),
                WireItem::Receipt(_) => None,
            })
            .collect()
    }

    /// The wire event ids, in order — the exact-sequence view.
    fn ids(&self) -> Vec<&'static str> {
        self.kinds()
            .iter()
            .map(|kind| wire_event_id(kind).as_str())
            .collect()
    }

    /// The state trail of every `state-changed`.
    fn state_trail(&self) -> Vec<(PlaybackState, PlaybackState)> {
        self.kinds()
            .iter()
            .filter_map(|kind| match kind {
                EventKind::SessionStateChanged { from, to } => Some((*from, *to)),
                _ => None,
            })
            .collect()
    }

    /// The receipt for one request.
    fn receipt_for(&self, request: RequestId) -> CommandReceipt {
        self.items
            .iter()
            .find_map(|item| match item {
                WireItem::Receipt(receipt) if receipt.request == request => Some(*receipt),
                _ => None,
            })
            .must()
    }

    /// The index (in items) of the first event matching.
    fn event_index(&self, matches: impl Fn(&EventKind) -> bool) -> Option<usize> {
        self.items.iter().position(|item| match item {
            WireItem::Event(event) => matches(&event.kind),
            WireItem::Receipt(_) => false,
        })
    }

    /// The end-of-recording verdicts: the strict guard's dangling
    /// invariants, and the bounded collector's no-drop accounting
    /// (no scenario may storm past the collector's bound).
    fn finish_clean(&self) {
        self.guard.finish().must();
        assert_eq!(
            self.collector.dropped_total(),
            0,
            "no scenario may storm past the collector bound"
        );
        assert!(self.collector.len() <= self.collector.capacity());
    }
}

/// Open a scripted session with full control (the contract-suite
/// builder shape, plus the recording machinery).
fn open_session(
    tracks: Vec<Track>,
    packets: Vec<Packet>,
    tick_budget: TickBudget,
    seekable: bool,
    fail_start: Option<EngineError>,
    stall_flush: bool,
) -> Session {
    let media = Arc::new(Mutex::new(Media {
        packets,
        cursor: 0,
        fail_start,
    }));
    let now = Arc::new(SharedNow(AtomicU64::new(0)));
    let sink = Arc::new(CounterSink::default());
    let clock = Arc::new(
        media_clock::MediaClock::system_monotonic(
            Arc::clone(&now) as Arc<dyn MonotonicSource>,
            Timestamp::new(0, base()),
            media_types::Rate::ONE,
        )
        .must(),
    );
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-events-contract.media").must(),
        Box::new(ScriptDemuxer {
            tracks,
            media: Arc::clone(&media),
            seekable,
        }),
        Arc::new(LimbProvider { stall_flush }),
        Arc::clone(&sink) as Arc<dyn FrameConsumer>,
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
    Session {
        engine,
        handle,
        now,
        items: Vec::new(),
        guard: OrderingGuard::new(),
        debug_guard: OrderingGuard::new(),
        collector: BoundedEventQueue::new(NonZeroUsize::new(4096).must()),
    }
}

/// The standard session: default budget, seekable, healthy.
fn open_standard(tracks: Vec<Track>, packets: Vec<Packet>) -> Session {
    open_session(tracks, packets, budget(12), true, None, false)
}

/// Open a session over a fixed demux event script (the damage
/// scenario's stage).
fn open_script(tracks: Vec<Track>, script: Vec<DemuxEvent>) -> Session {
    let now = Arc::new(SharedNow(AtomicU64::new(0)));
    let sink = Arc::new(CounterSink::default());
    let clock = Arc::new(
        media_clock::MediaClock::system_monotonic(
            Arc::clone(&now) as Arc<dyn MonotonicSource>,
            Timestamp::new(0, base()),
            media_types::Rate::ONE,
        )
        .must(),
    );
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-events-damage.media").must(),
        Box::new(MarkDemuxer {
            tracks,
            script: Mutex::new(VecDeque::from(script)),
        }),
        Arc::new(LimbProvider { stall_flush: false }),
        Arc::clone(&sink) as Arc<dyn FrameConsumer>,
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
    Session {
        engine,
        handle,
        now,
        items: Vec::new(),
        guard: OrderingGuard::new(),
        debug_guard: OrderingGuard::new(),
        collector: BoundedEventQueue::new(NonZeroUsize::new(4096).must()),
    }
}

// ---------------------------------------------------------------------------
// §A — the completeness map (doc 05 §3.2, D-017-frozen)
// ---------------------------------------------------------------------------

/// One minimal wire payload per catalog row — the bijection material
/// for the id bridge (and the proof that every row is representable
/// on the wire exactly as doc 05 §3.2 sketches it).
fn every_catalog_row_sample() -> Vec<EventKind> {
    let media_info = WireMediaInfo {
        source: SourceSpec::File {
            path: PathBuf::from("fixture://sine.mkv"),
        },
        container: None,
        title: None,
        duration: WireDuration::Unknown,
        seekable: true,
        is_live: false,
        metadata: BTreeMap::new(),
        tracks: Vec::new(),
        chapters: Vec::new(),
    };
    vec![
        EventKind::SessionStateChanged {
            from: PlaybackState::Idle,
            to: PlaybackState::Opening,
        },
        EventKind::SessionOpeningProgress { progress: 0.5 },
        EventKind::SessionBuffering { progress: 0.5 },
        EventKind::TimingTimeTick {
            time: WireTimestamp(250_000),
            position: 0.25,
        },
        EventKind::TimingDurationKnown {
            duration: WireTimestamp(600_000),
        },
        EventKind::TimingSeekCompleted {
            position: WireTimestamp(200_000),
            seek_kind: SeekKind::Command,
        },
        EventKind::TimingSeekFailed {
            error: CommandError::new(Stage::Control, ErrorCode::Unsupported, "nope"),
        },
        EventKind::TracksListReady {
            tracks: vec![WireTrackInfo {
                id: WireTrackId(1),
                kind: WireTrackKind::Audio,
                name: None,
                language: None,
                codec: commands::catalog::types::CodecDescriptor {
                    fourcc: None,
                    description: Some("fake-pcm".to_owned()),
                    profile: None,
                    level: None,
                },
                is_default: false,
                is_selected: true,
                origin: TrackOrigin::Stream,
                video: None,
                audio: None,
            }],
        },
        EventKind::TracksChanged {
            track_kind: TrackChangeKind::Audio,
            selected: Some(WireTrackId(2)),
        },
        EventKind::PlaybackEndOfTrack {
            track_kind: WireTrackKind::Audio,
            id: WireTrackId(2),
        },
        EventKind::PlaybackEndOfMedia,
        EventKind::PlaybackError {
            stage: Stage::Decode,
            code: ErrorCode::Decoder,
            message: "dropped".to_owned(),
            details: BTreeMap::new(),
            fatal: false,
        },
        EventKind::PlaybackRateChanged { rate: 1.25 },
        EventKind::AudioDeviceLost {
            device_id: "default".to_owned(),
        },
        EventKind::AudioDeviceChanged {
            device_id: "hdmi:0".to_owned(),
        },
        EventKind::VideoScreenshotSaved {
            path: PathBuf::from("/tmp/shot.png"),
            width: 1920,
            height: 1080,
        },
        EventKind::VideoBackendChanged {
            backend: "software".to_owned(),
            reason: "hardware backend unavailable".to_owned(),
        },
        EventKind::PlaylistChanged {
            change: PlaylistChange::ItemsAdded {
                ids: vec![PlaylistItemId(1)],
            },
        },
        EventKind::ConfigSettingChanged {
            area: "playback".to_owned(),
            key: "time-tick-interval-ms".to_owned(),
            value: SettingValue::Number(250.0),
            layer: "user".to_owned(),
        },
        EventKind::PluginsLoaded {
            id: "org.example.amp".to_owned(),
            plugin_kind: "visualizer".to_owned(),
            version: "1.0.0".to_owned(),
        },
        EventKind::PluginsFailed {
            id: "org.example.bad".to_owned(),
            error: "manifest parse".to_owned(),
        },
        EventKind::PluginsCrashed {
            id: "org.example.trap".to_owned(),
            error: "unreachable".to_owned(),
            action_taken: PluginAction::Disabled,
        },
        EventKind::SystemLog {
            level: LogLevel::Info,
            target: "lumen::engine".to_owned(),
            message: "line".to_owned(),
        },
        EventKind::MediaInfoReady { info: media_info },
        EventKind::DevicesVolumeAdded {
            id: "usb-1234".to_owned(),
            label: "USB Drive".to_owned(),
            volume_kind: commands::catalog::types::VolumeKind::Removable,
        },
        EventKind::DevicesVolumeRemoved {
            id: "usb-1234".to_owned(),
        },
    ]
}

/// The completeness map and the id bridge are bijections over the
/// generated catalog (the source of truth): 26 rows (D-017's 23 +
/// the issue #42 additive trio), every id hit exactly once by the
/// bridge, and the emitted/later split is the map's own arithmetic
/// (12 + 14).
#[test]
fn the_completeness_map_and_bridge_are_catalog_bijections() {
    let samples = every_catalog_row_sample();
    assert_eq!(samples.len(), EventId::ALL.len());
    assert_eq!(samples.len(), 26, "D-017 23 + the issue #42 trio");
    assert_eq!(CATALOG_EMISSIONS.len(), 26);
    // The bridge hits every id exactly once, in doc table order.
    let bridged: Vec<EventId> = samples.iter().map(wire_event_id).collect();
    for (sample_id, catalog_id) in bridged.iter().zip(EventId::ALL) {
        assert_eq!(*sample_id, *catalog_id);
    }
    let mut sorted = bridged.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 26, "no id hit twice");
    // The map agrees with the bridge row by row, and the split is
    // 12 emitted + 14 later.
    let emitted = CATALOG_EMISSIONS
        .iter()
        .filter(|row| row.status.is_emitted())
        .count();
    assert_eq!(emitted, ENGINE_EMITTED_ROWS);
    assert_eq!(emitted, 12);
    assert_eq!(CATALOG_EMISSIONS.len() - emitted, 14);
    for (row, id) in CATALOG_EMISSIONS.iter().zip(EventId::ALL) {
        assert_eq!(row.id, *id);
        assert_eq!(emission_status(*id), Some(row.status));
    }
}

/// Every engine event kind either maps onto an **emitted** catalog
/// row or is one of the two documented engine-private kinds — which
/// must answer `None` on the wire conversion (they never leak).
#[test]
fn every_engine_kind_lands_on_an_emitted_row_or_stays_private() {
    let position = Stamp::Present(Timestamp::new(0, base()));
    let engine_error = || {
        EngineError::new(
            PipelineStage::Demux,
            ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::InvalidStructure { offset: None },
            },
            media_types::error::ErrorContext::new("test.fatal"),
        )
    };
    let engine_media_info = || {
        Box::new(WireMediaInfo {
            source: SourceSpec::Url {
                url: "fixture://x".to_owned(),
            },
            container: None,
            title: None,
            duration: WireDuration::Unknown,
            seekable: false,
            is_live: false,
            metadata: BTreeMap::new(),
            tracks: Vec::new(),
            chapters: Vec::new(),
        })
    };
    let kinds = vec![
        EngineEventKind::StateChanged {
            from: SessionState::Idle,
            to: SessionState::Opening,
        },
        EngineEventKind::TracksChanged {
            tracks: Arc::new(Vec::new()),
            selected: Vec::new(),
        },
        EngineEventKind::Buffering { progress: 0.0 },
        EngineEventKind::EndOfMedia,
        EngineEventKind::Fatal {
            error: engine_error(),
        },
        EngineEventKind::DamagedData { count: 1 },
        EngineEventKind::FirstFrame { position },
        EngineEventKind::Seeked {
            requested: 0,
            actual: 0,
        },
        EngineEventKind::SeekFailed {
            reason: playback_engine::SeekFailure::NotSeekable,
            position: 0,
        },
        EngineEventKind::TimeTick {
            position_ns: 0,
            fraction: 0.0,
        },
        EngineEventKind::DurationKnown { duration_ns: 1 },
        EngineEventKind::MediaInfoReady {
            info: engine_media_info(),
        },
        EngineEventKind::EndOfTrack {
            track_kind: MediaKind::Audio,
            id: TrackId::new(2),
        },
        EngineEventKind::RecoveredDamage {
            stage: PipelineStage::Demux,
            count: 3,
        },
        EngineEventKind::TrackSelectionChanged {
            track_kind: MediaKind::Audio,
            selected: Some(TrackId::new(2)),
        },
    ];
    assert_eq!(kinds.len(), 15, "one sample per engine kind");
    for kind in kinds {
        match kind.as_wire() {
            Some(wire_kind) => {
                let id = wire_event_id(&wire_kind);
                assert!(
                    is_engine_emitted(id),
                    "{id} is engine-produced but not in the emitted bucket"
                );
            }
            // The engine-private pair: bounded-recovery observability
            // and the first-frame trigger (doc 05 §4.2 — no v1 wire
            // rows; they must never leak).
            None => {
                assert!(
                    matches!(
                        kind,
                        EngineEventKind::DamagedData { .. } | EngineEventKind::FirstFrame { .. }
                    ),
                    "an unmapped engine kind that is not the documented private pair: {kind:?}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// §B — exact sequences and payloads on virtual time (doc 05 §7)
// ---------------------------------------------------------------------------

/// The happy-path walk (open → prime → play → pause → resume → end)
/// pins the **exact** wire event sequence, every payload from the
/// §3.2 *Payload* column, and both guard forms accept the whole
/// stream (receipts included — the causal-order rule §4.3.2).
#[test]
fn happy_path_session_emits_the_exact_wire_sequence_and_payloads() {
    let (tracks, packets) = audio_media(600);
    let mut session = open_standard(tracks, packets);
    session.poll_until(|s| s == SessionState::Playing, 20, "priming");

    // 200 ms of media time — below the 250 ms tick interval, so the
    // only tick in the walk is the pause-entry immediate.
    advance_ms(&session.now, 200);
    session.poll();

    session.handle.pause(RequestId(1)).must();
    session.poll();
    assert_eq!(session.engine.state(), SessionState::Paused);

    session.handle.play(RequestId(2)).must();
    session.poll();
    assert_eq!(session.engine.state(), SessionState::Playing);

    session.poll_until(|s| s == SessionState::Ended, 400, "drain");
    session.finish_clean();

    // The exact sequence, as catalog ids.
    assert_eq!(
        session.ids(),
        vec![
            "session.state-changed",
            "tracks.list-ready",
            "session.buffering",
            "session.state-changed",
            "timing.duration-known",
            "media.info-ready",
            "session.buffering",
            "session.buffering",
            "session.state-changed",
            "timing.time-tick",
            "session.state-changed",
            "session.state-changed",
            "playback.end-of-media",
            "session.state-changed",
        ],
        "the doc 05 §3.2 open-to-end walk, exactly"
    );

    // Payload row by row (§3.2's Payload column).
    // `session.state-changed` — the §3.3-legal trail.
    assert_eq!(
        session.state_trail(),
        vec![
            (PlaybackState::Idle, PlaybackState::Opening),
            (PlaybackState::Opening, PlaybackState::Buffering),
            (PlaybackState::Buffering, PlaybackState::Playing),
            (PlaybackState::Playing, PlaybackState::Paused),
            (PlaybackState::Paused, PlaybackState::Playing),
            (PlaybackState::Playing, PlaybackState::Ended),
        ]
    );
    // `session.buffering` — opens 0.0, one in-episode climb
    // (0.75: the single sample the ≤ 5 Hz window permits under the
    // frozen virtual clock — monotonic, §3.4.2), closes 1.0.
    let progress: Vec<f32> = session
        .kinds()
        .iter()
        .filter_map(|kind| match kind {
            EventKind::SessionBuffering { progress } => Some(*progress),
            _ => None,
        })
        .collect();
    assert_eq!(progress, vec![0.0, 0.75, 1.0]);
    // `tracks.list-ready` — the wired list with selection truth.
    match &session.kinds()[1] {
        EventKind::TracksListReady { tracks } => {
            assert_eq!(tracks.len(), 1);
            assert_eq!(tracks[0].id, WireTrackId(1));
            assert_eq!(tracks[0].kind, WireTrackKind::Audio);
            assert!(tracks[0].is_selected, "the engine wired the limb");
        }
        _ => unreachable!("positioned the list-ready row"),
    }
    // `timing.duration-known` — the declared extent, in µs.
    match &session.kinds()[4] {
        EventKind::TimingDurationKnown { duration } => assert_eq!(duration.0, 600_000),
        _ => unreachable!("positioned duration-known"),
    }
    // `media.info-ready` — the open-time snapshot facts.
    match &session.kinds()[5] {
        EventKind::MediaInfoReady { info } => {
            assert!(matches!(info.source, SourceSpec::File { .. }));
            assert!(matches!(info.duration, WireDuration::Known { time } if time.0 == 600_000));
            assert!(info.seekable);
            assert_eq!(info.tracks.len(), 1);
        }
        _ => unreachable!("positioned info-ready"),
    }
    // `timing.time-tick` — µs time + the clamped fraction; the last
    // fact of the playing era, before the pause transition.
    match &session.kinds()[9] {
        EventKind::TimingTimeTick { time, position } => {
            assert_eq!(time.0, 200_000);
            let expected = (200_000_000.0_f64 / 600_000_000.0_f64).clamp(0.0, 1.0) as f32;
            assert_eq!(*position, expected);
        }
        _ => unreachable!("positioned the tick"),
    }
    // `playback.end-of-media` — present, before `state-changed →
    // ended` (the guard proves the order; pin the row here too).
    assert!(matches!(session.kinds()[12], EventKind::PlaybackEndOfMedia));

    // §4.3.2 — the receipts precede their caused events (the guard
    // observed it; one explicit spot-check for the report).
    for (at, item) in session.items.iter().enumerate() {
        if let WireItem::Event(event) = item
            && let Some(origin) = event.origin
        {
            let receipt_at = session
                .items
                .iter()
                .position(|probe| {
                    matches!(probe, WireItem::Receipt(receipt) if receipt.request == origin)
                })
                .must();
            assert!(receipt_at < at, "the receipt for {origin:?} came late");
        }
    }
}

/// The stop scenario: after `state-changed → stopped` the wire is
/// silent — no PositionUpdate-after-Stopped (the audit's named
/// violation class), and the guard catches exactly that when one is
/// planted onto the real stream's shape.
#[test]
fn stop_ends_the_stream_and_a_planted_tick_after_stop_is_caught() {
    let (tracks, packets) = audio_media(4_000);
    let mut session = open_standard(tracks, packets);
    session.poll_until(|s| s == SessionState::Playing, 20, "priming");
    advance_ms(&session.now, 300);
    session.poll();
    let ticks_before_stop = session
        .kinds()
        .iter()
        .filter(|kind| matches!(kind, EventKind::TimingTimeTick { .. }))
        .count();

    session.handle.stop(RequestId(9)).must();
    session.poll();
    assert_eq!(session.engine.state(), SessionState::Stopped);
    // Plenty of media and time left — yet the wire stays silent.
    for _ in 0..50 {
        advance_ms(&session.now, 250);
        session.poll();
    }
    session.finish_clean();

    let stopped_at = session
        .event_index(|kind| {
            matches!(
                kind,
                EventKind::SessionStateChanged {
                    to: PlaybackState::Stopped,
                    ..
                }
            )
        })
        .must();
    let tail: Vec<&EventKind> = session
        .items
        .iter()
        .enumerate()
        .filter_map(|(at, item)| match item {
            WireItem::Event(event) if at > stopped_at => Some(&event.kind),
            _ => None,
        })
        .collect();
    assert!(
        tail.iter()
            .all(|kind| !matches!(kind, EventKind::TimingTimeTick { .. })),
        "no tick after stopped: {tail:?}"
    );
    assert!(
        tail.iter()
            .all(|kind| !matches!(kind, EventKind::SessionBuffering { .. })),
        "no buffering after stopped: {tail:?}"
    );
    assert!(ticks_before_stop >= 1, "the pre-stop tick fired");

    // The planted violation: the real stopped stream plus one
    // synthetic tick — a fresh guard replays the recording and must
    // name the violation class.
    let mut guard = OrderingGuard::new();
    for item in &session.items {
        match item {
            WireItem::Receipt(receipt) => guard.observe_receipt(receipt.request),
            WireItem::Event(event) => guard.observe(event).must(),
        }
    }
    let planted = WireEvent {
        seq: u64::MAX,
        session: None,
        generation: None,
        origin: None,
        kind: EventKind::TimingTimeTick {
            time: WireTimestamp(5_000_000),
            position: 0.0,
        },
    };
    let verdict = guard.observe(&planted);
    assert!(
        matches!(
            verdict.err(),
            Some(
                playback_engine::events::OrderingViolation::TickOutsidePlaying {
                    at: PlaybackState::Stopped
                }
            )
        ),
        "the planted PositionUpdate-after-Stopped must be caught: {verdict:?}"
    );
}

/// A seek session: the accept receipt carries the bumped generation,
/// the terminal `timing.seek-completed` is stamped with that same
/// generation and the doc payload `{position, seekKind: "command"}`,
/// and the immediate `time-tick` at the landing directly follows it
/// (§3.2) — the one legal tick outside `playing`, which the guard
/// accepts.
#[test]
fn seek_completed_carries_the_gen_and_payload_then_the_immediate_tick() {
    let (tracks, packets) = audio_media(4_000);
    let mut session = open_standard(tracks, packets);
    session.poll_until(|s| s == SessionState::Playing, 20, "priming");
    // Burn the recorded open phase; the seek phase is the subject.
    session.items.clear();

    session
        .handle
        .seek(
            RequestId(20),
            SeekTarget::Absolute {
                position_ns: 200 * MS,
            },
        )
        .must();
    session.poll_until(|s| s == SessionState::Playing, 40, "re-prime");
    session.finish_clean();

    let receipt = session.receipt_for(RequestId(20));
    assert_eq!(
        receipt.result,
        ReceiptResult::Accepted {
            session: session.handle.session_id(),
            generation: Generation(1),
        },
        "the class-A accept carries the post-bump gen"
    );

    let terminals: Vec<&EventKind> = session
        .kinds()
        .iter()
        .filter(|kind| {
            matches!(kind, EventKind::TimingSeekCompleted { .. })
                || matches!(kind, EventKind::TimingSeekFailed { .. })
        })
        .copied()
        .collect();
    assert_eq!(terminals.len(), 1, "exactly one terminal");
    match terminals[0] {
        EventKind::TimingSeekCompleted {
            position,
            seek_kind,
        } => {
            assert_eq!(position.0, 200_000, "the landing, in µs");
            assert_eq!(*seek_kind, SeekKind::Command);
        }
        _ => unreachable!("filtered to the terminal"),
    }
    // The terminal's envelope generation matches the accept (§1.4
    // C3's completion-matching gen).
    let terminal_at = session
        .event_index(|kind| matches!(kind, EventKind::TimingSeekCompleted { .. }))
        .must();
    match &session.items[terminal_at] {
        WireItem::Event(event) => assert_eq!(event.generation, Some(Generation(1))),
        WireItem::Receipt(_) => unreachable!("positioned the terminal"),
    }
    // The immediate tick is the terminal's next event, at the landing.
    match &session.items[terminal_at + 1] {
        WireItem::Event(event) => match &event.kind {
            EventKind::TimingTimeTick { time, .. } => assert_eq!(time.0, 200_000),
            _ => unreachable!("the tick directly follows the terminal"),
        },
        WireItem::Receipt(_) => unreachable!("the tick directly follows the terminal"),
    }
}

/// An unseekable source: the seek's terminal is the typed
/// `timing.seek-failed` (stage control, code unsupported, the
/// `not-seekable` reason detail and the old position in µs), playback
/// continues at that old position — never a fabricated landing.
#[test]
fn an_unseekable_source_answers_the_typed_seek_failed() {
    let (tracks, packets) = audio_media(600);
    let mut session = open_session(tracks, packets, budget(12), false, None, false);
    session.poll_until(|s| s == SessionState::Playing, 20, "priming");
    // Virtual time never advanced: the old position is 0 µs — the
    // refusal must report it, never the requested landing.
    session.items.clear();

    session
        .handle
        .seek(
            RequestId(21),
            SeekTarget::Absolute {
                position_ns: 400 * MS,
            },
        )
        .must();
    session.poll_until(|s| s == SessionState::Playing, 40, "continue");
    session.finish_clean();

    let failures: Vec<&EventKind> = session
        .kinds()
        .iter()
        .filter(|kind| matches!(kind, EventKind::TimingSeekFailed { .. }))
        .copied()
        .collect();
    assert_eq!(failures.len(), 1, "exactly one typed refusal");
    match failures[0] {
        EventKind::TimingSeekFailed { error } => {
            assert_eq!(error.stage, Stage::Control);
            assert_eq!(error.code, ErrorCode::Unsupported);
            assert_eq!(
                error.details.get("reason").map(String::as_str),
                Some("not-seekable")
            );
            // The old position (0 µs — the frozen virtual clock),
            // never the requested 400 ms landing.
            assert_eq!(
                error.details.get("position_us").map(String::as_str),
                Some("0")
            );
        }
        _ => unreachable!("filtered to the refusal"),
    }
    // No fake success, and the state trail never left the family.
    assert!(
        session
            .kinds()
            .iter()
            .all(|kind| !matches!(kind, EventKind::TimingSeekCompleted { .. }))
    );
    assert_eq!(session.engine.state(), SessionState::Playing);
}

/// The asymmetric ends: the audio limb drains a tick before the
/// video limb's deep reorder buffer — `playback.end-of-track` for the
/// audio limb with `{trackKind, id}`, then `end-of-media` before
/// `state-changed → ended` (no second end-of-track: the last limb's
/// end IS the media's end).
#[test]
fn end_of_track_precedes_end_of_media_with_the_track_payload() {
    let tracks = vec![
        track(1, MediaKind::Video, 60),
        track(2, MediaKind::Audio, 60),
    ];
    let mut packets = Vec::new();
    for step in 0..30 {
        packets.push(packet(TrackId::new(1), step * 2));
        packets.push(packet(TrackId::new(2), step * 2 + 1));
    }
    let mut session = open_session(tracks, packets, budget(12), true, None, true);
    session.poll_until(|s| s == SessionState::Playing, 40, "priming");
    session.poll_until(|s| s == SessionState::Ended, 400, "drain");
    session.finish_clean();

    let ends: Vec<&EventKind> = session
        .kinds()
        .iter()
        .filter(|kind| matches!(kind, EventKind::PlaybackEndOfTrack { .. }))
        .copied()
        .collect();
    assert_eq!(ends.len(), 1, "the video limb's end is the media's end");
    match ends[0] {
        EventKind::PlaybackEndOfTrack { track_kind, id } => {
            assert_eq!(*track_kind, WireTrackKind::Audio);
            assert_eq!(*id, WireTrackId(2));
        }
        _ => unreachable!("filtered to end-of-track"),
    }
    let eot_at = session
        .event_index(|kind| matches!(kind, EventKind::PlaybackEndOfTrack { .. }))
        .must();
    let eom_at = session
        .event_index(|kind| matches!(kind, EventKind::PlaybackEndOfMedia))
        .must();
    let ended_at = session
        .event_index(|kind| {
            matches!(
                kind,
                EventKind::SessionStateChanged {
                    to: PlaybackState::Ended,
                    ..
                }
            )
        })
        .must();
    assert!(eot_at < eom_at);
    assert!(eom_at < ended_at);
}

/// Demux-stage bounded recovery coalesces into ONE non-fatal
/// `playback.error` per window with the count in its details —
/// flushed at the settling transition, so the frozen virtual window
/// never loses the last count.
#[test]
fn non_fatal_damage_coalesces_into_one_counted_error() {
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

    let mut session = open_script(vec![track(1, MediaKind::Audio, 40)], script);
    session.poll_until(|s| s == SessionState::Ended, 200, "drain");
    session.finish_clean();

    let errors: Vec<&EventKind> = session
        .kinds()
        .iter()
        .filter(|kind| matches!(kind, EventKind::PlaybackError { .. }))
        .copied()
        .collect();
    assert_eq!(errors.len(), 1, "coalesced: {errors:?}");
    match errors[0] {
        EventKind::PlaybackError {
            stage,
            code,
            details,
            fatal,
            ..
        } => {
            assert!(!*fatal);
            assert_eq!(*stage, Stage::Demux);
            assert_eq!(*code, ErrorCode::Corrupt);
            assert_eq!(
                details.get("count").map(String::as_str),
                Some("9"),
                "2 + 3 + 4 units, one count"
            );
        }
        _ => unreachable!("filtered to playback.error"),
    }
    // The state trail still ends cleanly (the recovery was bounded).
    assert_eq!(
        session.state_trail().last(),
        Some(&(PlaybackState::Playing, PlaybackState::Ended))
    );
}

/// A fatal open failure: `playback.error {fatal: true}` first, then
/// `state-changed → error` — the §3.3 ordering, with the payload
/// bridged to the wire's stage/code vocabulary.
#[test]
fn fatal_open_failure_emits_error_then_state_changed_error() {
    let (tracks, packets) = audio_media(40);
    let failure = EngineError::new(
        PipelineStage::Open,
        ErrorKind::Inaccessible {
            reason: media_types::error::AccessDenied::NotFound,
        },
        media_types::error::ErrorContext::new("open.start.missing"),
    );
    let mut session = open_session(tracks, packets, budget(12), true, Some(failure), false);
    session.poll_until(|s| s == SessionState::Error, 10, "fatal");
    session.finish_clean();

    let error_at = session
        .event_index(|kind| matches!(kind, EventKind::PlaybackError { fatal: true, .. }))
        .must();
    let state_at = session
        .event_index(|kind| {
            matches!(
                kind,
                EventKind::SessionStateChanged {
                    to: PlaybackState::Error,
                    ..
                }
            )
        })
        .must();
    assert!(error_at < state_at, "the fatal error precedes its state");
    match &session.kinds()[0] {
        EventKind::SessionStateChanged { from, to } => {
            assert_eq!(*from, PlaybackState::Idle);
            assert_eq!(*to, PlaybackState::Opening);
        }
        _ => unreachable!("the open transition is first"),
    }
    match &session.items[error_at] {
        WireItem::Event(event) => match &event.kind {
            EventKind::PlaybackError {
                stage, code, fatal, ..
            } => {
                assert_eq!(*stage, Stage::Input);
                assert_eq!(*code, ErrorCode::Inaccessible);
                assert!(*fatal);
            }
            _ => unreachable!("positioned the fatal error"),
        },
        WireItem::Receipt(_) => unreachable!("positioned the fatal error"),
    }
    assert_eq!(
        session.state_trail(),
        vec![
            (PlaybackState::Idle, PlaybackState::Opening),
            (PlaybackState::Opening, PlaybackState::Error),
        ]
    );
}

/// The track-selection terminal (`tracks.changed` — the map's row
/// 3.A3 moved into the emitted bucket): the class-A accept bumps the
/// generation, the terminal carries `{trackKind, selected}` under
/// the switch's own generation.
#[test]
fn track_selection_completes_with_the_changed_terminal() {
    let tracks = vec![
        track(1, MediaKind::Audio, 600),
        track(2, MediaKind::Audio, 600),
    ];
    let mut packets = Vec::new();
    for step in 0..300 {
        packets.push(packet(TrackId::new(1), step * 2));
        packets.push(packet(TrackId::new(2), step * 2 + 1));
    }
    let mut session = open_standard(tracks, packets);
    session.poll_until(|s| s == SessionState::Playing, 40, "priming");
    session.items.clear();

    session
        .handle
        .set_audio_track(
            RequestId(30),
            TrackSelector::Id {
                id: TrackId::new(2),
            },
        )
        .must();
    let mut saw_terminal = false;
    for _ in 0..40 {
        session.poll();
        if session
            .kinds()
            .iter()
            .any(|kind| matches!(kind, EventKind::TracksChanged { .. }))
        {
            saw_terminal = true;
            break;
        }
    }
    assert!(saw_terminal, "the selection terminal arrived");
    assert_eq!(session.engine.state(), SessionState::Playing);
    session.finish_clean();

    let receipt = session.receipt_for(RequestId(30));
    assert_eq!(
        receipt.result,
        ReceiptResult::Accepted {
            session: session.handle.session_id(),
            generation: Generation(1),
        },
        "the class-A accept carries the post-bump gen"
    );
    let changed: Vec<&EventKind> = session
        .kinds()
        .iter()
        .filter(|kind| matches!(kind, EventKind::TracksChanged { .. }))
        .copied()
        .collect();
    assert_eq!(changed.len(), 1, "exactly one selection terminal");
    match changed[0] {
        EventKind::TracksChanged {
            track_kind,
            selected,
        } => {
            assert_eq!(*track_kind, TrackChangeKind::Audio);
            assert_eq!(*selected, Some(WireTrackId(2)));
        }
        _ => unreachable!("filtered to tracks.changed"),
    }
    let changed_at = session
        .event_index(|kind| matches!(kind, EventKind::TracksChanged { .. }))
        .must();
    match &session.items[changed_at] {
        WireItem::Event(event) => assert_eq!(
            event.generation,
            Some(Generation(1)),
            "the terminal under the switch's own gen"
        ),
        WireItem::Receipt(_) => unreachable!("positioned the terminal"),
    }
}

// ---------------------------------------------------------------------------
// §C — the named gaps (the eleven `WiredLater` rows, doc 05 §3.2)
// ---------------------------------------------------------------------------

/// Drive the fullest session the engine offers today (open → prime →
/// play → pause → resume → end, virtual time across the tick
/// interval) and answer the wire event ids it produced — the shared
/// scenario behind every gap assertion below.
fn full_session_ids() -> Vec<&'static str> {
    let (tracks, packets) = audio_media(600);
    let mut session = open_standard(tracks, packets);
    session.poll_until(|s| s == SessionState::Playing, 20, "priming");
    advance_ms(&session.now, 300);
    session.poll();
    session.handle.pause(RequestId(1)).must();
    session.poll();
    session.handle.play(RequestId(2)).must();
    session.poll();
    session.poll_until(|s| s == SessionState::Ended, 400, "drain");
    session.finish_clean();
    session.ids()
}

/// The live negative half of the completeness map: none of the eleven
/// `WiredLater` rows may cross the wire today — a phantom emission
/// (an engine kind leaking an unmapped row) is as much a contract
/// break as a missing one. This one is NOT ignored: it passes today
/// and keeps passing until a row's wiring lands, at which point the
/// matching gap test below is un-ignored and this row moves to
/// `Emitted` in the map (they travel together).
#[test]
fn the_wired_later_rows_stay_silent_today() {
    let ids = full_session_ids();
    for row in CATALOG_EMISSIONS {
        if !row.status.is_emitted() {
            assert!(
                !ids.contains(&row.id.as_str()),
                "{} leaked onto the wire before its trigger was wired",
                row.id.as_str()
            );
        }
    }
}

/// One named gap per `WiredLater` row: the assertion is the
/// **positive** contract (the row crosses the wire when its trigger
/// occurs), kept with `#[ignore]` naming the owner — never deleted
/// (never weaken tests). When the owning card lands the emission, it
/// un-ignores the test and flips the map row to `Emitted`; the
/// `the_wired_later_rows_stay_silent_today` guard above forces the
/// pair to travel together.
#[test]
#[ignore = "gap: session.opening-progress not wired — handoff H-1 (streaming opens are Phase 6; local opens are synchronous)"]
fn gap_session_opening_progress() {
    assert!(
        full_session_ids().contains(&EventId::SessionOpeningProgress.as_str()),
        "a network open must report progress; no streaming source exists in the engine yet"
    );
}

#[test]
#[ignore = "gap: playback.rate-changed not wired — handoff H-2 (3.B1 rate support; no set-rate engine input exists)"]
fn gap_playback_rate_changed() {
    assert!(
        full_session_ids().contains(&EventId::PlaybackRateChanged.as_str()),
        "a rate change must echo on the wire; no engine rate input exists yet"
    );
}

#[test]
#[ignore = "gap: audio.device-lost not wired — handoff H-3 (2.C3/7.C1 device observation; the null device cannot be lost)"]
fn gap_audio_device_lost() {
    assert!(
        full_session_ids().contains(&EventId::AudioDeviceLost.as_str()),
        "an OS device removal must be reported; no device observation exists yet"
    );
}

#[test]
#[ignore = "gap: audio.device-changed not wired — handoff H-4 (2.C3/7.C1 device observation)"]
fn gap_audio_device_changed() {
    assert!(
        full_session_ids().contains(&EventId::AudioDeviceChanged.as_str()),
        "an output device change must be reported; no device observation exists yet"
    );
}

#[test]
#[ignore = "gap: video.screenshot-saved not wired — handoff H-5 (renderer/host work; the sink seam never writes files)"]
fn gap_video_screenshot_saved() {
    assert!(
        full_session_ids().contains(&EventId::VideoScreenshotSaved.as_str()),
        "a written screenshot must be reported; the engine cannot take screenshots yet"
    );
}

#[test]
#[ignore = "gap: playlist.changed not wired — handoff H-6 (no playlist model in the engine; Phase 4, 4.D1)"]
fn gap_playlist_changed() {
    assert!(
        full_session_ids().contains(&EventId::PlaylistChanged.as_str()),
        "a playlist mutation must be reported; no playlist model exists yet"
    );
}

#[test]
#[ignore = "gap: config.setting-changed not wired — handoff H-7 (settings writes are host/config-layer work; doc 07, Phase 4)"]
fn gap_config_setting_changed() {
    assert!(
        full_session_ids().contains(&EventId::ConfigSettingChanged.as_str()),
        "a settings write must be reported; the engine has no settings surface yet"
    );
}

#[test]
#[ignore = "gap: plugins.loaded not wired — handoff H-8 (plugin registry, doc 06)"]
fn gap_plugins_loaded() {
    assert!(
        full_session_ids().contains(&EventId::PluginsLoaded.as_str()),
        "a registered plugin must be reported; no plugin registry exists yet"
    );
}

#[test]
#[ignore = "gap: plugins.failed not wired — handoff H-9 (plugin registry, doc 06)"]
fn gap_plugins_failed() {
    assert!(
        full_session_ids().contains(&EventId::PluginsFailed.as_str()),
        "a failed plugin load must be reported; no plugin registry exists yet"
    );
}

#[test]
#[ignore = "gap: plugins.crashed not wired — handoff H-10 (sandboxed WASM host, plugin-host-wasm, Phase 7)"]
fn gap_plugins_crashed() {
    assert!(
        full_session_ids().contains(&EventId::PluginsCrashed.as_str()),
        "a crashed sandboxed extension must be reported; no WASM host exists yet"
    );
}

#[test]
#[ignore = "gap: system.log not wired — handoff H-11 (no logging bridge; the doc 05 §3.4 burst-bound drop refinement activates here)"]
fn gap_system_log() {
    assert!(
        full_session_ids().contains(&EventId::SystemLog.as_str()),
        "surfaced log lines must cross the wire; no logging bridge exists yet"
    );
}
