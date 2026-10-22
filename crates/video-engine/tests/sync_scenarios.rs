//! Card 3.B1 convergence scenarios, video half (W1-18 §6 S4/S5/S10):
//! the PTS-gap snap, the jitter no-flap property, and the
//! hysteresis-ladder grids — all on injected virtual clocks, zero
//! sleeps (doc 03 §8). The audio half (S1–S3/S5-fill/S6) lives in
//! audio-engine's governor/resample suites; the composition root
//! carries S7–S9 (apps/cli).
//!
//! The scheduler's own sync observability is counters and per-tick
//! `SyncSample`s only — never events (the no-event-storm policy,
//! asserted here by shape: nothing in this suite touches an event
//! surface because none exists on the scheduler).

mod common;

use common::{RecordingPresenter, VirtualClock, frame};
use media_clock::SyncBand;
use media_types::GenerationId;
use media_types::time::Nanoseconds;
use testkit::Must;
use video_engine::{
    DropPolicy, PresentAction, PresentationQueue, PresentationScheduler, PushOutcome,
    SchedulerCfg, SyncPolicy, SyncSample, TickOutcome,
};

/// 30 fps nominal frame duration (ns).
const FRAME: u64 = 33_333_334;
const GEN: GenerationId = GenerationId::FIRST;

fn default_cfg() -> SchedulerCfg {
    SchedulerCfg::for_frame_duration(Nanoseconds(FRAME)).must()
}

/// One production-shaped wiring step (the scheduling.rs harness's
/// shape): tick, enqueue the action's request, drain to the presenter.
fn step(
    scheduler: &mut PresentationScheduler,
    queue: &mut PresentationQueue,
    sink: &mut RecordingPresenter,
) -> TickOutcome {
    let outcome = scheduler.tick();
    let request = match &outcome.action {
        PresentAction::Present(request) | PresentAction::RepeatLast(request) => {
            Some(request.clone())
        }
        PresentAction::Background => None,
    };
    if let Some(request) = request {
        let _ = queue.try_push(request);
    }
    let _ = queue.drain_to(sink);
    outcome
}

fn wide_queue() -> PresentationQueue {
    PresentationQueue::new(std::num::NonZeroUsize::new(64).must())
}

// ---------------------------------------------------------------------------
// S4 — PTS step discontinuity: exactly one snap, estimator reset,
//      bounded drops, error back inside one frame within 2 s
// ---------------------------------------------------------------------------

#[test]
fn s4_pts_step_discontinuity_snaps_exactly_once() {
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = wide_queue();
    let mut sink = RecordingPresenter::new();

    // Phase 1 — steady 30 fps: 60 frames, one per tick, in lockstep.
    let mut pts = 0_i64;
    for _ in 0..60 {
        match scheduler.push(frame(pts, GEN)) {
            PushOutcome::Scheduled => {}
            other => unreachable!("{other:?}"),
        }
        let _ = step(&mut scheduler, &mut queue, &mut sink);
        clock.advance_ns(FRAME);
        pts += FRAME as i64;
    }
    let before = scheduler.stats();
    assert_eq!(before.snap_presents, 0);
    assert_eq!(before.sync_pts_gaps, 0);
    assert!(before.sync_samples >= 55);

    // Phase 2 — the discontinuity: the next frame's PTS jumps +500 ms
    // (the default snap threshold: ½ the doc 03 §4.4 audio queue
    // duration cap). The push-site seam arms snap mode and resets the
    // error estimator.
    pts += 500_000_000;
    match scheduler.push(frame(pts, GEN)) {
        PushOutcome::Scheduled => {}
        other => unreachable!("{other:?}"),
    }
    // The estimator reset is observable on the very next tick: no
    // present happens (the post-gap frame is not yet due), and the
    // EMA is gone — the pre-gap samples were a different regime.
    let outcome = step(&mut scheduler, &mut queue, &mut sink);
    assert_eq!(outcome.sync.error_ema_ns, None, "estimator must reset");
    clock.advance_ns(FRAME);

    // Phase 3 — the clock walks to the post-gap frame (500 ms ≈ 15
    // ticks of bounded repeats then background), the snap present
    // lands, and the stream continues at 30 fps from the new base.
    // Two seconds of post-snap stream follow (60 ticks).
    let mut snapped_at: Option<usize> = None;
    let mut post_snap_errors: Vec<i64> = Vec::new();
    let mut bands: Vec<SyncBand> = Vec::new();
    for tick in 0..(15 + 61) {
        let outcome = step(&mut scheduler, &mut queue, &mut sink);
        if outcome.sync.snapped {
            snapped_at.get_or_insert(tick);
        } else if snapped_at.is_some() {
            if let Some(error) = outcome.sync.error_ns {
                post_snap_errors.push(error);
            }
        }
        bands.push(outcome.sync.band);
        if tick >= 15 {
            pts += FRAME as i64;
            match scheduler.push(frame(pts, GEN)) {
                PushOutcome::Scheduled => {}
                other => unreachable!("{other:?}"),
            }
        }
        clock.advance_ns(FRAME);
    }

    // Exactly one snap, exactly one gap, no drop-storm: the post-gap
    // frames are EARLY relative to the clock's walk, so the late path
    // never fires (the exemption's other half — the backward
    // re-anchor — is S7's, in the composition root).
    let stats = scheduler.stats();
    assert_eq!(stats.sync_pts_gaps, 1);
    assert_eq!(stats.snap_presents, 1, "snapped at {snapped_at:?}");
    assert_eq!(
        stats.dropped_late - before.dropped_late,
        0,
        "a forward gap drops nothing"
    );
    sink.assert_in_order("S4");

    // Within 2 s (60 ticks) after the snap, every present's error is
    // inside one frame duration, and the band machinery never left
    // Hold (a one-off gap passes through the EMA without tripping the
    // dwell).
    assert!(
        !post_snap_errors.is_empty(),
        "the post-gap stream must present"
    );
    assert!(
        post_snap_errors.iter().all(|e| e.unsigned_abs() <= FRAME),
        "post-snap errors left the one-frame band: {post_snap_errors:?}"
    );
    assert!(
        bands.iter().all(|band| *band == SyncBand::Hold),
        "a one-off gap must not trip the ladder"
    );
}

// ---------------------------------------------------------------------------
// S5 — ±½ frame PTS jitter: no amplification, no threshold flapping
// ---------------------------------------------------------------------------

#[test]
fn s5_pts_jitter_never_flaps_the_threshold() {
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = wide_queue();
    let mut sink = RecordingPresenter::new();

    // A seeded xorshift (the repo's property convention — no external
    // dep): PTS jitter of ±½ frame around the 30 fps grid, 5 s worth.
    let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut threshold_changes = 0_u32;
    let mut last_threshold: Option<u64> = None;
    let mut correction_ticks = 0_u32;
    let mut presents = 0_u64;

    let mut pts = 0_i64;
    for _ in 0..150 {
        // One frame per tick, jittered ±½ frame around a strictly
        // increasing grid (the jitter can reorder arrivals; the
        // window sorts, and presentation order stays PTS order).
        let jitter = ((next() % 1001) as i64) - 500; // ±500 ‰ of a frame
        let jittered = pts + jitter * (FRAME as i64 / 1000);
        match scheduler.push(frame(jittered, GEN)) {
            PushOutcome::Scheduled => {}
            other => unreachable!("{other:?}"),
        }
        pts += FRAME as i64;
        let outcome = step(&mut scheduler, &mut queue, &mut sink);
        if matches!(outcome.action, PresentAction::Present(_)) {
            presents += 1;
        }
        if outcome.sync.band != SyncBand::Hold {
            correction_ticks += 1;
        }
        if let (Some(previous), Some(current)) = (last_threshold, outcome.sync.late_threshold_ns) {
            if previous != current {
                threshold_changes += 1;
            }
        }
        last_threshold = outcome.sync.late_threshold_ns;
        clock.advance_ns(FRAME);
    }

    // No flapping: the rate limit allows 2 threshold changes per 5 s
    // window; with pure ±½ frame jitter the ladder never engages at
    // all (0 is the measured value — the hold band absorbs what the
    // lead already absorbs, the EMA smooths the rest).
    assert_eq!(
        threshold_changes, 0,
        "jitter must not move the threshold"
    );
    assert_eq!(
        correction_ticks, 0,
        "jitter must not trip a correction band"
    );
    // The stream still presented (jitter is not loss) and in order.
    assert!(presents >= 140, "only {presents} presents in 150 ticks");
    sink.assert_in_order("S5");
    let stats = scheduler.stats();
    assert_eq!(stats.dropped_late, 0, "jitter is not lateness");
    assert_eq!(stats.snap_presents, 0);
    assert_eq!(stats.sync_pts_gaps, 0);
}

// ---------------------------------------------------------------------------
// S10 — the ladder grids (end-to-end through the tick + policy grids)
// ---------------------------------------------------------------------------

/// One more wiring step, answering the tick's `SyncSample` (the
/// ladder tests' observation point).
fn sync_step(
    scheduler: &mut PresentationScheduler,
    queue: &mut PresentationQueue,
    sink: &mut RecordingPresenter,
) -> SyncSample {
    step(scheduler, queue, sink).sync
}

/// WidenLate engaged: a sustained one-frame-late stream (every frame
/// presents exactly on the late boundary — zero drops) widens the
/// effective threshold to `clamp(2·|ê|, 1, 3)` frames at the
/// decide_every cadence.
#[test]
fn s10_sustained_latness_widens_the_threshold_in_band() {
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = wide_queue();
    let mut sink = RecordingPresenter::new();

    // Frame k (pts = k·F) arrives one tick late (presented at tick
    // k+1): the late bound is exactly its pts — boundary presents,
    // e_v = −F sustained, zero drops, the window never holds more
    // than the one arriving frame.
    for k in 0..=40_i64 {
        // Walk the clock to tick k+1 (stepping as we go — the steps
        // before the first arrival see an empty window: background).
        while clock.now_ns() < ((k + 1) as u64) * FRAME {
            let _ = sync_step(&mut scheduler, &mut queue, &mut sink);
            clock.advance_ns(FRAME);
        }
        match scheduler.push(frame(k * FRAME as i64, GEN)) {
            PushOutcome::Scheduled => {}
            other => unreachable!("{other:?}"),
        }
        let _ = sync_step(&mut scheduler, &mut queue, &mut sink);
        clock.advance_ns(FRAME);
    }
    let stats = scheduler.stats();
    assert_eq!(stats.dropped_late, 0, "boundary frames are not late");
    assert!(stats.presented >= 38, "{}", stats.presented);
    // The EMA settled at −F: WidenLate engaged and the threshold
    // widened to 2 frames (clamp(2·1, 1, 3) = 2) at the tick-16
    // ladder application.
    let last = sync_step(&mut scheduler, &mut queue, &mut sink);
    assert_eq!(last.band, SyncBand::WidenLate);
    assert_eq!(
        last.late_threshold_ns,
        Some(2 * FRAME),
        "widened to {:?}",
        last.late_threshold_ns
    );
    // Policy grid (S10's pure half): the widening is monotone in
    // |ê| and never passes the 3-frame cap, for any sustained error.
    let policy = SyncPolicy::for_frame_duration(Nanoseconds(FRAME));
    let mut previous = 0_u64;
    for eighth in 1..=48_i64 {
        let ema = -eighth * FRAME as i64 / 8; // −⅛F .. −6F
        let widened = policy
            .widened_threshold(ema, Some(Nanoseconds(FRAME)))
            .must()
            .0;
        assert!(widened >= previous, "not monotone at {ema}");
        assert!(widened <= 3 * FRAME, "{ema} → {widened}");
        previous = widened;
    }
}

/// The drop budget: once a window's late drops spend the budget, the
/// threshold stops widening — dropping is bounded, never a firehose
/// (W1-18 §3.1's bound; S10's "budget never exceeded for any input
/// sequence").
#[test]
fn s10_spent_drop_budget_freezes_the_widening() {
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = wide_queue();
    let mut sink = RecordingPresenter::new();

    // Every tick: one boundary frame (presents, e_v = −F — the EMA
    // asks for widening) PLUS three strictly-late stragglers (each
    // dropped, spending the 3-per-window budget before every ladder
    // application can widen).
    for t in 1..=48_i64 {
        while clock.now_ns() < (t as u64) * FRAME {
            let _ = sync_step(&mut scheduler, &mut queue, &mut sink);
            clock.advance_ns(FRAME);
        }
        match scheduler.push(frame((t - 1) * FRAME as i64, GEN)) {
            PushOutcome::Scheduled => {}
            other => unreachable!("{other:?}"),
        }
        for j in 1..=3_i64 {
            match scheduler.push(frame((t - 1 - j) * FRAME as i64, GEN)) {
                PushOutcome::Scheduled => {}
                other => unreachable!("{other:?}"),
            }
        }
        let _ = sync_step(&mut scheduler, &mut queue, &mut sink);
        clock.advance_ns(FRAME);
    }
    let stats = scheduler.stats();
    // The stragglers dropped (3 per tick), the boundary frames
    // presented — the stream is lossy but bounded, exactly as the
    // budget intends.
    assert!(
        stats.dropped_late >= 3 * 40,
        "the stragglers must drop: {}",
        stats.dropped_late
    );
    assert!(stats.presented >= 40, "{}", stats.presented);
    // The band engaged (sustained −F EMA) but the budget was spent at
    // every ladder application: the effective threshold NEVER left
    // the configured one frame — the firehose stays a firehose
    // instead of widening the definition of late.
    let last = sync_step(&mut scheduler, &mut queue, &mut sink);
    assert_eq!(last.band, SyncBand::WidenLate);
    assert_eq!(
        last.late_threshold_ns,
        Some(FRAME),
        "the threshold must freeze at the configured value while the budget is spent"
    );
}

/// ExtendEarly engaged (a configured lead wider than the hold band):
/// the repeat budget extends by +2 — starvation gaps are bridged
/// longer before backgrounding. Audio is never touched (there is no
/// audio actuator on the scheduler — structural).
#[test]
fn s10_sustained_earliness_extends_the_repeat_budget() {
    // Lead = one full frame (the hold band stays ½ frame): presents
    // land at +1 frame of error — beyond the band, on the early side.
    let cfg = SchedulerCfg::new(
        DropPolicy::Late {
            threshold: Nanoseconds(FRAME),
        },
        3,
        Nanoseconds(FRAME),
        Some(Nanoseconds(FRAME)),
        std::num::NonZeroUsize::new(16).must(),
    )
    .must();
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(cfg, clock.clock());
    let mut queue = wide_queue();
    let mut sink = RecordingPresenter::new();

    // Frame pushed at tick t with pts (t+2)·F: not due at t (bound
    // (t+1)F), due exactly at t+1 (bound (t+2)F) — every present
    // carries e_v = +F, sustaining ExtendEarly after the dwell.
    for t in 0..40_i64 {
        match scheduler.push(frame((t + 2) * FRAME as i64, GEN)) {
            PushOutcome::Scheduled => {}
            other => unreachable!("{other:?}"),
        }
        let _ = sync_step(&mut scheduler, &mut queue, &mut sink);
        clock.advance_ns(FRAME);
    }
    // Starve the scheduler: the EMA holds +F (no present, no decay),
    // so ExtendEarly stays in force and the repeat budget is 3 + 2 = 5
    // before the first background. (The final fed frame presents on
    // the first starved step — a Present resets the repeat count.)
    let mut repeats = 0_u32;
    let mut banded = false;
    for _ in 0..16 {
        let outcome = step(&mut scheduler, &mut queue, &mut sink);
        if outcome.sync.band == SyncBand::ExtendEarly {
            banded = true;
        }
        match outcome.action {
            PresentAction::RepeatLast(_) => repeats += 1,
            PresentAction::Background => break,
            PresentAction::Present(_) => repeats = 0,
        }
        clock.advance_ns(FRAME);
        assert!(repeats <= 5, "repeat budget runaway: {repeats}");
    }
    assert!(banded, "ExtendEarly never engaged");
    // 3 configured + 2 extension = 5 repeats, then background.
    assert_eq!(repeats, 5, "repeat budget with the +2 extension");
}
