//! The W5R-03 source-swap suite (card 3.A2 relaunch): the
//! play-new-while-playing acceptance criteria on the **hardest shape
//! the card names** — rapid source switches interleaved with the
//! seek.rs generation machinery and latency-holding decoders, so that
//! *stale decoder output* is structurally in flight at every switch.
//!
//! # Relation to the merged suites
//!
//! The wave-3 `replacement.rs` / `replacement_alloc.rs` suites pin the
//! base flow (marker attribution, the close-plan composition, the
//! undrained hub, the plain soak's memory band). This suite is the
//! relaunch's harder re-proof, in the file the W5R-03 card owns:
//!
//! - **THE AC** (`fifty_rapid_switches_with_in_flight_seeks`): 50
//!   switches where every window also drives a seek through
//!   [`SeekTarget`](playback_engine::SeekTarget) — half the seeks
//!   complete (the seven-step flow end-to-end), half are cut off
//!   mid-flight by the next `open` (doc 05 §4.2 supersession). The
//!   decoders hold **one undelivered frame at all times** (a
//!   one-packet latency — the reorder-buffer shape), so every switch
//!   and every seek flush has stale decoder output sitting inside a
//!   decoder when the generation moves. Proven per switch: the epoch
//!   bump (fresh session id, dead old handle), zero stale frames
//!   (inline marker check at sink-accept time + a per-poll
//!   generation check — every delivered frame carries the pipeline
//!   generation the poll ran under, and commands mint **before**
//!   ticks, so a delivered frame of any other generation is a bug),
//!   the seek flush draining exactly the latency-held frame per limb
//!   (counted as stale, never delivered), the superseded seek's
//!   pre-roll hold discarded and counted, and the suppressed-event
//!   arithmetic of the dying epoch.
//! - **the zero-poll storm** (50 switches with no poll at all — the
//!   fastest replacement a host can issue): every switch departs
//!   `Opening` with an unwired pipeline; nothing plays, nothing
//!   leaks, exactly one event survives the final drain, and the last
//!   source still plays.
//! - **teardown ordering** (the journal): at a switch the observable
//!   order is mint (the old token answers stale for its own pair) →
//!   the old epoch's decoders drop → the old demuxer drops → the new
//!   demuxer starts; teardown never *solicits* decoder output (no
//!   `flush` calls — the drain is drop-by-construction, which is why
//!   a held stale frame cannot reach the sink).
//! - **bounded memory** (the testkit counting allocator): the same
//!   seek-interleaved 50-switch soak under
//!   [`testkit::alloc_count`], with the live-bytes band / creep and
//!   the per-switch allocation churn pinned from measurement (D-6 —
//!   the pinned constants carry the measured values beside them).
//!
//! # Why every test serializes
//!
//! The counting allocator is process-global
//! ([`testkit::alloc_count`]); every test in this binary takes
//! [`serialize_tests`] first and holds it for its whole body, so no
//! sibling test thread can allocate inside a measured window (the
//! `epoch_staleness_alloc.rs` discipline, generalized to a
//! multi-test binary — the non-measuring tests are unaffected, the
//! allocator forwards to `System` and only counts).

#[global_allocator]
static ALLOC: testkit::alloc_count::CountingGlobalAlloc =
    testkit::alloc_count::CountingGlobalAlloc::new();

use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
use commands::SessionId;
use commands::envelope::{Generation, RequestId};
use media_clock::MonotonicSource;
use media_types::frame::{AudioLayout, ColorDescription, FrameStorage, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::{
    AudioFormat, Duration, EngineError, Frame, GenerationId, MediaKind, Nanoseconds, Packet,
    PixelFormat, SourceUri, Stamp, TimeBase, TimeSpan, Timestamp, Track, TrackFlags, TrackId,
    TrackTiming,
};
use packet_queue::QueueLimits;
use playback_engine::state::SessionState;
use playback_engine::{
    DecoderProvider, Engine, EngineEventKind, EngineOutput, FrameConsumer, QueuePlan, SeekTarget,
    SessionConfig, SessionError, SessionHandle, TickBudget,
};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};
use testkit::Must;
use testkit::alloc_count::{AllocSnapshot, serialize_tests};

/// Packets per track per source: no epoch reaches end-of-stream inside
/// its window (every switch departs a live, producing pipeline — the
/// AC's premise), and every seek's re-anchor has packet depth to
/// re-prime from.
const PACKETS_PER_TRACK: usize = 512;

/// The seek target every window drives (30 ms into a 512 ms stream):
/// past the start, far from the end — a real mid-stream re-anchor.
const SEEK_TARGET_NS: i64 = 30_000_000;

/// The per-poll frame-record buffer bound: one poll's decode slice
/// delivers at most `packets_per_limb × limbs` frames (≤ 8 here); 256
/// is the pre-sized reuse capacity (D-1 — the sink's records buffer
/// never reallocates in steady state; a poll that somehow exceeded it
/// would show up as churn in the memory window).
const RECORD_CAPACITY: usize = 256;

// ---------------------------------------------------------------------------
// Fakes
// ---------------------------------------------------------------------------

/// Shared virtual monotonic time (frozen at 0): the session clock's
/// source and the event throttle both read it, so every emission
/// window is deterministic.
struct SharedNow(AtomicU64);

impl MonotonicSource for SharedNow {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// The teardown-ordering journal: `(source marker, event)` pairs in
/// observation order. Shared by the journaling demuxer/decoder fakes;
/// `None` in the soak tests (no journaling, no cost).
type Journal = Arc<Mutex<Vec<(u8, &'static str)>>>;

fn note(journal: &Option<Journal>, marker: u8, event: &'static str) {
    if let Some(journal) = journal {
        match journal.lock() {
            Ok(mut entries) => entries.push((marker, event)),
            Err(poisoned) => poisoned.into_inner().push((marker, event)),
        }
    }
}

/// The marker-carrying, seekable fake source: round-robin packets
/// whose payload stamps the source marker, `reanchor` rewinds the
/// issue cursor (the seek.rs step-4 contract: the demuxer re-positions
/// and reports the landing). The shared `starve_audio` flag makes the
/// audio limb's turns yield `Discontinuity` instead of packets — the
/// mid-seek shape where the video limb keeps delivering into the
/// pre-roll hold while the audio limb can never satisfy it, keeping a
/// seek genuinely in flight **with a non-empty hold**.
struct SwapDemuxer {
    tracks: Vec<Track>,
    marker: u8,
    remaining: usize,
    issued: usize,
    starve_audio: Arc<AtomicBool>,
    journal: Option<Journal>,
}

impl Demuxer for SwapDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        note(&self.journal, self.marker, "demux:start");
        Ok(self.tracks.clone())
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        let index = self.issued;
        self.issued += 1;
        let src_track = &self.tracks[index % self.tracks.len()];
        if src_track.kind == MediaKind::Audio && self.starve_audio.load(Ordering::Relaxed) {
            // The starved limb's turn: a marked discontinuity, no
            // packet — the round-robin still advances (the other limb
            // keeps flowing).
            return Ok(DemuxEvent::Discontinuity);
        }
        if self.remaining == 0 {
            return Ok(DemuxEvent::EndOfStream);
        }
        self.remaining -= 1;
        Ok(DemuxEvent::Packet(marker_packet(
            src_track,
            index as i64,
            self.marker,
        )))
    }

    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
        // The engine's seek flow uses `reanchor` (the doc 03 §6 step-4
        // seam); `seek` is the v0 refusal hook.
        Err(EngineError::new(
            media_types::PipelineStage::Demux,
            media_types::ErrorKind::Unsupported { format: None },
            media_types::error::ErrorContext::new("demux.seek.not-the-reanchor-path"),
        ))
    }

    fn seekability(&self) -> SeekSupport {
        SeekSupport::Fast { indexed: true }
    }

    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        note(&self.journal, self.marker, "demux:reanchor");
        self.issued = 0;
        self.remaining = PACKETS_PER_TRACK * self.tracks.len();
        Ok(target)
    }
}

impl Drop for SwapDemuxer {
    fn drop(&mut self) {
        note(&self.journal, self.marker, "demux:drop");
    }
}

/// The one-packet-latency decoder: `decode` emits the *previous*
/// packet's frame and holds the current one — the reorder-buffer
/// shape, so at every generation boundary exactly one undelivered
/// frame sits inside the decoder (the "stale decoder output" the card
/// wants drained). `flush` emits the held frame (the realistic drain
/// the seek path solicits; the source-replacement teardown never
/// calls it — the decoder dies with its held frame, which is the
/// drain-by-drop proof).
struct LatencyDecoder {
    kind: MediaKind,
    marker: u8,
    generation: GenerationId,
    held: Option<(Stamp, u8)>,
    journal: Option<Journal>,
}

impl Decoder for LatencyDecoder {
    fn configure(&mut self, config: &DecoderConfig) -> Result<(), EngineError> {
        self.generation = config.generation;
        note(&self.journal, self.marker, "decoder:configure");
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
        let marker = packet.data.as_slice()[0];
        let mut frames = 0;
        if let Some((pts, held_marker)) = self.held.take() {
            out.accept(latency_frame(self.kind, pts, self.generation, held_marker))?;
            frames += 1;
        }
        self.held = Some((packet.pts, marker));
        if frames > 0 {
            Ok(DecodeOutcome::Emitted { frames })
        } else {
            Ok(DecodeOutcome::NeedMoreInput)
        }
    }

    fn flush(&mut self, out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        note(&self.journal, self.marker, "decoder:flush");
        if let Some((pts, held_marker)) = self.held.take() {
            // Emitted under the generation the decoder was last
            // configured with — the caller's generation-checking sink
            // decides its fate (the seek path counts it stale and
            // drops it; nothing stale is ever delivered).
            out.accept(latency_frame(self.kind, pts, self.generation, held_marker))?;
        }
        Ok(DecodeOutcome::Drained)
    }
}

impl Drop for LatencyDecoder {
    fn drop(&mut self) {
        note(&self.journal, self.marker, "decoder:drop");
    }
}

struct SwapProvider {
    marker: u8,
    journal: Option<Journal>,
}

impl DecoderProvider for SwapProvider {
    fn decoder_for(
        &self,
        track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        note(&self.journal, self.marker, "decoder:built");
        Ok(Box::new(LatencyDecoder {
            kind: track.kind,
            marker: self.marker,
            generation: config.generation,
            held: None,
            journal: self.journal.as_ref().map(Arc::clone),
        }))
    }
}

/// One sink-side delivery, as recorded per poll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DeliveryRecord {
    /// The source marker the frame's content carries.
    marker: Option<u8>,
    /// The frame's pipeline generation (the epoch-local counter the
    /// engine minted it under).
    generation: u64,
    /// The frame kind (audio/video).
    kind: MediaKind,
}

/// The attribution sink: records every delivery for the per-poll
/// checks, checks the marker **inline** against the epoch the harness
/// declared live (a stale frame is counted the instant it arrives —
/// `stale_hits` is the no-stale-frames proof), and keeps the
/// cumulative per-kind counters. The records buffer is pre-sized and
/// cleared in place (D-1: no per-poll allocation).
struct AttributionSink {
    expected_marker: AtomicU8,
    stale_hits: AtomicU64,
    frames: AtomicU64,
    audio: AtomicU64,
    video: AtomicU64,
    records: Mutex<Vec<DeliveryRecord>>,
}

impl AttributionSink {
    fn new() -> AttributionSink {
        AttributionSink {
            expected_marker: AtomicU8::new(0),
            stale_hits: AtomicU64::new(0),
            frames: AtomicU64::new(0),
            audio: AtomicU64::new(0),
            video: AtomicU64::new(0),
            records: Mutex::new(Vec::with_capacity(RECORD_CAPACITY)),
        }
    }

    /// Inspect this poll's records (assert over them, count them),
    /// then clear in place — the capacity stays, the next poll reuses
    /// it (the drain-reuse pattern, D-1).
    fn inspect_and_clear<R>(&self, inspect: impl FnOnce(&[DeliveryRecord]) -> R) -> R {
        let mut records = match self.records.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let result = inspect(&records);
        records.clear();
        result
    }
}

impl FrameConsumer for AttributionSink {
    fn accept(&self, frame: Frame, _track: TrackId) -> Result<(), EngineError> {
        let marker = frame_marker(&frame);
        let expected = self.expected_marker.load(Ordering::Relaxed);
        if marker != Some(expected) {
            // The inline check: a frame whose content belongs to any
            // earlier source is stale the instant it arrives. Counted
            // here, asserted loudly at every window boundary.
            self.stale_hits.fetch_add(1, Ordering::Relaxed);
        }
        self.frames.fetch_add(1, Ordering::Relaxed);
        match frame {
            Frame::Audio(_) => {
                self.audio.fetch_add(1, Ordering::Relaxed);
            }
            Frame::Video(_) => {
                self.video.fetch_add(1, Ordering::Relaxed);
            }
            Frame::Subtitle(_) => {}
        }
        let record = DeliveryRecord {
            marker,
            generation: frame.generation().counter(),
            kind: match frame {
                Frame::Audio(_) => MediaKind::Audio,
                Frame::Video(_) => MediaKind::Video,
                Frame::Subtitle(_) => MediaKind::Subtitle,
            },
        };
        let mut records = match self.records.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        records.push(record);
        Ok(())
    }
}

fn frame_marker(frame: &Frame) -> Option<u8> {
    match frame {
        Frame::Audio(audio) => audio.samples.as_slice().first().copied(),
        Frame::Video(video) => match &video.data {
            FrameStorage::Cpu(bytes) => bytes.as_slice().first().copied(),
            FrameStorage::OpaqueHandle(_) => None,
        },
        Frame::Subtitle(_) => None,
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

fn track(id: u32, kind: MediaKind) -> Track {
    Track {
        id: TrackId::new(id),
        stream: media_types::StreamId::new(id),
        kind,
        language: None,
        codec: media_types::ids::CodecDescriptor::new("fake-latency-marker").must(),
        title: None,
        flags: TrackFlags::NONE,
        timing: TrackTiming {
            span: TimeSpan::UNKNOWN,
            base: base(),
            gaps: Vec::new(),
        },
    }
}

/// The track-shape ladder (the `replacement.rs` shape, parity
/// mirrored for this suite's cut-off windows): video-only at
/// `k ≡ 0 (mod 10)`, audio-only at even `k`, A+V at odd `k` — the soak
/// must absorb the limb-count swing, and every odd window (the ones
/// whose seek is cut off mid-flight) carries both limbs so the
/// starved-audio stall below leaves video flowing into the hold.
fn source_tracks(k: u32) -> Vec<Track> {
    if k.is_multiple_of(10) {
        vec![track(1, MediaKind::Video)]
    } else if k.is_multiple_of(2) {
        vec![track(1, MediaKind::Audio)]
    } else {
        vec![track(1, MediaKind::Video), track(2, MediaKind::Audio)]
    }
}

fn source_kinds(k: u32) -> Vec<MediaKind> {
    source_tracks(k).iter().map(|t| t.kind).collect()
}

fn marker_packet(src_track: &Track, pts_ms: i64, marker: u8) -> Packet {
    Packet {
        track: src_track.id,
        pts: at_millis(pts_ms),
        dts: Stamp::Missing,
        duration: Some(Duration::Known(Nanoseconds(1_000_000))),
        data: SharedBytes::from_vec(vec![marker; 16]),
        keyframe: true,
        discontinuity: false,
        generation: GenerationId::FIRST,
    }
}

fn latency_frame(kind: MediaKind, pts: Stamp, generation: GenerationId, marker: u8) -> Frame {
    match kind {
        MediaKind::Audio => Frame::Audio(media_types::frame::AudioFrame {
            format: AudioFormat {
                sample: SampleFormat::S16,
                channels: 2,
                layout: AudioLayout::Interleaved,
                rate: 48_000,
            },
            samples: SharedBytes::from_vec(vec![marker; 8]),
            pts,
            duration: None,
            generation,
        }),
        _ => Frame::Video(media_types::frame::VideoFrame {
            format: PixelFormat::Rgba8,
            size: (32 + u32::from(marker), 24),
            data: FrameStorage::Cpu(SharedBytes::from_vec(vec![marker; 8])),
            pts,
            color: ColorDescription::BT709_LIMITED,
            interlaced: false,
            generation,
        }),
    }
}

fn switch_queues() -> QueuePlan {
    let limits = QueueLimits {
        max_packets: NonZeroUsize::new(64).must(),
        max_bytes: NonZeroUsize::new(1 << 20).must(),
        max_duration: Nanoseconds(1_000_000_000),
    };
    QueuePlan::new(limits, limits)
}

fn switch_budget() -> TickBudget {
    TickBudget {
        demux_events: 32,
        packets_per_limb: 4,
        flush_calls_per_limb: 8,
    }
}

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// What one drain delivered (assertion-friendly facts).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DrainFacts {
    /// Delivered (non-suppressed) events.
    events: usize,
    /// `seek-completed` terminals among them.
    seeked: usize,
}

/// The shared harness: one engine over host-retained seams (clock,
/// sink, throttle — the borrowed `Arc`s that survive every epoch, R4),
/// one reused drain buffer (D-1), the inline marker expectation, the
/// mid-seek audio-starvation flag, and the teardown journal
/// (populated only by the journaling opens).
struct Harness {
    engine: Engine,
    clock: Arc<media_clock::MediaClock>,
    throttle: Arc<dyn MonotonicSource>,
    sink: Arc<AttributionSink>,
    starve_audio: Arc<AtomicBool>,
    journal: Journal,
    out: Vec<EngineOutput>,
    /// The session id the drain attribution expects — set at every
    /// open, so the checks hold whatever order markers are opened in.
    expected_session: SessionId,
}

impl Harness {
    fn new() -> Harness {
        Harness {
            engine: Engine::new(),
            clock: Arc::new(
                media_clock::MediaClock::system_monotonic(
                    Arc::new(SharedNow(AtomicU64::new(0))),
                    Timestamp::new(0, base()),
                    media_types::Rate::ONE,
                )
                .must(),
            ),
            throttle: Arc::new(SharedNow(AtomicU64::new(0))),
            sink: Arc::new(AttributionSink::new()),
            starve_audio: Arc::new(AtomicBool::new(false)),
            journal: Arc::new(Mutex::new(Vec::new())),
            out: Vec::new(),
            expected_session: SessionId(0),
        }
    }

    /// Open source `k` (the replacement path when a session is live).
    /// `journaling` wires the fakes to the teardown journal; the soak
    /// tests pass `false` (no journal cost inside memory windows).
    /// The audio-starvation flag resets here: the flag belongs to the
    /// *dying* demuxer's window — the fresh source flows freely (the
    /// `replacement.rs` stall discipline).
    fn open_source(&mut self, k: u32, journaling: bool) -> SessionHandle {
        self.starve_audio.store(false, Ordering::Relaxed);
        let tracks = source_tracks(k);
        let journal = journaling.then(|| Arc::clone(&self.journal));
        let remaining = PACKETS_PER_TRACK * tracks.len();
        let mut config = SessionConfig::new(
            SourceUri::parse("file:///tmp/lumen-source-swap-fake.media").must(),
            Box::new(SwapDemuxer {
                tracks,
                marker: k as u8,
                remaining,
                issued: 0,
                starve_audio: Arc::clone(&self.starve_audio),
                journal: journal.as_ref().map(Arc::clone),
            }),
            Arc::new(SwapProvider {
                marker: k as u8,
                journal,
            }),
            Arc::clone(&self.sink) as Arc<dyn FrameConsumer>,
            Arc::clone(&self.clock),
        );
        config.queues = switch_queues();
        config.budget = switch_budget();
        config.throttle = playback_engine::EventPolicy {
            tick: playback_engine::TimeTickPolicy::default_policy(),
            throttle: Arc::clone(&self.throttle),
        };
        let handle = self.engine.open(config).must();
        // The inline expectations flip with the epoch: any frame
        // delivered under the new epoch whose content is older is
        // stale on arrival (counted in `stale_hits`), and the drain
        // attribution expects the new session id.
        self.sink.expected_marker.store(k as u8, Ordering::Relaxed);
        self.expected_session = handle.session_id();
        handle
    }

    /// Drain the output hub into the reused buffer and check the
    /// session attribution of every delivered event: within the live
    /// epoch's window (after its `open` returned, before the next
    /// one) nothing of any superseded session may survive the drain
    /// (doc 05 §4.2, cross-epoch staleness — the W1-10 rule).
    fn drain_and_check_events(&mut self, context: &str) -> DrainFacts {
        let session = self.expected_session;
        self.out.clear();
        self.engine.drain_outputs_into(&mut self.out);
        let mut facts = DrainFacts {
            events: 0,
            seeked: 0,
        };
        for output in &self.out {
            let EngineOutput::Event(event) = output else {
                continue;
            };
            assert_eq!(
                event.session, session,
                "{context}: a stale event (seq {}) of session {:?} survived the drain",
                event.seq, event.session
            );
            facts.events += 1;
            if matches!(event.kind, EngineEventKind::Seeked { .. }) {
                facts.seeked += 1;
            }
        }
        facts
    }

    /// One poll plus the per-poll invariants:
    ///
    /// - every event delivered by this poll belongs to the live epoch;
    /// - every frame delivered by this poll carries the current
    ///   source's marker (content attribution) **and** the pipeline
    ///   generation the poll ran under. Commands mint before the tick
    ///   applies them and ticks never mint, so the generation read
    ///   *after* the poll is exactly the generation every frame it
    ///   delivered must carry — a frame of any other generation (an
    ///   older epoch's, a superseded seek's, a straggler decoder's)
    ///   fails here.
    fn poll_and_check(&mut self, k: u32) -> DrainFacts {
        self.engine.poll();
        let facts = self.drain_and_check_events("poll");
        let generation = self.engine.generation();
        let poll_frames = self.sink.inspect_and_clear(|records| {
            for record in records {
                assert_eq!(
                    record.marker,
                    Some(k as u8),
                    "a stale frame (marker {:?}, generation {}) reached the sink \
                     during source {k}'s window",
                    record.marker,
                    record.generation
                );
                assert_eq!(
                    Some(record.generation),
                    generation.map(|Generation(counter)| counter),
                    "source {k}: a delivered frame carries generation {} but the \
                     poll ran under {:?} — stale decoder output leaked",
                    record.generation,
                    generation
                );
            }
            records.len()
        });
        let _ = poll_frames;
        facts
    }

    /// Drive polls until the state is `Playing` (bounded — a hang is a
    /// failed test, doc 03 §4.6(f)).
    fn pump_to_playing(&mut self, k: u32, context: &str) {
        for _ in 0..80 {
            self.poll_and_check(k);
            if self.engine.state() == SessionState::Playing {
                return;
            }
        }
        assert_eq!(
            self.engine.state(),
            SessionState::Playing,
            "{context}: never reached Playing"
        );
    }

    /// Drive polls until the window's seek has reported its terminal
    /// (`Seeked`) and the state is back to `Playing` (the rebuffer
    /// after a completed seek re-primes through the watermark
    /// machinery — the seven-step flow's tail).
    fn pump_until_seeked_and_playing(&mut self, k: u32) {
        let mut saw_seeked = false;
        for _ in 0..80 {
            let facts = self.poll_and_check(k);
            saw_seeked |= facts.seeked > 0;
            if saw_seeked && self.engine.state() == SessionState::Playing {
                return;
            }
        }
        assert!(
            saw_seeked,
            "source {k}: the in-window seek never reported a terminal"
        );
        assert_eq!(
            self.engine.state(),
            SessionState::Playing,
            "source {k}: never returned to Playing after the seek"
        );
    }

    /// The journal entries appended since `from` (the teardown test's
    /// observation window).
    fn journal_since(&self, from: usize) -> Vec<(u8, &'static str)> {
        let entries = match self.journal.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        entries[from..].to_vec()
    }

    fn journal_len(&self) -> usize {
        match self.journal.lock() {
            Ok(guard) => guard.len(),
            Err(poisoned) => poisoned.into_inner().len(),
        }
    }
}

// ---------------------------------------------------------------------------
// Test 1 — THE AC: 50 rapid switches, every window seeking
// ---------------------------------------------------------------------------

/// The W5R-03 acceptance criterion, hardest shape: 51 opens = exactly
/// 50 play-new-while-playing switches, **every** window driving a seek
/// through the seek.rs machinery (card: "reuse seek.rs generation
/// machinery + epoch.rs"), with one-packet-latency decoders holding an
/// undelivered frame at every generation boundary.
///
/// Even `k`: the seek completes (terminal + return to `Playing`) —
/// the switch departs `Playing`. Odd `k`: the seek is cut off
/// mid-flight by the next `open` — the switch departs mid-episode
/// `Buffering` and the superseded seek owes no terminal (doc 05
/// §4.2); its pre-roll hold is discarded and counted (M10/D-4).
#[test]
fn fifty_rapid_switches_with_in_flight_seeks() {
    let _serial = serialize_tests();
    let mut harness = Harness::new();
    let mut previous: Option<SessionHandle> = None;
    let mut completed_seeks = 0u32;
    let mut superseded_seeks = 0u32;

    for k in 0..=50u32 {
        let limbs = source_tracks(k).len() as u64;
        let before = harness.engine.stats();
        let audio_before = harness.sink.audio.load(Ordering::Relaxed);
        let video_before = harness.sink.video.load(Ordering::Relaxed);

        let handle = harness.open_source(k, false);
        let session = SessionId(u64::from(k) + 1);

        // The epoch bump: a fresh session id every switch, and the
        // previous handle died with its epoch (the typed command-side
        // staleness rejection — never a silent drop).
        assert_eq!(handle.session_id(), session, "switch {k}: fresh epoch");
        if let Some(previous) = &previous {
            assert!(
                matches!(
                    previous.play(RequestId(u64::from(k) + 100)),
                    Err(SessionError::SessionClosed)
                ),
                "switch {k}: the replaced session's handle must be dead"
            );
        }

        // The switch's own drain: the dying epoch's undelivered trail
        // is suppressed (asserted by arithmetic below); everything
        // delivered belongs to the new epoch.
        harness.drain_and_check_events("switch");

        // The fresh epoch plays on the retained seams.
        harness.pump_to_playing(k, &format!("switch {k}"));

        // The window's seek (seek.rs machinery: mint, flush, re-anchor,
        // pre-roll, terminal).
        handle
            .seek(
                RequestId(u64::from(k) * 2 + 1_000),
                SeekTarget::Absolute {
                    position_ns: SEEK_TARGET_NS,
                },
            )
            .must();
        if k.is_multiple_of(2) {
            harness.pump_until_seeked_and_playing(k);
            let tail = harness.poll_and_check(k);
            assert_eq!(tail.seeked, 0, "switch {k}: a second terminal");
            completed_seeks += 1;
        } else {
            // Starve the audio limb, then exactly one poll: the seek
            // is accepted, minted, flushed and re-anchored, the video
            // limb's frames pour into the pre-roll hold — but the
            // audio limb can never deliver, so the seek is genuinely
            // in flight **with a non-empty hold** when the next open()
            // cuts it off.
            harness.starve_audio.store(true, Ordering::Relaxed);
            let facts = harness.poll_and_check(k);
            assert_eq!(facts.seeked, 0, "switch {k}: no terminal before the cut");
            assert_eq!(
                harness.engine.state(),
                SessionState::Buffering,
                "switch {k}: the odd window departs mid-seek Buffering"
            );
            superseded_seeks += 1;
        }

        let after = harness.engine.stats();

        // The dying epoch's suppressed trail. NOTE the window shift:
        // the events a switch pushes at `open(k)` are the death of
        // marker k-1's session (window k-1's departure state) — so
        // window k's delta observes the PREVIOUS window's exit: an
        // odd (cut-off) window departs mid-episode `Buffering`, so
        // the even window that replaces it suppresses bookend +
        // stopped + idle; an even (completed-seek) window departs
        // `Playing`, so the odd window after it suppresses the
        // stopped+idle pair (the close-plan macro's arithmetic — the
        // replacement suite's pin, re-proven under the
        // seek-interleaved shape).
        let suppressed = after.suppressed_stale_events - before.suppressed_stale_events;
        if k == 0 {
            assert_eq!(suppressed, 0, "the first open closes nothing");
        } else if k.is_multiple_of(2) {
            assert_eq!(
                suppressed,
                3,
                "switch {k}: the mid-seek Buffering departure (window {}) \
                 suppresses bookend + stopped + idle",
                k - 1
            );
        } else {
            assert_eq!(
                suppressed,
                2,
                "switch {k}: the Playing departure (window {}) suppresses \
                 stopped + idle",
                k - 1
            );
        }

        // The seek flush's stale decoder-output drain: the latency
        // decoder held exactly one frame per limb under the old
        // generation; the flush solicits it and the generation
        // check drops it, counted (doc 03 §5 checking point 5). One
        // seek per window ⇒ exactly `limbs` stale frames flushed.
        assert_eq!(
            after.stale_frames_dropped - before.stale_frames_dropped,
            limbs,
            "switch {k}: the seek flush must drain exactly the held frame per limb"
        );

        // The superseded seek's pre-roll hold: discarded and counted
        // (never presented). Same window shift as above — the discard
        // of window k-1's cut-off seek lands at `open(k)`: the even
        // windows observe their predecessor's discarded hold; a
        // completed seek's hold was released, not discarded (odd
        // windows observe nothing).
        let preroll = after.preroll_frames_discarded - before.preroll_frames_discarded;
        if k == 0 {
            assert_eq!(preroll, 0, "the first window discards nothing");
        } else if k.is_multiple_of(2) {
            assert!(
                preroll > 0,
                "switch {k}: window {}'s superseded seek held pre-roll \
                 frames that must be discarded and counted (saw {preroll})",
                k - 1
            );
        } else {
            assert_eq!(preroll, 0, "switch {k}: a completed seek discards nothing");
        }

        // The window delivered frames on every wired limb of source k.
        let audio = harness.sink.audio.load(Ordering::Relaxed) - audio_before;
        let video = harness.sink.video.load(Ordering::Relaxed) - video_before;
        for kind in source_kinds(k) {
            let count = match kind {
                MediaKind::Audio => audio,
                _ => video,
            };
            assert!(
                count >= 1,
                "switch {k}: limb {kind:?} delivered no frame (audio {audio}, video {video})"
            );
        }

        // No stale frame ever arrived (the inline sink check): the
        // strongest form of the no-stale-frames AC — zero, not
        // "eventually consistent".
        assert_eq!(
            harness.sink.stale_hits.load(Ordering::Relaxed),
            0,
            "switch {k}: stale frames reached the sink"
        );

        previous = Some(handle);
    }

    // The soak really played, both seek flavors ran, and the engine's
    // counter agrees with the sink across all 51 epochs.
    assert_eq!(
        completed_seeks, 26,
        "even windows 0..=50 completed their seeks"
    );
    assert_eq!(
        superseded_seeks, 25,
        "odd windows 1..=49 were cut off mid-seek"
    );
    assert_eq!(harness.engine.state(), SessionState::Playing);
    assert_eq!(
        harness.engine.stats().frames_delivered,
        harness.sink.frames.load(Ordering::Relaxed),
        "frames_delivered must equal the sink's total across epochs"
    );
    let stats = harness.engine.stats();
    assert_eq!(stats.outbox_dropped_events, 0);
    assert_eq!(stats.outbox_dropped_receipts, 0);
    assert_eq!(harness.sink.stale_hits.load(Ordering::Relaxed), 0);
}

// ---------------------------------------------------------------------------
// Test 2 — the zero-poll storm: the fastest 50 switches possible
// ---------------------------------------------------------------------------

/// 51 opens with **no poll in between** — every switch departs
/// `Opening` with an unwired pipeline (the close-plan's direct row:
/// one mint, `DestroySession`, one `→ idle` event). Nothing plays,
/// nothing leaks on the output path, and the whole storm leaves
/// exactly one deliverable event: the final epoch's `→ opening`.
#[test]
fn fifty_switches_with_zero_polls_between_opens() {
    let _serial = serialize_tests();
    let mut harness = Harness::new();
    let mut previous: Option<SessionHandle> = None;

    for k in 0..=50u32 {
        let handle = harness.open_source(k, false);
        assert_eq!(
            handle.session_id(),
            SessionId(u64::from(k) + 1),
            "storm {k}: the epoch keeps bumping without any poll"
        );
        if let Some(previous) = &previous {
            assert!(
                matches!(
                    previous.play(RequestId(u64::from(k) + 900)),
                    Err(SessionError::SessionClosed)
                ),
                "storm {k}: the replaced handle must be dead"
            );
        }
        // NO poll: the session stays `Opening` across the whole storm.
        assert_eq!(harness.engine.state(), SessionState::Opening);
        previous = Some(handle);
    }

    // Nothing ever played: zero frames, zero stale hits, zero
    // decoder-output drains of any kind.
    assert_eq!(harness.sink.frames.load(Ordering::Relaxed), 0);
    assert_eq!(harness.sink.stale_hits.load(Ordering::Relaxed), 0);

    // The final drain: 101 events were pushed (51 openings + 50
    // idles); cross-epoch suppression leaves exactly the final
    // epoch's `Idle → Opening`.
    let facts = harness.drain_and_check_events("storm tail");
    assert_eq!(facts.events, 1, "only the final epoch's opening survives");
    assert_eq!(facts.seeked, 0);

    let stats = harness.engine.stats();
    assert_eq!(
        stats.suppressed_stale_events, 100,
        "50 dying-epoch idles + 50 superseded openings suppressed"
    );
    assert_eq!(stats.frames_delivered, 0);
    assert_eq!(stats.stale_frames_dropped, 0);
    assert_eq!(stats.preroll_frames_discarded, 0);
    assert_eq!(stats.outbox_dropped_events, 0);
    assert_eq!(stats.outbox_dropped_receipts, 0);

    // The last source is a fully playable session: the storm left no
    // wedge behind.
    harness.pump_to_playing(50, "storm tail");
    let audio = harness.sink.audio.load(Ordering::Relaxed);
    let video = harness.sink.video.load(Ordering::Relaxed);
    assert!(audio + video >= 1, "the last source delivers frames");
    assert_eq!(harness.sink.stale_hits.load(Ordering::Relaxed), 0);
    assert_eq!(
        harness.engine.stats().frames_delivered,
        harness.sink.frames.load(Ordering::Relaxed)
    );
}

// ---------------------------------------------------------------------------
// Test 3 — teardown ordering + the stale decoder-output drain
// ---------------------------------------------------------------------------

/// The observable teardown order at one switch (the journal's fakes
/// record their lifecycle), plus the drain-by-drop proof:
///
/// - the dying epoch's **token** is interrupted (and answers stale
///   for its own last pair — the mint moved first);
/// - the dying epoch's **decoders drop before its demuxer drops**,
///   and both precede the new epoch's demuxer `start`;
/// - teardown **never solicits decoder output**: no `flush` call
///   appears for the dying epoch — its latency-held frame dies inside
///   the decoder (which is why stale decoder output cannot reach the
///   sink: nothing asks for it, and the decoder that holds it is
///   gone). The stats agree: zero stale-frame drops, zero pre-roll
///   discards at this switch.
#[test]
fn teardown_orders_token_decoders_demuxer_and_drains_by_drop() {
    let _serial = serialize_tests();
    let mut harness = Harness::new();

    // Source 3 = the A+V ladder shape (odd markers carry both
    // limbs): two wired limbs, two latency decoders holding one frame
    // each at the switch. Its replacement is source 4 (audio-only).
    let first = harness.open_source(3, true);
    assert_eq!(first.session_id(), SessionId(1));
    harness.pump_to_playing(3, "teardown prologue");
    // One extra poll: the decoders have decoded and each holds exactly
    // one undelivered frame right now.
    harness.poll_and_check(3);
    assert!(
        harness.sink.frames.load(Ordering::Relaxed) > 0,
        "the dying epoch delivered frames before the switch"
    );

    // The dying epoch's token, while it is still live.
    let old_token = harness.engine.cancel_token().must();
    let old_pair = old_token.snapshot();
    assert!(old_token.is_current(&old_pair));

    let before = harness.engine.stats();
    let journal_before = harness.journal_len();
    let frames_before = harness.sink.frames.load(Ordering::Relaxed);

    // The switch (journaling opens on both sides).
    let second = harness.open_source(4, true);
    assert_eq!(second.session_id(), SessionId(2));

    // The token: interrupted, and stale for the pair it published.
    assert!(
        old_token.interrupted(),
        "DestroySession interrupts the token"
    );
    assert!(
        old_token.is_stale(&old_pair),
        "the mint + interrupt make the old pair stale"
    );

    let after = harness.engine.stats();
    assert_eq!(
        after.stale_frames_dropped - before.stale_frames_dropped,
        0,
        "teardown solicits no decoder output — nothing was flushed, \
         nothing counted as a stale drop"
    );
    assert_eq!(
        after.preroll_frames_discarded - before.preroll_frames_discarded,
        0,
        "no seek was in flight — no pre-roll discard"
    );

    // The journal, part 1 — the switch window itself (before the new
    // epoch's first poll, so the new demuxer has not started yet):
    // the dying epoch's decoders dropped (both), before its demuxer
    // dropped; and no decoder flush was solicited anywhere in the
    // window.
    let entries = harness.journal_since(journal_before);
    let decoder_drops: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, (marker, event))| *marker == 3 && *event == "decoder:drop")
        .map(|(index, _)| index)
        .collect();
    let demuxer_drop = entries
        .iter()
        .position(|(marker, event)| *marker == 3 && *event == "demux:drop")
        .must();
    assert_eq!(
        decoder_drops.len(),
        2,
        "the A+V epoch's two decoders dropped at the switch"
    );
    for index in &decoder_drops {
        assert!(
            *index < demuxer_drop,
            "a decoder dropped at {index} but the demuxer at {demuxer_drop} — \
             limbs must die before the pipeline that owns them"
        );
    }
    assert!(
        !entries
            .iter()
            .any(|(marker, event)| *marker == 3 && *event == "decoder:flush"),
        "teardown must not solicit decoder output — the stale drain is \
         drop-by-construction (re-pin consciously if a flush-at-teardown \
         is ever added)"
    );

    // The new epoch plays; the dying epoch's held frame (one per
    // decoder, in flight at the switch) never arrives late.
    harness.pump_to_playing(4, "teardown epilogue");
    harness.poll_and_check(4);
    assert!(
        harness.sink.frames.load(Ordering::Relaxed) > frames_before,
        "the new epoch delivered frames"
    );
    assert_eq!(
        harness.sink.stale_hits.load(Ordering::Relaxed),
        0,
        "the latency-held frames of the dying epoch never leaked"
    );

    // The journal, part 2 — the new epoch's lifecycle entries all
    // landed AFTER the old epoch died: the new demuxer started and
    // the new decoder was built only after every old decoder dropped
    // (wire-time builds happen on the new epoch's first poll — well
    // after the switch; the explicit `configure` calls ride the seek
    // flushes, which this window has none of).
    let entries = harness.journal_since(journal_before);
    let new_start = entries
        .iter()
        .position(|(marker, event)| *marker == 4 && *event == "demux:start")
        .must();
    assert!(
        demuxer_drop < new_start,
        "the old demuxer (drop at {demuxer_drop}) must die before the new \
         one starts (at {new_start})"
    );
    let first_new_build = entries
        .iter()
        .position(|(marker, event)| *marker == 4 && *event == "decoder:built")
        .must();
    assert!(
        decoder_drops.iter().all(|index| *index < first_new_build),
        "the new decoder is built only after the old epoch's died"
    );
    assert_eq!(
        harness.engine.stats().frames_delivered,
        harness.sink.frames.load(Ordering::Relaxed),
        "engine totals track the sink across the switch"
    );
}

// ---------------------------------------------------------------------------
// Test 4 — bounded memory over the seek-interleaved 50-switch soak
// ---------------------------------------------------------------------------

/// The measured live-bytes band ceiling (steady-state oscillation).
/// Measured on this suite's fakes: 7,982 bytes (max 38,169 /
/// min 30,187 — the two-queue vs one-queue footprint swing of the
/// track-shape ladder plus the seek-interleave state); pinned at
/// 32 KiB (~4× margin, the `replacement_alloc.rs` ceiling).
const BOUND_BAND: isize = 32 * 1024;

/// The measured live-bytes creep ceiling (first → last of the
/// window). Measured: ≤ 0 over 45 further switches (the live bytes
/// end at or below where they began — no growth at all); pinned at
/// 1 KiB — still tight enough to catch a one-slot-per-switch
/// retention leak.
const BOUND_CREEP: isize = 1024;

/// The per-switch allocation-events ceiling. Measured: 407 calls per
/// switch (packets, frames, events, the fresh queues/decoders/inbox,
/// the seek flush's transients — all freed at the teardown); pinned
/// at 2048 (~5× margin).
const BOUND_CHURN: u64 = 2048;

/// The AC's no-leak half, on the hardest shape: the same
/// seek-interleaved 50-switch soak as the AC test, under the testkit
/// counting allocator — after each switch's settle one
/// [`AllocSnapshot`]; the first five switches are warm-up (queue/hub/
/// vec pool sizing); the bounds are pinned from measurement over the
/// measured window (switches 5..=50):
///
/// - **band** — `max(live_bytes) − min(live_bytes)`: the soak's
///   steady-state oscillation (dominated by the track-shape ladder's
///   queue footprint swing);
/// - **creep** — `live_bytes[last] − live_bytes[first]`: the leak
///   detector proper — anything the engine retains per epoch and
///   never frees shows up here;
/// - **churn** — the largest per-switch `alloc_calls` delta: bounded
///   allocation *events* per switch (all freed at the teardown —
///   churn, not retention).
///
/// The inline stale check runs throughout (the memory soak is also a
/// no-stale-frames proof: `stale_hits` is asserted zero at the end).
#[test]
fn memory_stays_bounded_across_the_seek_interleaved_switches() {
    let _serial = serialize_tests();
    let mut harness = Harness::new();
    // Pre-sized (51 opens = 50 switches): the bookkeeping itself must
    // not be a growth term inside the measured window.
    let mut snapshots: Vec<AllocSnapshot> = Vec::with_capacity(51);

    for k in 0..=50u32 {
        let handle = harness.open_source(k, false);
        harness.pump_to_playing(k, "memory soak");
        handle
            .seek(
                RequestId(u64::from(k) * 2 + 2_000),
                SeekTarget::Absolute {
                    position_ns: SEEK_TARGET_NS,
                },
            )
            .must();
        if k.is_multiple_of(2) {
            harness.pump_until_seeked_and_playing(k);
        } else {
            // The same cut-off shape as the AC test: starved audio
            // limb, one poll, a seek in flight with a non-empty hold
            // when the next open() lands.
            harness.starve_audio.store(true, Ordering::Relaxed);
            harness.poll_and_check(k);
        }
        snapshots.push(ALLOC.state().snapshot());
    }

    // The soak really played and no stale frame ever arrived.
    assert_eq!(harness.engine.state(), SessionState::Playing);
    assert!(
        harness.sink.frames.load(Ordering::Relaxed) >= 51,
        "every epoch delivered frames"
    );
    assert_eq!(
        harness.sink.stale_hits.load(Ordering::Relaxed),
        0,
        "no stale frame reached the sink during the memory soak"
    );
    assert_eq!(
        harness.engine.stats().frames_delivered,
        harness.sink.frames.load(Ordering::Relaxed)
    );

    // ---- The measured window: switches 5..=50 (0..=4 are warm-up). ----
    let window = &snapshots[5..=50];
    assert_eq!(window.len(), 46);

    // (a) band: the steady-state oscillation is bounded.
    let max_live = window.iter().map(|s| s.live_bytes).max().must();
    let min_live = window.iter().map(|s| s.live_bytes).min().must();
    let band = max_live - min_live;
    assert!(
        band <= BOUND_BAND,
        "live-bytes band over 46 switches: {band} bytes (max {max_live}, \
         min {min_live}) exceeds the pinned {BOUND_BAND}"
    );

    // (b) creep: no monotone growth across the window.
    let creep = window[window.len() - 1].live_bytes - window[0].live_bytes;
    assert!(
        creep <= BOUND_CREEP,
        "live-bytes creep over 46 switches: {creep} bytes (from {} to {}) \
         exceeds the pinned {BOUND_CREEP}",
        window[0].live_bytes,
        window[window.len() - 1].live_bytes
    );

    // (c) churn: bounded allocation events per switch.
    let max_churn = window
        .windows(2)
        .map(|pair| pair[1].alloc_calls - pair[0].alloc_calls)
        .max()
        .must();
    assert!(
        max_churn <= BOUND_CHURN,
        "per-switch allocation events: {max_churn} exceeds the pinned {BOUND_CHURN}"
    );

    // The measured evidence, kept as a shape assert so a regression
    // cannot hide behind a passing bound: the window's live bytes
    // must actually oscillate (the limb-count ladder).
    assert!(band > 0, "a flat-zero band means the soak did not run");
}
