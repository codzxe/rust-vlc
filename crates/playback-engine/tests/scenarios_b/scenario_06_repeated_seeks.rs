//! Scenario 6 — **repeated seeks** (doc 10 §6.4 row 6): 100 random
//! targets, no waits — no stale output ever; bounded memory; each
//! completes or typed-fails.
//!
//! A deterministic LCG (no external RNG, byte-stable reruns) drives
//! 100 random absolute targets across the parent's 2 s span. Every
//! seek must answer its terminal (completed within the fixture's
//! tolerance, or the typed failure — none is expected on this
//! seekable in-range parent). No stale output ever: after every
//! completion, the frames that follow the landing point never carry
//! a stamp behind it (the in-flight one-quantum grace aside).
//! Bounded memory: after a warm-up of the seek paths, live bytes
//! return to the post-warm-up level at the end of the storm.

use commands::catalog::types::{OpenOptions, SeekTarget, SourceSpec, Timestamp};
use commands::envelope::CommandOutcome;
use commands::{Command, PlaybackState};
use testkit::Must;
use testkit::alloc_count::serialize_tests;

use crate::host::{PARENT, ScenarioHost, count_events, dispatch};
use crate::alloc_state;

/// The parent's recorded seek tolerance (its expectation TOML).
const TOLERANCE_US: i64 = 100_000;
/// The storm's size (doc 10 §6.4: 100 random targets).
const STORM: usize = 100;

/// A tiny deterministic LCG (Park–Miller) — no external RNG
/// dependency, byte-stable across runs and machines.
struct Lcg(u64);
impl Lcg {
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }
    /// One target in `[0, duration_us)`.
    fn target_us(&mut self, duration_us: i64) -> i64 {
        (self.next_u64() % duration_us.unsigned_abs() as u64) as i64
    }
}

#[test]
fn hundred_random_seeks_complete_without_stale_output_or_growth() {
    let _serial = serialize_tests();
    let mut host = ScenarioHost::new();
    let path = testkit::fixture_path(PARENT).must();
    let response = dispatch(
        &mut host,
        Command::SessionOpen {
            source: SourceSpec::File { path },
            options: OpenOptions::default(),
        },
    );
    assert!(
        matches!(&response.result, Ok(CommandOutcome::Accepted { .. })),
        "the parent opens: {:?}",
        response.result
    );
    host.settle();

    // Warm the seek paths (the first seeks' one-time machinery —
    // the seek pool, the locate caches) so the storm measures
    // steady state, not warm-up.
    for target_us in [200_000i64, 1_800_000, 900_000] {
        let warm = dispatch(
            &mut host,
            Command::PlaybackSeek {
                target: SeekTarget::Absolute {
                    time: Timestamp(target_us),
                },
            },
        );
        assert!(warm.result.is_ok(), "the warm-up seek: {:?}", warm.result);
    }
    // The seek-completed counter is SESSION-CUMULATIVE and every
    // warm-up seek already reached its terminal: this host's
    // `dispatch_seek` pumps to the seek terminal BEFORE answering
    // (host.rs `dispatch_seek` — the first terminal after submission
    // is the live seek's, doc 05 §4.2), so the storm's baseline is
    // the post-warm-up count, not zero.
    let storm_baseline = count_events(&host, "timing.seek-completed");
    assert_eq!(
        count_events(&host, "timing.seek-failed"),
        0,
        "the warm-up seeks never typed-fail either"
    );
    let baseline_live = alloc_state().snapshot().live_bytes;

    // The storm: 100 deterministic random targets, no waits between
    // submissions (each dispatch awaits its own terminal — the
    // synchronous host's "no waits" is back-to-back dispatches).
    let mut rng = Lcg(0x5EED_0000_00C0_FFEE);
    let mut completed = storm_baseline;
    let mut landed_deltas: Vec<i64> = Vec::with_capacity(STORM);
    for _ in 0..STORM {
        let target_us = rng.target_us(2_000_000);
        let response = dispatch(
            &mut host,
            Command::PlaybackSeek {
                target: SeekTarget::Absolute {
                    time: Timestamp(target_us),
                },
            },
        );
        assert!(
            matches!(&response.result, Ok(CommandOutcome::Accepted { .. })),
            "seek to {target_us} µs accepts: {:?}",
            response.result
        );
        let completed_now = count_events(&host, "timing.seek-completed");
        let failed_now = count_events(&host, "timing.seek-failed");
        assert_eq!(
            failed_now,
            0,
            "an in-range seek on the seekable parent never typed-fails"
        );
        assert_eq!(
            completed_now,
            completed + 1,
            "each seek completes before the next submits"
        );
        completed = completed_now;
        let landed = host
            .events()
            .iter()
            .rev()
            .find_map(|event| {
                if let commands::EventKind::TimingSeekCompleted { position, .. } = &event.kind {
                    Some(position.0)
                } else {
                    None
                }
            })
            .must();
        landed_deltas.push((landed - target_us).abs());

        // No stale output ever: the frames delivered after this
        // completion carry stamps at or beyond the landing point
        // (one quantum of in-flight grace: 40 ms).
        let ring = host.sink().must().frame_ring();
        let stale = ring
            .iter()
            .filter(|(_, _, pts)| *pts < landed * 1_000 - 40_000_000)
            .count();
        // The ring holds the storm's recent window; every entry in
        // it was delivered under SOME post-landing timeline — an
        // entry behind the CURRENT landing by more than the grace
        // is a stale frame.
        assert_eq!(
            stale, 0,
            "no stale frames behind the landing {landed} µs"
        );
    }
    assert_eq!(
        completed,
        storm_baseline + STORM,
        "all {STORM} storm seeks completed (on top of the warm-up's {storm_baseline})"
    );
    assert!(
        landed_deltas.iter().all(|delta| *delta <= TOLERANCE_US),
        "every landing within tolerance (worst {:?})",
        landed_deltas.iter().max()
    );

    // Bounded memory: the storm returns live bytes to the baseline.
    let after_live = alloc_state().snapshot().live_bytes;
    assert!(
        after_live <= baseline_live,
        "the seek storm is memory-bounded (live {baseline_live} → {after_live})"
    );

    // The media still ends honestly after the storm.
    let (pumps, terminal) = host.run_until_terminal(1024);
    assert_eq!(
        terminal,
        PlaybackState::Ended,
        "the parent ends after the storm ({pumps} pumps)"
    );
    let close = dispatch(&mut host, Command::SessionClose);
    assert!(close.result.is_ok(), "closes: {:?}", close.result);
}
