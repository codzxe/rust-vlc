//! The shared engine fixture of the property suites (card 3.D3): a
//! seekable scripted demuxer over a seeded synthetic timeline, an
//! immediate one-frame-per-packet decoder, and a recording collector.
//! The `tests/seek_flow.rs` harness shapes, compacted for property
//! driving — black-box: only public APIs are touched.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink};
use media_clock::MonotonicSource;
use media_types::frame::{AudioLayout, SampleFormat};
use media_types::packet::SharedBytes;
use media_types::{
    AudioFormat, Duration, EngineError, Frame, GenerationId, MediaKind, Nanoseconds, Packet,
    PipelineStage, SourceUri, Stamp, TimeBase, TimeSpan, Timestamp, Track, TrackFlags, TrackId,
    TrackTiming,
};
use packet_queue::QueueLimits;
use playback_engine::{
    DecoderProvider, Engine, FrameConsumer, QueuePlan, SessionConfig, SessionHandle, TickBudget,
};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};
use testkit::Must;

// ---------------------------------------------------------------------------
// Virtual time (doc 03 §8: no sleeps)
// ---------------------------------------------------------------------------

/// A never-advancing monotonic source (deterministic windows — the
/// `frozen_throttle` precedent).
pub struct FrozenNow(pub AtomicU64);

impl MonotonicSource for FrozenNow {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------------------
// The scripted timeline
// ---------------------------------------------------------------------------

/// One synthetic packet's timing shape (ns throughout).
#[derive(Debug, Clone, Copy)]
pub struct Slot {
    /// Owning track.
    pub track: TrackId,
    /// Presentation timestamp (ns, non-decreasing across the media —
    /// the landing oracle relies on the order).
    pub pts_ns: i64,
    /// Decode timestamp: `None` models a missing DTS; a value below
    /// `pts_ns` is the normal B-frame lead; above it, the adversarial
    /// `pts < dts` shape (stamps are data — the engine must not
    /// reorder by them in v0, and must never panic on them).
    pub dts_ns: Option<i64>,
}

/// The shared scripted media: the full packet list plus the demux
/// cursor. All packets are keyframes (the MJPEG-in-AVI shape), so the
/// keyframe-safe landing is the last packet at or before the target.
#[derive(Default)]
pub struct Media {
    pub slots: Vec<Slot>,
    pub cursor: usize,
}

fn lock_media(media: &Arc<Mutex<Media>>) -> std::sync::MutexGuard<'_, Media> {
    media
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The seekable scripted demux stage (the `SeekDemuxer` shape):
/// emits the timeline in order, then end-of-stream; `reanchor` is
/// compute-then-commit (contract D4).
pub struct TimelineDemuxer {
    tracks: Vec<Track>,
    media: Arc<Mutex<Media>>,
    seekable: bool,
}

impl Demuxer for TimelineDemuxer {
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        Ok(self.tracks.clone())
    }

    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        let mut media = lock_media(&self.media);
        if let Some(slot) = media.slots.get(media.cursor) {
            let slot = *slot;
            media.cursor += 1;
            return Ok(DemuxEvent::Packet(packet_of(slot)));
        }
        Ok(DemuxEvent::EndOfStream)
    }

    fn seek(&mut self, _target: TimeSpan) -> Result<TimeSpan, EngineError> {
        // The pure query is not the engine's seek path (doc 03 §6
        // step 4 goes through `reanchor`).
        Err(unsupported("demux.seek.not-the-reanchor-path"))
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
        let Some(target_ns) = stamp_ns(&target.start) else {
            return Err(corrupt("demux.reanchor.target-missing"));
        };
        // Keyframe-safe position ≤ target (all keyframes): the last
        // packet at or before it.
        let landing = media
            .slots
            .iter()
            .rposition(|slot| slot.pts_ns <= target_ns)
            .unwrap_or(0);
        media.cursor = landing;
        let start = media
            .slots
            .get(landing)
            .map_or(Stamp::Missing, |slot| at_nanos(slot.pts_ns));
        Ok(TimeSpan {
            start,
            end: Stamp::Missing,
        })
    }
}

fn unsupported(code: &'static str) -> EngineError {
    EngineError::new(
        PipelineStage::Demux,
        media_types::ErrorKind::Unsupported { format: None },
        media_types::error::ErrorContext::new(code),
    )
}

fn corrupt(code: &'static str) -> EngineError {
    EngineError::new(
        PipelineStage::Demux,
        media_types::ErrorKind::Corrupt {
            detail: media_types::error::CorruptDetail::InvalidStructure { offset: None },
        },
        media_types::error::ErrorContext::new(code),
    )
}

/// The immediate decode stage: one frame per packet, the frame
/// carrying the packet's own pts and generation (the honest shape).
pub struct ImmediateDecoder {
    generation: GenerationId,
}

impl Decoder for ImmediateDecoder {
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

/// The decoder seam: an immediate decoder for every track.
pub struct ImmediateProvider;

impl DecoderProvider for ImmediateProvider {
    fn decoder_for(
        &self,
        _track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError> {
        Ok(Box::new(ImmediateDecoder {
            generation: config.generation,
        }))
    }
}

// ---------------------------------------------------------------------------
// The recording collector
// ---------------------------------------------------------------------------

/// One delivered frame, as the property suites need it: track,
/// generation counter, and present-pts in ns (`None` = missing stamp
/// — delivered anyway, per the `FirstFrame` contract).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delivered {
    pub track: TrackId,
    pub generation: u64,
    pub pts_ns: Option<i64>,
}

/// The collecting sink: every frame the engine delivered, in delivery
/// order.
#[derive(Default)]
pub struct Collector {
    pub frames: Mutex<Vec<Delivered>>,
}

impl FrameConsumer for Collector {
    fn accept(&self, frame: Frame, track: TrackId) -> Result<(), EngineError> {
        let pts_ns = stamp_ns(&frame.pts());
        let delivered = Delivered {
            track,
            generation: frame.generation().counter(),
            pts_ns,
        };
        self.frames
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(delivered);
        Ok(())
    }
}

impl Collector {
    /// The delivery count (a snapshot under one lock).
    pub fn len(&self) -> usize {
        self.frames
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    /// The frames at `from..`, snapshotted under one lock.
    pub fn frames_from(&self, from: usize) -> Vec<Delivered> {
        self.frames
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())[from..]
            .to_vec()
    }
}

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

fn base() -> TimeBase {
    TimeBase::new(1, 1_000_000_000).must()
}

fn at_nanos(ns: i64) -> Stamp {
    Stamp::Present(Timestamp::new(ns, base()))
}

/// A present stamp's nanoseconds (`None` for a missing stamp).
pub fn stamp_ns(stamp: &Stamp) -> Option<i64> {
    let timestamp = stamp.timestamp().ok()?;
    match timestamp.as_nanos() {
        Ok(Nanoseconds(ns)) => Some(i64::try_from(ns).unwrap_or(i64::MAX)),
        Err(_) => None,
    }
}

fn packet_of(slot: Slot) -> Packet {
    Packet {
        track: slot.track,
        pts: at_nanos(slot.pts_ns),
        dts: slot.dts_ns.map_or(Stamp::Missing, at_nanos),
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

/// A track whose declared extent ends at `end_ns` (`None` leaves the
/// span unknown — the duration-less shape).
pub fn track(id: u32, kind: MediaKind, end_ns: Option<i64>) -> Track {
    Track {
        id: TrackId::new(id),
        stream: media_types::StreamId::new(id),
        kind,
        language: None,
        codec: media_types::ids::CodecDescriptor::new("fake-pcm").must(),
        title: None,
        flags: TrackFlags::NONE,
        timing: TrackTiming {
            span: match end_ns {
                Some(end) => TimeSpan {
                    start: at_nanos(0),
                    end: at_nanos(end),
                },
                None => TimeSpan::UNKNOWN,
            },
            base: base(),
            gaps: Vec::new(),
        },
    }
}

/// Small deterministic queue caps (the `seek_flow` numbers): the
/// packets axis governs — cap 16, high at 12, low at 4.
pub fn tiny_queues() -> QueuePlan {
    let limits = QueueLimits {
        max_packets: std::num::NonZeroUsize::new(16).must(),
        max_bytes: std::num::NonZeroUsize::new(1 << 20).must(),
        max_duration: Nanoseconds(1_000_000_000),
    };
    QueuePlan::new(limits, limits)
}

/// The `seek_flow` tick budget: demux slice 12, decode slice 4.
pub fn tiny_budget() -> TickBudget {
    TickBudget {
        demux_events: 12,
        packets_per_limb: 4,
        flush_calls_per_limb: 8,
    }
}

/// Everything one property case needs to drive a session.
pub struct Fixture {
    pub engine: Engine,
    pub handle: SessionHandle,
    pub collector: Arc<Collector>,
    /// The session clock (the seek suite's re-anchor checks read it).
    pub clock: Arc<media_clock::MediaClock>,
}

/// A built (not yet opened) session: the config plus the shared
/// collector a storm needs to reopen sessions on one engine.
pub struct Built {
    pub config: SessionConfig,
    pub collector: Arc<Collector>,
    /// The session clock (re-anchored by seeks; handed to the fixture).
    pub clock: Arc<media_clock::MediaClock>,
}

/// Build a session config over the given timeline (`tracks` must list
/// every track the timeline references; each track's declared extent
/// carries the duration the engine clamps against — there is no
/// separate duration knob). `seekable` arms the demuxer's reanchor.
pub fn build(tracks: Vec<Track>, slots: Vec<Slot>, seekable: bool) -> Built {
    let media = Arc::new(Mutex::new(Media { slots, cursor: 0 }));
    let collector = Arc::new(Collector::default());
    let clock = Arc::new(
        media_clock::MediaClock::system_monotonic(
            Arc::new(FrozenNow(AtomicU64::new(0))),
            Timestamp::new(0, base()),
            media_types::Rate::ONE,
        )
        .must(),
    );
    let mut config = SessionConfig::new(
        SourceUri::parse("file:///tmp/lumen-prop-fake.media").must(),
        Box::new(TimelineDemuxer {
            tracks,
            media: Arc::clone(&media),
            seekable,
        }),
        Arc::new(ImmediateProvider),
        Arc::clone(&collector) as Arc<dyn FrameConsumer>,
        Arc::clone(&clock),
    );
    config.queues = tiny_queues();
    config.budget = tiny_budget();
    // The frozen throttle keeps every emission window deterministic.
    config.throttle = playback_engine::EventPolicy {
        tick: playback_engine::TimeTickPolicy::default_policy(),
        throttle: Arc::new(FrozenNow(AtomicU64::new(0))),
    };
    Built {
        config,
        collector,
        clock,
    }
}

/// Open a fresh engine + one session over the timeline (the
/// single-session fixture; storms use [`build`] to reopen on one
/// engine).
pub fn open(tracks: Vec<Track>, slots: Vec<Slot>, seekable: bool) -> Fixture {
    let Built {
        config,
        collector,
        clock,
    } = build(tracks, slots, seekable);
    let mut engine = Engine::new();
    let handle = engine.open(config).must();
    Fixture {
        engine,
        handle,
        collector,
        clock,
    }
}

/// The keyframe-safe landing oracle for the scripted timeline: the
/// last slot at or before `target_ns` (all keyframes), as
/// `(landing_index, landing_pts_ns)`. A target before the first pts
/// lands at index 0 (the demuxer's `unwrap_or(0)`).
pub fn landing_of(slots: &[Slot], target_ns: i64) -> (usize, i64) {
    let index = slots
        .iter()
        .rposition(|slot| slot.pts_ns <= target_ns)
        .unwrap_or(0);
    let pts = slots.get(index).map_or(0, |slot| slot.pts_ns);
    (index, pts)
}

/// The engine's target resolution (the `resolve_target` contract,
/// mirrored test-side): absolute/relative clamp to `[0, duration]`
/// when the duration is known (else floor at zero); percent needs a
/// known duration. Answers the clamped ns, or `None` for the typed
/// out-of-range family.
pub fn resolve_target_ns(
    target: playback_engine::SeekTarget,
    duration_ns: Option<i64>,
    position_ns: i64,
) -> Option<i64> {
    use playback_engine::SeekTarget;
    let clamp = |ns: i64| -> i64 {
        match duration_ns {
            Some(duration) => ns.clamp(0, duration),
            None => ns.max(0),
        }
    };
    match target {
        SeekTarget::Absolute { position_ns } => Some(clamp(position_ns)),
        SeekTarget::Relative { delta_ns } => Some(clamp(position_ns.saturating_add(delta_ns))),
        SeekTarget::Percent { fraction } => duration_ns.map(|duration| {
            // The engine's exact arithmetic (seek.rs `resolve_target`),
            // mirrored: a different rounding would be a false
            // counterexample.
            clamp((f64::from(fraction.clamp(0.0, 1.0)) * duration as f64).round() as i64)
        }),
        SeekTarget::Chapter { .. } => None,
    }
}
