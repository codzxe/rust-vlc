# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_05_stop.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2025-06-27 |
| Lines / functions | 148 / 1 test |
| File verdict | REAL (real stack: FileAccess + AviDemuxer + real MJPEG/PCM decoders + real Engine; exact assertions with a genuine Drop-hook teardown oracle); the doc-10 row it claims is satisfied only under a redefined teardown boundary |
| Reachable from shipped entry point? | Stack under test is shipped (apps/cli/src/host.rs:231/983 uses the same Engine; host.rs:1496 `AviDemuxer::new`, :705 `RealProvider`). Test binary is CI-only. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `sed -n`/`rg` verification of host.rs routes (exit 0); baseline cargo-test-workspace.txt:4391 — `scenario_05_stop::stop_midstream_releases_stage_objects_within_deadline ... ok` (23/23 in the binary, 5.06 s) |

## 1. One-paragraph verdict
A real end-to-end stop scenario on the real container (PARENT = generated/valid/avi_mjpeg_pcm.avi through real access/demux/decode): stop mid-stream answers, settles `Stopped`, is provably inert (frame count unchanged across 8 more pumps), the position freezes (asserted twice plus a `MediaGetCurrentTime` consistency check), the close returns `Idle` with both stage objects' Drop hooks fired, the post-close time query answers typed `NotReady`, and the host reopens from zero. The teardown oracle is genuine — `DropCountingDemuxer` counts its own `Drop` (host.rs:366-370), so "released" is an object-graph fact, not a log line. The one material defect: the doc-10 §6.4 row this scenario owns says "teardown complete (threads/fd/device released) within deadline" — the test proves teardown at the CLOSE, not at the STOP (the stopped session retains the demuxer and sink by design), and redefines the row's "stop's completion boundary" to mean the close. Honest, documented, but the row's plain meaning is not what v0 does.

## 2. What actually works (verified)
— Baseline: cargo-test-workspace.txt:4391 — the test passes (whole binary 23/23 in 5.06 s).
— The deadline IS enforced where it matters: `await_receipt` (host.rs:1387-1401) fails typed ("the engine never answered the command (bounded wait expired)") on bound breach, so a hung stop/close fails the test's `result.is_ok()` asserts — "a hang is a bound breach, never a mystery" is implemented, not just said.
— Inertness oracle: `frames_accepted()` equality across 8 post-stop pumps (lines 64-72) — a stop that keeps decoding fails exactly.

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/tests/scenarios_b/scenario_05_stop.rs:1-14 and 96-102 vs docs/10-testing-ci-quality.md:685
Code:
```
//! The v0 synchronous engine's teardown evidence: `playback.stop`
//! mid-stream answers its receipt, the state settles to `Stopped`
//! (the session survives, inert until close — state.rs's `Stopped`
...
//! number of pumps (the deadline — a hang is a bound breach, never
//! a mystery)
```
and doc 10 §6.4 row 5: "`stop` | stop mid-stream | teardown complete (threads/fd/device released) within deadline | teardown report".
What is wrong: the row's pass criteria, read plainly, requires the STOP to release resources. The test proves the demuxer and sink survive the stop (the stopped session is "inert until close") and are released only at the CLOSE (`DestroySession`); the docstring redefines "the stop's completion boundary" as the close (lines 7-10, 98-102). Hostile summary: the scenario passes because its criterion was reinterpreted to match the implementation. The reinterpretation is prominently documented (no concealment), and the v0 engine is threadless (host.rs:319-321), so "threads/fd" never applied — but "device released at stop" is a real VLC-expectation gap recorded nowhere as a divergence from doc 10. What correct looks like: either the row gains a v0 note ("release at close in v0") or the stop path drops the stage objects. Confidence: HIGH — Verification: read of test + doc 10 + host.rs drop-hook wiring.

F-002 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_05_stop.rs:48-50
Code:
```
    host.pump_pipeline_until(|host| host.sink().map(|s| s.audio_accepted()).must() >= 10);
    let position_at_stop = host.position_us();
    assert!(position_at_stop > 0, "the stop lands mid-stream");
```
What is wrong: `pump_pipeline_until` returns whether the predicate was observed within `AWAIT_PUMPS` (host.rs:1125-1133); the bool is discarded, so a bound breach at the 10-samples predicate is silent. The follow-up `position_at_stop > 0` still guarantees ≥1 sample, but the intended "10 samples in" mid-stream condition can degrade to "1 sample in" without any failure. What correct looks like: `assert!(host.pump_pipeline_until(...))`. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| test stop_midstream_releases_stage_objects_within_deadline | 24-148 | REAL | S3/S4 | every phase exactly asserted; F-001 (row reinterpretation), F-002 (discarded bound) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| stop_midstream_releases_stage_objects_within_deadline | YES | state equality, inertness equality across pumps, frozen-position equality, drop-hook booleans, typed NotReady match, reopen-from-zero — a stop that leaks frames, unfreezes the clock, retains the demuxer at close, or poisons reopen all fail |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| doc 10 §6.4 row 5: "stop ... teardown complete (threads/fd/device released) within deadline" | docs/10-testing-ci-quality.md:685 | SHALLOW | teardown proven at CLOSE under a documented v0 reinterpretation (F-001); deadline enforced via bounded await |
| "a hang is a bound breach, never a mystery" | scenario_05_stop.rs:10-11 | REAL | await_receipt typed failure on expiry (host.rs:1398-1401) |
| "the wired stage objects are released at the stop's completion boundary (the close)" | scenario_05_stop.rs:7-9 | REAL (as redefined) | Drop hooks fire between close dispatch and the assertions |

## 7. Dead / duplicated / unreachable code
None in this file.

## 8. Dependencies & flags
commands, testkit (serializer + fixture path), crate::host (ScenarioHost over the real stack). No feature gates; runs in the default lane (baseline :4391).

## 9. Open questions for the next agent
- The stopped-session-retains-demuxer design is the engine's (state.rs `Stopped` row family) — whoever owns playback-engine/src/state.rs should confirm the resource-retention-until-close behavior is a recorded v0 decision in the engine docs, not just in this test's prose.
- scenario_12/16 (device loss, exit during playback) share the DropHooks machinery — same reinterpretation risk for their §6.4 rows ("recovery on return", "teardown within deadline").
