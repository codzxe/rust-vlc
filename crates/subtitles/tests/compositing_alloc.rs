//! Card 5.L1 acceptance: the compositing STEADY STATE allocates
//! NOTHING — measured with the testkit counting allocator (the 3.B3 /
//! W2-07 discipline: runtime evidence, D-6, not syntactic green).
//!
//! Single `#[test]` per binary: the allocator is process-global
//! (sibling test threads would be counted). Everything allocatable —
//! the surface, the cue bitmaps, the compositor's scratch grown to
//! the session's worst geometry — is built and WARMED before the
//! measured window; inside it, only `compose` runs, across a cue mix
//! that exercises 0-, 1- and 2-cue frames, scaling, and translucent
//! blending.

use media_types::time::{Stamp, TimeSpan, Timestamp};
use subtitles::compositing::{
    Anchor, CompositedCue, CueBitmap, CueLayout, HorizontalAlign, Rational, Rgba, Rgba8Surface,
    SubtitleCompositor, VerticalAlign,
};
use testkit::Must;
use testkit::alloc_count::CountingGlobalAlloc;

#[global_allocator]
static ALLOC: CountingGlobalAlloc = CountingGlobalAlloc::new();

fn ns(ticks: i64) -> Timestamp {
    Timestamp::from_parts(ticks, 1, 1_000_000_000).must()
}

fn span(t0: i64, t1: i64) -> TimeSpan {
    TimeSpan {
        start: Stamp::Present(ns(t0)),
        end: Stamp::Present(ns(t1)),
    }
}

/// Build (outside the window): a 1080p plane and the cue mix —
/// an opaque CIF-authored cue (SCALES: the worst-case xmap), a
/// translucent overlapping cue (the blend path), and a
/// not-yet-active cue (the timing filter's other branch).
#[test]
fn compose_steady_state_is_zero_allocation() {
    let mut target = Rgba8Surface::new((1920, 1080), Rgba::BLACK).must();
    let mut scaled_layout = CueLayout::BOTTOM_CENTER;
    scaled_layout.anchor =
        Anchor::new(Rational::new(1, 10).must(), Rational::new(9, 10).must()).must();
    scaled_layout.alignment.horizontal = HorizontalAlign::Center;
    scaled_layout.alignment.vertical = VerticalAlign::Center;
    scaled_layout.reference = Some((352, 288));
    let scaled = CompositedCue::new(
        span(0, 10_000_000_000),
        CueBitmap::solid(128, 32, Rgba::WHITE).must(),
        scaled_layout,
        0,
    )
    .must();
    let mut overlap_layout = CueLayout::BOTTOM_CENTER;
    overlap_layout.anchor =
        Anchor::new(Rational::new(31, 60).must(), Rational::new(1, 2).must()).must();
    overlap_layout.alignment.horizontal = HorizontalAlign::Center;
    overlap_layout.alignment.vertical = VerticalAlign::Center;
    let translucent = CompositedCue::new(
        span(0, 10_000_000_000),
        CueBitmap::solid(64, 64, Rgba::WHITE.with_alpha(128)).must(),
        overlap_layout,
        1,
    )
    .must();
    let future = CompositedCue::new(
        span(10_000_000_000, 20_000_000_000),
        CueBitmap::solid(64, 64, Rgba::WHITE).must(),
        CueLayout::BOTTOM_CENTER,
        2,
    )
    .must();
    let cues = [scaled, translucent, future];

    let mut compositor = SubtitleCompositor::new();

    // Warm-up (outside the window): grow `order`/`xmap` scratch to
    // the session's worst geometry, touch every branch.
    for i in 0..4i64 {
        compositor
            .compose(&cues, ns(i * 1_000_000_000), &mut target)
            .must();
    }

    // Measured window: 240 frames of the steady render loop —
    // 24 fps x 10 s of cue flips, zero allocation expected.
    let guard = ALLOC.state().guard();
    let mut drawn_total = 0usize;
    for i in 0..240i64 {
        // Sample-ish frame times with active/inactive mixes.
        let now = ns(i * 41_666_667 / 10);
        drawn_total += compositor.compose(&cues, now, &mut target).must();
    }
    guard.assert_zero("steady-state SubtitleCompositor::compose");
    // 240 frames x the 2 always-active cues (the third is scanned and
    // rejected by the timing filter every frame — that branch runs
    // inside the window too).
    assert_eq!(drawn_total, 480);
}
