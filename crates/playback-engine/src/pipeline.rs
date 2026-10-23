//! The trait-level pipeline wiring (card 2.C2): **demux → packet
//! queues → decoders → sinks**, driven one bounded tick at a time.
//!
//! # Stage boundaries are existing contracts
//!
//! No concrete demuxer or decoder is depended on (those are cards
//! 2.B2/2.B4/2.B5, in flight). The stages meet at:
//!
//! - **demux**: [`plugin_api::Demuxer`] — the engine calls
//!   `start()` during `Opening` (header parse + track enumeration is
//!   the v0 probe boundary) and `next_event()` during ticks;
//! - **queues**: [`packet_queue::BoundedQueue<Packet>`] — one per
//!   enabled track (audio/video), engine-constructed from the
//!   [`QueuePlan`], generation-stamped at push (doc 03 §5 checking
//!   point 3) and flushed at every pipeline mint (point 4);
//! - **decoders**: [`codec_api::Decoder`] — obtained per track from
//!   the [`DecoderProvider`] seam (the future plugin registry), and
//!   configured with the current queue generation so
//!   `drop_stale_packet` semantics line up with the queues';
//! - **sinks**: [`codec_api::FrameSink`] — the engine's
//!   [`LimbSink`] adapter that checks each output frame's generation
//!   (doc 03 §5 checking point 5) and hands it to the
//!   [`FrameConsumer`] seam (the audio pump / video scheduler of
//!   cards 2.C3/2.C4). While a seek is in flight the hand-off is
//!   **held** (M10): pre-roll frames are captured into a bounded
//!   per-limb hold and reach the consumer only at the seek terminal —
//!   never while the clock is frozen at the old position (see
//!   [`Pipeline::release_preroll_hold`]).
//!
//! # The tick model (doc 03 §4.6(c)/§9 — non-blocking)
//!
//! v0 is synchronous and thread-free: the pipeline advances only
//! when the session polls it, in **bounded** slices — at most
//! `budget.demux_events` demux events, `budget.packets_per_limb`
//! decode calls and `budget.flush_calls_per_limb` flush calls per
//! tick. Nothing blocks, nothing spins: a full queue hands the packet
//! back and the engine *holds* it (`pending`) for the next tick —
//! the synchronous form of doc 03 §4.4's parking producer. The
//! decode→engine **frame queue** stage is deliberately absent
//! (packet-queue's `Weigh` is not yet implemented for frames —
//! 2.B1's own recorded limitation); decoded frames flow directly
//! through the sink seam, which is exactly the card's
//! "decoders → sinks" boundary.
//!
//! # Damage policy (doc 03 §4.5(d), audit M9 / ED-FIX-4)
//!
//! Demux-stage bounded recovery and decoder-reported `Recovered`
//! outcomes flow into the tick's damage facts (the engine-private
//! `DamagedData` event + the coalesced non-fatal `playback.error`).
//! Since W1-08 the decode slice extends the same isolation to video
//! packets that **fail** with a per-frame damage kind (`Corrupt`,
//! `Decoder`/`Bitstream`): the damaged slot is counted, the
//! presentation conceals by repeating the limb's last presented frame
//! (or holds when none exists), and playback continues — one corrupt
//! frame no longer ends the session. Since 3.C2 the audio lane
//! mirrors the same shape MINUS concealment (defect G3, design D-2:
//! there is no "last presented audio frame" worth repeating, and
//! inserting silence would fabricate data): a per-frame-damaged audio
//! packet is **dropped, counted, never synthesized** — the ring
//! simply has a gap and the clock continues from the next delivered
//! frame's pts. A bounded run of consecutive failures
//! ([`VIDEO_CONCEALMENT_BUDGET`], one shared budget for both limbs)
//! still reaches a typed terminal, because a stream whose every
//! packet fails is not playing.
//!
//! # Watermark policy (card 3.B3)
//!
//! Packet-queue *reports* watermarks; this crate (the engine) *owns
//! the policy* (2.B1's recorded hand-off). [`QueuePlan`] sets the high
//! threshold at three quarters of every cap and makes **packets the
//! starvation axis**: the low threshold binds on packets alone (the
//! byte/duration low marks sit at their caps, never binding), so the
//! queue's all-axes-low predicate degenerates to packets-only — a
//! bytes-heavy or unknown-duration mix can always reach `low`
//! through its packet count (design B2 gap 2: no unreachable-low
//! silent stalls). Priming (start) requires **every** enabled limb to
//! be high; an underrun fires when **any** limb runs low while the
//! demuxer can still refill — the end-of-stream drain is not an
//! underrun.
//!
//! ## Starve debounce (design B2/F1.5)
//!
//! A starvation **episode** opens when a limb's queue runs low and
//! the stall fires; it closes when the limb recovers to its high
//! watermark (a full refill — priming — or any in-Playing recovery)
//! or at a seek mint. The **first** stall of a fresh episode fires at
//! 1 tick (first-stall latency is never delayed); a **re-entry** —
//! the queue dipping low again while an episode is still open, i.e.
//! without a full recovery in between — must persist
//! [`QueuePlan::starve_debounce_ticks`] consecutive low ticks before
//! it fires. That filters the flapping producer/consumer rate mix
//! that hovers at the low line (single-tick dips inside an open
//! episode never re-fire) without delaying honest rebuffer cycles
//! (each of those passes through a prime, which closes the episode).
//!
//! Backpressure itself never flushes or closes anything (D-4):
//! *backpressure modulates state, generations modulate data* — the
//! mint-only flush rule holds everywhere in this file.

use std::sync::Arc;

use codec_api::{DecodeOutcome, Decoder, DecoderConfig, FrameSink, ThreadBudget};
use media_types::time::TimeSpan;
use media_types::{EngineError, Frame, GenerationId, MediaKind, Packet, Stamp, Track, TrackId};
use packet_queue::{
    BoundedQueue, PopOutcome, PushOutcome, QueueLimits, QueueUsage, Stamped, WatermarkThresholds,
};
use plugin_api::{DemuxEvent, Demuxer, SeekSupport};

use crate::seek::nanos_i64;
use crate::state::SessionState;
use crate::tracks::WiredKind;

/// The registry seam (card 2.C2): produces a decoder for a track.
///
/// The future plugin registry (doc 06) implements this; tests
/// implement it with fakes. Returning `Err` is a *fatal* selection
/// failure (the registry said it could decode and then could not) —
/// the session enters `Error` with the typed cause.
pub trait DecoderProvider: Send + Sync {
    /// Build the decoder for `track`, configured with `config` (which
    /// carries the current queue generation — see
    /// [`DecoderConfig::generation`]).
    ///
    /// # Errors
    /// The provider's typed [`EngineError`] (stage
    /// [`media_types::PipelineStage::Decode`] by convention).
    fn decoder_for(
        &self,
        track: &Track,
        config: &DecoderConfig,
    ) -> Result<Box<dyn Decoder>, EngineError>;
}

/// The sink seam (card 2.C2): where decoded frames leave the engine.
///
/// The audio pump (2.C3) and the video scheduler (2.C4) implement
/// this; v0 hosts pass a collector. Called on the polling thread (the
/// single writer) — implementations must not block (doc 03 §9).
pub trait FrameConsumer: Send + Sync {
    /// Take delivery of one current-generation frame from `track`.
    ///
    /// # Errors
    /// The consumer's typed [`EngineError`] (backpressure, teardown)
    /// — propagated as the decode call's failure, per the
    /// [`FrameSink`] contract.
    fn accept(&self, frame: Frame, track: TrackId) -> Result<(), EngineError>;
}

/// Which per-kind packet axis a [`QueuePlanError`] names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueuePlanKind {
    /// The audio packet cap (`limits.queue_packets_audio`).
    Audio,
    /// The video packet cap (`limits.queue_packets_video`).
    Video,
}

impl std::fmt::Display for QueuePlanKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            QueuePlanKind::Audio => "audio",
            QueuePlanKind::Video => "video",
        })
    }
}

/// An invalid product-shaped queue plan (card 3.B3, design A2):
/// out-of-range packet caps are **rejected, never clamped** (MP §5 —
/// the same posture as product.toml's own schema ranges).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueuePlanError {
    /// A packet cap below the schema floor (64).
    PacketsBelowFloor {
        /// Which per-kind axis.
        kind: QueuePlanKind,
        /// The value the host supplied.
        value: u64,
        /// The schema floor (product.schema.json:192-193).
        floor: u64,
    },
    /// A packet cap above the schema ceiling (8192 audio / 4096
    /// video).
    PacketsAboveCeiling {
        /// Which per-kind axis.
        kind: QueuePlanKind,
        /// The value the host supplied.
        value: u64,
        /// The schema ceiling for that kind.
        ceiling: u64,
    },
}

impl std::fmt::Display for QueuePlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueuePlanError::PacketsBelowFloor { kind, value, floor } => write!(
                f,
                "{kind} queue packet cap {value} is below the schema floor {floor} \
                 (product.schema.json limits.queue_packets_{kind})"
            ),
            QueuePlanError::PacketsAboveCeiling {
                kind,
                value,
                ceiling,
            } => write!(
                f,
                "{kind} queue packet cap {value} is above the schema ceiling {ceiling} \
                 (product.schema.json limits.queue_packets_{kind})"
            ),
        }
    }
}

impl std::error::Error for QueuePlanError {}

/// The shared schema floor for per-kind packet caps
/// (product.schema.json:192-193: `minimum: 64`). Duplicated by design
/// A2 — the schema is the authority; the constants test in
/// `tests/queue_policy.rs` pins the duplication against the file.
pub const QUEUE_PACKETS_FLOOR: u64 = 64;
/// The audio packet-cap ceiling (product.schema.json:192:
/// `maximum: 8192`).
pub const QUEUE_PACKETS_AUDIO_CEILING: u64 = 8192;
/// The video packet-cap ceiling (product.schema.json:193:
/// `maximum: 4096`).
pub const QUEUE_PACKETS_VIDEO_CEILING: u64 = 4096;

/// The default starve debounce (design B2/F1.5): a re-entry dip must
/// persist this many consecutive low ticks to fire. `[PROPOSAL:
/// pending 3.B1 sync observations; config key
/// `queue_starve_debounce_ticks` in design A5 awaits Lead
/// ratification]`.
pub const DEFAULT_STARVE_DEBOUNCE_TICKS: u32 = 2;

/// Per-kind queue caps for the session's packet queues (doc 03
/// §4.4 defaults via packet-queue; the watermark policy lives here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueuePlan {
    /// Caps for audio limbs.
    pub audio: QueueLimits,
    /// Caps for video limbs.
    pub video: QueueLimits,
    /// Caps for subtitle limbs (doc 03 §4.4(b): "small and
    /// duration-only"; card 3.A3): subtitle packets are tiny, sparse
    /// and latency-tolerant, so the caps sit far below the A/V
    /// defaults — `[PROPOSAL: doc-default constants; the product.toml
    /// wiring is owed to card 3.B3, which owns per-kind limits]`.
    pub subtitle: QueueLimits,
    /// How many consecutive low ticks a re-entry dip must persist
    /// before it fires `starved` (design B2/F1.5).
    pub starve_debounce_ticks: u32,
}

impl QueuePlan {
    /// A plan from explicit caps (both must be valid — they are the
    /// packet-queue constructor's own outputs); the subtitle lane and
    /// the debounce take their documented defaults (see
    /// [`QueuePlan::default`]).
    #[must_use]
    pub fn new(audio: QueueLimits, video: QueueLimits) -> QueuePlan {
        QueuePlan {
            audio,
            video,
            subtitle: default_subtitle_limits(),
            starve_debounce_ticks: DEFAULT_STARVE_DEBOUNCE_TICKS,
        }
    }

    /// A plan with every lane explicit (card 3.A3's track-switching
    /// shape).
    #[must_use]
    pub fn with_subtitle(
        audio: QueueLimits,
        video: QueueLimits,
        subtitle: QueueLimits,
    ) -> QueuePlan {
        QueuePlan {
            audio,
            video,
            subtitle,
            starve_debounce_ticks: DEFAULT_STARVE_DEBOUNCE_TICKS,
        }
    }

    /// A plan from product-config-shaped integers (card 3.B3, design
    /// A2): the `limits.queue_packets_audio` / `queue_packets_video`
    /// values product.toml carries. Packet caps map 1:1 and are
    /// validated against the schema ranges — **reject, never clamp**
    /// (MP §5). Byte caps keep the doc 03 §4.4 figures (1 MiB / 4 MiB
    /// `[PROPOSAL: until the Lead ratifies the byte-axis keys, design
    /// A5]`); duration caps stay 1 s / 2 s; the subtitle lane and the
    /// starve debounce take their documented defaults.
    ///
    /// `QueuePlan::default()` does not change when a host injects a
    /// plan: the library default (200/300 packets) and the product
    /// posture (1024/512) are different layers, visible by diff.
    ///
    /// # Errors
    /// [`QueuePlanError`] — the typed axis and the violated schema
    /// bound, per kind.
    pub fn from_product_limits(
        audio_packets: u64,
        video_packets: u64,
    ) -> Result<QueuePlan, QueuePlanError> {
        let check = |kind: QueuePlanKind, value: u64, ceiling: u64| {
            if value < QUEUE_PACKETS_FLOOR {
                Err(QueuePlanError::PacketsBelowFloor {
                    kind,
                    value,
                    floor: QUEUE_PACKETS_FLOOR,
                })
            } else if value > ceiling {
                Err(QueuePlanError::PacketsAboveCeiling {
                    kind,
                    value,
                    ceiling,
                })
            } else {
                Ok(())
            }
        };
        check(
            QueuePlanKind::Audio,
            audio_packets,
            QUEUE_PACKETS_AUDIO_CEILING,
        )?;
        check(
            QueuePlanKind::Video,
            video_packets,
            QUEUE_PACKETS_VIDEO_CEILING,
        )?;
        Ok(QueuePlan {
            audio: QueueLimits {
                max_packets: nonzero(audio_packets as usize),
                max_bytes: nonzero(1 << 20),
                max_duration: media_types::Nanoseconds(1_000_000_000),
            },
            video: QueueLimits {
                max_packets: nonzero(video_packets as usize),
                max_bytes: nonzero(4 << 20),
                max_duration: media_types::Nanoseconds(2_000_000_000),
            },
            subtitle: default_subtitle_limits(),
            starve_debounce_ticks: DEFAULT_STARVE_DEBOUNCE_TICKS,
        })
    }

    /// The caps for a wired kind (design A3; audit F-1): audio,
    /// video and subtitle each have a lane — the function is
    /// **total** over [`WiredKind`], so a lane-less kind cannot
    /// reach the lane-building seams at all. (The retired form
    /// answered `Option<QueueLimits>` for a [`MediaKind`] and its
    /// only caller carried the crate's one production `expect`; the
    /// data-is-lane-less truth now lives in the typed
    /// [`WiredKind::try_from`](crate::tracks::WiredKind::try_from)
    /// refusal.)
    #[must_use]
    pub fn for_kind(&self, kind: WiredKind) -> QueueLimits {
        match kind {
            WiredKind::Audio => self.audio,
            WiredKind::Video => self.video,
            WiredKind::Subtitle => self.subtitle,
        }
    }

    /// The engine's watermark thresholds for a queue built with
    /// `limits` (card 3.B3): high at three quarters of every cap
    /// (eager on any axis — "full enough in some dimension"); **low
    /// on the packets axis alone** — the byte and duration low marks
    /// sit at their caps so the queue's all-axes-low predicate
    /// degenerates to packets-only and every payload mix can reach
    /// `low` (design B2 gap 2: a bytes-heavy or unknown-duration mix
    /// can never wedge the signal unreachable). Floored at one unit
    /// so degenerate caps still signal.
    #[must_use]
    pub fn thresholds(&self, limits: &QueueLimits) -> WatermarkThresholds {
        let high = QueueUsage {
            packets: limits.max_packets.get() - (limits.max_packets.get() / 4),
            bytes: limits.max_bytes.get() - (limits.max_bytes.get() / 4),
            duration: media_types::Nanoseconds(limits.max_duration.0 - limits.max_duration.0 / 4),
        };
        let low = QueueUsage {
            packets: ((limits.max_packets.get() as u64) / 4).max(1) as usize,
            // Never-binding: usage can never exceed its own cap, so
            // these two marks make the packets axis the only low
            // authority (see the method docs).
            bytes: limits.max_bytes.get(),
            duration: limits.max_duration,
        };
        WatermarkThresholds::new(high, low)
    }
}

/// The default subtitle lane (design A3): 128 packets / 16 MiB /
/// 10 s `[PROPOSAL figures]` — duration-led, small.
const fn default_subtitle_limits() -> QueueLimits {
    QueueLimits {
        max_packets: nonzero(128),
        max_bytes: nonzero(16 << 20),
        max_duration: media_types::Nanoseconds(10_000_000_000),
    }
}

impl Default for QueuePlan {
    /// Doc 03 §4.4's defaults through packet-queue's constructors:
    /// audio 200 packets / 1 s / 1 MiB, video 300 packets / 2 s /
    /// 4 MiB, subtitle 128 packets / 16 MiB / 10 s `[PROPOSAL: the
    /// byte caps; the packet/duration figures are doc 03 §4.4's]`.
    fn default() -> QueuePlan {
        let one_mib = 1usize << 20;
        let four_mib = 4usize << 20;
        QueuePlan {
            audio: QueueLimits::audio_default(nonzero(one_mib)),
            video: QueueLimits::video_default(nonzero(four_mib)),
            subtitle: default_subtitle_limits(),
            starve_debounce_ticks: DEFAULT_STARVE_DEBOUNCE_TICKS,
        }
    }
}

/// `NonZeroUsize` from a constant-nonzero literal, panic-free (the
/// fallback is unreachable for the literals used; `MIN` keeps the
/// function total regardless).
const fn nonzero(value: usize) -> std::num::NonZeroUsize {
    match std::num::NonZeroUsize::new(value) {
        Some(n) => n,
        None => std::num::NonZeroUsize::MIN,
    }
}

/// The per-tick work bound (doc 03 §4.6(b): command latency is
/// bounded by one tick — the numbers are `[PROPOSAL: measured in
/// Phase 3 benches per doc 03 §4.6(b)]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickBudget {
    /// Demux events (packets + container signals) per tick.
    pub demux_events: u32,
    /// Packets popped and decoded per limb per tick.
    pub packets_per_limb: u32,
    /// Decoder flush calls per limb per tick (draining at
    /// end-of-stream is bounded like everything else).
    pub flush_calls_per_limb: u32,
}

impl Default for TickBudget {
    fn default() -> TickBudget {
        TickBudget {
            demux_events: 64,
            packets_per_limb: 16,
            flush_calls_per_limb: 8,
        }
    }
}

/// What one pipeline tick observed (raw facts — the session decides
/// what they mean; see [`crate::state::translate_pipeline_signal`]).
/// Not `Clone`/`PartialEq`: the fatal fact carries the typed
/// [`EngineError`] (neither trait — the payload moves, it never
/// duplicates).
#[derive(Debug)]
pub(crate) struct TickReport {
    /// A fatal demux/decode failure, if one happened.
    pub fatal: Option<EngineError>,
    /// Every enabled limb's queue reached its high watermark.
    pub primed: bool,
    /// Some limb's queue drained to its low watermark while the
    /// demuxer can still refill.
    pub starved: bool,
    /// The demuxer hit end-of-stream **newly** this tick.
    pub eof_reached: bool,
    /// End-of-stream, all queues empty, all decoders drained.
    pub drained: bool,
    /// The first frame's presentation timestamp, the first time one
    /// arrives (doc 03 §4.6 `FirstFrame`).
    pub first_frame: Option<Stamp>,
    /// Units skipped by bounded recovery this tick (doc 03 §4.6
    /// `DamagedData`) — the demux and decode halves separately (the
    /// non-fatal `playback.error` coalescer is per stage, card 3.B4).
    pub damaged_demux: u64,
    /// The decode half of `damaged`.
    pub damaged_decode: u64,
    /// Units skipped by bounded recovery this tick, both stages
    /// combined (the engine-private `DamagedData` fact).
    pub damaged: u64,
    /// The least-filled limb's queue as a fraction of its high
    /// watermark, `0.0..=1.0` — the in-episode `session.buffering`
    /// progress (card 3.B4, doc 05 §3.2: the bottleneck limb is the
    /// honest progress; the episode's exit condition is every limb
    /// high). Packets are the axis by the card 3.B3 policy (see
    /// [`QueuePlan::thresholds`]): the byte/duration axes would fire
    /// first for unusual packet-size mixes.
    pub buffer_fill: f32,
    /// Limbs whose decoder newly flushed to `Drained` this tick (the
    /// `playback.end-of-track` candidates, card 3.B4) — the count;
    /// the `(kind, id)` pairs ride the pipeline's reused
    /// [`Pipeline::take_ended_limbs`] scratch.
    pub newly_drained: u32,
    /// Frames delivered to the sink seam this tick (during a seek's
    /// pre-roll, accepted frames enter the presentation **hold**
    /// instead of the consumer — released at the terminal, discarded
    /// on rollback; the session's totals subtract the discards, so
    /// `EngineStats::frames_delivered` stays the consumer-delivered
    /// truth).
    pub frames: u64,
    /// Concealment presentations this tick: frames the engine
    /// re-presented from the per-limb last-good cache because the
    /// packet that owned the slot failed with per-frame damage (audit
    /// M9 / doc 03 §4.5(d)) — repeated content, never new pixels.
    /// Labeled separately from `frames` so a repeated frame is never
    /// counted as decoded output.
    pub concealed_frames: u64,
    /// Frames dropped at the sink for stale generations (doc 03 §5
    /// checking point 5). Always zero in v0's single-writer ticks —
    /// the checkpoint exists for the threaded future, and the tests
    /// pin that it never fires on live-generation frames.
    pub stale_frames: u64,
    /// Demuxer-marked discontinuities observed this tick (the
    /// re-anchor policy is Phase 3; the count is the observable
    /// fact).
    pub discontinuities: u64,
}

/// One wired track: its queue and its decoder.
pub(crate) struct Limb {
    /// The demuxed track descriptor.
    pub track: Track,
    /// The demux→decode packet queue (engine-owned).
    pub queue: Arc<BoundedQueue<Packet>>,
    /// The track's decoder (registry-provided).
    pub decoder: Box<dyn Decoder>,
    /// The decoder has flushed to `Drained` at end-of-stream.
    pub drained: bool,
    /// The queue's high watermark in packets — the denominator of
    /// the in-episode buffering progress (the priming target).
    pub high_packets: usize,
    /// Consecutive low-watermark ticks (card 3.B3, design B2): the
    /// starve-debounce streak. Reset by any non-low observation;
    /// per-pipeline-generation state (cleared at seek mints).
    pub low_streak: u32,
    /// A starvation episode has fired since this limb's queue last
    /// reached its high watermark (card 3.B3, design B2/F1.5): the
    /// first stall of a fresh episode fires at 1 tick; a re-entry
    /// dip while the episode is open must persist
    /// [`QueuePlan::starve_debounce_ticks`] consecutive low ticks.
    pub episode_open: bool,
    /// The queue's low watermark in packets — the **recovery target**
    /// that ends a switch-installed limb's re-prime exemption (see
    /// [`Limb::first_frame_pending`]).
    pub low_packets: usize,
    /// **The limb's generation floor** (card 3.A3): the generation
    /// this limb was last flushed/configured at. The sink checks
    /// frames against this, not the pipeline-global scalar — after a
    /// switch mint the unchanged kind's G-stamped residents stay
    /// presentable while the switched kind's stale stragglers (stamps
    /// < the new limb's floor) still die. Soundness: every packet a
    /// limb decodes passed its queue's boundary (`stamp <
    /// queue.published` refused), and flushes only ratchet
    /// (`published ≥ floor`), so emitted stamps are never spuriously
    /// stale; genuinely stale work carries stamps below the floor of
    /// whichever limb would present it. Doc 03 §5 point 5's
    /// checkpoint semantics are thereby read **per the generation
    /// the consumer path was last invalidated at** (noted for
    /// 3.B4's contract suite).
    pub generation: GenerationId,
    /// A limb installed by a switch whose queue has not yet
    /// recovered from the swap (card 3.A3 step 5, the S2.5
    /// carve-out): while armed, the limb is exempt from the underrun
    /// check and from priming — the new queue starts empty through
    /// no fault of the input, and a selection change is not a
    /// starvation. Cleared when the queue first reaches its **low
    /// watermark** (not at the first frame: the queue is still
    /// climbing then, and the completing tick would otherwise flash
    /// playing→buffering). A sparse or span-exhausted track simply
    /// never re-paces (VLC parity — honest silence, no rebuffer).
    pub first_frame_pending: bool,
    /// The last presented **video** frame of the current generation —
    /// the per-frame-damage concealment cache (doc 03 §4.5(d), audit
    /// M9): when a packet fails with a per-frame damage kind, the
    /// presentation conceals by repeating this frame at the damaged
    /// packet's pts (never new content). One slot per limb is the
    /// hard bound (D-1); a stale-generation entry is never repeated
    /// (the staleness check at concealment time is the discard rule,
    /// so seek mints need no extra invalidation).
    last_frame: Option<Frame>,
    /// Consecutive per-frame damage failures on this limb — the video
    /// concealment run or the audio skip run (3.C2 G3), whichever
    /// lane this limb is. Reset by any `Ok` decode outcome; the run
    /// is bounded by [`VIDEO_CONCEALMENT_BUDGET`] (D-3: recovery is
    /// bounded, never an infinite freeze).
    conceal_failures: u32,
}

/// The per-frame damage budget, shared by BOTH lanes: how many
/// **consecutive** damaged packets the engine may ride out — video
/// conceals (repeating the last good frame / holding black), audio
/// **skips** (drops the packet, never synthesizing silence; 3.C2
/// G3) — before the damage is declared catastrophic and reaches a
/// typed terminal.
///
/// The figure inherits doc 03 §4.3(d)'s bounded-recovery window for
/// corrupt-but-skippable demux data — "attempt resynchronization once
/// within a bounded window [PROPOSAL: 1 MiB or 1000 packets], else
/// fatal `Corrupt`" — applied to the decode stage's per-frame damage
/// (audit M9 / ED-FIX-4). Every concealed-or-skipped unit is
/// observable (a `DamagedData` fact + the coalesced non-fatal
/// `playback.error`), so the budget only draws the line where
/// "playing" would become a lie: a stream whose every packet fails
/// is not playing. ONE budget for both limbs by design (design D-2:
/// the audio lane mirrors the video lane's shape) — the historical
/// name says "video" because W1-08 landed it video-only.
pub const VIDEO_CONCEALMENT_BUDGET: u32 = 1000;

/// Whether `error` is **per-frame damage** — the doc 03 §4.5(d)
/// `CorruptPacket` class the engine isolates on a limb (concealing on
/// video, skipping on audio) instead of tearing the session down
/// (audit M9; the audio lane since 3.C2 G3):
///
/// - [`ErrorKind::Corrupt`] — the bytes of *this* packet are
///   structurally broken or truncated;
/// - [`ErrorKind::Decoder { cause: Bitstream }`] — this packet's
///   entropy data violated the codec grammar.
///
/// Everything else stays fatal on both lanes: `Unsupported` and
/// `ResourceLimit` are deterministic per stream (retrying the next
/// frame cannot help), `BadState` is contract misuse, and a future
/// inter-frame codec that *cannot* continue after damage must not
/// report these kinds for it (doc 03 §4.5(d): such decoders "must
/// document that and emit `Fatal` instead" — today's only video
/// decoder, MJPEG, is stateless per frame by construction, and PCM
/// audio accepts any bytes, so the audio lane is exercised by
/// scripted decoder faults, never by real bytes).
fn per_frame_damage(error: &EngineError) -> bool {
    matches!(
        &error.kind,
        media_types::ErrorKind::Corrupt { .. }
            | media_types::ErrorKind::Decoder {
                cause: media_types::error::DecodeCause::Bitstream
            }
    )
}

/// The typed terminal for a spent concealment budget (D-3 boundary
/// honesty): the run of consecutive per-frame damage exceeded
/// [`VIDEO_CONCEALMENT_BUDGET`], so the recovery attempt budget is
/// exhausted — the last damage rides in the error's `source` chain
/// for triage.
fn concealment_budget_exhausted(failures: u32, last: EngineError) -> EngineError {
    EngineError::new(
        media_types::PipelineStage::Decode,
        media_types::ErrorKind::ResourceLimit {
            limit: media_types::error::LimitKind::RecoveryAttempts,
            budget: u64::from(VIDEO_CONCEALMENT_BUDGET),
            attempted: u64::from(failures),
        },
        media_types::error::ErrorContext::new("decode.concealment.budget"),
    )
    .with_source(last)
}

/// Pipeline-side starvation observability (card 3.B3, design C2 —
/// the engine-owned half; the wire-visible surface stays the 3.B4
/// vocabulary: `session.state-changed` + `session.buffering`). The
/// session folds these into [`EngineStats`](crate::EngineStats)
/// through `Session::absorb_pipeline_starvation` (the one-field
/// snapshot at the tick boundary and at every pipeline-retiring
/// effect).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct StarvationCounters {
    /// Ticks in `Playing` where any limb observed the low watermark,
    /// **pre-debounce** — the honest raw rate of near-empty queues
    /// (design C2).
    pub starved_ticks: u64,
    /// Starve signals actually reported (`TickReport.starved`,
    /// post-debounce) — the pipeline-side upper bound on applied
    /// Underrun transitions; the session's applied-episode count can
    /// only be ≤ this (a higher-priority signal can preempt).
    pub starve_signals: u64,
    /// Ticks the held packet waited before re-offer — the
    /// ingestion-stall metric (a full queue is parking the demuxer).
    pub pending_hold_ticks: u64,
    /// Full-rejection tally on the packets axis ("the packet cap is
    /// the binding cap") — the which-cap-starves-us diagnostic.
    pub full_rejections_packets: u64,
    /// Full-rejection tally on the bytes axis.
    pub full_rejections_bytes: u64,
    /// Full-rejection tally on the duration axis.
    pub full_rejections_duration: u64,
}

/// The wired pipeline: demuxer, limbs, and the tick driver. Exists
/// from session construction (holding the demuxer) until teardown.
pub(crate) struct Pipeline {
    /// The demux stage (plugin-api contract).
    demuxer: Box<dyn Demuxer>,
    /// The wired audio/video/subtitle limbs — at most one per kind
    /// (card 3.A3's wire change: the container-default track of each
    /// kind, never every track).
    limbs: Vec<Limb>,
    /// The demuxed track list (set at wiring time).
    tracks: Vec<Track>,
    /// Which tracks got limbs (selection truth for events).
    selected: Vec<bool>,
    /// A packet a full queue handed back — held for the next tick
    /// (the synchronous form of doc 03 §4.4's parking producer).
    pending: Option<Packet>,
    /// The demuxer returned end-of-stream.
    eof: bool,
    /// The current queue/decoder generation stamp (doc 03 §5
    /// checking point 3 — the demux driver stamps every packet).
    generation: GenerationId,
    /// The frame sink seam.
    consumer: Arc<dyn FrameConsumer>,
    /// The first delivered frame's timestamp, once (doc 03 §4.6
    /// `FirstFrame`).
    first_frame: Option<Stamp>,
    /// The in-flight re-prime observation window (doc 03 §6 step 5 /
    /// card 3.A3 step 5): per-track delivery facts keyed by `TrackId`
    /// (index-keying died with limb swaps), shared by the in-flight
    /// seek and the in-flight per-kind switches — the `owners` bits
    /// are the flows still watching. `None` when nothing is in
    /// flight. Windows are rare (per-command, not per-tick), so the
    /// small allocation on open is the documented events.rs rule-3
    /// exception family.
    window: Option<ObserveWindow>,
    /// Retired-but-cached decoder instances (card 3.A3, D-1): keyed
    /// by `TrackId`, bounded by the source's track count (a finite,
    /// container-declared list) — bounded by construction, no LRU
    /// needed. Dies with the pipeline (epoch invariant S4 — no
    /// cross-source reuse). Keeping MJPEG's multi-MB `DecodeContext`
    /// alive across an A→B→A switch is the point: `configure` is the
    /// codec-api reset point, so re-activation is cheap and honest.
    decoder_cache: Vec<CachedDecoder>,
    /// Frames discarded from a seek's pre-roll presentation hold
    /// (M10, D-4): a superseded or failed seek's held frames are
    /// stale by generation and never present — this is the rollback's
    /// observable cost, handed to the caller by
    /// [`Pipeline::take_preroll_discarded`].
    preroll_discarded: u32,
    /// Frames dropped as stale during the last seek/switch flush
    /// (handed to the caller by [`Pipeline::take_seek_flush_stale`]
    /// for the session's observability totals).
    seek_flush_stale: u32,
    /// The limbs whose decoder newly flushed to `Drained` this tick
    /// (the `playback.end-of-track` candidates, card 3.B4) — a
    /// **reused** scratch (drained by the session after each tick,
    /// capacity retained), so the steady-state tick allocates
    /// nothing (engineering directive D-1).
    ended_limbs: Vec<(MediaKind, TrackId)>,
    /// The starve debounce in force (from the plan at wire time).
    starve_debounce: u32,
    /// Lifetime starvation observability (card 3.B3, design C2).
    starvation: StarvationCounters,
}

/// One parked decoder instance (the per-session decoder cache, card
/// 3.A3 — see [`Pipeline::decoder_cache`]). The parked **codec
/// descriptor** rides the entry (audit F-4): the recycle path's
/// `configure` call reuses the exact descriptor the decoder was built
/// under, decoupled from both the live track list (a cache entry
/// outliving its row was the old fallback's premise) and the
/// registry's validation rules (the retired `new("unknown")` dance
/// could panic if the registry ever rejected the literal).
struct CachedDecoder {
    track: TrackId,
    codec: media_types::ids::CodecDescriptor,
    decoder: Box<dyn Decoder>,
}

/// The open-time wiring report (card 3.A3): how many limbs were
/// wired, and how many decoders the provider **built** vs. the cache
/// **reused** while wiring them — the open-time half of the
/// decoder-recycling observability (`EngineStats::decoders_built` /
/// `decoders_reused`; the switch-time half rides `SwitchOutcome`).
#[derive(Default)]
pub(crate) struct WireOutcome {
    pub wired: usize,
    pub decoders_built: u32,
    pub decoders_reused: u32,
}

/// Which in-flight flows still watch the observation window. Bitmask
/// (a window can serve a seek and both kind-switches at once).
#[derive(Clone, Copy, PartialEq, Eq)]
struct WindowOwner(u8);

impl WindowOwner {
    const SEEK: WindowOwner = WindowOwner(1);
    const SWITCH_AUDIO: WindowOwner = WindowOwner(2);
    const SWITCH_VIDEO: WindowOwner = WindowOwner(4);

    fn union(self, other: WindowOwner) -> WindowOwner {
        WindowOwner(self.0 | other.0)
    }

    fn without(self, other: WindowOwner) -> WindowOwner {
        WindowOwner(self.0 & !other.0)
    }

    fn contains(self, other: WindowOwner) -> bool {
        self.0 & other.0 != 0
    }

    fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The switch bit for a limb kind (audio/video).
    fn switch_for(kind: MediaKind) -> WindowOwner {
        match kind {
            MediaKind::Audio => WindowOwner::SWITCH_AUDIO,
            _ => WindowOwner::SWITCH_VIDEO,
        }
    }
}

/// Per-track facts the re-prime phases track (doc 03 §6 step 5 for
/// seeks, card 3.A3 step 5 for switches): has this track delivered a
/// frame of the watched generation, and what was its first frame's
/// pts (the anchor truth). Keyed by `TrackId` so limb swaps re-arm
/// instead of invalidating.
pub(crate) struct ObserveWindow {
    /// Per-track delivery facts (find-on-observe: windows are rare,
    /// never per-tick, and the limb count is ≤ 3).
    saw: Vec<WatchEntry>,
    /// The first **audio** limb's first delivered frame, in ns — the
    /// seek step-6 anchor truth (`None` until an audio frame
    /// arrives).
    first_audio_ns: Option<i64>,
    /// The flows still watching (seek + per-kind switches).
    owners: WindowOwner,
    /// The M10 presentation hold, keyed by track (limb swaps re-key
    /// it like the watches): frames decoded during a **seek's**
    /// pre-roll, in arrival order, per tracked limb. Bounded by
    /// `hold_cap` (a hard per-track capacity — D-1); the decode slice
    /// backpressures at the cap (packets stay in the bounded queues),
    /// so the only path past it is a single decode call's burst,
    /// which is dropped and counted, never buffered unboundedly. A
    /// switch's re-prime never holds — its frames deliver straight
    /// through (card 3.A3 step 7: the frame's delivery IS the
    /// completion fact).
    hold: Vec<HoldSlot>,
    /// The per-track hard capacity of the hold (the pre-roll budget:
    /// the decode-slice bound the seek flow carries).
    hold_cap: usize,
}

/// One watched track's delivery facts.
struct WatchEntry {
    track: TrackId,
    saw_frame: bool,
    /// The first delivered frame's pts (ns) — the switch step-6
    /// anchor truth for the new track.
    first_pts_ns: Option<i64>,
}

/// One tracked track's M10 presentation-hold slot.
struct HoldSlot {
    track: TrackId,
    /// Pre-allocated once per seek (not per tick — D-1): pushes
    /// within the capacity never allocate.
    frames: Vec<Frame>,
}

impl ObserveWindow {
    /// Observe one limb's decode slice: frames delivered, and the
    /// first frame's stamp.
    fn observe(&mut self, track: TrackId, kind: MediaKind, frames: u32, first: Option<Stamp>) {
        if frames == 0 {
            return;
        }
        if let Some(entry) = self.saw.iter_mut().find(|entry| entry.track == track) {
            entry.saw_frame = true;
            if entry.first_pts_ns.is_none()
                && let Some(stamp) = first
                && let Some(ns) = stamp_ns(stamp)
            {
                entry.first_pts_ns = Some(ns);
            }
        }
        if kind == MediaKind::Audio
            && self.first_audio_ns.is_none()
            && let Some(stamp) = first
            && let Some(ns) = stamp_ns(stamp)
        {
            self.first_audio_ns = Some(ns);
        }
    }

    /// The first-frame pts recorded for `track` (the anchor truth).
    fn first_pts_ns(&self, track: TrackId) -> Option<i64> {
        self.saw
            .iter()
            .find(|entry| entry.track == track)
            .and_then(|entry| entry.first_pts_ns)
    }

    /// Whether `track` delivered ≥ 1 frame under the watched
    /// generation.
    fn saw(&self, track: TrackId) -> bool {
        self.saw
            .iter()
            .any(|entry| entry.track == track && entry.saw_frame)
    }

    /// Total frames currently held across all slots.
    fn held_count(&self) -> u32 {
        self.hold.iter().fold(0u32, |sum, slot| {
            sum.saturating_add(slot.frames.len() as u32)
        })
    }

    /// The M10 hold slot one decode slice routes its frames through,
    /// by track id (limb swaps re-key the hold like the watches).
    /// `None` when no slot exists for the track — a switch's re-prime
    /// never creates one (its frames deliver straight through), and
    /// the seek flush's residue sink is built without a hold at all.
    fn hold_slot(&mut self, track: TrackId) -> Option<PrerollHoldSlot<'_>> {
        let cap = self.hold_cap;
        self.hold
            .iter_mut()
            .find(|slot| slot.track == track)
            .map(|slot| PrerollHoldSlot {
                frames: &mut slot.frames,
                cap,
            })
    }
}

/// The pre-roll status of the in-flight seek (doc 03 §6 steps 5–6).
pub(crate) enum SeekPreroll {
    /// Still priming: some enabled limb has not delivered a frame of
    /// the new generation (and could still).
    NotReady,
    /// Every enabled limb delivered (or can provably never — its
    /// queue is empty at end-of-stream), and at least one did. The
    /// payload is the step-6 anchor truth: the first audio frame's
    /// own timestamp, when one was delivered.
    Ready {
        /// The first audio frame's pts (ns), when audio delivered.
        audio_anchor_ns: Option<i64>,
    },
}

/// The completion status of an in-flight track switch (card 3.A3
/// steps 5–6). The payload is the step-6 anchor truth: the new
/// track's first delivered frame's own pts (audio switches
/// re-anchor the clock on it; video switches ignore it).
pub(crate) enum SwitchPreroll {
    /// The new limb has not delivered a frame under the switch's
    /// generation yet (and could still).
    NotReady,
    /// The switch's selection landed: the new track delivered (or
    /// provably never could / was deselected).
    Ready {
        /// The new track's first frame's pts (ns), when it delivered.
        anchor_ns: Option<i64>,
    },
}

impl Pipeline {
    /// The `Opening`-phase pipeline: demuxer + sink seam, no limbs
    /// yet (they are wired after `Demuxer::start` reports tracks).
    pub(crate) fn new(demuxer: Box<dyn Demuxer>, consumer: Arc<dyn FrameConsumer>) -> Pipeline {
        Pipeline {
            demuxer,
            limbs: Vec::new(),
            tracks: Vec::new(),
            selected: Vec::new(),
            pending: None,
            eof: false,
            generation: GenerationId::FIRST,
            consumer,
            first_frame: None,
            window: None,
            decoder_cache: Vec::new(),
            preroll_discarded: 0,
            seek_flush_stale: 0,
            ended_limbs: Vec::new(),
            starve_debounce: DEFAULT_STARVE_DEBOUNCE_TICKS,
            starvation: StarvationCounters::default(),
        }
    }

    /// The demuxed tracks and selection flags (valid after
    /// [`Pipeline::wire`]).
    pub(crate) fn tracks(&self) -> (&[Track], &[bool]) {
        (&self.tracks, &self.selected)
    }

    /// Wire the limbs for a started demuxer (card 3.A3's selection
    /// change): **one limb per kind** — the container-default track
    /// (`TrackFlags::default_on`, else first-of-kind in stream order)
    /// of each audio/video/subtitle kind. Every other track stays
    /// `selected: false` (its packets are dropped at push — the track
    /// list event already tells the truth about selection). This
    /// fixes the v0 decode-all-tracks waste and the first-audio
    /// anchor ambiguity, and is the precondition for the switch
    /// flow's per-kind limb swap.
    ///
    /// The subtitle kind is **optional**: a provider that cannot
    /// build a subtitle decoder deselects that track instead of
    /// failing the open (the class-S seam — subtitles are never the
    /// reason playback refuses to start; an A/V decoder failure
    /// remains fatal per the recorded contract).
    ///
    /// Returns the wire outcome — the wired limb count (zero means
    /// nothing is decodable; the session turns that into the typed
    /// `NoDecodableTrack` failure) plus the decoder-lifecycle counts
    /// for the session's `EngineStats` (the open-time builds the
    /// recycling proof counts alongside the switch-time ones).
    ///
    /// # Errors
    /// The provider's typed error, propagated verbatim (a selection
    /// that fails after promising is fatal, not a skip).
    pub(crate) fn wire(
        &mut self,
        tracks: Vec<Track>,
        provider: &dyn DecoderProvider,
        plan: &QueuePlan,
        generation: GenerationId,
    ) -> Result<WireOutcome, EngineError> {
        self.generation = generation;
        self.tracks = tracks;
        self.selected = vec![false; self.tracks.len()];
        self.starve_debounce = plan.starve_debounce_ticks.max(1);
        let mut outcome = WireOutcome::default();
        for wired in [WiredKind::Video, WiredKind::Audio, WiredKind::Subtitle] {
            let Some(track) = crate::tracks::default_track(&self.tracks, wired.as_media_kind())
            else {
                continue;
            };
            let track = track.clone();
            match self.obtain_decoder(&track, generation, provider) {
                Ok((decoder, reused)) => {
                    if reused {
                        outcome.decoders_reused += 1;
                    } else {
                        outcome.decoders_built += 1;
                    }
                    if let Some(index) = self.tracks.iter().position(|t| t.id == track.id) {
                        self.selected[index] = true;
                    }
                    self.limbs
                        .push(self.build_limb(track, wired, decoder, plan, generation));
                    outcome.wired += 1;
                }
                Err(_error) if wired == WiredKind::Subtitle => {
                    // The optional kind: deselect, never fail the
                    // open (the class-S seam's open-time half).
                    continue;
                }
                Err(error) => return Err(error),
            }
        }
        Ok(outcome)
    }

    /// Build one wired limb: a fresh queue from the plan, published
    /// at `generation` (doc 03 §5 checking point 4 — a fresh queue
    /// publishes FIRST, so this flush is a no-op at session start and
    /// the standing boundary for every later mint).
    ///
    /// The lane caps come from the total
    /// [`QueuePlan::for_kind(WiredKind)`](QueuePlan::for_kind): the
    /// kind parameter is lane-typed, so a lane-less kind is
    /// unrepresentable here (audit F-1 — the `expect` + `allow` this
    /// replaces was the crate's one production panic surface).
    fn build_limb(
        &self,
        track: Track,
        kind: WiredKind,
        decoder: Box<dyn Decoder>,
        plan: &QueuePlan,
        generation: GenerationId,
    ) -> Limb {
        let limits = plan.for_kind(kind);
        let thresholds = plan.thresholds(&limits);
        let queue = Arc::new(BoundedQueue::new(limits, thresholds));
        let high_packets = thresholds.high.packets.max(1);
        let low_packets = thresholds.low.packets;
        let _ = queue.flush(generation);
        Limb {
            track,
            queue,
            decoder,
            drained: false,
            high_packets,
            low_packets,
            low_streak: 0,
            episode_open: false,
            generation,
            first_frame_pending: false,
            last_frame: None,
            conceal_failures: 0,
        }
    }

    /// The demux stage boundary during `Opening` (v0's probe):
    /// parse the container header and enumerate tracks.
    ///
    /// # Errors
    /// The demuxer's typed error (unrecognized container, corrupt
    /// header).
    pub(crate) fn start_demuxer(&mut self) -> Result<Vec<Track>, EngineError> {
        self.demuxer.start()
    }

    /// Teardown (doc 03 §5's order, synchronous form): flush every
    /// queue with `new_generation` (producers of older stamps are
    /// refused from the boundary on — packet-queue Q2), close them
    /// (parked producers would wake with `Closed`), then drop the
    /// limbs and the parked decoder cache. The caller mints and
    /// interrupts *before* calling this. An in-flight window's held
    /// pre-roll frames die with the pipeline — counted, never
    /// silent (M10).
    pub(crate) fn teardown(&mut self, new_generation: GenerationId) {
        if let Some(window) = self.window.take() {
            self.preroll_discarded = self.preroll_discarded.saturating_add(window.held_count());
        }
        for limb in &self.limbs {
            let _ = limb.queue.flush(new_generation);
            limb.queue.close();
        }
        self.limbs.clear();
        self.decoder_cache.clear();
        self.pending = None;
    }

    /// The demuxer's declared seek capability (doc 03 §6 step 1,
    /// delegated to the plugin-api hook).
    pub(crate) fn seekability(&self) -> SeekSupport {
        self.demuxer.seekability()
    }

    /// The greatest declared track extent, in nanoseconds — the
    /// seek target's clamp bound (doc 05 §2.4: absolute/relative
    /// targets clamp to `[0, duration]` when the duration is known).
    /// `None` when no track declares a present span end (the
    /// demuxer's own range check is the backstop then).
    pub(crate) fn duration_ns(&self) -> Option<i64> {
        self.tracks
            .iter()
            .filter_map(|track| stamp_ns(track.timing.span.end))
            .max()
    }

    /// Seek step 3 (doc 03 §6): flush the pipeline for `generation`
    /// — the mint the row's `BumpPipeline` just published. Unlike
    /// [`Pipeline::teardown`] the queues stay **open** (the seek
    /// re-fills them): every queue is flushed at the new boundary
    /// (stale residents die — packet-queue Q2), the held demux
    /// packet is dropped (it predates the mint), the end-of-stream
    /// flag resets (a backward seek revives an exhausted stream),
    /// every decoder — **including the parked cache entries** (card
    /// 3.A3: a cached decoder holding pre-seek reorder frames must
    /// not replay them into a post-seek recycle) — is flushed to
    /// `Drained` through a generation-checking sink (its
    /// old-generation reorder buffers are dropped and counted — doc
    /// 03 §5 checking point 5 — never presented) and then
    /// reconfigured for the new generation (the codec-api seek
    /// contract, `decode.rs`'s "drain path after seek generation
    /// switches"). A fresh observation window opens over the current
    /// limbs **with its M10 presentation hold** (per-track,
    /// hard-capped at `hold_cap` — the pre-roll budget).
    ///
    /// A superseded seek's still-held frames are discarded here,
    /// counted (D-4: they are stale by generation — the flush that
    /// opens this window already invalidated their content).
    ///
    /// A flush budget that exhausts before a decoder answers
    /// `Drained` is **not** a failure: the straggler frames it still
    /// holds are dropped at the sink's generation check in later
    /// decode slices (the checkpoint is the guarantee, the budget
    /// only bounds the work).
    ///
    /// # Errors
    /// The decoder's typed error (a flush or reconfigure refusal of
    /// the new generation) — the seek fails honestly per D4 and the
    /// pipeline limps on (the next decode error surfaces through
    /// the normal fatal path).
    pub(crate) fn seek_flush(
        &mut self,
        generation: GenerationId,
        flush_budget: u32,
        hold_cap: usize,
    ) -> Result<(), EngineError> {
        // Invalidate the seek's claim on any previous observation
        // window first: a half-completed flush must not leave a
        // superseded seek's pre-roll progress armed (it would answer
        // Ready from stale data and keep the Paused decode carve-out
        // alive). The superseded seek's held frames die with it —
        // discarded, counted, never presented (M10/D-4). A switch
        // still in flight keeps its claim — its watched limb is a
        // current limb and survives the rebuild.
        self.end_seek();
        for limb in &self.limbs {
            let _ = limb.queue.flush(generation);
        }
        self.pending = None;
        self.eof = false;
        self.generation = generation;
        // Starve-debounce state is per-pipeline-generation (design
        // B4): the seek's queues are empty and refilling — streaks and
        // episodes reset with them.
        for limb in &mut self.limbs {
            limb.low_streak = 0;
            limb.episode_open = false;
        }
        let consumer = Arc::clone(&self.consumer);
        for limb in &mut self.limbs {
            let track_id = limb.track.id;
            let mut sink = LimbSink {
                consumer: Arc::clone(&consumer),
                track: track_id,
                generation,
                frames: 0,
                first: None,
                stale_dropped: 0,
                // No hold: the flush's emissions are the OLD
                // generation's reorder residue — the generation check
                // drops them, they are never captured (M10). No
                // concealment latch either: a residue frame is never
                // repeated (M9).
                hold: None,
                last: None,
                concealed: 0,
            };
            for _ in 0..flush_budget {
                match limb.decoder.flush(&mut sink) {
                    Ok(DecodeOutcome::Drained) => break,
                    Ok(_) => continue,
                    Err(error) => {
                        // The frames this limb already purged are
                        // still gone — count them before refusing.
                        self.seek_flush_stale =
                            self.seek_flush_stale.saturating_add(sink.stale_dropped);
                        return Err(error);
                    }
                }
            }
            self.seek_flush_stale = self.seek_flush_stale.saturating_add(sink.stale_dropped);
            let config = DecoderConfig::new(
                limb.track.codec.clone(),
                generation,
                ThreadBudget::default(),
                None,
                None,
            );
            limb.decoder.configure(&config)?;
            limb.drained = false;
            limb.generation = generation;
        }
        // The parked decoders: flush-to-Drained (bounded, counted)
        // and reconfigure for the new generation — they were not
        // asked to decode anything, but a future inter-frame codec
        // parked mid-reorder must not replay old frames on recycle.
        for cached in &mut self.decoder_cache {
            let mut sink = LimbSink {
                consumer: Arc::clone(&consumer),
                track: cached.track,
                generation,
                frames: 0,
                first: None,
                stale_dropped: 0,
                // No hold, no latch: the parked decoder's residue is
                // old-generation work — dropped at the generation
                // check, never captured, never repeated (M10/M9).
                hold: None,
                last: None,
                concealed: 0,
            };
            for _ in 0..flush_budget {
                match cached.decoder.flush(&mut sink) {
                    Ok(DecodeOutcome::Drained) => break,
                    Ok(_) => continue,
                    Err(error) => {
                        self.seek_flush_stale =
                            self.seek_flush_stale.saturating_add(sink.stale_dropped);
                        return Err(error);
                    }
                }
            }
            self.seek_flush_stale = self.seek_flush_stale.saturating_add(sink.stale_dropped);
            let config = DecoderConfig::new(
                cached.codec.clone(),
                generation,
                ThreadBudget::default(),
                None,
                None,
            );
            cached.decoder.configure(&config)?;
        }
        self.window_open_seek(hold_cap);
        Ok(())
    }

    /// Open (or re-arm) the observation window for the in-flight
    /// seek: fresh per-track entries over the current limbs, the
    /// seek's owner bit set (a switch still in flight keeps its bit
    /// and its watched limb — the new limb of a swapped kind is a
    /// current limb, so the rebuild covers it), and the M10
    /// presentation hold rebuilt over the current limbs at
    /// `hold_cap` (the superseded seek's held frames were already
    /// discarded by [`Pipeline::end_seek`]).
    fn window_open_seek(&mut self, hold_cap: usize) {
        let entries = self
            .limbs
            .iter()
            .map(|limb| WatchEntry {
                track: limb.track.id,
                saw_frame: false,
                first_pts_ns: None,
            })
            .collect::<Vec<_>>();
        // Pre-allocated once per seek (not per tick — D-1): pushes
        // within the capacity never allocate.
        let hold = self
            .limbs
            .iter()
            .map(|limb| HoldSlot {
                track: limb.track.id,
                frames: Vec::with_capacity(hold_cap),
            })
            .collect::<Vec<_>>();
        match &mut self.window {
            Some(window) => {
                window.saw = entries;
                window.first_audio_ns = None;
                window.hold = hold;
                window.hold_cap = hold_cap;
                window.owners = window.owners.union(WindowOwner::SEEK);
            }
            None => {
                self.window = Some(ObserveWindow {
                    saw: entries,
                    first_audio_ns: None,
                    hold,
                    hold_cap,
                    owners: WindowOwner::SEEK,
                });
            }
        }
    }

    /// Arm the window for an in-flight switch of `kind`: the owner
    /// bit is claimed and the new track (when one was selected) is
    /// watched — added to the existing entries so an in-flight seek
    /// keeps its own watches (the seek completes on the new track's
    /// first frame), and its hold covers the swapped limb (the new
    /// track's pre-roll frames are seek pre-roll frames too — held,
    /// not presented). A switch flying alone holds nothing: its
    /// frames deliver straight through (card 3.A3 step 7).
    fn window_arm_switch(&mut self, kind: MediaKind, track: Option<TrackId>) {
        let owner = WindowOwner::switch_for(kind);
        match &mut self.window {
            Some(window) => {
                window.owners = window.owners.union(owner);
                if let Some(track) = track {
                    if !window.saw.iter().any(|entry| entry.track == track) {
                        window.saw.push(WatchEntry {
                            track,
                            saw_frame: false,
                            first_pts_ns: None,
                        });
                    }
                    if window.owners.contains(WindowOwner::SEEK)
                        && !window.hold.iter().any(|slot| slot.track == track)
                    {
                        let cap = window.hold_cap;
                        window.hold.push(HoldSlot {
                            track,
                            frames: Vec::with_capacity(cap),
                        });
                    }
                }
            }
            None => {
                let mut saw = Vec::new();
                if let Some(track) = track {
                    saw.push(WatchEntry {
                        track,
                        saw_frame: false,
                        first_pts_ns: None,
                    });
                }
                self.window = Some(ObserveWindow {
                    saw,
                    first_audio_ns: None,
                    hold: Vec::new(),
                    hold_cap: 0,
                    owners: owner,
                });
            }
        }
    }

    /// Release `owner`'s claim on the observation window; the window
    /// itself dies with the last owner.
    fn window_end(&mut self, owner: WindowOwner) {
        if let Some(window) = &mut self.window {
            window.owners = window.owners.without(owner);
            if window.owners.is_empty() {
                self.window = None;
            }
        }
    }

    /// Whether any in-flight flow still watches the window — the
    /// Paused decode carve-out's gate (a seek **or** a switch
    /// accepted while `Paused` must decode to complete).
    fn window_live(&self) -> bool {
        self.window.is_some()
    }

    /// Frames dropped as stale during the last seek flush (doc 03
    /// §5 checking point 5 — the decoder-output stale purge), handed
    /// to the caller once for its observability totals, then reset.
    pub(crate) fn take_seek_flush_stale(&mut self) -> u32 {
        std::mem::take(&mut self.seek_flush_stale)
    }

    /// Frames discarded from the pre-roll presentation hold (M10,
    /// D-4) — superseded or failed seeks' held frames, plus any
    /// frames the release itself had to refuse — handed to the caller
    /// once for its observability totals, then reset.
    pub(crate) fn take_preroll_discarded(&mut self) -> u32 {
        std::mem::take(&mut self.preroll_discarded)
    }

    /// Seek step 4 (doc 03 §6): position the demuxer at its
    /// keyframe-safe position ≤ `target` and report where the stream
    /// will resume. Invariant D4 lives in the demuxer (compute-then-
    /// commit): on `Err` it stands where it stood.
    ///
    /// # Errors
    /// The demuxer's typed error (out of range, bounded-scan budget,
    /// damage) — with the demuxer unmoved.
    pub(crate) fn demuxer_reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        self.demuxer.reanchor(target)
    }

    /// Seek step 5's completion observation: the pre-roll status of
    /// the in-flight seek. `Ready` when every enabled limb has
    /// delivered ≥ 1 frame of the current generation, or can
    /// provably never deliver (end-of-stream with nothing resident
    /// and the decoder drained). A landing where **every** limb
    /// provably never delivers still completes — the alternative is
    /// a seek that never reports; the drained signal then ends the
    /// media honestly at the landing. The payload is the step-6
    /// anchor truth: the first audio frame's own timestamp, when an
    /// audio frame was delivered.
    pub(crate) fn seek_preroll(&self) -> SeekPreroll {
        let Some(window) = self.window.as_ref() else {
            return SeekPreroll::NotReady;
        };
        if !window.owners.contains(WindowOwner::SEEK) {
            return SeekPreroll::NotReady;
        }
        if self.limbs.is_empty() {
            return SeekPreroll::NotReady;
        }
        for limb in &self.limbs {
            if window.saw(limb.track.id) {
                continue;
            }
            if !self.can_never_deliver(limb) {
                return SeekPreroll::NotReady;
            }
        }
        SeekPreroll::Ready {
            audio_anchor_ns: window.first_audio_ns,
        }
    }

    /// M10 — release the pre-roll hold at the seek terminal (doc 03
    /// §6 steps 6–7). The caller has already re-anchored the clock on
    /// the landing truth and published the seek's generation, so the
    /// held frames may finally present: they leave through the same
    /// [`FrameConsumer`] seam they would have used without the hold,
    /// in **pts order** (stable among equals — arrival order), every
    /// tracked limb merged. The seek's claim on the observation
    /// window ends with the release (a switch still in flight keeps
    /// the window for its own completion — its re-prime never held,
    /// so nothing of its own can leak here).
    ///
    /// A consumer refusal here is a bounded, counted loss (the
    /// discard counter), never a swallowed error and never an
    /// un-report of the already-emitted terminal: the seek completed
    /// truthfully; the sink's refusal is a separate degraded fact the
    /// host observes through the count (doc 03 §4.10 d — presentation
    /// faults degrade, they do not unwind playback state).
    pub(crate) fn release_preroll_hold(&mut self) {
        // Merge the per-track holds, then order by pts across limbs
        // (once per seek — the merge/sort is not a hot path). The
        // track id rides along so a released video frame can latch
        // the concealment cache (M9) on the limb that owns it — by
        // id, because a switch may have swapped limbs since the
        // capture (card 3.A3).
        let mut held: Vec<(TrackId, Frame)> = Vec::new();
        if let Some(window) = self.window.as_mut() {
            for slot in window.hold.drain(..) {
                for frame in slot.frames {
                    held.push((slot.track, frame));
                }
            }
            window.owners = window.owners.without(WindowOwner::SEEK);
            if window.owners.is_empty() {
                self.window = None;
            }
        }
        // Missing stamps sort last (arrival order among equals — the
        // stable sort keeps it); a present stamp orders by its
        // nanoseconds on the media base.
        held.sort_by_key(|(_, frame)| stamp_ns(frame.pts()).unwrap_or(i64::MAX));
        let generation = self.generation;
        for (track, frame) in held {
            // Defense in depth: a frame that went stale between its
            // capture and the terminal (a threaded future's race)
            // never presents — dropped and counted, not guessed
            // around (doc 03 §5 checking point 5).
            if frame.is_stale(generation) {
                self.preroll_discarded = self.preroll_discarded.saturating_add(1);
                continue;
            }
            // The release hand-off is the presentation event: latch
            // the concealment cache exactly as a live `accept` would
            // have (a clone of fixed-size fields plus one
            // `SharedBytes` reference bump — D-1), so a damaged slot
            // after the terminal repeats the new landing's last
            // frame, not nothing.
            let latched = if matches!(frame, Frame::Video(_)) {
                Some(frame.clone())
            } else {
                None
            };
            if self.consumer.accept(frame, track).is_err() {
                self.preroll_discarded = self.preroll_discarded.saturating_add(1);
            } else if let Some(frame) = latched
                && let Some(limb) = self.limbs.iter_mut().find(|limb| limb.track.id == track)
            {
                limb.last_frame = Some(frame);
            }
        }
    }

    /// End the in-flight seek's observation window — completion,
    /// failure, or supersession: releases the seek's claim (and with
    /// the last owner, the Paused decode carve-out of
    /// [`Pipeline::tick`]). A switch still in flight keeps the window
    /// alive for its own completion. Any frames still in the hold are
    /// **discarded and counted** (M10, D-4): a seek that will never
    /// complete owns nothing presentable — its frames are stale by
    /// generation. (After [`Pipeline::release_preroll_hold`] the hold
    /// is already drained, so this adds zero.)
    pub(crate) fn end_seek(&mut self) {
        if let Some(window) = &mut self.window {
            let held = window.held_count();
            self.preroll_discarded = self.preroll_discarded.saturating_add(held);
            window.hold.clear();
            window.owners = window.owners.without(WindowOwner::SEEK);
            if window.owners.is_empty() {
                self.window = None;
            }
        }
    }

    /// Card 3.A3 step 5's completion observation for the in-flight
    /// switch of `kind`: `Ready` once the newly-selected track has
    /// delivered ≥ 1 frame under the switch's generation, or can
    /// provably never (EOF, nothing resident, decoder `Drained`), or
    /// was deselected (nothing to wait for — the terminal fires on
    /// the next completion check). The payload is the step-6 anchor
    /// truth: the new track's first delivered frame's own pts.
    pub(crate) fn switch_preroll(
        &self,
        kind: MediaKind,
        selected: Option<TrackId>,
    ) -> SwitchPreroll {
        let Some(selected) = selected else {
            return SwitchPreroll::Ready { anchor_ns: None };
        };
        let Some(window) = self.window.as_ref() else {
            return SwitchPreroll::NotReady;
        };
        if !window.owners.contains(WindowOwner::switch_for(kind)) {
            return SwitchPreroll::NotReady;
        }
        if window.saw(selected) {
            return SwitchPreroll::Ready {
                anchor_ns: window.first_pts_ns(selected),
            };
        }
        match self.limbs.iter().find(|limb| limb.track.id == selected) {
            // No limb (nothing wireable): nothing can ever deliver —
            // the switch completes honestly rather than hanging.
            None => SwitchPreroll::Ready { anchor_ns: None },
            Some(limb) if self.can_never_deliver(limb) => SwitchPreroll::Ready { anchor_ns: None },
            Some(_) => SwitchPreroll::NotReady,
        }
    }

    /// End the in-flight switch's observation claim (completion or
    /// supersession).
    pub(crate) fn end_switch(&mut self, kind: MediaKind) {
        self.window_end(WindowOwner::switch_for(kind));
    }

    /// Obtain a decoder for `track` at `generation` — the decoder
    /// cache first (card 3.A3, D-1), the provider second. A cache hit
    /// is re-`configure`d (the codec-api reset point — for both
    /// landed codecs configure is trivial; for a future inter-frame
    /// codec it is the documented full-state-reset seam).
    ///
    /// # Errors
    /// The provider's typed error on a miss; a cached decoder's
    /// configure refusal on a hit. Either way the caller decides
    /// fatality (A/V switches: fatal; the subtitle kind's open-time
    /// path: deselect).
    fn obtain_decoder(
        &mut self,
        track: &Track,
        generation: GenerationId,
        provider: &dyn DecoderProvider,
    ) -> Result<(Box<dyn Decoder>, bool), EngineError> {
        if let Some(position) = self
            .decoder_cache
            .iter()
            .position(|cached| cached.track == track.id)
        {
            let mut cached = self.decoder_cache.swap_remove(position);
            let config = DecoderConfig::new(
                track.codec.clone(),
                generation,
                ThreadBudget::default(),
                None,
                None,
            );
            cached.decoder.configure(&config)?;
            return Ok((cached.decoder, true));
        }
        let config = DecoderConfig::new(
            track.codec.clone(),
            generation,
            ThreadBudget::default(),
            None,
            None,
        );
        let decoder = provider.decoder_for(track, &config)?;
        Ok((decoder, false))
    }

    /// Card 3.A3 step 3 — the limb swap for `kind` under
    /// `generation`, **prepare-before-commit** (the D4 discipline
    /// applied to limbs):
    ///
    /// 1. *Obtain the new decoder first* (cache hit ⇒ configure as
    ///    reset; miss ⇒ the provider — a provider failure leaves the
    ///    old limb untouched, so no intermediate state exists; the
    ///    caller treats the failure as fatal per the wire()
    ///    contract).
    /// 2. *Retire the old limb of the kind*: flush its queue at
    ///    `generation` (residents die wholesale — packet-queue Q2),
    ///    close it, drain its decoder to `Drained` through a
    ///    generation-checking sink (bounded by `flush_budget`, stale
    ///    drops counted), then park the decoder **instance** in the
    ///    cache. A retired limb is not "newly drained" — no
    ///    `end-of-track` fires for it.
    /// 3. *Install the new limb*: fresh queue published at
    ///    `generation`, floor set, `first_frame_pending` armed (the
    ///    starvation carve-out of [`Pipeline::tick`]).
    /// 4. A held demux packet of the retired track is dropped (it
    ///    predates the mint; the new queue would refuse it as stale
    ///    anyway).
    /// 5. The demux stamp advances to `generation` — every packet
    ///    from the next tick on is stamped with it, including the
    ///    unchanged kind's. The unchanged kind's queue is **not**
    ///    flushed and its limb floor does not move: that is the
    ///    entire UX point (video keeps playing through an audio
    ///    switch).
    /// 6. The observation window is armed for the switch (and the new
    ///    track watched — an in-flight seek's window re-arms for the
    ///    swapped limb, so the seek completes on the new track's
    ///    first frame).
    ///
    /// `new_track == None` is a deselect: retire only.
    ///
    /// # Errors
    /// See [`Pipeline::obtain_decoder`] (fatal for A/V kinds; the
    /// class-S subtitle path is the caller's policy decision).
    pub(crate) fn switch_limbs(
        &mut self,
        generation: GenerationId,
        kind: WiredKind,
        new_track: Option<TrackId>,
        provider: &dyn DecoderProvider,
        plan: &QueuePlan,
        flush_budget: u32,
    ) -> Result<crate::tracks::SwitchOutcome, EngineError> {
        let mut outcome = crate::tracks::SwitchOutcome::default();
        // 1. Prepare: the new decoder exists before anything retires.
        let prepared = match new_track {
            Some(id) => match self.tracks.iter().find(|track| track.id == id) {
                Some(track) => {
                    let track = track.clone();
                    let (decoder, reused) = self.obtain_decoder(&track, generation, provider)?;
                    outcome.decoder_reused = reused;
                    outcome.decoder_built = !reused;
                    Some((track, decoder))
                }
                // Unreachable through the command path (resolution
                // checked the live list); honored honestly.
                None => None,
            },
            None => None,
        };
        // 2. Retire the old limb of the kind.
        if let Some(position) = self
            .limbs
            .iter()
            .position(|limb| limb.track.kind == kind.as_media_kind())
        {
            let mut limb = self.limbs.swap_remove(position);
            let retired = limb.track.id;
            let _ = limb.queue.flush(generation);
            limb.queue.close();
            let consumer = Arc::clone(&self.consumer);
            let mut sink = LimbSink {
                consumer,
                track: retired,
                generation,
                frames: 0,
                first: None,
                stale_dropped: 0,
                // No hold, no latch: the retired limb's residue is
                // old-generation work — dropped at the generation
                // check, never captured, never repeated (M10/M9).
                hold: None,
                last: None,
                concealed: 0,
            };
            for _ in 0..flush_budget {
                match limb.decoder.flush(&mut sink) {
                    Ok(DecodeOutcome::Drained) => break,
                    Ok(_) => continue,
                    // A drain refusal still parks the instance — the
                    // sink checkpoint is the guarantee (stale frames
                    // cannot present), the budget only bounds the
                    // work. Count what was purged and carry on.
                    Err(_) => break,
                }
            }
            self.seek_flush_stale = self.seek_flush_stale.saturating_add(sink.stale_dropped);
            self.decoder_cache.push(CachedDecoder {
                track: retired,
                // Audit F-4: the parked codec IS the recycle path's
                // descriptor — captured at park time, never re-derived
                // from the track list or a fabricated "unknown"
                // literal.
                codec: limb.track.codec.clone(),
                decoder: limb.decoder,
            });
            // Selection truth + the held packet of the retired track.
            if let Some(index) = self.tracks.iter().position(|track| track.id == retired) {
                self.selected[index] = false;
            }
            if self
                .pending
                .as_ref()
                .is_some_and(|packet| packet.track == retired)
            {
                self.pending = None;
            }
        }
        // 3. Install the new limb.
        if let Some((track, decoder)) = prepared {
            if let Some(index) = self
                .tracks
                .iter()
                .position(|candidate| candidate.id == track.id)
            {
                self.selected[index] = true;
            }
            let mut limb = self.build_limb(track.clone(), kind, decoder, plan, generation);
            limb.first_frame_pending = true;
            let watched = track.id;
            self.limbs.push(limb);
            // 5. The demux stamp advances with the mint.
            self.generation = generation;
            // 6. Arm the window.
            self.window_arm_switch(kind.as_media_kind(), Some(watched));
        } else {
            self.generation = generation;
            self.window_arm_switch(kind.as_media_kind(), None);
        }
        outcome.stale_frames_flushed = self.take_seek_flush_stale();
        Ok(outcome)
    }

    /// The limbs whose decoders newly flushed to `Drained` during
    /// the last tick (the `playback.end-of-track` candidates, card
    /// 3.B4), draining the reused scratch — the `Drain` iterator
    /// yields the pairs and **keeps the capacity** (zero
    /// steady-state allocation, D-1).
    pub(crate) fn take_ended_limbs(&mut self) -> std::vec::Drain<'_, (MediaKind, TrackId)> {
        self.ended_limbs.drain(..)
    }

    /// Whether a limb can provably never deliver another frame:
    /// end-of-stream, nothing held, nothing resident, and the decoder
    /// already flushed to `Drained` (the seek-flush reset both flags,
    /// so "never" is a fact about the post-seek stream, not the
    /// pre-seek state).
    fn can_never_deliver(&self, limb: &Limb) -> bool {
        self.eof && self.pending.is_none() && limb.drained && limb.queue.usage().packets == 0
    }

    /// Whether `limb` paces the session's progress signals (card
    /// 3.A3):
    ///
    /// - **Subtitle limbs never pace** — subtitle packets are sparse
    ///   and latency-tolerant (the [`QueuePlan`] note); an empty
    ///   subtitle queue is not an underrun and must not gate priming
    ///   (VLC parity: a sparse text stream never triggers
    ///   rebuffering).
    /// - A limb installed by a switch whose queue has not yet
    ///   recovered to its low watermark does not pace — the S2.5
    ///   carve-out: without it every switch from `Playing` would
    ///   flash playing→buffering→playing (the new queue starts
    ///   empty). The exemption is a property of the limb, not of the
    ///   switch window: the window completes at the first delivered
    ///   frame while the queue is still climbing, so gating on the
    ///   window would re-admit the flash on the very tick that
    ///   completes the switch.
    fn paces(&self, limb: &Limb) -> bool {
        if limb.track.kind == MediaKind::Subtitle {
            return false;
        }
        !limb.first_frame_pending
    }

    /// Run one bounded tick for `state`. Never blocks; every loop is
    /// budget-bounded (see the module docs).
    pub(crate) fn tick(&mut self, state: SessionState, budget: &TickBudget) -> TickReport {
        let mut report = TickReport {
            fatal: None,
            primed: false,
            starved: false,
            eof_reached: false,
            drained: false,
            first_frame: None,
            damaged_demux: 0,
            damaged_decode: 0,
            damaged: 0,
            buffer_fill: 0.0,
            newly_drained: 0,
            frames: 0,
            concealed_frames: 0,
            stale_frames: 0,
            discontinuities: 0,
        };
        // The demux step runs in every state that owns a live
        // pipeline (Buffering fills, Playing streams, Paused fills to
        // the watermark then parks — doc 03 §4.6's pause row).
        self.demux_step(budget.demux_events, &mut report);
        // Decode only where decoders are allowed to make progress;
        // Paused freezes the sinks (frames must not present) — with
        // one carve-out: while a seek **or a track switch** is in
        // flight, decode runs even in Paused so the landing frame
        // exists to re-anchor/complete on (the wire-visible state
        // stays paused; the clock stays frozen until the completion
        // report). While the seek is in flight the decode's frames
        // enter the M10 presentation hold, not the consumer — the
        // carve-out produces evidence and held frames, never a
        // pre-terminal presentation. A switch's re-prime holds
        // nothing (card 3.A3 step 7: the frame's delivery IS the
        // completion fact).
        // [3.C1 resolution of Lead decision 3.HARD-AUDIT D-a: the
        // buffered pause (Buffering+Pause → Paused) now rides its own
        // episode bookend rows — `Paused + Primed` closes the episode
        // the pause held open — so the pre-roll of a pause-departed
        // SEEK keeps this carve-out (there is still no
        // paused→buffering wire edge, doc 05 §3.3; the seeking
        // pre-roll presents through the M10 hold, not an episode).
        // 3.A3 extends the same carve-out to its switch windows.]
        // NOTE (Lead, 3.B1 merge): plain-Paused decode-ahead (VLC
        // parity — s7's pending AC) is deliberately NOT here yet: it
        // consumes short media to EOS during the pause and inverts
        // the resume-flow expectations; it lands WITH the stage-time
        // fill observation seam (the registered follow-up card).
        if matches!(state, SessionState::Buffering | SessionState::Playing) || self.window_live() {
            self.decode_step(budget, &mut report);
        }
        // The drained check is pure state inspection and also runs
        // while Paused (the media may have fully drained just before
        // the pause landed — doc 05 §3.3's `paused → ended` edge).
        if matches!(state, SessionState::Playing | SessionState::Paused) {
            report.drained = self.is_drained();
        }
        // Priming is computed in Buffering AND Paused (card 3.C1):
        // the paused-during-buffering hold keeps filling — when the
        // watermark lands, the session's `Paused + Primed` bookend
        // row closes the episode. The session gates the Paused signal
        // on the episode being open (a normal pause with full queues
        // absorbs the fact), so computing it here is side-effect
        // free for that case.
        if matches!(state, SessionState::Buffering | SessionState::Paused) {
            // Priming waits for every PACING limb's high watermark
            // (subtitles and switch-installed limbs still recovering
            // are exempt — see [`Pipeline::paces`]).
            let mut primed = !self.limbs.is_empty();
            for limb in self.limbs.iter_mut() {
                // paces(), inlined: subtitles and switch-installed
                // limbs still recovering are exempt.
                if limb.track.kind == MediaKind::Subtitle || limb.first_frame_pending {
                    continue;
                }
                if limb.queue.watermark().high {
                    // Priming is a full recovery: any starvation
                    // episode closes here, so the next dip is a fresh
                    // stall (1-tick latency, 3.B3 design B2/F1.5).
                    limb.episode_open = false;
                } else {
                    primed = false;
                }
            }
            report.primed = primed;
        }
        // The in-episode progress: the bottleneck limb's fill
        // toward its high watermark (card 3.B4 — doc 05 §3.2's
        // `0.0..1.0` while buffering). The exit bookend `1.0` is
        // the table effect; this is the honest mid-episode view.
        if state == SessionState::Buffering {
            // The in-episode progress: the bottleneck pacing limb's
            // fill toward its high watermark (card 3.B4 — doc 05
            // §3.2's `0.0..1.0` while buffering). The exit bookend
            // `1.0` is the table effect; this is the honest
            // mid-episode view.
            report.buffer_fill = self
                .limbs
                .iter()
                .filter(|limb| self.paces(limb))
                .map(|limb| {
                    (limb.queue.usage().packets as f32 / limb.high_packets as f32).clamp(0.0, 1.0)
                })
                .fold(1.0_f32, f32::min);
        }
        if state == SessionState::Playing {
            // An underrun is an input that cannot keep up — the
            // end-of-stream drain is the media ending, not starving
            // (rebuffering at the end would be a lie), and a
            // mid-switch re-prime is a selection change, not a
            // starvation (the paces() carve-out). The re-entry
            // debounce (module docs) filters low-line flapping
            // without delaying a fresh episode's first stall.
            let mut starved = false;
            let mut any_low = false;
            for limb in self.limbs.iter_mut() {
                // paces(), inlined (the iter_mut borrow forbids the
                // method call; the predicate reads limb fields only).
                if limb.track.kind == MediaKind::Subtitle || limb.first_frame_pending {
                    continue;
                }
                let mark = limb.queue.watermark();
                if mark.high {
                    // A full recovery closes the episode even without
                    // a rebuffer cycle in between.
                    limb.episode_open = false;
                }
                if mark.low {
                    any_low = true;
                    limb.low_streak = limb.low_streak.saturating_add(1);
                    if !limb.episode_open || limb.low_streak >= self.starve_debounce {
                        limb.episode_open = true;
                        starved = true;
                    }
                } else {
                    limb.low_streak = 0;
                }
            }
            if any_low {
                self.starvation.starved_ticks = self.starvation.starved_ticks.saturating_add(1);
            }
            report.starved = !self.eof && starved;
            if report.starved {
                self.starvation.starve_signals = self.starvation.starve_signals.saturating_add(1);
            }
        }
        report
    }

    /// The lifetime starvation counters (card 3.B3, design C2) — a
    /// snapshot for the session's stats surface
    /// ([`EngineStats`](crate::EngineStats), through
    /// `Session::absorb_pipeline_starvation`) and for the
    /// engine-level property tests.
    pub(crate) fn starvation_counters(&self) -> StarvationCounters {
        self.starvation
    }

    /// End-of-stream with nothing left anywhere: no held packet, no
    /// resident queue items, every decoder flushed to `Drained`. The
    /// empty-limb set counts as drained (card 3.A3: deselecting both
    /// A/V kinds must let the media end honestly at EOF — the
    /// refusing-an-unwirrable-open boundary lives at wire time,
    /// `NoDecodableTrack`, not here).
    fn is_drained(&self) -> bool {
        self.eof
            && self.pending.is_none()
            && self
                .limbs
                .iter()
                .all(|limb| limb.drained && limb.queue.usage().packets == 0)
    }

    /// The demux slice: re-offer the held packet, then pull bounded
    /// events, stamping every packet with the live generation before
    /// pushing into its limb's queue (doc 03 §5 checking point 3).
    fn demux_step(&mut self, events: u32, report: &mut TickReport) {
        // Re-offer first (FIFO across ticks is preserved: the held
        // packet predates everything still undemuxed). A full queue
        // hands it back again — held, never dropped, never spun on.
        if self.pending.is_some() {
            // The ingestion-stall metric (design C2): this tick began
            // with the demuxer still parked on a full queue.
            self.starvation.pending_hold_ticks =
                self.starvation.pending_hold_ticks.saturating_add(1);
        }
        if let Some(packet) = self.pending.take() {
            self.push_packet(packet);
        }
        if self.eof {
            // The demuxer is exhausted; nothing new can arrive. The
            // drains report through the decode step.
            return;
        }
        for _ in 0..events {
            match self.demuxer.next_event() {
                Ok(DemuxEvent::Packet(mut packet)) => {
                    // The demux driver stamps the current token
                    // generation onto the item at push (doc 03 §5
                    // checking point 3) — the demuxer's own stamp is
                    // authoritative for nothing.
                    packet.generation = self.generation;
                    self.push_packet(packet);
                    if self.pending.is_some() {
                        // The queue is full: backpressure holds the
                        // packet and ends this tick's demux slice.
                        return;
                    }
                }
                Ok(DemuxEvent::Recovered { skipped, .. }) => {
                    // The count is a LOSS count (units skipped): a
                    // zero-skip `Recovered` is a reason-only note —
                    // e.g. the legal negative-`biHeight` bottom-up
                    // DIB, or the absolute-idx1 interpretation
                    // choice — with zero units lost. Those stay
                    // demuxer-level observability (`damage_total()`,
                    // the demux drains); the wire's damage event
                    // speaks only when something was actually
                    // skipped (the recovery_env stall stand-in's
                    // documented inertness relies on this).
                    report.damaged_demux = report.damaged_demux.saturating_add(skipped);
                    report.damaged = report.damaged.saturating_add(skipped);
                }
                Ok(DemuxEvent::Discontinuity) => {
                    // A marked discontinuity: v0 counts and carries
                    // on (the clock re-anchor policy is Phase 3 sync,
                    // card 3.B1). Counted so the fact is observable.
                    report.discontinuities = report.discontinuities.saturating_add(1);
                }
                Ok(DemuxEvent::EndOfStream) => {
                    if !self.eof {
                        self.eof = true;
                        report.eof_reached = true;
                    }
                    return;
                }
                Err(error) => {
                    report.fatal = Some(error);
                    return;
                }
            }
        }
    }

    /// Push one stamped packet into its limb's queue, holding it if
    /// the queue is full. Packets for unwired tracks (subtitle/data
    /// in v0) are dropped — they were never selected for decode.
    fn push_packet(&mut self, packet: Packet) {
        let Some(limb) = self.limbs.iter().find(|limb| limb.track.id == packet.track) else {
            // Unwired track: not selected, dropped by policy (the
            // track list event already told the truth about
            // selection).
            return;
        };
        let queue = Arc::clone(&limb.queue);
        match queue.try_push(Stamped::from_payload(packet)) {
            PushOutcome::Pushed => {}
            PushOutcome::Full { item, binding } => {
                // The which-cap-starves-us diagnostic (design C2):
                // tally the binding axis the queue reported.
                match binding {
                    packet_queue::LimitAxis::Packets => {
                        self.starvation.full_rejections_packets =
                            self.starvation.full_rejections_packets.saturating_add(1);
                    }
                    packet_queue::LimitAxis::Bytes => {
                        self.starvation.full_rejections_bytes =
                            self.starvation.full_rejections_bytes.saturating_add(1);
                    }
                    packet_queue::LimitAxis::Duration => {
                        self.starvation.full_rejections_duration =
                            self.starvation.full_rejections_duration.saturating_add(1);
                    }
                }
                // Backpressure: hold the untouched packet for the
                // next tick (doc 03 §4.4 Q3 — fresh data is never
                // dropped).
                self.pending = Some(item.value);
            }
            // Stale and Closed can only strike once mints and
            // teardown race producer threads (multi-threaded Phase
            // 3+); the synchronous v0 engine never observes them
            // here, and the queue's own stats count both.
            PushOutcome::Stale { .. } | PushOutcome::Closed { .. } => {}
        }
    }

    /// The decode slice: pop and decode bounded packets per limb,
    /// then (at end-of-stream) flush bounded. Frames leave through
    /// the sink seam with a generation check (doc 03 §5 checking
    /// point 5). While a seek is in flight the slice's frames are
    /// **captured into the M10 pre-roll hold** instead of the
    /// consumer — and the slice backpressures at the hold's cap
    /// (packets stay in the bounded queues; the pre-roll evidence
    /// they already produced is what completes the seek).
    ///
    /// # Per-frame damage isolation (doc 03 §4.5(d), audit M9)
    ///
    /// A packet that fails with a per-frame damage kind
    /// ([`per_frame_damage`]) does **not** end the session:
    ///
    /// - **video** (W1-08): the damage is counted into the tick's
    ///   `damaged` facts (the engine `DamagedData` event + the
    ///   coalesced non-fatal `playback.error`, card 3.B4 — the same
    ///   Recovered-class shape the PCM decoder already reports
    ///   in-band), the presentation conceals by repeating the limb's
    ///   last presented frame at the damaged slot's pts (or holds
    ///   black when no current-generation frame exists), and the
    ///   slice continues. During a seek's pre-roll the concealment
    ///   slot holds (nothing repeats against the frozen clock — M10).
    /// - **audio** (3.C2 G3, design D-2): the damaged packet is
    ///   **dropped and counted** — no concealment (no "last presented
    ///   audio frame" to repeat) and NO synthesized silence (that
    ///   would fabricate data); the output ring simply has a gap and
    ///   the clock continues from the next delivered frame's pts.
    ///   [FLAG 3.B1/W2-13: the pts gap should ride the
    ///   discontinuity re-anchor path, not be read as drift.]
    ///
    /// The run of consecutive failures is bounded by
    /// [`VIDEO_CONCEALMENT_BUDGET`] (one shared budget); past it, the
    /// damage is catastrophic and reaches a typed terminal (D-3: one
    /// corrupt packet must not kill the session, but a stream of
    /// nothing but corrupt packets must not fake playing either).
    fn decode_step(&mut self, budget: &TickBudget, report: &mut TickReport) {
        let consumer = Arc::clone(&self.consumer);
        for limb in self.limbs.iter_mut() {
            let track_id = limb.track.id;
            let limb_kind = limb.track.kind;
            // The **limb's** generation floor, not the pipeline-wide
            // scalar (card 3.A3): a switch mint advances the demux
            // stamp without invalidating the unchanged kind's
            // consumer path — its G-stamped residents stay
            // presentable, while the switched kind's stale
            // stragglers still die at the new limb's floor.
            let generation = limb.generation;
            let mut fatal: Option<EngineError> = None;
            // The slice's observed facts, hoisted out of the sink's
            // scope so the pre-roll hold borrow ends before the
            // observation merge below (the hold slot borrows the same
            // `window` the merge reads). Assigned exactly once
            // at the end of the scope below.
            let sink_frames: u32;
            let sink_first: Option<Stamp>;
            let sink_stale: u32;
            let sink_concealed: u32;
            let sink_last: Option<Frame>;
            {
                // M10: while a seek is in flight, the sink routes the
                // slice's frames into the per-track hold — they reach
                // the consumer only at the seek terminal
                // (`release_preroll_hold`), never against the frozen
                // clock. A switch flying alone arms no hold (its
                // re-prime delivers straight through), so
                // `hold_slot` answers `None` and the frames present
                // live.
                let mut sink = LimbSink {
                    consumer: Arc::clone(&consumer),
                    track: track_id,
                    generation,
                    frames: 0,
                    first: None,
                    stale_dropped: 0,
                    // The concealment cache's slice-local half (M9):
                    // seeded from the limb, written back after the
                    // slice.
                    last: limb.last_frame.clone(),
                    concealed: 0,
                    hold: self.window.as_mut().and_then(|w| w.hold_slot(track_id)),
                };
                for _ in 0..budget.packets_per_limb {
                    // Hold backpressure (M10, D-1): a full hold ends
                    // this limb's slice — the packets stay queued,
                    // bounded, and the seek completes on the evidence
                    // already held (a full slot implies the limb
                    // delivered, so this can never wedge the
                    // pre-roll).
                    if sink.hold_is_full() {
                        break;
                    }
                    // Three pop outcomes matter: an item (decode it), a
                    // discontinuity marker (observed, keep going), and
                    // nothing resident (stop this limb's slice).
                    let popped = match limb.queue.try_pop() {
                        PopOutcome::Item(stamped) => Some(Some(stamped.value)),
                        PopOutcome::Discontinuity(_) => Some(None),
                        PopOutcome::Empty | PopOutcome::Closed => None,
                    };
                    match popped {
                        None => break,
                        Some(None) => continue,
                        Some(Some(packet)) => {
                            let pts = packet.pts;
                            match limb.decoder.decode(packet, &mut sink) {
                                Ok(outcome) => {
                                    // The decoder stayed inside its
                                    // contract on this packet: whatever
                                    // per-frame damage run was building
                                    // ends here.
                                    limb.conceal_failures = 0;
                                    if let DecodeOutcome::Recovered { skipped, .. } = outcome {
                                        report.damaged_decode = report
                                            .damaged_decode
                                            .saturating_add(u64::from(skipped));
                                        report.damaged =
                                            report.damaged.saturating_add(u64::from(skipped));
                                    }
                                }
                                Err(error) => {
                                    if per_frame_damage(&error) {
                                        limb.conceal_failures =
                                            limb.conceal_failures.saturating_add(1);
                                        if limb.conceal_failures > VIDEO_CONCEALMENT_BUDGET {
                                            // The bounded window is spent:
                                            // catastrophic damage, typed
                                            // terminal (D-3 honesty).
                                            fatal = Some(concealment_budget_exhausted(
                                                limb.conceal_failures,
                                                error,
                                            ));
                                            break;
                                        }
                                        // The Recovered-class conversion:
                                        // one damaged unit, counted and
                                        // observable.
                                        report.damaged_decode =
                                            report.damaged_decode.saturating_add(1);
                                        report.damaged = report.damaged.saturating_add(1);
                                        if limb_kind == MediaKind::Video {
                                            // Conceal by repeating the last
                                            // presented frame at this slot's
                                            // pts — or holding (nothing
                                            // presented) when no
                                            // current-generation frame
                                            // exists. During a seek's
                                            // pre-roll the concealment
                                            // holds too (M10 — see
                                            // [`LimbSink::conceal_last`]).
                                            // A consumer refusal is the
                                            // sink contract's fatal.
                                            if let Err(error) = sink.conceal_last(pts) {
                                                fatal = Some(error);
                                                break;
                                            }
                                        }
                                        // Audio (3.C2 G3, design D-2):
                                        // skip-and-report. The packet is
                                        // dropped — no sink call, no
                                        // synthesized silence, no pts
                                        // invention; the ring has a gap
                                        // and the next delivered frame
                                        // re-anchors the clock. The
                                        // damage counts above are the
                                        // observable.
                                    } else {
                                        fatal = Some(error);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                // End-of-stream draining: flush the decoder until it
                // reports `Drained`, bounded per tick. The same hold
                // backpressure applies (a landing flush's frames are
                // pre-roll frames too — held, not presented).
                if fatal.is_none() && self.eof && !limb.drained {
                    let queue_empty = limb.queue.usage().packets == 0;
                    if queue_empty && self.pending.is_none() {
                        for _ in 0..budget.flush_calls_per_limb {
                            if sink.hold_is_full() {
                                break;
                            }
                            match limb.decoder.flush(&mut sink) {
                                Ok(DecodeOutcome::Drained) => {
                                    limb.drained = true;
                                    // Newly drained this tick: the
                                    // `playback.end-of-track` candidate
                                    // (card 3.B4) — pushed into the reused
                                    // scratch the session drains after
                                    // the tick.
                                    self.ended_limbs.push((limb_kind, track_id));
                                    report.newly_drained = report.newly_drained.saturating_add(1);
                                    break;
                                }
                                Ok(_) => continue,
                                Err(error) => {
                                    fatal = Some(error);
                                    break;
                                }
                            }
                        }
                    }
                }
                sink_frames = sink.frames;
                sink_first = sink.first;
                sink_stale = sink.stale_dropped;
                sink_concealed = sink.concealed;
                sink_last = sink.last;
            }
            if let Some(error) = fatal {
                report.fatal = Some(error);
                return;
            }
            // Merge the sink's observations: the first frame ever
            // delivered becomes the session's FirstFrame fact; stale
            // frame drops are counted (v0 cannot produce them — the
            // generation check is the doc 03 §5 point-5 checkpoint);
            // concealment presentations are counted separately so a
            // repeated frame is never reported as decoded output.
            report.frames = report.frames.saturating_add(u64::from(sink_frames));
            report.concealed_frames = report
                .concealed_frames
                .saturating_add(u64::from(sink_concealed));
            report.stale_frames = report.stale_frames.saturating_add(u64::from(sink_stale));
            // The concealment cache follows the slice back into the
            // limb (the sink's latch or the frame it still holds).
            limb.last_frame = sink_last;
            // The re-prime exemption (card 3.A3 step 5) ends when the
            // limb's queue has **recovered to its low watermark** —
            // not at the first delivered frame: the queue is still
            // climbing from empty on the completing tick, and
            // re-admitting the limb to the underrun check at that
            // moment would flash playing→buffering on every switch
            // (the exact S2.5 bug the carve-out exists to prevent).
            // A sparse or span-exhausted track whose queue never
            // recovers simply stays exempt — honest silence, VLC
            // parity (a selection change is not an underrun).
            if limb.first_frame_pending && limb.queue.usage().packets >= limb.low_packets {
                limb.first_frame_pending = false;
            }
            if self.first_frame.is_none()
                && let Some(position) = sink_first
            {
                self.first_frame = Some(position);
                report.first_frame = Some(position);
            }
            // The in-flight re-prime observation (doc 03 §6 step 5 /
            // card 3.A3 step 5): frames delivered this slice, and the
            // first frame's stamp (the anchor truth for the seek's
            // audio re-anchor and the switch's new-track re-anchor).
            if let Some(window) = self.window.as_mut() {
                window.observe(track_id, limb_kind, sink_frames, sink_first);
            }
        }
    }
}

/// A present stamp's nanoseconds (`None` when missing or
/// unrepresentable — absence is data, never a guess).
fn stamp_ns(stamp: Stamp) -> Option<i64> {
    let timestamp = stamp.timestamp().ok()?;
    match timestamp.as_nanos() {
        Ok(ns) => Some(nanos_i64(ns)),
        Err(_) => None,
    }
}

/// The per-limb [`FrameSink`]: the only path frames leave the engine
/// through in v0. Checks each frame's generation (doc 03 §5 checking
/// point 5 — stale frames never reach the consumer) and records the
/// first-frame timestamp. While a seek is in flight the sink carries
/// the limb's M10 hold slot: live frames are captured into it instead
/// of being delivered (released at the seek terminal, discarded on
/// rollback — see [`Pipeline::release_preroll_hold`]). The sink also
/// owns the slice's half of the per-frame-damage concealment cache
/// (see [`Pipeline::decode_step`]): the last presented video frame,
/// latched on `accept` and written back into the limb after the
/// slice.
struct LimbSink<'a> {
    consumer: Arc<dyn FrameConsumer>,
    track: TrackId,
    generation: GenerationId,
    frames: u32,
    first: Option<Stamp>,
    stale_dropped: u32,
    /// The pre-roll hold slot (M10): `None` outside a seek's
    /// pre-roll (frames deliver straight through) or during the seek
    /// flush itself (the flush's emissions are the old generation's
    /// residue — the generation check drops them, never held).
    hold: Option<PrerollHoldSlot<'a>>,
    /// The last presented video frame of the current generation (the
    /// concealment cache's slice-local half; `None` until a video
    /// frame is presented under this generation — a frame captured
    /// into the hold is not presented yet, so it never latches here;
    /// the release path latches it at the terminal instead).
    last: Option<Frame>,
    /// Concealment presentations this slice (repeated frames —
    /// labeled, never counted as decoded output).
    concealed: u32,
}

impl LimbSink<'_> {
    /// Conceal one per-frame-damaged video slot (doc 03 §4.5(d),
    /// audit M9): re-present the latched last-good frame at `pts` —
    /// the **same pixels again**, never new content. Counts
    /// `concealed`, not `frames`; never sets `first` (a repeated
    /// frame is not a first frame) and never latches itself. With no
    /// latched current-generation frame the slot **holds**: nothing
    /// is presented, the damage counts are the observable.
    ///
    /// During a seek's pre-roll (M10) the slot holds unconditionally:
    /// a concealment repeat is a presentation, and nothing presents
    /// while the clock is frozen at the old position.
    ///
    /// # Errors
    /// The consumer's typed [`EngineError`] — a refusal downstream is
    /// the [`FrameSink`] propagation contract's fatal, exactly as for
    /// decoded frames.
    fn conceal_last(&mut self, pts: Stamp) -> Result<(), EngineError> {
        // M10: the pre-roll hold owns presentation until the terminal.
        if self.hold.is_some() {
            return Ok(());
        }
        let repeated = match self.last.as_ref() {
            None => return Ok(()),
            // A latched frame from an older generation (a seek minted
            // since it was presented) is never repeated: the slot
            // holds, matching the no-prior-frame rule.
            Some(held) if held.is_stale(self.generation) => return Ok(()),
            Some(held) => {
                let mut repeated = held.clone();
                if let Frame::Video(video) = &mut repeated {
                    video.pts = pts;
                }
                repeated
            }
        };
        self.consumer.accept(repeated, self.track)?;
        self.concealed = self.concealed.saturating_add(1);
        Ok(())
    }

    /// Whether the sink's hold (if any) is at capacity — `false`
    /// when frames deliver straight through (no hold, no bound).
    fn hold_is_full(&self) -> bool {
        self.hold.as_ref().is_some_and(PrerollHoldSlot::is_full)
    }
}

/// One limb's pre-roll hold slot, as handed to a decode slice (M10):
/// a hard-capped frame buffer plus the capacity that bounds it.
#[derive(Debug)]
struct PrerollHoldSlot<'a> {
    frames: &'a mut Vec<Frame>,
    cap: usize,
}

impl PrerollHoldSlot<'_> {
    /// Whether the slot is at its hard capacity (D-1: the bound the
    /// decode slice backpressures against).
    fn is_full(&self) -> bool {
        self.frames.len() >= self.cap
    }

    /// Capture one frame. At capacity the frame is refused — the
    /// caller counts the loss (never buffered unboundedly, never
    /// silently swallowed); reachable only through a single decode
    /// call bursting past the whole slice's worth of frames.
    fn push(&mut self, frame: Frame) -> bool {
        if self.is_full() {
            return false;
        }
        self.frames.push(frame);
        true
    }
}

impl FrameSink for LimbSink<'_> {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        let pts = frame.pts();
        if frame.is_stale(self.generation) {
            // A stale frame is dropped on sight — no error, no
            // consumer call, counted (doc 03 §5 point 5).
            self.stale_dropped = self.stale_dropped.saturating_add(1);
            return Ok(());
        }
        if let Some(hold) = self.hold.as_mut() {
            // M10: capture, do not deliver. A refusal here is the
            // single-call-burst backstop — counted as a discard,
            // never an unbounded buffer and never a pre-terminal
            // presentation. The capture does not latch the
            // concealment cache (`last`): a held frame is not
            // presented yet — the release path latches it at the
            // terminal, on the actual hand-off.
            if !hold.push(frame) {
                self.stale_dropped = self.stale_dropped.saturating_add(1);
            }
            self.frames = self.frames.saturating_add(1);
            if self.first.is_none() {
                self.first = Some(pts);
            }
            return Ok(());
        }
        // Latch the last presented video frame for the concealment
        // cache before the hand-off — a clone of fixed-size fields
        // plus one `SharedBytes` reference bump, no copy of the pixel
        // bytes (D-1: no heap allocation on the frame path).
        let latched = if matches!(frame, Frame::Video(_)) {
            Some(frame.clone())
        } else {
            None
        };
        self.consumer.accept(frame, self.track)?;
        self.frames = self.frames.saturating_add(1);
        if self.first.is_none() {
            self.first = Some(pts);
        }
        if let Some(frame) = latched {
            self.last = Some(frame);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! Card 3.B3's engine-side policy proofs: the starve debounce and
    //! the packets-axis low, exercised against the real `Pipeline`
    //! tick with minimal fakes. The queue levels are driven by hand
    //! (the demuxer produces nothing, the decoder pops one packet per
    //! tick through a 1-decode budget), so every watermark transition
    //! is exactly scripted; the state-machine consequences (Underrun →
    //! Buffering → Primed) live in `session_flow.rs`.

    use super::*;
    use media_types::packet::SharedBytes;
    use std::sync::Mutex;
    use testkit::Must;

    /// A demuxer that reports one audio track and produces nothing
    /// (a producer stall with no end-of-stream — the underrun shape).
    struct StalledDemuxer;

    impl Demuxer for StalledDemuxer {
        fn start(&mut self) -> Result<Vec<Track>, EngineError> {
            Ok(vec![Track {
                id: TrackId::new(1),
                stream: media_types::StreamId::new(1),
                kind: MediaKind::Audio,
                language: None,
                codec: media_types::ids::CodecDescriptor::new("fake-pcm").must(),
                title: None,
                flags: media_types::TrackFlags::NONE,
                timing: media_types::TrackTiming {
                    span: media_types::TimeSpan::UNKNOWN,
                    base: media_types::TimeBase::new(1, 1_000_000).must(),
                    gaps: Vec::new(),
                },
            }])
        }

        fn next_event(&mut self) -> Result<plugin_api::DemuxEvent, EngineError> {
            Ok(plugin_api::DemuxEvent::Discontinuity)
        }

        fn seek(
            &mut self,
            _target: media_types::TimeSpan,
        ) -> Result<media_types::TimeSpan, EngineError> {
            // This producer never demuxes; seeks land nowhere (the
            // tests drive queue levels by hand).
            Ok(media_types::TimeSpan::UNKNOWN)
        }
    }

    /// A decoder that swallows packets silently (the tests observe
    /// queue watermarks, not frames).
    struct NullDecoder;

    impl Decoder for NullDecoder {
        fn configure(&mut self, _config: &DecoderConfig) -> Result<(), EngineError> {
            Ok(())
        }

        fn decode(
            &mut self,
            _packet: Packet,
            _out: &mut dyn FrameSink,
        ) -> Result<DecodeOutcome, EngineError> {
            Ok(DecodeOutcome::NeedMoreInput)
        }

        fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
            Ok(DecodeOutcome::Drained)
        }
    }

    struct NullProvider;

    impl DecoderProvider for NullProvider {
        fn decoder_for(
            &self,
            _track: &Track,
            _config: &DecoderConfig,
        ) -> Result<Box<dyn Decoder>, EngineError> {
            Ok(Box::new(NullDecoder))
        }
    }

    struct NullConsumer;

    impl FrameConsumer for NullConsumer {
        fn accept(&self, _frame: Frame, _track: TrackId) -> Result<(), EngineError> {
            Ok(())
        }
    }

    /// A wired single-limb pipeline with an 8-packet cap (high 6, low
    /// 2 on the packets axis) and a caller-chosen debounce; one
    /// packet is decoded per tick, the demuxer produces nothing.
    fn wired(debounce: u32) -> Pipeline {
        let limits = QueueLimits {
            max_packets: nonzero(8),
            max_bytes: nonzero(1 << 20),
            max_duration: media_types::Nanoseconds(1_000_000_000),
        };
        let mut plan = QueuePlan::new(limits, limits);
        plan.starve_debounce_ticks = debounce;
        let consumer: Arc<dyn FrameConsumer> = Arc::new(NullConsumer);
        let mut pipeline = Pipeline::new(Box::new(StalledDemuxer), Arc::clone(&consumer));
        let tracks = pipeline.start_demuxer().must();
        let wired = pipeline
            .wire(tracks, &NullProvider, &plan, GenerationId::FIRST)
            .must();
        assert_eq!(wired.wired, 1);
        // The wire-time mint publishes the generation and leaves a
        // dropped=0 discontinuity marker pending in each fresh queue;
        // drain it here so the scripted packet levels below are exact
        // (the marker would otherwise eat the first tick's decode
        // slice — `decode_step` spends one budget slot observing it).
        for limb in pipeline.limbs.iter_mut() {
            assert!(matches!(limb.queue.try_pop(), PopOutcome::Discontinuity(_)));
            assert!(matches!(limb.queue.try_pop(), PopOutcome::Empty));
        }
        pipeline
    }

    /// The tick budget the scripted levels assume: no demux events, a
    /// single decode per limb per tick.
    fn budget() -> TickBudget {
        TickBudget {
            demux_events: 0,
            packets_per_limb: 1,
            flush_calls_per_limb: 0,
        }
    }

    /// Push `n` plain packets into the limb's queue (generation-live,
    /// duration-less — the packets-axis wedge case).
    fn fill(pipeline: &mut Pipeline, n: usize) {
        for _ in 0..n {
            let packet = Packet {
                track: TrackId::new(1),
                pts: Stamp::Missing,
                dts: Stamp::Missing,
                duration: None,
                data: SharedBytes::from_static(&[7u8; 32]),
                keyframe: false,
                discontinuity: false,
                generation: GenerationId::FIRST,
            };
            let limb = pipeline.limbs.first_mut().must();
            assert_eq!(
                limb.queue.try_push(Stamped::from_payload(packet)),
                PushOutcome::Pushed
            );
        }
    }

    /// The packets-axis low (design B2 gap 2): with the engine's
    /// thresholds the all-axes low predicate degenerates to
    /// packets-only — a bytes-heavy or duration-less mix can always
    /// reach `low` through its packet count, and `high` stays
    /// all-axes-eager.
    #[test]
    fn packets_axis_is_the_starvation_axis() {
        let plan = QueuePlan::default();
        let marks = plan.thresholds(&plan.audio);
        let at_caps = QueueUsage {
            packets: 1,
            bytes: plan.audio.max_bytes.get(),
            duration: plan.audio.max_duration,
        };
        assert!(
            packet_queue::Watermark::evaluate(at_caps, &marks).low,
            "bytes/duration at their caps never block low"
        );
        let high_on_bytes = QueueUsage {
            packets: 1,
            bytes: plan.audio.max_bytes.get(),
            duration: media_types::Nanoseconds(0),
        };
        assert!(packet_queue::Watermark::evaluate(high_on_bytes, &marks).high);
    }

    /// First stall of a fresh episode fires at 1 tick (design F1.5:
    /// first-stall latency is never delayed by the debounce).
    #[test]
    fn first_stall_of_a_fresh_episode_fires_at_one_tick() {
        let mut pipeline = wired(2);
        let budget = budget();
        fill(&mut pipeline, 8); // 8 → high
        let _ = pipeline.tick(SessionState::Playing, &budget); // 7? no: pops 1 → 7 (high)
        let _ = pipeline.tick(SessionState::Playing, &budget); // 6 (high)
        let _ = pipeline.tick(SessionState::Playing, &budget); // 5
        let _ = pipeline.tick(SessionState::Playing, &budget); // 4
        let _ = pipeline.tick(SessionState::Playing, &budget); // 3
        let near = pipeline.tick(SessionState::Playing, &budget); // 2 → low
        assert!(near.starved, "the first low tick fires immediately");
        assert_eq!(pipeline.starvation_counters().starve_signals, 1);
    }

    /// A re-entry dip inside an open episode must persist the debounce
    /// window: single-tick dips between recoveries never re-fire (the
    /// flapping producer/consumer rate mix, design B2 gap 1).
    #[test]
    fn single_tick_reentry_dips_do_not_refire() {
        let mut pipeline = wired(2);
        let budget = budget();
        // Open the episode: 3 packets, one tick pops to 2 → low → fire.
        fill(&mut pipeline, 3);
        let fired = pipeline.tick(SessionState::Playing, &budget); // 2 → low
        assert!(fired.starved);
        // Recover above low without reaching high, then dip once.
        fill(&mut pipeline, 2); // 4
        let _ = pipeline.tick(SessionState::Playing, &budget); // 3 (not low)
        let dip = pipeline.tick(SessionState::Playing, &budget); // 2 → low
        assert!(
            !dip.starved,
            "a first re-entry tick inside an open episode is debounced"
        );
        // Recover, dip again: still debounced — the flapping never fires.
        fill(&mut pipeline, 2); // 4
        let _ = pipeline.tick(SessionState::Playing, &budget); // 3
        let dip = pipeline.tick(SessionState::Playing, &budget); // 2 → low
        assert!(
            !dip.starved,
            "the second single-tick dip is still debounced"
        );
        assert_eq!(
            pipeline.starvation_counters().starve_signals,
            1,
            "flapping never re-fires"
        );
        assert_eq!(
            pipeline.starvation_counters().starved_ticks,
            3,
            "the pre-debounce raw rate counts every low tick"
        );
    }

    /// A sustained low after a re-entry fires exactly at the debounce
    /// count (design B2): persistence is starvation, a dip is jitter.
    #[test]
    fn sustained_low_fires_at_the_debounce_count() {
        let mut pipeline = wired(3);
        let budget = budget();
        fill(&mut pipeline, 3);
        assert!(pipeline.tick(SessionState::Playing, &budget).starved); // 2, fire
        fill(&mut pipeline, 2); // 4
        let _ = pipeline.tick(SessionState::Playing, &budget); // 3, not low
        let t1 = pipeline.tick(SessionState::Playing, &budget); // 2, streak 1
        assert!(!t1.starved, "re-entry streak 1 of 3");
        let t2 = pipeline.tick(SessionState::Playing, &budget); // 1, streak 2
        assert!(!t2.starved, "re-entry streak 2 of 3");
        let t3 = pipeline.tick(SessionState::Playing, &budget); // 0, streak 3
        assert!(t3.starved, "re-entry streak 3 of 3 fires");
        let counters = pipeline.starvation_counters();
        assert_eq!(counters.starved_ticks, 4, "the fire + 3 low re-entry ticks");
        assert_eq!(counters.starve_signals, 2);
    }

    /// A full recovery (high watermark) closes the episode: the next
    /// dip is a fresh stall and fires immediately again.
    #[test]
    fn full_recovery_closes_the_episode() {
        let mut pipeline = wired(2);
        let budget = budget();
        fill(&mut pipeline, 3);
        assert!(pipeline.tick(SessionState::Playing, &budget).starved); // 2
        fill(&mut pipeline, 6); // 8 → high on the next tick's observation
        let priming = pipeline.tick(SessionState::Playing, &budget); // 7, high
        assert!(!priming.starved);
        // Drain to low again: a FRESH stall — immediate.
        let _ = pipeline.tick(SessionState::Playing, &budget); // 6
        let _ = pipeline.tick(SessionState::Playing, &budget); // 5
        let _ = pipeline.tick(SessionState::Playing, &budget); // 4
        let _ = pipeline.tick(SessionState::Playing, &budget); // 3
        let fresh = pipeline.tick(SessionState::Playing, &budget); // 2 → low
        assert!(fresh.starved, "post-prime dips are fresh stalls");
        assert_eq!(pipeline.starvation_counters().starve_signals, 2);
    }

    /// A seek mint resets the debounce state: streaks and episodes die
    /// with the generation (design B4).
    #[test]
    fn seek_mint_resets_the_debounce_state() {
        let mut pipeline = wired(2);
        let budget = budget();
        fill(&mut pipeline, 3);
        assert!(pipeline.tick(SessionState::Playing, &budget).starved);
        let generation = GenerationId::FIRST.next().must();
        pipeline.seek_flush(generation, 8, 1).must();
        let limb = pipeline.limbs.first().must();
        assert_eq!(limb.low_streak, 0, "streaks reset at the mint");
        assert!(!limb.episode_open, "episodes reset at the mint");
    }

    /// Debounce 1 is exactly the undebounced behavior: every low tick
    /// fires (the configuration floor — a re-entry needs only itself).
    #[test]
    fn debounce_one_is_the_undebounced_behavior() {
        let mut pipeline = wired(1);
        let budget = budget();
        fill(&mut pipeline, 3);
        assert!(pipeline.tick(SessionState::Playing, &budget).starved); // 2
        assert!(pipeline.tick(SessionState::Playing, &budget).starved); // 1
        assert!(pipeline.tick(SessionState::Playing, &budget).starved); // 0
        assert_eq!(pipeline.starvation_counters().starve_signals, 3);
    }

    /// A duration-less mix (the `Weigh`-maps-unknown-to-0 wedge case)
    /// still starves on the packets axis.
    #[test]
    fn unknown_duration_mix_reaches_low() {
        let mut pipeline = wired(2);
        let budget = budget();
        fill(&mut pipeline, 8);
        for _ in 0..5 {
            let report = pipeline.tick(SessionState::Playing, &budget);
            assert!(!report.starved);
        }
        let report = pipeline.tick(SessionState::Playing, &budget); // 2 → low
        assert!(
            report.starved,
            "a pure unknown-duration mix still starves on packets"
        );
    }

    /// A demuxer that reports two audio tracks with distinct codec
    /// descriptors (the F-4 recycle shape: one gets parked, the other
    /// takes over the lane).
    struct TwoCodecDemuxer;

    impl Demuxer for TwoCodecDemuxer {
        fn start(&mut self) -> Result<Vec<Track>, EngineError> {
            let track = |id: u32, codec: &str| Track {
                id: TrackId::new(id),
                stream: media_types::StreamId::new(id),
                kind: MediaKind::Audio,
                language: None,
                codec: media_types::ids::CodecDescriptor::new(codec).must(),
                title: None,
                flags: media_types::TrackFlags::NONE,
                timing: media_types::TrackTiming {
                    span: media_types::TimeSpan::UNKNOWN,
                    base: media_types::TimeBase::new(1, 1_000_000).must(),
                    gaps: Vec::new(),
                },
            };
            Ok(vec![
                track(1, "fake-pcm-a"),
                track(2, "fake-pcm-b"),
            ])
        }

        fn next_event(&mut self) -> Result<plugin_api::DemuxEvent, EngineError> {
            Ok(plugin_api::DemuxEvent::Discontinuity)
        }

        fn seek(
            &mut self,
            _target: media_types::TimeSpan,
        ) -> Result<media_types::TimeSpan, EngineError> {
            Ok(media_types::TimeSpan::UNKNOWN)
        }
    }

    /// A decoder that records every codec it is configured with (the
    /// F-4 observable: exactly which descriptor the recycle path
    /// handed over).
    struct RecordingDecoder {
        log: Arc<Mutex<Vec<String>>>,
    }

    impl Decoder for RecordingDecoder {
        fn configure(&mut self, config: &DecoderConfig) -> Result<(), EngineError> {
            self.log.lock().must().push(config.codec.as_str().to_string());
            Ok(())
        }

        fn decode(
            &mut self,
            _packet: Packet,
            _out: &mut dyn FrameSink,
        ) -> Result<DecodeOutcome, EngineError> {
            Ok(DecodeOutcome::NeedMoreInput)
        }

        fn flush(&mut self, _out: &mut dyn FrameSink) -> Result<DecodeOutcome, EngineError> {
            Ok(DecodeOutcome::Drained)
        }
    }

    /// Hands out [`RecordingDecoder`] instances sharing one log.
    struct RecordingProvider {
        log: Arc<Mutex<Vec<String>>>,
    }

    impl DecoderProvider for RecordingProvider {
        fn decoder_for(
            &self,
            _track: &Track,
            _config: &DecoderConfig,
        ) -> Result<Box<dyn Decoder>, EngineError> {
            Ok(Box::new(RecordingDecoder {
                log: Arc::clone(&self.log),
            }))
        }
    }

    /// Audit F-4: the recycle path's reconfigure uses the codec
    /// parked **with** the decoder (captured at park time), never a
    /// descriptor re-derived from the track list or a fabricated
    /// `unknown` literal — the retire→park→seek-flush chain stays
    /// decoupled from the registry's validation rules. The retired
    /// fallback (`CodecDescriptor::new("unknown")` +
    /// `unreachable!`) would have panicked here the day the registry
    /// learned to reject that literal.
    #[test]
    fn parked_codec_rides_the_recycle_path() {
        let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let provider = RecordingProvider {
            log: Arc::clone(&log),
        };
        let consumer: Arc<dyn FrameConsumer> = Arc::new(NullConsumer);
        let mut pipeline = Pipeline::new(Box::new(TwoCodecDemuxer), Arc::clone(&consumer));
        let tracks = pipeline.start_demuxer().must();
        let limits = QueueLimits {
            max_packets: nonzero(8),
            max_bytes: nonzero(1 << 20),
            max_duration: media_types::Nanoseconds(1_000_000_000),
        };
        let plan = QueuePlan::new(limits, limits);
        let wired = pipeline
            .wire(tracks, &provider, &plan, GenerationId::FIRST)
            .must();
        assert_eq!(wired.wired, 1, "the default audio track (id 1) wires");
        // Switch audio 1 → 2 under a fresh generation: track 1's
        // decoder parks (with its codec), track 2's takes the lane.
        let generation = GenerationId::FIRST.next().must();
        let outcome = pipeline
            .switch_limbs(
                generation,
                WiredKind::Audio,
                Some(TrackId::new(2)),
                &provider,
                &plan,
                8,
            )
            .must();
        assert!(outcome.decoder_built && !outcome.decoder_reused);
        // A seek mint reconfigures the live limb and every parked
        // decoder — each with the codec that rode its owner.
        let seek_generation = generation.next().must();
        pipeline.seek_flush(seek_generation, 8, 1).must();
        let configured = log.lock().must().clone();
        assert_eq!(
            configured,
            ["fake-pcm-b".to_string(), "fake-pcm-a".to_string()],
            "the live limb's codec first, then the parked decoder's own codec — never a fabricated fallback"
        );
        assert!(
            !configured.iter().any(|codec| codec == "unknown"),
            "the recycle path must not fabricate descriptors"
        );
    }
}
