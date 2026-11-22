# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_06_repeated_seeks.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 236 / 1 `#[test]` fn + 1 helper struct |
| File verdict | REAL-UNPROVEN (the seek flow's behavioral storm proof) |
| Reachable from shipped entry point? | N/A (test file); it drives the exact seek path my seek.rs audit traced (`Command::PlaybackSeek` → `dispatch_seek` → the seven-step flow) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |
| Commands run | full-file read; cross-check of the seek-terminal accounting against the host's dispatch semantics (apps/cli/src/host.rs `dispatch_seek` pumps to terminal — my seek.rs audit §1); grep of the tolerance constants |

## 1. One-paragraph verdict

Doc 10 §6.4 row 6 as a real test: 100 deterministic (LCG-seeded, byte-stable) random absolute seeks over the 2 s parent, no waits, each dispatch awaiting its own terminal — per-seek asserts: accept, `seek-completed` increments by exactly one, `seek-failed` stays zero, the landing is within the fixture's 100 ms tolerance, and no frame delivered under this seek carries a stamp behind the landing (per-seek window over the capped ring, one 40 ms in-flight grace). Then the memory story, done honestly: the storm's live-byte growth is bounded by an explicit accounting of the *host's own* recording (amortized event log + 1024-entry ring + the deltas scratch), and a 5-seek trailing probe asserts the engine's seek path is allocation-steady (≤ 4096 bytes) — with a comment explaining exactly why a per-seek engine leak cannot hide there. The session ends honestly (`Ended`) and closes. This is the proof my seek.rs audit pointed to for the flow's behavior; combined with seek_flow.rs (my file #27), the seek machinery is the best-tested feature in the engine.

## 2. What was checked and how (zero-findings file, §13 requirement)

- Terminal accounting: `completed` must increase by exactly 1 per dispatch (130-134) and total `storm_baseline + STORM` (167-171) — a superseded/lost terminal would break the count. The warm-up baseline subtlety (session-cumulative counters) is documented at 79-90.
- Stale-output oracle: `ring[ring_before..]` filtered by `pts < landed×1000 − 40ms` (154-165) — the per-seek window is the correct scoping (a backward storm target legitimately puts earlier landings behind later ones; the comment at 102-109 explains why a whole-ring check would be wrong).
- Memory oracles: the storm bound (185-194) explicitly models the host's recording (2× events amortization, ring cap, scratch); the probe (203-224) is the tight per-seek leak detector. Both read as sound; the potential masking (an engine leak smaller than host-recording growth) is covered by the probe — and the file says so.
- LCG: Park–Miller constants, modulo target selection (minor bias irrelevant for a test); determinism claimed and structural.
- W00 baseline: green in the 1751-pass run — cited, not re-run (ADAPTATION).

## 3. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `TOLERANCE_US`/`STORM` consts | 25-27 | REAL-UNPROVEN | — | from the fixture's expectation TOML |
| `Lcg` + `next_u64`/`target_us` | 31-44 | REAL-UNPROVEN | — | deterministic, no deps |
| `hundred_random_seeks_complete_without_stale_output_or_growth` | 47-235 | REAL-UNPROVEN | — | warm-up → storm → tolerance → memory bound → probe → honest end → close |

## 4. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `hundred_random_seeks…` | YES | per-seek terminal count, zero-failure pin, landing tolerance, stale-frame filter, two independent memory oracles, terminal state |

## 5. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "100 random targets, no waits — no stale output ever; bounded memory; each completes or typed-fails" (doc 10 §6.4 row 6) | module docs 1-3 | REAL-UNPROVEN | all four properties asserted; green in W00 run |

## 6. Dead / duplicated / unreachable code

None.

## 7. Dependencies & flags

`commands`, `testkit` (fixture_path, serialize_tests, alloc_count); the scenarios_b host + `crate::alloc_state` (another agent's files). Always runs.

## 8. Open questions for the next agent

- `scenario_04_seek.rs` (the single-seek oracle this file borrows) and the host's `frame_ring`/`RING_CAP` — other agents' files; the ring's 1024×12B shape is asserted here and defined there.
- The 40 ms in-flight grace constant — where is it derived (quantum size)? If the pump quantum changes, the grace may need to follow; undocumented coupling.
