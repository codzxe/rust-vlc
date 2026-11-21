# AUDIT: crates/media-clock/src/backends.rs#part4 (lines 1580-2117)
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-21 |
| Lines / functions | 538 (chunk of 2118) / 20 tests (video-steering, Ended-latch/seek-back, i64-edge, event-ring, vocabulary groups) |
| Chunk boundaries | part4 ≈ 1588-2117; the boundary cuts through the test starting at 1580 (`video_master_snaps_within_the_bound_and_interpolates_between`) — audited in full from 1580. Production code (EventRing, plan_fallback, with_backends, on_master_lost, switch_master, observe_video, mark_ended, seek_back, switch_locked) is parts 1-3 (other agents); read here as context for test honesty. |
| File verdict (chunk) | REAL — adversarial, hand-verifiable tests, all executed in baseline |
| Reachable from shipped entry point? | N/A for a test chunk; the code under test ships: media-clock is a dependency of playback-engine/audio-output in the `lumen` CLI graph (W01 audits + rg imports). All 20 chunk tests passed: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt (media_clock lib `test result: ok. 92 passed`; every `backends::tests::<chunk name> ... ok` verified individually by grep — 20/20). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | grep per-test-name in orchrun baseline log (20/20 ok, exact strings) · Read of production halves 505-1013 (EventRing/plan_fallback/with_backends/on_master_lost/switch_master/observe_video/mark_ended/seek_back/switch_locked) · hand-simulation of every chunk assertion against the implementation (see §5) · sed -n quotes (verified) |

## 1. One-paragraph verdict
This is the hard half of the backend test suite and it is honest: the
video-steer tests hand-verify snap/jump classification including the exact `<=`
boundary and the monotone-clamp "held until the wall catches up" behavior; the
Ended-latch tests close the card's named traps (resume-must-not-free-run,
reanchor-keeps-latch, seek-back-re-arms-paused) with exact position assertions
I re-derived by hand from the implementation; the edge tests drive i64::MIN/MAX
anchors, a u64::MAX-saturated wall, and a ~4.3e9× rate through switches without
a single panic; the event-ring tests prove bounded overflow both directly and
through the public API. Two hygiene nits only: one test's comment promises an
order check its body does not perform, and the awaiting-anchor × resume
interleaving (the S3 filed against clock.rs#part2) has no coverage here.

## 2. What actually works (verified)
— Snap/jump boundary: `video_steer_errors_at_the_i64_bounds_are_i128_safe` (1953-1984) pins `error_ns.abs() <= bound` exactly at the bound (Snapped at +1_000_000_000 with snap bound 1_000_000_000) and the ~2^64-ns error at the opposite i64 extreme as a Jump — hand-checked against observe_video's i128 math (backends.rs:829-842).
— Interpolation + clamp: `video_master_snaps...` (1580-1617) — I re-derived every step (10 ms wall → snap at 30 ms; +100 ms → 130 ms; behind-frame snap → held at 130 ms; +20 ms caught up; +5 ms flows at 135 ms) against extrapolate/now_locked: exact.
— Wall-regression cure: `wall_regression_holds_and_an_in_bound_frame_cures` (1698-1736) — regression → hold + `try_now` err; in-bound frame re-anchors on the *current* (regressed) wall; subsequent forward jump counts as elapsed (1.03 s + 850 ms = 1.88 s) — all three steps hand-verified.
— Ended latch traps: `ended_latch_freezes_and_seek_back_re_arms_paused_not_free_running` (1741-1786), `reanchor_while_ended_moves_the_freeze_but_keeps_the_latch` (1869-1884), `play_from_ended_restarts_through_seek_back_zero` (1889-1903) — each asserts state machine + exact positions; resume-under-latch no-op verified against MediaClock::resume's guard (clock.rs:560).
— Audio-master seek_back drain case: `seek_back_on_audio_master_resumes_without_jump_after_drain` (1789-1816) — counter at 72 000 at re-arm; resume anchors there; +48 000 → 1.25 s exactly (hand-computed).
— i64 edge matrix: `switches_across_i64_position_edges_carry_saturated` (1911-1947) — 7 edges incl. i64::MIN; position carried exactly, event position representable, advance monotone, saturation at i64::MAX.
— Bounded ring: `event_ring_overflow_counts_instead_of_growing` (2045-2066) drives the real `EventRing` (lost=1, len=8, FIFO drain then None); `switch_storm_without_drain_surfaces_the_lost_count` (2071-2096) reproduces it through the public switch API.
— Baseline: all 20 tests individually grepped as `... ok` in orchrun-test-workspace-nofailfast.log.txt (media_clock lib 92/92; zero media-clock failures among the 7 workspace failures).

## 3. Findings

### F-001 [S4] — comment claims an order/identity check the test does not perform
Where: crates/media-clock/src/backends.rs:2090-2095
Code:
```rust
        // The drained events are the first 8, in order.
        let mut drained = 0;
        while clock.take_master_change().is_some() {
            drained += 1;
        }
        assert_eq!(drained, MASTER_EVENT_CAPACITY);
```
What is wrong: The comment asserts "the first 8, in order", but the loop
discards every event (`is_some()` only) — the test proves a count, not which
events survived or their order. A ring that kept the *newest* 8, or drained in
reverse, passes. (The direct EventRing test at 2045-2066 pins FIFO with
identical events, so real coverage exists — but this public-API test's comment
overstates its own body, and the alternating targets (Video/Audio at
2082-2086) would have made an order check trivial: from/to pairs alternate.)
Why it matters: overstated test comments are how regressions hide — a future
rewrite of the ring's overflow policy survives this test with its comment
intact.
What correct looks like: collect the drained events and assert the
from/to alternation (event i has from = i%2==0 ? AudioOutput : VideoOutput),
or delete the "in order" claim.
Confidence: HIGH — Verification: read (loop body discards; alternation
derivable from 2081-2088).

### F-002 [S4] — no coverage for the awaiting-anchor × resume/reanchor interleaving
Where: crates/media-clock/src/backends.rs:906-913 (the latch-setting branch, exercised only by part-3's test at 1543) + this chunk's seek_back tests (1789-1850), all of which use usable readings
Code:
```rust
        let reading = read_source(&core.source);
        if matches!(reading, crate::clock::Reading::Usable { .. }) {
            adopt_anchor(&mut core, &reading);
        } else {
            // The source cannot be read right now: hold at the
            // target and adopt its first usable reading (fresh
            // baseline — no false discontinuity against the stale
            // anchor of the pre-end master).
            core.awaiting_anchor = true;
        }
```
What is wrong: `seek_back` (and `switch_locked`, backends.rs:993) set
`awaiting_anchor = true` only when the reading is unusable — and no test in
this chunk (or part 3's `switch_onto_unreadable_audio_awaits_first_usable_reading`,
which stops at the first usable read) then calls `reanchor`/`resume` before the
next `now()`. That is exactly the interleaving where clock.rs#part2 F-001 (S3:
double-adopt silently swallows elapsed progress) lives. The chunk's
seek_back tests all have readable sources, so the hole is untested from both
ends.
Why it matters: a suspected S3 has zero reproducing test; any future "fix"
cannot be validated against the suite, and the defect itself cannot be
refuted by evidence.
What correct looks like: one test — switch onto a zero-rate (unreadable)
audio backend, let the rate recover, call `clock.resume()`, advance the
counter, and assert the exact post-resume position (the swallow shows up as a
shortfall).
Confidence: HIGH — Verification: read (all seek_back/switch/awaiting test
sites enumerated across parts 3-4).
## 4. Per-function table (chunk — all tests)
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| video_master_snaps_within_the_bound_and_interpolates_between | 1580-1617 | REAL | — | every step hand-re-derived; pins snap + interpolation + clamp-hold |
| out_of_bound_frame_is_a_typed_jump_with_epoch_bump | 1620-1646 | REAL | — | epoch+1, forward and backward jumps |
| steering_is_rejected_outside_video_master | 1649-1667 | REAL | — | NotMaster + anchor untouched (counter still rules) |
| steering_is_rejected_while_paused_and_ended | 1670-1679 | REAL | — | Paused/Ended steers |
| unrepresentable_frame_pts_is_a_typed_overflow | 1682-1692 | REAL | — | typed Overflow, clock unchanged |
| wall_regression_holds_and_an_in_bound_frame_cures | 1698-1736 | REAL | — | hold + cure + forward-jump-as-elapsed, hand-verified |
| ended_latch_freezes_and_seek_back_re_arms_paused_not_free_running | 1741-1786 | REAL | — | the card's trap matrix, exact positions |
| seek_back_on_audio_master_resumes_without_jump_after_drain | 1789-1816 | REAL | — | dirty-drain world, resume anchors at current counter (1.25 s hand-checked) |
| seek_back_refuses_when_not_ended_leaving_the_clock_alone | 1819-1834 | REAL | — | typed NotEnded payload + untouched clock |
| seek_back_overflow_keeps_the_latch_and_the_frozen_position | 1837-1850 | REAL | — | latch + frozen survive Overflow |
| mark_ended_is_idempotent_and_reports_the_same_freeze | 1853-1863 | REAL | — | full-snapshot equality |
| reanchor_while_ended_moves_the_freeze_but_keeps_the_latch | 1869-1884 | REAL | — | reanchor ≠ re-arm |
| play_from_ended_restarts_through_seek_back_zero | 1889-1903 | REAL | — | the doc 05 restart shape |
| switches_across_i64_position_edges_carry_saturated | 1911-1947 | REAL | — | 7-edge matrix incl. i64::MIN; carry exact; saturate at MAX |
| video_steer_errors_at_the_i64_bounds_are_i128_safe | 1953-1984 | REAL | — | ~2^64 error no-panic; exact `<=` boundary pinned |
| saturated_u64_wall_source_is_handled | 1990-2010 | REAL | — | u64::MAX wall: construct/steer/try_now all sound |
| extreme_rate_saturates_rather_than_wrapping_under_switching | 2016-2040 | REAL | — | 4.29e9× rate: saturate + typed Overflow + switch carries |
| event_ring_overflow_counts_instead_of_growing | 2045-2066 | REAL | — | direct ring unit: lost/len/FIFO |
| switch_storm_without_drain_surfaces_the_lost_count | 2071-2096 | REAL | S4 | F-001: count-only drain |
| vocabulary_strings_are_stable | 2101-2116 | REAL | — | wire vocabulary pinned |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| video_master_snaps… | YES | snap classification (error 20 ms vs 50 ms bound), interpolation arithmetic, clamp-hold-then-catch-up — each step exact |
| out_of_bound_frame… | YES | Jumped values ±5e9, epoch+1, post-jump positions |
| steering_is_rejected_outside_video_master | YES | NotMaster variant + counter-derived position (anchor untouched) |
| steering_is_rejected_while_paused_and_ended | YES | Paused/Ended variants |
| unrepresentable_frame_pts… | YES | Overflow + position unchanged |
| wall_regression_holds… | YES | hold + typed err + cure + 1.88 s exact (a wrong cure baseline shifts it) |
| ended_latch_freezes… | YES | freeze/resume-no-op/pause-no-op/re-arm-paused/hold-at-target/resume-continues — 12 exact assertions |
| seek_back_on_audio_master… | YES | 1.25 s exact pins the anchor-at-current-counter semantics |
| seek_back_refuses… | YES | exact NotEnded payload + 2 s untouched |
| seek_back_overflow… | YES | latch + frozen survive |
| mark_ended_is_idempotent… | YES | full ClockSnapshot equality (second call must not re-freeze at the advanced wall) |
| reanchor_while_ended… | YES | state Ended + 9 s frozen + resume no-op |
| play_from_ended… | YES | 0 → resume → 1 s |
| switches_across_i64_edges… | YES | carry-exact + advance + saturate per edge |
| video_steer_errors_at_i64_bounds… | YES | Jumped at ~2^64 error (no panic) + Snapped exactly AT the bound (pins `<=`) |
| saturated_u64_wall… | YES | 123 ns stable across construct/steer/read |
| extreme_rate… | YES | i64::MAX + typed Overflow + FreeRunning demotion carries saturation |
| event_ring_overflow… | YES | lost/len/FIFO-then-None (a LIFO or drop-newest ring fails the drain equality) |
| switch_storm… | PARTIAL | lost-count pinned; drained-event identity/order NOT checked (F-001) |
| vocabulary_strings… | YES | exact strings (a rename fails it) |
Not mocks: FakeCounter/FakeMonotonic are source seams (the crate's injection
doctrine); the subject (MediaClock + backends) is the real implementation.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "In-bound frames (within the snap bound) move the anchor invisibly; out-of-bound frames are adopted as typed discontinuities with an epoch bump" | backends.rs:792-795 (observe_video doc) | REAL | tests 1580-1646 hand-verified against 829-842 |
| "resume() is a no-op under the latch; the only re-arm is seek_back" | backends.rs:848-850 | REAL | tests 1741-1786, 1869-1884 |
| "a seek re-arms the clock paused at the new position — never free-running" | backends.rs:869-873 | REAL | test 1741: wall keeps running, position held at 500 ms |
| "the difference of two i64s always fits (card 3.B2's overflow edge tests pin this at the bounds)" | backends.rs:826-828 | REAL | test 1953-1984 executes the ~2^64-ns error |
| "on overflow instead of growth. All operations are allocation-free" (EventRing) | backends.rs:506-508 | REAL | ring test 2045-2066; fixed [Option;8] array |
| "i128 by construction … the position clamps" (u64::MAX wall) | test doc 1986-1988 | REAL | test 1990-2010 passed in baseline |

## 7. Dead / duplicated / unreachable code
None in this chunk. (Note for parts 1-3 auditors: `WiredBackends::wired`
duplicates a master-kind check also done by `source_for` — cosmetic only,
verified harmless here.)

## 8. Dependencies & flags
Tests use std atomics + testkit::Must; no features; no unsafe; everything
compiles in the default workspace lane (baseline-cited).

## 9. Open questions for the next agent
- Add the awaiting-anchor × resume reproducing test (F-002) — it also serves
  as the regression test for clock.rs#part2 F-001 (S3).
- part-3 auditor: `switch_onto_unreadable_audio_awaits_first_usable_reading`
  (1543) — does it assert the position AFTER the first usable read advanced
  correctly (i.e., no swallowed interval)? If it only checks master/state, the
  swallow class is untested there too.
