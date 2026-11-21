# AUDIT: crates/playback-engine/src/events.rs#part4 (lines 1927-2568)
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-21 |
| Lines / functions | 642 (chunk of 2569) / 7 tests + 5 helpers |
| Chunk boundaries | part4 = 1927-2568, the tail of `#[cfg(test)] mod tests` (test module starts 1545; parts 1-3 = the production code + earlier tests, other agents) |
| File verdict (chunk) | REAL — honest, adversarial tests; all executed in baseline |
| Reachable from shipped entry point? | N/A for a test chunk; the code under test (OrderingGuard, LatestCoalescer, BoundedEventQueue, CATALOG_EMISSIONS, StalenessStamp, wire_event_id) is playback-engine library code reachable from `lumen` CLI (apps/cli → playback-engine; OrderingGuard also consumed by contract suites). Tests executed: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3740-3868 `playback_engine` lib `test result: ok. 83 passed; 0 failed`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | sed -n on 108-110… quotes (all verified) · Read of impl halves (600-842 map+bridge, 920-1543 guard/coalescer/queue) to verify test-vs-impl agreement · rg `OrderingViolation` variants (18 in enum 852-918) vs planted scenarios (18/18 covered) · rg `debug_assert_wire_ordered` (src:1312 def; tests/events_contract.rs:481 only consumer) · read tests/events_contract.rs:682-847 (26-sample bijection test) · baseline orchrun log 3740-3930 (playback_engine lib 83/83 ok; events_contract 11 passed/11 ignored; `the_completeness_map_and_bridge_are_catalog_bijections ... ok`) · hand-simulation of the coalescer rate-cap loop (see §5) |

## 1. One-paragraph verdict
This chunk is the test half of the events card set, and it is the strongest test
code audited so far in this domain: the ordering-guard negative matrix plants
**all 18** `OrderingViolation` variants, each against a fresh guard with a
hand-built stream, each asserting the exact variant and payload; the
completeness-map test proves a bijection onto the generated catalog with a
hand-written `ENGINE_EMITTED_ROWS = 15` constant (not derived — not
tautological); the coalescer test hand-pins the ≤5 Hz rate cap over a virtual
second; the bounded-queue test proves the structural bound mid-storm with
oldest-first drop semantics. Every test ran and passed in the no-fail-fast
baseline. The only defects are hygiene: a stale "23-sample" count (the catalog
is 26), a debug_assert form whose contract-suite exercise is vacuous under the
release-profile baseline, and untested sliding-window eviction bounds.

## 2. What actually works (verified)
— All 18 enum variants reachable and asserted: `OrderingViolation` (events.rs:852-918) has 18 variants; `the_guard_catches_the_planted_violations` (2188-2480) plants SeqRegression, IllegalTransition, TickOutsidePlaying, TickAssumesUnknownDuration, MissingFatalBeforeError, FatalWithoutErrorState, EndOfMediaAfterEnded, EndOfTrackAfterMedia, FirstPlayingWithoutTracksList, FirstPlayingWithoutMediaInfo, EpisodeOpenedWithoutZero, EpisodeUnclosedAtExit, NonMonotonicProgress, ProgressOutOfBounds, TerminalWithoutGeneration, DuplicateSeekTerminal, EventBeforeItsReceipt, EpisodeNeverClosed — 18/18, each `assert_eq!` on the full variant value (except NaN, honestly handled via `matches!` at 2416-2419 with a comment saying why).
— Completeness arithmetic is not tautological: `ENGINE_EMITTED_ROWS` is a hand-written `pub const` (events.rs:638), so `assert_eq!(emitted, ENGINE_EMITTED_ROWS)` (1972) compares computed vs frozen — a reclassification fails the test. I recounted the map by hand (645-787): 15 Emitted + 11 WiredLater = 26 ✓.
— Rate-cap test is simulation-verified: hand-running the loop (impl 1386-1422: window 200 ms, offer-then-flush per 50 ms step) yields emissions at t=400/600/800/1000/1200 ms (values 0.91…0.99) = exactly 5 ≤ 5, strictly monotonic, `flush_now()` = Some(0.995) — the assertions at 2518-2527 pin real behavior, and a broken `permits` (never/always) fails earlier assertions (2492-2499).
— Bounded queue: 10_000 pushes, len asserted ≤ 256 **inside** the loop (2546-2549), dropped_total = 9744, survivors seq 9744..=9999 in order (2560-2561) — catches drop-newest and never-drop mutations.
— Staleness boundary math: is_stale_against (427-429 `epoch != … || counter != …`) vs the u64::MAX assertions (1941-1943) — hand-checked, all three correct.
— Baseline execution: orchrun log 3740-3868 — `playback_engine` lib: `test result: ok. 83 passed; 0 failed`, all chunk tests listed ok (`the_guard_catches_the_planted_violations`, `the_bounded_queue_stays_bounded_under_any_push_count`, `the_latest_coalescer_keeps_the_newest_value`, `the_completeness_map_partitions_the_catalog_exactly`, `the_wire_id_bridge_agrees_with_the_map`, `the_guard_accepts_a_legal_hand_built_stream`, `stamps_round_trip_through_pairs_and_envelopes`); contract binary `events_contract`: 11 passed / 11 ignored, including `the_completeness_map_and_bridge_are_catalog_bijections ... ok` (the 26-sample full proof, events_contract.rs:820-847).

## 3. Findings

### F-001 [S4] — stale "23-sample" count in the bridge test's contract
Where: crates/playback-engine/src/events.rs:1994-1997
Code:
```rust
    /// The `EventKind` ↔ `EventId` bridge: a representative sample
    /// maps onto the right ids, and every emitted row's sample lands
    /// in the emitted bucket (the full 23-sample bijection proof is
    /// the contract suite's job — `tests/events_contract.rs`).
```
What is wrong: The catalog has 26 rows (23 D-017 + the issue #42 additive
trio — asserted by this very chunk at line 1962 and by the contract suite at
events_contract.rs:823 `assert_eq!(samples.len(), 26, "D-017 23 + the issue
#42 trio")`). The comment still promises a "full 23-sample bijection proof".
events_contract.rs:14-15 carries the same stale "23" in its header. After the
trio landed, nobody updated the prose.
Why it matters: a reader auditing coverage will conclude 3 rows are unproven
(or was it 23 of 26?) — stale counts in test contracts breed exactly the
drift the bijection tests exist to prevent.
What correct looks like: "the full 26-sample bijection proof" in both files.
Confidence: HIGH — Verification: read (both files; counts asserted by the
tests themselves).

### F-002 [S4] — the `debug_assert` guard form is never exercised in a profile where it can fail
Where: crates/playback-engine/tests/events_contract.rs:481 (consumer of events.rs:1312-1321, the API under test)
Code:
```rust
                        debug_assert_wire_ordered(&mut self.debug_guard, &wire_event);
```
What is wrong: `debug_assert_wire_ordered` (events.rs:1312-1321) panics only
under `debug_assertions`; the contract suite's "second guard … through the
debug_assert form" (events_contract.rs:25-27) therefore proves nothing in the
cited baseline, which ran the whole workspace with `--release`
(audit/runs/baseline/INDEX.md §"Test suites were run with `--release`" and
orchrun log: binaries under `target/release/deps/`). In that profile the call
compiles to `guard.observe(event)` with the verdict discarded — a violation
would not fail the test. The wrapper's panic path has zero executed coverage
in any committed evidence.
Why it matters: the API is advertised as the engine's own self-check seam
("emission sites and hosts can sprinkle over their streams"); if someone
breaks the `debug_assert!` (e.g. swaps the condition), no run in the repo's
evidence ever notices.
What correct looks like: one unit test in this chunk that calls
`debug_assert_wire_ordered` on a violating stream under a debug-profile CI
lane (or `#[cfg(debug_assertions)]`-gated test that panics), or a REQUEST-ORCH-RUN debug-profile spot run.
Confidence: HIGH — Verification: read (release profile disables
debug_assertions; only consumer found by rg is events_contract.rs:481);
UNVERIFIED: whether any CI lane outside the baseline ran a debug-profile test
pass (none found in audit/runs/baseline/).

### F-003 [S4] — sliding-window eviction bounds (64 terminals / 128 receipts) never tested
Where: crates/playback-engine/src/events.rs:920-928 (consts exercised by this chunk's guard tests)
Code:
```rust
const SEEK_TERMINAL_WINDOW: usize = 64;
```
```rust
const ANSWERED_REQUEST_WINDOW: usize = 128;
```
What is wrong: The guard's bounded-memory eviction (`observe_receipt`,
events.rs:1071-1077, drains the front once len > window; same pattern for seek
terminals) is never exercised at its boundary: no test in this chunk (max 2
terminals, 0 receipts — the planted `event_before_receipt` uses no prior
receipt) and none in the contract suite (rg for 64/128-boundary scenarios:
none) pushes 65 distinct seek generations or 129 receipts. An off-by-one in
the drain arithmetic (`len > N` vs `>=`, drain count) would pass every
committed test.
Why it matters: the duplicate-terminal and receipt-ordering rules are D-2
bounded-memory invariants; the eviction path is exactly where a regression is
silent until a long session hits it.
What correct looks like: one test driving 65+ distinct terminals (asserting
the 65th is still caught as duplicate while the 1st is correctly evicted) and
129+ receipts.
Confidence: HIGH — Verification: read (rg over tests/ for boundary scenarios:
absent); mutation reasoning — eviction bug survives all existing tests.

## 4. Per-function table (chunk)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| tests: stamps_round_trip_through_pairs_and_envelopes | 1927-1944 | REAL | — | u64::MAX boundary assertions hand-verified against is_stale_against (427-429); would fail on any staleness-rule mutation |
| tests: the_completeness_map_partitions_the_catalog_exactly | 1951-1992 | REAL | S4 | bijection+order+split vs hand-written const (not tautological); F-001 stale comment on its neighbor; unreachable!-as-assertion for empty owners (1977-1983) is a real check |
| tests: the_wire_id_bridge_agrees_with_the_map | 1994-2069 | REAL | S4 | 8 representative samples incl. 2 WiredLater rows; full proof delegated (and delivered) by contract suite |
| helper: wire | 2071-2081 | REAL | — | bare wire event constructor |
| helper: state_changed | 2083-2085 | REAL | — | wrapper |
| helper: list_ready | 2087-2091 | REAL | — | minimal §4.3.4 open fact |
| helper: info_ready | 2093-2113 | REAL | — | minimal §3.2 open fact |
| helper: open_to_playing | 2115-2127 | REAL | — | legal 5-event prefix; note: emits no buffering bookends (legal — no episode opened), unlike the accept-test's richer stream |
| tests: the_guard_accepts_a_legal_hand_built_stream | 2129-2182 | REAL | — | 10-event happy path incl. 0.0/1.0 bookends, duration-known, pause-entry tick; `.must()` panics on any Err — catches over-strict guard |
| tests: the_guard_catches_the_planted_violations | 2184-2480 | REAL | S4 | 18/18 variants, fresh guard per scenario, exact-value asserts; F-003: eviction bounds untested |
| tests: the_latest_coalescer_keeps_the_newest_value | 2482-2533 | REAL | — | frozen window, latest-wins, stale-drop count, ≤5 Hz cap (hand-simulated = exactly 5), terminal flush, reset; catches permits-never and permits-always mutations |
| tests: the_bounded_queue_stays_bounded_under_any_push_count | 2535-2567 | REAL | — | mid-storm bound, oldest-first survivors, reuse-API append; catches drop-newest/never-drop mutations |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| stamps_round_trip… | YES | 3 boundary asserts cover not-stale/stale-epoch/stale-counter; any is_stale_against mutation trips one |
| the_completeness_map… | YES | len/order/split/owner-note checks against generated `EventId::ALL` + frozen consts; row removal, reorder, reclassify, empty owner all fail it |
| the_wire_id_bridge… | YES (per sample) | 8 exact mapping asserts; the remaining 18 mappings are covered by the contract bijection test (verified: events_contract.rs:820-847, passed in baseline) |
| the_guard_accepts… | YES | over-strict guard (false positive) → `.must()` panics |
| the_guard_catches… | YES | over-permissive guard → 18 exact-value `assert_eq!` fail; mis-sequenced checks (e.g. bounds vs episode-open ordering, seen in `out_of_bounds` expecting ProgressOutOfBounds not EpisodeOpenedWithoutZero) are pinned by expectation values |
| the_latest_coalescer… | YES | window never/always permitting both fail (2492-2499 / 2518-2521); latest-vs-first retention pinned by Some(0.9) at 2495-2499; reset pinned at 2532 |
| the_bounded_queue… | YES | len bound asserted per-push; survivors' seq range pins oldest-first; dropped_total pins the count |
Baseline proof all executed: orchrun log (playback_engine lib 83/83 ok). The
tests are not mocks of the subject — they drive the real library types.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the full 23-sample bijection proof is the contract suite's job" | events.rs:1996 | FALSE (count) | contract suite asserts 26 samples (events_contract.rs:823) — proof exists, count stale |
| "every emitted row's sample lands in the emitted bucket" | events.rs:1995-1996 | REAL | asserted at 2063-2067 for 6 emitted + 2 later samples; full 26-sample check in events_contract.rs:836-846 (passed, baseline) |
| "15 + 11 = 26 — the 23 D-017 rows + the issue #42 additive trio" | events.rs:1953-1954 | REAL | hand-recount of CATALOG_EMISSIONS (645-787): 15 Emitted, 11 WiredLater |
| "planted violations … are caught and named against the **real** stream shape" (contract suite header) | events_contract.rs:25-27 | REAL (plain-guard half) / vacuous in the cited run (debug_assert half) | 11 passed in baseline release profile; debug_assert form compiled out (F-002) |

## 7. Dead / duplicated / unreachable code
None in this chunk. (Cross-chunk note: `open_to_playing` duplicates the
accept-test's richer stream in weaker form — acceptable scenario scaffolding,
both used.)

## 8. Dependencies & flags
Tests use `testkit::Must`, `commands::catalog` types, `media_clock` via
super::*; all in dev/test scope. No feature gates. The whole test module runs
in the default workspace test lane (baseline #3/#4 equivalent: orchrun log).

## 9. Open questions for the next agent
- parts 1-3 auditor: `observe_buffering` (1164+) — confirm bounds-check
  before episode-open ordering (the `out_of_bounds` scenario depends on it).
- Whoever owns the guard: add the 64/128 eviction-boundary tests (F-003) and
  a debug-profile lane for the debug_assert form (F-002).
- events_contract.rs has **11 ignored tests** in the baseline run
  (orchrun log: `test result: ok. 11 passed; 0 failed; 11 ignored`) — the
  contract-suite auditor should enumerate why (feature-gated? platform?) and
  whether the ignore reasons are honest.
