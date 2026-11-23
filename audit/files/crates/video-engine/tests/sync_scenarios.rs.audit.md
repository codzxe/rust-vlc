# AUDIT: crates/video-engine/tests/sync_scenarios.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2025-06-27 |
| Lines / functions | 421 / 7 (3 helpers + 5 tests) |
| File verdict | REAL (production scheduler + queue driven over virtual clocks with exact-value assertions); one deferred-coverage claim points at tests that are `#[ignore]`d |
| Reachable from shipped entry point? | The code under test is the shipped scheduler: `PresentationScheduler::new` is wired at apps/cli/src/pipeline_host.rs:1324 and ticked per pump (:768-775). Test binary is CI-only. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `sed -n` quotes (exit 0); `rg` over apps/cli/tests/sync_scenarios.rs (exit 0); baseline cargo-test-workspace.txt:6536-6543 — all 5 tests pass; :3225-3233 — the CLI composition-root suite: `s7 ... ignored`, `s8 ... ignored`, `s9 ... ok` (1 passed, 2 ignored) |

## 1. One-paragraph verdict
These are real convergence-scenario tests of the real scheduler: virtual clocks (the honest injected-time pattern, verified in tests/common/mod.rs — real `MediaClock` over a settable monotonic source), synthetic frames, and a recording `Presenter` implementing the production seam, driving exact assertions — one snap per PTS gap, estimator reset observable, zero late drops under a forward gap, zero threshold changes under ±½-frame jitter, widening to exactly 2 frames under sustained one-frame lateness, threshold frozen at 1 frame while the drop budget is spent, repeat budget exactly 3+2 under sustained earliness. The one real defect is a coverage deferral that does not hold: the S4 test's comment hands the snap exemption's backward-re-anchor half to scenario S7 in apps/cli — where S7 (and S8) sit behind `#[ignore]` ("pending the stage-time fill observation seam", "pending the 10ms steady-state margin investigation"). The claimed 3.B1 composition-root coverage is 1 of 3 running.

## 2. What actually works (verified)
— Baseline: cargo-test-workspace.txt:6536-6543 — all 5 video-engine sync tests pass.
— Exact-value discipline: `assert_eq!(stats.snap_presents, 1)`, `assert_eq!(last.late_threshold_ns, Some(2 * FRAME))`, `assert_eq!(repeats, 5)` — no tautologies, no `>=`-only oracles on the decisive facts.
— The jitter test's math is correct: `(next() % 1001) - 500` per-mille × FRAME/1000 = ±16,666 ns ≈ ±½ frame, and asserts BOTH no threshold flapping AND no drops AND in-order presentation AND ≥140/150 presents.
— The drop-budget test proves the firehose bound end-to-end: 3 stragglers per tick spend the 3-per-window budget, and the effective threshold stays at the configured 1 frame while the band is WidenLate.

## 3. Findings
F-001 [S3]
Where: crates/video-engine/tests/sync_scenarios.rs:133-136 and header:4-6
Code:
```
    // Exactly one snap, exactly one gap, no drop-storm: the post-gap
    // frames are EARLY relative to the clock's walk, so the late path
    // never fires (the exemption's other half — the backward
    // re-anchor — is S7's, in the composition root).
```
What is wrong: the deferral target does not run. apps/cli/tests/sync_scenarios.rs:345-346 has `#[ignore = "pending the stage-time fill observation seam (3.B1 follow-up)"] fn s7_pause_resume_and_seek_compose_with_the_drift_regimes()` and :232-233 `#[ignore = ...] fn s8_end_to_end_av_tolerance_holds_over_the_closed_loop()` (baseline :3228-3232: 1 passed, 2 ignored). So the snap machinery's backward-re-anchor exemption — the half S4 explicitly does not cover — is proven by no executing test, and the end-to-end A/V tolerance criterion (S8, which measured a 10.001 ms breach vs a 10 ms criterion per the CLI file's own PENDING-AC note) is unproven. The header's "the composition root carries S7–S9 (apps/cli)" carries 1 of 3. Why it matters: any doc/matrix row citing the 3.B1 convergence scenarios as landed evidence is citing a suite that is 5/8ths present (S4/S5/S10×3/S9 run; S7/S8 ignored; audio S1-S3/S6 not verifiable from this file). What correct looks like: the S4 comment must state the backward half is UNTESTED (ignored upstream), not "is S7's". Confidence: HIGH — Verification: read + baseline ignore lines. (Mutation note: the ignore attributes themselves are honest and carry registered reasons — the defect is this file's comment claiming coverage that is switched off.)

F-002 [S4]
Where: crates/video-engine/tests/sync_scenarios.rs:178-184
Code:
```
    let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
```
What is wrong: a third copy of the xorshift64 driver in the test tree (packet-queue/tests/limits.rs:38-53 has `XorShift`, and the same constant seed 0x9E37_79B9_7F4A_7C15 appears in limits.rs:127). Copy-paste drivers can drift (this one lacks the `seed | 1` zero-guard, safe only because the constant is odd). What correct looks like: one shared driver in testkit. Confidence: HIGH — Verification: read both.

F-003 [S4]
Where: crates/video-engine/tests/sync_scenarios.rs:47-50
Code:
```
    if let Some(request) = request {
        let _ = queue.try_push(request);
    }
    let _ = queue.drain_to(sink);
```
What is wrong: the push outcome is discarded. With the 64-wide queue a `Full`/`Stale` refusal is impossible today, so the counting assertions (which read `outcome.action`, not the sink) are unaffected — but a future queue-cap change would silently lose presents from the sink's in-order log while the counts still pass. What correct looks like: `must` on the push or an explicit unreachable match. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| default_cfg | 29-31 | REAL | — | production constructor |
| step | 35-52 | REAL | S4 | production-shaped wiring; F-003 discard |
| wide_queue | 54-56 | REAL | — | |
| sync_step | 242-248 | REAL | — | observation point |
| test s4_pts_step_discontinuity_snaps_exactly_once | 63-163 | REAL | S3 | exact snap/gap/drop/band facts; backward half deferred to an ignored test (F-001) |
| test s5_pts_jitter_never_flaps_the_threshold | 169-234 | REAL | S4 | exact no-flap/no-drop/in-order; duplicated xorshift (F-002) |
| test s10_sustained_latness_widens_the_threshold_in_band | 254-307 | REAL | — | widened == 2·FRAME exact + monotone/cap grid sweep |
| test s10_spent_drop_budget_freezes_the_widening | 313-363 | REAL | — | threshold frozen at 1·FRAME while budget spent |
| test s10_sustained_earliness_extends_the_repeat_budget | 369-421 | REAL | — | repeats == 5 exact; runaway guard inside the loop |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| s4 | YES | snap/gap/drop counts exact; estimator reset asserted on the tick after the gap; post-snap error band and Hold-only bands swept |
| s5 | YES | threshold_changes == 0 and correction_ticks == 0 and dropped_late == 0 exact |
| s10 widening | YES | band + threshold value exact; grid half re-derives the clamp |
| s10 budget | YES | dropped >= 120, presented >= 40, threshold == configured exact |
| s10 earliness | YES | banded + repeats == 5 exact; inner runaway assert |
| CAVEAT | — | all five run on the same virtual-clock abstraction; the composition-root behaviors (S7/S8) are ignored upstream (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the composition root carries S7–S9 (apps/cli)" | header:4-6 | SHALLOW | S9 runs; S7/S8 are #[ignore]d (baseline :3228-3232); 1 of 3 |
| "the exemption's other half — the backward re-anchor — is S7's, in the composition root" | lines:135-136 | FALSE (as coverage) | S7 ignored; backward-reanchor snap untested in any running suite I can see |
| "nothing in this suite touches an event surface because none exists on the scheduler" | header:8-11 | REAL | no event API consumed in this file |

## 7. Dead / duplicated / unreachable code
- Inline xorshift duplicate (F-002). No `#[ignore]` in this file itself; no feature gates.

## 8. Dependencies & flags
video-engine (production scheduler/queue/policy), media-clock, media-types, testkit; tests/common shared with scheduling.rs. Runs in the default lane (baseline :6536-6543).

## 9. Open questions for the next agent
- audio-engine's S1-S3/S5-fill/S6 scenarios are claimed by the header but were NOT audited here — whoever owns audio-engine's governor/resample suites should confirm they run (not ignored).
- The S8 PENDING-AC note (10.001 ms vs 10 ms criterion) is a live spec-vs-math discrepancy in the closed loop — a candidate S2 for the composition-root owner once un-ignored.
- No test in this file covers widening RETRACTION (Hold clearing `widened` after sustained lateness ends) — scheduler.rs's own tests may; verify before citing "ladder fully proven".
