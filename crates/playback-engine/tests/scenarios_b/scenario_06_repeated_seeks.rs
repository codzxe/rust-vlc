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

use crate::alloc_state;
use crate::host::{PARENT, ScenarioHost, count_events, dispatch};

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
        (self.next_u64() % duration_us.unsigned_abs()) as i64
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
    let events_at_baseline = host.events().len();

    // The storm: 100 deterministic random targets, no waits between
    // submissions (each dispatch awaits its own terminal — the
    // synchronous host's "no waits" is back-to-back dispatches).
    let mut rng = Lcg(0x5EED_0000_00C0_FFEE);
    let mut completed = storm_baseline;
    let mut landed_deltas: Vec<i64> = Vec::with_capacity(STORM);
    for _ in 0..STORM {
        let target_us = rng.target_us(2_000_000);
        // The per-seek delivery window: the ring is a bounded HISTORY
        // across the whole storm (cap 1024, host.rs `RING_CAP`), so
        // "no stale output" is a per-seek invariant over the frames
        // THIS seek delivered — the green scenario 4 shape
        // (`ring[ring_before..]`, scenario_04_seek.rs:30/69-75), not a
        // whole-ring check: an earlier landing's frames sit behind a
        // later landing by construction (a backward storm target is
        // legitimate), they are not stale output.
        let ring_before = host.sink().must().frame_ring().len();
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
            failed_now, 0,
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

        // No stale output ever: the frames delivered under THIS seek
        // (its own pumps — submission through terminal) carry stamps
        // at or beyond the landing point, one quantum of in-flight
        // grace aside (40 ms) — scenario_04_seek.rs:66-75's oracle.
        let ring = host.sink().must().frame_ring();
        let window = &ring[ring_before.min(ring.len())..];
        let stale = window
            .iter()
            .filter(|(_, _, pts)| *pts < landed * 1_000 - 40_000_000)
            .count();
        assert_eq!(
            stale,
            0,
            "no stale frames behind the landing {landed} µs (window of {})",
            window.len()
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

    // Bounded memory (doc 10 §6.4 row 6): every live byte the storm
    // added is accountable to the HOST's own session-scoped recording
    // — the wire-event log (a Vec: amortized capacity ≤ 2× its len)
    // and the capped frame ring (1024 entries × 12 B, host.rs
    // `RING_CAP`) — never the engine's seek machinery.
    let after_live = alloc_state().snapshot().live_bytes;
    let events_added = host.events().len().saturating_sub(events_at_baseline);
    let recording_bound = (2 * events_added * std::mem::size_of::<commands::WireEvent>()
        + 1024 * 12
        + landed_deltas.capacity() * std::mem::size_of::<i64>()) as isize;
    assert!(
        after_live <= baseline_live + recording_bound,
        "the storm's live growth ({}) is the host's own recording, \
         not the engine (bound {recording_bound}: {events_added} events, \
         the capped ring, the deltas scratch)",
        after_live - baseline_live
    );

    // The no-leak oracle: a trailing probe of further seeks adds
    // (almost) nothing — the engine's seek path is allocation-steady
    // once the one-time machinery is warm. Measured: 5 seeks ≈ tens
    // of bytes (the log's spare capacity absorbs the probe's
    // events; the deterministic seed/fixture/pump sequence keeps it
    // clear of a Vec re-growth). A per-seek engine leak (the row's
    // actual target) cannot hide here.
    let probe_before = alloc_state().snapshot().live_bytes;
    for target_us in [1_234_567i64, 500_000, 1_700_000, 300_000, 900_000] {
        let probe = dispatch(
            &mut host,
            Command::PlaybackSeek {
                target: SeekTarget::Absolute {
                    time: Timestamp(target_us),
                },
            },
        );
        assert!(
            probe.result.is_ok(),
            "the probe seek to {target_us} µs accepts: {:?}",
            probe.result
        );
    }
    let probe_after = alloc_state().snapshot().live_bytes;
    assert!(
        probe_after <= probe_before + 4_096,
        "the seek path is allocation-steady (the 5-seek probe grew {})",
        probe_after - probe_before
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
