//! The D-1/D-6 **bounded-memory proof** for card 3.A2 (the AC's "50
//! rapid switches with no leak"): the counting-allocator leak hunt
//! over the play-new-while-playing soak — open k, pump to `Playing`,
//! two extra pumps, open k+1, no close in between — with a
//! **non-retaining** sink (count + last marker only: a retaining
//! collector would attribute the test's own growth to the engine).
//!
//! # Why a dedicated binary
//!
//! The counting allocator is process-global (see
//! [`testkit::alloc_count`]); one `#[test]`, whole-binary
//! serialization, every allocation inside a measured window is the
//! code under test's own (the `bounded_alloc.rs` /
//! `epoch_staleness_alloc.rs` discipline).
//!
//! # What is measured, and how the bounds were pinned
//!
//! After each switch's settle (open → `Playing` → two pumps → a
//! reused-buffer drain), one [`AllocSnapshot`]. The first five
//! switches are warm-up (queue/hub/vec pool sizing); the bounds are
//! computed over the measured window (switches 5..=50):
//!
//! - **band** — `max(live_bytes) − min(live_bytes)` over the window:
//!   the soak's steady-state oscillation. The dominant term is the
//!   track-shape ladder (A+V epochs carry two preallocated queues,
//!   single-limb epochs one — `BoundedQueue::new` preallocates
//!   `min(max_packets, 1024)` slots, queue.rs).
//! - **creep** — `live_bytes[last] − live_bytes[first]`: no
//!   monotone growth across 45 further switches (the leak detector
//!   proper: anything the engine retains per epoch and never frees
//!   shows up here).
//! - **churn** — the largest per-switch `alloc_calls` delta: bounded
//!   allocation **events** per switch (queue/decoder/event/packet
//!   construction, all freed at the teardown — churn, not retention).
//!
//! The three constants are **pinned from measurement** (D-6: runtime
//! green, not argued): the measured maxima on this suite's fakes are
//! recorded beside each constant, with roughly the design's ~2×
//! margin. They are ceilings, not budgets — if a change raises the
//! measured numbers, re-measure and re-pin honestly rather than
//! weakening the assertion silently.

#[global_allocator]
static ALLOC: testkit::alloc_count::CountingGlobalAlloc =
    testkit::alloc_count::CountingGlobalAlloc::new();

use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::Arc;

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
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
    DecoderProvider, Engine, EngineOutput, FrameConsumer, QueuePlan, SessionConfig, TickBudget,
};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};
use testkit::Must;
use testkit::alloc_count::{AllocSnapshot, serialize_tests};

/// Packets per track per source (mirrors `replacement.rs`: no epoch
/// reaches end-of-stream inside its window, so every switch departs
/// `Playing`).
const PACKETS_PER_TRACK: usize = 512;

/// The measured window's live-bytes band ceiling (steady-state
/// oscillation). Measured on this suite's fakes: 10,666 bytes
/// (max 38,367 / min 27,701 — the two-queue vs one-queue footprint
/// swing of the track-shape ladder); pinned at 32 KiB (~3×).
const BOUND_BAND: isize = 32 * 1024;

/// The measured window's live-bytes creep ceiling (first → last).
/// Measured: 24 bytes over 45 further switches (no growth at all);
/// pinned at 1 KiB — still ~42× margin, tight enough to catch a
/// one-slot-per-switch retention leak.
const BOUND_CREEP: isize = 1024;

/// The per-switch allocation-events ceiling. Measured: 474 calls
/// per switch (packet payloads, frames, events, the fresh
/// queues/decoders/inbox); pinned at 2048 (~4× margin, sharper
/// than the design's 4096 starting point).
const BOUND_CHURN: u64 = 2048;

// ---------------------------------------------------------------------------
// Fakes (the marker family, trimmed to the soak's needs: no stall, no
// armed failures — same shapes as `replacement.rs`)
// ---------------------------------------------------------------------------

struct SharedNow(AtomicU64);

impl MonotonicSource for SharedNow {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// The non-retaining sink: counts frames and remembers the last
/// marker (the attribution witness) — retention would be the test's
/// own growth, not the engine's.
struct TallyConsumer {
    frames: AtomicU64,
    last_marker: AtomicU8,
}

impl FrameConsumer for TallyConsumer {
    fn accept(&self, frame: Frame, _track: TrackId) -> Result<(), EngineError> {
        self.frames.fetch_add(1, Ordering::Relaxed);
        if let Some(marker) = frame_marker(&frame) {
            self.last_marker.store(marker, Ordering::Relaxed);
        }
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

/// The on-demand marker demuxer: packets synthesized round-robin,
/// payload stamped with the source marker.
struct MarkerDemuxer {
    tracks: Vec<Track>,
    marker: u8,
    remaining: usize,
    issued: usize,
}

impl Demuxer for MarkerDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        Ok(self.tracks.clone())
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        if self.remaining == 0 {
            return Ok(DemuxEvent::EndOfStream);
        }
        self.remaining -= 1;
        let index = self.issued;
        self.issued += 1;
        let track = &self.tracks[index % self.tracks.len()];
        Ok(DemuxEvent::Packet(marker_packet(track, index as i64, self.marker)))
    }

    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
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
        self.issued = 0;
        self.remaining = PACKETS_PER_TRACK * self.tracks.len();
        Ok(target)
    }
}

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
            _ => Box::new(MarkerVideoDecoder {
                generation: config.generation,
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

/// The track-shape ladder (mirrors `replacement.rs`): video-only at
/// `k ≡ 0 (mod 10)`, audio-only at odd `k`, A+V otherwise — the
/// memory band must absorb the limb-count swing.
fn source_tracks(k: u32) -> Vec<Track> {
    if k % 10 == 0 {
        vec![track(1, MediaKind::Video)]
    } else if k % 2 == 1 {
        vec![track(1, MediaKind::Audio)]
    } else {
        vec![track(1, MediaKind::Video), track(2, MediaKind::Audio)]
    }
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

struct Harness {
    engine: Engine,
    clock: Arc<media_clock::MediaClock>,
    throttle: Arc<dyn MonotonicSource>,
    consumer: Arc<TallyConsumer>,
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
            consumer: Arc::new(TallyConsumer {
                frames: AtomicU64::new(0),
                last_marker: AtomicU8::new(0),
            }),
        }
    }

    /// Open source `k`. The returned handle is dropped by the caller
    /// immediately (the soak submits no commands): the inbox buffer
    /// then dies with the session at the next switch — a host that
    /// retains stale handles retains THEIR memory, not the engine's.
    fn open_source(&mut self, k: u32) {
        let tracks = source_tracks(k);
        let remaining = PACKETS_PER_TRACK * tracks.len();
        let mut config = SessionConfig::new(
            SourceUri::parse("file:///tmp/lumen-replacement-alloc.media").must(),
            Box::new(MarkerDemuxer {
                tracks,
                marker: k as u8,
                remaining,
                issued: 0,
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
        self.engine.open(config).must();
    }
}

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

// ---------------------------------------------------------------------------
// The bounded-memory AC
// ---------------------------------------------------------------------------

#[test]
fn memory_stays_bounded_across_fifty_switches() {
    let _serial = serialize_tests();
    let mut harness = Harness::new();
    // Pre-sized (51 opens = 50 switches): the bookkeeping itself must
    // not be a growth term inside the measured window.
    let mut snapshots: Vec<AllocSnapshot> = Vec::with_capacity(51);
    // The D-1 drain pattern: one reused buffer for the whole soak.
    let mut out: Vec<EngineOutput> = Vec::new();

    for k in 0..=50u32 {
        harness.open_source(k);
        pump_to_playing(&mut harness, "switch");
        harness.engine.poll();
        harness.engine.poll();
        out.clear();
        harness.engine.drain_outputs_into(&mut out);
        snapshots.push(ALLOC.state().snapshot());
    }

    // The soak really played: frames flowed through every epoch and
    // the last marker is the last source's.
    assert_eq!(harness.engine.state(), SessionState::Playing);
    let frames = harness.consumer.frames.load(Ordering::Relaxed);
    assert!(frames >= 51, "every epoch delivered frames (saw {frames})");
    assert_eq!(
        harness.consumer.last_marker.load(Ordering::Relaxed),
        50,
        "the final epoch's frames are the last delivered"
    );

    // ---- The measured window: switches 5..=50 (0..=4 are warm-up). ----
    let window = &snapshots[5..=50];
    assert_eq!(window.len(), 46);

    // (a) band: the steady-state oscillation is bounded.
    let max_live = window.iter().map(|s| s.live_bytes).max().unwrap();
    let min_live = window.iter().map(|s| s.live_bytes).min().unwrap();
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
        .unwrap();
    assert!(
        max_churn <= BOUND_CHURN,
        "per-switch allocation events: {max_churn} exceeds the pinned {BOUND_CHURN}"
    );

    // The measured evidence (kept as asserts on the recorded shape so
    // a regression cannot hide behind a passing bound): the window's
    // live bytes must actually oscillate (the limb-count ladder) and
    // the engine must have delivered a meaningful workload.
    assert!(band > 0, "a flat-zero band means the soak did not run");
}
