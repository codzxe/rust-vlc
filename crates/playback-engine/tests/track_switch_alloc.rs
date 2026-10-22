//! The D-1/D-6 **counting-allocator proof** for card 3.A3's decoder
//! cache (design W1-17 S3/S7 case 9): switching tracks must not
//! re-allocate decoder working memory — the per-session cache parks
//! decoder **instances**, so the multi-MB decode context (MJPEG's
//! padded planes + Huffman LUTs, the card's motivating case) survives
//! every switch and only the cheap per-limb state (queue, bookkeeping)
//! is rebuilt.
//!
//! # Why a dedicated binary
//!
//! The counting allocator is process-global (see
//! [`testkit::alloc_count`]); one `#[test]`, whole-binary
//! serialization, every allocation inside a measured window is the
//! code under test's own (the `bounded_alloc.rs` /
//! `replacement_alloc.rs` discipline).
//!
//! # The detector, and why it is sharp
//!
//! The fake decoders allocate a **256 KiB context block at
//! construction and nothing ever after** (`configure` sets the
//! generation; `decode` emits static-payload frames — zero per-packet
//! allocation). A soak of 41 alternating audio switches (A→B→A→…)
//! then has exactly two possible shapes:
//!
//! - **cache works**: three decoder builds total (video + A at wire,
//!   B at the first switch), and every later switch's allocation
//!   churn is the per-limb rebuild — a fresh queue (16 preallocated
//!   packet slots), the limb struct, the command/receipt/event
//!   traffic: a few KiB;
//! - **cache broken** (teardown + rebuild per switch): every switch
//!   re-allocates the 256 KiB context — the cumulative
//!   `total_bytes` delta per switch jumps by the context size, and
//!   the per-track build counts stop being one.
//!
//! Both oracles are asserted: the provider's build counts (exact)
//! and the allocator's per-switch byte churn (pinned ceiling
//! **8× below** one context — a broken cache cannot hide under it).
//!
//! # What is measured, and how the bounds were pinned
//!
//! After each switch's settle (terminal observed, outputs drained
//! into one reused buffer), one [`AllocSnapshot`]. The first five
//! switches are warm-up (queue/vec pool sizing, B's first build);
//! the bounds are computed over the measured window (switches
//! 5..=40), mirroring `replacement_alloc.rs`:
//!
//! - **band** — `max(live_bytes) − min(live_bytes)` over the window;
//! - **creep** — `live_bytes[last] − live_bytes[first]` (the leak
//!   detector proper: anything retained per switch and never freed);
//! - **churn calls / churn bytes** — the largest per-switch
//!   `alloc_calls` / `total_bytes` delta: bounded allocation work per
//!   switch, and — the sharp one — far below one decoder context.
//!
//! The constants are **pinned from measurement** (D-6): the measured
//! maxima on this suite's fakes are recorded beside each constant,
//! with margin. They are ceilings, not budgets — if a change raises
//! the measured numbers, re-measure and re-pin honestly rather than
//! weakening the assertion silently.

#[global_allocator]
static ALLOC: testkit::alloc_count::CountingGlobalAlloc =
    testkit::alloc_count::CountingGlobalAlloc::new();

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
use media_clock::MonotonicSource;
use media_types::error::ErrorKind;
use media_types::frame::{AudioLayout, FrameStorage, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::{
    AudioFormat, Duration, EngineError, Frame, GenerationId, MediaKind, Nanoseconds, Packet,
    PipelineStage, PixelFormat, SourceUri, Stamp, TimeBase, TimeSpan, Timestamp, Track, TrackFlags,
    TrackId, TrackTiming,
};
use packet_queue::QueueLimits;
use playback_engine::{
    DecoderProvider, Engine, EngineEvent, EngineEventKind, EngineOutput, FrameConsumer, QueuePlan,
    SessionConfig, SessionHandle, SessionState, TickBudget, TrackSelector,
};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};
use testkit::Must;
use testkit::alloc_count::{AllocSnapshot, serialize_tests};

use commands::envelope::RequestId;

/// The decoder working-memory stand-in, sized to the same order as a
/// 1080p MJPEG `DecodeContext` (multi-MB in the real codec; 256 KiB
/// keeps the soak's RSS trivial while dwarfing every per-switch
/// bookkeeping allocation — the detector's contrast).
const CONTEXT_BYTES: usize = 256 * 1024;

/// Packets per track per source: no epoch reaches end-of-stream
/// inside the soak window, so every switch departs `Playing`.
const PACKETS_PER_TRACK: usize = 512;

/// The measured window's live-bytes band ceiling. Measured on this
/// suite's fakes: 4,096 bytes (the fresh-queue preallocation
/// oscillation); pinned at 16 KiB (~4×).
const BOUND_BAND: isize = 16 * 1024;

/// The measured window's live-bytes creep ceiling (first → last).
/// Measured: 0 bytes over 36 further switches (flat); pinned at
/// 1 KiB — tight enough to catch a one-slot-per-switch retention.
const BOUND_CREEP: isize = 1024;

/// The per-switch allocation-events ceiling. Measured: 214 calls per
/// switch (the fresh queue's 16 preallocated slots arrive as one
/// alloc; the rest is command/receipt/event traffic); pinned at 1024.
const BOUND_CHURN_CALLS: u64 = 1024;

/// The per-switch cumulative-bytes ceiling — **the cache proof**: a
/// working cache's per-switch work is the fresh queue + bookkeeping
/// (measured: ~6 KiB); a broken cache would re-allocate the 256 KiB
/// context every switch. Pinned at 32 KiB: 8× below one context,
/// ~5× above the measured churn.
const BOUND_CHURN_BYTES: u64 = 32 * 1024;

// ---------------------------------------------------------------------------
// Fakes (the marker family of `replacement_alloc.rs`, plus the
// context-allocating decoder this proof needs)
// ---------------------------------------------------------------------------

struct SharedNow(AtomicU64);

impl MonotonicSource for SharedNow {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// The non-retaining sink: counts frames (retention would be the
/// test's own growth, not the engine's).
struct TallyConsumer {
    frames: AtomicU64,
}

impl FrameConsumer for TallyConsumer {
    fn accept(&self, _frame: Frame, _track: TrackId) -> Result<(), EngineError> {
        self.frames.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

/// The on-demand marker demuxer: packets synthesized round-robin,
/// static payloads (the packet clone is an `Arc` bump, no alloc).
struct MarkerDemuxer {
    tracks: Vec<Track>,
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
        Ok(DemuxEvent::Packet(marker_packet(track, index as i64)))
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

/// The context-allocating decoder: **one** heap block at construction
/// (the `DecodeContext` stand-in), zero allocation ever after —
/// `configure` is the generation-only reset (the codec-api contract's
/// re-configuration seam), `decode` emits a static-payload frame per
/// packet.
struct ContextDecoder {
    generation: GenerationId,
    kind: MediaKind,
    #[allow(dead_code)]
    context: Vec<u8>,
}

impl Decoder for ContextDecoder {
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
                    sample: SampleFormat::S16,
                    channels: 2,
                    layout: AudioLayout::Interleaved,
                    rate: 48_000,
                },
                samples: SharedBytes::from_static(&[0u8; 4]),
                pts: packet.pts,
                duration: None,
                generation: packet.generation,
            }),
            _ => Frame::Video(media_types::frame::VideoFrame {
                format: PixelFormat::I420,
                size: (64, 48),
                data: FrameStorage::Cpu(SharedBytes::from_static(&[0u8; 8])),
                pts: packet.pts,
                color: media_types::frame::ColorDescription::BT709_LIMITED,
                interlaced: false,
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

/// The counting provider: builds the context decoders and counts
/// every build per track id (the recycling oracle).
struct ContextProvider {
    builds: Mutex<Vec<(TrackId, u32)>>,
}

impl ContextProvider {
    fn build_count(&self, track: u32) -> u32 {
        self.builds
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .filter(|(id, _)| id.handle() == track)
            .map(|(_, count)| *count)
            .sum()
    }
}

impl DecoderProvider for ContextProvider {
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
        Ok(Box::new(ContextDecoder {
            generation: config.generation,
            kind: track.kind,
            context: vec![0u8; CONTEXT_BYTES],
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

fn track(id: u32, kind: MediaKind) -> Track {
    Track {
        id: TrackId::new(id),
        stream: media_types::StreamId::new(id),
        kind,
        language: None,
        codec: media_types::ids::CodecDescriptor::new("fake-context").must(),
        title: None,
        flags: TrackFlags::NONE,
        timing: TrackTiming {
            span: TimeSpan::UNKNOWN,
            base: base(),
            gaps: Vec::new(),
        },
    }
}

fn marker_packet(track: &Track, pts_ms: i64) -> Packet {
    Packet {
        track: track.id,
        pts: at_millis(pts_ms),
        dts: Stamp::Missing,
        duration: Some(Duration::Known(Nanoseconds(1_000_000))),
        data: SharedBytes::from_static(&[7u8; 16]),
        keyframe: true,
        discontinuity: false,
        generation: GenerationId::FIRST,
    }
}

/// The track_switch.rs fake caps: 16 packets / 1 MiB / 1 s per queue
/// (high watermark 12, low 4).
fn soak_queues() -> QueuePlan {
    let limits = QueueLimits {
        max_packets: NonZeroUsize::new(16).must(),
        max_bytes: NonZeroUsize::new(1 << 20).must(),
        max_duration: Nanoseconds(1_000_000_000),
    };
    QueuePlan::new(limits, limits)
}

/// The track_switch.rs fake budget: 18 demux events (6 per track per
/// tick, of which the two wired limbs keep 6 pushed vs 4 decoded),
/// 4 packets + 8 flush calls per limb.
fn soak_budget() -> TickBudget {
    TickBudget {
        demux_events: 18,
        packets_per_limb: 4,
        flush_calls_per_limb: 8,
    }
}

struct Harness {
    engine: Engine,
    handle: SessionHandle,
    consumer: Arc<TallyConsumer>,
    provider: Arc<ContextProvider>,
}

impl Harness {
    fn new() -> Harness {
        let tracks = vec![
            track(1, MediaKind::Video),
            track(2, MediaKind::Audio),
            track(3, MediaKind::Audio),
        ];
        let consumer = Arc::new(TallyConsumer {
            frames: AtomicU64::new(0),
        });
        let clock = Arc::new(
            media_clock::MediaClock::system_monotonic(
                Arc::new(SharedNow(AtomicU64::new(0))),
                Timestamp::new(0, base()),
                media_types::Rate::ONE,
            )
            .must(),
        );
        let provider = Arc::new(ContextProvider {
            builds: Mutex::new(Vec::new()),
        });
        let mut config = SessionConfig::new(
            SourceUri::parse("file:///tmp/lumen-track-switch-alloc.media").must(),
            Box::new(MarkerDemuxer {
                tracks,
                remaining: PACKETS_PER_TRACK * 3,
                issued: 0,
            }),
            Arc::clone(&provider) as Arc<dyn DecoderProvider>,
            Arc::clone(&consumer) as Arc<dyn FrameConsumer>,
            clock,
        );
        config.queues = soak_queues();
        config.budget = soak_budget();
        config.throttle = playback_engine::EventPolicy {
            tick: playback_engine::TimeTickPolicy::default_policy(),
            throttle: Arc::new(SharedNow(AtomicU64::new(0))),
        };
        let mut engine = Engine::new();
        let handle = engine.open(config).must();
        Harness {
            engine,
            handle,
            consumer,
            provider,
        }
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

/// Drive one switch to its `tracks.changed` terminal, draining into
/// the reused buffer (D-1: no per-switch output allocation in the
/// measured window beyond the engine's own).
fn settle_switch(
    harness: &mut Harness,
    request: RequestId,
    selector: TrackSelector,
    out: &mut Vec<EngineOutput>,
    context: &str,
) {
    harness
        .handle
        .set_audio_track(request, selector)
        .must();
    out.clear();
    for _ in 0..20 {
        harness.engine.poll();
        harness.engine.drain_outputs_into(out);
        if out.iter().any(|output| {
            matches!(
                output,
                EngineOutput::Event(EngineEvent {
                    kind: EngineEventKind::TrackSelectionChanged { .. },
                    ..
                })
            )
        }) {
            return;
        }
    }
    unreachable!("{context}: no tracks.changed terminal within 20 polls");
}

// ---------------------------------------------------------------------------
// The bounded-memory AC
// ---------------------------------------------------------------------------

/// The decoder-cache recycling proof: 41 alternating audio switches
/// with (a) exact build counts — one decoder per track, ever; (b) the
/// engine's own `decoders_reused` accounting; (c) bounded memory
/// (band/creep) and bounded, context-free allocation churn per
/// switch.
#[test]
fn decoder_contexts_survive_switches_without_reallocation() {
    let _serial: MutexGuard<'static, ()> = serialize_tests();
    let mut harness = Harness::new();
    pump_to_playing(&mut harness, "prime");
    // The D-1 drain pattern: one reused buffer for the whole soak.
    let mut out: Vec<EngineOutput> = Vec::new();
    // Pre-sized (41 switches): the bookkeeping itself must not be a
    // growth term inside the measured window.
    let mut snapshots: Vec<AllocSnapshot> = Vec::with_capacity(41);

    for k in 0..=40u64 {
        // Alternate the target: k even → B (track 3), k odd → A
        // (track 2). The first switch builds B; every later switch
        // must be a cache hit.
        let target = if k.is_multiple_of(2) { 3 } else { 2 };
        let frames_before = harness.consumer.frames.load(Ordering::Relaxed);
        settle_switch(
            &mut harness,
            RequestId(100 + k),
            TrackSelector::Id {
                id: TrackId::new(target),
            },
            &mut out,
            "soak switch",
        );
        // The switch really completed on a delivered frame: the
        // tally grew this switch (the frame's delivery IS the
        // completion fact).
        let frames_after = harness.consumer.frames.load(Ordering::Relaxed);
        assert!(
            frames_after > frames_before,
            "switch {k}: the new limb delivered ({} → {frames_after})",
            frames_before
        );
        assert_eq!(harness.engine.state(), SessionState::Playing);
        snapshots.push(ALLOC.state().snapshot());
    }

    // (a) The recycling oracle: exactly one build per track across the
    // whole soak — the wire built video + A, the first switch built
    // B, and every one of the 40 later activations was a cache hit.
    assert_eq!(
        harness.provider.build_count(1),
        1,
        "the video decoder was built once, at wire"
    );
    assert_eq!(
        harness.provider.build_count(2),
        1,
        "audio A was built once, at wire"
    );
    assert_eq!(
        harness.provider.build_count(3),
        1,
        "audio B was built once, at the first switch"
    );
    // (b) The engine's own accounting agrees.
    let stats = harness.engine.stats();
    assert_eq!(stats.decoders_built, 3, "three builds total, ever");
    assert_eq!(
        stats.decoders_reused, 40,
        "every switch after B's first build was a cache hit"
    );

    // ---- The measured window: switches 5..=40 (0..=4 are warm-up). ----
    let window = &snapshots[5..=40];
    assert_eq!(window.len(), 36);

    // (c) band: the steady-state oscillation is bounded.
    let max_live = window.iter().map(|s| s.live_bytes).max().must();
    let min_live = window.iter().map(|s| s.live_bytes).min().must();
    let band = max_live - min_live;
    assert!(
        band <= BOUND_BAND,
        "live-bytes band over 36 switches: {band} bytes (max {max_live}, \
         min {min_live}) exceeds the pinned {BOUND_BAND}"
    );
    // (d) creep: no monotone growth across the window.
    let creep = window[window.len() - 1].live_bytes - window[0].live_bytes;
    assert!(
        creep <= BOUND_CREEP,
        "live-bytes creep over 36 switches: {creep} bytes (from {} to {}) \
         exceeds the pinned {BOUND_CREEP}",
        window[0].live_bytes,
        window[window.len() - 1].live_bytes
    );
    // (e) churn calls: bounded allocation events per switch.
    let max_calls = window
        .windows(2)
        .map(|pair| pair[1].alloc_calls - pair[0].alloc_calls)
        .max()
        .must();
    assert!(
        max_calls <= BOUND_CHURN_CALLS,
        "per-switch allocation events: {max_calls} exceeds the pinned {BOUND_CHURN_CALLS}"
    );
    // (f) churn bytes — the cache proof proper: the per-switch
    // cumulative allocation stays far below one decoder context. A
    // cache that tore down and rebuilt decoders would spend the
    // 256 KiB context on every switch and blow this bound by 8×.
    let max_bytes = window
        .windows(2)
        .map(|pair| pair[1].total_bytes - pair[0].total_bytes)
        .max()
        .must();
    assert!(
        max_bytes <= BOUND_CHURN_BYTES,
        "per-switch allocation bytes: {max_bytes} exceeds the pinned \
         {BOUND_CHURN_BYTES} (one decoder context is {CONTEXT_BYTES} — a \
         broken cache re-allocates it every switch)"
    );

    // The measured evidence (kept as asserts on the recorded shape so
    // a regression cannot hide behind a passing bound): the soak
    // really switched 41 times and delivered frames throughout.
    let frames = harness.consumer.frames.load(Ordering::Relaxed);
    assert!(frames >= 41, "every switch delivered frames (saw {frames})");
    assert!(band > 0, "a flat-zero band means the soak did not run");
}
