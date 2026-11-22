# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_16_exit_during_playback.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 238 / 2 tests + helpers |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | N/A — scenario test (doc 10 §6.4 row 16) over the command layer; the in-process drop is the v0 shape of "exit mid-playback". |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | `head -57` + `sed -n '79,140p'` (exit 0). No cargo; run UNVERIFIED (W00: 1751/2/25). |

## 1. One-paragraph verdict

A teardown test with real oracles: abrupt drop mid-playback (no stop, no close) must release every owned stage object (drop-hook counters asserted at exactly 1 for the demux stage and the consumer seam), return the process fd count to baseline immediately, leak no threads (bounded wait for scheduler-tick lag — the CI-load shape, disclosed), leave the scratch dir byte-identical (no sidecars/journals — the "no temp state written" criterion), and complete the drop within the 2 s deadline (measured, recorded). A second test repeats mid-timeline. The header names what is NOT tested (dropping during Opening/Buffering — impossible to hit deterministically in this synchronous host; the process-level exit arm delegated to the first-vertical suite). Honest bounds, exact counters.

## 2. What actually works (verified)

- fd-count oracle (`open_fds()` before/`+1` while playing/back to baseline after drop) — a real resource-leak detector, read-verified at 111-135.
- Drop hooks assert exact counts (1 demux drop, 1 sink drop) — `Arc`-clone leaks would show as 0 or >1.
- The scratch-dir identity check pins "the engine never writes anything anywhere".
- Mid-play precondition asserted as a range (30..120 frames) — guards against testing teardown of a nearly-dead or barely-started session.

## 3. Findings

### F-001 [S4] The 2 s teardown deadline is generous and the measured value is only recorded
The bound is doc 03 §4.6 S2's proposed deadline, "generous on purpose — the point is *bounded*, not fast" (header). A teardown regression from 50 ms to 1.9 s would pass. The report records the measured value; the test does not trend it. Acceptable for the row's wording; recorded.

### F-002 [S4] Thread-count check tolerates bounded scheduler lag rather than asserting exact synchronous release
The disclosed "a session worker's exit can land a scheduler tick behind it" wait is the honest shape under CI load, but it means a genuine one-thread leak that takes longer than the bounded wait would slip. Hygiene; the engine is threadless in v0, so the oracle is mostly vacuous today — it exists for the threaded future (2.E1).

## 4. Per-function table

| Function | Lines | Verdict | Notes |
|---|---|---|---|
| `open_file` | 57-77 | REAL | |
| `abrupt_drop_mid_playback_releases_everything_within_deadline` | 78-176 | REAL | fd/drop-hook/dir/time oracles |
| `abrupt_drop_mid_timeline_releases_stage_objects` | 177-238 | REAL | the timeline variant |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| abrupt_drop_mid_playback… | YES (reasoned) | exact drop-hook counts, exact fd arithmetic, dir identity, deadline — a leaked Arc, fd, file, or slow drop each fails |
| abrupt_drop_mid_timeline… | YES (reasoned) | same counters at a different point |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| doc 10 §6.4 row 16: teardown within deadline; device closed; exit code clean; teardown report | header 6-9 | REAL-UNPROVEN | all four criteria have oracles (exit-code criterion = the test binary's own exit, disclosed); run UNVERIFIED |
| "NOT tested (named, not hidden)" list | header 47-56 | REAL (an admission) | the Opening/Buffering drop and the process-level arm are named with owners |

## 7-9. Dead code / Dependencies / Open questions

None. Open: does the first-vertical suite actually carry the process-level exit arm it is delegated to (W5-07 scenario 5)? File: tests/integration/first-vertical/src/scenarios.rs (another agent).
