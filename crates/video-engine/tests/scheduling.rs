//! Scheduling acceptance tests (card 2.C4 AC, doc 03 §4.10 f):
//! virtual-clock determinism — in-order presentation, late-frame
//! drops, early holds, boundary conditions, generation stale-drops —
//! all on injected time, **zero sleeps** (doc 03 §8).
//!
//! The wiring under test is the production shape at v0: scheduler
//! over an injected [`PlaybackClock`], requests through the bounded
//! presentation queue, recorder at the [`Presenter`] boundary. The
//! clock is a [`MediaClock`] — monotonic master for exact-nanosecond
//! scenarios, audio master (fake 48 kHz sample counter) for the
//! production-shaped one.

mod common;

use std::num::NonZeroUsize;
use std::sync::Arc;

use common::{AudioMasterClock, RecordingPresenter, VirtualClock, frame};
use media_types::time::Nanoseconds;
use media_types::GenerationId;
use testkit::Must;
use video_engine::{
    DropPolicy, EnqueueOutcome, PresentAction, PresentationQueue, PresentationScheduler,
    PushOutcome, SchedulerCfg, TickOutcome,
};

/// 30 fps nominal frame duration (ns) — the doc 03 §4.10 e default
/// threshold and the half-frame lead's basis.
const FRAME: u64 = 33_333_334;

/// The doc's default configuration for 30 fps media.
fn default_cfg() -> SchedulerCfg {
    SchedulerCfg::for_frame_duration(Nanoseconds(FRAME)).must()
}

/// One production-shaped wiring step: tick, enqueue the action's
/// request (if any), drain the queue to the presenter.
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

/// A queue big enough that bound refusal never interferes with the
/// scheduling scenarios (bound behavior is this suite's neighbor
/// `presentation_queue.rs`).
fn wide_queue() -> PresentationQueue {
    PresentationQueue::new(NonZeroUsize::new(64).must())
}

// ---------------------------------------------------------------------------
// AC: in-order presentation when the clock advances normally
// ---------------------------------------------------------------------------

#[test]
fn ac_in_order_presentation_when_the_clock_advances_normally() {
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = wide_queue();
    let mut sink = RecordingPresenter::new();

    // Produce-then-present, one frame per slot, 2 seconds of 30 fps.
    for frame_index in 0..60i64 {
        let pts = frame_index * FRAME as i64;
        let _ = scheduler.push(frame(pts, GenerationId::FIRST));
        clock.set_ns(pts.unsigned_abs());
        let outcome = step(&mut scheduler, &mut queue, &mut sink);
        match &outcome.action {
            PresentAction::Present(r) => assert_eq!(r.when.ticks, pts),
            other => unreachable!("frame {frame_index} due, got {other:?}"),
        }
        assert!(outcome.drops.late.is_empty(), "no drops in steady state");
        assert!(outcome.drops.superseded.is_empty());
    }

    let want: Vec<i64> = (0..60).map(|k| k * FRAME as i64).collect();
    assert_eq!(sink.slots_seen(), want, "every frame, exact order");
    sink.assert_in_order("normal advance");
    let stats = scheduler.stats();
    assert_eq!(stats.presented, 60);
    assert_eq!(stats.dropped_late, 0);
    assert_eq!(stats.dropped_superseded, 0);
    assert_eq!(stats.dropped_stale, 0);
    // Conservation: every admitted frame was presented, none lost.
    assert_eq!(stats.pushed, stats.presented);
    assert_eq!(sink.gens_seen(), vec![GenerationId::FIRST; 60]);
}

// ---------------------------------------------------------------------------
// AC: late frames dropped — never presented late, never reordered
// ---------------------------------------------------------------------------

#[test]
fn ac_late_frames_dropped_never_presented_late_or_reordered() {
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = wide_queue();
    let mut sink = RecordingPresenter::new();

    // Steady state through frame 9, then the clock stalls 30 frames
    // while 10..19 wait in the window.
    for frame_index in 0..20i64 {
        let pts = frame_index * FRAME as i64;
        let _ = scheduler.push(frame(pts, GenerationId::FIRST));
        if frame_index < 10 {
            clock.set_ns(pts.unsigned_abs());
            step(&mut scheduler, &mut queue, &mut sink);
        }
    }
    clock.set_ns(40 * FRAME); // 30 frames past the last presented slot
    let outcome = step(&mut scheduler, &mut queue, &mut sink);

    // Late bound: 40 − 1 frames → everything at or below 39×FRAME is
    // late; frames 10..19 all are. None was presented.
    assert_eq!(outcome.drops.late.len(), 10, "{:?}", outcome.drops.late);
    assert_eq!(scheduler.stats().dropped_late, 10);
    assert!(
        !sink.slots_seen().iter().any(|p| (10..20).contains(&(*p / FRAME as i64))),
        "late frames never presented: {:?}",
        sink.slots_seen()
    );

    // Playback resumes: fresh frames present in order, on their slots.
    for frame_index in 40..50i64 {
        let pts = frame_index * FRAME as i64;
        let _ = scheduler.push(frame(pts, GenerationId::FIRST));
        clock.set_ns(pts.unsigned_abs());
        step(&mut scheduler, &mut queue, &mut sink);
    }
    sink.assert_in_order("after the stall");
    let got: Vec<i64> = sink.slots_seen();
    assert_eq!(&got[..10], &(0..10).map(|k| k * FRAME as i64).collect::<Vec<_>>());
    assert_eq!(
        &got[10..],
        &(40..50).map(|k| k * FRAME as i64).collect::<Vec<_>>(),
        "the stall gap is exactly the dropped 10..20"
    );
    assert_eq!(scheduler.stats().presented, 20);
}

#[test]
fn drops_are_never_resurrected_after_their_stall() {
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = wide_queue();
    let mut sink = RecordingPresenter::new();
    for frame_index in 0..5i64 {
        let _ = scheduler.push(frame(frame_index * FRAME as i64, GenerationId::FIRST));
    }
    clock.set_ns(100 * FRAME);
    let outcome = step(&mut scheduler, &mut queue, &mut sink);
    assert_eq!(outcome.drops.late.len(), 5);
    // Repeated ticks at the same stalled position never re-present or
    // re-drop them: the window is empty and the accounting is stable.
    for _ in 0..4 {
        let outcome = step(&mut scheduler, &mut queue, &mut sink);
        assert_eq!(outcome.drops.late.len(), 0);
        assert!(matches!(outcome.action, PresentAction::Background));
    }
    assert_eq!(scheduler.stats().dropped_late, 5);
    assert!(sink.slots_seen().is_empty());
}

// ---------------------------------------------------------------------------
// AC: early frames held until their slot
// ---------------------------------------------------------------------------

#[test]
fn ac_early_frames_are_held_until_their_slot() {
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = wide_queue();
    let mut sink = RecordingPresenter::new();

    // Six frames pushed up front; the clock walks to each slot.
    for frame_index in 0..6i64 {
        let _ = scheduler.push(frame(frame_index * FRAME as i64, GenerationId::FIRST));
    }
    // Before frame 1's due position (pts − lead), ticks hold it.
    clock.set_ns(10_000_000);
    let outcome = step(&mut scheduler, &mut queue, &mut sink);
    assert!(matches!(outcome.action, PresentAction::Present(_))); // frame 0 (lead)
    clock.set_ns(15_000_000);
    let outcome = step(&mut scheduler, &mut queue, &mut sink);
    match outcome.action {
        PresentAction::RepeatLast(_) => {}
        other => unreachable!("frame 1 still early at now=15ms, got {other:?}"),
    }
    assert_eq!(sink.slots_seen(), vec![0], "frame 1 not yet presented");
    assert_eq!(
        outcome.next_due.map(|t| t.ticks),
        Some(FRAME as i64 - 16_666_667),
        "wait hint: frame 1 due at pts − lead"
    );
    assert_eq!(scheduler.pending_len(), 5, "held, not dropped");

    // At the due position it presents — not before, not after.
    clock.set_ns(FRAME);
    let outcome = step(&mut scheduler, &mut queue, &mut sink);
    match outcome.action {
        PresentAction::Present(r) => assert_eq!(r.when.ticks, FRAME as i64),
        other => unreachable!("{other:?}"),
    }
    assert_eq!(sink.slots_seen(), vec![0, FRAME as i64]);
}

// ---------------------------------------------------------------------------
// AC: boundary conditions — PTS == now, just-late, far-late, lead edges
// ---------------------------------------------------------------------------

/// One boundary scenario: a fresh scheduler at `now` with threshold 10,
/// lead 5, one frame at `pts`, stepping once.
fn boundary_step(now: i64, pts: i64) -> (PresentAction, usize) {
    let clock = VirtualClock::new();
    let cfg = SchedulerCfg::new(
        DropPolicy::Late {
            threshold: Nanoseconds(10),
        },
        0,
        Nanoseconds(5),
        Some(Nanoseconds(10)),
        NonZeroUsize::new(8).must(),
    )
    .must();
    let mut scheduler = PresentationScheduler::new(cfg, clock.clock());
    let _ = scheduler.push(frame(pts, GenerationId::FIRST));
    clock.set_ns(now.unsigned_abs());
    let outcome = scheduler.tick();
    (outcome.action, outcome.drops.late.len())
}

#[test]
fn ac_boundary_pts_equals_now_presents() {
    let (action, late) = boundary_step(1000, 1000);
    assert!(matches!(action, PresentAction::Present(_)), "{action:?}");
    assert_eq!(late, 0);
}

#[test]
fn ac_boundary_pts_exactly_on_the_late_threshold_presents() {
    // pts == now − threshold: ON the bound (strict <) — presents.
    let (action, late) = boundary_step(1000, 990);
    assert!(matches!(action, PresentAction::Present(_)), "{action:?}");
    assert_eq!(late, 0);
}

#[test]
fn ac_boundary_pts_one_past_the_late_threshold_drops() {
    let (action, late) = boundary_step(1000, 989);
    assert!(matches!(action, PresentAction::Background), "{action:?}");
    assert_eq!(late, 1, "just-late is dropped and counted");
}

#[test]
fn ac_boundary_far_late_drops_the_same_way() {
    let (action, late) = boundary_step(1000, 0);
    assert!(matches!(action, PresentAction::Background), "{action:?}");
    assert_eq!(late, 1, "far-late: one drop, same accounting");
}

#[test]
fn ac_boundary_pts_exactly_on_the_due_bound_presents() {
    // pts == now + lead: inclusive ≤ — presents.
    let (action, _late) = boundary_step(1000, 1005);
    assert!(matches!(action, PresentAction::Present(_)), "{action:?}");
}

#[test]
fn ac_boundary_one_past_the_due_bound_holds() {
    let clock = VirtualClock::new();
    let cfg = SchedulerCfg::new(
        DropPolicy::Late {
            threshold: Nanoseconds(10),
        },
        0,
        Nanoseconds(5),
        Some(Nanoseconds(10)),
        NonZeroUsize::new(8).must(),
    )
    .must();
    let mut scheduler = PresentationScheduler::new(cfg, clock.clock());
    let _ = scheduler.push(frame(1006, GenerationId::FIRST));
    clock.set_ns(1000);
    let outcome = scheduler.tick();
    assert!(matches!(outcome.action, PresentAction::Background));
    assert_eq!(outcome.drops.late.len(), 0);
    assert_eq!(outcome.next_due.map(|t| t.ticks), Some(1001));
    assert_eq!(scheduler.pending_len(), 1, "held until its slot");
}

// ---------------------------------------------------------------------------
// AC: generation-stale frames dropped (doc 03 §5 checking point 7)
// ---------------------------------------------------------------------------

#[test]
fn ac_generation_stale_frames_are_dropped_end_to_end() {
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = wide_queue();
    let mut sink = RecordingPresenter::new();

    // Present one frame of generation FIRST, leave three pending.
    let _ = scheduler.push(frame(0, GenerationId::FIRST));
    clock.set_ns(0);
    step(&mut scheduler, &mut queue, &mut sink);
    for frame_index in 5..8i64 {
        let _ = scheduler.push(frame(frame_index * FRAME as i64, GenerationId::FIRST));
    }

    // The session seeks: new generation minted on the clock AND the
    // presentation queue's mailbox replaced wholesale (doc 03 §5(7)).
    let next = GenerationId::FIRST.next().must();
    clock.set_generation(next);
    assert_eq!(queue.publish(next), 0, "nothing resident yet");

    let outcome = step(&mut scheduler, &mut queue, &mut sink);
    assert_eq!(outcome.drops.stale, 3, "pending frames of the dead generation");
    assert_eq!(scheduler.stats().dropped_stale, 3);
    assert!(matches!(outcome.action, PresentAction::Background));
    assert_eq!(
        sink.gens_seen(),
        vec![GenerationId::FIRST],
        "no stale-generation frame reached the presenter"
    );

    // Late-arriving old-generation pushes are refused at the boundary.
    match scheduler.push(frame(50 * FRAME as i64, GenerationId::FIRST)) {
        PushOutcome::Stale { current, .. } => assert_eq!(current, next),
        other => unreachable!("{other:?}"),
    }
    assert_eq!(scheduler.stats().stale_rejected_push, 1);

    // The new generation flows: seek target presents on its slot.
    let seek_target = 90 * FRAME;
    let _ = scheduler.push(frame(seek_target as i64, next));
    clock.set_ns(seek_target);
    let outcome = step(&mut scheduler, &mut queue, &mut sink);
    match &outcome.action {
        PresentAction::Present(r) => {
            assert_eq!(r.when.ticks, seek_target as i64);
            assert_eq!(r.generation, next);
        }
        other => unreachable!("{other:?}"),
    }
    assert_eq!(sink.gens_seen(), vec![GenerationId::FIRST, next]);
    sink.assert_in_order("across the generation boundary");
}

// ---------------------------------------------------------------------------
// Doc 03 §4.10 f properties, exercised at the wiring level
// ---------------------------------------------------------------------------

#[test]
fn no_frame_is_presented_out_of_order_under_shuffled_arrival() {
    // Hand-picked arrival permutations (a property framework is a doc
    // 04 candidate; deterministic permutations cover the same ground).
    let permutations: [Vec<i64>; 5] = [
        vec![4, 2, 0, 1, 3],
        vec![0, 4, 3, 2, 1],
        vec![3, 1, 4, 0, 2],
        vec![2, 4, 1, 3, 0],
        vec![1, 3, 0, 4, 2],
    ];
    for (run, order) in permutations.iter().enumerate() {
        let clock = VirtualClock::new();
        let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
        let mut queue = wide_queue();
        let mut sink = RecordingPresenter::new();
        for frame_index in order {
            let _ = scheduler.push(frame(frame_index * FRAME as i64, GenerationId::FIRST));
        }
        for frame_index in 0..5i64 {
            clock.set_ns((frame_index * FRAME as i64).unsigned_abs());
            step(&mut scheduler, &mut queue, &mut sink);
        }
        sink.assert_in_order(&format!("permutation {run}: {order:?}"));
        let want: Vec<i64> = (0..5).map(|k| k * FRAME as i64).collect();
        assert_eq!(sink.slots_seen(), want, "permutation {run}: {order:?}");
    }
}

#[test]
fn presented_error_stays_within_the_policy_bounds_at_every_present() {
    // The doc 03 §4.10 f A/V-sync scenario, at the layer this crate
    // owns: at every present, the scheduled slot is never beyond the
    // late threshold behind the clock and never more than the lead
    // ahead of it (the full-pipeline ±10 ms tolerance `[PROPOSAL]` is
    // 2.C3/2.E's integration; these are the scheduler's own bounds).
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = wide_queue();
    let mut sink = RecordingPresenter::new();
    let threshold = FRAME as i64;
    let lead = 16_666_667i64;

    let mut check = |now_ns: i64| {
        let outcome = scheduler.tick();
        if let PresentAction::Present(r) = &outcome.action {
            assert!(
                r.when.ticks >= now_ns - threshold,
                "presented {} too late at now {now_ns}",
                r.when.ticks
            );
            assert!(
                r.when.ticks <= now_ns + lead,
                "presented {} too early at now {now_ns}",
                r.when.ticks
            );
        }
        let request = match &outcome.action {
            PresentAction::Present(r) | PresentAction::RepeatLast(r) => Some(r.clone()),
            PresentAction::Background => None,
        };
        if let Some(request) = request {
            let _ = queue.try_push(request);
        }
        let _ = queue.drain_to(&mut sink);
    };

    // Normal advance, then a 15-frame stall, then recovery.
    for frame_index in 0..30i64 {
        let pts = frame_index * FRAME as i64;
        let _ = scheduler.push(frame(pts, GenerationId::FIRST));
        clock.set_ns(pts.unsigned_abs());
        check(pts);
    }
    for frame_index in 30..40i64 {
        let _ = scheduler.push(frame(frame_index * FRAME as i64, GenerationId::FIRST));
    }
    clock.set_ns(55 * FRAME);
    check(55 * FRAME as i64);
    for frame_index in 55..65i64 {
        let pts = frame_index * FRAME as i64;
        let _ = scheduler.push(frame(pts, GenerationId::FIRST));
        clock.set_ns(pts.unsigned_abs());
        check(pts);
    }
    sink.assert_in_order("sync-bounds run");
    assert!(sink.slots_seen().len() >= 35, "{}", sink.slots_seen().len());
}

// ---------------------------------------------------------------------------
// The production-shaped master: audio samples drive the clock
// ---------------------------------------------------------------------------

#[test]
fn audio_master_sample_counter_drives_the_schedule() {
    // 48 kHz counter, 30 fps video: 1600 samples per frame exactly.
    // The video grid (33_333_334 ns) and the sample grid (33_333_333⅓
    // ns) disagree by <1 ns per frame — the lead absorbs it, exactly
    // as in real A/V playback.
    let clock = AudioMasterClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = wide_queue();
    let mut sink = RecordingPresenter::new();

    for frame_index in 0..60i64 {
        let pts = frame_index * FRAME as i64;
        let _ = scheduler.push(frame(pts, GenerationId::FIRST));
        clock.advance_samples(1600);
        let outcome = step(&mut scheduler, &mut queue, &mut sink);
        match &outcome.action {
            PresentAction::Present(r) => assert_eq!(r.when.ticks, pts),
            other => unreachable!("frame {frame_index}, got {other:?}"),
        }
    }
    let want: Vec<i64> = (0..60).map(|k| k * FRAME as i64).collect();
    assert_eq!(sink.slots_seen(), want);
    let stats = scheduler.stats();
    assert_eq!(stats.presented, 60);
    assert_eq!(stats.dropped_late, 0, "the <1 ns grid drift never drops");
}

#[test]
fn audio_master_stall_drops_late_frames() {
    let clock = AudioMasterClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = wide_queue();
    let mut sink = RecordingPresenter::new();
    for frame_index in 0..10i64 {
        let _ = scheduler.push(frame(frame_index * FRAME as i64, GenerationId::FIRST));
    }
    // The audio pump consumes 5 seconds while video waits.
    clock.advance_samples(5 * 48_000);
    let outcome = step(&mut scheduler, &mut queue, &mut sink);
    assert_eq!(outcome.drops.late.len(), 10, "{:?}", outcome.drops.late);
    assert!(matches!(outcome.action, PresentAction::Background));
    assert!(sink.slots_seen().is_empty());
}

// ---------------------------------------------------------------------------
// Policy axis: Never presents late (in order) — and Auto == doc default
// ---------------------------------------------------------------------------

#[test]
fn never_policy_presents_late_frames_in_order_via_supersede() {
    let clock = VirtualClock::new();
    let cfg = SchedulerCfg::new(
        DropPolicy::Never,
        0,
        Nanoseconds(0),
        Some(Nanoseconds(FRAME)),
        NonZeroUsize::new(16).must(),
    )
    .must();
    let mut scheduler = PresentationScheduler::new(cfg, clock.clock());
    let mut queue = wide_queue();
    let mut sink = RecordingPresenter::new();

    for frame_index in 0..6i64 {
        let _ = scheduler.push(frame(frame_index * FRAME as i64, GenerationId::FIRST));
    }
    // The clock lands on slot 3: frames 0..3 are due, the largest (3)
    // presents and 0..2 supersede — dropped structurally, NOT as late
    // (the Never policy never drops for lateness), never out of order.
    clock.set_ns(3 * FRAME);
    let outcome = step(&mut scheduler, &mut queue, &mut sink);
    match &outcome.action {
        PresentAction::Present(r) => assert_eq!(r.when.ticks, 3 * FRAME as i64),
        other => unreachable!("{other:?}"),
    }
    assert_eq!(outcome.drops.late.len(), 0, "never drops for lateness");
    assert_eq!(outcome.drops.superseded.len(), 3);
    assert_eq!(scheduler.stats().dropped_superseded, 3);

    for frame_index in 4..6i64 {
        clock.set_ns((frame_index * FRAME as i64).unsigned_abs());
        step(&mut scheduler, &mut queue, &mut sink);
    }
    assert_eq!(sink.slots_seen(), vec![3, 4, 5].map(|k| k * FRAME as i64));
    sink.assert_in_order("never policy");
}

#[test]
fn auto_policy_resolves_to_the_doc_default_one_frame_threshold() {
    let clock = VirtualClock::new();
    let cfg = SchedulerCfg::new(
        DropPolicy::Auto,
        3,
        Nanoseconds(16_666_667),
        Some(Nanoseconds(FRAME)),
        NonZeroUsize::new(16).must(),
    )
    .must();
    let mut scheduler = PresentationScheduler::new(cfg, clock.clock());
    let mut queue = wide_queue();
    let mut sink = RecordingPresenter::new();

    for frame_index in 0..5i64 {
        let _ = scheduler.push(frame(frame_index * FRAME as i64, GenerationId::FIRST));
    }
    clock.set_ns(10 * FRAME);
    let outcome = step(&mut scheduler, &mut queue, &mut sink);
    // One-frame threshold: frames 0..9 are late → 0..4 all dropped.
    assert_eq!(outcome.drops.late.len(), 5);
    assert!(sink.slots_seen().is_empty());
    assert_eq!(scheduler.stats().dropped_late, 5);
}

// ---------------------------------------------------------------------------
// Pause and repeat/background semantics at the wiring level
// ---------------------------------------------------------------------------

#[test]
fn paused_clock_freezes_the_schedule_mechanically() {
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = wide_queue();
    let mut sink = RecordingPresenter::new();
    let _ = scheduler.push(frame(0, GenerationId::FIRST));
    clock.set_ns(0);
    step(&mut scheduler, &mut queue, &mut sink);
    assert_eq!(sink.slots_seen(), vec![0]);

    // Pause: now() frozen, nothing new becomes due — v0 ticks are
    // mechanical (repeat budget drains to background; the wiring
    // decides whether to keep ticking while paused).
    clock.pause();
    let _ = scheduler.push(frame(FRAME as i64, GenerationId::FIRST));
    for _ in 0..6 {
        step(&mut scheduler, &mut queue, &mut sink);
    }
    assert_eq!(sink.slots_seen(), vec![0], "frame 1 held while frozen");
    assert_eq!(scheduler.pending_len(), 1);

    // Resume from the same position: the frame presents on its slot.
    clock.resume();
    clock.set_ns(FRAME);
    let outcome = step(&mut scheduler, &mut queue, &mut sink);
    match &outcome.action {
        PresentAction::Present(r) => assert_eq!(r.when.ticks, FRAME as i64),
        other => unreachable!("{other:?}"),
    }
}

#[test]
fn stats_conserve_every_admitted_frame() {
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = wide_queue();
    let mut sink = RecordingPresenter::new();

    for frame_index in 0..30i64 {
        let _ = scheduler.push(frame(frame_index * FRAME as i64, GenerationId::FIRST));
    }
    // A wild stall, then recovery.
    clock.set_ns(100 * FRAME);
    step(&mut scheduler, &mut queue, &mut sink);
    for frame_index in 100..110i64 {
        let _ = scheduler.push(frame(frame_index * FRAME as i64, GenerationId::FIRST));
        clock.set_ns((frame_index * FRAME as i64).unsigned_abs());
        step(&mut scheduler, &mut queue, &mut sink);
    }
    let stats = scheduler.stats();
    let accounted = stats.presented
        + stats.dropped_late
        + stats.dropped_superseded
        + stats.dropped_stale
        + scheduler.pending_len() as u64;
    assert_eq!(
        accounted, stats.pushed,
        "every admitted frame is presented, dropped (counted), or pending"
    );
    assert_eq!(
        sink.slots_seen().len() as u64,
        stats.presented,
        "the presenter saw exactly the presents"
    );
}

// ---------------------------------------------------------------------------
// Wiring-level integration with the queue's typed bound (neighbor suite
// owns the mechanics; this pins the scheduler-side accounting)
// ---------------------------------------------------------------------------

#[test]
fn window_bound_surfaces_as_typed_window_full_at_the_wiring_level() {
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    // Default window is 16: fill it with far-future frames.
    for frame_index in 0..16i64 {
        assert!(matches!(
            scheduler.push(frame((1000 + frame_index) * FRAME as i64, GenerationId::FIRST)),
            PushOutcome::Scheduled
        ));
    }
    let outcome = scheduler.push(frame(2000 * FRAME as i64, GenerationId::FIRST));
    match outcome {
        PushOutcome::WindowFull { frame, capacity } => {
            assert_eq!(capacity, 16);
            assert_eq!(
                frame.pts,
                media_types::time::Stamp::Present(media_types::time::Timestamp::new(
                    2000 * FRAME as i64,
                    media_clock::NANO_BASE
                ))
            );
        }
        other => unreachable!("{other:?}"),
    }
    assert_eq!(scheduler.stats().full_rejections, 1);
}

#[test]
fn queue_bound_surfaces_as_typed_full_at_the_wiring_level() {
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = PresentationQueue::new(NonZeroUsize::new(1).must());
    let mut sink = RecordingPresenter::new();

    // Tick once and enqueue WITHOUT draining: the depth-1 queue is now
    // full. The next tick's request is refused with the typed Full
    // signal — handed back to the wiring, never silently lost.
    let _ = scheduler.push(frame(0, GenerationId::FIRST));
    clock.set_ns(0);
    let outcome = scheduler.tick();
    let request = match &outcome.action {
        PresentAction::Present(r) => r.clone(),
        other => unreachable!("{other:?}"),
    };
    assert!(matches!(queue.try_push(request), EnqueueOutcome::Enqueued));
    assert_eq!(queue.len(), 1, "depth-1 queue now full");

    let _ = scheduler.push(frame(FRAME as i64, GenerationId::FIRST));
    clock.set_ns(FRAME);
    let outcome = scheduler.tick();
    let request = match &outcome.action {
        PresentAction::Present(r) => r.clone(),
        other => unreachable!("{other:?}"),
    };
    match queue.try_push(request) {
        EnqueueOutcome::Full { capacity, .. } => assert_eq!(capacity, 1),
        other => unreachable!("{other:?}"),
    }
    assert_eq!(queue.stats().full_rejections, 1);
    // Draining frees the slot; the producer's retry then lands.
    assert_eq!(queue.drain_to(&mut sink).must(), 1);
    assert_eq!(sink.slots_seen(), vec![0]);
}
