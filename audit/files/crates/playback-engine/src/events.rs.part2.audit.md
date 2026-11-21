# AUDIT: crates/playback-engine/src/events.rs#part2 (lines 643-1284)
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-21 |
| Lines / functions | 642 (chunk 643-1284 of 2569) / 16 items in range |
| File verdict | UNREACHABLE (real, test-executed policy layer; no shipped entry point calls it) |
| Reachable from shipped entry point? | NO — `rg -ln "OrderingGuard\|is_engine_emitted\|wire_event_id\|emission_status"` over crates/apps/tests → only `crates/playback-engine/tests/events_contract.rs` and events.rs itself. Neither `lumen` CLI nor any host runs the guard. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `rg -ln OrderingGuard` → tests-only; `rg -n "EventKind::SessionBuffering"` → zero constructions in src (engine emits `EngineEventKind::Buffering` at session.rs:1667/1678/1713/2494, converted in event.rs); `sed -n '2485,2496p' session.rs` → in-episode emission is strictly-increasing + 5 Hz-gated (matches guard rule); `sed -n '819,851p' events_contract.rs` → bijection test; `sed -n '1305,1320p' events.rs` → debug_assert_wire_ordered body |

## 1. One-paragraph verdict
This chunk is the wire-event emission catalog (26 rows, 15 Emitted / 11 WiredLater — count verified by hand and pinned by a bijection test) and the `OrderingGuard` doc-05 §3/§4 policy checker. The logic is real, internally consistent with the engine's actual emission discipline (traced: strict monotonic buffering progress, 0.0/1.0 bookends, seq monotonicity), and exercised against real scripted-session output by `tests/events_contract.rs`, which passed in the no-fail-fast baseline (`orchrun-test-workspace-nofailfast.log.txt`: no playback-engine binary among the 7 failures). But none of it is reachable from a shipped binary — it is contract-test infrastructure — and it carries three hygiene defects: an asymmetric NaN policy on tick fractions, a doc comment claiming engine-internal self-checks that do not exist, and partial state mutation on error paths.

## 2. What actually works (verified)
- Catalog/test bijection: `the_completeness_map_and_bridge_are_catalog_bijections` (events_contract.rs:819-851) asserts 26 rows, 15 emitted + 11 later, `EventId::ALL` order-agreement — my hand count of the in-range table (rows at 645-787) gives exactly 15 Emitted (SessionStateChanged, SessionBuffering, TimingTimeTick, TimingDurationKnown, TimingSeekCompleted, TimingSeekFailed, TracksListReady, TracksChanged, PlaybackEndOfTrack, PlaybackEndOfMedia, PlaybackError, AudioDeviceLost, AudioDeviceChanged, VideoBackendChanged, MediaInfoReady) and 11 WiredLater — matches.
- WiredLater claims spot-verified: `rg "EngineEventKind::(OpeningProgress|RateChanged)"` over src/apps → no emission sites (consistent with "no set-rate engine input exists", 703); AudioDeviceLost/Changed ARE emitted (hotplug.rs:196,281 via push_session_event) matching their Emitted status — cross-checked against my hotplug.rs assignment.
- Guard rules match engine discipline: session.rs:2491 emits in-episode buffering only when `fill > self.emissions.buffering_last` (strictly increasing) — so the guard's `progress <= self.episode_progress → NonMonotonicProgress` (1184) can never fire on real engine output; bookends 0.0/1.0 emitted at session.rs:1667/1678/1713.
- `wire_event_id` (813-842) is a compiler-exhaustive match — a new EventKind variant breaks the build, enforced by the bijection test. Verified by reading.
- The contract suite runs the guard over real session output: harness at events_contract.rs:454-481 feeds every wire event through `OrderingGuard` and `debug_assert_wire_ordered`; `finish_clean` (565-570) asserts `guard.finish()` and zero collector drops. These 22 tests passed in baseline (`orchrun-test-workspace-nofailfast.log.txt` — playback-engine not among failing binaries).

## 3. Findings

### F-001 [S4]
Where: crates/playback-engine/src/events.rs:1156-1161
Code:
```rust
        // §4.3.4 — a tick that assumes the duration (a nonzero
        // fraction) must follow `duration-known`.
        if position > 0.0 && !self.duration_known {
            return Err(OrderingViolation::TickAssumesUnknownDuration);
        }
```
What is wrong: `observe_tick` validates no bounds on `position` at all: NaN (`NaN > 0.0` is false) and negative fractions pass the guard silently in any legal state. The sibling `observe_buffering` explicitly bans NaN and enforces `0.0..=1.0` (1166: `progress.is_nan() || !(0.0..=1.0).contains(&progress)`), citing the same §3.2 vocabulary. Asymmetric policy for two float-carrying rows of the same catalog.
Why it matters: the guard is the documented differential checker for hosts; a host or future engine path emitting a NaN/negative tick fraction would pass validation. Today the engine cannot produce it (traced: `position_fraction` session.rs:1765-1772 clamps 0.0..1.0 with a `duration_ns > 0` guard), so this is latent.
What correct looks like: `if position.is_nan() || !(0.0..=1.0).contains(&position) { return Err(...) }` plus the existing duration rule.
Confidence: HIGH — Verification: read; engine-side impossibility traced via `sed -n '1762,1772p' session.rs`.

### F-002 [S4]
Where: crates/playback-engine/src/events.rs:937-944
Code:
```rust
/// — the differential check that keeps the policy and the engine
/// honest about each other (D-5). The [`debug_assert_wire_ordered`]
/// form wraps [`OrderingGuard::observe`] for engine-internal
/// self-checks: zero cost in release builds, a named violation in
/// debug builds.
```
What is wrong: the comment claims the `debug_assert_wire_ordered` form exists "for engine-internal self-checks". `rg -n "debug_assert_wire_ordered"` over crates+apps: the ONLY call site is `tests/events_contract.rs:481`. No engine-internal (src/) caller exists — the shipped engine never runs the guard on its own output, not even in debug builds.
Why it matters: a reader concludes debug builds self-check wire ordering; they do not. The claimed safety net is test-only.
What correct looks like: either wire `debug_assert_wire_ordered` into the session's emit path under `#[cfg(debug_assertions)]`, or rewrite the comment to "for the contract suite's debug-assert harness".
Confidence: HIGH — Verification: `rg -n "debug_assert_wire_ordered"` (definition events.rs:1312 in part3 range; sole external caller the test).

### F-003 [S4]
Where: crates/playback-engine/src/events.rs:1019,1062-1065
Code:
```rust
        self.last_seq = Some(event.seq);
```
and
```rust
            _ => Ok(()),
        };
        self.prev_was_seek_completion = matches!(event.kind, EventKind::TimingSeekCompleted { .. });
        verdict
```
What is wrong: `observe` mutates `last_seq` (1019) and `prev_was_seek_completion` (1064) even when the returned verdict is `Err`, while the state-machine half (`observe_state_change`) deliberately refuses to mutate `state` on its error paths. A host that logs the violation and continues feeding events runs a guard with a moved seq baseline but stale flags (e.g. `pending_fatal` stays true forever after the first error). Also asymmetric: the `SeqRegression` path (1011-1018) returns BEFORE updating `last_seq`, so exactly the one violation about seq leaves the stale baseline in place.
Why it matters: the guard's usefulness after a first violation is undocumented and inconsistent; partial-mutation discipline differs per rule.
What correct looks like: pick one semantics (all-or-nothing per observe) and document it; simplest is to keep mutating (the host owns recovery) but then also update `last_seq` on the regression path.
Confidence: MEDIUM — Verification: read (all three mutation sites traced; no test feeds events after an Err to pin intended semantics — none of the 22 contract tests does).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `CATALOG_EMISSIONS` (const table) | 645-787 | REAL | — | 26 rows; 15 Emitted / 11 WiredLater — count verified against bijection test (events_contract.rs:846-851); WiredLater owners are honest pointer strings (spot-verified two, §2) |
| `emission_status` | 791-796 | REAL | — | Linear scan, 26 rows — fine; test-only caller |
| `is_engine_emitted` | 800-802 | REAL | — | Test-only caller |
| `wire_event_id` | 813-842 | REAL | — | Compiler-exhaustive bridge; bijection-tested; test-only caller |
| `OrderingViolation` (enum, 16 variants) | 852-918 | REAL | — | Each variant carries its spec citation; used in Display |
| `SEEK_TERMINAL_WINDOW` / `ANSWERED_REQUEST_WINDOW` (consts) | 924,928 | REAL | — | Bounded memory (D-2), documented |
| `OrderingGuard` (struct) | 949-979 | REAL | — | Plain state; 12 fields |
| `OrderingGuard::new` | 984-1000 | REAL | — | Correct initial state (Idle, nothing seen) |
| `OrderingGuard::observe` | 1008-1066 | REAL | S4 | Seq + receipt checks then dispatch; F-003 partial mutation; fall-through `_ => Ok(())` documented (1004-1007) |
| `OrderingGuard::observe_receipt` | 1071-1077 | REAL | — | Sliding window 128, O(n) contains — receipts are rare, documented bound |
| `OrderingGuard::finish` | 1083-1091 | REAL | — | Dangling-episode + dangling-fatal checks; called by finish_clean in tests |
| `OrderingGuard::observe_state_change` | 1093-1143 | REAL | — | Illegal-transition/fatal-bookend/episode-bookend/eom/first-playing checks — all trace to doc rules; no mutation on Err (the good half of F-003's asymmetry) |
| `OrderingGuard::observe_tick` | 1145-1162 | REAL | S4 | F-001 (no NaN/bounds check); immediate-tick exceptions tracked via prev_was_seek_completion |
| `OrderingGuard::observe_buffering` | 1164-1192 | REAL | — | NaN ban + 0.0 open / 1.0 close bookends + strict monotonicity — engine-compatible (traced, §2) |
| `OrderingGuard::observe_seek_terminal` | 1194-1210 | REAL | — | Generation required, duplicate window 64 — correct |
| `Default for OrderingGuard` | 1213-1217 | REAL | — | Delegates to new() — correct |
| `Display for OrderingViolation` (head; continues into part3) | 1221-1284 | REAL | — | Spec-citation strings; read through 1284, continues past chunk boundary |

(Chunk boundaries: `EmissionStatus`/`CatalogEmission` types and `EventId` live in part1 (A09); `debug_assert_wire_ordered` (1312) and the coalescer/tests live in part3/part4 (A06/A05). I read the definition of debug_assert_wire_ordered only to verify F-002.)

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `the_completeness_map_and_bridge_are_catalog_bijections` (events_contract.rs:819-851) | YES | Deleting/duplicating a catalog row, flipping a status, or reordering breaks the exact-count/order asserts (26/15/11 pinned) |
| `every_engine_kind_lands_on_an_emitted_row_or_stays_private` (853+) | YES | An engine kind mapping to a WiredLater row or leaking a private kind fails |
| happy-path / stop / seek / end-of-track / damage tests (974-1387+, 22 total) | YES | Each feeds real scripted sessions through the guard; a guard rule that never fires would surface as an untested variant — e.g. no test covers NaN tick (F-001) or post-Err feeding (F-003) |
| Baseline execution: passed — `orchrun-test-workspace-nofailfast.log.txt` (playback-engine not in the 7 failing binaries; 4534/7/49 totals) | — | cited |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "a catalog change that does not update the map fails the build rather than drifting silently" | comment 643-644 + doc 05 | REAL | Exhaustive match (813-842) + bijection test (819-851) — compile-time + test-time enforcement, both verified |
| "hosts and the contract suites run the guard over what actually crossed the wire" | comment 939-941 | HALF-FALSE | Contract suites: YES (events_contract.rs:454-481). Hosts: NO — no host (apps/cli, apps/web, apps/desktop) references OrderingGuard (rg over crates/apps/tests) |
| "for engine-internal self-checks" (debug_assert_wire_ordered) | comment 941-943 | FALSE | F-002: sole caller is the test harness |
| "progress events are monotonic within one episode" (§3.4.2) | doc 05, guard 1183-1189 | REAL | Engine side enforces strict increase (session.rs:2491 `fill > buffering_last`) — guard rule can only fire on foreign input |
| AudioDeviceLost/Changed "Emitted … tests/hotplug.rs proves the payloads live" | comment 706-708 | REAL | hotplug.rs:196,281 emit via push_session_event (verified in my hotplug.rs audit) |

## 7. Dead / duplicated / unreachable code
- The entire chunk's public surface (catalog, bridge, guard) is unreachable from shipped binaries — test-only. This is its designed role (contract infrastructure), but `pub` + dead-in-production means the shipped `lumen` binary carries none of it (not even the debug self-check the docs claim).
- No duplicated logic found in range.

## 8. Dependencies & flags
- No feature gates, no unsafe, no allocation beyond the two bounded Vec windows (push/drain amortized).
- Depends on `PlaybackState::can_transition_to` (state.rs, other agent) for IllegalTransition — the guard's correctness there is only as good as that table.

## 9. Open questions for the next agent
- Q-1 (part3/A06 owner): does any test pin the intended semantics of feeding the guard after an Err (F-003)? If the contract suite never does it, pick semantics and document.
- Q-2 (session.rs owner): is the guard ever intended to run inside the engine under debug builds (F-002's claim)? If yes, wire it; if no, fix the comment.
- Q-3: `EventId::ALL` order is asserted equal to catalog order (bijection test) — the catalog's doc-table order is load-bearing; any EventId reorder must update 3 places (this table, the bridge, ALL). Noted for future catalog rows.
