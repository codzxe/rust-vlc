//! The 2.C4 ↔ 2.D1 seam proof (card 2.D1: "Presenter-trait
//! integration with video-engine"): video-engine's **real**
//! [`PresentationScheduler`] and [`PresentationQueue`] driving this
//! crate's [`Renderer`] through the [`Presenter`] trait — the wiring
//! shape doc 03 §4.10 c describes (scheduler → queue → the render
//! side), on a virtual clock, zero sleeps (doc 03 §8).
//!
//! The frames carry a per-frame **marker pixel** (the deterministic
//! synthetic pattern with pixel (5,5) set to the frame index), so
//! every surface readback identifies exactly which frame is on screen
//! — ordering, staleness and retry claims are checked against *content*,
//! not counters.

mod common;

use std::num::NonZeroUsize;

use common::{VirtualClock, expected_pattern};
use media_types::GenerationId;
use media_types::frame::{
    ColorDescription, ColorMatrix, ColorPrimaries, ColorRange, FrameStorage,
    TransferCharacteristics, VideoFrame,
};
use media_types::packet::SharedBytes;
use media_types::time::{Nanoseconds, Stamp, Timestamp};
use renderer_wgpu::Renderer;
use renderer_wgpu::color::SurfaceFormat;
use renderer_wgpu::params::Extent2D;
use renderer_wgpu::surface::{PresentMode, SurfaceConfig};
use testkit::{Must, assert_err};
use video_engine::{
    EnqueueOutcome, NANO_BASE, PresentAction, PresentRequest, PresentationQueue,
    PresentationScheduler, PushOutcome, SchedulerCfg,
};

/// 30 fps nominal frame duration (ns) — the doc 03 §4.10 e default.
const FRAME: u64 = 33_333_334;

/// The suite's frame extent: small, and with pixel (5,5) clear of the
/// pattern's corner markers (4×4) and 16-px grid — the marker slot.
const EXTENT: Extent2D = Extent2D {
    width: 32,
    height: 24,
};

/// The doc's default scheduler configuration for 30 fps media.
fn default_cfg() -> SchedulerCfg {
    SchedulerCfg::for_frame_duration(Nanoseconds(FRAME)).must()
}

/// A headless renderer at the suite's extent.
fn renderer() -> Renderer {
    Renderer::new_headless(
        EXTENT,
        SurfaceConfig {
            format: SurfaceFormat::Bgra8Srgb,
            present_mode: PresentMode::Fifo,
        },
    )
    .must()
}

/// The synthetic pattern with pixel (5,5) overwritten by the frame's
/// index marker (the corner/grid structure at (5,5) is deliberately
/// destroyed — the marker is the only thing that matters here).
fn marked_bytes(extent: Extent2D, index: u8) -> Vec<u8> {
    let mut rgba = expected_pattern(extent);
    let at = (5 * extent.width as usize + 5) * 4;
    rgba[at..at + 4].copy_from_slice(&[index, 0xEE, 0x77, 255]);
    rgba
}

/// A synthetic frame carrying `index`'s marker, PTS-stamped for the
/// `pts_ns` slot.
fn marked_frame(extent: Extent2D, pts_ns: i64, index: u8) -> VideoFrame {
    VideoFrame {
        format: media_types::PixelFormat::Rgba8,
        size: (extent.width, extent.height),
        data: FrameStorage::Cpu(SharedBytes::from_vec(marked_bytes(extent, index))),
        pts: Stamp::Present(Timestamp::new(pts_ns, NANO_BASE)),
        color: ColorDescription {
            primaries: ColorPrimaries::Bt709,
            transfer: TransferCharacteristics::Srgb,
            matrix: ColorMatrix::Bt709,
            range: ColorRange::Full,
        },
        interlaced: false,
        generation: GenerationId::FIRST,
    }
}

/// A marked request scheduled for the `when_ns` slot.
fn marked_request(extent: Extent2D, when_ns: i64, index: u8) -> PresentRequest {
    PresentRequest {
        frame: marked_frame(extent, when_ns, index),
        when: Timestamp::new(when_ns, NANO_BASE),
        generation: GenerationId::FIRST,
    }
}

/// Read the surface's marker pixel — *which frame is on screen*.
fn surface_marker(surface: &Renderer, extent: Extent2D) -> [u8; 4] {
    let rgba = surface.readback_rgba().must();
    let at = (5 * extent.width as usize + 5) * 4;
    [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]]
}

/// One production-shaped wiring step (the 2.C4 suite's shape): tick,
/// enqueue the action's request (if any), drain the queue to the
/// renderer — exactly one delivery per tick that enqueued one. A
/// refusal here is a wiring bug, not a scenario (the detached-surface
/// refusal is driven by `drain_to` directly in its own test).
fn step(
    scheduler: &mut PresentationScheduler,
    queue: &mut PresentationQueue,
    sink: &mut Renderer,
) -> PresentAction {
    let outcome = scheduler.tick();
    let request = match &outcome.action {
        PresentAction::Present(request) | PresentAction::RepeatLast(request) => {
            Some(request.clone())
        }
        PresentAction::Background => None,
    };
    let mut enqueued = false;
    if let Some(request) = request {
        match queue.try_push(request) {
            EnqueueOutcome::Enqueued => enqueued = true,
            refused => unreachable!("wide queue refused a push: {refused:?}"),
        }
    }
    let delivered = queue
        .drain_to(sink)
        .unwrap_or_else(|_| unreachable!("wiring drain refused"));
    // One tick enqueues at most one request (present or repeat), and
    // the wiring drains immediately: delivered is exactly that.
    assert_eq!(
        delivered,
        usize::from(enqueued),
        "the drain delivered exactly what the tick enqueued"
    );
    outcome.action
}

// ---------------------------------------------------------------------------
// AC: the full production path, end to end
// ---------------------------------------------------------------------------

#[test]
fn ac_scheduler_queue_renderer_end_to_end() {
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = PresentationQueue::new(NonZeroUsize::new(64).must());
    let mut sink = renderer();

    // Produce-then-present, one frame per slot, 2 seconds of 30 fps —
    // every present lands on the renderer through the queue.
    for frame_index in 0..60u8 {
        let pts = i64::from(frame_index) * FRAME as i64;
        match scheduler.push(marked_frame(EXTENT, pts, frame_index)) {
            PushOutcome::Scheduled => {}
            other => unreachable!("steady-state push refused: {other:?}"),
        }
        clock.set_ns(pts.unsigned_abs());
        let action = step(&mut scheduler, &mut queue, &mut sink);
        match action {
            PresentAction::Present(ref r) => assert_eq!(r.when.ticks, pts),
            other => unreachable!("frame {frame_index} due, got {other:?}"),
        }
        // Content-ordering proof: the surface shows THIS frame now.
        assert_eq!(
            surface_marker(&sink, EXTENT),
            [frame_index, 0xEE, 0x77, 255]
        );
    }

    // Conservation: everything pushed was presented, nothing lost.
    let stats = scheduler.stats();
    assert_eq!(stats.pushed, 60);
    assert_eq!(stats.presented, 60);
    assert_eq!(
        stats.dropped_late + stats.dropped_superseded + stats.dropped_stale,
        0
    );
    assert!(queue.is_empty());

    // Measured staging through the seam: 60 transfers of exactly the
    // frame bytes each (the card's "measured staging" observable,
    // asserted at the wiring level).
    let staging = sink.staging_stats();
    assert_eq!(staging.frames_staged, 60);
    assert_eq!(staging.staging_copies, 60);
    assert_eq!(staging.bytes_staged, 60 * 32 * 24 * 4);

    // And the final surface is frame 59's full content (marker + all).
    assert_eq!(sink.readback_rgba().must(), marked_bytes(EXTENT, 59));
}

// ---------------------------------------------------------------------------
// Repeats: a repeat IS a present (the 2.C4 sink contract)
// ---------------------------------------------------------------------------

#[test]
fn repeat_actions_re_present_and_the_surface_persists() {
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = PresentationQueue::new(NonZeroUsize::new(64).must());
    let mut sink = renderer();

    // Frame 0 presented at slot 0.
    match scheduler.push(marked_frame(EXTENT, 0, 0)) {
        PushOutcome::Scheduled => {}
        other => unreachable!("{other:?}"),
    }
    let action = step(&mut scheduler, &mut queue, &mut sink);
    assert!(matches!(action, PresentAction::Present(_)));
    assert_eq!(surface_marker(&sink, EXTENT), [0, 0xEE, 0x77, 255]);

    // The clock advances but no new frames arrive: within the repeat
    // budget the scheduler emits RepeatLast, the wiring delivers it,
    // and the renderer re-presents (a surface-persisting renderer MAY
    // no-op; ours re-composites — both honor the contract, this pins
    // ours). The surface content is unchanged.
    for _ in 0..3 {
        clock.advance_ns(FRAME);
        let action = step(&mut scheduler, &mut queue, &mut sink);
        assert!(matches!(action, PresentAction::RepeatLast(_)));
        assert_eq!(surface_marker(&sink, EXTENT), [0, 0xEE, 0x77, 255]);
    }
    assert_eq!(scheduler.stats().repeats, 3);

    // 1 present + 3 repeats = 4 staging transfers; the budget is spent.
    assert_eq!(sink.staging_stats().staging_copies, 4);

    // Further ticks go Background: nothing enqueued, renderer idle.
    clock.advance_ns(FRAME);
    let action = step(&mut scheduler, &mut queue, &mut sink);
    assert!(matches!(action, PresentAction::Background));
    assert_eq!(
        sink.staging_stats().staging_copies,
        4,
        "background touches nothing"
    );
    assert_eq!(scheduler.stats().background, 1);
}

// ---------------------------------------------------------------------------
// Background before anything: an idle renderer
// ---------------------------------------------------------------------------

#[test]
fn background_actions_leave_the_renderer_idle() {
    let clock = VirtualClock::new();
    let mut scheduler = PresentationScheduler::new(default_cfg(), clock.clock());
    let mut queue = PresentationQueue::new(NonZeroUsize::new(64).must());
    let mut sink = renderer();

    clock.advance_ns(FRAME * 10);
    let action = step(&mut scheduler, &mut queue, &mut sink);
    assert!(matches!(action, PresentAction::Background));

    // Nothing staged, nothing on the surface (the untouched zeros).
    let staging = sink.staging_stats();
    assert_eq!(staging.frames_staged, 0);
    assert_eq!(staging.staging_copies, 0);
    assert_eq!(staging.bytes_staged, 0);
    assert!(sink.readback_rgba().must().iter().all(|&b| b == 0));
    assert!(queue.is_empty());
}

// ---------------------------------------------------------------------------
// Surface loss: the drain stops, keeps the request, resumes in order
// ---------------------------------------------------------------------------

#[test]
fn detached_surface_stops_the_drain_and_reattachment_resumes_in_order() {
    let mut queue = PresentationQueue::new(NonZeroUsize::new(4).must());
    let mut sink = renderer();

    // Two requests queued directly (the queue is the seam under test
    // here; the scheduler path is proven above).
    match queue.try_push(marked_request(EXTENT, 0, 10)) {
        EnqueueOutcome::Enqueued => {}
        other => unreachable!("{other:?}"),
    }
    match queue.try_push(marked_request(EXTENT, FRAME as i64, 20)) {
        EnqueueOutcome::Enqueued => {}
        other => unreachable!("{other:?}"),
    }

    // Detach: the first present refuses with the typed engine error.
    let surface = sink.detach_surface().must();
    let err = assert_err(queue.drain_to(&mut sink), "no surface");
    assert_eq!(err.delivered, 0);
    assert_eq!(err.source.stage, media_types::error::PipelineStage::Present);
    assert_eq!(err.source.context.code, "render.surface.missing");

    // The refused request stays queued — retry is in-order, no skip.
    assert_eq!(queue.len(), 2);
    assert_eq!(queue.stats().delivered, 0);

    // Reattach and drain: both land, in order, the last one shows.
    sink.attach_surface(surface);
    let delivered = queue.drain_to(&mut sink).must();
    assert_eq!(delivered, 2);
    assert_eq!(queue.stats().delivered, 2);
    assert!(queue.is_empty());
    assert_eq!(surface_marker(&sink, EXTENT), [20, 0xEE, 0x77, 255]);
    assert_eq!(sink.staging_stats().frames_staged, 2);
}

// ---------------------------------------------------------------------------
// Generation staleness: purged wholesale, only live frames composite
// ---------------------------------------------------------------------------

#[test]
fn generation_publish_purges_stale_requests_before_the_renderer() {
    let live = GenerationId::new(1);
    let mut queue =
        PresentationQueue::with_generation(NonZeroUsize::new(4).must(), GenerationId::FIRST);
    let mut sink = renderer();

    // A live request resident, then the session mints a new generation.
    match queue.try_push(marked_request(EXTENT, 0, 7)) {
        EnqueueOutcome::Enqueued => {}
        other => unreachable!("{other:?}"),
    }
    let flushed = queue.publish(live);
    assert_eq!(flushed, 1, "the resident request dropped wholesale");
    assert!(queue.is_empty());

    // An old-generation push is refused at the boundary, request back.
    match queue.try_push(marked_request(EXTENT, 0, 8)) {
        EnqueueOutcome::Stale { .. } => {}
        other => unreachable!("stale push accepted: {other:?}"),
    }

    // The live generation's frame is the only one that composites.
    let request = PresentRequest {
        frame: marked_frame(EXTENT, 0, 9),
        when: Timestamp::new(0, NANO_BASE),
        generation: live,
    };
    match queue.try_push(request) {
        EnqueueOutcome::Enqueued => {}
        other => unreachable!("{other:?}"),
    }
    let delivered = queue.drain_to(&mut sink).must();
    assert_eq!(delivered, 1);
    assert_eq!(surface_marker(&sink, EXTENT), [9, 0xEE, 0x77, 255]);
    assert_eq!(
        sink.staging_stats().frames_staged,
        1,
        "the purged and refused frames never reached the renderer"
    );
    assert_eq!(queue.stats().stale_rejected, 1);
}
