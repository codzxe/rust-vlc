# AUDIT: crates/video-engine/tests/scheduling.rs
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-23 |
| Lines / functions | 757 / 25 (21 tests + 4 helpers) |
| File verdict | REAL (production types under test, virtual clocks, precise assertions; 21/21 executed pass) |
| Reachable from shipped entry point? | N/A (test file) — the code under test IS shipped: `PresentationScheduler` constructed in apps/cli/src/pipeline_host.rs:1324 (lumen release lane) and apps/desktop |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p video-engine --test scheduling` → `21 passed; 0 failed; 0 ignored` in 0.00s (zero sleeps confirmed — virtual-clock suite) |

## 1. One-paragraph verdict
This suite is what it claims: acceptance tests for the presentation scheduler over injected time, zero sleeps, driving the PRODUCTION types (`PresentationScheduler`/`PresentationQueue`/`Presenter`/`MediaClock`) with a recording presenter and settable virtual clocks (common/mod.rs — real `MediaClock::system_monotonic` over a settable `FakeMonotonic`, real `MediaClock::audio_master` over a `FakeCounter`). Assertions are exact: boundary pins (`pts == now − threshold` presents, one past drops — 990 vs 989), drop-count arithmetic, stats conservation (`presented + late + superseded + stale + pending == pushed`), wait hints, pending lengths, and full delivery-order vectors. I executed all 21 tests — pass in 0.00s. One named AC is not actually enforced (Auto-policy threshold, F-001) and the "property" test is 5 hand-picked permutations (F-002).

## 2. What actually works (verified)
- `cargo test -p video-engine --test scheduling` → `test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured` (executed by me, exit 0).
- Mutation reasoning on the strongest tests: `ac_boundary_pts_one_past_the_late_threshold_drops` pins the STRICT `<` at the late threshold (990 presents / 989 drops); `stats_conserve_every_admitted_frame` pins the accounting law; `ac_late_frames_dropped_...` pins `presented == 20` + `repeats == 1` with a 21-slot delivery vector — these fail on off-by-one, accounting, or ordering regressions.
- Helpers verified real (tests/common/mod.rs:1–273): `VirtualClock` wraps `MediaClock::system_monotonic` (line 44–46), `AudioMasterClock` wraps `MediaClock::audio_master` (125–128) — injected-truth clocks per doc 03 §8, not fake schedulers; `RecordingPresenter` records every present and `assert_in_order` walks the log.

## 3. Findings
F-001 [S3]
Where: crates/video-engine/tests/scheduling.rs:633–658
Code:
```
    for frame_index in 0..5i64 {
        let _ = scheduler.push(frame(frame_index * FRAME as i64, GenerationId::FIRST));
    }
    clock.set_ns(10 * FRAME);
    let outcome = step(&mut scheduler, &mut queue, &mut sink);
    // One-frame threshold: frames 0..9 are late → 0..4 all dropped.
    assert_eq!(outcome.drops.late.len(), 5);
```
What is wrong: the test is named `auto_policy_resolves_to_the_doc_default_one_frame_threshold` but its frames sit at pts 0..4×FRAME with the clock at 10×FRAME — late under ANY threshold from 0 up to ~6 frames. It cannot distinguish Auto resolving to the doc's one-frame threshold from a half-frame or three-frame resolution; it only proves Auto is not `Never` (drops late frames at all). The named AC (the exact default) is unpinned. Why it matters: a regression that changes Auto's resolved threshold (e.g. to 0 — aggressive dropping of everything not exactly on time) passes this test silently. What correct looks like: a frame in the discrimination band (e.g. pts = 9.5×FRAME at now = 10×FRAME: on-time under a one-frame threshold, late under a half-frame one) asserted both ways. Confidence: HIGH — Verification: read + mutation reasoning (not executed — no code edits allowed).
F-002 [S4]
Where: crates/video-engine/tests/scheduling.rs:414–443
Code:
```
    // Hand-picked arrival permutations (a property framework is a doc
    // 04 candidate; deterministic permutations cover the same ground).
    let permutations: [Vec<i64>; 5] = [
```
What is wrong: 5 hand-picked permutations of 5 frames (120 exist) carry a "no frame out of order under shuffled arrival" name. Body-disclosed, each permutation asserts the exact full order, so it is honest but thin. Confidence: HIGH — Verification: read.

## 4. Per-function table (tests + helpers)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `default_cfg` | 32–34 | REAL | — | `for_frame_duration(FRAME)` doc default |
| `step` | 39–55 | REAL | — | production wiring: tick → push request → drain to presenter |
| `wide_queue` | 58–60 | REAL | — | 64-deep so bound never interferes |
| `ac_in_order_presentation_when_the_clock_advances_normally` | 68–100 | REAL | — | 60 frames, exact order + conservation + gens |
| `ac_late_frames_dropped_never_presented_late_or_reordered` | 108–157 | REAL | — | 10 drops, resume order, `presented==20`/`repeats==1` arithmetic (presented excludes repeats — self-consistent) |
| `drops_are_never_resurrected_after_their_stall` | 159–177 | REAL | — | stable accounting across repeated stalled ticks |
| `ac_early_frames_are_held_until_their_slot` | 184–226 | REAL | — | hold, repeat-as-present, `next_due == pts − lead`, pending_len |
| `boundary_step` | 232–264 | REAL | — | fresh scheduler, threshold 10, lead 5 |
| `ac_boundary_pts_equals_now_presents` | 266–270 | REAL | — | |
| `ac_boundary_pts_exactly_on_the_late_threshold_presents` | 272–276 | REAL | — | strict `<` pinned |
| `ac_boundary_pts_one_past_the_late_threshold_drops` | 278–282 | REAL | — | |
| `ac_boundary_far_late_drops_the_same_way` | 284–288 | REAL | — | |
| `ac_boundary_pts_exactly_on_the_due_bound_presents` | 290–294 | REAL | — | inclusive `≤` pinned |
| `ac_boundary_one_past_the_due_bound_holds` | 296–318 | REAL | — | hold + next_due hint |
| `ac_generation_stale_frames_are_dropped_end_to_end` | 325–379 | REAL | — | generation publish → 3 stale drops, stale push refusal typed, new-gen flow; end-to-end doc 03 §5(7) |
| `no_frame_is_presented_out_of_order_under_shuffled_arrival` | 414–443 | REAL, thin | S4 | F-002 |
| `presented_error_stays_within_the_policy_bounds_at_every_present` | 450–522 | REAL | — | per-present bounds `[now−threshold, now+lead]`, stall + recovery; lower-bound len assert is weak but secondary |
| `audio_master_sample_counter_drives_the_schedule` | 529–555 | REAL | — | 1600 samples/frame, 60 presents, 0 drops (grid drift <1ns/frame absorbed by lead) |
| `audio_master_stall_drops_late_frames` | 557–571 | REAL | — | 5s audio stall → 10 late drops |
| `never_policy_presents_late_frames_in_order_via_supersede` | 578–611 | REAL | — | supersede ≠ late accounting pinned |
| `auto_policy_resolves_to_the_doc_default_one_frame_threshold` | 633–658 | SHALLOW | S3 | F-001: cannot discriminate the one-frame resolution |
| `paused_clock_freezes_the_schedule_mechanically` | 664–696 | REAL | — | frozen now holds frame 1; repeat budget 3 drains to background; resume presents |
| `stats_conserve_every_admitted_frame` | 701–732 | REAL | — | conservation law |
| `window_bound_surfaces_as_typed_window_full_at_the_wiring_level` | 739–766 | REAL | — | 16-cap + typed WindowFull + counter |
| `queue_bound_surfaces_as_typed_full_at_the_wiring_level` | 768–791 | REAL | — | depth-1 queue typed Full, drain frees, retry lands |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all boundary tests | YES | exact on/off-bound values (990 vs 989; 1005 vs 1006) |
| late-drop / hold / pause / conservation / stale tests | YES | exact counts, order vectors, stats arithmetic |
| `auto_policy_resolves_to_the_doc_default_one_frame_threshold` | NO (for its named claim) | any threshold in (0, ~6 frames] passes — F-001 |
| `no_frame_is_presented_out_of_order_under_shuffled_arrival` | YES (for what it asserts) | exact order per permutation; but only 5/120 permutations |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "virtual-clock determinism … all on injected time, zero sleeps" | scheduling.rs:1–5 | REAL | no `sleep`/`thread` anywhere; suite ran in 0.00s (executed) |
| "The wiring under test is the production shape at v0" | scheduling.rs:6–8 | REAL | same `PresentationScheduler`/`PresentationQueue` types the shipped CLI constructs (pipeline_host.rs:1324) |
| "a repeat is a present (scheduler.rs step 5)" | scheduling.rs:203–205 | REAL-UNPROVEN | consistent with all passing order/count asserts; scheduler.rs itself not in my assignment |
| "Auto == doc default" (test name) | scheduling.rs:633 | SHALLOW | F-001 |

## 7. Dead / duplicated / unreachable code
- None. `frame_missing`/`failing_at`/`presenter_failure`/`advance_ns` in common/mod.rs are unused by this suite (used by the neighbor suites sync_scenarios.rs/zero_alloc_scheduler.rs; `#![allow(dead_code)]` at mod.rs:17 discloses it).

## 8. Dependencies & flags
- Test-only; dev-deps media-types/media-clock/testkit + the crate itself. No feature gates, no #[ignore]. Ran in the baseline per-package pass set (4744) and re-executed by me.

## 9. Open questions for the next agent
- crates/video-engine/src/scheduler.rs (the implementation) is not in my assignment — the "repeat is a present" and "presented excludes repeats" semantics I inferred from test arithmetic should be confirmed there.
- sync_scenarios.rs and zero_alloc_scheduler.rs (neighbor suites) unassigned to me.
