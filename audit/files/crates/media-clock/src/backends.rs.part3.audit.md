# AUDIT: crates/media-clock/src/backends.rs#part3 (chunk: lines 1059-1588 of 2117)
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-21 |
| Lines / functions | 530 audited / 18 units (fixture tail + 16 tests; chunk is entirely inside `#[cfg(test)] mod tests`, which starts at 1014) |
| File verdict (chunk) | REAL (test module: deterministic real-trait fixtures, exhaustive property test, behavioral chain tests; all green in the baseline workspace run) |
| Reachable from shipped entry point? | The module under test is: YES — `playback-engine/src/session.rs` `SessionRuntime.clock: Arc<MediaClock>` (session.rs:1410-1411), wired by `apps/cli` pipeline host; media-clock tests executed green in audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt (no media-clock test among its 7 failures). The tests themselves run in `cargo test` (same run: suite totals 4534/7/49). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `sed -n 1059,1588p` (full chunk, twice); `rg -n "mod tests\|fn \|impl "` (inventory with true line numbers); `rg -n "thread\|spawn" crates/media-clock/src/` (concurrency-test search — none); `sed -n 1014,1058p` (fixture definitions straddling the part2/3 boundary); Read-verification of quotes. |

## 1. One-paragraph verdict
This chunk is the backends test module: the master-selection matrix (pure
planner exhaustively over 640 wiring×loss×current cells), the behavioral
demotion chain (audio→video→free-running with position carry, epoch bumps,
FIFO typed events, lost-event accounting), external-before-video demotion
priority, stale/preemptive/idempotent loss reports, the single-master
no-fallback freeze, the Ended-latch interaction, determinism-by-rerun, and the
engine-initiated switch surface (typed refusals, unreadable-audio anchoring).
The fixtures implement the **real** traits (`crate::SampleCounterSource`,
`crate::MonotonicSource`) — deterministic test doubles, not mocks of the
subject. The tests assert observable behavior through the public API and would
fail under every obvious mutation I traced. Two hygiene notes only: zero
concurrent-access tests for a Mutex-guarded clock that production shares
across threads, and the chunk boundary slicing one test in half.

## 2. What actually works (verified)
— All media-clock tests pass in the no-fail-fast baseline: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt — suite totals 4534 passed / 7 failed / 49 ignored; the 7 failures (listed in its summary footer) are apps/desktop, tests/integration/first-vertical, crates/localization — none in media-clock.
— Fixtures are real-trait doubles: `impl crate::SampleCounterSource for FakeCounter` (backends.rs:1042-1049) and `impl crate::MonotonicSource for FakeMonotonic` (backends.rs:1064-1068) — verified by reading the straddling definitions.

## 3. Findings

### F-001 [S4]
Where: crates/media-clock/src/backends.rs:1015-1588 (whole test module); evidence: `rg -n "thread|spawn" crates/media-clock/src/` → only doc-comment hits in clock.rs:40 and lib.rs:33/106
Code:
```
    /// The pure planner, exhaustively over every wiring × loss
    /// state × current master (2^4 wirings × 2^3 loss sets × 5
    /// kinds = 640 cells): the plan is deterministic, never picks a
    /// lost or unwired kind, demotes exactly when the current
    /// master is lost and something remains, and never invents a
    /// master out of nothing.
```
What is wrong: not this test — the module around it. Every one of the 16+
tests in this chunk is single-threaded, yet the production clock is an
`Arc<MediaClock>` shared by the pipeline thread, the hotplug hooks (session.rs
documented "the hotplug hooks read the loss-time position") and the
presentation scheduler's `clock.snapshot()` per tick, with correctness
resting on "One short internal `Mutex` guards the anchor" (clock.rs:40).
Why it matters: the mutex discipline is claimed in prose and untested in
code — a lock-scope regression (e.g. reading the anchor outside the lock)
would pass this entire suite and only surface as a production race.
What correct looks like: one test spawning N threads hammering
`now()`/`on_master_lost`/`take_master_change` concurrently, asserting the
invariants that must hold (position monotonicity per master, event ring
consistency, no lost-update on the epoch).
Confidence: HIGH — Verification: read + rg (no thread/spawn test exists in the crate).

### F-002 [S4]
Where: crates/media-clock/src/backends.rs:1580-1588 (chunk boundary)
Code:
```
    fn video_master_snaps_within_the_bound_and_interpolates_between() {
```
What is wrong: the chunk boundary (part3 = 1059-1588) slices this test after
its setup lines; its body belongs to part4 (W03-A05, lines 1589+). Purely a
coverage-boundary note so the body's assertions are audited exactly once, by
the part4 owner.
Why it matters: boundary-adjacent assertions risk being audited by nobody.
What correct looks like: nothing to fix in code; part4 must cover
1580-1712's assertions.
Confidence: HIGH — Verification: read (test starts 1580; chunk ends 1588).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FakeMonotonic::advance_ns` + `impl MonotonicSource` (fixture tail; defs start 1052, part2) | 1059-1068 | REAL | — | real-trait deterministic double |
| `ts` / `backends` helpers | 1070-1078 | REAL | — | construction shorthands |
| `auto_picks_audio_when_wired` | 1081-1092 | REAL | — | master selection: audio preferred |
| `auto_picks_video_for_audioless_with_video` | 1095-1100 | REAL | — | video when no audio |
| `auto_picks_system_wall_when_only_wall` | 1103-1110 | REAL | — | wall fallback + position anchoring verified (1 s advance → 1 s now) |
| `external_master_is_explicit_and_reads_the_injected_source` | 1113-1138 | REAL | — | auto never picks External silently; explicit External anchors initial+delta (2 s + 0.5 s = 2.5 s) — the carry discipline |
| `unwired_initial_master_is_a_typed_refusal` | 1141-1163 | REAL | — | typed `Unsupported` refusal, stage Control — no guessed master |
| `fallback_plan_is_deterministic_over_the_full_matrix` | 1166-1235 | REAL | — | 640-cell exhaustive property test on the pure planner: un-lost current never switches; a plan never picks lost/unwired; None only when nothing available; first-in-DEMOTION_ORDER determinism — would fail under any planner mutation |
| `master_loss_chain_audio_video_free_runs_with_events` | 1238-1307 | REAL | — | behavioral: position carried across both demotions, epoch +1 each, wall interpolation after video switch, terminal free-run drift, FIFO event trail with exact from/to/reason/position/epoch/generation, `master_changes_lost()==0` |
| `lost_audio_demotes_to_wired_external_before_video` | 1311-1342 | REAL | — | external-before-video priority + chain continuation (external lost → video) |
| `non_current_loss_is_recorded_not_switched` | 1346-1374 | REAL | — | stale loss recorded and *spent* as later fallback target (audio loss then falls to wall, not the already-lost video) |
| `repeated_loss_reports_are_idempotent` | 1377-1397 | REAL | — | Recorded no-op against the new master |
| `single_master_clock_reports_no_fallback_and_holds` | 1400-1419 | REAL | — | NoFallback + frozen position ("frozen truth, not invented time") |
| `loss_while_ended_switches_but_stays_frozen` | 1422-1449 | REAL | — | Ended latch: demotion event still emitted, position stays frozen at 1 s despite 5 s wall advance |
| `loss_scripts_are_deterministic_across_identical_clocks` | 1452-1475 | REAL | — | `assert_eq!(run(), run())` — determinism by rerun with fresh identical fixtures; honest |
| `engine_switch_carries_position_and_records_the_reason` | 1480-1509 | REAL | — | EngineSwitch reason, position carried, idempotent Already, no spurious event |
| `engine_switch_refusals_are_typed` | 1512-1540 | REAL | — | NotWired / Lost typed errors; master unchanged |
| `switch_onto_unreadable_audio_awaits_first_usable_reading` | 1543-1577 | REAL | — | the device-not-open race (rate 0): position holds, `try_now()` errs, first usable reading anchors with no jump — covers a real audio-device-opens-late race |
| `video_master_snaps_within_the_bound_and_interpolates_between` (start only) | 1580-1588 | REAL | S4 | F-002 boundary; body in part4 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fallback_plan_is_deterministic_over_the_full_matrix` | YES | asserts four independent properties per cell (no-switch-unlost, never-lost/unwired pick, None-implies-nothing-available, first-in-order) — any planner regression trips at least one |
| `master_loss_chain_audio_video_free_runs_with_events` | YES | exact position/epoch/event-field assertions on both demotion steps |
| `switch_onto_unreadable_audio_awaits_first_usable_reading` | YES | asserts both the hold (try_now errs) and the anchor adoption (no jump: 3 s before and after) |
| `loss_scripts_are_deterministic_across_identical_clocks` | YES (if nondeterminism exists) | full-tuple equality of two runs' masters/events/epochs |
| all others | YES | exact public-API assertions; no tautologies found |
`.unwrap_or_else(|e| unreachable!("{e}"))` on construction is test-only panic-on-broken-precondition — acceptable.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "external … before video `[PROPOSAL in module docs]`" demotion priority | backends.rs:1309-1310 test doc | REAL (as tested behavior) | lost_audio_demotes_to_wired_external_before_video (1311) — but note the `[PROPOSAL]` marker: the priority is a proposal that this test freezes; production consumers should be aware it is not a doc-03-mandated order |
| "One short internal Mutex guards the anchor" (thread-safety claim) | clock.rs:40 | REAL-UNPROVEN | no concurrent test anywhere in the crate (F-001); single-writer usage in production is the actual guard |

## 7. Dead / duplicated / unreachable code
- None. Every test and fixture in the chunk is used; the matrix covers
  wall-unwired planner cells production cannot produce (ClockBackends::new
  always wires a wall) — harmless extra coverage for a pure function.

## 8. Dependencies & flags
- Test-only: `testkit::Must`, std atomics. No features. The `#[cfg(test)]`
  gate is correct — nothing test-shaped leaks into the production build.

## 9. Open questions for the next agent
- part4 (W03-A05): audit the video-steering test bodies (1580-1712) —
  especially whether the snap-bound assertions cover the boundary value
  exactly (at vs beyond the bound).
- Whoever owns clock.rs: does the "short internal Mutex" also guard
  `take_master_change`/EventRing, or only the anchor? F-001's severity depends
  on that answer.
