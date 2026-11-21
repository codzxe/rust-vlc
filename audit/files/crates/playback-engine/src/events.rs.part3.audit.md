# AUDIT: crates/playback-engine/src/events.rs#part3 (lines 1285–1926)
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-21 |
| Lines / functions | 642 (chunk L1285–1926 of 2569) / 26 rows |
| File verdict | REAL (with one dead-code component) |
| Reachable from shipped entry point? | Split: `debug_assert_wire_ordered`/`LatestCoalescer`/`BoundedEventQueue` have NO shipped caller (see F-001/F-003); the subjects under test (TimeTickPolicy, ProgressWindow, DamageCoalescer, StalenessStamp, DrainStaleness) are live via `session.rs:74` `use crate::events::{DamageCoalescer, DrainStaleness, EventPolicy, ProgressWindow, TimeTickPolicy}` → `Session`/`Engine` → apps/cli `lumen`. Call chain for the live half: lumen CLI → `SessionConfig` → `session.rs:1452` `buffering_window: ProgressWindow::for_buffering()` / `:1455` `DamageCoalescer::new()`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "LatestCoalescer|BoundedEventQueue|debug_assert_wire_ordered"` over crates/ and apps/ (usage map — LatestCoalescer: only its own unit test; BoundedEventQueue/debug_assert: only tests/events_contract.rs); `rg -n "ProgressWindow|BUFFERING_WINDOW_MS"` over src+apps (real buffering path is raw ProgressWindow); `rg -n "events::tests::"` in nofailfast baseline (all chunk tests green, log :3765–3803); `sed -n` on every quoted range. |

Chunk boundary note: part3 = L1285–1926: tail of `fmt::Display for OrderingViolation` (1221–1300; arms from `ProgressOutOfBounds` on), `debug_assert_wire_ordered` (1302–1321), `LatestCoalescer` (1323–1460), `BoundedEventQueue` (1462–1542), then `mod tests` L1544–1922 — tests for part1/part2 subjects (TimeTickPolicy, ProgressWindow, DamageCoalescer) plus the M11 epoch-staleness suite. Part4 (A05) takes L1927+ (`stamps_round_trip_through_pairs_and_envelopes` onward).

## 1. One-paragraph verdict
The test half of this chunk is genuinely strong: the M11 epoch-staleness suite (`cross_epoch_is_stale_by_construction_whatever_the_counter`, `seek_terminal_exemption_is_intra_epoch_only`, and the differential `the_stamp_rule_agrees_with_the_cancel_token_oracle` against the independently-written `CancelToken`) pins the exact masquerade shapes a bare generation-counter comparison misses, and all of it executed green in the no-fail-fast baseline (log :3765–3803, :3803). The production half is two small utilities and one dead policy: `LatestCoalescer` — 100+ lines implementing latest-wins drop-coalescing with a doc that promises "the newest fact is never lost to the window" — is wired to NOTHING: the shipped `session.buffering` path uses raw `ProgressWindow` (session.rs:1378/1452/1673), and the struct's own `[PROPOSAL]` admits it is "a handoff, not wired here". The module documents a coalescing policy the shipped binary does not have. `BoundedEventQueue` and `debug_assert_wire_ordered` are real but test-consumer-only in-repo.

## 2. What actually works (verified)
— Epoch-staleness rule (epoch mismatch ⇒ stale regardless of counter; intra-epoch terminal exemption; receipts never suppressed): `StalenessStamp::is_stale_against`/`DrainStaleness::is_stale_at_drain` executed green in baseline (nofailfast :3767, :3770, :3780, :3783, :3803) and differentially cross-checked against `crate::epoch::CancelToken` over a 3×4×3×4 matrix (L1901–1922) — two independent implementations agreeing.
— Time/progress/damage window boundary semantics (250 ms tick floor 50 ms, 200 ms buffering window, 1 s damage window, terminal flush unconditional): tests pin exact ns boundaries (L1594–1595, L1605–1608, L1619–1621) — off-by-one mutations fail; green in baseline (:3766, :3768, :3772, :3778, :3782).
— `BoundedEventQueue` drop-oldest-at-capacity, O(1), counted: implementation read (L1504–1510), used by the contract suite collector (tests/events_contract.rs:460, 627, 674), bounded-population test at :3773.

## 3. Findings
### F-001 [S3]
Where: crates/playback-engine/src/events.rs:1345-1351
Code:
```
/// `[PROPOSAL]` — the doc's "coalesced to their rate cap" admits a
/// lossier reading (drop until the next sample arrives); the
/// keep-the-latest form is this policy layer's choice, and the
/// engine's in-episode `session.buffering` path
/// ([`crate::session`], the `EmissionState` fields) can adopt it as
/// a one-structure swap — recorded as a handoff, not wired here
/// (scope discipline: the emission site is sibling-owned).
```
What is wrong: `LatestCoalescer` (L1355–1460, incl. `for_buffering`) is dead code in the shipped binary. rg over crates/ and apps/ shows its only caller is its own unit test (`the_latest_coalescer_keeps_the_newest_value`, L2487); it is not even re-exported at crate root (lib.rs:118–121 omits it). The live `session.buffering` emission path uses raw `ProgressWindow` (session.rs:1378 `buffering_window: ProgressWindow`, :1452, :1673), which is the "lossier reading" — a window-denied progress value is dropped with no pending/latest retention.
Why it matters: The module's documentation (L1341–1343: "the **latest** offered value is the one that emits … the newest fact is never lost to the window") describes a guarantee the shipped engine does not provide; a host author reading events.rs would believe `session.buffering` keeps the latest value. ~100 lines of maintained, tested, exported code with zero production callers is maintenance surface for nothing.
What correct looks like: Either wire `LatestCoalescer` into `EmissionState` (the "one-structure swap" the comment defers) or delete it and fix the docs to state the shipped policy.
Confidence: HIGH — Verification: read + rg usage map (crates/, apps/ both searched).

### F-002 [S4]
Where: crates/playback-engine/src/events.rs:1549-1559
Code:
```
    fn the_default_policy_is_constructible_and_readable() {
        // Regression shape for the wasm32 trap (issue #49, found by
        // card 7.I1's browser matrix): `SessionConfig::new`
        // constructs `EventPolicy::default_policy()` transiently, so
```
What is wrong: The test is named and commented as the regression pin for the wasm32 trap, but on the native target it exercises `InstantSource` (L566–572); the wasm32 `ZeroMonotonic` path (L574–600) is never compiled or executed by the workspace suite. Mutating `ZeroMonotonic::now_ns` to `panic!()` leaves this test green.
Why it matters: The wasm32 half of the fix is UNVERIFIED by the in-repo suite; the real evidence is deferred to `docs/web/browser-matrix-*.json` (not re-verified here — no wasm target can run under the audit's no-build rule).
What correct looks like: A `#[cfg(target_arch = "wasm32")]` companion test (wasm-pack runner), or a CI lane that builds the wasm target.
Confidence: HIGH — Verification: read (L566–600 cfg split); wasm execution UNVERIFIED (blocked: no builds allowed).
### F-003 [S4]
Where: crates/playback-engine/src/events.rs:1466-1479
Code:
```
/// This is the policy twin for the engine's *edges*: hosts, test
/// collectors and future bridges that accumulate wire events must be
/// as bounded as the engine feeding them — an unbounded collector on
/// a bounded channel is a leak with extra steps. The contract suite
/// (`tests/events_contract.rs`) collects its streams through this
/// queue, so the policy is proven on every scenario it runs.
```
What is wrong: `BoundedEventQueue` (and `debug_assert_wire_ordered`, L1311–1321) have no consumer outside `tests/events_contract.rs` — no shipped host (apps/cli, apps/web, desktop) accumulates events through them. The doc positions them as host-facing policy, but every shipped host currently ignores them.
Why it matters: "Hosts must be as bounded as the engine" is a policy that exists only in a doc comment; the actual hosts are unproven against it.
What correct looks like: Wire the CLI/web event sinks through `BoundedEventQueue` (or drop the "hosts" framing and call it a test utility).
Confidence: HIGH — Verification: rg over apps/ (no matches) + crates/ (test-only matches).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Display for OrderingViolation` (tail: `ProgressOutOfBounds` … `EventBeforeItsReceipt` arms) | 1282–1300 | REAL | — | Spec-citation strings only; matches enum arms 1:1 (enum at L851–921, part2). |
| `debug_assert_wire_ordered` | 1311–1321 | REAL | S4(F-003) | `#[inline]`; release builds still run `observe` but discard the verdict (documented L1309–1310); test-only caller. |
| `LatestCoalescer::new` / `for_buffering` | 1366–1380 | REAL-UNPROVEN (dead in shipped path) | S3(F-001) | Correct construction; 200 ms ⇒ ≤5 Hz. |
| `LatestCoalescer::offer` | 1386–1410 | REAL-UNPROVEN | S3(F-001) | Monotonic gate vs `last_emitted` (strict `<=` drop, counted); window-denied value retained as `pending` via `max` — traced: `pending > last_emitted` invariant holds (pending only set from values that passed the monotonic gate); no NaN handling — a NaN offer fails both `value <= last` (false) and compares false against pending max → NaN can become pending and emit; unreachable from the queue-fill domain (always finite) but unguarded. |
| `LatestCoalescer::flush_if_due` | 1415–1422 | REAL-UNPROVEN | S3(F-001) | Idle-caller short-circuit correct (permits never consulted with nothing pending). |
| `LatestCoalescer::flush_now` | 1426–1431 | REAL-UNPROVEN | S3(F-001) | Unconditional terminal flush; window untouched (documented terminal exception). |
| `LatestCoalescer::reset` | 1437–1441 | REAL-UNPROVEN | S3(F-001) | Fresh window + baseline; lifetime `dropped_stale` retained (documented). |
| `LatestCoalescer::pending`/`dropped_stale`/`window_ms` | 1444–1459 | REAL-UNPROVEN | — | Trivial accessors. |
| `BoundedEventQueue::new` | 1494–1500 | REAL | — | NonZeroUsize capacity — zero-cap refused at type level. |
| `BoundedEventQueue::push` | 1504–1510 | REAL | — | Drop-oldest at capacity, counted, O(1); no allocation after warm-up (VecDeque ring). |
| `BoundedEventQueue::drain_into` | 1514–1516 | REAL | — | Caller-owned Vec reuse (D-1). |
| `BoundedEventQueue::len`/`is_empty`/`capacity`/`dropped_total` | 1520–1541 | REAL | — | Trivial accessors. |
| `tests::the_default_policy_is_constructible_and_readable` | 1548–1563 | REAL | S4(F-002) | Would fail if the NATIVE default source regressed; cannot catch the wasm32 regression it is named for. |
| `tests::the_interval_is_floored_at_50ms` | 1565–1580 | REAL | — | Floor + u32::MAX passthrough pinned; mutation: remove floor → fails. |
| `tests::a_fresh_policy_only_sets_the_baseline` | 1582–1589 | REAL | — | No immediate tick on play (doc 05 §3.2) pinned. |
| `tests::ticks_are_due_only_at_the_interval` | 1591–1600 | REAL | — | Exact ns boundary 249_999_999/250_000_000 + backwards-position clamp pinned. |
| `tests::a_frozen_source_permits_exactly_one_progress_emission` | 1602–1609 | REAL | — | 200 ms window boundary pinned. |
| `tests::damage_coalesces_then_flushes_on_the_window` | 1611–1627 | REAL | — | Window reopen at 1 s, demux-first ordering, fresh window after flush pinned. |
| `tests::terminal_flush_answers_pending_counts_and_reopens` | 1629–1640 | REAL | — | Unconditional terminal flush + reopen pinned. |
| `tests::pair`/`stamped_event`/`receipt` (helpers) | 1653–1678 | REAL | — | Test vocabulary builders; `stamped_event` hard-codes `seq: 0` (drain decisions don't read seq — verified against `is_stale_at_drain` impl in part1). |
| `tests::cross_epoch_is_stale_by_construction_whatever_the_counter` | 1685–1732 | REAL | — | The same-counter masquerade (both directions), the higher-counter leak, the gen-0-vs-gen-0 reopen, and per-kind drain agreement — mutation: drop the epoch compare in `is_stale_against` → fails. |
| `tests::fresh_epoch_fresh_counter_passes` | 1735–1746 | REAL | — | Near misses both directions pinned. |
| `tests::supersede_then_push_storm_all_dropped_counted_bounded` | 1754–1807 | REAL | — | 10k mixed-kind storm, all suppressed, 10 receipts survive, nothing retained; the drain loop is hand-rolled ("the hub's own shape") — subject is `is_stale_at_drain` (real method), so mutation of the decision fails; a hub-side regression would not be caught here (hub covered by events_contract.rs). |
| `tests::seek_terminal_exemption_is_intra_epoch_only` | 1813–1877 | REAL | — | Seeked/SeekFailed survive same-epoch supersede, siblings don't, cross-epoch always stale — both directions pinned. |
| `tests::track_selection_terminal_joins_the_intra_epoch_exemption` | 1884–1894 | REAL | — | TrackSelectionChanged terminal exemption pinned. |
| `tests::the_stamp_rule_agrees_with_the_cancel_token_oracle` | 1900–1922 | REAL | — | Differential test vs independently-written `CancelToken` over 3×4×3×4 matrix — the D-5 anti-echo-chamber check; strongest test in the chunk. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| the_default_policy_is_constructible_and_readable | NO (for its named subject) | Native-only execution cannot see the wasm32 path it claims to regress (F-002). |
| the_interval_is_floored_at_50ms | YES | Floor removal or clamping bug flips asserts. |
| a_fresh_policy_only_sets_the_baseline | YES | Immediate-tick-on-baseline regression flips asserts. |
| ticks_are_due_only_at_the_interval | YES | Off-by-one at the boundary or backwards-position handling flips asserts. |
| a_frozen_source_permits_exactly_one_progress_emission | YES | Window-advance bug double-permits or over-delays. |
| damage_coalesces_then_flushes_on_the_window | YES | Early/late flush or lost counts flip asserts. |
| terminal_flush_answers_pending_counts_and_reopens | YES | Conditional terminal flush returns None. |
| cross_epoch_is_stale_by_construction_whatever_the_counter | YES | Bare-counter comparison (the bug class this exists for) fails the leak assert. |
| fresh_epoch_fresh_counter_passes | YES | Over-eager staleness (fresh flagged stale) fails first asserts. |
| supersede_then_push_storm_all_dropped_counted_bounded | YES (for `is_stale_at_drain`) | Suppression-count and receipt-survival asserts are exact. |
| seek_terminal_exemption_is_intra_epoch_only | YES | Exemption extended cross-epoch or denied intra-epoch flips asserts. |
| track_selection_terminal_joins_the_intra_epoch_exemption | YES | Both directions asserted. |
| the_stamp_rule_agrees_with_the_cancel_token_oracle | YES | Any divergence between the two independent implementations fails with a named pair. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the newest fact is never lost to the window" (LatestCoalescer semantics) | events.rs L1341–1343 | UNREACHABLE (the struct has no shipped caller) | rg: only caller is its own unit test; session uses raw `ProgressWindow` (session.rs:1452). |
| "hosts, test collectors and future bridges … must be as bounded as the engine" | events.rs L1474–1477 | PAPER-ONLY (for hosts) | No host uses `BoundedEventQueue` (rg over apps/ empty). |
| wasm32 default-source trap regression pinned | events.rs L1550–1558 comment | REAL-UNPROVEN (native half only) | Native asserts green in baseline (:3775); wasm half not compiled by the suite. |
| Chunk tests pass | test suite | REAL | nofailfast baseline :3765–3783, :3803 — all green. |

## 7. Dead / duplicated / unreachable code
- `LatestCoalescer` entire impl (L1355–1460): dead in shipped binary (F-001).
- `BoundedEventQueue` + `debug_assert_wire_ordered`: live API, test-only consumers (F-003).
- No duplication found within the chunk.

## 8. Dependencies & flags
- `cfg(target_arch = "wasm32")` split for the throttle source (L566–600) — the only flag surface near this chunk; workspace builds compile the native arm (baseline build #1 CLEAN). `NonZeroUsize`, `VecDeque` std-only. No new deps.

## 9. Open questions for the next agent
- To W03-A05 (events.rs#part4): `the_latest_coalescer_keeps_the_newest_value` (L2487) tests dead code — see F-001; also please confirm `emission_status`/`CATALOG_EMISSIONS` completeness claims (L638–808, part2) against the wire catalog, since the contract suite depends on them.
- To the session owner (W03 session.rs agent): whether `EmissionState.buffering_window` dropping denied values without latest-retention is acceptable for the host UX (progress can skip values mid-episode but converges by episode bookends 0.0/1.0 — L34 module doc).
- NaN note (from `offer` analysis): a NaN `f32` offer would be retained as pending and emitted (`NaN <= last` is false); unreachable from queue-fill (finite) but worth a debug_assert.
