AUDIT: apps/cli/tests/sync_scenarios.rs
=======================================

| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-17 |
| Lines / functions | 600 / 12 (3 scenario tests, 2 of them `#[ignore]`d) |
| File verdict | SHALLOW (the composition harness is real and detailed; the two flagship scenarios never run, and the module doc claims the result one of them failed to deliver) |
| Reachable from shipped entry point? | NO — test-only (drives `lumen_cli` library internals: `CliHost`, `AudioSession`, `PresentationScheduler`). The s9 test drives the same dispatch the binary uses. |
| Findings | S0:1 S1:0 S2:1 S3:0 S4:2 |

Commands run (static; no cargo per adaptations):
- Read file complete (2 chunks)
- `ls tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` → EXISTS (s9's fixture is committed)
- `sed -n '12,16p;226,233p' apps/cli/tests/sync_scenarios.rs` (quote verification, passed)
- W00 baseline: `audit/runs/baseline/cargo-test.log` + `honesty-counters.txt` — workspace totals "1751 pass / 2 fail / 25 ignored"; this file contributes 2 of the 25 ignored (S7, S8).

## 1. One-paragraph verdict

This file is sold as the proof that the A/V sync loop actually closes — "the doc 03 §4.10 f A/V tolerance holds at every present" (module doc, lines 12-16) — and then ships that proof as `#[ignore]`d, with a comment admitting the measured steady-state offset was **10.001 ms against the 10 ms criterion**: the composition misses its own acceptance criterion, and instead of fixing it the scenario was parked. A second scenario (S7, pause/resume/seek drift) is also ignored pending a missing observation seam. Only S9 (no event storm) runs, and it is a real test — real fixture, real dispatch, real assertions — but it proves event hygiene, not sync. The harness machinery (virtual-time device drift, i128 deficit arithmetic, production wiring order) is genuine and well-built; the file's problem is that its two headline claims are unproven by construction.

## 2. What actually works (verified)

— verified by reading s9 (543-600): opens the committed `avi_mjpeg_pcm.avi` through `lumen_dispatch`, `settle()`, `advance(RUN_PUMPS)`, asserts `PlaybackState::Ended`, every observed event matches the 23-arm catalog whitelist (513-541), total events ≤ 2000, time-ticks ≤ 10 over the 2 s fixture. These assertions fail on an event-storm regression or an uncataloged event kind.
— verified by reading the Harness (68-220): pure virtual time (zero sleeps), deterministic synthesized audio (115), RGBA 2×2 video frames, drift injected only through the device's period appetite (182-191, i128 exact), production `advance_time` ordering (stage → tick → render → observe). No mocks of the subject: it composes the REAL `AudioSession`, `MediaClock`, `PresentationScheduler`.
— UNVERIFIED at runtime: S7/S8 bodies are read-verified only (and cannot run — ignored); S9 not executed here (no cargo).

## 3. Findings

### F-001 [S0] — module doc claims the A/V tolerance "holds at every present"; the only test that checks it measured a breach and is ignored
Where: apps/cli/tests/sync_scenarios.rs:12-16 (claim) vs 226-233 (the ignored test and its measured breach)
Code:
```rust
//! The S8 scenario is the R1 closure proof (W1-18 §8): the loop that
//! lives outside the owned crates actually runs — fill → governor →
//! correction → resampler → ring → counter → clock → scheduler →
//! instrumented presents — and the doc 03 §4.10 f A/V tolerance holds
//! at every present.
```
```rust
// PENDING-AC (registered at the 3.B1 merge): the closed loop's
// steady-state A/V offset measured 10.001 ms against the 10 ms
// criterion — a razor-thin margin breach that needs the merged
// 3.B3 debounce/3.C1 priming interactions tuned (or the criterion's
// measurement point clarified), not an assertion nudge.
#[test]
#[ignore = "pending the 10ms steady-state margin investigation (3.B1 follow-up)"]
fn s8_end_to_end_av_tolerance_holds_over_the_closed_loop() {
```
What is wrong: the header states the tolerance holds; the file's own registration comment records that when the test ran, it measured 10.001 ms against a 10.000 ms criterion — i.e., the tolerance does NOT hold at every present in the composed loop. The test is then ignored. The claim and the code's own recorded measurement contradict each other, and the contradiction is resolved in favor of the claim being kept in the docs.
Why it matters: this is exactly the audit's target pattern — a doc assertion whose disproving test exists but is parked. Anyone reading the module doc (or a worklog citing "R1 closure proven") believes the sync loop meets doc 03 §4.10 f. The repo's own evidence says it misses by a hair in the harness regime (which the comment attributes to unresolved 3.B3/3.C1 interactions — plausible, but unproven either way since the test no longer runs).
What correct looks like: either fix the composition until the un-ignored test passes, or change the doc to "measured 10.001 ms against the 10 ms criterion; investigation pending" — a claim that matches the evidence.
Confidence: HIGH — Verification: read (both quotes verified by sed; both tests' `#[ignore]` attributes read).
### F-002 [S2] — S7 (pause/resume/seek drift composition) also ignored: 2 of 3 advertised scenarios never run
Where: apps/cli/tests/sync_scenarios.rs:338-346
Code:
```rust
// PENDING-AC (registered by the Lead at the 3.B1 merge): the fill
// observation seam is render-time only (governor.rs `observe`),
// so "decode-ahead while paused still fills the ring" cannot be
// observed yet — the stage-time fill observation is the follow-up
// card (audio-engine governor + limb seam). Un-ignoring requires
// that seam, not a test change.
#[test]
#[ignore = "pending the stage-time fill observation seam (3.B1 follow-up)"]
fn s7_pause_resume_and_seek_compose_with_the_drift_regimes() {
```
What is wrong: the second of the file's three scenarios cannot observe its subject (stage-time fill) because the production seam does not exist. The test body still asserts things it cannot currently verify (`frozen.fill_ns_last.must() > pre.fill_ns_last.must()` at 379-382 — the very observation the comment says is unavailable… which is why it is ignored).
Why it matters: the pause/resume/seek × drift-regime composition — the exact area where a real player's clock bugs live — has zero running coverage in the CLI. Combined with run.rs's F-001 (no real-time pump) the drift governor is also never armed in production (pipeline_host F-7/`arm_drift_governor` unreachable): the entire drift-correction subsystem is harness-only.
What correct looks like: land the stage-time observation seam (audio-engine card) and un-ignore; until then the module doc should say S7 is unproven.
Confidence: HIGH — Verification: read.
### F-003 [S4] — comment says "24-variant whitelist", the match has 23 arms
Where: apps/cli/tests/sync_scenarios.rs:571-572 vs 513-541
Code:
```rust
    // Every event is a catalog row (the 24-variant whitelist above —
    // a 25th kind fails this match) and the burst bound holds with
```
What is wrong: the `is_catalog_kind` match lists 23 variants (counted: SessionStateChanged … MediaInfoReady); the comment and the D-017 reference say the catalog is 23 rows elsewhere (line 511-512: "the frozen doc 05 catalog (D-017, 23 rows") — so the "24-variant" phrasing at 571 contradicts the file's own line 511.
Why it matters: trivial, but it is an inventory comment in an audit-sensitive spot.
Confidence: HIGH — Verification: read (counted both).
### F-004 [S4] — dead loop variable silenced
Where: apps/cli/tests/sync_scenarios.rs:446-464
Code:
```rust
    for step in 0..10 * 30 {
        harness.step(generation);
        ...
        let _ = step;
    }
```
What is wrong: `step` is never used; `let _ = step;` silences the warning instead of `for _ in`.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts RATE/QNS/PRIME_QUANTA/CONVERGENCE_S/TAIL_S/STEP_LIMIT_PPM/MAX_CORRECTION_PPM | 43-61 | REAL | — | 44 100/1470 = exactly 30 fps (checked: 44100/1470 = 30 exactly). |
| `Harness` (struct) | 68-86 | REAL | — | presents/corrections/fills trails. |
| `Harness::new` | 91-108 | REAL | — | arms governor with production target (93) — the ONLY arming call anywhere (production never arms; see pipeline_host audit §7). |
| `Harness::stage` | 112-135 | REAL | — | deterministic s16 synth; pts in ns. |
| `Harness::push_video` | 139-153 | REAL | — | RGBA 2×2, BT709_LIMITED tag; Scheduled must hold. |
| `Harness::step` | 159-168 | REAL | — | production advance_time order. |
| `Harness::consume` | 175-210 | REAL | — | i128 deficit exact drift; paused clock gate mirrors host.rs; sub-quantum nibble → no tick (documented). |
| `Harness::prime` | 214-219 | REAL | — | 2-quanta pre-roll. |
| `s8_end_to_end_av_tolerance_holds_over_the_closed_loop` | 231-332 | IGNORED | S0 | F-001; body asserts fill convergence, limiter bounds, ordering, ≤10 ms at every present, ≥95% within half a frame — would fail today per the PENDING-AC comment. |
| `s7_pause_resume_and_seek_compose_with_the_drift_regimes` | 344-504 | IGNORED | S2 | F-002; double reanchor (411+422) is deliberate (discard absorption, commented). |
| `is_catalog_kind` | 513-541 | REAL | S4 | 23 arms vs "24-variant" comment (F-003). |
| `s9_no_event_storm_over_the_first_vertical` | 543-600 | REAL | — | fixture exists (verified); real assertions: Ended, whitelist, ≤2000 events, ≤10 ticks. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| s8 (IGNORED) | Would fail TODAY | its own registration comment says it measured 10.001 ms vs 10 ms — the ignore is hiding a failing test, honestly labeled. |
| s7 (IGNORED) | Unknown | cannot observe its subject (missing seam); the assertion at 379-382 depends on `fill_ns_last` being stage-observable — the comment says it is not. |
| s9 | YES | runs: Ended-state, whitelist, storm bounds on a committed fixture. |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the doc 03 §4.10 f A/V tolerance holds at every present" | sync_scenarios.rs:12-16 | FALSE (unproven + contradicted) | the S8 proof is `#[ignore]`d with a recorded 10.001 ms breach (226-232). |
| "S9's composed half … the sync machinery ran the whole time" | sync_scenarios.rs:318-323 (inside ignored S8) | UNREACHABLE | inside the ignored test. |
| "zero underruns and zero ring refusals" (S8 assertions) | sync_scenarios.rs:256-258 | UNVERIFIED | ignored test. |
| "the D-017 ceiling is 1000/s; the steady budget is time-tick at 4 Hz" | sync_scenarios.rs:573-575 | REAL-UNPROVEN | s9 asserts ≤10 ticks/2 s (≈4 Hz+2); D-017 doc itself is another wave's file. |

## 7. Dead / duplicated / unreachable code

- `Harness` + s7/s8 bodies: reachable only by un-ignoring; currently dead test code (deliberately parked, honestly labeled).
- `arm_drift_governor`/`DRIFT_TARGET_FILL_NS` (pipeline_host.rs) have their ONLY caller here (93) — the drift governor is a harness-only subsystem today (cross-ref pipeline_host audit §7).

## 8. Dependencies & flags

- Integration test of lumen-cli; uses `lumen_cli` public API (dispatch/host/pipeline_host), `video_engine`, `media_clock`, `testkit`. No features. Fixture committed (verified).

## 9. Open questions for the next agent

1. audio-engine `governor.rs` — confirm the fill observation is render-time only (the S7 blocking seam) and whether the 3.B1 follow-up card exists anywhere in the tree.
2. Whoever audits doc 03 §4.10 f / W1-18: check whether "R1 closure proven" is claimed with this file as evidence — if so, that claim inherits F-001.
3. The 10.001 ms breach: is the measurement point (slot − clock-at-delivery) the criterion's intended point? The PENDING-AC offers both tuning and clarification as out — nobody has decided.
