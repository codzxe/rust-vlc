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
}

impl QueuePlan {
    /// A plan from explicit caps (both must be valid — they are the
    /// packet-queue constructor's own outputs).
    #[must_use]
    pub fn new(audio: QueueLimits, video: QueueLimits) -> QueuePlan {
        QueuePlan { audio, video }
    }

    /// The caps for a track kind. Audio/video map to their limbs;
    /// other kinds are not wired in v0 (subtitle/data limbs are owed
    /// to the card that needs them — 2.B1's recorded limitation) and
    /// the answer is only consulted for wired kinds.
    #[must_use]
    pub fn for_kind(&self, kind: MediaKind) -> QueueLimits {
        match kind {
            MediaKind::Video => self.video,
            _ => self.audio,
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
    /// 4 MiB `[PROPOSAL: the byte caps; the packet/duration figures
    /// are doc 03 §4.4's]`.
    fn default() -> QueuePlan {
        let one_mib = 1usize << 20;
        let four_mib = 4usize << 20;
        QueuePlan {
            audio: QueueLimits::audio_default(nonzero(one_mib)),
            video: QueueLimits::video_default(nonzero(four_mib)),
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
    /// high). `[PROPOSAL: packets are the axis — bytes/duration axes
    /// would fire first for unusual packet-size mixes, and the
    /// watermark policy is packets-driven in v0]`.
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

/// The wired pipeline: demuxer, limbs, and the tick driver. Exists
/// from session construction (holding the demuxer) until teardown.
pub(crate) struct Pipeline {
    /// The demux stage (plugin-api contract).
    demuxer: Box<dyn Demuxer>,
    /// The wired audio/video limbs.
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
    /// The in-flight seek's pre-roll progress (doc 03 §6 step 5),
    /// set by [`Pipeline::seek_flush`] and read by
    /// [`Pipeline::seek_preroll`]; `None` between seeks.
    seek_progress: Option<SeekProgress>,
    /// Frames discarded from a seek's pre-roll presentation hold
    /// (M10, D-4): a superseded or failed seek's held frames are
    /// stale by generation and never present — this is the rollback's
    /// observable cost, handed to the caller by
    /// [`Pipeline::take_preroll_discarded`].
    preroll_discarded: u32,
    /// Frames dropped as stale during the last seek flush (handed to
    /// the caller by [`Pipeline::take_seek_flush_stale`] for the
    /// session's observability totals).
    seek_flush_stale: u32,
    /// The limbs whose decoder newly flushed to `Drained` this tick
    /// (the `playback.end-of-track` candidates, card 3.B4) — a
    /// **reused** scratch (drained by the session after each tick,
    /// capacity retained), so the steady-state tick allocates
    /// nothing (engineering directive D-1).
    ended_limbs: Vec<(MediaKind, TrackId)>,
}

/// Per-limb facts the seek's pre-roll phase (doc 03 §6 step 5)
/// tracks: has this limb delivered a frame of the new generation,
/// the first audio frame's timestamp (the step-6 anchor truth), and
/// the M10 **presentation hold** — the bounded per-limb frame buffers
/// that keep pre-roll frames away from the consumer until the seek
/// terminal lands (doc 03 §6 "no `Playing`-looking output" during
/// steps 2–6; D-4: nothing leaks across the state boundary).
pub(crate) struct SeekProgress {
    /// Aligned with `limbs`: ≥ 1 frame delivered under the current
    /// generation.
    saw_frame: Vec<bool>,
    /// The first audio limb's first delivered frame, in ns (None
    /// until an audio frame arrives — then the landing anchors the
    /// clock instead).
    first_audio_ns: Option<i64>,
    /// The presentation hold, aligned with `limbs`: frames decoded
    /// during the pre-roll, in arrival order. Bounded by `hold_cap`
    /// (a hard per-limb capacity — D-1); the decode slice
    /// backpressures at the cap (packets stay in the bounded queues),
    /// so the only path past it is a single decode call's burst,
    /// which is dropped and counted, never buffered unboundedly.
    hold: Vec<Vec<Frame>>,
    /// The per-limb hard capacity of the hold (the pre-roll budget:
    /// the decode slice bound the seek flow already carries).
    hold_cap: usize,
}

impl SeekProgress {
    fn new(limbs: &[Limb], hold_cap: usize) -> SeekProgress {
        SeekProgress {
            saw_frame: vec![false; limbs.len()],
            first_audio_ns: None,
            // Pre-allocated once per seek (not per tick — D-1): pushes
            // within the capacity never allocate.
            hold: limbs.iter().map(|_| Vec::with_capacity(hold_cap)).collect(),
            hold_cap,
        }
    }

    /// Total frames currently held across all limbs.
    fn held_count(&self) -> u32 {
        self.hold
            .iter()
            .fold(0u32, |sum, slot| sum.saturating_add(slot.len() as u32))
    }

    /// The M10 hold slot one decode slice routes its frames through;
    /// `None` when the limb index is out of range (defensive — the
    /// hold is aligned with the limbs the window was opened for).
    fn hold_slot(&mut self, index: usize) -> Option<PrerollHoldSlot<'_>> {
        let cap = self.hold_cap;
        self.hold
            .get_mut(index)
            .map(|frames| PrerollHoldSlot { frames, cap })
    }

    /// Observe one limb's decode slice: frames delivered, and the
    /// first frame's stamp.
    fn observe(&mut self, index: usize, kind: MediaKind, frames: u32, first: Option<Stamp>) {
        if frames == 0 {
            return;
        }
        if let Some(slot) = self.saw_frame.get_mut(index) {
            *slot = true;
        }
        if kind == MediaKind::Audio
            && self.first_audio_ns.is_none()
            && let Some(stamp) = first
            && let Ok(ts) = stamp.timestamp()
            && let Ok(ns) = ts.as_nanos()
        {
            self.first_audio_ns = Some(crate::seek::nanos_i64(ns));
        }
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
            seek_progress: None,
            preroll_discarded: 0,
            seek_flush_stale: 0,
            ended_limbs: Vec::new(),
        }
    }

    /// The demuxed tracks and selection flags (valid after
    /// [`Pipeline::wire`]).
    pub(crate) fn tracks(&self) -> (&[Track], &[bool]) {
        (&self.tracks, &self.selected)
    }

    /// Wire the limbs for a started demuxer: one queue + one decoder
    /// per audio/video track the provider can build (doc 03 §4.6
    /// `Probing | format ≥ min_score | Demuxing` — the v0 open step).
    ///
    /// Returns the wired limb count; zero means nothing is decodable
    /// (the session turns that into the typed
    /// `NoDecodableTrack` failure).
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
        self.selected = Vec::with_capacity(self.tracks.len());
        let mut wired = 0;
        for track in &self.tracks {
            let wireable = matches!(track.kind, MediaKind::Audio | MediaKind::Video);
            if !wireable {
                self.selected.push(false);
                continue;
            }
            let limits = plan.for_kind(track.kind);
            let queue = Arc::new(BoundedQueue::new(limits, plan.thresholds(&limits)));
            let high_packets = plan.thresholds(&limits).high.packets.max(1);
            // Publish the live generation before any producer starts
            // (doc 03 §5 checking point 4): a fresh queue publishes
            // FIRST, so this flush is a no-op at session start and
            // the standing boundary check for every later mint.
            let _ = queue.flush(generation);
            let config = DecoderConfig::new(
                track.codec.clone(),
                generation,
                ThreadBudget::default(),
                None,
                None,
            );
            let decoder = provider.decoder_for(track, &config)?;
            self.limbs.push(Limb {
                track: track.clone(),
                queue,
                decoder,
                drained: false,
                high_packets,
                last_frame: None,
                conceal_failures: 0,
            });
            self.selected.push(true);
            wired += 1;
        }
        Ok(wired)
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
    /// An in-flight seek's held pre-roll frames die with the pipeline
    /// — counted, never silent (M10).
    pub(crate) fn teardown(&mut self, new_generation: GenerationId) {
        if let Some(progress) = self.seek_progress.take() {
            self.preroll_discarded = self.preroll_discarded.saturating_add(progress.held_count());
        }
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
    /// every decoder is flushed to `Drained` through a
    /// generation-checking sink (its old-generation reorder buffers
    /// are dropped and counted — doc 03 §5 checking point 5 — never
    /// presented) and then reconfigured for the new generation (the
    /// codec-api seek contract, `decode.rs`'s "drain path after seek
    /// generation switches"). A fresh [`SeekProgress`] opens the
    /// step-5 observation window **with its M10 presentation hold**
    /// (per-limb, hard-capped at `hold_cap` — the pre-roll budget).
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
        // Invalidate any previous observation window first: a
        // half-completed flush must not leave a superseded seek's
        // pre-roll progress armed (it would answer Ready from stale
        // data and keep the Paused decode carve-out alive). The
        // superseded seek's held frames die with it — discarded,
        // counted, never presented (M10/D-4).
        if let Some(progress) = self.seek_progress.take() {
            self.preroll_discarded = self.preroll_discarded.saturating_add(progress.held_count());
        }
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
        }
        self.seek_progress = Some(SeekProgress::new(&self.limbs, hold_cap));
        Ok(())
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
        let Some(progress) = self.seek_progress.as_ref() else {
            return SeekPreroll::NotReady;
        };
        if self.limbs.is_empty() {
            return SeekPreroll::NotReady;
        }
        for (index, limb) in self.limbs.iter().enumerate() {
            if progress.saw_frame.get(index).copied().unwrap_or(false) {
                continue;
            }
            if !self.can_never_deliver(limb) {
                return SeekPreroll::NotReady;
            }
        }
        SeekPreroll::Ready {
            audio_anchor_ns: progress.first_audio_ns,
        }
    }

    /// M10 — release the pre-roll hold at the seek terminal (doc 03
    /// §6 steps 6–7). The caller has already re-anchored the clock on
    /// the landing truth and published the seek's generation, so the
    /// held frames may finally present: they leave through the same
    /// [`FrameConsumer`] seam they would have used without the hold,
    /// in **pts order** (stable among equals — arrival order), every
    /// limb merged. The observation window closes with the release
    /// (the terminal is the window's end).
    ///
    /// A consumer refusal here is a bounded, counted loss (the
    /// discard counter), never a swallowed error and never an
    /// un-report of the already-emitted terminal: the seek completed
    /// truthfully; the sink's refusal is a separate degraded fact the
    /// host observes through the count (doc 03 §4.10 d — presentation
    /// faults degrade, they do not unwind playback state).
    pub(crate) fn release_preroll_hold(&mut self) {
        let Some(progress) = self.seek_progress.take() else {
            return;
        };
        // Merge the per-limb holds, then order by pts across limbs
        // (once per seek — the merge/sort is not a hot path). The
        // limb index rides along so a released video frame can latch
        // the concealment cache (M9) on the limb that owns it.
        let mut held: Vec<(usize, TrackId, Frame)> = Vec::new();
        for (index, (limb, slot)) in self.limbs.iter().zip(progress.hold).enumerate() {
            for frame in slot {
                held.push((index, limb.track.id, frame));
            }
        }
        // Missing stamps sort last (arrival order among equals — the
        // stable sort keeps it); a present stamp orders by its
        // nanoseconds on the media base.
        held.sort_by_key(|(_, _, frame)| stamp_ns(frame.pts()).unwrap_or(i64::MAX));
        let generation = self.generation;
        for (index, track, frame) in held {
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
                && let Some(limb) = self.limbs.get_mut(index)
            {
                limb.last_frame = Some(frame);
            }
        }
    }

    /// End the in-flight seek's observation window — completion,
    /// failure, or supersession: clears the pre-roll progress (and
    /// with it the Paused decode carve-out of [`Pipeline::tick`]).
    /// Any frames still in the hold are **discarded and counted**
    /// (M10, D-4): a seek that will never complete owns nothing
    /// presentable — its frames are stale by generation.
    pub(crate) fn end_seek(&mut self) {
        if let Some(progress) = self.seek_progress.take() {
            self.preroll_discarded = self.preroll_discarded.saturating_add(progress.held_count());
        }
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
        // one carve-out: while a seek is in flight, decode runs even
        // in Paused so the landing frame exists to re-anchor on (the
        // wire-visible state stays paused; the clock stays frozen
        // until the completion report). While the seek is in flight
        // the decode's frames enter the M10 presentation hold, not
        // the consumer — the carve-out produces evidence and held
        // frames, never a pre-terminal presentation.
        // [PROPOSAL: Lead decision 3.HARD-AUDIT D-a — there is no
        // paused→buffering wire edge (doc 05 §3.3), so the pre-roll
        // cannot ride the buffering episode while paused; revisit at
        // 3.C1 state-machine completeness.]
        if matches!(state, SessionState::Buffering | SessionState::Playing)
            || self.seek_progress.is_some()
        {
            self.decode_step(budget, &mut report);
        }
        // The drained check is pure state inspection and also runs
        // while Paused (the media may have fully drained just before
        // the pause landed — doc 05 §3.3's `paused → ended` edge).
        if matches!(state, SessionState::Playing | SessionState::Paused) {
            report.drained = self.is_drained();
        }
        if state == SessionState::Buffering {
            report.primed =
                !self.limbs.is_empty() && self.limbs.iter().all(|limb| limb.queue.watermark().high);
            // The in-episode progress: the bottleneck limb's fill
            // toward its high watermark (card 3.B4 — doc 05 §3.2's
            // `0.0..1.0` while buffering). The exit bookend `1.0` is
            // the table effect; this is the honest mid-episode view.
            report.buffer_fill = self
                .limbs
                .iter()
                .map(|limb| {
                    (limb.queue.usage().packets as f32 / limb.high_packets as f32).clamp(0.0, 1.0)
                })
                .fold(1.0_f32, f32::min);
        }
        if state == SessionState::Playing {
            // An underrun is an input that cannot keep up — the
            // end-of-stream drain is the media ending, not starving
            // (rebuffering at the end would be a lie).
            report.starved = !self.eof && self.limbs.iter().any(|limb| limb.queue.watermark().low);
        }
        report
    }

    /// End-of-stream with nothing left anywhere: no held packet, no
    /// resident queue items, every decoder flushed to `Drained`.
    fn is_drained(&self) -> bool {
        self.eof
            && self.pending.is_none()
            && !self.limbs.is_empty()
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
        let generation = self.generation;
        for (index, limb) in self.limbs.iter_mut().enumerate() {
            let track_id = limb.track.id;
            let limb_kind = limb.track.kind;
            let mut fatal: Option<EngineError> = None;
            // The slice's observed facts, hoisted out of the sink's
            // scope so the pre-roll hold borrow ends before the
            // observation merge below (the hold slot borrows the same
            // `seek_progress` the merge reads). Assigned exactly once
            // at the end of the scope below.
            let sink_frames: u32;
            let sink_first: Option<Stamp>;
            let sink_stale: u32;
            let sink_concealed: u32;
            let sink_last: Option<Frame>;
            {
                // M10: while a seek is in flight, the sink routes the
                // slice's frames into the per-limb hold — they reach
                // the consumer only at the seek terminal
                // (`release_preroll_hold`), never against the frozen
                // clock.
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
                    hold: self.seek_progress.as_mut().and_then(|p| p.hold_slot(index)),
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
            if self.first_frame.is_none()
                && let Some(position) = sink_first
            {
                self.first_frame = Some(position);
                report.first_frame = Some(position);
            }
            // The in-flight seek's pre-roll observation (doc 03 §6
            // step 5): frames delivered this slice, and the first
            // frame's stamp (the audio anchor truth for step 6).
            if let Some(progress) = self.seek_progress.as_mut() {
                progress.observe(index, limb_kind, sink_frames, sink_first);
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
