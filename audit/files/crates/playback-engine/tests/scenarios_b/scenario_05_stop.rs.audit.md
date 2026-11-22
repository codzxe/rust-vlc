# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_05_stop.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 148 / 1 test |
| File verdict | REAL — doc 10 §6.4 scenario 5 (stop mid-stream): teardown oracle via drop hooks, inertness, frozen-position semantics, typed not-ready after close, clean reopen |
| Reachable from shipped entry point? | N/A (test binary; uses the scenarios_b ScenarioHost — host.rs in that dir is another agent's file). The semantics verified match the shipped CLI (`playback.stop` verb) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep (not executed — no cargo) |

## 1. One-paragraph verdict

One thorough scenario test. It arms **drop hooks** on the wired stage
objects (:29) so teardown is verified by observed drops, not by state
strings: stop mid-stream (position > 0 asserted, :49-50), settle to
`Stopped`, prove inertness (8 more pumps, frame count unchanged,
:64-72), prove the stopped-but-open session answers its FROZEN
position and stays frozen across more pumps (:79-94), close → Idle
with `demuxer_dropped()` and `sink_dropped()` both observed at the
close boundary (:111-118), the post-close position snapshot answered
by the typed `NotReady` (never fabricated, :122-129), and a clean
reopen from zero (:131-147). Every claim cites its cross-oracle
(session_flow.rs rows, state.rs line refs) in comments.

## 2. What actually works (verified)

— The teardown oracle (drop hooks) is the honest form of "stage
objects released": a leak (demuxer or sink twin retained) fails
`hooks.demuxer_dropped()`/`sink_dropped()`.
— Inertness is measured by frame-count equality across 8 pumps — a
post-stop decode regression fails it.
— The frozen-position semantics pinned here match the CLI's
documented behavior (host.rs:1064-1071 cited; verified consistent
with the engine's `StopClock` effect family in the expected_table
audit).
— Mutation reasoning (UNVERIFIED): a stop that tears down early
(fails the frozen-position check), a stop that keeps decoding (fails
inertness), a close that leaks (fails hooks) — all detectable.

## 3. Findings

### F-001 [S4] The "deadline" of the scenario title is a fixed 8-pump loop, not a measured deadline
Where: scenario_05_stop.rs:65-67 (and :87-89)

Code:
```
    for _ in 0..8 {
        host.pump_pipeline();
    }
```
What is wrong: doc 10 §6.4 row 5 says "teardown complete within
deadline"; the test's deadline is the implicit settle inside
`dispatch`/`host.settle()` plus two hardcoded 8-pump loops. That is a
bounded-hang discipline (good), but nothing measures wall time or
asserts the bound is the documented one — the "deadline" language
oversells what is checked (eventual, bounded completion).
What correct looks like: name the pump bound (e.g.
`const TEARDOWN_DEADLINE_PUMPS: usize = 8`) and reference doc 10's
figure, or state in the module doc that v0's deadline is
pump-count-bounded by design.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `stop_midstream_releases_stage_objects_within_deadline` | 24-148 | REAL | S4 | drop-hook oracle; inertness; frozen position; typed not-ready; reopen (F-001 deadline note) |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `stop_midstream_releases_stage_objects_within_deadline` | YES | observed drops, frame-count inertness, exact frozen-position equality, typed error pins |

Not executed by me (no cargo); W00 ran the suite green.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "teardown complete (stage objects released) within deadline" | :1-2 | REAL (pump-bounded form) | drop hooks + bounded loops; see F-001 |
| "a stopped-but-open session answers its frozen position" | :11-13 | REAL | :79-94 |

## 7. Dead / duplicated / unreachable code

None. Depends on `crate::host` (scenarios_b/host.rs — not my file).

## 8. Dependencies & flags

`commands`, `testkit` (fixture_path, serialize_tests), `crate::host`.
No feature gates.

## 9. Open questions for the next agent

- Whoever holds scenarios_b/host.rs: verify `arm_drop_hooks` actually
  hooks the real wired objects (not a parallel count that could pass
  while the real demuxer leaks).
