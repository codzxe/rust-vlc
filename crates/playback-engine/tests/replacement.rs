//! The source-replacement acceptance suite (card 3.A2; doc 03 §4.6
//! `SetSource` row + invariants, §5 teardown, doc 05 §2.1/§3.3):
//! play-new-while-playing driven end-to-end through [`Engine::poll`]
//! with **marker-carrying fake sources** — every packet of source k
//! encodes k in its payload, and the fake decoders copy the marker
//! into every frame, so the sink can attribute each delivered frame
//! to its source **by content**, with no epoch plumbing (a frame's
//! bare `GenerationId` cannot carry the epoch — the pipeline
//! generation resets each epoch by design, doc 03 §5).
//!
//! What is proven here:
//!
//! - **THE AC** (`fifty_rapid_switches_playing_to_playing`): 50
//!   play-new-while-playing switches — open k, pump to `Playing`,
//!   two extra pumps, open k+1 with no close in between — with per
//!   switch: a fresh session id (the epoch bump), a live-old-handle
//!   typed death, zero delivered events of any superseded session
//!   (the W1-10 epoch-aware drain pin), the implicit close's pushed
//!   trail (exactly the two minted `state-changed → stopped` /
//!   `→ idle` events of the dying session, suppressed at drain —
//!   see the deviation note below), every delivered frame carrying
//!   marker k (**no stale frames**), ≥ 1 frame per wired limb, and
//!   the `frames_delivered` counter cross-checked against the sink;
//! - the **Buffering typed refusal** (the one state with no v1 wire
//!   edge out — 3.C1's) with the pipeline provably un-torn-down
//!   afterwards, and the **Opening replacement** (close-row direct,
//!   one minted trail event — the differential pin);
//! - the **in-flight seek dying silently** at a replacement (doc 05
//!   §4.2 supersession: a generation that never completed owes no
//!   terminal; the seek's *receipt* survives — request-correlated,
//!   never pipeline-scoped);
//! - **engine totals and `seq` surviving** the switch (R4 merges the
//!   dying session's totals before the drop; `seq` is engine-global
//!   strictly increasing across epochs);
//! - the **undrained hub** across six switches: receipts pinned,
//!   old-epoch events suppressed and counted, the live session's
//!   events all present, order preserved, nothing dropped;
//! - **failed open after a replacement**: the new epoch lands in
//!   `Error` with the typed fatal *before* `state-changed → error`,
//!   a plain `close` returns the engine to `Idle`, and a third open
//!   succeeds (no rollback — the old epoch is legally dead).
//!
//! # Deviation from the W1-16 design, documented (design §5 test 1)
//!
//! The design's test-1 wording expects the drained trail to include
//! the dying session's `playing→stopped→idle` half "stamped session
//! k". That was written before W1-10's final shape landed: the
//! merged epoch-aware drain (`DrainStaleness::is_stale_at_drain`,
//! events.rs) suppresses **every** cross-epoch event — `StateChanged`
//! included, seek terminals included — because D-4 forbids any
//! visible state crossing a session boundary and the stopped/idle
//! pair is *created by* the replacing `open` itself (no drain can
//! precede it; the W1-10 doc's "drain before opening" covers the
//! dying session's *playing-era* events, which per-poll hosts do
//! deliver). This suite therefore pins the pushed trail by its
//! **arithmetic**: exactly two old-epoch events are resident per
//! Playing-departure switch (`state-changed → stopped` + `→ idle`,
//! both minted generations of the dying session — one from the Stop
//! row, one from the Close row; the Opening departure pushes exactly
//! one), observable as the `suppressed_stale_events` delta, plus the
//! zero-delivered-events pin for everything older. Every *edge* the
//! composition performs remains a table row inside the doc 05 §3.3
//! conformance list, which is what P1 was chosen for.
//!
//! The bounded-memory half of the AC (counting-allocator leak proof)
//! lives in the dedicated binary `tests/replacement_alloc.rs` — the
//! allocator is process-global (the testkit discipline).

use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
use commands::envelope::{Generation, RequestId};
use commands::SessionId;
use media_clock::MonotonicSource;
use media_types::error::ErrorKind;
use media_types::frame::{AudioLayout, ColorDescription, FrameStorage, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::{
    AudioFormat, Duration, EngineError, Frame, GenerationId, MediaKind, Nanoseconds, Packet,
    PixelFormat, PipelineStage, SourceUri, Stamp, TimeBase, TimeSpan, Timestamp, Track, TrackFlags,
    TrackId, TrackTiming,
};
use packet_queue::QueueLimits;
use playback_engine::state::SessionState;
use playback_engine::{
    CommandReceipt, DecoderProvider, Engine, EngineEvent, EngineEventKind, EngineOutput,
    FrameConsumer, QueuePlan, SeekTarget, SessionCommandKind, SessionConfig, SessionError,
    SessionHandle, TickBudget,
};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};
use testkit::Must;

/// Packets per track per source: enough that no epoch reaches
/// end-of-stream inside its ~8-poll window (every switch departs from
/// `Playing` — the AC's premise), small enough that 51 epochs stay
/// fast.
const PACKETS_PER_TRACK: usize = 512;

// ---------------------------------------------------------------------------
// Fakes: the marker-carrying source family
// ---------------------------------------------------------------------------

/// Shared virtual monotonic time (doc 03 §8: no sleeps). Frozen at 0
/// for the session clock's source and the event throttle — every
/// emission window is deterministic.
struct SharedNow(AtomicU64);

impl MonotonicSource for SharedNow {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// The collecting sink: every frame the engine delivered, with its
/// track, in delivery order. The suite drains it per epoch window
/// (the window's frames are the no-stale-frames evidence; the
/// running total is the `frames_delivered` cross-check).
#[derive(Default)]
struct Collector {
    frames: Mutex<Vec<(TrackId, Frame)>>,
}

impl Collector {
    /// Take the delivered frames accumulated so far (capacity reused).
    fn take(&self) -> Vec<(TrackId, Frame)> {
        let mut frames = self
            .frames
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        std::mem::take(&mut *frames)
    }
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

/// The marker-carrying scripted demux stage: synthesizes packets on
/// demand (no pre-built script — the harness's own allocation
/// footprint stays constant across the switch loop), round-robin
/// over its tracks, each packet's payload stamped with the source
/// marker. An armed stall answers discontinuity markers (the honest
/// non-blocking "nothing new yet, stream not ended"); an armed
/// `fail_start` is the corrupt-source corner.
struct MarkerDemuxer {
    tracks: Vec<Track>,
    marker: u8,
    remaining: usize,
    issued: usize,
    stall: Arc<AtomicBool>,
    fail_start: Option<EngineError>,
}

impl Demuxer for MarkerDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        match self.fail_start.take() {
            Some(error) => Err(error),
            None => Ok(self.tracks.clone()),
        }
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        if self.stall.load(Ordering::Relaxed) {
            return Ok(DemuxEvent::Discontinuity);
        }
        if self.remaining == 0 {
            return Ok(DemuxEvent::EndOfStream);
        }
        self.remaining -= 1;
        let index = self.issued;
        self.issued += 1;
        let track = &self.tracks[index % self.tracks.len()];
        Ok(DemuxEvent::Packet(marker_packet(
            track,
            index as i64,
            self.marker,
        )))
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
        SeekSupport::Fast { indexed: true }
    }

    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        // Compute-then-commit (D4): every landing here succeeds and
        // restarts the stream from its head — the tests that seek get
        // superseded by a replacement before any post-seek packet
        // matters, so the simplest honest commit is a full restart.
        self.issued = 0;
        self.remaining = PACKETS_PER_TRACK * self.tracks.len();
        Ok(target)
    }
}

/// The audio decode stage: one frame per packet, the frame's PCM
/// bytes stamped with the packet's source marker (read from the
/// payload — the decoder is per-epoch fresh, the marker is content,
/// not plumbing).
struct MarkerAudioDecoder {
    generation: GenerationId,
}

impl Decoder for MarkerAudioDecoder {
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
        let marker = packet.data.as_slice()[0];
        out.accept(audio_frame(packet.pts, self.generation, marker))?;
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        Ok(DecodeOutcome::Drained)
    }
}

/// The video decode stage: one frame per packet; the frame's width
/// (`32 + marker`) and its pixel bytes both carry the source marker —
/// two independent content attributions per frame.
struct MarkerVideoDecoder {
    generation: GenerationId,
}

impl Decoder for MarkerVideoDecoder {
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
        let marker = packet.data.as_slice()[0];
        out.accept(video_frame(packet.pts, self.generation, marker))?;
        Ok(DecodeOutcome::Emitted { frames: 1 })
    }

    fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
        Ok(DecodeOutcome::Drained)
    }
}

/// The decoder seam: a marker decoder per wireable track kind.
struct MarkerProvider;

impl DecoderProvider for MarkerProvider {
    fn decoder_for(
        &self,
        track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        Ok(match track.kind {
            MediaKind::Audio => Box::new(MarkerAudioDecoder {
                generation: config.generation,
            }),
            MediaKind::Video => Box::new(MarkerVideoDecoder {
                generation: config.generation,
            }),
            // `Pipeline::wire` only asks for audio/video tracks; the
            // arm exists to keep the match total, surfaced typed.
            _ => {
                return Err(EngineError::new(
                    PipelineStage::Decode,
                    ErrorKind::Unsupported { format: None },
                    media_types::error::ErrorContext::new("decoder.no-marker-decoder-for-kind"),
                ));
            }
        })
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
        codec: media_types::ids::CodecDescriptor::new("fake-marker").must(),
        title: None,
        flags: TrackFlags::NONE,
        timing: TrackTiming {
            span: TimeSpan::UNKNOWN,
            base: base(),
            gaps: Vec::new(),
        },
    }
}

/// The source marker extracted from a delivered frame — the content
/// attribution the whole suite rests on (audio: first PCM byte;
/// video: first pixel byte, cross-checked against the width).
fn frame_marker(frame: &Frame) -> Option<u8> {
    match frame {
        Frame::Audio(audio) => audio.samples.as_slice().first().copied(),
        Frame::Video(video) => {
            let byte = match &video.data {
                FrameStorage::Cpu(bytes) => bytes.as_slice().first().copied(),
                FrameStorage::OpaqueHandle(_) => None,
            };
            debug_assert_eq!(byte.map(|m| 32 + m as u32), Some(video.size.0));
            byte
        }
        Frame::Subtitle(_) => None,
    }
}

/// The track-shape ladder (design §5): `k ≡ 0 (mod 10)` video-only,
/// odd `k` audio-only, otherwise A+V — the switches exercise
/// limb-count changes (2 ↔ 1) and kind flips across the epoch
/// boundary. (A zero-wired-limb source is a typed open failure —
/// `NoDecodableTrack` — pinned by the failed-open test instead; the
/// soak's every epoch must play.)
fn source_tracks(k: u32) -> Vec<Track> {
    if k % 10 == 0 {
        vec![track(1, MediaKind::Video)]
    } else if k % 2 == 1 {
        vec![track(1, MediaKind::Audio)]
    } else {
        vec![track(1, MediaKind::Video), track(2, MediaKind::Audio)]
    }
}

/// The track kinds source k wires (the per-window "≥ 1 frame per
/// wired limb" expectation).
fn source_kinds(k: u32) -> Vec<MediaKind> {
    source_tracks(k).iter().map(|track| track.kind).collect()
}

fn marker_packet(track: &Track, pts_ms: i64, marker: u8) -> Packet {
    Packet {
        track: track.id,
        pts: at_millis(pts_ms),
        dts: Stamp::Missing,
        duration: Some(Duration::Known(Nanoseconds(1_000_000))),
        data: SharedBytes::from_vec(vec![marker; 16]),
        keyframe: true,
        discontinuity: false,
        generation: GenerationId::FIRST,
    }
}

fn audio_frame(pts: Stamp, generation: GenerationId, marker: u8) -> Frame {
    Frame::Audio(media_types::frame::AudioFrame {
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
    })
}

fn video_frame(pts: Stamp, generation: GenerationId, marker: u8) -> Frame {
    Frame::Video(media_types::frame::VideoFrame {
        format: PixelFormat::Rgba8,
        size: (32 + u32::from(marker), 24),
        data: FrameStorage::Cpu(SharedBytes::from_vec(vec![marker; 8])),
        pts,
        color: ColorDescription::BT709_LIMITED,
        interlaced: false,
        generation,
    })
}

/// Queue caps sized for the switch loop: 64-packet caps put the high
/// watermark at 48 (reachable in a handful of 32-event demux ticks,
/// single- or two-limb) and the low at 16 (never re-approached once
/// primed — no underrun episodes: every switch departs `Playing`).
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

/// The one engine + the host-retained seams (the C8 retention shape:
/// clock, sink, throttle source are `Arc`s the engine merely
/// borrows; the producer stall is shared so a test can freeze the
/// *current* session's demuxer at will).
struct Harness {
    engine: Engine,
    clock: Arc<media_clock::MediaClock>,
    throttle: Arc<dyn MonotonicSource>,
    consumer: Arc<Collector>,
    stall: Arc<AtomicBool>,
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
            consumer: Arc::new(Collector::default()),
            stall: Arc::new(AtomicBool::new(false)),
        }
    }

    /// The config for source `k` (track shape per the ladder,
    /// marker `k`), wired to the harness's retained seams.
    fn source_config(&self, k: u32) -> SessionConfig {
        self.config_over(source_tracks(k), k as u8, None)
    }

    /// The corrupt-source corner: `Demuxer::start` fails typed.
    fn corrupt_config(&self) -> SessionConfig {
        self.config_over(
            vec![track(1, MediaKind::Audio)],
            99,
            Some(EngineError::new(
                PipelineStage::Demux,
                ErrorKind::Corrupt {
                    detail: media_types::error::CorruptDetail::InvalidStructure { offset: Some(0) },
                },
                media_types::error::ErrorContext::new("demux.start.corrupt-header"),
            )),
        )
    }

    fn config_over(
        &self,
        tracks: Vec<Track>,
        marker: u8,
        fail_start: Option<EngineError>,
    ) -> SessionConfig {
        let remaining = PACKETS_PER_TRACK * tracks.len();
        let mut config = SessionConfig::new(
            SourceUri::parse("file:///tmp/lumen-replacement-fake.media").must(),
            Box::new(MarkerDemuxer {
                tracks,
                marker,
                remaining,
                issued: 0,
                stall: Arc::clone(&self.stall),
                fail_start,
            }),
            Arc::new(MarkerProvider),
            Arc::clone(&self.consumer) as Arc<dyn FrameConsumer>,
            Arc::clone(&self.clock),
        );
        config.queues = switch_queues();
        config.budget = switch_budget();
        config.throttle = playback_engine::EventPolicy {
            tick: playback_engine::TimeTickPolicy::default_policy(),
            throttle: Arc::clone(&self.throttle),
        };
        config
    }
}

/// Open source `k` on the harness engine (the replacement path when
/// a session is live).
fn open_source(harness: &mut Harness, k: u32) -> SessionHandle {
    let config = harness.source_config(k);
    harness.engine.open(config).must()
}

/// Drive polls until the state is `Playing`, or fail loudly (a hang
/// is a failed test — doc 03 §4.6(f)). Always polls at least once
/// (the open's `Opening` step needs its poll).
fn pump_to_playing(harness: &mut Harness, context: &str) {
    for _ in 0..80 {
        harness.engine.poll();
        if harness.engine.state() == SessionState::Playing {
            return;
        }
    }
    assert_eq!(
        harness.engine.state(),
        SessionState::Playing,
        "{context}: never reached Playing"
    );
}

/// Two extra pumps past `Playing` — the delivery window's tail
/// (≥ 1 frame per wired limb flows here).
fn two_extra_pumps(harness: &mut Harness) {
    harness.engine.poll();
    harness.engine.poll();
}

/// The delivered events of one drain, as `(seq, session, kind)`
/// tuples (assertion-friendly without owning the payload).
fn event_facts(outputs: &[EngineOutput]) -> Vec<(u64, SessionId, &EngineEventKind)> {
    outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Receipt(_) => None,
            EngineOutput::Event(event) => Some((event.seq, event.session, &event.kind)),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Test 1 — THE AC: fifty rapid play-new-while-playing switches
// ---------------------------------------------------------------------------

#[test]
fn fifty_rapid_switches_playing_to_playing() {
    let mut harness = Harness::new();
    let mut total_frames: u64 = 0;
    let mut previous: Option<SessionHandle> = None;

    // 51 opens = exactly 50 replacements, every one departing from
    // `Playing` (a superset of the design's 0..50 loop — the card's
    // AC headline is "50 rapid switches").
    for k in 0..=50u32 {
        let stats_before = harness.engine.stats();
        let handle = open_source(&mut harness, k);
        let session = SessionId(u64::from(k) + 1);

        // R5/R6 — the epoch bump: a fresh session id every switch,
        // and the previous handle died with its epoch (the typed
        // command-side staleness rejection, never a silent drop).
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

        // R7/R8 — the fresh epoch plays on the retained seams.
        pump_to_playing(&mut harness, &format!("switch {k}"));
        two_extra_pumps(&mut harness);
        assert_eq!(
            harness.engine.state(),
            SessionState::Playing,
            "switch {k}: still Playing at the next switch (no underrun/ended drift)"
        );

        let outputs = harness.engine.take_outputs();
        let stats_after = harness.engine.stats();

        // R9 — the W1-10 pin: ZERO delivered pipeline-scoped events of
        // any superseded session after the switch (all v0 kinds are
        // pipeline-scoped).
        for (seq, event_session, _kind) in event_facts(&outputs) {
            assert_eq!(
                event_session, session,
                "switch {k}: stale event (seq {seq}) survived the drain"
            );
        }

        // The fresh epoch's wire trail: exactly opening → buffering →
        // playing, all under the new session.
        let trail: Vec<(SessionState, SessionState)> = outputs
            .iter()
            .filter_map(|output| match output {
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::StateChanged { from, to },
                    ..
                }) => Some((*from, *to)),
                _ => None,
            })
            .collect();
        assert_eq!(
            trail,
            vec![
                (SessionState::Idle, SessionState::Opening),
                (SessionState::Opening, SessionState::Buffering),
                (SessionState::Buffering, SessionState::Playing),
            ],
            "switch {k}: the new epoch's state trail"
        );

        // The implicit close's pushed trail (the P1 composition,
        // pinned by arithmetic — see the module docs' deviation
        // note): from `Playing` exactly TWO old-epoch events were
        // resident (`state-changed → stopped` from the Stop row,
        // `→ idle` from the Close row, both under minted generations
        // of the dying session) and were suppressed at drain.
        let suppressed = stats_after.suppressed_stale_events - stats_before.suppressed_stale_events;
        if k == 0 {
            assert_eq!(suppressed, 0, "the first open closes nothing");
        } else {
            assert_eq!(
                suppressed, 2,
                "switch {k}: exactly the stopped+idle pair of the dying session"
            );
        }

        // THE no-stale-frames proof: every frame delivered in epoch
        // k's window (after open_k returned, before open_{k+1})
        // carries marker k — no frame of any j < k ever appears
        // after switch k.
        let frames = harness.consumer.take();
        assert!(!frames.is_empty(), "switch {k}: no frames delivered");
        for (track, frame) in &frames {
            assert_eq!(
                frame_marker(frame),
                Some(k as u8),
                "switch {k}: a stale frame (track {}) reached the sink",
                track.handle()
            );
        }
        // Progress: ≥ 1 frame per wired limb of this source's shape.
        for kind in source_kinds(k) {
            let count = frames
                .iter()
                .filter(|(_, frame)| matches!(
                    (kind, frame),
                    (MediaKind::Audio, Frame::Audio(_)) | (MediaKind::Video, Frame::Video(_))
                ))
                .count();
            assert!(
                count >= 1,
                "switch {k}: limb {kind:?} delivered no frame ({count})"
            );
        }
        total_frames += frames.len() as u64;

        // Engine-level observability accumulates monotonically
        // across the switch (R4 merges before the drop).
        assert!(
            stats_after.frames_delivered >= stats_before.frames_delivered,
            "switch {k}: frames_delivered must not regress across a replacement"
        );

        previous = Some(handle);
    }

    // Final state and the counter cross-check: the engine counted
    // exactly what the sink received, across all 51 epochs.
    assert_eq!(harness.engine.state(), SessionState::Playing);
    assert_eq!(
        harness.engine.stats().frames_delivered,
        total_frames,
        "frames_delivered must equal the sink's total across epochs"
    );
    // Nothing was dropped or lost on the output path either.
    let stats = harness.engine.stats();
    assert_eq!(stats.outbox_dropped_events, 0);
    assert_eq!(stats.outbox_dropped_receipts, 0);
}

// ---------------------------------------------------------------------------
// Test 3 — the Buffering typed refusal and the Opening replacement
// ---------------------------------------------------------------------------

#[test]
fn replacement_refuses_typed_from_buffering_and_replaces_from_opening() {
    // ---- Part A: a live `Buffering` session refuses typed ----
    let mut harness = Harness::new();
    let handle = open_source(&mut harness, 0);
    harness.engine.poll(); // Opening → Buffering (wire + OpenReady)
    assert_eq!(harness.engine.state(), SessionState::Buffering);
    // Freeze the producer: the queue can never prime, so the session
    // stays genuinely mid-episode.
    harness.stall.store(true, Ordering::Relaxed);
    harness.engine.poll();
    harness.engine.poll();
    assert_eq!(harness.engine.state(), SessionState::Buffering);

    let stats_before = harness.engine.stats();
    assert!(matches!(
        harness.engine.open(harness.source_config(1)),
        Err(SessionError::SessionExists { session: SessionId(1) })
    ));
    // The refusal changed nothing: same state, same session, no
    // partial teardown (nothing pushed, nothing suppressed).
    assert_eq!(harness.engine.state(), SessionState::Buffering);
    assert_eq!(
        harness.engine.stats().suppressed_stale_events,
        stats_before.suppressed_stale_events
    );
    assert!(handle.play(RequestId(1)).is_ok(), "the handle still answers");
    // The proof the pipeline survived the refusal untouched: release
    // the stall and the SAME session primes and plays.
    harness.stall.store(false, Ordering::Relaxed);
    pump_to_playing(&mut harness, "recovered after refusal");
    two_extra_pumps(&mut harness);
    let frames = harness.consumer.take();
    assert!(!frames.is_empty());
    for (_, frame) in &frames {
        assert_eq!(frame_marker(frame), Some(0), "the refused-open session's frames");
    }

    // ---- Part B: a live `Opening` session replaces (close-row
    // direct — no Stop step, so exactly ONE minted trail event). ----
    let mut harness = Harness::new();
    let first = open_source(&mut harness, 0);
    assert_eq!(harness.engine.state(), SessionState::Opening);
    // Drain the first open's own announcement while its epoch is
    // live, so the switch window's arithmetic stays exact.
    let _ = harness.engine.take_outputs();
    let stats_before = harness.engine.stats();

    let second = open_source(&mut harness, 1);
    assert_eq!(second.session_id(), SessionId(2));
    assert!(matches!(
        first.play(RequestId(2)),
        Err(SessionError::SessionClosed)
    ));
    pump_to_playing(&mut harness, "opening replacement");
    two_extra_pumps(&mut harness);
    let outputs = harness.engine.take_outputs();
    let stats_after = harness.engine.stats();

    for (_, session, _) in event_facts(&outputs) {
        assert_eq!(session, SessionId(2), "opening replacement: stale event");
    }
    assert_eq!(
        stats_after.suppressed_stale_events - stats_before.suppressed_stale_events,
        1,
        "the Opening departure pushes exactly the dying session's \
         `state-changed → idle` (the Close row alone)"
    );
    let frames = harness.consumer.take();
    assert!(!frames.is_empty());
    for (_, frame) in &frames {
        assert_eq!(frame_marker(frame), Some(1));
    }
}

// ---------------------------------------------------------------------------
// Test 4 — an in-flight seek dies silently at the replacement
// ---------------------------------------------------------------------------

#[test]
fn in_flight_seek_dies_silently_at_replacement() {
    // Arm A — the design's failure corner (c): an in-flight seek
    // (slot occupied, pre-roll starved) dies silently at the
    // replacement. W1-02's merged pre-roll row made a
    // Playing-departed seek land in the *Buffering* episode (which
    // R1 refuses — Arm B below), so the Paused-departed seek is the
    // arm that reaches R2: the carve-out keeps the session in
    // `Paused`, which the Stop row closes.
    let mut harness = Harness::new();
    let handle = open_source(&mut harness, 0);
    pump_to_playing(&mut harness, "first source");
    handle.pause(RequestId(5)).must();
    harness.engine.poll(); // the pause applies (Paused + entry tick)
    assert_eq!(harness.engine.state(), SessionState::Paused);
    let _ = harness.engine.take_outputs(); // clean slate for the window
    let _ = harness.consumer.take(); // the pre-seek frames are epoch 1's
    let stats_before = harness.engine.stats();

    // Submit the seek, then freeze the producer BEFORE it can
    // complete: the seek applies from `Paused` (mint + flush +
    // reanchor; the wire state never leaves `Paused` — the carve-out
    // row), but the pre-roll can never see a new-generation packet —
    // it stays genuinely in flight.
    handle
        .seek(RequestId(7), SeekTarget::Absolute { position_ns: 5_000_000 })
        .must();
    harness.stall.store(true, Ordering::Relaxed);
    harness.engine.poll(); // the seek applies (Paused → Paused)
    harness.engine.poll();
    harness.engine.poll();
    assert_eq!(
        harness.engine.state(),
        SessionState::Paused,
        "the seek is still in flight (pre-roll starved)"
    );

    // Switch: R2 stops from `Paused`; the in-flight slot dies with
    // the teardown — silently (doc 05 §4.2: a generation that never
    // completed owes no terminal).
    let second = open_source(&mut harness, 1);
    assert_eq!(second.session_id(), SessionId(2));
    // The stall was the OLD demuxer's condition; the fresh source
    // flows freely (release before its first poll).
    harness.stall.store(false, Ordering::Relaxed);
    pump_to_playing(&mut harness, "second source");
    two_extra_pumps(&mut harness);
    let outputs = harness.engine.take_outputs();
    let stats_after = harness.engine.stats();

    // No seek terminal was EVER delivered — not for the old session,
    // not for anyone (the new epoch never seeks).
    for (_, session, kind) in event_facts(&outputs) {
        assert!(
            !matches!(
                kind,
                EngineEventKind::Seeked { .. } | EngineEventKind::SeekFailed { .. }
            ),
            "an in-flight seek must not complete across a replacement \
             (session {:?})",
            session
        );
    }
    // The seek's ACCEPT receipt did survive: receipts are
    // request-correlated, never pipeline-scoped — the host's
    // truth-timeout path keeps its trigger.
    let receipts: Vec<&CommandReceipt> = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Receipt(receipt) => Some(receipt),
            EngineOutput::Event(_) => None,
        })
        .collect();
    let seek_receipt = receipts
        .iter()
        .find(|receipt| receipt.request == RequestId(7))
        .expect("the seek's accept receipt survives the replacement");
    assert_eq!(seek_receipt.command, SessionCommandKind::Seek);
    assert_eq!(
        seek_receipt.result.accepted_generation(),
        Some(Generation(1)),
        "the class-A receipt carries the post-bump generation"
    );

    // The window's suppression arithmetic: the dying session's seek
    // confirmation (`Paused → Paused`, a C8 self-transition event),
    // `→ stopped` and `→ idle` (the implicit close) — three
    // old-epoch events, nothing else was resident.
    assert_eq!(
        stats_after.suppressed_stale_events - stats_before.suppressed_stale_events,
        3,
        "seek-confirm + stopped + idle of the dying session"
    );
    // And the new epoch plays marker-1 frames only.
    let frames = harness.consumer.take();
    assert!(!frames.is_empty());
    for (_, frame) in &frames {
        assert_eq!(frame_marker(frame), Some(1));
    }

    // Arm B — the W1-02 interaction the design could not know: a
    // Playing-departed seek opens the *seeking episode* (`Buffering`,
    // doc 03 §6 rebuffer shape), and a replacement attempt
    // mid-episode is the R1 typed refusal — the 3.C1 boundary
    // catches it, honestly and without touching the in-flight seek.
    let mut harness = Harness::new();
    let handle = open_source(&mut harness, 0);
    pump_to_playing(&mut harness, "arm B first source");
    handle
        .seek(RequestId(8), SeekTarget::Absolute { position_ns: 5_000_000 })
        .must();
    harness.stall.store(true, Ordering::Relaxed);
    harness.engine.poll(); // the seek applies: the rebuffer episode
    assert_eq!(harness.engine.state(), SessionState::Buffering);
    harness.engine.poll();
    assert_eq!(
        harness.engine.state(),
        SessionState::Buffering,
        "the seek is still in flight inside the episode"
    );
    assert!(matches!(
        harness.engine.open(harness.source_config(1)),
        Err(SessionError::SessionExists { session: SessionId(1) })
    ));
    // The refusal is honest, not destructive: release the stall and
    // the SAME seek completes normally (terminal + resume).
    harness.stall.store(false, Ordering::Relaxed);
    for _ in 0..80 {
        harness.engine.poll();
        if harness.engine.state() == SessionState::Playing {
            break;
        }
    }
    assert_eq!(harness.engine.state(), SessionState::Playing);
    let outputs = harness.engine.take_outputs();
    let seeked = event_facts(&outputs)
        .into_iter()
        .filter(|(_, session, kind)| {
            *session == SessionId(1) && matches!(kind, EngineEventKind::Seeked { .. })
        })
        .count();
    assert_eq!(
        seeked, 1,
        "the refused replacement must not disturb the in-flight seek's completion"
    );
}

// ---------------------------------------------------------------------------
// Test 5 — engine totals and seq survive the switch
// ---------------------------------------------------------------------------

#[test]
fn replacement_preserves_engine_totals_and_seq() {
    let mut harness = Harness::new();
    let mut seqs: Vec<u64> = Vec::new();

    // Epoch 1 (source 0, A+V).
    open_source(&mut harness, 0);
    pump_to_playing(&mut harness, "epoch 1");
    two_extra_pumps(&mut harness);
    let outputs_one = harness.engine.take_outputs();
    let frames_one = harness.consumer.take();
    let mut all_seqs: Vec<u64> = event_facts(&outputs_one)
        .into_iter()
        .map(|(seq, _, _)| seq)
        .collect();
    seqs.append(&mut all_seqs);
    assert!(!frames_one.is_empty());

    // The switch (source 2 — A+V again: same shape, clean totals
    // comparison).
    let mid = harness.engine.stats();
    open_source(&mut harness, 2);
    pump_to_playing(&mut harness, "epoch 2");
    two_extra_pumps(&mut harness);
    let outputs_two = harness.engine.take_outputs();
    let frames_two = harness.consumer.take();
    let after = harness.engine.stats();
    let mut all_seqs: Vec<u64> = event_facts(&outputs_two)
        .into_iter()
        .map(|(seq, _, _)| seq)
        .collect();
    seqs.append(&mut all_seqs);

    // `seq` is engine-global strictly increasing across the switch
    // (extends session_flow's reopen proof to the replacement path).
    for pair in seqs.windows(2) {
        assert!(pair[0] < pair[1], "engine-global seq must strictly increase");
    }
    // Totals: merged across the boundary, exactly the sink's count,
    // never regressing at the switch instant.
    assert!(after.frames_delivered >= mid.frames_delivered);
    assert_eq!(
        after.frames_delivered,
        (frames_one.len() + frames_two.len()) as u64,
        "frames_delivered accumulates across epochs (the R4 merge)"
    );
    // Content: window one is all marker 0, window two all marker 2 —
    // the totals belong to the right sources.
    for (_, frame) in &frames_one {
        assert_eq!(frame_marker(frame), Some(0));
    }
    for (_, frame) in &frames_two {
        assert_eq!(frame_marker(frame), Some(2));
    }
}

// ---------------------------------------------------------------------------
// Test 6 — the undrained hub across six switches
// ---------------------------------------------------------------------------

#[test]
fn undrained_hub_suppresses_old_epoch_events() {
    let mut harness = Harness::new();
    let mut request = 0u64;
    let mut reference_delivered = 0usize;
    let mut stats_mid = harness.engine.stats();

    // Windows 10..=15 are never drained (the 3.B4 shape: a host that
    // stalls draining) and receive pause/play commands whose receipts
    // must survive; window 16 is likewise left undrained in-loop so
    // the single post-loop drain sees the whole accumulated stretch
    // plus the live window. Epoch 2's window (A+V, drained) is the
    // reference for what one epoch delivers; `stats_mid` after epoch
    // 9's drain bounds the undrained stretch's suppression
    // arithmetic from below.
    for k in 0..=16u32 {
        let handle = open_source(&mut harness, k);
        pump_to_playing(&mut harness, "undrained loop");
        two_extra_pumps(&mut harness);
        // In the undrained stretch, submit pause/play pairs whose
        // RECEIPTS must survive the later drain (request-correlated,
        // never pipeline-scoped) even though their events belong to
        // dying sessions.
        if (10..=15).contains(&k) {
            request += 1;
            handle.pause(RequestId(request)).must();
            harness.engine.poll();
            request += 1;
            handle.play(RequestId(request)).must();
            harness.engine.poll();
        }
        // The window's frames belong to epoch k (the per-window
        // no-stale-frames pin, same as the soak).
        let frames = harness.consumer.take();
        assert!(!frames.is_empty(), "window {k} delivered no frames");
        for (_, frame) in &frames {
            assert_eq!(frame_marker(frame), Some(k as u8), "window {k}");
        }
        if (10..=16).contains(&k) {
            continue; // no drain — the hub accumulates
        }
        let outputs = harness.engine.take_outputs();
        if k == 2 {
            reference_delivered = event_facts(&outputs).len();
        }
        if k == 9 {
            stats_mid = harness.engine.stats();
        }
    }

    // The single drain at 16: six undrained epochs' worth of events
    // plus epoch 16's own window, against epoch 16's live pair.
    let outputs = harness.engine.take_outputs();
    let stats = harness.engine.stats();
    assert_eq!(harness.engine.state(), SessionState::Playing);

    // Every delivered event is the live session's (17 = epoch 16).
    let facts = event_facts(&outputs);
    assert!(!facts.is_empty());
    for (seq, session, _) in &facts {
        assert_eq!(
            *session, SessionId(17),
            "an undrained old-epoch event (seq {seq}) survived the drain"
        );
    }
    // The undrained stretch contributed ZERO delivered events: the
    // live window's delivered count equals the reference epoch's
    // (same track shape — both A+V — and the same pump flow).
    assert_eq!(
        facts.len(),
        reference_delivered,
        "epoch 16's delivered count must match the drained reference \
         (the six stalled windows delivered nothing)"
    );
    // All twelve receipts survived, in request order.
    let receipts: Vec<&CommandReceipt> = outputs
        .iter()
        .filter_map(|output| match output {
            EngineOutput::Receipt(receipt) => Some(receipt),
            EngineOutput::Event(_) => None,
        })
        .collect();
    assert_eq!(receipts.len(), 12, "receipts are pinned, never suppressed");
    for (index, receipt) in receipts.iter().enumerate() {
        assert_eq!(receipt.request, RequestId(index as u64 + 1));
    }
    // Order preserved: the delivered stream's event seqs are strictly
    // increasing (the hub is causal; suppression never reorders).
    let seqs: Vec<u64> = facts.iter().map(|(seq, _, _)| *seq).collect();
    for pair in seqs.windows(2) {
        assert!(pair[0] < pair[1], "delivery order preserved");
    }
    // The stalled epochs' events were suppressed and counted. The
    // exact total is policy-shaped (the pause-entry ticks follow the
    // hub's replace-resident-tick rule, counted separately), so the
    // lower bound pins what must hold for ANY policy: the six
    // implicit-close trail pairs (12 `state-changed` events, never
    // tick-replaceable) plus at least one window's worth of pipeline
    // events per stalled epoch.
    let suppressed_stretch = stats.suppressed_stale_events - stats_mid.suppressed_stale_events;
    assert!(
        suppressed_stretch >= 12 + reference_delivered as u64,
        "the undrained old-epoch events must be suppressed and counted \
         (saw {suppressed_stretch}; the six close trails alone are 12, \
         plus six windows' events)"
    );
    // Nothing overflowed: the hub held six epochs' worth under the
    // 1024-event cap, and no receipt was lost.
    assert_eq!(stats.outbox_dropped_events, 0);
    assert_eq!(stats.outbox_dropped_receipts, 0);
}

// ---------------------------------------------------------------------------
// Test 7 — a failed open after a replacement lands closeably in Error
// ---------------------------------------------------------------------------

#[test]
fn replacement_then_failed_open_lands_in_error_closeably() {
    let mut harness = Harness::new();
    open_source(&mut harness, 0);
    pump_to_playing(&mut harness, "first source");
    let _ = harness.engine.take_outputs();
    let first_frames = harness.consumer.take();
    assert!(!first_frames.is_empty());
    for (_, frame) in &first_frames {
        assert_eq!(frame_marker(frame), Some(0));
    }
    let stats_before = harness.engine.stats();

    // Switch to a corrupt source: the replacement itself succeeds
    // (the old epoch is legally dead — no rollback), and the NEW
    // epoch's open step fails typed.
    let second = harness.engine.open(harness.corrupt_config()).must();
    assert_eq!(second.session_id(), SessionId(2));
    harness.engine.poll(); // open_step: start() fails → Failed → Error
    assert_eq!(harness.engine.state(), SessionState::Error);
    // The failing epoch delivered nothing to the sink.
    assert!(
        harness.consumer.take().is_empty(),
        "a session that failed to open must not deliver frames"
    );

    let outputs = harness.engine.take_outputs();
    let stats_after = harness.engine.stats();
    // The typed fatal precedes `state-changed → error` (the row's
    // ReportFatal effect runs before the confirmation emission), and
    // both belong to the failing epoch — the dead session is silent.
    let mut fatal_at = None;
    let mut error_at = None;
    for (index, (_, session, kind)) in event_facts(&outputs).iter().enumerate() {
        assert_eq!(*session, SessionId(2), "failed open: stale event");
        if matches!(kind, EngineEventKind::Fatal { .. }) {
            fatal_at = Some(index);
        }
        if matches!(
            kind,
            EngineEventKind::StateChanged {
                to: SessionState::Error,
                ..
            }
        ) {
            error_at = Some(index);
        }
    }
    let (fatal_at, error_at) = (
        fatal_at.expect("the typed fatal was emitted"),
        error_at.expect("state-changed → error was emitted"),
    );
    assert!(fatal_at < error_at, "the fatal precedes the state change");
    // The implicit close of the first epoch was suppressed as usual.
    assert_eq!(
        stats_after.suppressed_stale_events - stats_before.suppressed_stale_events,
        2,
        "the playing→stopped→idle pair of the replaced epoch"
    );

    // A plain close returns the engine to Idle (the host's
    // teardown-after-fatal shape, engine-proven).
    second.close(RequestId(9)).must();
    harness.engine.poll();
    assert_eq!(harness.engine.state(), SessionState::Idle);

    // A third open succeeds on the same engine and seams.
    let third = open_source(&mut harness, 2);
    assert_eq!(third.session_id(), SessionId(3));
    pump_to_playing(&mut harness, "third source");
    two_extra_pumps(&mut harness);
    let frames = harness.consumer.take();
    assert!(!frames.is_empty());
    for (_, frame) in &frames {
        assert_eq!(frame_marker(frame), Some(2));
    }
}
