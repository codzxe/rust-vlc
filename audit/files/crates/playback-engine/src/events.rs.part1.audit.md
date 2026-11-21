# AUDIT: crates/playback-engine/src/events.rs (part1, lines 1–644 of 2568)
> Chunk scope: part1 = lines 1–644 (assignments.tsv LOC: **part1 642** /
> part2 642 / part3 642 / part4 642; the 642-line cut lands inside the
> doc comment of `CATALOG_EMISSIONS` at 640-644 — audited through 644 so
> the doc comment stays with its const; the `CATALOG_EMISSIONS` map
> itself (645-787), `emission_status`, `is_engine_emitted`,
> `wire_event_id`, the `OrderingGuard`, the `LatestCoalescer` and the
> `mod tests` belong to A10's part2 / A06's part3 / A05's part4).
> Contains: the module-doc event policy (§3.4 rules 1–4 + the 26-row
> emission-audit table), the four window constants, `TimeTickPolicy`,
> `ProgressWindow`, `DamageCoalescer`, `StalenessStamp`, the
> `DrainStaleness` trait + `EngineEvent` impl, `EventPolicy` + the
> wasm32 default-source split, `EmissionStatus`, `CatalogEmission`,
> `ENGINE_EMITTED_ROWS`.
> Tooling note: bash-tool output corrupts the literal `#[must_use]`
> (ANSI-stripping eats `[m`); all quotes below were verified via the
> Read tool, which renders it correctly.

| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-21 |
| Lines / functions | 644 (of 2568) / 30 production units |
| File verdict | REAL (policy types executed in the shipped open lane; every module-doc claim I could ground was verified against its consumer) |
| Reachable from shipped entry point? | YES, EXECUTED-ADJACENT. `EventPolicy`/`TimeTickPolicy` are constructed by the shipped CLI host (apps/cli/src/host.rs:976-978 `config.throttle = playback_engine::EventPolicy { tick: TimeTickPolicy::default_policy(), … }`) and ride every `lumen open` (baseline `mp4-smoke-open-session.txt`/`avi-firstvertical-open-session.txt` show the event stream these policies shape: state-changed/tracks/timing events). `DrainStaleness::is_stale_at_drain` is the OutputHub's drain check (session.rs:748). The web host injects its `PumpClock` through the same seam (apps/web/src/host.rs:97, 178-191). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | Read tool over lines 1-777 + 1580-1719 (full range + tests that prove it); `rg -n "is_due|last_tick_ns" crates/playback-engine/src/session.rs` (caller protocol at 2649-2662 — None arm never reaches is_due); `rg -n "throttle|EventPolicy|PumpClock" apps/web/src/host.rs` (injection doctrine honored); `rg -n "time_ticks_replaced|OUTBOX_CAPACITY|RECEIPT_CAPACITY|is_stale_at_drain" crates/playback-engine/src/session.rs` (doc claims 93/98/109/566/623/748 verified); `sed -n '590,704p' session.rs` (hub implements oldest-event-drop/receipt-pinning/tick-replacement exactly as the module doc claims); `rg -n "EventId::ALL" crates/commands/src/catalog/ids.rs` (ALL.len()==26 asserted at ids.rs:280); `sed -n '1,40p;179,190p' crates/commands/tests/catalog_emission_ledger.rs` (the 14/12 vs 15/11 ledger drift); `grep -n "events::tests::" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (14 events tests ok at 3765-3778, incl. every part1 policy test); `grep -n "hotplug" …nofailfast.log` (hotplug suite ok at 3897-3906) |

## 1. One-paragraph verdict
This is policy-as-data done with unusual discipline: every doc-05 rate rule is a named type, every constant carries its spec citation, the wasm32 `Instant::now` panic trap is documented with the issue number that found it (#49) and structurally fixed by a cfg-split default, the epoch-aware staleness rule kills the bare-counter masquerade its own docs name, and the 26-row emission table is machine-checked by a bijection test that passes in the new no-fail-fast baseline (3765-3778). I traced every load-bearing module-doc claim to its consumer and found no lie: the hub really pins receipts while dropping oldest events (session.rs:599-643), really replaces resident ticks and counts it (614-627), really applies `is_stale_at_drain` at drain (748), the web host really injects its pump clock, and the catalog really has 26 event ids. What remains are three hygiene defects: `is_due`'s doc comment describes a side effect the pure function cannot have (the None-baseline protocol lives in the caller), the public `ProgressWindow::new` can overflow its ns window in a debug build, and the repo carries two committed emission ledgers that disagree (engine 15/11 vs catalog 14/12) — a drift the module doc itself confesses as a handoff but which both suites happily assert into permanence.

## 2. What actually works (verified)
— `TimeTickPolicy::is_due` interval math — verified by trace + test: `saturating_sub` handles backward positions; `i64::from(u32) * 1_000_000` cannot overflow (max ≈4.3e15 < 9.2e18); `ticks_are_due_only_at_the_interval` pins 249_999_999/250_000_000/backward — passing at baseline 3765-region.
— The media-position (not wall-time) tick judgment — verified: session.rs:2647-2662 reads `clock_position_ns` and owns the None-baseline protocol itself; the frozen-clock-deterministic claim holds (no position advance → no tick).
— `ProgressWindow` tumbling semantics — verified: first `permits` opens+permits; re-anchored only on a permitted emission; `a_frozen_source_permits_exactly_one_progress_emission` pins it (baseline ok).
— `DamageCoalescer` window-opens-at-first-damage — verified: `record`'s `was_pending` guard opens via `open_if_closed` (the documented fix of the original flush-check-opens-window bug); `damage_coalesces_then_flushes_on_the_window` + `terminal_flush_answers_pending_counts_and_reopens` pin both flush paths (baseline ok).
— `StalenessStamp::is_stale_against` — verified: epoch compare first (cross-epoch stale regardless of counter — the masquerade killer), then counter `!=` (conservative fail-safe documented); `cross_epoch_is_stale_by_construction_whatever_the_counter` + `the_stamp_rule_agrees_with_the_cancel_token_oracle` + `stamps_round_trip_through_pairs_and_envelopes` pin it (baseline ok at 3767/3770/3771).
— The intra-epoch terminal exemption — verified: `seek_terminal_exemption_is_intra_epoch_only` (baseline 3770) + the survives-match at 499-505 (terminals + non-pipeline-scoped kinds).
— wasm32 default-source split — verified: `#[cfg(not(target_arch = "wasm32"))]`/`#[cfg(target_arch = "wasm32")]` pair at 566-589; the panic narrative matches std's documented wasm32 `Instant` behavior; the web host always injects (host.rs:97).
— Module-doc rule-4 boundedness claims — verified against session.rs:93-109 (constants 1024/4096) and 599-704 (pin/drop/replace logic + `compact_if_due` tombstone reclamation keeping memory ~2× live).
— The 15/11/26 emission arithmetic — verified: `the_completeness_map_partitions_the_catalog_exactly` asserts bijection+order+counts (baseline 3774) against `EventId::ALL.len()==26` (commands/catalog/ids.rs:280).

## 3. Findings

### F-001 [S4]
Where: crates/playback-engine/src/events.rs:185-199 (doc comment vs. signature)
Code:
```rust
    /// Whether a tick is due, judged on media position (nanoseconds
    /// — see the module docs for the rate-1.0 equivalence note):
    /// `None` baseline means "started, not yet ticked" and only
    /// *sets* the baseline (doc 05 §3.2 lists no immediate tick on
    /// play; the immediate cases are seek-completion, pause and rate
    /// change).
    #[must_use]
    pub fn is_due(&self, last_emitted_ns: Option<i64>, position_ns: i64) -> bool {
        match last_emitted_ns {
            None => false,
```
What is wrong: The doc says the `None` case "only *sets* the baseline" — but `is_due` takes `&self` and `last_emitted_ns` **by value**; it is pure and sets nothing. The None-baseline protocol is implemented by the caller (session.rs:2649-2650: `None => self.emissions.last_tick_ns = Some(position)`), which never actually calls `is_due` with `None`. The comment describes the surrounding protocol as if it were this function's behavior — a reader implementing a second consumer from this doc alone would wait for a baseline-setting that never happens (their first tick would never fire if they passed `None` through, as the doc implies the function handles it).
Why it matters: This family's selling point is spec-precision; the one function whose doc misstates its own effect is the one future hosts will copy for a second clock. The shipped caller is correct, so no runtime defect — pure doc rot on a load-bearing seam.
What correct looks like: "`None` means *no baseline yet* — the caller records the first position itself and never asks with `None` (see `Session::translate`, which owns the baseline protocol); this function answers `false` defensively."
Confidence: HIGH — Verification: read (signature + both call sites: session.rs:2651 is the only `is_due` call in the tree — `rg -n "is_due"` over the crate).

### F-002 [S4]
Where: crates/playback-engine/src/events.rs:239-254 (permits), 221-226 (the pub constructor)
Code:
```rust
            Some(opened) => {
                if now_ns.saturating_sub(opened) >= self.window_ms * 1_000_000 {
                    self.opened_ns = Some(now_ns);
                    true
                } else {
                    false
                }
            }
```
What is wrong: `self.window_ms * 1_000_000` is unchecked `u64` multiplication in a `pub` API: `ProgressWindow::new(window_ms)` with `window_ms > u64::MAX/10⁶ ≈ 1.8·10¹³` overflows — panic in debug builds, wrap in release (a wrapped threshold makes the window near-unreachable or always-open depending on the value). In-tree callers pass only `BUFFERING_WINDOW_MS` (200) and `DAMAGE_WINDOW_MS` (1000), so nothing shipped can trip it.
Why it matters: Public constructor + silent wrap on absurd input is exactly the "unbounded/unchecked arithmetic on caller-controlled values" class the checklist names; the type is exported for hosts to build custom policies (`ProgressWindow::new` is `pub` and `#[must_use]`).
What correct looks like: `const fn ns(window_ms: u64) -> u64 { window_ms.saturating_mul(1_000_000) }` used at both sites (221/246 — the same product appears in the `TimeTickPolicy` sibling too, which is already safe via i64), or store `window_ns` up front with the conversion checked once.
Confidence: HIGH — Verification: read (arithmetic traced; both in-tree constructors verified constant-fed).

### F-003 [S4]
Where: crates/playback-engine/src/events.rs:117-126 (module doc confessing the drift); the disagreeing ledger at crates/commands/tests/catalog_emission_ledger.rs:26-37, 179-183
Code (events.rs, the confession):
```rust
//! cross-checked by the catalog-side 23-row ledger
//! (`crates/commands/tests/catalog_emission_ledger.rs` — still
//! carrying the pre-3.A3 11/12 split for `tracks.changed`; a
//! handoff for its owner)
```
and the other ledger's own arithmetic (catalog_emission_ledger.rs:26-28):
```rust
//! Arithmetic (post-3.A1, post-3.B4, post-7.C1): **14 emitted / 12
```
What is wrong: Two committed, test-enforced ledgers of the same fact disagree: the engine-side map says 15 emitted / 11 later (this file, `ENGINE_EMITTED_ROWS = 15`, bijection test), the commands-side ledger says 14 emitted / 12 later (its own test asserts `WIRED_BY_LATER.len() == 12`). Both suites pass in the no-fail-fast baseline because each asserts only its own copy — the contradiction is enforced, not caught. The engine side is authoritative (contract suites drive the rows); the catalog side is stale by exactly the 3.A3 `tracks.changed` flip the comment names.
Why it matters: The emission table is the project's event-completeness truth; two divergent machine-checked counts of the same 26 rows is drift with a test harness blessing each half. The confession comment makes it honest, which is why this is S4 and not a matrix-inflation finding — but "a handoff for its owner" with both owners' tests green is how the drift becomes permanent.
What correct looks like: One ledger. Either the commands-side test reads `playback_engine::events::ENGINE_EMITTED_ROWS` (dependency direction may forbid it — then delete the duplicated count from the ledger test and cite the engine map), or the engine map moves into the commands crate where the catalog lives and both suites consume it.
Confidence: HIGH — Verification: read (both files, both tests, both passing lines in `orchrun-test-workspace-nofailfast.log.txt`: events bijection at 3774; ledger suite in the commands run).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| module doc (policy §3.4 + emission table) | 1-126 | REAL | S4 (F-003 cross-ledger) | Every load-bearing claim grounded: hub pin/drop (session.rs:599-643), tick replace (614-627), constants (98/109), injection (web host.rs:97), 26 ids (ids.rs:280). 15/11/26 arithmetic correct in-map; contradicted only by the sibling ledger (F-003). |
| `TIME_TICK_DEFAULT_MS` / `TIME_TICK_FLOOR_MS` | 142/147 | REAL | — | 250/50 ms, doc-cited; floor enforced in `from_config_ms`. |
| `BUFFERING_WINDOW_MS` / `DAMAGE_WINDOW_MS` | 152/157 | REAL | — | 200 ms = 5 Hz ✓; 1000 ms per doc §3.2 proposal. |
| `TimeTickPolicy` (struct) | 160-165 | REAL | — | Copy, one field. |
| `TimeTickPolicy::from_config_ms` | 173-177 | REAL | — | `.max(FLOOR)` — the floor is enforced, 0 (unset) lands on 50. Test `the_interval_is_floored_at_50ms` (baseline 3778). |
| `TimeTickPolicy::default_policy` / `Default` | 181-183, 202-206 | REAL | — | 250 ms. |
| `TimeTickPolicy::is_due` | 192-199 | REAL | S4 (F-001 doc) | Pure; saturating_sub; no overflow; None arm protocol-dead in the sole caller. |
| `ProgressWindow` (struct) | 213-216 | REAL | — | `opened_ns: Option<u64>`. |
| `ProgressWindow::new` | 221-226 | REAL | S4 (F-002) | Pub; overflow foot-gun on absurd windows. |
| `ProgressWindow::for_buffering` | 230-232 | REAL | — | 200 ms. |
| `ProgressWindow::permits` | 239-254 | REAL | S4 (F-002) | Tumbling window; re-anchor on permit only; frozen source ⇒ exactly one emission (test-pinned). |
| `ProgressWindow::open_if_closed` | 260-264 | REAL | — | pub(crate); idempotent open — the DamageCoalescer fix's carrier. |
| `ProgressWindow::window_ms` | 268-271 | REAL | — | accessor. |
| `DamageCoalescer` (struct) | 279-285 | REAL | — | demux/decode u64s, saturating throughout. |
| `DamageCoalescer::new` / `Default` | 290-296, 350-354 | REAL | — | 1 s window. |
| `DamageCoalescer::record` | 304-311 | REAL | — | was_pending guard + open_if_closed — the documented original-bug fix; test-pinned (1612-1627). |
| `DamageCoalescer::is_pending` | 315-317 | REAL | — | disjunction. |
| `DamageCoalescer::flush_if_due` | 323-328 | REAL | — | Short-circuit keeps closed-window invariant (pending ⇒ open); demux-first order. |
| `DamageCoalescer::flush_all` | 333-338 | REAL | — | Terminal completeness half; test-pinned (1630-1640). |
| `DamageCoalescer::take_pending` | 340-347 | REAL | — | Fresh window on flush — next burst re-anchors. |
| `StalenessStamp` (struct) | 378-384 | REAL | — | Two u64s, Copy — D-1/D-2 flat drain claim holds. |
| `StalenessStamp::new` / `from_pair` / `epoch` / `counter` | 389-413 | REAL | — | Accessors; from_pair reads `pair.pipeline.0`. |
| `StalenessStamp::is_stale_against` | 427-429 | REAL | — | Epoch-first, then `!=` (documented fail-safe); oracle test-pinned. |
| `DrainStaleness` (trait) | 444-465 | REAL | — | The one-line-consumer seam; wired at session.rs:748. |
| `impl DrainStaleness for EngineEvent::staleness_stamp` | 468-476 | REAL | — | Envelope session↔epoch, generation↔counter mapping; round-trip test-pinned. |
| `impl DrainStaleness for EngineEvent::is_stale_at_drain` | 478-507 | REAL | — | Live-pair fast path; cross-epoch stale by construction; intra-epoch survives = terminals ∪ non-pipeline-scoped; `seek_terminal_exemption_is_intra_epoch_only` test-pinned. Depends on `is_pipeline_scoped` (event.rs — audited in my part2 file). |
| `EventPolicy` (struct) | 515-521 | REAL | — | pub fields; hosts inject. |
| `impl Debug for EventPolicy` | 523-530 | REAL | — | Manual (source has no Debug) — renders presence, no leak. |
| `EventPolicy::default_policy` | 554-560 | REAL | — | cfg-split default; wasm32 issue-#49 narrative matches std behavior; web host always injects. |
| `default_throttle_source` ×2 + `ZeroMonotonic` | 566-589 | REAL | — | wasm arm returns const 0 — honest placeholder, documented. |
| `EmissionStatus` + `is_emitted` | 602-623 | REAL | — | Enum + predicate. |
| `CatalogEmission` (struct) | 626-632 | REAL | — | Row carrier. |
| `ENGINE_EMITTED_ROWS` | 638 | REAL | — | 15; asserted by the bijection test (part4's range, passing at baseline 3774). |

## 5. Test honesty (tests for this range live at 1582-1992, part3/4 ranges — assessed for part1 coverage)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `a_fresh_policy_only_sets_the_baseline` (1583) | YES | Pins the None-arm contract (false) — would catch a silent immediate-tick regression. Note it tests the *function*, so F-001's doc rot is invisible to it by construction. |
| `ticks_are_due_only_at_the_interval` (1592) | YES | Exact boundary (249_999_999 vs 250_000_000) + backward-position case. |
| `a_frozen_source_permits_exactly_one_progress_emission` (1603) | YES | Window boundary exact. |
| `damage_coalesces_then_flushes_on_the_window` (1612) | YES | Window-open-at-first-record, both-stage flush, fresh-window reopen — covers the documented original-bug fix. |
| `terminal_flush_answers_pending_counts_and_reopens` (1630) | YES | Unconditional flush + reopen semantics. |
| `cross_epoch_is_stale_by_construction_whatever_the_counter` (1686) | YES | Both masquerade directions. |
| `the_stamp_rule_agrees_with_the_cancel_token_oracle` (1901) / `stamps_round_trip_through_pairs_and_envelopes` (1928) | YES | Independent-oracle and round-trip proofs. |
| `the_completeness_map_partitions_the_catalog_exactly` (1956, part4 range) | YES | Bijection + order + 15/11/26. |
| All above pass in `orchrun-test-workspace-nofailfast.log.txt` (3765-3778) | — | The fail-fast Wave-0 baseline also ran these (83-test lib target), so no coverage change from the new evidence — the new run confirms rather than reveals here. |
| Coverage hole | — | No test feeds `ProgressWindow::new` an oversized window (F-002 unreachable in-tree); no test asserts the is_due doc's "sets the baseline" narrative (it can't — the doc is wrong, F-001). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`timing.time-tick` … never queued: at most one tick in flight … a newer tick replaces an undelivered older one … counted" | events.rs:25-31 (rule 1) | REAL | session.rs:613-627 kill+replace+`time_ticks_replaced` counter (566/623) |
| "`session.buffering` to ≤ 5 Hz … bookends bypass the window" | events.rs:32-36 (rule 2) | REAL | `BUFFERING_WINDOW_MS`=200; bookends are table effects (session's episode logic — cross-file, consistent with the emitted bookends in baseline open transcripts) |
| "at capacity the oldest event is dropped and counted while receipts are pinned … hard-capped at 4096" | events.rs:41-50 (rule 4) | REAL | session.rs:599-666 + constants 98/109; `compact_if_due` keeps memory ~2× live (691-704) |
| "The time-tick interval is judged against the media position … a frozen test clock produces no ticks" | events.rs:66-69 | REAL | session.rs:2647-2662 reads the media clock; frozen ⇒ no advance ⇒ no tick |
| "wasm32: every wasm `session.open` trapped in the `unreachable` panic before the injection" | events.rs:536-545 | REAL-UNPROVEN (history) | The fix (cfg-split) is in-tree and the narrative matches std's wasm32 `Instant` panic; the original trap is historical claim I cannot re-run — issue #49 cited |
| "15 of 26 rows are engine-emitted today … 11 rows are honestly owned by later cards" | events.rs:109-113 | REAL | Map count + bijection test (baseline 3774); the sibling 14/12 ledger contradicts it (F-003) — both machine-checked, engine side authoritative |
| "Emitted rows are proven by the contract suite" | events.rs:76-78 | REAL | contract_suite.rs + events_contract.rs exist and pass (baseline lib/integration targets); hotplug trio additionally by tests/hotplug.rs (3897-3906) |
| "`tracks.changed` … the class-A set-*-track terminal under the switch's own gen" (engine-emitted) | events.rs:90 | REAL (engine) / UNREACHABLE (shipped) | Engine-side true (track_switch.rs 12/12); no shipped host can cause it — see pipeline.rs part3 F-003 |
| "doc 05 §3.2: 'floor 50 ms' … cannot go below it, whatever the config says" | events.rs:144-147 | REAL | `from_config_ms` `.max()` + test 3778 |

## 7. Dead / duplicated / unreachable code
- No dead code in range. The `None` arm of `is_due` is protocol-dead in the sole caller but intentionally defensive (documented as such once F-001's doc is fixed).
- Duplication: the ms→ns product appears in `TimeTickPolicy::is_due` (i64, safe) and `ProgressWindow::permits` (u64, F-002) — two spellings of one conversion; the S4 fix centralizes it.

## 8. Dependencies & flags
Deps: `commands` (envelope/catalog types), `media_clock` (MonotonicSource), `testkit` (tests only). One cfg split: `target_arch = "wasm32"` for the default throttle source — the only flag-relevant behavior in range, and it matches the shipped web target's injection pattern. No `unsafe`. No feature gates.

## 9. Open questions for the next agent
1. A10 (events part2): the `OrderingGuard`'s `debug_assert` form — confirm it is never compiled into the shipped release build (doc 05 §4.3 conformance would then be test-only, not shipped-runtime); if it is `debug_assert!`, that is a deliberate choice worth a row in your claim table.
2. Session auditor: the module doc's "the exact `session.rs` wiring line is registered as a Lead-merge follow-up in the Wave-1 worklog" (15-17) — the wiring exists (748); the sentence reads as pending. Fold its rewording into any session doc pass.
3. Commands-crate auditor owns F-003's ledger unification.
