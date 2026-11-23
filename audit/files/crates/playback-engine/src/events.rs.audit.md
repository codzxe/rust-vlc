# AUDIT: crates/playback-engine/src/events.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-23 |
| Lines / functions | 2568 / 60 prod + 20 tests (+7 test helpers) |
| File verdict | REAL-UNPROVEN (core policies real+wired; ~40% of the module is test-only exported API) |
| Reachable from shipped entry point? | PARTIAL — YES for TimeTickPolicy/ProgressWindow/DamageCoalescer/EventPolicy/StalenessStamp/DrainStaleness (chain: apps/cli/src/host.rs:91 `use playback_engine::{... SessionConfig ...}` → session.rs:480/503/1452/2491/2499/2651/748 → this file); NO for OrderingGuard, debug_assert_wire_ordered, LatestCoalescer, BoundedEventQueue, wire_event_id, emission_status, is_engine_emitted, CATALOG_EMISSIONS (zero consumers outside `cargo test`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 3 |
| Commands run | `./target/debug/deps/playback_engine-30ee2bde67c1e3d9 events::tests` -> 20 passed, 0 failed, exit 0 (binary 18:07 newer than source 17:35, so current); Grep callers across apps/ + crates/ (see F-001..F-003); sed quote verification |

## 1. One-paragraph verdict
This is the event *policy* layer, not the emission layer — emission lives in session.rs/hotplug.rs, and the hostile question resolves FAVORABLY for the wired half: the four policies the engine actually runs (tick interval, buffering rate window, damage coalescing, epoch-aware drain staleness) are real, are consumed by session.rs production code (session.rs:74 imports, :748 `event.is_stale_at_drain(current)`), and the 15/26 emission-completeness map is accurate — every "Emitted" row has a real production emission site (session.rs:1574/1585/1613/1659/1667/1682/1732/2006/2310/2477/2536/2565/2654/2777/2792, hotplug.rs:196/281/313/392), and the 11 "WiredLater" rows are exactly the 11 W02-A08 found never emitted catalog-side (opening-progress, rate-changed, screenshot-saved, playlist.changed, setting-changed, plugins.loaded/failed/crashed, system.log, devices.volume-added/removed). The dishonest half is that the module's flagship artifacts — OrderingGuard, LatestCoalescer, BoundedEventQueue, the emission-map query API — have ZERO production consumers; they are `cargo test` scaffolding exported as public API, while the module doc claims "hosts and the contract suites run the guard over what actually crossed the wire" — no host does (grepped apps/ and crates/: only tests/events_contract.rs).

## 2. What actually works (verified)
- `./target/debug/deps/playback_engine-30ee2bde67c1e3d9 events::tests` -> `test result: ok. 20 passed; 0 failed` (exit 0). Includes the completeness-map bijection test (asserts 15 emitted / 11 later / 26 total against generated `EventId::ALL`) and the staleness oracle test (stamp rule vs `CancelToken::is_stale` across a 3×4×3×4 matrix).
- Production wiring verified by grep: session.rs:748 `if event.is_stale_at_drain(current) {` (drain suppression), session.rs:2491 `buffering_window.permits(now)`, session.rs:2499 `damage.flush_if_due(now)`, session.rs:2651 `tick.is_due(Some(last), position)` — the policies run on every poll of the real engine, which lumen-cli constructs (apps/cli/src/host.rs:91-93).
- Engine emission sites for all 15 "Emitted" rows verified by grep (see §1); cross-ref W02-A08's 11/26 never-emitted list matches this file's `WiredLater` bucket exactly, including the TracksChanged divergence (engine map says emitted since 3.A3; catalog ledger still 14/12 — engine is right: session.rs:2310/2715 emit `TrackSelectionChanged`, event.rs:583-588 maps it to wire `EventKind::TracksChanged`).

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/src/events.rs:1356-1460 (LatestCoalescer)
Code:
```
pub struct LatestCoalescer {
    window: ProgressWindow,
    last_emitted: Option<f32>,
    pending: Option<f32>,
    dropped_stale: u64,
}
```
What is wrong: zero production callers. Grep over apps/ and crates/ (excluding this file) finds LatestCoalescer only in its own unit test (events.rs:2488) — session.rs implements in-episode buffering coalescing with `ProgressWindow` directly (session.rs:2491), not through this type. The module doc admits it: "the engine's in-episode `session.buffering` path ... can adopt it as a one-structure swap — recorded as a handoff, not wired here" (1348-1351). Additionally `offer()` (1386-1410) has a NaN hole the sibling OrderingGuard explicitly bans (§3.2 0.0..=1.0): `value <= last` is false for NaN, so a NaN offer is neither dropped-as-stale nor rejected — if the window permits it is EMITTED (`Some(NaN)`) and `last_emitted = Some(NaN)` disables the monotonic filter for every subsequent offer (`value <= NaN` is always false). Why it matters: dead policy code whose documented adoption path ("one-structure swap") would silently ship the NaN hole into the live buffering stream. What correct looks like: either delete it, or ban NaN at the top of `offer()` mirroring OrderingGuard::observe_buffering (1164-1167). Confidence: HIGH — Verification: read + grep (no run; dead code has no live path).

F-002 [S3]
Where: crates/playback-engine/src/events.rs:1484-1542 (BoundedEventQueue)
Code:
```
/// This is the policy twin for the engine's *edges*: hosts, test
/// collectors and future bridges that accumulate wire events must be
/// as bounded as the engine feeding them
```
What is wrong: no host, bridge, or production edge uses it — the only consumer is tests/events_contract.rs:460/627/674 (test collector). The doc framing ("hosts ... must be as bounded") implies an ecosystem adoption that does not exist; the actual hosts (apps/cli run loop) drain-and-print per poll and accumulate nothing. Why it matters: exported API that looks like load-bearing infrastructure but is test scaffolding; a reader auditing "is the event path bounded end-to-end" is misdirected. What correct looks like: gate it `#[cfg(test)]`/doccfg or state plainly "test-only today, no host uses it". Confidence: HIGH — Verification: read + grep (apps/**/*.rs and crates/**: zero matches outside playback-engine tests).

F-003 [S3]
Where: crates/playback-engine/src/events.rs:937-944 (OrderingGuard doc), :813-842 (wire_event_id), :789-802 (emission_status/is_engine_emitted)
Code:
```
/// emission sites ( [`crate::session`] ) are the production
/// carriers; hosts and the contract suites run the guard over what
/// actually crossed the wire — the differential check that keeps the
/// policy and the engine honest about each other (D-5).
```
What is wrong: "hosts ... run the guard" is false — no host (apps/cli, apps/desktop, apps/control-server, apps/web) references OrderingGuard or debug_assert_wire_ordered (grepped). The entire differential-check apparatus (guard, debug_assert form, wire_event_id bridge, emission-map query fns) executes only inside `cargo test` (tests/events_contract.rs:71-72, 454-481). The D-5 "escape the echo chamber" claim is true only of the test suite, never of shipped builds — a shipped engine that violates §3.3/§4.3 ordering ships silently. Why it matters: doc claims runtime honesty-checking that does not exist in any shipped lane. What correct looks like: wire debug_assert_wire_ordered into session.rs emit (it is zero-cost in release by its own doc, 1305-1307) or fix the doc. Confidence: HIGH — Verification: read + grep.

F-004 [S4]
Where: crates/playback-engine/src/events.rs:14-17 (module doc)
Code:
```
//! lives in [`crate::session`] (sibling-owned); the policy is
//! defined here so the hub's check is a one-line consumer — the
//! exact `session.rs` wiring line is registered as a Lead-merge
//! follow-up in the Wave-1 worklog.
```
What is wrong: stale — the wiring landed (session.rs:748 consumes `is_stale_at_drain`; worklog archive-2026-10.md:4236 "Applied W1-10's Lead wiring verbatim"). The doc still tells the reader the wiring is pending. Confidence: HIGH — Verification: read + grep session.rs:748.

F-005 [S4]
Where: crates/playback-engine/src/events.rs:246
Code:
```
                if now_ns.saturating_sub(opened) >= self.window_ms * 1_000_000 {
```
What is wrong: `self.window_ms * 1_000_000` is unchecked u64 multiplication on a value from the public constructor `ProgressWindow::new(window_ms: u64)`; window_ms > ~1.8e13 overflows (panic in debug, wrap in release — a wrapped product can make a huge window compare smaller than elapsed, opening every call). All real callers pass 200/1000 (BUFFERING_WINDOW_MS/DAMAGE_WINDOW_MS), so blast radius is a pub-API foot-gun only. (Contrast: TimeTickPolicy::is_due:196 correctly widens via `i64::from(...)`.) What correct looks like: `u128` compare or `window_ms.checked_mul(1_000_000)`. Confidence: HIGH — Verification: read.

F-006 [S4]
Where: crates/playback-engine/src/events.rs:928,1071-1077
Code:
```
const ANSWERED_REQUEST_WINDOW: usize = 128;
...
    pub fn observe_receipt(&mut self, request: RequestId) {
        self.answered_requests.push(request);
```
What is wrong: the EventBeforeItsReceipt check (1021-1025) consults a 128-entry sliding window; a legitimately-answered request whose event arrives after >128 later receipts would false-positive as a violation. Documented D-2 tradeoff and unreachable in the engine's emit-immediately-after-receipt discipline — and the guard is test-only today (F-003), so no shipped impact. Confidence: HIGH — Verification: read.

## 4. Per-function table (production items; tests in §5)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| TimeTickPolicy::from_config_ms | 167-177 | REAL | — | floor clamp verified by test (ran) |
| TimeTickPolicy::default_policy / Default | 179-206 | REAL | — | 250ms |
| TimeTickPolicy::is_due | 191-199 | REAL | — | ns/ms units correct; i64::from widening safe; wired session.rs:2651 |
| ProgressWindow::new | 218-226 | REAL | S4 | F-005 overflow via pub ctor |
| ProgressWindow::for_buffering | 228-232 | REAL | — | 200ms; wired session.rs:1452 |
| ProgressWindow::permits | 239-254 | REAL | S4 | F-005 |
| ProgressWindow::open_if_closed | 260-264 | REAL | — | damage window anchor |
| ProgressWindow::window_ms | 267-271 | REAL | — | |
| DamageCoalescer::new/Default | 287-296,350-354 | REAL | — | wired session.rs:1455 |
| DamageCoalescer::record | 298-311 | REAL | — | opens window on first pending; saturating adds |
| DamageCoalescer::is_pending | 313-317 | REAL | — | |
| DamageCoalescer::flush_if_due | 319-328 | REAL | — | wired session.rs:2499 |
| DamageCoalescer::flush_all | 330-338 | REAL | — | wired session.rs:1740 |
| DamageCoalescer::take_pending | 340-347 | REAL | — | resets window |
| StalenessStamp::new/from_pair/epoch/counter | 386-413 | REAL | — | Copy, no alloc; oracle-tested (ran) |
| StalenessStamp::is_stale_against | 415-429 | REAL | — | pair rule; wired via drain |
| DrainStaleness (trait) | 444-465 | REAL | — | consumed session.rs:748 |
| impl DrainStaleness for EngineEvent | 467-507 | REAL | — | cross-epoch stale by construction; intra-epoch terminal + session-scoped exemption |
| EventPolicy::default_policy + Debug | 515-559 | REAL | — | wasm32 zero-clock trap avoidance documented; wired session.rs:503 |
| default_throttle_source (both cfg) | 566-589 | REAL | — | wasm arm UNVERIFIED at runtime (no wasm run here; doc cites browser-matrix) |
| EmissionStatus::is_emitted | 617-623 | REAL | — | data only |
| emission_status | 791-796 | UNREACHABLE(prod) | S3 | F-003: test-only consumers |
| is_engine_emitted | 798-802 | UNREACHABLE(prod) | S3 | F-003 |
| wire_event_id | 813-842 | UNREACHABLE(prod) | S3 | F-003: exhaustive match, but no prod caller |
| OrderingGuard::new/Default | 981-1000,1213-1217 | UNREACHABLE(prod) | S3 | F-003 |
| OrderingGuard::observe | 1002-1066 | UNREACHABLE(prod) | S3 | seq/receipt/fan-out; logic verified by negative-matrix test (ran) |
| OrderingGuard::observe_receipt | 1068-1077 | UNREACHABLE(prod) | S4 | F-006 window |
| OrderingGuard::finish | 1079-1091 | UNREACHABLE(prod) | S3 | dangling invariants |
| OrderingGuard::observe_state_change | 1093-1143 | UNREACHABLE(prod) | S3 | family/episode logic; test-verified |
| OrderingGuard::observe_tick | 1145-1162 | UNREACHABLE(prod) | S3 | negative fraction not checked (position<0 passes) — gap |
| OrderingGuard::observe_buffering | 1164-1192 | UNREACHABLE(prod) | S3 | NaN correctly banned here (contrast F-001) |
| OrderingGuard::observe_seek_terminal | 1194-1210 | UNREACHABLE(prod) | S3 | bounded window |
| Display for OrderingViolation | 1221-1300 | UNREACHABLE(prod) | S3 | spec-citation strings |
| debug_assert_wire_ordered | 1311-1321 | UNREACHABLE(prod) | S3 | only caller events_contract.rs:481 (a test) |
| LatestCoalescer::new/for_buffering | 1363-1380 | STUB(dead) | S3 | F-001 |
| LatestCoalescer::offer | 1382-1410 | STUB(dead) | S3 | F-001 NaN hole |
| LatestCoalescer::flush_if_due/flush_now/reset | 1412-1441 | STUB(dead) | S3 | F-001 |
| LatestCoalescer::pending/dropped_stale/window_ms | 1443-1459 | STUB(dead) | S3 | F-001 |
| BoundedEventQueue::new/push/drain_into/len/is_empty/capacity/dropped_total | 1490-1542 | UNREACHABLE(prod) | S3 | F-002; logic itself correct, test-verified (ran) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the_completeness_map_partitions_the_catalog_exactly (1955-1992) | YES | asserts 26 == EventId::ALL.len(), order, 15 emitted, 11 later, non-empty owners — ran, passes |
| the_wire_id_bridge_agrees_with_the_map (1998-2069) | PARTIAL | 8 of 26 samples only; full bridge proof delegated to events_contract.rs (comment says so) |
| the_guard_catches_the_planted_violations (2187-2480) | YES | plants all 18 violation names incl. NaN; ran, passes |
| the_latest_coalescer_keeps_the_newest_value (2486-2533) | YES (for the dead type) | would fail if latest-wins or ≤5Hz broke — but nothing production uses the type (F-001) |
| the_bounded_queue_stays_bounded_under_any_push_count (2539-2567) | YES (for the dead type) | 10k pushes, bound + drop count + survivor order asserted |
| supersede_then_push_storm_... (1754-1807) | YES | 10k storm, 10 receipts survive, all events suppressed — but it hand-rolls the hub loop, it does not call session.rs's OutputHub |
| the_stamp_rule_agrees_with_the_cancel_token_oracle (1900-1922) | YES | differential vs independently-written CancelToken (ran) |
| the_default_policy_is_constructible_and_readable (1548-1563) | WEAK | asserts b >= a on two reads — near-tautology on real clock; value is the wasm32 contract note |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "15 of 26 rows are engine-emitted today" (109-115) | module doc | REAL | all 15 emission sites grep-verified (session.rs/hotplug.rs, §1); map bijection test ran |
| 11 rows "honestly owned by later cards" (113) | module doc | REAL | zero production emission paths for those 11 kinds (grep EngineEventKind constructors; matches W02-A08 catalog ledger) |
| "hosts and the contract suites run the guard" (940-941) | OrderingGuard doc | FALSE | no host references OrderingGuard (grep apps/); only events_contract.rs |
| "the exact session.rs wiring line is registered as a Lead-merge follow-up" (16-17) | module doc | FALSE (stale) | wiring landed: session.rs:748 |
| OUTBOX_CAPACITY=1024 / RECEIPT_CAPACITY=4096 / time_ticks_replaced counter (44-48) | module doc | REAL | session.rs:98/109/566/623/776 |
| "emission audit: Emitted rows are proven by the contract suite tests/contract_suite.rs" (76-78) | module doc | REAL-UNPROVEN | contract_suite.rs exists (not audited by me — next agent); events_contract.rs drive-site claims verified by grep |

## 7. Dead / duplicated / unreachable code
- LatestCoalescer — dead in production (F-001).
- BoundedEventQueue — test-only (F-002).
- OrderingGuard + debug_assert_wire_ordered + wire_event_id + emission_status + is_engine_emitted + CATALOG_EMISSIONS/ENGINE_EMITTED_ROWS/EmissionStatus/CatalogEmission — test-only public API (F-003). The map DATA is accurate; its consumers are all tests.
- Duplication: LatestCoalescer vs session.rs's ProgressWindow+`buffering_last` path (session.rs:2491) — two implementations of the ≤5Hz rule, one live, one dead.

## 8. Dependencies & flags
Imports: commands (EventId/EventKind/PlaybackState/WireEvent/Generation/RequestId), media_clock::MonotonicSource, crate epoch/event. `#[cfg(target_arch = "wasm32")]` arms (574-589) are for the wasm lane; the release lane (`cargo build --release -p lumen-cli -p desktop`) builds the native arm (verified by baseline cargo-build-release-lumen-cli.txt exit 0). No features gate this module (lib.rs:98 `pub mod events;` unconditional).

## 9. Open questions for the next agent
- tests/contract_suite.rs and tests/events_contract.rs: do the "emitted" rows get driven through the REAL pipeline (real demux/decode ticks) or through scripted inputs? (events_contract.rs:23 says "scripted" — the D-5 honesty of that suite needs its own audit; assigned elsewhere.)
- W02 ledger divergence (tracks.changed 14 vs 15) — engine map is right; the commands-side ledger fix is still open (cross-ref audit/files/crates/commands/tests/catalog_emission_ledger.rs.audit.md F-002).
- If session.rs ever adopts LatestCoalescer per the handoff note, F-001's NaN hole becomes an S2 on the live buffering stream.
