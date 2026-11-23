# AUDIT: crates/playback-engine/tests/epoch_staleness_alloc.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-23 |
| Lines / functions | 144 / 6 (4 builders + 1 test) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test binary). It measures `EngineEvent::is_stale_at_drain` (events.rs:478) which IS the shipped drain-time check — the production drain loop is session.rs:740-755 (`if event.is_stale_at_drain(current)` per resident event, receipts pushed unconditionally — the test's drain shape mirrors it exactly). Ran live: prebuilt `epoch_staleness_alloc-d436df7c246ac0da` -> 1 passed, 0 failed (exit 0). Baseline: cargo-test-workspace.txt green. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/epoch_staleness_alloc-d436df7c246ac0da` -> 1 passed (exit 0); `sed`/`rg` verifications of the production drain loop |

## 1. One-paragraph verdict
A genuine zero-allocation proof of the shipped epoch-staleness decision, in the exact shape the production hub drains (pop one slot, decide, count): a 10,000-event superseded-epoch storm (four mixed kinds including the seek terminal that carries the intra-epoch exemption — proving the cross-epoch arm classifies it stale anyway) with interleaved receipts, plus the all-fresh common path, both under the counting allocator's assert-zero guard, with setup strictly outside the measured windows. The one defect is a cosmetic arithmetic slip: the doc and the assert message say "10 001 decisions" while the storm actually pops 10,010 slots (10,000 events + 10 receipts) and decides 10,000 events.

## 2. What actually works (verified)
— verified by running the prebuilt binary: `test result: ok. 1 passed; 0 failed` (exit 0) — both windows assert-zero pass (zero allocation events, zero bytes on the decision path).
— verified the tested function is the shipped one: `session.rs:748` calls `event.is_stale_at_drain(current)` in the hub's `drain_into`; the trait's cross-epoch-stale-by-construction rule (events.rs:478-486) is exactly the arm the storm exercises (epoch 1 events vs current epoch 2, terminals included, all 10,000 suppressed).
— verified the test's semantics: receipts are never suppressed (10 delivered), matching the production loop's unconditional receipt push (session.rs:744-746).

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/epoch_staleness_alloc.rs:25-26 and 119
Code:
```
//! push is setup. Bounded work is the claim, and bounded work is what
//! is measured: 10 001 decisions, zero allocation events, zero bytes.
...
    guard.assert_zero("the epoch-staleness storm drain (10_001 decisions)");
```
What is wrong: The count is wrong on both readings. The storm loop (lines 91-96) pushes 10,000 events plus 10 receipts (`seq % 1_000 == 0` for seq in 0..10_000 → 0,1000,…,9000), so the drain pops 10,010 slots, of which 10,000 get the staleness decision (receipts take the unconditional arm). "10 001" matches neither 10,010 nor 10,000.
Why it matters: The file's whole discipline is measured-number honesty (D-6); its own headline count is off by nine (or ten, depending on the intended reading), and a reader re-deriving the workload from the doc gets a different number than the code produces.
What correct looks like: "10 010 slots (10 000 staleness decisions + 10 receipts)".
Confidence: HIGH — Verification: read + hand-recount of the loop.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| current_pair | 41-46 | REAL | — | Epoch 2 / pipeline 0 — the live pair both windows judge against. |
| resident_event | 48-56 | REAL | — | Envelope stamped (session=epoch, generation=counter) exactly as the trait's `staleness_stamp` reads it (events.rs:469-476). |
| resident_receipt | 58-66 | REAL | — | Completed-play receipt shape. |
| storm_kind | 68-81 | REAL | — | 4-kind rotation incl. `Seeked` — the exemption-carrying terminal proven stale cross-epoch. |
| the_epoch_staleness_decision_is_zero_allocation_under_storm | 83-144 | REAL | S4 | Two measured windows, both assert-zero; semantic asserts (10,000 stale / 10 receipts / 0 delivered; 10,000 fresh pass). F-001 is the count slip. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the_epoch_staleness_decision_is_zero_allocation_under_storm | YES | An allocation on the decision path (e.g. a kind-clone or format! in `is_stale_at_drain`) trips assert_zero; a wrong cross-epoch classification fails `suppressed_stale == 10_000` (e.g. if the seek-terminal exemption were applied cross-epoch, ~2,500 Seeked events would survive and `events_delivered == 0` fails); a receipt-suppressing regression fails `receipts_delivered == 10`. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "10 001 decisions, zero allocation events, zero bytes" | lines 25-26 | FALSE (the count) / REAL (the zeros) | F-001; the zeros are asserted and pass. |
| "The construction of the storm … happens before the guard: the drain is the hot path" | lines 22-24 | REAL | Both `guard`s (103, 130) start after the pushes; pops/decisions inside. |
| "seek terminals included: the cross-epoch arm must classify them stale without the exemption" | lines 20-21 | REAL | storm_kind's `Seeked` arm (25% of the storm) is inside the 10,000 suppressed; the exemption is intra-epoch only (events.rs:486-503). |
| "receipts are never suppressed" | line 124 (assert message) | REAL | Matches production loop (session.rs:744-746) and the trait (receipts take no staleness check). |

## 7. Dead / duplicated / unreachable code
- None. Every builder is used by the single test; the file is minimal by design.

## 8. Dependencies & flags
- playback_engine (epoch, event, session types, DrainStaleness), commands (envelope), testkit (alloc_count, serialize_tests). Whole-binary single-test discipline; no feature gates; runs in the plain test lane (baseline + my live run).

## 9. Open questions for the next agent
- The intra-epoch exemption semantics (terminals + session-scoped kinds survive later mints within the epoch) are unit-tested in events.rs's own suite (events.rs:1728-1745) — this file deliberately does not re-prove them; events.rs's audit (another agent) should confirm those negative cases exist (they appeared to in my file-1 cross-checks).
