//! Card 5.L1 acceptance: DURATION on the REAL shared clock — the
//! media-clock crate's `MediaClock::audio_master` over a counting
//! sample source, exactly the production shape (doc 03 §4.7). The
//! prior session proved sample-exact boundaries on this clock; this
//! suite replicates that discipline for cues: the half-open `[t0,t1)`
//! interval flips on/off at exact SAMPLE boundaries (48 kHz), and at
//! exact NANOSECOND boundaries through the same compose path.

use media_clock::{MediaClock, PlaybackClock, SampleCounterSource};
use media_types::time::{Rate, Stamp, TimeSpan, Timestamp};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use subtitles::compositing::{
    CompositedCue, CueBitmap, CueLayout, Rgba, Rgba8Surface, SubtitleCompositor,
};
use testkit::Must;

/// The audio-device stand-in: the sample counter the clock observes.
struct Counter(AtomicU64);

impl SampleCounterSource for Counter {
    fn sample_count(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
    fn sample_rate_hz(&self) -> u32 {
        48_000
    }
}

fn ns(ticks: i64) -> Timestamp {
    Timestamp::from_parts(ticks, 1, 1_000_000_000).must()
}

fn cue_from(t0: i64, t1: i64) -> CompositedCue {
    CompositedCue::new(
        TimeSpan {
            start: Stamp::Present(ns(t0)),
            end: Stamp::Present(ns(t1)),
        },
        CueBitmap::solid(16, 16, Rgba::WHITE).must(),
        CueLayout::BOTTOM_CENTER,
        0,
    )
    .must()
}

/// Compose `cues` with the clock advanced to `samples` over a FRESH
/// video frame (the presentation loop repaints the plane; a stale
/// frame would keep old cue pixels) and report how many drew.
fn drawn_at(
    compositor: &mut SubtitleCompositor,
    counter: &Counter,
    clock: &MediaClock,
    cues: &[CompositedCue],
    samples: u64,
    target: &mut Rgba8Surface,
) -> usize {
    counter.0.store(samples, Ordering::Relaxed);
    target.fill(Rgba::BLACK);
    compositor.compose(cues, clock.now(), target).must()
}

#[test]
fn cue_duration_is_sample_exact_on_the_real_clock() {
    // The clock: audio master, 48 kHz, anchored at 0. One second of
    // media time = exactly 48_000 samples.
    let counter = Arc::new(Counter(AtomicU64::new(0)));
    let clock = MediaClock::audio_master(counter.clone(), ns(0), Rate::ONE).must();
    let cue = cue_from(1_000_000_000, 2_000_000_000); // [1s, 2s)

    let mut target = Rgba8Surface::new((64, 64), Rgba::BLACK).must();
    let mut compositor = SubtitleCompositor::new();
    let cues = [cue];

    // The sample BEFORE the start: 47_999 samples = 999_979_166.6 ns
    // — one sample short of 1 s: the cue is OFF.
    assert_eq!(
        drawn_at(
            &mut compositor,
            &counter,
            &clock,
            &cues,
            47_999,
            &mut target
        ),
        0
    );
    assert_eq!(target.pixel(32, 50).must(), Rgba::BLACK);
    // Exactly 48_000 samples = 1 s: ON (inclusive start).
    assert_eq!(
        drawn_at(
            &mut compositor,
            &counter,
            &clock,
            &cues,
            48_000,
            &mut target
        ),
        1
    );
    assert_eq!(target.pixel(32, 50).must(), Rgba::WHITE);
    // The interior midpoint (72_000 samples = 1.5 s): ON — checked
    // BEFORE the end boundary because the real clock is monotonic
    // and holds its position across a counter regression (doc 03
    // §4.7: never backwards).
    assert_eq!(
        drawn_at(
            &mut compositor,
            &counter,
            &clock,
            &cues,
            72_000,
            &mut target
        ),
        1
    );
    // The last active sample: 95_999 = 1_999_979_166.6 ns < 2 s: ON.
    assert_eq!(
        drawn_at(
            &mut compositor,
            &counter,
            &clock,
            &cues,
            95_999,
            &mut target
        ),
        1
    );
    // Exactly 96_000 samples = 2 s: OFF (exclusive end).
    assert_eq!(
        drawn_at(
            &mut compositor,
            &counter,
            &clock,
            &cues,
            96_000,
            &mut target
        ),
        0
    );
    assert_eq!(target.pixel(32, 50).must(), Rgba::BLACK);
}

#[test]
fn adjacent_cues_hand_over_without_gap_or_double_draw() {
    // [1s, 2s) then [2s, 3s): at exactly 2 s the first is gone and
    // the second is on — one clock, one swap instant.
    let counter = Arc::new(Counter(AtomicU64::new(0)));
    let clock = MediaClock::audio_master(counter.clone(), ns(0), Rate::ONE).must();
    let first = cue_from(1_000_000_000, 2_000_000_000);
    let second = cue_from(2_000_000_000, 3_000_000_000);
    let cues = [first, second];

    let mut target = Rgba8Surface::new((64, 64), Rgba::BLACK).must();
    let mut compositor = SubtitleCompositor::new();
    let mut at = |samples: u64| -> usize {
        counter.0.store(samples, Ordering::Relaxed);
        compositor.compose(&cues, clock.now(), &mut target).must()
    };
    // NOTE: `at` borrows compositor/target mutably for its lifetime;
    // this test only reads counts.
    assert_eq!(at(48_000), 1); // first on, second not yet
    assert_eq!(at(95_999), 1);
    assert_eq!(at(96_000), 1); // the handover: second on, first off
    assert_eq!(at(143_999), 1);
    assert_eq!(at(144_000), 0); // both done
}

#[test]
fn cue_duration_is_nanosecond_exact_through_compose() {
    // The same half-open rule at nanosecond granularity, driven by
    // direct clock-vocabulary timestamps (the video scheduler's
    // frame-PTS comparison shape).
    let cue = cue_from(1_000_000_000, 2_000_000_000);
    let mut target = Rgba8Surface::new((64, 64), Rgba::BLACK).must();
    let mut compositor = SubtitleCompositor::new();
    for (now, expect) in [
        (999_999_999i64, 0usize),
        (1_000_000_000, 1),
        (1_000_000_001, 1),
        (1_999_999_999, 1),
        (2_000_000_000, 0),
        (2_000_000_001, 0),
    ] {
        // A fresh video frame per presentation, like the render loop.
        target.fill(Rgba::BLACK);
        let drawn = compositor
            .compose(std::slice::from_ref(&cue), ns(now), &mut target)
            .must();
        assert_eq!(drawn, expect, "at now = {now} ns");
        let lit = target.pixel(32, 50).must() == Rgba::WHITE;
        assert_eq!(lit, expect == 1, "pixel at now = {now} ns");
    }
}

#[test]
fn zero_duration_and_open_ended_cues() {
    // A zero-duration cue never displays (an interval that contains
    // no sample); an open-ended cue displays from its start onward.
    let zero = cue_from(1_000_000_000, 1_000_000_000);
    let open = CompositedCue::new(
        TimeSpan {
            start: Stamp::Present(ns(1_000_000_000)),
            end: Stamp::Missing,
        },
        CueBitmap::solid(16, 16, Rgba::WHITE).must(),
        CueLayout::BOTTOM_CENTER,
        0,
    )
    .must();
    let mut target = Rgba8Surface::new((64, 64), Rgba::BLACK).must();
    let mut compositor = SubtitleCompositor::new();
    let cues = [zero, open];
    let drawn = compositor
        .compose(&cues, ns(1_000_000_000), &mut target)
        .must();
    assert_eq!(drawn, 1); // the open cue only
    let drawn = compositor
        .compose(&cues, ns(4_000_000_000), &mut target)
        .must();
    assert_eq!(drawn, 1); // and forever after
}
