# AUDIT: crates/input/src/buffered.rs#part2 (chunk: lines 414-827 of 827)
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-21 |
| Lines / functions | 414 audited / 16 units (helpers + 14 tests; chunk is entirely `#[cfg(test)] mod tests`, which starts at line 418) |
| File verdict (chunk) | REAL (test module with real-trait observability fixtures; syscall-count, byte-exactness and typed-error assertions; green in the baseline workspace run) |
| Reachable from shipped entry point? | The code under test is: YES — `crates/demux` depends on `input` (demux/Cargo.toml:39) and `crates/demux/src/avi/riff.rs` uses `BufferedReader`; `apps/cli` depends on `input` (apps/cli/Cargo.toml:54). Executed in baseline: AVI first-vertical plays end-to-end (avi-firstvertical-open-session.txt) and MP4 demux reads work ("Demux works (2 tracks, duration, seekable)" — INDEX.md MP4 section). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `sed -n 414,827p` (full chunk, twice); `rg -n "#\[test\]\|fn \|const MAX"` (inventory, true line numbers); `rg -l "BufferedReader" crates/ apps/` (callers); `rg -n "with_default_capacity"` repo-wide (definition only — dead); `sed -n` ScriptSource definition + SyntheticStream in contract.rs (real-trait doubles verified); `sed -n 595,615p contract.rs` (mid-read fault path covered there). |

## 1. One-paragraph verdict
This chunk is the buffered-reader test module, and it is the strong kind:
fixtures implement the real `Seekable`/`RawRead` traits with I/O observability
(`ScriptSource` counts reads served and the largest ask seen; the inline
`Dribble` source pins the slow-pipe shape at 40 bytes per call), and the tests
assert **mechanics**, not vibes — exact syscall counts for window refills vs.
the 7-c bulk bypass, byte-exact contents, positions across buffer-hit seeks,
sticky clean EOF, failed seeks never moving, typed error codes with exact
`budget`/`attempted` arithmetic, capacity never growing, and truncation
detection with the (open-time, shrunk) pair. Every test I traced would fail
under the corresponding mutation. The only finding is a dead untested
constructor in the production half of the file (`with_default_capacity`, part1
range) that this module never touches.

## 2. What actually works (verified)
— All input-crate tests green in the no-fail-fast baseline: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt (4534/7/49; the 7 failures are in apps/desktop, first-vertical, localization — none in input).
— The demux path that consumes this reader executed in baseline: avi-firstvertical-open-session.txt (full lifecycle) and the MP4 smoke matrix (INDEX.md "Corrected MP4 smoke matrix" — demux/seek work through input).
— Fixtures are real-trait doubles: `impl Seekable for ScriptSource` / `impl RawRead for ScriptSource` (contract.rs:124/142) with `max_read_seen`/`reads_served`/`shrink_to`/`fail_next_read` observability; `SyntheticStream` wraps `BufferedReader<ScriptSource>` (contract.rs:163-166).

## 3. Findings

### F-001 [S3]
Where: crates/input/src/buffered.rs:108 (production half, part1's range) — coverage gap established from this chunk: no test in the module calls it
Code:
```
    pub fn with_default_capacity(source: S) -> Result<BufferedReader<S>, EngineError> {
```
What is wrong: `BufferedReader::with_default_capacity` has **zero callers
repo-wide** (rg over `crates/`, `apps/`, `tests/` finds only the definition)
and zero tests (none of the 14 tests in this module construct via it; they all
pass explicit capacities).
Why it matters: dead public API on a workspace-internal crate — the default
capacity it would pin is untested, so if anyone ever wires it, its behavior
(which capacity? validated against MAX_BUFFER_BYTES?) ships unverified.
What correct looks like: either delete it, or construct every test's reader
through it in at least one test to pin the default.
Confidence: HIGH — Verification: rg (definition-only match) + read of all 14
tests. (Flagged here because the *coverage* fact lives in this chunk; the
line itself belongs to W03-A02's part1 — cross-reference for their audit.)
Note: part1 may have already caught this; my chunk independently confirms the
absence of any test exercising it.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `pattern` / `reader` helpers | 427-433 | REAL | — | deterministic 251-cycle pattern; construction shorthand |
| `capacity_bounds_are_typed_errors` | 436-465 | REAL | — | zero capacity → `input.buffer.capacity-zero` Unsupported; over-ceiling → ResourceLimit with exact budget=MAX_BUFFER_BYTES(16 MiB), attempted=+1 |
| `read_requests_over_the_ceiling_are_typed_limit_errors` | 467-491 | REAL | — | over-ceiling read → `input.read.request-over-limit` with exact arithmetic; at-ceiling read proceeds and short-reads cleanly |
| `positions_and_readahead_are_exact` | 493-538 | REAL | — | positions across reads; **I/O counting**: one 64-byte refill serves a 10-byte ask (54 read-ahead), second read does no I/O, buffer-hit seek does no I/O, zero-length read does no I/O |
| `large_reads_bypass_the_window_in_one_source_call` | 540-562 | REAL | — | the 7-c bypass: 1000-byte ask through a 32-byte window = 2 source calls (bulk + refill), max ask = the caller's request (never speculative), buffer never grows — the superseding of the card-2.A1 "no chunk above capacity" pin is documented in the test doc |
| `bypass_reads_serve_leftover_read_ahead_first` | 564-589 | REAL | — | leftover window bytes drain first; direct ask is exactly the remainder (136); trailing refill hits EOF |
| `bypass_reads_survive_short_source_reads` | 591-640 | REAL | — | inline `Dribble` source (≤40 bytes/call): strict-progress re-asks, byte-exact across 8 dribbled bulk calls + 1 refill (calls==9), asks stay caller-sized |
| `bypass_read_past_end_short_reads_cleanly` | 642-651 | REAL | — | short result past EOF is data; sticky EOF after |
| `clean_eof_at_declared_length` | 653-675 | REAL | — | repeatable clean EOF; seek-to-length legal; one-past → `input.seek.out-of-range` and "failed seek never moves" |
| `shrink_mid_read_surfaces_typed_truncation` | 677-696 | REAL | — | shrink underneath → `input.file.truncated` Corrupt::Truncated with (expected=600 open-time, found=300 shrunk) |
| `non_zero_start_positions_are_respected` | 698-711 | REAL | — | seek-before-construct invariant |
| `read_into_reconstructs_and_bypasses_for_payload_asks` | 713-743 | REAL | — | borrow twin byte-exact, ONE bulk call + refill, Ok(0) only at EOS, zero-length ask does no I/O |
| `read_into_serves_small_asks_from_read_ahead` | 745-765 | REAL | — | small asks served from read-ahead (no second refill) — the Vec-path locality minus allocation |
| `read_exact_into_fills_or_fails_typed` | 767-799 | REAL | — | complete fill silent; past-end → Truncated(expected=250, found=200); shrink mid-read keeps the (open-time, shrunk) pair |
| `peek_into_never_advances_and_never_fabricates` | 801-826 | REAL | — | empty window peeks 0; peeks never advance, never do I/O; oversized peek returns exactly the window (120) — window-bound honesty |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `large_reads_bypass_the_window_in_one_source_call` | YES | asserts reads_served==2 (not 32) and max_read_seen==1000 — breaks both if the bypass regresses to window-chunking or over-reads speculatively |
| `positions_and_readahead_are_exact` | YES | I/O-count assertions per step — breaks on any refill-policy regression |
| `bypass_reads_survive_short_source_reads` | YES | calls==9 + byte-exactness — breaks if the dribble loop loses data or loops forever (test would hang) |
| `shrink_mid_read_surfaces_typed_truncation` | YES | exact (expected, found) pair — breaks if the reader reports the ask instead of the open-time length |
| `peek_into_never_advances_and_never_fabricates` | YES | position and I/O counters pinned around every peek |
| all others | YES | exact contents + counters + typed codes; no tautologies found |
Mid-read I/O *failure* propagation (not in this module) is covered in contract.rs:604-610 (`scripted_read_failure_propagates_typed`) through the same reader — cross-file coverage confirmed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the reader never asks the source for more than the caller's own request, and the window stays fixed" | test doc, buffered.rs:552-555 | REAL | max_read_seen==request asserted at 558-561 and 726-728; buffered_bytes ≤ capacity asserted |
| "Supersedes the card-2.A1 pin 'no chunk above capacity'" | test doc, buffered.rs:546-549 | REAL | the bypass (production `read_bypassing_window`, line 237, part1) + this test pair pin the new contract |
| "Ok(0) only at end of stream" | test doc, buffered.rs:735 | REAL | asserted twice at 736-739 + sticky-EOF loop at 657-661 |

## 7. Dead / duplicated / unreachable code
- F-001: `with_default_capacity` (line 108, part1 range) — dead, untested.
- Nothing in this chunk is dead; the Dribble fixture is used by exactly one
  test (inline, appropriate).

## 8. Dependencies & flags
- Test-only deps: `testkit::Must`/`assert_err`, `crate::contract::ScriptSource`.
- **Confirmed after writing §9's question:** `crates/input/src/lib.rs:65` declares `pub mod contract;` with NO cfg gate, and contract.rs's own `#[cfg(test)]` appears only at line 537 (its test module) — so `ScriptSource`/`SyntheticStream` (hand-rolled test doubles with `fail_next_read` fault injection) **ship in the production library**. That is an S3 defect on contract.rs/lib.rs (not this chunk's file) — flagged for whoever owns those files; this chunk's tests legitimately consume them.

## 9. Open questions for the next agent
- Owner of crates/input/src/contract.rs + lib.rs: the shipped test double above (S3) — is any production path constructing a SyntheticStream (a fake source) at runtime? If yes it escalates.
- part1 (W03-A02): `read_bypassing_window` (237) — the bypass loop's short-read progress guarantee is only as strong as this chunk's Dribble test; confirm no zero-progress infinite-loop path exists when a source returns Ok(0) mid-stream (the Dribble fixture never returns 0 before EOF; a source returning Ok(0) repeatedly mid-stream would spin forever unless the loop treats 0 as EOF).
