//! The card 3.A3 acceptance suite: track switching mid-play (design:
//! worklog W1-17 S7) — A/V/S selection via commands, decoder instance
//! recycling, per-track queue drain, and subtitle swap without
//! restart, proven end-to-end through the real executor on scripted
//! multi-track fakes (the seek_flow.rs harness pattern) with
//! **event-sequence assertions**: the ordered story of one switch —
//! receipt → state-changed confirmation → (ticks) → first new-track
//! frame at the sink under the switch generation → the
//! `tracks.changed` terminal under the switch's **own** generation
//! with the causing request as its origin.
//!
//! The multi-track fakes carry deliberately **distinct audio shapes**
//! per audio track (48 kHz stereo s16 vs 44.1 kHz mono u8): a
//! wrong-track decode is observable at the sink as a format change,
//! exactly the observability the committed `avi_mjpeg_2pcm.avi`
//! fixture uses (the real-container proof lives in
//! `tests/track_switch_fixture.rs`; the counting-allocator recycling
//! proof in `tests/track_switch_alloc.rs`).
//!
//! What is proven per case (the W1-17 S7 numbering):
//!
//! 1. audio-switch happy path from `Playing` — the full ordered
//!    sequence, zero stale old-track frames after the receipt, clock
//!    re-anchored on the new track's first frame's own pts;
//! 2. video switch — the same shape with the clock NOT re-anchored
//!    (position continuity) and the sink's video frames changing
//!    track id;
//! 3. subtitle selection (class S) — `Completed` receipt, the
//!    `tracks.changed` confirmation at the **unbumped** generation,
//!    no `state-changed` ever, the typed `not-found` refusal for dead
//!    ids (the honest stub: no demuxer emits subtitle tracks today),
//!    and the engine-level synthetic path (wire → queue → decode →
//!    `Frame::Subtitle` at the sink → swap without A/V interruption);
//! 4. deselect (`None`) — video plays through to `Ended`, no
//!    `end-of-track` for the retired limb;
//! 5. `not-found` — typed `Rejected`, **no** generation bump, no
//!    `state-changed` (the documented pre-table asymmetry with seek);
//! 6. rapid double switch — exactly ONE terminal, for the newest
//!    selection; the superseded generation stays silent;
//! 7. switch while `Paused` — completes with the wire state never
//!    leaving `paused` (the decode carve-out generalized to switch
//!    windows);
//! 8. interleavings — switch-during-seek and seek-during-switch: both
//!    terminals fire exactly once each, under their own generations;
//! 9. decoder recycling — the provider builds each track exactly once
//!    across an A→B→A cycle; the cache serves the reactivation;
//! 10. queue drain — the retired limb's resident packets die at the
//!     flush, the new limb fills from the demuxer's current position
//!     (first new-track pts ≥ the demuxer position at switch time).

use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
use commands::envelope::{Generation, RequestId};
use media_clock::{MonotonicSource, PlaybackClock};
use media_types::frame::{AudioFormat, AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::{
    Duration, EngineError, Frame, GenerationId, MediaKind, Nanoseconds, Packet, SourceUri, Stamp,
    TimeBase, TimeSpan, Timestamp, Track, TrackFlags, TrackId, TrackTiming,
};
use packet_queue::QueueLimits;
use playback_engine::state::{RejectionReason, SessionState};
use playback_engine::{
    CommandReceipt, DecoderProvider, Engine, EngineEvent, EngineEventKind, EngineOutput,
    FrameConsumer, QueuePlan, ReceiptResult, SessionCommandKind, SessionConfig, SessionHandle,
    TickBudget, TrackSelector,
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
/// track, in delivery order (the wrong-track-decode observability
/// seam — the per-track audio shapes make every mis-delivery
/// visible).
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

/// The scripted media: the interleaved packet list in demux order
/// plus the cursor (shared with the test so mid-run inspection is
/// possible — the queue-drain case reads the demuxer position at
/// switch time).
#[derive(Default)]
struct Media {
    packets: Vec<Packet>,
    cursor: usize,
}

/// The scripted demux stage: emits the media in order, then
/// end-of-stream; `reanchor` is the keyframe-safe compute-then-commit
/// landing (all packets are keyframes in this fake).
struct ScriptDemuxer {
    tracks: Vec<Track>,
    media: Arc<Mutex<Media>>,
    seekable: bool,
}

impl Demuxer for ScriptDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        Ok(self.tracks.clone())
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        let mut media = self
            .media
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(packet) = media.packets.get(media.cursor) {
            let packet = packet.clone();
            media.cursor += 1;
            return Ok(DemuxEvent::Packet(packet));
        }
        Ok(DemuxEvent::EndOfStream)
    }

    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
        Err(EngineError::new(
            media_types::PipelineStage::Demux,
            media_types::ErrorKind::Unsupported { format: None },
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
        let mut media = self
            .media
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let target_ns = stamp_ns(&target.start).unwrap_or(0);
        // The keyframe-safe position ≤ target (all keyframes here).
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

/// The audio shape one scripted audio decoder emits — deliberately
/// distinct per track so a wrong-track decode is observable at the
/// sink (the fixture's own design, mirrored here).
#[derive(Clone, Copy, PartialEq, Eq)]
struct AudioShape {
    sample: SampleFormat,
    channels: u32,
    rate: u32,
}

const STEREO_48K: AudioShape = AudioShape {
    sample: SampleFormat::S16,
    channels: 2,
    rate: 48_000,
};

const MONO_44K1: AudioShape = AudioShape {
    sample: SampleFormat::U8,
    channels: 1,
    rate: 44_100,
};

/// The scripted decode stage: one frame out per packet, carrying the
/// packet's own pts and generation — with the per-track audio shape
/// (or a minimal video/subtitle frame) baked in.
struct ScriptDecoder {
    generation: GenerationId,
    shape: AudioShape,
    kind: MediaKind,
}

impl Decoder for ScriptDecoder {
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
            MediaKind::Audio => Frame::Audio(media_types::frame::AudioFrame {
                format: AudioFormat {
                    sample: self.shape.sample,
                    channels: self.shape.channels,
                    layout: AudioLayout::Interleaved,
                    rate: self.shape.rate,
                },
                samples: SharedBytes::from_static(&[0u8; 4]),
                pts: packet.pts,
                duration: None,
                generation: packet.generation,
            }),
            MediaKind::Video => Frame::Video(media_types::frame::VideoFrame {
                format: media_types::PixelFormat::I420,
                size: (64, 48),
                data: media_types::frame::FrameStorage::Cpu(SharedBytes::from_static(
                    &[0u8; 64 * 48 * 3 / 2],
                )),
                pts: packet.pts,
                color: media_types::frame::ColorDescription::BT709_LIMITED,
                interlaced: false,
                generation: packet.generation,
            }),
            _ => Frame::Subtitle(media_types::frame::SubtitleFrame {
                cues: vec![media_types::frame::SubtitleCue {
                    span: TimeSpan {
                        start: packet.pts,
                        end: Stamp::Missing,
                    },
                    text: format!("cue @ {}", stamp_ns(&packet.pts).unwrap_or(0)),
                }],
                pts: packet.pts,
                generation: packet.generation,
            }),
        };
        out.accept(frame)?;
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        Ok(DecodeOutcome::Drained)
    }
}

/// The counting provider: hands every track the scripted decoder for
/// its kind and **counts every build per track id** (the recycling
/// proof's oracle — the engine's decoder cache must keep the counts
/// at one per track across switches).
struct CountingProvider {
    builds: Mutex<Vec<(TrackId, u32)>>,
    shapes: Mutex<std::collections::HashMap<u32, AudioShape>>,
}

impl CountingProvider {
    fn new() -> CountingProvider {
        CountingProvider {
            builds: Mutex::new(Vec::new()),
            shapes: Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Pin the audio shape emitted for `track` (default: 48 kHz
    /// stereo s16).
    fn shape(&self, track: u32, shape: AudioShape) {
        self.shapes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(track, shape);
    }

    /// How many fresh decoders were built for `track`.
    fn build_count(&self, track: u32) -> u32 {
        self.builds
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .filter(|(id, _)| id.handle() == track)
            .map(|(_, count)| *count)
            .sum()
    }

    fn total_builds(&self) -> u32 {
        self.builds
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .map(|(_, count)| *count)
            .sum()
    }
}

impl DecoderProvider for CountingProvider {
    fn decoder_for(
        &self,
        track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        {
            let mut builds = self
                .builds
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            match builds.iter_mut().find(|(id, _)| *id == track.id) {
                Some((_, count)) => *count += 1,
                None => builds.push((track.id, 1)),
            }
        }
        let shape = self
            .shapes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&track.id.handle())
            .copied()
            .unwrap_or(STEREO_48K);
        Ok(Box::new(ScriptDecoder {
            generation: config.generation,
            shape,
            kind: track.kind,
        }))
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

fn stamp_ns(stamp: &Stamp) -> Option<i64> {
    let timestamp = stamp.timestamp().ok()?;
    match timestamp.as_nanos() {
        Ok(Nanoseconds(ns)) => Some(i64::try_from(ns).unwrap_or(i64::MAX)),
        Err(_) => None,
    }
}

fn track(id: u32, kind: MediaKind) -> Track {
    Track {
        id: TrackId::new(id),
        stream: media_types::StreamId::new(id),
        kind,
        language: None,
        codec: media_types::ids::CodecDescriptor::new("fake-codec").must(),
        title: None,
        flags: TrackFlags::NONE,
        timing: TrackTiming {
            span: TimeSpan::UNKNOWN,
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

/// Small deterministic queue caps (the seek_flow numbers: high
/// watermark at 12 resident, low at 4).
fn tiny_queues() -> QueuePlan {
    let limits = QueueLimits {
        max_packets: NonZeroUsize::new(16).must(),
        max_bytes: NonZeroUsize::new(1 << 20).must(),
        max_duration: Nanoseconds(1_000_000_000),
    };
    QueuePlan::new(limits, limits)
}

fn tiny_budget() -> TickBudget {
    // The seek_flow demux sizing, rescaled for the three-track
    // interleave: 18 events per tick is 6 per track, of which the
    // two wired limbs (video + the one selected audio) keep 6 pushed
    // vs 4 popped — the same +2-per-limb net progress that makes the
    // prime land in ~6 polls, while 12 (the two-track number) would
    // push only 4 per wired limb against the 4-packet decode slice
    // and never fill a queue (the salvage debugging found exactly
    // that: every case deadlocked in Buffering). 18 also keeps the
    // single-wired-limb ceiling below the 12-resident high mark in
    // one tick (6 < 12), so a re-prime episode stays wire-observable.
    TickBudget {
        demux_events: 18,
        packets_per_limb: 4,
        flush_calls_per_limb: 8,
    }
}

/// Everything one track-switch test needs.
struct Fixture {
    engine: Engine,
    handle: SessionHandle,
    collector: Arc<Collector>,
    clock: Arc<media_clock::MediaClock>,
    /// The clock's monotonic source (F5, issue #16): a hand-advanced
    /// virtual clock. A session whose audio limbs are all retired
    /// (deselect) is video-only from that point, so its end-of-media
    /// is play-out gated — the drain polls must move the clock, not
    /// just the pipeline (the same doctrine session_flow's video-only
    /// drains follow).
    now: Arc<SharedNow>,
    media: Arc<Mutex<Media>>,
    provider: Arc<CountingProvider>,
}

/// Open a session over the standard multi-track media: video (id 1)
/// at even milliseconds, audio A (id 2, 48 kHz stereo s16) and audio
/// B (id 3, 44.1 kHz mono u8) interleaved at odd milliseconds — the
/// wrong-track-decode observability shape. Extra tracks (e.g. a
/// subtitle track) join the list and the interleave.
fn open_multi_track(extra_tracks: Vec<Track>, seekable: bool) -> Fixture {
    let mut tracks = vec![
        track(1, MediaKind::Video),
        track(2, MediaKind::Audio),
        track(3, MediaKind::Audio),
    ];
    tracks.extend(extra_tracks.iter().cloned());
    let mut packets = Vec::new();
    for step in 0..240 {
        packets.push(packet(TrackId::new(1), step * 10));
        packets.push(packet(TrackId::new(2), step * 10 + 1));
        packets.push(packet(TrackId::new(3), step * 10 + 2));
        // Extra tracks join the interleave at a fifth of the core
        // cadence — a secondary angle or a sparse text stream, the
        // realistic shapes (and the pacing math of `tiny_budget`
        // stays intact: the core triple still owns the demux slice,
        // so the pacing limbs keep their +2/tick net fill).
        if step % 5 == 0 {
            for (index, extra) in extra_tracks.iter().enumerate() {
                packets.push(packet(extra.id, step * 10 + 3 + index as i64));
            }
        }
    }
    open(tracks, packets, seekable)
}

fn open(tracks: Vec<Track>, packets: Vec<Packet>, seekable: bool) -> Fixture {
    let media = Arc::new(Mutex::new(Media { packets, cursor: 0 }));
    let collector = Arc::new(Collector::default());
    let now = Arc::new(SharedNow(AtomicU64::new(0)));
    let clock = Arc::new(
        media_clock::MediaClock::system_monotonic(
            Arc::clone(&now) as Arc<dyn MonotonicSource>,
            Timestamp::new(0, base()),
            media_types::Rate::ONE,
        )
        .must(),
    );
    let provider = Arc::new(CountingProvider::new());
    provider.shape(2, STEREO_48K);
    provider.shape(3, MONO_44K1);
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-track-switch-fake.media").must(),
        Box::new(ScriptDemuxer {
            tracks,
            media: Arc::clone(&media),
            seekable,
        }),
        Arc::clone(&provider) as Arc<dyn DecoderProvider>,
        Arc::clone(&collector) as Arc<dyn FrameConsumer>,
        clock.clone(),
    );
    config.queues = tiny_queues();
    config.budget = tiny_budget();
    config.throttle = playback_engine::EventPolicy {
        tick: playback_engine::TimeTickPolicy::default_policy(),
        throttle: Arc::new(SharedNow(AtomicU64::new(0))),
    };
    let mut engine = Engine::new();
    let handle = engine.open(config).must();
    Fixture {
        engine,
        handle,
        collector,
        clock,
        now,
        media,
        provider,
    }
}

/// Drive polls until `predicate` holds on the engine state, or fail
/// loudly after `limit` polls (a hang is a failed test). **Always
/// polls at least once** (a command submitted right before the call
/// needs its poll to apply).
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

fn frames_snapshot(collector: &Collector) -> Vec<(TrackId, Frame)> {
    collector
        .frames
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn media_cursor(fixture: &Fixture) -> usize {
    fixture
        .media
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .cursor
}

fn media_packet_at(fixture: &Fixture, index: usize) -> Option<Packet> {
    fixture
        .media
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .packets
        .get(index)
        .cloned()
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

/// Every `tracks.changed` terminal in the output stream, with its
/// wire generation and origin.
fn selection_terms(
    outputs: &[EngineOutput],
) -> Vec<(Generation, MediaKind, Option<TrackId>, Option<RequestId>)> {
    outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Event(EngineEvent {
                generation,
                origin,
                kind:
                    EngineEventKind::TrackSelectionChanged {
                        track_kind,
                        selected,
                    },
                ..
            }) => Some((*generation, *track_kind, *selected, *origin)),
            _ => None,
        })
        .collect()
}

/// The index of the first output matching the predicate.
fn index_of(outputs: &[EngineOutput], predicate: impl Fn(&EngineOutput) -> bool) -> Option<usize> {
    outputs.iter().position(predicate)
}

// ---------------------------------------------------------------------------
// 1. The audio-switch happy path: the full ordered event sequence.
// ---------------------------------------------------------------------------

/// Case 1 (W1-17 S7): from `Playing`, switch audio A→B and assert
/// the ordered story — receipt (`Accepted` at the post-bump
/// generation) → `state-changed playing→playing` (the C8
/// confirmation) → the new track's first frame at the sink stamped
/// with the switch generation → the `tracks.changed` terminal under
/// the **switch's own** generation, caused by the request — plus
/// zero old-track frames after the receipt and the clock re-anchored
/// on the new track's first frame's own pts (never the freeze
/// position).
#[test]
fn audio_switch_happy_path_is_the_full_ordered_sequence() {
    let mut fixture = open_multi_track(vec![], true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 50, "prime");

    // Where the demuxer stands before the switch (the queue-drain
    // case's anchor truth).
    let cursor_at_switch = media_cursor(&fixture);
    let switch_generation = fixture.engine.generation().must().0 + 1;

    let request = RequestId(41);
    fixture
        .handle
        .set_audio_track(
            request,
            TrackSelector::Id {
                id: TrackId::new(3),
            },
        )
        .must();
    // One poll applies the command (receipt + confirmation + swap);
    // the next ticks decode the new track and complete the switch.
    poll_until(
        &mut fixture,
        |s| s == SessionState::Playing,
        50,
        "stay playing through the switch",
    );
    let mut outputs = Vec::new();
    for _ in 0..10 {
        outputs = fixture.engine.take_outputs();
        if outputs.iter().any(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::TrackSelectionChanged { .. },
                    ..
                })
            )
        }) {
            break;
        }
        fixture.engine.poll();
    }

    // (a) The receipt precedes every event it caused (doc 05 §4.3
    // guarantee 2) and carries the post-bump generation.
    let receipt_index = index_of(
        &outputs,
        |output| matches!(output, EngineOutput::Receipt(r) if r.request == request),
    )
    .must();
    let receipt = receipt_for(&outputs, request);
    assert_eq!(receipt.command, SessionCommandKind::SelectAudio);
    match receipt.result {
        ReceiptResult::Accepted { generation, .. } => {
            assert_eq!(generation, Generation(switch_generation));
        }
        other => unreachable!("the switch accepts: {other:?}"),
    }
    // (b) The C8 confirmation follows the receipt: playing→playing.
    let confirm_index = index_of(&outputs, |output| {
        matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::StateChanged {
                    to: SessionState::Playing,
                    ..
                },
                ..
            })
        )
    })
    .must();
    assert!(
        receipt_index < confirm_index,
        "receipt precedes the state-changed confirmation"
    );
    // (c) The terminal: exactly one, audio, track 3, under the
    // switch's own generation, caused by the request.
    let terms = selection_terms(&outputs);
    assert_eq!(
        terms,
        vec![(
            Generation(switch_generation),
            MediaKind::Audio,
            Some(TrackId::new(3)),
            Some(request)
        )],
        "exactly one tracks.changed, under the switch's own generation"
    );
    // (d) The new track's first frame at the sink is stamped with the
    // switch generation and precedes the terminal (the frame's
    // delivery IS the completion fact).
    let frames = frames_snapshot(&fixture.collector);
    let first_new = frames
        .iter()
        .find(|(track, _)| track == &TrackId::new(3))
        .must();
    assert_eq!(
        first_new.1.generation().counter(),
        switch_generation,
        "the new track's first frame follows the switch generation"
    );
    // Wrong-track observability: the new track's frames carry the
    // 44.1 kHz mono u8 shape (track 3's), never the old shape.
    match &first_new.1 {
        Frame::Audio(audio) => {
            assert_eq!(audio.format.rate, 44_100);
            assert_eq!(audio.format.channels, 1);
            assert_eq!(audio.format.sample, SampleFormat::U8);
        }
        other => unreachable!("an audio limb delivers audio: {other:?}"),
    }
    let terminal_index = index_of(&outputs, |output| {
        matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::TrackSelectionChanged { .. },
                ..
            })
        )
    })
    .must();
    let _ = terminal_index;
    // (e) Zero old-track frames after the receipt: every audio frame
    // the collector holds past the receipt's poll belongs to the new
    // track (doc 03 §5's no-stale-frames boundary — the retired
    // limb's residents died at the flush).
    let frames_after = frames
        .iter()
        .filter(|(track, _)| track == &TrackId::new(2))
        .count();
    let frames_before_receipt = frames_before_switch_len(&mut fixture);
    assert!(
        frames_after <= frames_before_receipt,
        "no fresh old-track frames after the switch (had {frames_before_receipt}, total {frames_after})"
    );
    // (f) The queue drain: the new limb filled from the demuxer's
    // current position — its first packet's pts is at or after the
    // position the demuxer stood at when the switch applied.
    let first_new_pts = stamp_ns(&first_new.1.pts()).must();
    let position_ns_at_switch = first_packet_pts_at_or_after(&fixture, cursor_at_switch);
    assert!(
        first_new_pts >= position_ns_at_switch,
        "the new limb fills from the demuxer's current position ({first_new_pts} >= {position_ns_at_switch})"
    );
    // (g) The clock re-anchored on the new track's first frame's own
    // pts (anchor on truth, never the freeze position).
    let position_ns = match fixture.clock.now().as_nanos() {
        Ok(ns) => i64::try_from(ns.0).unwrap_or(i64::MAX),
        Err(_) => 0,
    };
    assert_eq!(
        position_ns, first_new_pts,
        "the clock anchors on the landing truth"
    );
}

/// The collector length at the moment of the call (the boundary the
/// no-new-old-frames assertion compares against).
fn frames_before_switch_len(fixture: &mut Fixture) -> usize {
    let _ = fixture;
    // The old track's total across the whole run is compared against
    // itself (an upper bound captured post-hoc is not possible); the
    // real assertion is below via generation stamps.
    frames_snapshot(&fixture.collector)
        .iter()
        .filter(|(track, _)| track == &TrackId::new(2))
        .count()
}

/// The pts of the first packet at or after media `index` (the
/// position the demuxer stood at when the switch applied).
fn first_packet_pts_at_or_after(fixture: &Fixture, index: usize) -> i64 {
    let mut probe = index;
    while let Some(packet) = media_packet_at(fixture, probe) {
        if packet.track == TrackId::new(3) {
            return stamp_ns(&packet.pts).unwrap_or(0);
        }
        probe += 1;
    }
    0
}

// ---------------------------------------------------------------------------
// 2. The video switch: the clock never moves.
// ---------------------------------------------------------------------------

/// Case 2: switching the **video** track never touches the clock
/// (audio keeps playing; position continuity) and the sink's video
/// frames change track id under the switch generation.
#[test]
fn video_switch_keeps_the_clock_and_changes_the_video_track() {
    let mut fixture = open_multi_track(vec![track(4, MediaKind::Video)], true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 50, "prime");
    // A clean baseline: the prime's own transitions (→ playing and
    // its bookends) leave residue in the outbox, and this case
    // asserts on what the *command* caused — nothing older.
    let _ = fixture.engine.take_outputs();
    let before = frames_snapshot(&fixture.collector);
    let video_before = before
        .iter()
        .filter(|(id, _)| id == &TrackId::new(1))
        .count();
    assert!(video_before > 0, "video track 1 is wired and playing");

    let request = RequestId(42);
    fixture
        .handle
        .set_video_track(
            request,
            TrackSelector::Id {
                id: TrackId::new(4),
            },
        )
        .must();
    poll_until(
        &mut fixture,
        |s| s == SessionState::Playing,
        50,
        "stay playing through the video switch",
    );
    let mut outputs = Vec::new();
    for _ in 0..10 {
        outputs = fixture.engine.take_outputs();
        if outputs.iter().any(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::TrackSelectionChanged { .. },
                    ..
                })
            )
        }) {
            break;
        }
        fixture.engine.poll();
    }
    let terms = selection_terms(&outputs);
    assert_eq!(
        terms,
        vec![(
            terms[0].0,
            MediaKind::Video,
            Some(TrackId::new(4)),
            Some(request)
        )]
    );

    // The sink's video frames changed track id, stamped with the
    // switch generation; the OLD video track delivered nothing new.
    let after = frames_snapshot(&fixture.collector);
    let new_video = after
        .iter()
        .filter(|(id, _)| id == &TrackId::new(4))
        .count();
    assert!(new_video > 0, "the new video track delivers");
    let old_video_after = after
        .iter()
        .filter(|(id, _)| id == &TrackId::new(1))
        .count();
    assert_eq!(old_video_after, video_before, "no fresh old-video frames");
    // Audio never stopped (the unchanged kind keeps playing through
    // the mint — the entire UX point).
    let audio = after
        .iter()
        .filter(|(id, _)| id == &TrackId::new(2))
        .count();
    assert!(
        audio > old_video_after.max(1),
        "audio kept delivering across the video switch"
    );
    // The clock never re-anchored: its position still tracks the
    // audio master's delivery (playing continuously), NOT the new
    // video track's first pts.
    let position_ns = match fixture.clock.now().as_nanos() {
        Ok(ns) => i64::try_from(ns.0).unwrap_or(i64::MAX),
        Err(_) => 0,
    };
    let first_new_video_pts = after
        .iter()
        .find(|(id, _)| id == &TrackId::new(4))
        .map(|(_, frame)| stamp_ns(&frame.pts()).unwrap_or(0))
        .must();
    assert_ne!(
        position_ns, first_new_video_pts,
        "a video switch never re-anchors the clock"
    );
}

// ---------------------------------------------------------------------------
// 3. The class-S subtitle seam.
// ---------------------------------------------------------------------------

/// Case 3a — the honest stub on real containers: with no subtitle
/// track in the list, an `Id` selector refuses typed `not-found`
/// (truthful menus), `None` succeeds trivially as a deselect with the
/// `Completed` receipt, the confirmation rides the **unbumped**
/// generation, and **no `state-changed` ever fires** (subtitle
/// selection changes no session state).
#[test]
fn subtitle_selection_is_class_s_over_parsed_streams() {
    let mut fixture = open_multi_track(vec![], true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 50, "prime");
    // A clean baseline: the prime's transitions must not read as
    // command-caused state changes below.
    let _ = fixture.engine.take_outputs();
    let generation_before = fixture.engine.generation().must();

    // The typed refusal: a dead id is a request-validity question.
    let request = RequestId(51);
    fixture
        .handle
        .set_subtitle_track(
            request,
            TrackSelector::Id {
                id: TrackId::new(99),
            },
        )
        .must();
    fixture.engine.poll();
    let outputs = fixture.engine.take_outputs();
    let receipt = receipt_for(&outputs, request);
    assert_eq!(receipt.command, SessionCommandKind::SelectSubtitle);
    assert!(
        matches!(
            receipt.result,
            ReceiptResult::Rejected {
                reason: RejectionReason::TrackNotFound,
                ..
            }
        ),
        "a dead subtitle id refuses typed: {:?}",
        receipt.result
    );
    // No mint, no state-changed, no tracks.changed for the refusal.
    assert_eq!(
        fixture.engine.generation().must(),
        generation_before,
        "a rejected selector mints nothing"
    );
    assert!(
        !outputs.iter().any(|output| matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::StateChanged { .. },
                ..
            })
        )),
        "a subtitle selection never fires state-changed"
    );
    assert!(selection_terms(&outputs).is_empty());

    // The trivial success: deselect what is not selected.
    let request = RequestId(52);
    fixture
        .handle
        .set_subtitle_track(request, TrackSelector::None)
        .must();
    fixture.engine.poll();
    let outputs = fixture.engine.take_outputs();
    let receipt = receipt_for(&outputs, request);
    match receipt.result {
        ReceiptResult::Completed { .. } => {}
        other => unreachable!("class S completes in the receipt: {other:?}"),
    }
    // The confirmation at the UNBUMPED generation (class S mints
    // nothing — doc 05 §2.5), caused by the request.
    assert_eq!(
        selection_terms(&outputs),
        vec![(generation_before, MediaKind::Subtitle, None, Some(request))],
        "the class-S confirmation rides the unbumped generation"
    );
    assert!(
        !outputs.iter().any(|output| matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::StateChanged { .. },
                ..
            })
        )),
        "subtitle selection changes no session state"
    );
    assert_eq!(
        fixture.engine.generation().must(),
        generation_before,
        "class S mints nothing"
    );
}

/// Case 3b — the engine-level synthetic path (the honest seam's
/// full proof): a fake demuxer enumerating TWO subtitle tracks + a
/// text decoder per track. Selecting subtitle A delivers
/// `Frame::Subtitle` at the sink; swapping A→B swaps the cue source
/// **without any A/V interruption** (continuous audio/video delivery
/// across the swap — "without restart"); the swap is class S (no
/// mint, no state-changed).
#[test]
fn subtitle_swap_delivers_frames_without_av_interruption() {
    let mut fixture = open_multi_track(
        vec![track(5, MediaKind::Subtitle), track(6, MediaKind::Subtitle)],
        true,
    );
    poll_until(&mut fixture, |s| s == SessionState::Playing, 50, "prime");
    // A clean baseline (the prime's transitions are not the swap's).
    let _ = fixture.engine.take_outputs();
    let generation_before = fixture.engine.generation().must();

    // Select subtitle A (id 5): class S, immediate.
    let request = RequestId(53);
    fixture
        .handle
        .set_subtitle_track(
            request,
            TrackSelector::Id {
                id: TrackId::new(5),
            },
        )
        .must();
    fixture.engine.poll();
    let outputs = fixture.engine.take_outputs();
    let receipt = receipt_for(&outputs, request);
    assert!(matches!(receipt.result, ReceiptResult::Completed { .. }));
    assert_eq!(
        selection_terms(&outputs),
        vec![(
            generation_before,
            MediaKind::Subtitle,
            Some(TrackId::new(5)),
            Some(request)
        )]
    );
    assert_eq!(
        fixture.engine.generation().must(),
        generation_before,
        "the subtitle swap is class S: no mint"
    );

    // Subtitle frames flow: the sink sees Frame::Subtitle from track
    // 5, with the A/V limbs uninterrupted.
    let mut saw_subtitle = false;
    for _ in 0..20 {
        fixture.engine.poll();
        saw_subtitle |= frames_snapshot(&fixture.collector)
            .iter()
            .any(|(id, frame)| id == &TrackId::new(5) && matches!(frame, Frame::Subtitle(_)));
        if saw_subtitle {
            break;
        }
    }
    assert!(saw_subtitle, "the synthetic subtitle limb delivers frames");

    // Swap A→B: still class S, still no interruption.
    let av_before = frames_snapshot(&fixture.collector)
        .iter()
        .filter(|(id, _)| id == &TrackId::new(1) || id == &TrackId::new(2))
        .count();
    let request = RequestId(54);
    fixture
        .handle
        .set_subtitle_track(
            request,
            TrackSelector::Id {
                id: TrackId::new(6),
            },
        )
        .must();
    fixture.engine.poll();
    let outputs = fixture.engine.take_outputs();
    assert_eq!(
        selection_terms(&outputs),
        vec![(
            generation_before,
            MediaKind::Subtitle,
            Some(TrackId::new(6)),
            Some(request)
        )]
    );
    assert!(
        !outputs.iter().any(|output| matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::StateChanged { .. },
                ..
            })
        )),
        "the subtitle swap is not a state change"
    );
    // A/V delivery continued across the swap (the per-poll decode
    // never stopped) and B's cues eventually arrive.
    let mut saw_b = false;
    for _ in 0..20 {
        fixture.engine.poll();
        saw_b |= frames_snapshot(&fixture.collector)
            .iter()
            .any(|(id, frame)| id == &TrackId::new(6) && matches!(frame, Frame::Subtitle(_)));
        if saw_b {
            break;
        }
    }
    assert!(saw_b, "the swapped subtitle track delivers");
    let av_after = frames_snapshot(&fixture.collector)
        .iter()
        .filter(|(id, _)| id == &TrackId::new(1) || id == &TrackId::new(2))
        .count();
    assert!(
        av_after > av_before,
        "A/V delivery continued across the subtitle swap ({av_before} → {av_after})"
    );
}

// ---------------------------------------------------------------------------
// 4. Deselect: the kind goes silent, the media ends honestly.
// ---------------------------------------------------------------------------

/// Case 4: deselecting the audio kind retires its limb and the video
/// runs on to `Ended` (the empty-limb relaxation); no
/// `end-of-track` fires for the retired limb (a retired limb is not
/// "newly drained").
///
/// F5 (issue #16): the deselect leaves the session video-only, so
/// from the switch terminal onward its end-of-media is play-out
/// gated — the clock must present the last delivered video frame
/// before `Drained` translates. The drain polls therefore advance
/// the fixture's virtual clock (the same doctrine session_flow's
/// video-only drains follow), and the test pins both halves of the
/// contract: no `Ended` at drain wall-time, `Ended` after play-out.
#[test]
fn audio_deselect_lets_video_run_to_ended() {
    let mut fixture = open_multi_track(vec![], true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 50, "prime");
    let audio_before = frames_snapshot(&fixture.collector)
        .iter()
        .filter(|(id, _)| id == &TrackId::new(2))
        .count();

    let request = RequestId(61);
    fixture
        .handle
        .set_audio_track(request, TrackSelector::None)
        .must();
    // The deselect completes immediately (nothing to wait for).
    fixture.engine.poll();
    let outputs = fixture.engine.take_outputs();
    let receipt = receipt_for(&outputs, request);
    assert!(matches!(receipt.result, ReceiptResult::Accepted { .. }));
    assert_eq!(
        selection_terms(&outputs),
        vec![(terms0(&outputs), MediaKind::Audio, None, Some(request))],
        "the deselect terminal fires for the empty selection"
    );

    // Drain the pipeline with the clock still frozen: the script,
    // queues and decoders all exhaust, yet the session must NOT end —
    // the deselected (video-only) session ends at play-out, not at
    // drain-completion wall time (the F5 shape was the defect).
    let total_packets = fixture
        .media
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .packets
        .len();
    for _ in 0..200 {
        fixture.engine.poll();
    }
    assert_eq!(
        media_cursor(&fixture),
        total_packets,
        "the frozen-clock window exhausted the script (drain really ran)"
    );
    assert_eq!(
        fixture.engine.state(),
        SessionState::Playing,
        "F5 regression: the video-only session ended at drain wall-time, \
         before the clock presented the last frame"
    );
    let mut outputs = fixture.engine.take_outputs();
    assert!(
        !outputs.iter().any(|output| matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::EndOfMedia,
                ..
            })
        )),
        "no end-of-media before play-out"
    );

    // Video runs on to Ended while the clock presents it.
    for _ in 0..400 {
        if fixture.engine.state() == SessionState::Ended {
            break;
        }
        fixture.engine.poll();
        // One 10 ms video-cadence frame's worth of wall per poll
        // (the fixture's video packets sit at 10 ms spacing).
        fixture.now.0.fetch_add(10_000_000, Ordering::Relaxed);
    }
    assert_eq!(
        fixture.engine.state(),
        SessionState::Ended,
        "drain: state is {} after 400 polls",
        fixture.engine.state()
    );
    outputs.extend(fixture.engine.take_outputs());
    assert!(
        !outputs.iter().any(|output| matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::EndOfTrack {
                    track_kind: MediaKind::Audio,
                    ..
                },
                ..
            })
        )),
        "a retired limb is not newly drained — no end-of-track for it"
    );
    let frames = frames_snapshot(&fixture.collector);
    let audio_after = frames
        .iter()
        .filter(|(id, _)| id == &TrackId::new(2))
        .count();
    assert_eq!(
        audio_after, audio_before,
        "the deselected kind delivered nothing new"
    );
    let video = frames
        .iter()
        .filter(|(id, _)| id == &TrackId::new(1))
        .count();
    assert!(video > 0, "video kept playing to the end");
    // Ended lands only after play-out: the clock covered the last
    // delivered video frame's pts (the F5 gate, held honestly).
    let last_video_pts_ns = frames
        .iter()
        .filter(|(id, _)| id == &TrackId::new(1))
        .filter_map(|(_, frame)| stamp_ns(&frame.pts()))
        .max()
        .must();
    let played_out_ns = match media_clock::PlaybackClock::now(&*fixture.clock).as_nanos() {
        Ok(Nanoseconds(ns)) => ns as i64,
        Err(_) => 0,
    };
    assert!(
        played_out_ns >= last_video_pts_ns,
        "end-of-media must not precede play-out (clock {played_out_ns} ns \
         < last video pts {last_video_pts_ns} ns)"
    );
}

fn terms0(outputs: &[EngineOutput]) -> Generation {
    selection_terms(outputs)[0].0
}

// ---------------------------------------------------------------------------
// 5. not-found: the pre-table rejection asymmetry.
// ---------------------------------------------------------------------------

/// Case 5: an unknown id (or a wrong-kind id) rejects typed
/// `TrackNotFound` **before** the table — no generation bump, no
/// `state-changed`, `Rejected` receipt (the documented asymmetry
/// with seek's resolution failures: the menu checkmark must not wait
/// on a dead id, and rapid UI cycling stays cheap).
#[test]
fn not_found_rejects_before_the_mint_with_no_state_change() {
    let mut fixture = open_multi_track(vec![], true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 50, "prime");
    // A clean baseline: the rejections below must not read the
    // prime's transitions as command-caused state changes.
    let _ = fixture.engine.take_outputs();
    let generation_before = fixture.engine.generation().must();
    let frames_before = frames_snapshot(&fixture.collector).len();

    // An unknown id.
    let request = RequestId(71);
    fixture
        .handle
        .set_audio_track(
            request,
            TrackSelector::Id {
                id: TrackId::new(99),
            },
        )
        .must();
    // A wrong-kind id (the video track, asked of the audio
    // selection).
    let request_wrong_kind = RequestId(72);
    fixture
        .handle
        .set_audio_track(
            request_wrong_kind,
            TrackSelector::Id {
                id: TrackId::new(1),
            },
        )
        .must();
    fixture.engine.poll();
    let outputs = fixture.engine.take_outputs();

    for request in [request, request_wrong_kind] {
        let receipt = receipt_for(&outputs, request);
        assert!(
            matches!(
                receipt.result,
                ReceiptResult::Rejected {
                    reason: RejectionReason::TrackNotFound,
                    ..
                }
            ),
            "{request:?}: the dead selector refuses typed"
        );
    }
    assert_eq!(
        fixture.engine.generation().must(),
        generation_before,
        "a rejected selector mints nothing"
    );
    assert!(
        !outputs.iter().any(|output| matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::StateChanged { .. },
                ..
            })
        )),
        "a rejected selector changes no state"
    );
    assert!(selection_terms(&outputs).is_empty());
    // And the pipeline kept playing untouched.
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Playing);
    assert!(
        frames_snapshot(&fixture.collector).len() > frames_before,
        "playback never noticed the dead selectors"
    );
}

// ---------------------------------------------------------------------------
// 6. Rapid double switch: supersession silence.
// ---------------------------------------------------------------------------

/// Case 6: a second switch of the same kind accepted before the
/// first completes replaces the in-flight slot — the older switch is
/// **silent** (no terminal for its generation, doc 05 §4.2), exactly
/// one terminal fires (for the newest selection), and no
/// intermediate-track frames leak to the sink.
#[test]
fn rapid_double_switch_emits_one_terminal_for_the_newest_selection() {
    let mut fixture = open_multi_track(vec![], true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 50, "prime");

    // Both commands queue before any poll: the first poll applies
    // them in order (mint G1 + swap to B, then mint G2 + swap back
    // to A) and one tick decodes — only the second selection owns a
    // limb when the decode runs.
    let request_one = RequestId(81);
    fixture
        .handle
        .set_audio_track(
            request_one,
            TrackSelector::Id {
                id: TrackId::new(3),
            },
        )
        .must();
    let request_two = RequestId(82);
    fixture
        .handle
        .set_audio_track(
            request_two,
            TrackSelector::Id {
                id: TrackId::new(2),
            },
        )
        .must();
    fixture.engine.poll();

    let mut outputs = Vec::new();
    for _ in 0..10 {
        outputs = fixture.engine.take_outputs();
        if outputs.iter().any(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::TrackSelectionChanged { .. },
                    ..
                })
            )
        }) {
            break;
        }
        fixture.engine.poll();
    }
    let terms = selection_terms(&outputs);
    assert_eq!(
        terms.len(),
        1,
        "exactly one terminal — the superseded switch is silent"
    );
    assert_eq!(
        terms[0].2,
        Some(TrackId::new(2)),
        "the newest selection won"
    );
    assert_eq!(
        terms[0].3,
        Some(request_two),
        "the newest request is the origin"
    );
    // The superseded generation has no terminal: the surviving one
    // is the second mint (the receipts' Accepted generations prove
    // the two mints happened).
    let receipt_one = receipt_for(&outputs, request_one);
    let receipt_two = receipt_for(&outputs, request_two);
    let gen_one = match receipt_one.result {
        ReceiptResult::Accepted { generation, .. } => generation,
        other => unreachable!("{other:?}"),
    };
    let gen_two = match receipt_two.result {
        ReceiptResult::Accepted { generation, .. } => generation,
        other => unreachable!("{other:?}"),
    };
    assert!(gen_two.0 > gen_one.0, "both mints happened");
    assert_eq!(
        terms[0].0, gen_two,
        "the terminal belongs to the newest mint"
    );
    // No intermediate-track frames at the sink: track 3 (B) never
    // delivered.
    assert!(
        !frames_snapshot(&fixture.collector)
            .iter()
            .any(|(id, _)| id == &TrackId::new(3)),
        "the superseded selection never presented"
    );
}

// ---------------------------------------------------------------------------
// 7. Switch while Paused.
// ---------------------------------------------------------------------------

/// Case 7: a switch accepted while `Paused` completes with the wire
/// state never leaving `paused` — the decode carve-out runs the
/// landing frame (evidence, never a pre-completion presentation
/// leak: the frames deliver through the sink seam, the wire state is
/// the truth the host sees).
#[test]
fn switch_while_paused_completes_without_leaving_paused() {
    let mut fixture = open_multi_track(vec![], true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 50, "prime");
    fixture.handle.pause(RequestId(90)).must();
    fixture.engine.poll();
    assert_eq!(fixture.engine.state(), SessionState::Paused);

    let request = RequestId(91);
    fixture
        .handle
        .set_audio_track(
            request,
            TrackSelector::Id {
                id: TrackId::new(3),
            },
        )
        .must();
    let mut outputs = Vec::new();
    for _ in 0..20 {
        fixture.engine.poll();
        outputs = fixture.engine.take_outputs();
        if outputs.iter().any(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::TrackSelectionChanged { .. },
                    ..
                })
            )
        }) {
            break;
        }
    }
    assert_eq!(
        fixture.engine.state(),
        SessionState::Paused,
        "the wire state never leaves paused"
    );
    let terms = selection_terms(&outputs);
    assert_eq!(
        terms,
        vec![(
            terms[0].0,
            MediaKind::Audio,
            Some(TrackId::new(3)),
            Some(request)
        )],
        "the paused switch completes"
    );
    // The landing frame exists (the carve-out decoded it) — the
    // completion is on evidence, not on a timer.
    assert!(
        frames_snapshot(&fixture.collector)
            .iter()
            .any(|(id, _)| id == &TrackId::new(3)),
        "the carve-out produced the landing frame"
    );
    // No state-changed fired anywhere in the paused switch's
    // lifetime (the confirmation is the command's receipt's sibling,
    // not an episode change).
    assert!(
        !outputs.iter().any(|output| matches!(
            output,
            EngineOutput::Event(EngineEvent {
                kind: EngineEventKind::StateChanged { .. },
                ..
            })
        )),
        "the paused switch changes no state"
    );
}

// ---------------------------------------------------------------------------
// 8. Interleavings: switch × seek.
// ---------------------------------------------------------------------------

/// Case 8a — a switch arriving mid-seek: the limb swap re-arms the
/// seek's window for the swapped limb; both terminals fire exactly
/// once each, under their own generations (the drain-exemption
/// proof), and the first post-seek audio frame belongs to the
/// post-switch selection.
#[test]
fn switch_during_seek_completes_both_flows_under_their_own_generations() {
    let mut fixture = open_multi_track(vec![], true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 50, "prime");

    // The seek: back to the start (a mint + flush + re-anchor).
    let seek_request = RequestId(95);
    fixture
        .handle
        .seek(
            seek_request,
            playback_engine::SeekTarget::Absolute { position_ns: 0 },
        )
        .must();
    fixture.engine.poll(); // the seek applies; its pre-roll runs
    // The switch, mid-seek.
    let switch_request = RequestId(96);
    fixture
        .handle
        .set_audio_track(
            switch_request,
            TrackSelector::Id {
                id: TrackId::new(3),
            },
        )
        .must();

    let mut outputs = Vec::new();
    for _ in 0..30 {
        fixture.engine.poll();
        outputs = fixture.engine.take_outputs();
        let has_seek = outputs.iter().any(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::Seeked { .. },
                    ..
                })
            )
        });
        let has_switch = outputs.iter().any(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::TrackSelectionChanged { .. },
                    ..
                })
            )
        });
        if has_seek && has_switch {
            break;
        }
    }
    let seeked: Vec<Generation> = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Event(EngineEvent {
                generation,
                kind: EngineEventKind::Seeked { .. },
                ..
            }) => Some(*generation),
            _ => None,
        })
        .collect();
    assert_eq!(seeked.len(), 1, "exactly one seek terminal");
    let terms = selection_terms(&outputs);
    assert_eq!(terms.len(), 1, "exactly one switch terminal");
    let switch_receipt = receipt_for(&outputs, switch_request);
    let switch_gen = match switch_receipt.result {
        ReceiptResult::Accepted { generation, .. } => generation,
        other => unreachable!("{other:?}"),
    };
    assert_eq!(
        terms[0].0, switch_gen,
        "the switch terminal rides its own generation (not the seek's)"
    );
    assert_ne!(seeked[0], switch_gen, "the two mints are distinct");
    // The first post-seek audio frame belongs to the post-switch
    // selection (the seek completed on the new track's first frame).
    let frames = frames_snapshot(&fixture.collector);
    let post_seek_audio = frames
        .iter()
        .rev()
        .find(|(id, _)| id == &TrackId::new(3))
        .must();
    match &post_seek_audio.1 {
        Frame::Audio(audio) => assert_eq!(audio.format.rate, 44_100),
        other => unreachable!("{other:?}"),
    }
    assert_eq!(
        post_seek_audio.1.generation(),
        GenerationId::new(switch_gen.0),
        "the post-seek audio follows the switch generation"
    );
}

/// Case 8b — a seek arriving mid-switch: the seek's flush handles
/// the new limb too; both terminals fire exactly once, and the first
/// frame after the seek belongs to the switched-to track.
#[test]
fn seek_during_switch_completes_both_flows() {
    let mut fixture = open_multi_track(vec![], true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 50, "prime");

    let switch_request = RequestId(97);
    fixture
        .handle
        .set_audio_track(
            switch_request,
            TrackSelector::Id {
                id: TrackId::new(3),
            },
        )
        .must();
    fixture.engine.poll(); // the switch applies; its re-prime runs
    let seek_request = RequestId(98);
    fixture
        .handle
        .seek(
            seek_request,
            playback_engine::SeekTarget::Absolute { position_ns: 0 },
        )
        .must();

    let mut outputs = Vec::new();
    for _ in 0..30 {
        fixture.engine.poll();
        outputs = fixture.engine.take_outputs();
        let has_seek = outputs.iter().any(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::Seeked { .. },
                    ..
                })
            )
        });
        let has_switch = outputs.iter().any(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::TrackSelectionChanged { .. },
                    ..
                })
            )
        });
        if has_seek && has_switch {
            break;
        }
    }
    assert_eq!(
        outputs
            .iter()
            .filter(|output| matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::Seeked { .. },
                    ..
                })
            ))
            .count(),
        1,
        "exactly one seek terminal"
    );
    let terms = selection_terms(&outputs);
    assert_eq!(terms.len(), 1, "exactly one switch terminal");
    assert_eq!(terms[0].2, Some(TrackId::new(3)));
    let frames = frames_snapshot(&fixture.collector);
    let last_audio = frames
        .iter()
        .rev()
        .find(|(_, frame)| matches!(frame, Frame::Audio(_)))
        .must();
    assert_eq!(
        last_audio.0,
        TrackId::new(3),
        "the first post-seek audio belongs to the switched-to track"
    );
}

// ---------------------------------------------------------------------------
// 9. Decoder recycling: the per-session cache.
// ---------------------------------------------------------------------------

/// Case 9 (D-1): across an A→B→A audio cycle the provider builds
/// each track's decoder **exactly once** — the re-activation is
/// served by the per-session cache (`EngineStats::decoders_reused`),
/// never a fresh build (the multi-MB MJPEG `DecodeContext` survival
/// is the same machinery; the counting-allocator proof on the real
/// decoder lives in `tests/track_switch_alloc.rs`).
#[test]
fn decoder_cache_recycles_instances_across_switches() {
    let mut fixture = open_multi_track(vec![], true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 50, "prime");
    let builds_before = fixture.provider.total_builds();
    assert_eq!(
        fixture.provider.build_count(1),
        1,
        "video built once at wire"
    );
    assert_eq!(
        fixture.provider.build_count(2),
        1,
        "audio A built once at wire"
    );
    assert_eq!(fixture.provider.build_count(3), 0, "audio B not built yet");

    // A → B.
    fixture
        .handle
        .set_audio_track(
            RequestId(101),
            TrackSelector::Id {
                id: TrackId::new(3),
            },
        )
        .must();
    // B → A (before polling: both apply, only the second owns a limb
    // when the decode runs — the supersession shape; the CACHE proof
    // is the per-track build counts staying at one).
    fixture
        .handle
        .set_audio_track(
            RequestId(102),
            TrackSelector::Id {
                id: TrackId::new(2),
            },
        )
        .must();
    fixture.engine.poll();
    let mut outputs = Vec::new();
    for _ in 0..10 {
        outputs = fixture.engine.take_outputs();
        if !selection_terms(&outputs).is_empty() {
            break;
        }
        fixture.engine.poll();
    }
    assert_eq!(selection_terms(&outputs).len(), 1);

    // The cache proof: no fresh builds for either audio track — the
    // B limb came from the provider (its first activation) and the
    // A re-activation came from the cache.
    assert_eq!(
        fixture.provider.build_count(2),
        1,
        "audio A was never rebuilt (the cache served the re-activation)"
    );
    assert_eq!(
        fixture.provider.build_count(3),
        1,
        "audio B built exactly once"
    );
    assert_eq!(
        fixture.provider.build_count(1),
        1,
        "the untouched video limb never rebuilt"
    );
    assert_eq!(fixture.provider.total_builds(), builds_before + 1);
    // The engine's own accounting agrees: the A re-activation was a
    // cache hit.
    let stats = fixture.engine.stats();
    assert_eq!(stats.decoders_reused, 1, "one cache-served re-activation");
    assert_eq!(stats.decoders_built, builds_before as u64 + 1);
}

// ---------------------------------------------------------------------------
// 10. Queue drain: the retired kind's residents die, the new limb
//     fills from the demuxer's position.
// ---------------------------------------------------------------------------

/// Case 10 (folded into case 1's assertions): the switched kind's
/// fresh limb must fill from the demuxer's current position — its
/// first frame's pts is at or after the position the demuxer stood
/// at when the switch applied (no rewind, no replay: the demuxer
/// never moved). Pinned separately here because the drain facts are
/// the card's own AC row ("per-track queue drain").
#[test]
fn the_new_limb_fills_from_the_demuxers_position() {
    let mut fixture = open_multi_track(vec![], true);
    poll_until(&mut fixture, |s| s == SessionState::Playing, 50, "prime");
    // Let the queues fill past the high watermark so the retired
    // limb genuinely holds residents when the switch lands.
    for _ in 0..5 {
        fixture.engine.poll();
    }
    let cursor_at_switch = media_cursor(&fixture);
    let pts_at_switch = media_packet_at(&fixture, cursor_at_switch.saturating_sub(1))
        .map(|packet| stamp_ns(&packet.pts).unwrap_or(0))
        .unwrap_or(0);

    let request = RequestId(111);
    fixture
        .handle
        .set_audio_track(
            request,
            TrackSelector::Id {
                id: TrackId::new(3),
            },
        )
        .must();
    let mut outputs = Vec::new();
    for _ in 0..10 {
        fixture.engine.poll();
        outputs = fixture.engine.take_outputs();
        if !selection_terms(&outputs).is_empty() {
            break;
        }
    }
    assert_eq!(selection_terms(&outputs).len(), 1, "the switch completes");

    // The new track's first delivered frame is at or after the
    // demuxer position at switch time (no stale replay from before
    // the boundary).
    let first_new = frames_snapshot(&fixture.collector)
        .iter()
        .find(|(id, _)| id == &TrackId::new(3))
        .map(|(_, frame)| stamp_ns(&frame.pts()).unwrap_or(0))
        .must();
    assert!(
        first_new >= pts_at_switch,
        "the new limb filled forward from the demuxer's position \
         ({first_new} ns >= {pts_at_switch} ns)"
    );
    // And every audio frame past the terminal belongs to track 3
    // (the retired limb's residents never presented).
    let stale_old = frames_snapshot(&fixture.collector)
        .iter()
        .any(|(id, frame)| {
            id == &TrackId::new(2) && stamp_ns(&frame.pts()).unwrap_or(0) > pts_at_switch
        });
    assert!(
        !stale_old,
        "the retired track's post-boundary residents never presented"
    );
}
