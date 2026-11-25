//! FX3-A06: the media-clock truth-source edge (doc 03 §4.9 —
//! `sample_counter`, "the clock's truth source", implemented by the
//! open device session; the production `SampleCounterSource` lives in
//! crates/audio-output's counter.rs and reaches the clock through
//! `AudioBackend::clock_source()`).
//!
//! The crate-side half of that edge, proven here on plain injected
//! sources (media-clock cannot depend on audio-output — L2 sits below
//! L4; the production adapter itself is exercised by audio-output's
//! `tests/clock_truth.rs` over a driven null session):
//!
//! 1. a counter source driving the clock advances media time
//!    **exactly** with samples — the position is a pure function of
//!    the counter;
//! 2. a stalled counter stalls the clock — the documented tolerance
//!    for a stall is zero (the ±1000 ppm drift budget belongs to the
//!    absorbers, never to the clock's read path);
//! 3. switching the counter source at runtime
//!    (`MediaClock::adopt_sample_source`, the FX4 re-open seam) is
//!    clean: position carried, anchor re-taken in the new counter's
//!    own domain, epoch bumped exactly once, the dead source goes
//!    dark, the wired audio slot is refreshed, and an unreadable new
//!    source parks until its first usable reading.
//!
//! Deterministic atomics only — no sleeps, no wall clock (doc 03 §8).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use media_clock::{
    AdoptSourceError, ClockBackends, ClockMaster, MasterSwitch, MasterTarget, MediaClock,
    MonotonicSource, NANO_BASE, PlaybackClock, SampleCounterSource,
};
use media_types::time::{Rate, Timestamp};
use testkit::Must;

/// A deterministic fake sample counter: an atomic count plus an
/// atomic rate (the rate can sit at 0 to model a device that has not
/// finished opening — media-clock treats that reading as unusable).
struct FakeCounter {
    sample: AtomicU64,
    rate_hz: AtomicU64,
}

impl FakeCounter {
    fn new(rate_hz: u32) -> FakeCounter {
        FakeCounter {
            sample: AtomicU64::new(0),
            rate_hz: AtomicU64::new(u64::from(rate_hz)),
        }
    }

    /// A counter starting mid-domain at `base` — the shape of a fresh
    /// device session's counter (its own domain, not a continuation
    /// of the previous device's count).
    fn with_base(base: u64, rate_hz: u32) -> FakeCounter {
        FakeCounter {
            sample: AtomicU64::new(base),
            rate_hz: AtomicU64::new(u64::from(rate_hz)),
        }
    }

    fn advance(&self, samples: u64) {
        let _ = self.sample.fetch_add(samples, Ordering::Relaxed);
    }

    fn count(&self) -> u64 {
        self.sample.load(Ordering::Relaxed)
    }
}

impl SampleCounterSource for FakeCounter {
    fn sample_count(&self) -> u64 {
        self.count()
    }

    fn sample_rate_hz(&self) -> u32 {
        u32::try_from(self.rate_hz.load(Ordering::Relaxed)).unwrap_or(0)
    }
}

/// A deterministic fake monotonic source (the audio-less fallback's
/// injected wall).
struct FakeMonotonic(AtomicU64);

impl MonotonicSource for FakeMonotonic {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

fn ts(ns: i64) -> Timestamp {
    Timestamp::new(ns, NANO_BASE)
}

// ---- 1. the counter drives media time exactly ----------------------------

#[test]
fn fx3_a06_counter_advances_media_time_exactly_with_samples() {
    // 48 kHz: every 2 400-instant chunk is exactly 50 ms of media
    // time — the position after each chunk is exact, never rounded.
    let counter = Arc::new(FakeCounter::new(48_000));
    let clock = MediaClock::audio_master(counter.clone(), ts(0), Rate::ONE)
        .unwrap_or_else(|e| unreachable!("{e}"));
    let mut expected = 0i64;
    for _ in 0..20 {
        counter.advance(2_400);
        expected += 50_000_000;
        let now = clock.now();
        assert_eq!(now.ticks, expected, "chunk should end at {expected} ns");
        assert_eq!(clock.try_now().must(), now);
    }
    assert_eq!(counter.count(), 48_000);
    assert_eq!(clock.now().ticks, 1_000_000_000);

    // 44.1 kHz: whole seconds stay exact (44 100 instants at
    // 44 100 Hz — the odd-rate production shape).
    let odd = Arc::new(FakeCounter::new(44_100));
    let clock = MediaClock::audio_master(odd.clone(), ts(0), Rate::ONE)
        .unwrap_or_else(|e| unreachable!("{e}"));
    for second in 1..=3i64 {
        odd.advance(44_100);
        assert_eq!(clock.now().ticks, second * 1_000_000_000);
    }

    // A rate ≠ 1 keeps the exactness: 5/4 over four counter-seconds
    // is five media-seconds, to the nanosecond.
    let counter = Arc::new(FakeCounter::new(48_000));
    let clock = MediaClock::audio_master(
        counter.clone(),
        ts(0),
        Rate::new(5, 4).unwrap_or_else(|e| unreachable!("{e}")),
    )
    .unwrap_or_else(|e| unreachable!("{e}"));
    counter.advance(4 * 48_000);
    assert_eq!(clock.now().ticks, 5_000_000_000);
}

// ---- 2. a stalled counter stalls the clock -------------------------------

#[test]
fn fx3_a06_stalled_counter_stalls_the_clock_without_drift() {
    let counter = Arc::new(FakeCounter::new(48_000));
    let clock = MediaClock::audio_master(counter.clone(), ts(0), Rate::ONE)
        .unwrap_or_else(|e| unreachable!("{e}"));
    counter.advance(72_000); // 1.5 s consumed
    let stalled_at = clock.now();
    assert_eq!(stalled_at.ticks, 1_500_000_000);

    // The device stalls mid-session (nothing renders; the counter is
    // frozen). The audio-master position is a pure function of the
    // counter — no wall term exists in the extrapolation — so the
    // documented tolerance for a stall is ZERO: 64 consecutive reads
    // are exactly equal, and the typed read agrees. (The ±1000 ppm
    // drift budget is the absorbers' limiter — see drift.rs — never
    // a license for the clock to free-run.)
    let epoch = clock.snapshot().anchor_epoch;
    for _ in 0..64 {
        assert_eq!(clock.now(), stalled_at);
    }
    assert_eq!(clock.try_now().must(), stalled_at);
    // Nothing re-anchored by itself either: the anchor still points at
    // the construction anchor (sample 0), in the same epoch.
    assert_eq!(clock.anchor().at_sample, Some(0));
    assert_eq!(clock.snapshot().anchor_epoch, epoch);

    // Resumption is exact: the counter's next 48 000 instants move
    // the clock by exactly one second from the stalled position.
    counter.advance(48_000);
    assert_eq!(clock.now().ticks, 2_500_000_000);
}

// ---- 3. runtime source switching is clean --------------------------------

#[test]
fn fx3_a06_runtime_source_switch_is_clean() {
    // Session A at 48 kHz plays 2 s...
    let a = Arc::new(FakeCounter::new(48_000));
    let clock = MediaClock::audio_master(a.clone(), ts(0), Rate::ONE)
        .unwrap_or_else(|e| unreachable!("{e}"));
    a.advance(96_000);
    assert_eq!(clock.now().ticks, 2_000_000_000);
    let epoch = clock.snapshot().anchor_epoch;

    // ...the device dies mid-session (its counter regresses to a
    // fresh count — the documented discontinuity: the clock holds,
    // the typed read refuses)...
    a.sample.store(0, Ordering::Relaxed);
    let held = clock.now();
    assert_eq!(held.ticks, 2_000_000_000);
    assert!(clock.try_now().is_err());

    // ...and the device is re-opened as session B at 44.1 kHz whose
    // counter starts mid-domain (735 instants). The swap is clean:
    // the position carries exactly (the hold is cured without a
    // jump), the master is still the audio output, the anchor epoch
    // bumps exactly once, and the anchor is re-taken in B's own
    // counter domain.
    let b = Arc::new(FakeCounter::with_base(735, 44_100));
    let receipt = clock.adopt_sample_source(b.clone()).must();
    assert_eq!(receipt.position.ticks, 2_000_000_000);
    assert_eq!(receipt.master, ClockMaster::AudioOutput);
    assert_eq!(receipt.anchor_epoch, epoch + 1);
    assert_eq!(clock.now().ticks, 2_000_000_000);
    assert!(clock.try_now().is_ok());
    assert_eq!(clock.anchor().at_sample, Some(735));

    // B's consumption advances media time exactly (one second of B's
    // instants is one second of media time — a different counter
    // domain, the same truth)...
    b.advance(44_100);
    assert_eq!(clock.now().ticks, 3_000_000_000);
    // ...and A is dark: its count no longer moves the clock at all.
    a.advance(96_000);
    assert_eq!(clock.now().ticks, 3_000_000_000);
    assert!(clock.try_now().is_ok());

    // A second swap is equally clean (B -> C at 48 kHz from zero).
    let c = Arc::new(FakeCounter::new(48_000));
    let receipt = clock.adopt_sample_source(c.clone()).must();
    assert_eq!(receipt.position.ticks, 3_000_000_000);
    assert_eq!(receipt.anchor_epoch, epoch + 2);
    c.advance(48_000);
    assert_eq!(clock.now().ticks, 4_000_000_000);

    // The seam installs onto the audio master only: a non-audio clock
    // refuses typed, unchanged (installing a sample counter onto the
    // wall master would be a silent master promotion).
    let wall = Arc::new(FakeMonotonic(AtomicU64::new(0)));
    let mono = MediaClock::system_monotonic(wall.clone(), ts(0), Rate::ONE)
        .unwrap_or_else(|e| unreachable!("{e}"));
    assert_eq!(
        mono.adopt_sample_source(b.clone()),
        Err(AdoptSourceError::NotAudioMaster {
            current: ClockMaster::SystemMonotonic
        })
    );
    let _ = wall.0.fetch_add(1_000_000_000, Ordering::Relaxed);
    assert_eq!(mono.now().ticks, 1_000_000_000);
}

// ---- 4. the wired audio slot follows the adopted source ------------------

#[test]
fn fx3_a06_adopt_refreshes_the_wired_audio_slot() {
    // The multi-backend clock: after adopting a new session's
    // counter, the wired audio slot targets IT — a switch away and
    // back runs on the new source, and the old one is dark.
    let wall = Arc::new(FakeMonotonic(AtomicU64::new(0)));
    let first = Arc::new(FakeCounter::new(48_000));
    let clock = MediaClock::with_backends(
        ClockBackends::new(wall).audio(first.clone()).video(true),
        ts(0),
        Rate::ONE,
    )
    .unwrap_or_else(|e| unreachable!("{e}"));
    first.advance(48_000);
    assert_eq!(clock.now().ticks, 1_000_000_000);

    let second = Arc::new(FakeCounter::with_base(1_000, 44_100));
    let receipt = clock.adopt_sample_source(second.clone()).must();
    assert_eq!(receipt.position.ticks, 1_000_000_000);
    assert_eq!(receipt.master, ClockMaster::AudioOutput);

    // Away (video master, wall-driven) and back: the switch-back must
    // read the NEW session's counter, not the dead `first` one the
    // construction-time wiring held.
    assert_eq!(
        clock.switch_master(MasterTarget::Video),
        Ok(MasterSwitch::Switched {
            from: ClockMaster::AudioOutput,
            to: ClockMaster::VideoOutput
        })
    );
    assert_eq!(
        clock.switch_master(MasterTarget::Audio),
        Ok(MasterSwitch::Switched {
            from: ClockMaster::VideoOutput,
            to: ClockMaster::AudioOutput
        })
    );
    second.advance(44_100);
    assert_eq!(clock.now().ticks, 2_000_000_000);
    first.advance(96_000); // dark: the old session cannot move the clock
    assert_eq!(clock.now().ticks, 2_000_000_000);
}

// ---- 5. an unreadable adopted source parks until its first reading ------

#[test]
fn fx3_a06_unreadable_adopted_source_parks_until_first_reading() {
    // The re-opened session's device has not finished opening (its
    // rate still reads 0 — unusable): the swap parks the clock at the
    // carried position, the typed read reports the discontinuity, and
    // the first usable reading becomes the fresh anchor — no jump, no
    // false discontinuity against the old domain.
    let a = Arc::new(FakeCounter::new(48_000));
    let clock = MediaClock::audio_master(a.clone(), ts(0), Rate::ONE)
        .unwrap_or_else(|e| unreachable!("{e}"));
    a.advance(48_000);
    assert_eq!(clock.now().ticks, 1_000_000_000);

    let b = Arc::new(FakeCounter::new(0)); // device not open yet
    let receipt = clock.adopt_sample_source(b.clone()).must();
    assert_eq!(receipt.position.ticks, 1_000_000_000);
    assert_eq!(clock.now().ticks, 1_000_000_000);
    assert!(clock.try_now().is_err());

    // The device opens at 44.1 kHz: the first usable reading anchors
    // and the position still has not moved.
    b.rate_hz.store(44_100, Ordering::Relaxed);
    assert_eq!(clock.now().ticks, 1_000_000_000);
    assert!(clock.try_now().is_ok());
    // Consumption from the new domain resumes exactly.
    b.advance(44_100);
    assert_eq!(clock.now().ticks, 2_000_000_000);
}
