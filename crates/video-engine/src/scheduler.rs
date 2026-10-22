//! The presentation scheduler (card 2.C4; doc 03 §4.10 a/c/e): the
//! state machine that decides *which frame is on screen when*.
//!
//! # Shape (doc 03 §4.10 c, adapted for v0)
//!
//! The doc places one vsync-paced scheduler thread in production. That
//! thread is session wiring (the playback engine's, Phase 3): it hosts
//! this state machine and calls [`PresentationScheduler::tick`] once
//! per vsync or per timer wake. v0 delivers the deterministic core —
//! single-threaded, explicitly stepped, all time injected through
//! [`media_clock::PlaybackClock`] — the same code paths the threaded
//! wiring will drive (doc 03 §8's single-threaded-harness discipline).
//!
//! # Per-tick algorithm (doc 03 §4.10 e, binding order)
//!
//! 1. **Generation check** (doc 03 §5 checking point 7): the clock
//!    snapshot's generation is the live one; every pending frame whose
//!    stamp differs is dropped wholesale and counted, and the
//!    last-presented request is invalidated (a repeat *is* a present —
//!    stale frames never reappear).
//! 2. **Late purge**: pending frames with `pts < now − threshold` are
//!    dropped, counted, advanced past. The window is PTS-sorted, so the
//!    late frames are a prefix.
//! 3. **Selection**: present the frame with the **largest**
//!    `pts ≤ now + target_lead`. Everything due before it is
//!    *superseded* — dropped and counted, because presenting it after
//!    the selected frame would violate in-order presentation (the
//!    structural drop applies under every policy, [`DropPolicy::Never`]
//!    included; it is not a lateness drop).
//! 4. **Nothing due**: repeat the last frame (bounded by
//!    [`SchedulerCfg::max_repeat`]), then fall back to background —
//!    "video is subordinate to the audio clock, always" (doc 03
//!    §4.10 e).
//!
//! Missing presentation timestamps are never invented silently
//! (media-types `TimeError::Missing` contract): infer from the previous
//! sequence position + nominal frame duration when both are known (an
//! observable, counted event), else hold the frame until they are.
//!
//! # Pause
//!
//! A paused clock freezes `now()`, so nothing new becomes due and the
//! repeat budget drains to background mechanically. Whether to keep
//! ticking (and when to extend repeats) while paused is the session
//! wiring's decision — v0 `tick` is mechanical and does not special-case
//! [`ClockState::Paused`](media_clock::ClockState::Paused).

use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;

use media_clock::NANO_BASE;
use media_clock::sync::{DriftEstimator, SyncBand, SyncBands};
use media_clock::{ClockState, PlaybackClock};
use media_types::GenerationId;
use media_types::frame::VideoFrame;
use media_types::time::{Nanoseconds, Stamp, TimeError, Timestamp};

use crate::policy::SchedulerCfg;
use crate::sink::PresentRequest;

/// The result of offering a decoded frame to the scheduler.
///
/// Every variant that hands the frame back (`WindowFull`, `Stale`,
/// `UnschedulablePts`) leaves it untouched: the scheduler never
/// silently swallows fresh data — the producer reacts, retries or drops
/// it on its own books (the packet-queue Q3 discipline, applied to
/// frames).
#[must_use = "the outcome carries the frame back on every refusal path; ignoring it can lose a frame"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushOutcome {
    /// Accepted into the scheduling window at its own PTS.
    Scheduled,
    /// Accepted with an **inferred** PTS (the stamp was missing; the
    /// previous sequence position + nominal frame duration supplied
    /// one — the observable inference event of the media-types
    /// `TimeError::Missing` contract, counted in
    /// [`SchedulerStats::inferred_pts`]).
    Inferred {
        /// The inferred slot, on the clock's nanosecond base.
        pts: Timestamp,
    },
    /// The stamp was missing and cannot yet be inferred: the frame is
    /// held (at most one — a newer missing-stamp frame replaces an
    /// older held one, which is dropped and counted). Admission is
    /// retried on later ticks.
    Held,
    /// A missing-stamp frame was dropped: the window was full at
    /// admission, or it replaced an older held frame's slot. Counted in
    /// [`SchedulerStats::missing_pts_dropped`].
    MissingDropped {
        /// The window bound that refused admission.
        capacity: usize,
    },
    /// The reorder window is at capacity. The frame is handed back
    /// untouched; counted in [`SchedulerStats::full_rejections`].
    WindowFull {
        /// The untouched frame, handed back.
        frame: VideoFrame,
        /// The window bound that refused the push.
        capacity: usize,
    },
    /// The frame's generation is behind the live one — dropped at the
    /// boundary (doc 03 §5 checking point 7), counted in
    /// [`SchedulerStats::stale_rejected_push`]. The frame is handed
    /// back untouched; the producer must stop, not retry.
    Stale {
        /// The untouched frame, handed back.
        frame: VideoFrame,
        /// The live generation the frame fell behind.
        current: GenerationId,
    },
    /// The frame's PTS cannot be represented on the clock's nanosecond
    /// base (checked conversion failed — corrupt timing, doc 01 §3.2).
    /// Counted in [`SchedulerStats::unschedulable_pts`]; the frame is
    /// handed back untouched for the producer's own accounting.
    UnschedulablePts {
        /// The untouched frame, handed back.
        frame: VideoFrame,
        /// The conversion failure, verbatim.
        error: TimeError,
    },
}

/// The screen action one tick decided (doc 03 §4.10 e).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresentAction {
    /// A due frame was selected — put it on screen now (doc 01 §2.2
    /// `Present`).
    Present(PresentRequest),
    /// Nothing is due; the last presented frame stays on screen (doc 03
    /// §4.10 e "repeat the last frame"). The request is the same one
    /// originally presented; surface-persisting renderers may treat the
    /// repeat as a no-op redraw (see [`Presenter`]).
    RepeatLast(PresentRequest),
    /// No frame should be on screen: nothing was ever presented, or the
    /// repeat budget is spent (doc 03 §4.10 e "then background"). The
    /// presenter shows background (+ subtitles if any — doc 03 §4.10 d).
    Background,
}

/// The drops one tick performed — skips are **observable events, never
/// silent** (doc 01 §1.3 rule 9). Lifetime totals live in
/// [`SchedulerStats`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TickDrops {
    /// PTS (nanosecond base) of the frames the late policy dropped this
    /// tick, in window order (ascending).
    pub late: Vec<Timestamp>,
    /// PTS of the frames dropped because a later frame was selected
    /// over them (the in-order structural drop — see the [module
    /// docs](self) step 3), ascending.
    pub superseded: Vec<Timestamp>,
    /// How many generation-stale frames were purged wholesale this tick
    /// (doc 03 §5 checking point 7), window and held-missing together.
    pub stale: u64,
}

/// One scheduler tick's full report: the action, the wait hint for
/// timer-driven drivers, every drop it performed, and the A/V sync
/// drift sample (card 3.B1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickOutcome {
    /// What to put on screen this tick.
    pub action: PresentAction,
    /// The earliest clock position at which the next pending frame
    /// becomes due (`None` when nothing is pending) — the sleep target
    /// for a timer-driven wiring (doc 01 §2.2 `WaitUntil`).
    pub next_due: Option<Timestamp>,
    /// Every drop this tick performed (observable, doc 01 §1.3 rule 9).
    pub drops: TickDrops,
    /// The A/V sync drift observation for this tick (card 3.B1;
    /// additive field — pure observability, never an event).
    pub sync: SyncSample,
}

/// One tick's A/V sync drift observation (card 3.B1; W1-18 §2.1):
/// the audio-master presentation error measured at each present,
/// its filtered EMA, the correction band in force, and the effective
/// late threshold after the ladder's bounded widening. All
/// counters-not-events (the "no event storms" policy, §5 of the
/// design: observability is polled, never pushed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncSample {
    /// This tick's raw presentation error `e_v = pts − now` (ns),
    /// when a frame presented on a running clock. `None` otherwise
    /// (no present, or a paused clock — samples are discarded, not
    /// smeared).
    pub error_ns: Option<i64>,
    /// The filtered error EMA after this tick's sample, when primed.
    pub error_ema_ns: Option<i64>,
    /// The correction band in force (the Schmitt-trigger ladder).
    pub band: SyncBand,
    /// Whether this tick's present was a snap (a PTS-gap or
    /// re-anchor exemption — counted in
    /// [`SchedulerStats::snap_presents`], never in
    /// [`SchedulerStats::dropped_late`]).
    pub snapped: bool,
    /// The effective late threshold after the ladder's widening (ns),
    /// when the configuration drops for lateness at all.
    pub late_threshold_ns: Option<u64>,
}

impl Default for SyncSample {
    fn default() -> SyncSample {
        SyncSample {
            error_ns: None,
            error_ema_ns: None,
            band: SyncBand::Hold,
            snapped: false,
            late_threshold_ns: None,
        }
    }
}

/// Lifetime counters for observability (doc 01 §2.2 e, MP §11
/// dropped-frame metrics).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SchedulerStats {
    /// Frames admitted to the window (own PTS or inferred).
    pub pushed: u64,
    /// Admissions that used an inferred PTS.
    pub inferred_pts: u64,
    /// Due frames selected for presentation.
    pub presented: u64,
    /// Repeat presents emitted within the budget.
    pub repeats: u64,
    /// Background actions emitted (nothing due, no repeat left).
    pub background: u64,
    /// Frames dropped by the late policy.
    pub dropped_late: u64,
    /// Frames dropped as superseded by the selected frame.
    pub dropped_superseded: u64,
    /// Frames dropped wholesale at generation changes (doc 03 §5(7)).
    pub dropped_stale: u64,
    /// Pushes refused at the boundary for a stale generation.
    pub stale_rejected_push: u64,
    /// Pushes refused at the window bound.
    pub full_rejections: u64,
    /// Pushes refused as unschedulable (unrepresentable PTS).
    pub unschedulable_pts: u64,
    /// Missing-stamp frames taken into the hold slot.
    pub missing_held: u64,
    /// Held or admission-refused missing-stamp frames dropped.
    pub missing_pts_dropped: u64,
    /// First presents after a PTS gap or a clock re-anchor, exempt
    /// from late-drop accounting (card 3.B1; doc 03 §6 step 6's "no
    /// drop-count penalty for the first frame after a seek",
    /// generalized to non-seek jumps).
    pub snap_presents: u64,
    /// A/V error samples taken (one per present on a running clock).
    pub sync_samples: u64,
    /// Lifetime maximum `|e_v|` (ns, saturating) — the doc 03 §4.10 f
    /// A/V-sync tolerance's measurable.
    pub sync_error_abs_max_ns: u64,
    /// Lifetime `Σ e_v` (ns; i128 so long runs cannot overflow).
    pub sync_error_sum: i128,
    /// PTS-gap snaps armed at the push boundary (doc 03 §4.7 e's
    /// discontinuity tier, scheduler-side detection).
    pub sync_pts_gaps: u64,
    /// Estimator resets from clock re-anchors (anchor-epoch bumps).
    pub sync_reanchors: u64,
}

/// One window entry: the frame plus its normalized slot.
struct Pending {
    pts: Timestamp,
    frame: VideoFrame,
}

/// The presentation-scheduler state machine (see the [module
/// docs](self) for the algorithm and its doc 03 §4.10 anchors).
pub struct PresentationScheduler {
    cfg: SchedulerCfg,
    clock: Arc<dyn PlaybackClock>,
    /// The live generation as of the last snapshot (push-boundary
    /// check; refreshed every tick).
    generation: GenerationId,
    /// Pending frames, sorted by PTS ascending (stable among equals —
    /// arrival order). Bounded by `cfg.max_pending`.
    window: VecDeque<Pending>,
    /// A missing-stamp frame waiting for inference to become possible
    /// (at most one; media-types `TimeError::Missing` contract: "else
    /// hold").
    held_missing: Option<VideoFrame>,
    /// The last presented request, for repeats.
    last: Option<PresentRequest>,
    /// Consecutive repeats since the last present.
    repeats: usize,
    /// The A/V sync drift seams' runtime state (card 3.B1): the
    /// estimator, the band machine, the epoch tracker, snap mode and
    /// the ladder's bounded actuators. Fixed-size, zero-allocation.
    sync: SchedulerSync,
    stats: SchedulerStats,
}

/// The runtime half of the 3.B1 drift seams (the parameters live in
/// [`SchedulerCfg::sync`]; this is the mutable state the tick feeds).
/// Every field is a plain integer or a fixed-size pure type from
/// media-clock — no allocation, no growth (D-1).
struct SchedulerSync {
    /// The presentation-error EMA (W1-18 §2.1).
    estimator: DriftEstimator,
    /// The Schmitt-trigger band machine (W1-18 §3.1).
    bands: SyncBands,
    /// The last-seen clock anchor epoch — the mid-stream re-anchor
    /// detector (a bump is a new timing regime: reset + snap).
    last_anchor_epoch: Option<u64>,
    /// A present is exempt from late-drop accounting (armed by a PTS
    /// gap or an anchor-epoch bump; cleared by the first present).
    snap_pending: bool,
    /// Skip the late purge for exactly this tick (armed together with
    /// `snap_pending`: a backward re-anchor would otherwise
    /// late-purge the entire fresh window before the clock catches
    /// up — W1-18 §4 tier 3).
    purge_skip: bool,
    /// The tick counter (decision-cadence and window-keying basis).
    tick_index: u64,
    /// The widened effective late threshold (the WidenLate actuator),
    /// `None` while the configured threshold is in force.
    widened: Option<Nanoseconds>,
    /// The repeat-budget extension (the ExtendEarly actuator).
    repeat_extension: u32,
    /// Late drops in the current drop-budget window.
    drops_in_window: u32,
    /// The drop-budget window's opening tick.
    drop_window_start: u64,
    /// Threshold changes in the current rate window.
    changes_in_window: u32,
    /// The rate window's opening tick.
    change_window_start: u64,
}

impl SchedulerSync {
    fn new(cfg: &SchedulerCfg) -> SchedulerSync {
        SchedulerSync {
            estimator: DriftEstimator::new(),
            bands: cfg.sync.bands(),
            last_anchor_epoch: None,
            snap_pending: false,
            purge_skip: false,
            tick_index: 0,
            widened: None,
            repeat_extension: 0,
            drops_in_window: 0,
            drop_window_start: 0,
            changes_in_window: 0,
            change_window_start: 0,
        }
    }

    /// A new timing regime: clear the estimator and the band machine
    /// (the snap flags are the caller's to arm — they carry meaning
    /// the reset must not drop).
    fn reset_estimators(&mut self, cfg: &SchedulerCfg) {
        self.estimator.reset();
        self.bands = cfg.sync.bands();
        self.widened = None;
        self.repeat_extension = 0;
        self.drops_in_window = 0;
        self.changes_in_window = 0;
    }

    /// Account one late drop against the budget window (tumbling,
    /// tick-calibrated — the scheduler has no time source).
    fn note_late_drop(&mut self, policy: &crate::policy::SyncPolicy) {
        self.rotate_windows(policy);
        self.drops_in_window = self.drops_in_window.saturating_add(1);
    }

    /// Rotate the tumbling budget/rate windows (called per tick and
    /// per drop; both are `O(1)` compares).
    fn rotate_windows(&mut self, policy: &crate::policy::SyncPolicy) {
        if self.tick_index.saturating_sub(self.drop_window_start)
            >= u64::from(policy.drop_window_ticks)
        {
            self.drops_in_window = 0;
            self.drop_window_start = self.tick_index;
        }
        if self.tick_index.saturating_sub(self.change_window_start)
            >= u64::from(policy.threshold_window_ticks)
        {
            self.changes_in_window = 0;
            self.change_window_start = self.tick_index;
        }
    }
}

impl fmt::Debug for PresentationScheduler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The clock handle is not debug-printable (object trait); its
        // identity is not diagnostic state.
        f.debug_struct("PresentationScheduler")
            .field("cfg", &self.cfg)
            .field("generation", &self.generation)
            .field("pending", &self.window.len())
            .field("holds_missing", &self.held_missing.is_some())
            .field("last", &self.last)
            .field("repeats", &self.repeats)
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

impl PresentationScheduler {
    /// Build a scheduler over an injected clock. The initial live
    /// generation is read from the clock's snapshot (the engine minted
    /// it before wiring the scheduler; [`GenerationId::FIRST`] is the
    /// clock's default).
    #[must_use = "a scheduler nobody ticks presents nothing"]
    pub fn new(cfg: SchedulerCfg, clock: Arc<dyn PlaybackClock>) -> PresentationScheduler {
        let generation = clock.snapshot().generation;
        PresentationScheduler {
            cfg,
            clock,
            generation,
            window: VecDeque::new(),
            held_missing: None,
            last: None,
            repeats: 0,
            sync: SchedulerSync::new(&cfg),
            stats: SchedulerStats::default(),
        }
    }

    /// Offer one decoded frame (doc 03 §4.10 a "consume decoded
    /// frames"). See [`PushOutcome`] for the acceptance and refusal
    /// shapes; the window is bounded, so a burst beyond
    /// [`SchedulerCfg::max_pending`] is refused, never absorbed.
    pub fn push(&mut self, frame: VideoFrame) -> PushOutcome {
        // Boundary check first (packet-queue Q2 semantics): strictly
        // older is stale; equal flows; newer waits for the clock to
        // publish it.
        if frame.generation < self.generation {
            self.stats.stale_rejected_push += 1;
            return PushOutcome::Stale {
                frame,
                current: self.generation,
            };
        }
        match frame.pts {
            Stamp::Present(ts) => match ts.convert(&NANO_BASE) {
                Ok(pts) => {
                    if self.window.len() >= self.cfg.max_pending.get() {
                        self.stats.full_rejections += 1;
                        return PushOutcome::WindowFull {
                            frame,
                            capacity: self.cfg.max_pending.get(),
                        };
                    }
                    // 3.B1 discontinuity seam (doc 03 §4.7 e's PTS-jump
                    // tier, scheduler-side): an incoming PTS that
                    // jumps from the window's tail by at least the
                    // snap threshold arms snap mode — the next present
                    // is exempt from late-drop accounting — and
                    // restarts the error estimator (a new regime).
                    // The window is PTS-sorted; ordering needs no
                    // help. The threshold is wiring-configurable
                    // (`SchedulerCfg::with_sync`), default ½ the doc
                    // 03 §4.4 audio queue duration cap.
                    if let Some(tail) = self.window.back() {
                        let gap = i128::from(pts.ticks) - i128::from(tail.pts.ticks);
                        if gap.abs() >= i128::from(self.cfg.sync.pts_gap_threshold.0) {
                            self.sync.reset_estimators(&self.cfg);
                            self.sync.snap_pending = true;
                            self.sync.purge_skip = true;
                            self.stats.sync_pts_gaps += 1;
                        }
                    }
                    insert_sorted(&mut self.window, Pending { pts, frame });
                    self.stats.pushed += 1;
                    PushOutcome::Scheduled
                }
                Err(error) => {
                    self.stats.unschedulable_pts += 1;
                    PushOutcome::UnschedulablePts { frame, error }
                }
            },
            Stamp::Missing => {
                let base = self.sequence_base();
                match (base, self.cfg.frame_duration) {
                    (Some(base), Some(Nanoseconds(dur))) => {
                        if self.window.len() >= self.cfg.max_pending.get() {
                            self.stats.missing_pts_dropped += 1;
                            return PushOutcome::MissingDropped {
                                capacity: self.cfg.max_pending.get(),
                            };
                        }
                        let pts = Timestamp::new(saturating_add(base, dur), NANO_BASE);
                        insert_sorted(&mut self.window, Pending { pts, frame });
                        self.stats.pushed += 1;
                        self.stats.inferred_pts += 1;
                        PushOutcome::Inferred { pts }
                    }
                    _ => {
                        if self.held_missing.is_some() {
                            self.stats.missing_pts_dropped += 1;
                        }
                        self.held_missing = Some(frame);
                        self.stats.missing_held += 1;
                        PushOutcome::Held
                    }
                }
            }
        }
    }

    /// One scheduling tick: read the clock, apply the policy, decide
    /// the screen action (doc 03 §4.10 e — see the [module docs](self)
    /// for the binding order). Drives nothing itself: the wiring
    /// enqueues the action's request (if any) on the presentation
    /// queue and drains that to the presenter.
    ///
    /// The 3.B1 drift seams run inside the same order: the
    /// anchor-epoch check (a re-anchor is a new timing regime), the
    /// presentation-error sample on each present, and the
    /// hysteresis-ladder application at the [`SyncPolicy::
    /// decide_every`] cadence — all counters, never events.
    #[must_use = "a tick outcome ignored is a frame presented late, dropped silently, or never shown"]
    pub fn tick(&mut self) -> TickOutcome {
        let snapshot = self.clock.snapshot();
        let now = snapshot.position;
        let mut drops = TickDrops::default();
        let mut sync_sample = SyncSample::default();

        // 3.B1 seam 1 — anchor-epoch tracking: a bump (reanchor,
        // set_rate, resume) is a new timing regime. Reset the
        // estimator, arm the snap exemption, and skip the late purge
        // for this one tick: a backward re-anchor would otherwise
        // late-purge the entire fresh window before the clock catches
        // up (W1-18 §4 tier 3).
        match self.sync.last_anchor_epoch {
            None => self.sync.last_anchor_epoch = Some(snapshot.anchor_epoch),
            Some(last) if last != snapshot.anchor_epoch => {
                self.sync.last_anchor_epoch = Some(snapshot.anchor_epoch);
                self.sync.reset_estimators(&self.cfg);
                self.sync.snap_pending = true;
                self.sync.purge_skip = true;
                self.stats.sync_reanchors += 1;
            }
            Some(_) => {}
        }

        // Step 1: generation change → wholesale purge (doc 03 §5(7)).
        if snapshot.generation != self.generation {
            let current = snapshot.generation;
            let before = self.window.len() as u64;
            self.window.retain(|p| p.frame.generation == current);
            let mut stale = before - self.window.len() as u64;
            if self
                .held_missing
                .as_ref()
                .is_some_and(|f| f.generation != current)
            {
                self.held_missing = None;
                self.stats.missing_pts_dropped += 1;
                stale += 1;
            }
            // A repeat is a present: the old generation's last frame
            // must never be re-emitted.
            if self.last.as_ref().is_some_and(|r| r.generation != current) {
                self.last = None;
                self.repeats = 0;
            }
            self.stats.dropped_stale += stale;
            drops.stale = stale;
            self.generation = current;
        }

        // Step 2: admit a held missing-stamp frame if inference became
        // possible (it is then judged by the same rules as any frame).
        self.admit_held();

        // 3.B1 seam 2 — the window rotation for the ladder's tumbling
        // budget/rate windows (per tick; O(1) compares).
        self.sync.tick_index = self.sync.tick_index.wrapping_add(1);
        self.sync.rotate_windows(&self.cfg.sync);

        // Step 3: late purge — a prefix of the sorted window, against
        // the *effective* threshold (the configured one, widened while
        // the ladder's WidenLate band is in force). Skipped for the
        // one tick after a snap trigger (the exemption's purpose).
        let effective_threshold = self.effective_late_threshold();
        if !self.sync.purge_skip {
            while let Some(front) = self.window.front() {
                if is_late(front.pts, now, effective_threshold) {
                    if let Some(dropped) = self.window.pop_front() {
                        drops.late.push(dropped.pts);
                        self.stats.dropped_late += 1;
                        self.sync.note_late_drop(&self.cfg.sync);
                    }
                } else {
                    break;
                }
            }
        }
        self.sync.purge_skip = false;

        // Step 4: selection — the largest pts ≤ now + target_lead.
        let due_bound = i128::from(now.ticks) + i128::from(self.cfg.target_lead.0);
        let due_count = self
            .window
            .iter()
            .take_while(|p| i128::from(p.pts.ticks) <= due_bound)
            .count();
        let mut selected = None;
        if due_count > 0 {
            let superseded: Vec<Timestamp> =
                self.window.drain(..due_count - 1).map(|p| p.pts).collect();
            self.stats.dropped_superseded += superseded.len() as u64;
            drops.superseded = superseded;
            selected = self.window.pop_front();
        }
        let action = if let Some(pending) = selected {
            let pts = pending.pts;
            let request = PresentRequest {
                generation: pending.frame.generation,
                frame: pending.frame,
                when: pts,
            };
            self.last = Some(request.clone());
            self.repeats = 0;
            self.stats.presented += 1;
            // 3.B1 seam 3 — the A/V presentation-error sample
            // (audio-master): e_v = pts − now, computed in i128 and
            // saturated to i64 (the `decide()` discipline). Paused
            // clocks discard samples, never smear them (W1-18 §2.1);
            // a snap present feeds the fresh regime like any first
            // sample.
            if snapshot.state == ClockState::Running {
                let e_v = (i128::from(pts.ticks) - i128::from(now.ticks))
                    .clamp(i128::from(i64::MIN), i128::from(i64::MAX))
                    as i64;
                self.sync.estimator.observe_sample(e_v);
                self.stats.sync_samples += 1;
                self.stats.sync_error_sum =
                    self.stats.sync_error_sum.saturating_add(i128::from(e_v));
                self.stats.sync_error_abs_max_ns =
                    self.stats.sync_error_abs_max_ns.max(e_v.unsigned_abs());
                sync_sample.error_ns = Some(e_v);
            }
            if self.sync.snap_pending {
                self.stats.snap_presents += 1;
                self.sync.snap_pending = false;
                sync_sample.snapped = true;
            }
            PresentAction::Present(request)
        } else if self.repeats < self.effective_max_repeat() {
            // Step 5: bounded repeat of the last frame (doc 03 §4.10 e),
            // with the ladder's ExtendEarly extension in force.
            if let Some(last) = self.last.clone() {
                self.repeats += 1;
                self.stats.repeats += 1;
                PresentAction::RepeatLast(last)
            } else {
                self.stats.background += 1;
                PresentAction::Background
            }
        } else {
            self.stats.background += 1;
            PresentAction::Background
        };

        // 3.B1 seam 4 — the band decision runs every tick (the dwell
        // is tick-scaled); the actuators apply at the decide_every
        // cadence (bounded churn, deterministic without a time source).
        let band = self
            .sync
            .bands
            .classify(if snapshot.state == ClockState::Running {
                self.sync.estimator.error_ema()
            } else {
                None
            });
        if self
            .sync
            .tick_index
            .is_multiple_of(u64::from(self.cfg.sync.decide_every))
        {
            self.apply_sync_ladder(band);
        }

        sync_sample.error_ema_ns = self.sync.estimator.error_ema();
        sync_sample.band = band;
        sync_sample.late_threshold_ns = effective_threshold.map(|t| t.0);

        let next_due = self.window.front().map(|p| self.cfg.due_at(p.pts));
        TickOutcome {
            action,
            next_due,
            drops,
            sync: sync_sample,
        }
    }

    /// The effective late threshold: the configured one, widened (never
    /// narrowed) while the ladder's WidenLate band is in force.
    fn effective_late_threshold(&self) -> Option<Nanoseconds> {
        match (self.cfg.late_threshold(), self.sync.widened) {
            (None, _) => None,
            (Some(configured), None) => Some(configured),
            (Some(configured), Some(widened)) => Some(widened.max(configured)),
        }
    }

    /// The effective repeat budget: the configured one plus the
    /// ladder's ExtendEarly extension.
    fn effective_max_repeat(&self) -> usize {
        self.cfg
            .max_repeat
            .saturating_add(self.sync.repeat_extension as usize)
    }

    /// Apply the ladder's actuators for the band in force (the
    /// decide_every-cadence half of seam 4). Bounds: the widened
    /// threshold never exceeds the cap, never applies under
    /// [`DropPolicy::Never`], changes at most
    /// [`SyncPolicy::threshold_change_limit`] per rate window, and
    /// stops widening once the drop budget for the window is spent
    /// (dropping is bounded, never a firehose).
    fn apply_sync_ladder(&mut self, band: SyncBand) {
        match band {
            SyncBand::Hold => {
                if self.sync.widened.take().is_some() {
                    self.sync.changes_in_window = self.sync.changes_in_window.saturating_add(1);
                }
                self.sync.repeat_extension = 0;
            }
            SyncBand::WidenLate => {
                self.sync.repeat_extension = 0;
                // Never under a Never policy; never without a frame
                // duration to scale by.
                if self.cfg.late_threshold().is_none() || self.cfg.frame_duration.is_none() {
                    return;
                }
                let Some(ema) = self.sync.estimator.error_ema() else {
                    return;
                };
                // Beyond the drop budget: hold the current widening,
                // widen no further.
                if self.sync.drops_in_window >= self.cfg.sync.drop_budget {
                    return;
                }
                let Some(want) = self
                    .cfg
                    .sync
                    .widened_threshold(ema, self.cfg.frame_duration)
                else {
                    return;
                };
                if Some(want) == self.sync.widened {
                    return; // no change to make
                }
                // The anti-flapping rate limit: at most the configured
                // changes per rate window.
                if self.sync.changes_in_window >= self.cfg.sync.threshold_change_limit {
                    return;
                }
                self.sync.widened = Some(want);
                self.sync.changes_in_window = self.sync.changes_in_window.saturating_add(1);
            }
            SyncBand::ExtendEarly => {
                if self.sync.widened.take().is_some() {
                    self.sync.changes_in_window = self.sync.changes_in_window.saturating_add(1);
                }
                self.sync.repeat_extension = self.cfg.sync.repeat_extension;
            }
        }
    }

    /// The configuration in force.
    #[must_use]
    pub fn cfg(&self) -> &SchedulerCfg {
        &self.cfg
    }

    /// Lifetime counters (see [`SchedulerStats`]).
    #[must_use]
    pub fn stats(&self) -> SchedulerStats {
        self.stats
    }

    /// Frames currently awaiting their slot.
    #[must_use]
    pub fn pending_len(&self) -> usize {
        self.window.len()
    }

    /// Whether a missing-stamp frame is currently held.
    #[must_use]
    pub fn holds_missing(&self) -> bool {
        self.held_missing.is_some()
    }

    /// The last presented request, if any (the repeat source).
    #[must_use]
    pub fn last_presented(&self) -> Option<&PresentRequest> {
        self.last.as_ref()
    }

    /// The live generation as of the last tick (push-boundary check).
    #[must_use]
    pub fn current_generation(&self) -> GenerationId {
        self.generation
    }

    /// The latest known sequence position for missing-stamp inference:
    /// the pending tail (the newest frame still awaiting its slot),
    /// else the last presented slot.
    fn sequence_base(&self) -> Option<i64> {
        self.window
            .back()
            .map(|p| p.pts.ticks)
            .or_else(|| self.last.as_ref().map(|r| r.when.ticks))
    }

    /// Retry admission of the held missing-stamp frame (tick step 2).
    /// Dropped admission paths count in [`SchedulerStats`]; the frame
    /// goes back to the hold slot when inference is still impossible.
    fn admit_held(&mut self) {
        let Some(frame) = self.held_missing.take() else {
            return;
        };
        let base = self.sequence_base();
        match (base, self.cfg.frame_duration) {
            (Some(base), Some(Nanoseconds(dur))) => {
                if self.window.len() >= self.cfg.max_pending.get() {
                    self.stats.missing_pts_dropped += 1;
                } else {
                    let pts = Timestamp::new(saturating_add(base, dur), NANO_BASE);
                    insert_sorted(&mut self.window, Pending { pts, frame });
                    self.stats.pushed += 1;
                    self.stats.inferred_pts += 1;
                }
            }
            _ => self.held_missing = Some(frame),
        }
    }
}

/// Insert keeping the window sorted by PTS ascending, stable among
/// equals (arrival order — doc 03 §4.10 c "display order only").
fn insert_sorted(window: &mut VecDeque<Pending>, item: Pending) {
    let mut index = window.len();
    for (i, pending) in window.iter().enumerate() {
        if pending.pts.ticks > item.pts.ticks {
            index = i;
            break;
        }
    }
    window.insert(index, item);
}

/// Saturating i64 + u64 → i64 (the media-clock position discipline:
/// saturate, never wrap — a slot beyond ±292 years is equivalent to its
/// clamp for any real schedule).
fn saturating_add(base: i64, dur: u64) -> i64 {
    (i128::from(base) + i128::from(dur)).clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

/// The late-purge predicate against an **effective** threshold (the
/// 3.B1 ladder can widen it): `pts < now − threshold` is late, with
/// the strict comparison and i128 widening of `SchedulerCfg::decide`
/// (`pts == now − threshold` is *on* the bound and presents). Kept as
/// a free function so the purge loop needs no `&self` borrow while
/// mutating the window.
fn is_late(pts: Timestamp, now: Timestamp, threshold: Option<Nanoseconds>) -> bool {
    let Some(Nanoseconds(threshold)) = threshold else {
        return false;
    };
    i128::from(pts.ticks) < i128::from(now.ticks) - i128::from(threshold)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::DropPolicy;
    use crate::test_support::{VirtualVideoClock, frame, frame_missing, frame_stream_base};
    use testkit::Must;

    /// 10 ns nominal frames, late threshold 10, lead 5, max_repeat 2,
    /// window 8: the arithmetic stays hand-checkable.
    fn cfg() -> SchedulerCfg {
        SchedulerCfg::new(
            DropPolicy::Late {
                threshold: Nanoseconds(10),
            },
            2,
            Nanoseconds(5),
            Some(Nanoseconds(10)),
            std::num::NonZeroUsize::new(8).must(),
        )
        .must()
    }

    #[test]
    fn presents_in_order_under_a_normally_advancing_clock() {
        let clock = VirtualVideoClock::new();
        let mut sched = PresentationScheduler::new(cfg(), clock.clock());
        // Produce-then-present, one frame per slot: every frame is due
        // exactly at its own PTS (nothing else resident).
        for pts in 0i64..50 {
            let _ = sched.push(frame(pts, GenerationId::FIRST));
            clock.set_ns(pts.unsigned_abs());
            let outcome = sched.tick();
            let request = match outcome.action {
                PresentAction::Present(r) => r,
                other => unreachable!("frame {pts} due, got {other:?}"),
            };
            assert_eq!(request.when.ticks, pts, "frame {pts} on its slot");
            assert!(outcome.drops.late.is_empty());
            assert!(outcome.drops.superseded.is_empty());
            assert_eq!(outcome.drops.stale, 0);
        }
        let stats = sched.stats();
        assert_eq!(stats.presented, 50);
        assert_eq!(stats.dropped_late, 0);
        assert_eq!(stats.dropped_superseded, 0);
    }

    #[test]
    fn early_frames_are_held_until_their_slot() {
        let clock = VirtualVideoClock::new();
        let mut sched = PresentationScheduler::new(cfg(), clock.clock());
        let _ = sched.push(frame(100, GenerationId::FIRST));
        // Ticks before the slot: background (nothing ever shown), with
        // the wait hint pointing at the due position.
        for now in [0u64, 20, 60] {
            clock.set_ns(now);
            let outcome = sched.tick();
            assert!(
                matches!(outcome.action, PresentAction::Background),
                "nothing to show at {now}"
            );
            assert_eq!(outcome.next_due.map(|t| t.ticks), Some(95));
            assert_eq!(sched.pending_len(), 1, "held, not dropped");
        }
        clock.set_ns(95); // due exactly at 100 − lead 5
        let outcome = sched.tick();
        match outcome.action {
            PresentAction::Present(r) => assert_eq!(r.when.ticks, 100),
            other => unreachable!("due at the boundary, got {other:?}"),
        }
        assert_eq!(sched.stats().presented, 1);
        assert_eq!(sched.stats().dropped_late, 0);
    }

    #[test]
    fn late_frames_are_dropped_counted_never_presented() {
        let clock = VirtualVideoClock::new();
        // A wider window than `cfg()` so all ten frames fit.
        let wide = SchedulerCfg::new(
            DropPolicy::Late {
                threshold: Nanoseconds(10),
            },
            2,
            Nanoseconds(5),
            Some(Nanoseconds(10)),
            std::num::NonZeroUsize::new(16).must(),
        )
        .must();
        let mut sched = PresentationScheduler::new(wide, clock.clock());
        // A stall: the clock jumps 100 ns while frames 0..9 wait.
        for pts in 0i64..10 {
            let _ = sched.push(frame(pts, GenerationId::FIRST));
        }
        clock.set_ns(100);
        let outcome = sched.tick();
        // Late bound: now 100 − threshold 10 = 90 → every frame < 90
        // drops; all of 0..9 are late, none is presented late.
        assert_eq!(
            outcome.drops.late.len(),
            10,
            "all frames late: {:?}",
            outcome.drops.late
        );
        assert!(matches!(outcome.action, PresentAction::Background));
        assert_eq!(sched.stats().dropped_late, 10);
        // The dropped frames are gone for good: a later tick with a
        // fresh on-time frame never resurrects them.
        let _ = sched.push(frame(100, GenerationId::FIRST));
        let outcome = sched.tick();
        match outcome.action {
            PresentAction::Present(r) => assert_eq!(r.when.ticks, 100),
            other => unreachable!("{other:?}"),
        }
        assert_eq!(sched.stats().presented, 1);
        assert_eq!(sched.stats().dropped_late, 10);
    }

    #[test]
    fn the_largest_due_frame_wins_and_supersedes() {
        let clock = VirtualVideoClock::new();
        let mut sched = PresentationScheduler::new(cfg(), clock.clock());
        // Two frames already due at now=0 (lead 5): 0 and 3.
        let _ = sched.push(frame(0, GenerationId::FIRST));
        let _ = sched.push(frame(3, GenerationId::FIRST));
        let outcome = sched.tick();
        match &outcome.action {
            PresentAction::Present(r) => assert_eq!(r.when.ticks, 3, "largest due wins"),
            other => unreachable!("{other:?}"),
        }
        assert_eq!(outcome.drops.superseded, vec![Timestamp::new(0, NANO_BASE)]);
        assert_eq!(sched.stats().dropped_superseded, 1);
        assert_eq!(sched.pending_len(), 0);
    }

    #[test]
    fn out_of_order_arrivals_present_in_pts_order() {
        let clock = VirtualVideoClock::new();
        let mut sched = PresentationScheduler::new(cfg(), clock.clock());
        // Display-order arrival with a local shuffle inside the window.
        for pts in [12i64, 10, 11, 15, 13] {
            let _ = sched.push(frame(pts, GenerationId::FIRST));
        }
        let mut presented = Vec::new();
        for now in [5u64, 7, 9, 11] {
            clock.set_ns(now);
            if let PresentAction::Present(r) = sched.tick().action {
                presented.push(r.when.ticks);
            }
        }
        // The largest-due rule selects 12 over 11 at now=7 (bound 12):
        // 11 is superseded — dropped, never presented out of order.
        assert_eq!(presented, vec![10, 12, 13, 15], "non-decreasing");
        assert_eq!(sched.stats().dropped_superseded, 1);
        assert_eq!(sched.last_presented().map(|r| r.when.ticks), Some(15));
    }

    #[test]
    fn repeat_budget_then_background() {
        let clock = VirtualVideoClock::new();
        let mut sched = PresentationScheduler::new(cfg(), clock.clock());
        let _ = sched.push(frame(0, GenerationId::FIRST));
        clock.set_ns(0);
        assert!(matches!(sched.tick().action, PresentAction::Present(_)));
        // Nothing new arrives: repeat exactly max_repeat (2) times...
        for _ in 0..2 {
            match sched.tick().action {
                PresentAction::RepeatLast(r) => assert_eq!(r.when.ticks, 0),
                other => unreachable!("{other:?}"),
            }
        }
        // ...then background, and stay there.
        for _ in 0..3 {
            assert!(matches!(sched.tick().action, PresentAction::Background));
        }
        let stats = sched.stats();
        assert_eq!(stats.repeats, 2);
        assert_eq!(stats.background, 3);
        // A fresh present resets the budget.
        let _ = sched.push(frame(10, GenerationId::FIRST));
        clock.set_ns(10);
        assert!(matches!(sched.tick().action, PresentAction::Present(_)));
        assert!(matches!(sched.tick().action, PresentAction::RepeatLast(_)));
        assert!(matches!(sched.tick().action, PresentAction::RepeatLast(_)));
        assert!(matches!(sched.tick().action, PresentAction::Background));
    }

    #[test]
    fn generation_change_purges_the_window_and_kills_repeats() {
        let clock = VirtualVideoClock::new();
        let mut sched = PresentationScheduler::new(cfg(), clock.clock());
        let next = GenerationId::FIRST.next().must();
        let _ = sched.push(frame(100, GenerationId::FIRST));
        clock.set_ns(100);
        match sched.tick().action {
            PresentAction::Present(r) => assert_eq!(r.when.ticks, 100),
            other => unreachable!("{other:?}"),
        }
        // Three not-yet-due frames are pending when the session mints
        // the new generation on the clock (seek): they are dropped
        // wholesale, counted, never presented.
        for pts in 200i64..203 {
            let _ = sched.push(frame(pts, GenerationId::FIRST));
        }
        clock.set_generation(next);
        let outcome = sched.tick();
        assert_eq!(outcome.drops.stale, 3, "the three still-pending frames");
        assert_eq!(sched.stats().dropped_stale, 3);
        assert_eq!(sched.pending_len(), 0);
        // The old last frame is not repeatable (a repeat is a present).
        assert!(matches!(outcome.action, PresentAction::Background));
        // Old-generation pushes are refused at the boundary.
        match sched.push(frame(500, GenerationId::FIRST)) {
            PushOutcome::Stale { current, .. } => assert_eq!(current, next),
            other => unreachable!("{other:?}"),
        }
        assert_eq!(sched.stats().stale_rejected_push, 1);
        // New-generation frames flow again.
        assert!(matches!(
            sched.push(frame(600, next)),
            PushOutcome::Scheduled
        ));
    }

    #[test]
    fn missing_pts_is_inferred_from_the_previous_sequence_position() {
        let clock = VirtualVideoClock::new();
        let mut sched = PresentationScheduler::new(cfg(), clock.clock());
        let _ = sched.push(frame(10, GenerationId::FIRST));
        clock.set_ns(10);
        assert!(matches!(sched.tick().action, PresentAction::Present(_)));
        // A missing-stamp frame arrives: inferred at last + duration 10.
        match sched.push(frame_missing(GenerationId::FIRST)) {
            PushOutcome::Inferred { pts } => assert_eq!(pts.ticks, 20),
            other => unreachable!("{other:?}"),
        }
        assert_eq!(sched.stats().inferred_pts, 1);
        clock.set_ns(20);
        match sched.tick().action {
            PresentAction::Present(r) => {
                assert_eq!(r.when.ticks, 20);
                assert!(
                    matches!(r.frame.pts, Stamp::Missing),
                    "the frame itself is untouched"
                );
            }
            other => unreachable!("{other:?}"),
        }
    }

    #[test]
    fn uninferable_missing_pts_is_held_never_presented_never_lost_silently() {
        // No frame_duration configured → inference impossible → hold.
        let clock = VirtualVideoClock::new();
        let hold_cfg = SchedulerCfg::new(
            DropPolicy::Never,
            0,
            Nanoseconds(0),
            None,
            std::num::NonZeroUsize::new(8).must(),
        )
        .must();
        let mut sched = PresentationScheduler::new(hold_cfg, clock.clock());
        assert!(matches!(
            sched.push(frame_missing(GenerationId::FIRST)),
            PushOutcome::Held
        ));
        assert!(sched.holds_missing());
        // Ticks keep holding (never presented, never silently lost).
        assert!(matches!(sched.tick().action, PresentAction::Background));
        assert!(sched.holds_missing());
        assert_eq!(sched.stats().missing_pts_dropped, 0);
        // A second missing frame replaces the first (bounded hold).
        assert!(matches!(
            sched.push(frame_missing(GenerationId::FIRST)),
            PushOutcome::Held
        ));
        assert_eq!(sched.stats().missing_pts_dropped, 1);
    }

    #[test]
    fn window_bound_is_enforced_with_the_frame_handed_back() {
        let clock = VirtualVideoClock::new();
        let mut sched = PresentationScheduler::new(cfg(), clock.clock());
        // Window 8: fill it with far-future frames (none due).
        for pts in 1000i64..1008 {
            assert!(matches!(
                sched.push(frame(pts, GenerationId::FIRST)),
                PushOutcome::Scheduled
            ));
        }
        match sched.push(frame(2000, GenerationId::FIRST)) {
            PushOutcome::WindowFull { frame, capacity } => {
                assert_eq!(capacity, 8);
                assert_eq!(frame.pts, Stamp::Present(Timestamp::new(2000, NANO_BASE)));
            }
            other => unreachable!("{other:?}"),
        }
        assert_eq!(sched.stats().full_rejections, 1);
        assert_eq!(sched.pending_len(), 8);
        // Advancing the clock past the window drains it (all late →
        // dropped), after which the push succeeds.
        clock.set_ns(5000);
        let _ = sched.tick();
        assert_eq!(sched.pending_len(), 0);
        assert!(matches!(
            sched.push(frame(6000, GenerationId::FIRST)),
            PushOutcome::Scheduled
        ));
    }

    #[test]
    fn unrepresentable_pts_is_unschedulable_with_the_error_verbatim() {
        let clock = VirtualVideoClock::new();
        let mut sched = PresentationScheduler::new(cfg(), clock.clock());
        // A 1/1 base with i64::MAX ticks: × 1e9 overflows the i64
        // nanosecond range — the checked conversion refuses.
        match sched.push(frame_stream_base(i64::MAX, 1, 1, GenerationId::FIRST)) {
            PushOutcome::UnschedulablePts { error, .. } => {
                assert!(matches!(error, TimeError::Overflow { .. }), "{error:?}");
            }
            other => unreachable!("{other:?}"),
        }
        assert_eq!(sched.stats().unschedulable_pts, 1);
        assert_eq!(sched.pending_len(), 0);
    }

    #[test]
    fn stream_base_pts_are_normalized_before_scheduling() {
        let clock = VirtualVideoClock::new();
        let mut sched = PresentationScheduler::new(cfg(), clock.clock());
        // A 1/90_000 base (the demux-standard video tick): 9000 ticks
        // = 100 ms = 1e8 ns on the clock's base.
        let _ = sched.push(frame_stream_base(9000, 1, 90_000, GenerationId::FIRST));
        clock.set_ns(100_000_000);
        match sched.tick().action {
            PresentAction::Present(r) => assert_eq!(r.when.ticks, 100_000_000),
            other => unreachable!("{other:?}"),
        }
    }
}
