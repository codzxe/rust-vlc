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
//!   cards 2.C3/2.C4).
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
//! frame no longer ends the session. A bounded run of consecutive
//! failures ([`VIDEO_CONCEALMENT_BUDGET`]) still reaches a typed
//! terminal, because a stream whose every frame fails is not playing.
//!
//! # Watermark policy
//!
//! Packet-queue *reports* watermarks; this crate (the engine) *owns
//! the policy* (2.B1's recorded hand-off). [`QueuePlan`] sets the
//! high threshold at three quarters of each cap `[PROPOSAL: v0
//! policy; the product.toml wiring is owed to card 3.B3]` and keeps
//! the low threshold at packet-queue's quarter default. Priming
//! (start) requires **every** enabled limb to be high; an underrun
//! fires when **any** limb runs low while the demuxer can still
//! refill — the end-of-stream drain is not an underrun.

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

/// Per-kind queue caps for the session's packet queues (doc 03
/// §4.4 defaults via packet-queue; the watermark policy lives here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueuePlan {
    /// Caps for audio limbs.
    pub audio: QueueLimits,
    /// Caps for video limbs.
    pub video: QueueLimits,
    /// Caps for subtitle limbs (card 3.A3): subtitle packets are tiny,
    /// sparse and latency-tolerant, so the caps sit far below the A/V
    /// defaults — `[PROPOSAL: doc-default constants; the product.toml
    /// wiring is owed to card 3.B3, which owns per-kind limits]`.
    pub subtitle: QueueLimits,
}

impl QueuePlan {
    /// A plan from explicit caps (both must be valid — they are the
    /// packet-queue constructor's own outputs). The subtitle axis
    /// takes the doc default (see [`QueuePlan::default`]).
    #[must_use]
    pub fn new(audio: QueueLimits, video: QueueLimits) -> QueuePlan {
        QueuePlan {
            audio,
            video,
            subtitle: subtitle_default_limits(),
        }
    }

    /// The caps for a track kind. Audio/video/subtitle map to their
    /// limbs; `Data` keeps no limb (dropped at push, the existing
    /// policy) — the answer is only consulted for wired kinds.
    #[must_use]
    pub fn for_kind(&self, kind: MediaKind) -> QueueLimits {
        match kind {
            MediaKind::Video => self.video,
            MediaKind::Audio => self.audio,
            MediaKind::Subtitle => self.subtitle,
            MediaKind::Data => self.audio,
        }
    }

    /// The engine's watermark thresholds for a queue built with
    /// `limits`: high at three quarters of every cap, low at one
    /// quarter (floored at one unit so degenerate caps still signal).
    ///
    /// `[PROPOSAL: v0 engine policy — packet-queue reports, the
    /// engine decides; product.toml wiring owed to card 3.B3]`
    #[must_use]
    pub fn thresholds(&self, limits: &QueueLimits) -> WatermarkThresholds {
        let quarter = |value: u64| (value / 4).max(1);
        let high = QueueUsage {
            packets: limits.max_packets.get() - (limits.max_packets.get() / 4),
            bytes: limits.max_bytes.get() - (limits.max_bytes.get() / 4),
            duration: media_types::Nanoseconds(limits.max_duration.0 - limits.max_duration.0 / 4),
        };
        let low = QueueUsage {
            packets: quarter(limits.max_packets.get() as u64) as usize,
            bytes: quarter(limits.max_bytes.get() as u64) as usize,
            duration: media_types::Nanoseconds(quarter(limits.max_duration.0)),
        };
        WatermarkThresholds::new(high, low)
    }
}

impl Default for QueuePlan {
    /// Doc 03 §4.4's defaults through packet-queue's constructors:
    /// audio 200 packets / 1 s / 1 MiB, video 300 packets / 2 s /
    /// 4 MiB, subtitle 64 packets / 30 s / 64 KiB `[PROPOSAL: the
    /// byte caps and the subtitle axis; the packet/duration figures
    /// are doc 03 §4.4's]`.
    fn default() -> QueuePlan {
        let one_mib = 1usize << 20;
        let four_mib = 4usize << 20;
        QueuePlan {
            audio: QueueLimits::audio_default(nonzero(one_mib)),
            video: QueueLimits::video_default(nonzero(four_mib)),
            subtitle: subtitle_default_limits(),
        }
    }
}

/// The subtitle axis's doc-default caps (see [`QueuePlan::default`]).
fn subtitle_default_limits() -> QueueLimits {
    QueueLimits {
        max_packets: nonzero(64),
        max_bytes: nonzero(64 << 10),
        max_duration: media_types::Nanoseconds(30 * 1_000_000_000),
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
    /// high). `[PROPOSAL: packets are the axis — bytes/duration axes
    /// would fire first for unusual packet-size mixes, and the
    /// watermark policy is packets-driven in v0]`.
    pub buffer_fill: f32,
    /// Limbs whose decoder newly flushed to `Drained` this tick (the
    /// `playback.end-of-track` candidates, card 3.B4) — the count;
    /// the `(kind, id)` pairs ride the pipeline's reused
    /// [`Pipeline::take_ended_limbs`] scratch.
    pub newly_drained: u32,
    /// Frames delivered to the sink seam this tick.
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
    /// A limb installed by a switch that has not yet delivered its
    /// first frame under that switch (card 3.A3 step 5): while a
    /// switch of this kind is in flight, the limb is exempt from the
    /// underrun check and from priming — a switch is not a
    /// starvation. Cleared on the first delivered frame.
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
    /// Consecutive per-frame damage failures on this limb. Reset by
    /// any `Ok` decode outcome; the run is bounded by
    /// [`VIDEO_CONCEALMENT_BUDGET`] (D-3: recovery is bounded, never
    /// an infinite freeze).
    conceal_failures: u32,
}

/// The per-frame video concealment budget: how many **consecutive**
/// damaged frames the engine may conceal (repeating the last good
/// frame / holding black) before the damage is declared catastrophic
/// and reaches a typed terminal.
///
/// The figure inherits doc 03 §4.3(d)'s bounded-recovery window for
/// corrupt-but-skippable demux data — "attempt resynchronization once
/// within a bounded window [PROPOSAL: 1 MiB or 1000 packets], else
/// fatal `Corrupt`" — applied to the decode stage's per-frame damage
/// (audit M9 / ED-FIX-4). Every concealment is observable (a
/// `DamagedData` fact + the coalesced non-fatal `playback.error`), so
/// the budget only draws the line where "playing" would become a lie:
/// a stream whose every frame fails is not playing.
pub const VIDEO_CONCEALMENT_BUDGET: u32 = 1000;

/// Whether `error` is **per-frame damage** — the doc 03 §4.5(d)
/// `CorruptPacket` class the engine isolates on video limbs instead
/// of tearing the session down (audit M9):
///
/// - [`ErrorKind::Corrupt`] — the bytes of *this* packet are
///   structurally broken or truncated;
/// - [`ErrorKind::Decoder { cause: Bitstream }`] — this packet's
///   entropy data violated the codec grammar.
///
/// Everything else stays fatal: `Unsupported` and `ResourceLimit` are
/// deterministic per stream (retrying the next frame cannot help),
/// `BadState` is contract misuse, and a future inter-frame codec that
/// *cannot* continue after damage must not report these kinds for it
/// (doc 03 §4.5(d): such decoders "must document that and emit
/// `Fatal` instead" — today's only video decoder, MJPEG, is
/// stateless per frame by construction).
fn per_frame_video_damage(error: &EngineError) -> bool {
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
}

/// One parked decoder instance (the per-session decoder cache, card
/// 3.A3 — see [`Pipeline::decoder_cache`]).
struct CachedDecoder {
    track: TrackId,
    decoder: Box<dyn Decoder>,
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
}

/// One watched track's delivery facts.
struct WatchEntry {
    track: TrackId,
    saw_frame: bool,
    /// The first delivered frame's pts (ns) — the switch step-6
    /// anchor truth for the new track.
    first_pts_ns: Option<i64>,
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
            seek_flush_stale: 0,
            ended_limbs: Vec::new(),
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
    /// Returns the wired limb count; zero means nothing is decodable
    /// (the session turns that into the typed `NoDecodableTrack`
    /// failure).
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
    ) -> Result<usize, EngineError> {
        self.generation = generation;
        self.tracks = tracks;
        self.selected = vec![false; self.tracks.len()];
        let mut wired = 0;
        for kind in [MediaKind::Video, MediaKind::Audio, MediaKind::Subtitle] {
            let Some(track) = crate::tracks::default_track(&self.tracks, kind) else {
                continue;
            };
            let track = track.clone();
            match self.obtain_decoder(&track, generation, provider) {
                Ok((decoder, _reused)) => {
                    if let Some(index) = self.tracks.iter().position(|t| t.id == track.id) {
                        self.selected[index] = true;
                    }
                    self.limbs
                        .push(self.build_limb(track, decoder, plan, generation));
                    wired += 1;
                }
                Err(_error) if kind == MediaKind::Subtitle => {
                    // The optional kind: deselect, never fail the
                    // open (the class-S seam's open-time half).
                    continue;
                }
                Err(error) => return Err(error),
            }
        }
        Ok(wired)
    }

    /// Build one wired limb: a fresh queue from the plan, published
    /// at `generation` (doc 03 §5 checking point 4 — a fresh queue
    /// publishes FIRST, so this flush is a no-op at session start and
    /// the standing boundary for every later mint).
    fn build_limb(
        &self,
        track: Track,
        decoder: Box<dyn Decoder>,
        plan: &QueuePlan,
        generation: GenerationId,
    ) -> Limb {
        let limits = plan.for_kind(track.kind);
        let queue = Arc::new(BoundedQueue::new(limits, plan.thresholds(&limits)));
        let high_packets = plan.thresholds(&limits).high.packets.max(1);
        let _ = queue.flush(generation);
        Limb {
            track,
            queue,
            decoder,
            drained: false,
            high_packets,
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
    /// limbs. The caller mints and interrupts *before* calling this.
    pub(crate) fn teardown(&mut self, new_generation: GenerationId) {
        for limb in &self.limbs {
            let _ = limb.queue.flush(new_generation);
            limb.queue.close();
        }
        self.limbs.clear();
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
    /// limbs (the step-5 watcher).
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
    ) -> Result<(), EngineError> {
        // Invalidate the seek's claim on any previous observation
        // window first: a half-completed flush must not leave a
        // superseded seek's pre-roll progress armed (it would answer
        // Ready from stale data and keep the Paused decode carve-out
        // alive). A switch still in flight keeps its claim — its
        // watched limb is a current limb and survives the rebuild.
        self.window_end(WindowOwner::SEEK);
        for limb in &self.limbs {
            let _ = limb.queue.flush(generation);
        }
        self.pending = None;
        self.eof = false;
        self.generation = generation;
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
                cached_track_codec(&self.tracks, cached.track),
                generation,
                ThreadBudget::default(),
                None,
                None,
            );
            cached.decoder.configure(&config)?;
        }
        self.window_open_seek();
        Ok(())
    }

    /// Open (or re-arm) the observation window for the in-flight
    /// seek: fresh per-track entries over the current limbs, the
    /// seek's owner bit set (a switch still in flight keeps its bit
    /// and its watched limb — the new limb of a swapped kind is a
    /// current limb, so the rebuild covers it).
    fn window_open_seek(&mut self) {
        let entries = self
            .limbs
            .iter()
            .map(|limb| WatchEntry {
                track: limb.track.id,
                saw_frame: false,
                first_pts_ns: None,
            })
            .collect::<Vec<_>>();
        match &mut self.window {
            Some(window) => {
                window.saw = entries;
                window.first_audio_ns = None;
                window.owners = window.owners.union(WindowOwner::SEEK);
            }
            None => {
                self.window = Some(ObserveWindow {
                    saw: entries,
                    first_audio_ns: None,
                    owners: WindowOwner::SEEK,
                });
            }
        }
    }

    /// Arm the window for an in-flight switch of `kind`: the owner
    /// bit is claimed and the new track (when one was selected) is
    /// watched — added to the existing entries so an in-flight seek
    /// keeps its own watches (the seek completes on the new track's
    /// first frame).
    fn window_arm_switch(&mut self, kind: MediaKind, track: Option<TrackId>) {
        let owner = WindowOwner::switch_for(kind);
        match &mut self.window {
            Some(window) => {
                window.owners = window.owners.union(owner);
                if let Some(track) = track
                    && !window.saw.iter().any(|entry| entry.track == track)
                {
                    window.saw.push(WatchEntry {
                        track,
                        saw_frame: false,
                        first_pts_ns: None,
                    });
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

    /// End the in-flight seek's observation window — completion,
    /// failure, or supersession: releases the seek's claim (and with
    /// the last owner, the Paused decode carve-out of
    /// [`Pipeline::tick`]). A switch still in flight keeps the window
    /// alive for its own completion.
    pub(crate) fn end_seek(&mut self) {
        self.window_end(WindowOwner::SEEK);
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
        kind: MediaKind,
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
        if let Some(position) = self.limbs.iter().position(|limb| limb.track.kind == kind) {
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
            let mut limb = self.build_limb(track.clone(), decoder, plan, generation);
            limb.first_frame_pending = true;
            let watched = track.id;
            self.limbs.push(limb);
            // 5. The demux stamp advances with the mint.
            self.generation = generation;
            // 6. Arm the window.
            self.window_arm_switch(kind, Some(watched));
        } else {
            self.generation = generation;
            self.window_arm_switch(kind, None);
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
    /// - A limb installed by a switch that has not yet delivered its
    ///   first frame does not pace **while its switch is in flight**
    ///   — a switch is not a starvation; without the carve-out every
    ///   audio switch from `Playing` would flash
    ///   playing→buffering→playing (the new queue starts empty).
    fn paces(&self, limb: &Limb) -> bool {
        if limb.track.kind == MediaKind::Subtitle {
            return false;
        }
        if limb.first_frame_pending {
            let switching = WindowOwner::switch_for(limb.track.kind);
            if self
                .window
                .as_ref()
                .is_some_and(|window| window.owners.contains(switching))
            {
                return false;
            }
        }
        true
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
        // report).
        // [PROPOSAL: Lead decision 3.HARD-AUDIT D-a — there is no
        // paused→buffering wire edge (doc 05 §3.3), so the pre-roll
        // cannot ride the buffering episode while paused; revisit at
        // 3.C1 state-machine completeness. 3.A3 extends the same
        // carve-out to its switch windows.]
        if matches!(state, SessionState::Buffering | SessionState::Playing) || self.window_live() {
            self.decode_step(budget, &mut report);
        }
        // The drained check is pure state inspection and also runs
        // while Paused (the media may have fully drained just before
        // the pause landed — doc 05 §3.3's `paused → ended` edge).
        if matches!(state, SessionState::Playing | SessionState::Paused) {
            report.drained = self.is_drained();
        }
        if state == SessionState::Buffering {
            // Priming waits for every pacing limb's high watermark
            // (subtitles and in-flight-switch limbs are exempt — see
            // [`Pipeline::paces`]).
            report.primed = !self.limbs.is_empty()
                && self
                    .limbs
                    .iter()
                    .filter(|limb| self.paces(limb))
                    .all(|limb| limb.queue.watermark().high);
            // The in-episode progress: the bottleneck pacing limb's
            // fill toward its high watermark (card 3.B4 — doc 05
            // §3.2's `0.0..0.1` while buffering). The exit bookend
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
            // starvation (the carve-out).
            report.starved = !self.eof
                && self
                    .limbs
                    .iter()
                    .filter(|limb| self.paces(limb))
                    .any(|limb| limb.queue.watermark().low);
        }
        report
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
            PushOutcome::Full { item, .. } => {
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
    /// point 5).
    ///
    /// # Per-frame damage isolation (doc 03 §4.5(d), audit M9)
    ///
    /// A video packet that fails with a per-frame damage kind
    /// ([`per_frame_video_damage`]) does **not** end the session: the
    /// damage is counted into the tick's `damaged` facts (the engine
    /// `DamagedData` event + the coalesced non-fatal `playback.error`,
    /// card 3.B4 — the same Recovered-class shape the PCM decoder
    /// already reports in-band), the presentation conceals by
    /// repeating the limb's last presented frame at the damaged
    /// slot's pts (or holds black when no current-generation frame
    /// exists), and the slice continues. The run of consecutive
    /// failures is bounded by [`VIDEO_CONCEALMENT_BUDGET`]; past it,
    /// the damage is catastrophic and reaches a typed terminal
    /// (D-3: one corrupt frame must not kill the session, but a
    /// stream of nothing but corrupt frames must not fake playing
    /// either).
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
            let mut sink = LimbSink {
                consumer: Arc::clone(&consumer),
                track: track_id,
                generation,
                frames: 0,
                first: None,
                stale_dropped: 0,
                last: limb.last_frame.clone(),
                concealed: 0,
            };
            let mut fatal: Option<EngineError> = None;
            for _ in 0..budget.packets_per_limb {
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
                                    report.damaged_decode =
                                        report.damaged_decode.saturating_add(u64::from(skipped));
                                    report.damaged =
                                        report.damaged.saturating_add(u64::from(skipped));
                                }
                            }
                            Err(error) => {
                                if limb_kind == MediaKind::Video && per_frame_video_damage(&error) {
                                    limb.conceal_failures = limb.conceal_failures.saturating_add(1);
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
                                    report.damaged_decode = report.damaged_decode.saturating_add(1);
                                    report.damaged = report.damaged.saturating_add(1);
                                    // Conceal by repeating the last
                                    // presented frame at this slot's
                                    // pts — or holding (nothing
                                    // presented) when no
                                    // current-generation frame
                                    // exists. A consumer refusal is
                                    // the sink contract's fatal.
                                    if let Err(error) = sink.conceal_last(pts) {
                                        fatal = Some(error);
                                        break;
                                    }
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
            // reports `Drained`, bounded per tick.
            if fatal.is_none() && self.eof && !limb.drained {
                let queue_empty = limb.queue.usage().packets == 0;
                if queue_empty && self.pending.is_none() {
                    for _ in 0..budget.flush_calls_per_limb {
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
            report.frames = report.frames.saturating_add(u64::from(sink.frames));
            report.concealed_frames = report
                .concealed_frames
                .saturating_add(u64::from(sink.concealed));
            report.stale_frames = report
                .stale_frames
                .saturating_add(u64::from(sink.stale_dropped));
            // The concealment cache follows the slice back into the
            // limb (the sink's latch or the frame it still holds).
            limb.last_frame = sink.last.take();
            // The first delivered frame under a switch clears the
            // re-prime exemption (the limb paces progress again).
            if sink.frames > 0 {
                limb.first_frame_pending = false;
            }
            if self.first_frame.is_none()
                && let Some(position) = sink.first
            {
                self.first_frame = Some(position);
                report.first_frame = Some(position);
            }
            // The in-flight re-prime observation (doc 03 §6 step 5 /
            // card 3.A3 step 5): frames delivered this slice, and the
            // first frame's stamp (the anchor truth for the seek's
            // audio re-anchor and the switch's new-track re-anchor).
            if let Some(window) = self.window.as_mut() {
                window.observe(track_id, limb_kind, sink.frames, sink.first);
            }
        }
    }
}

/// The codec descriptor of a cached track — the configure call the
/// recycle path needs. A cache entry always has a live `tracks` row
/// (the cache is bounded by the source's track list), so the
/// fallback descriptor is the unreachable-by-construction guard.
fn cached_track_codec(tracks: &[Track], track: TrackId) -> media_types::ids::CodecDescriptor {
    tracks
        .iter()
        .find(|candidate| candidate.id == track)
        .map_or_else(empty_codec_descriptor, |candidate| candidate.codec.clone())
}

/// The unreachable-by-construction fallback descriptor for a cached
/// track with no live row (kept total, never panics).
fn empty_codec_descriptor() -> media_types::ids::CodecDescriptor {
    media_types::ids::CodecDescriptor::new("unknown").unwrap_or_else(|error| {
        unreachable!("the literal `unknown` is a valid descriptor: {error}")
    })
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
/// first-frame timestamp. The sink also owns the slice's half of the
/// per-frame-damage concealment cache (see [`Pipeline::decode_step`]):
/// the last presented video frame, latched on `accept` and written
/// back into the limb after the slice.
struct LimbSink {
    consumer: Arc<dyn FrameConsumer>,
    track: TrackId,
    generation: GenerationId,
    frames: u32,
    first: Option<Stamp>,
    stale_dropped: u32,
    /// The last presented video frame of the current generation (the
    /// concealment cache's slice-local half; `None` until a video
    /// frame is presented under this generation).
    last: Option<Frame>,
    /// Concealment presentations this slice (repeated frames —
    /// labeled, never counted as decoded output).
    concealed: u32,
}

impl LimbSink {
    /// Conceal one per-frame-damaged video slot (doc 03 §4.5(d),
    /// audit M9): re-present the latched last-good frame at `pts` —
    /// the **same pixels again**, never new content. Counts
    /// `concealed`, not `frames`; never sets `first` (a repeated
    /// frame is not a first frame) and never latches itself. With no
    /// latched current-generation frame the slot **holds**: nothing
    /// is presented, the damage counts are the observable.
    ///
    /// # Errors
    /// The consumer's typed [`EngineError`] — a refusal downstream is
    /// the [`FrameSink`] propagation contract's fatal, exactly as for
    /// decoded frames.
    fn conceal_last(&mut self, pts: Stamp) -> Result<(), EngineError> {
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
}

impl FrameSink for LimbSink {
    fn accept(&mut self, frame: Frame) -> Result<(), EngineError> {
        let pts = frame.pts();
        if frame.is_stale(self.generation) {
            // A stale frame is dropped on sight — no error, no
            // consumer call, counted (doc 03 §5 point 5).
            self.stale_dropped = self.stale_dropped.saturating_add(1);
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
