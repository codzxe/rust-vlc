# AUDIT: apps/cli/tests/sync_scenarios.rs
| Field | Value |
|---|---|
| Auditor | W13-A10 |
| Date | 2025-06-27 |
| Lines / functions | 600 / 10 (6 harness methods + 1 helper + 3 tests) |
| File verdict | SHALLOW — the harness composes the REAL production halves (AudioSession/NullDevice/MediaClock/PresentationScheduler, virtual time, exact i128 ppm math), but 2 of its 3 scenarios are `#[ignore]`d, including the file's central A/V-tolerance proof, which is ignored because the composition MEASURES 10.001 ms against the 10 ms criterion (a recorded breach, not a missing seam) |
| Reachable from shipped entry point? | N/A (integration test). Runs under `cargo test -p lumen-cli` — executed by me: `1 passed; 0 failed; 2 ignored ... finished in 0.50s`. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `~/.cargo/bin/cargo test -p lumen-cli --test sync_scenarios` → output pasted in §2; baseline cargo-test-workspace.txt:3228-3230 (same shape: 2 ignored, 1 ok); sed quote verification |

## 1. One-paragraph verdict
The harness is the genuine article: `Harness::new` opens a real `AudioSession` through `open_audio_session`, arms the production drift governor with `DRIFT_TARGET_FILL_NS`, wires the real `PresentationScheduler` over the shared `MediaClock`, and injects device rate error through an exact i128 whole-frame deficit accumulator (`consume`, L175-210) — no mocks, no sleeps. But the file exists to prove S7 (pause/resume/seek composition) and S8 (end-to-end A/V tolerance, "the R1 closure proof"), and both are `#[ignore]`d. S8's own comment records why: the steady-state A/V offset measured 10.001 ms against the 10 ms doc-03 §4.10 f criterion — the composition currently FAILS the tolerance it exists to prove, and the module header (L15-16) still asserts the tolerance "holds at every present". Only S9 (no event storm) runs, and it is real: whole first vertical to `Ended` over the real fixture with a 23-variant catalog whitelist and storm bounds.

## 2. What actually works (verified)
— verified by `~/.cargo/bin/cargo test -p lumen-cli --test sync_scenarios` →
```
test s7_pause_resume_and_seek_compose_with_the_drift_regimes ... ignored, pending the stage-time fill observation seam (3.B1 follow-up)
test s8_end_to_end_av_tolerance_holds_over_the_closed_loop ... ignored, pending the 10ms steady-state margin investigation (3.B1 follow-up)
test s9_no_event_storm_over_the_first_vertical ... ok
test result: ok. 1 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.50s
```
S9 drives the real `CliHost` + `lumen_dispatch` open → `settle` → `advance(RUN_PUMPS)` → asserts `PlaybackState::Ended`, non-empty events, all catalog kinds, ≤2000 events, ≤10 time-ticks over the 2 s fixture. It would fail on an event-storm regression or an uncataloged kind.

## 3. Findings
F-001 [S2]
Where: apps/cli/tests/sync_scenarios.rs:12-16, 226-233
Code:
```
//! the audio-master [`MediaClock`], and the [`PresentationScheduler`] —
```
and
```
// PENDING-AC (registered at the 3.B1 merge): the closed loop's
// steady-state A/V offset measured 10.001 ms against the 10 ms
// criterion — a razor-thin margin breach that needs the merged
// 3.B3 debounce/3.C1 priming interactions tuned (or the criterion's
// measurement point clarified), not an assertion nudge.
#[test]
#[ignore = "pending the 10ms steady-state margin investigation (3.B1 follow-up)"]
fn s8_end_to_end_av_tolerance_holds_over_the_closed_loop() {
```
What is wrong: the module header claims "the doc 03 §4.10 f A/V tolerance holds at every present" (L15-16) while the only proof of that claim is ignored because the measured composition breaches the criterion (10.001 ms > 10 ms). The claim is therefore FALSE as measured and UNPROVEN in every CI run; the drift-loop closure (fill → governor → correction → resampler → ring → counter → clock → scheduler → presents) has no executing test. Why it matters: this is the composition-root A/V sync guarantee for the whole player; a razor-thin breach at +100 ppm device drift means real hardware (±100 ppm is typical crystal error) sits right at the tolerance edge, and nothing in CI will notice further degradation. What correct looks like: either fix the composition to hold the criterion and un-ignore, or rewrite the header to state the measured breach and the pending status. Confidence: HIGH — Verification: ran (ignored, live) + read.

F-002 [S3]
Where: apps/cli/tests/sync_scenarios.rs:338-345 vs 379-382
Code:
```
// PENDING-AC (registered by the Lead at the 3.B1 merge): the fill
// observation seam is render-time only (governor.rs `observe`),
// so "decode-ahead while paused still fills the ring" cannot be
// observed yet — the stage-time fill observation is the follow-up
// card (audio-engine governor + limb seam).
```
vs
```
    assert!(
        frozen.fill_ns_last.must() > pre.fill_ns_last.must(),
        "decode-ahead while paused still fills the ring"
    );
```
What is wrong: the ignored S7 test asserts an observation (`fill_ns_last` rising while paused, i.e. without renders) that the ignore reason says the current seam cannot produce — the test is written against a future API, so un-ignoring after only the S8 margin fix would still fail here. The scenario's pause/seek regime is double-blocked. Why it matters: two independent blockers on one ignored test lengthens the dead window; the pause/resume + seek composition has zero executing coverage. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: apps/cli/tests/sync_scenarios.rs:511-512 vs 571-572
Code:
```
/// The frozen doc 05 catalog (D-017, 23 rows — the pre-3.B1 set; the
/// sync machinery must never need a 24th).
```
vs
```
    // Every event is a catalog row (the 24-variant whitelist above —
    // a 25th kind fails this match) and the burst bound holds with
```
What is wrong: the whitelist lists exactly 23 variants (counted in `is_catalog_kind`, L514-540); the inline comment says "24-variant ... a 25th kind". Off-by-one comment. The code is correct (any 24th kind fails the match); only the comment lies. Confidence: HIGH — Verification: read + count.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Harness (struct) | 68-86 | REAL | — | session + scheduler + drift + deficit + trails |
| Harness::new | 91-108 | REAL | — | production wiring, arms governor with DRIFT_TARGET_FILL_NS |
| Harness::stage | 112-135 | REAL | — | synthetic quantum, real limb.stage |
| Harness::push_video | 139-153 | REAL | — | real scheduler.push, unreachable! on refusal (test context, acceptable) |
| Harness::step | 159-168 | REAL | — | production advance_time order; records corrections/fills |
| Harness::consume | 175-210 | REAL | — | exact i128 ppm deficit; paused-gate honored; sub-quantum nibble modeled |
| Harness::prime | 214-219 | REAL | — | 2-quantum pre-roll |
| is_catalog_kind | 513-541 | REAL | S4 | 23 variants; comment says 24 (F-003) |
| s8_end_to_end_av_tolerance... | 231-332 | IGNORED / would FAIL | S2 | strong asserts (limiter, ±10 ms fill, order, 95% half-frame) — disabled (F-001) |
| s7_pause_resume_and_seek... | 344-504 | IGNORED / seam-blocked | S3 | pause/seek regime asserts incl. one written against a nonexistent seam (F-002) |
| s9_no_event_storm_over_the_first_vertical | 543-600 | REAL (executed) | — | live pass; real fixture to Ended; storm bounds |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| s8_end_to_end_av_tolerance_holds_over_the_closed_loop | YES — it fails NOW (measured 10.001 ms breach), hence ignored | asserts limiter bound, fill convergence ±10 ms by 45 s + 30 s tail, present order, per-present ≤10 ms, ≥95% within half-frame |
| s7_pause_resume_and_seek_compose_with_the_drift_regimes | YES when running; blocked on two independent seams | frozen-window no-reevaluate, post-resume step limit, epoch reanchor, snap ≤1, post-seek tolerance |
| s9_no_event_storm_over_the_first_vertical | YES (verified live, 0.50 s) | Ended state, catalog whitelist, ≤2000 events, ≤10 ticks |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the doc 03 §4.10 f A/V tolerance holds at every present" | sync_scenarios.rs:12-16 | FALSE as measured / UNPROVEN in CI | the proof test is `#[ignore]`d with the recorded 10.001 ms breach (L227-232); live run shows ignored |
| "Virtual time only, zero sleeps (doc 03 §8)" | sync_scenarios.rs:8-9 | REAL | no sleep/Instant in harness; loop is step-counted |
| "the sync machinery must never need a 24th [event kind]" | sync_scenarios.rs:511-512 | REAL (enforced) | is_catalog_kind match fails on any 24th kind; s9 passes live |

## 7. Dead / duplicated / unreachable code
- s8 + s7 bodies: dead in every run (ignored) — 270 of 600 lines never execute in CI.

## 8. Dependencies & flags
- Production imports only (lumen_cli::pipeline_host, media_clock, video_engine, media_types, commands) + testkit::Must. No feature gates. No unsafe.

## 9. Open questions for the next agent
- audio-engine governor owner: does the stage-time fill observation seam (F-002 blocker) exist anywhere yet? If the 3.B1 follow-up card was dropped, S7 stays dead permanently.
- Who owns doc 03 §4.10 f? The 10.001 ms measurement needs either a composition fix or a criterion clarification — the file says "not an assertion nudge", which is the right posture; verify nobody nudged the 10_000_000 ns constant elsewhere (it is still literal here).
