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
use media_types::{EngineError, Frame, GenerationId, MediaKind, Packet, Stamp, Track, TrackId};
use packet_queue::{
    BoundedQueue, PopOutcome, PushOutcome, QueueLimits, QueueUsage, Stamped, WatermarkThresholds,
};
use plugin_api::{DemuxEvent, Demuxer};

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
    /// `DamagedData`).
    pub damaged: u64,
    /// Frames delivered to the sink seam this tick.
    pub frames: u64,
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
    pub(crate) fn teardown(&mut self, new_generation: GenerationId) {
        for limb in &self.limbs {
            let _ = limb.queue.flush(new_generation);
            limb.queue.close();
        }
        self.limbs.clear();
        self.pending = None;
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
            damaged: 0,
            frames: 0,
            stale_frames: 0,
            discontinuities: 0,
        };
        // The demux step runs in every state that owns a live
        // pipeline (Buffering fills, Playing streams, Paused fills to
        // the watermark then parks — doc 03 §4.6's pause row).
        self.demux_step(budget.demux_events, &mut report);
        // Decode only where decoders are allowed to make progress;
        // Paused freezes the sinks (frames must not present).
        if matches!(state, SessionState::Buffering | SessionState::Playing) {
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
    fn decode_step(&mut self, budget: &TickBudget, report: &mut TickReport) {
        let consumer = Arc::clone(&self.consumer);
        let generation = self.generation;
        for limb in &mut self.limbs {
            let track_id = limb.track.id;
            let mut sink = LimbSink {
                consumer: Arc::clone(&consumer),
                track: track_id,
                generation,
                frames: 0,
                first: None,
                stale_dropped: 0,
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
                    Some(Some(packet)) => match limb.decoder.decode(packet, &mut sink) {
                        Ok(DecodeOutcome::Recovered { skipped, .. }) => {
                            report.damaged = report.damaged.saturating_add(u64::from(skipped));
                        }
                        Ok(_) => {}
                        Err(error) => {
                            fatal = Some(error);
                            break;
                        }
                    },
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
            // generation check is the doc 03 §5 point-5 checkpoint).
            report.frames = report.frames.saturating_add(u64::from(sink.frames));
            report.stale_frames = report
                .stale_frames
                .saturating_add(u64::from(sink.stale_dropped));
            if self.first_frame.is_none()
                && let Some(position) = sink.first
            {
                self.first_frame = Some(position);
                report.first_frame = Some(position);
            }
        }
    }
}

/// The per-limb [`FrameSink`]: the only path frames leave the engine
/// through in v0. Checks each frame's generation (doc 03 §5 checking
/// point 5 — stale frames never reach the consumer) and records the
/// first-frame timestamp.
struct LimbSink {
    consumer: Arc<dyn FrameConsumer>,
    track: TrackId,
    generation: GenerationId,
    frames: u32,
    first: Option<Stamp>,
    stale_dropped: u32,
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
        self.consumer.accept(frame, self.track)?;
        self.frames = self.frames.saturating_add(1);
        if self.first.is_none() {
            self.first = Some(pts);
        }
        Ok(())
    }
}
