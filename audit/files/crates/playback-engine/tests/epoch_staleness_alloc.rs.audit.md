# AUDIT: crates/playback-engine/tests/epoch_staleness_alloc.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 144 / 1 test fn + 4 helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test) — plain `#[test]`, single-test-per-binary discipline with `serialize_tests`; part of W00 baseline (execution UNVERIFIED, no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read; cross-check of `DrainStaleness`/`is_stale_at_drain` against playback-engine exports and contract_suite's drain-exemption test (1108-1172); sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

A measured zero-allocation proof for the output hub's drain-time staleness decision (`DrainStaleness::is_stale_at_drain`), using the same honest discipline as `zero_alloc_resample.rs`: counting global allocator, one test per binary, storm construction strictly outside the measured guards, and exact non-vacuity assertions (10 000 stale suppressed, 10 receipts delivered, 0 stale survivors; then 10 000 fresh all passing). One coverage gap: the measured windows exercise the cross-epoch arm and the all-fresh arm, but not the same-epoch-superseded-generation arm of the decision — that path's correctness is left to other files.

## 2. What actually works (verified)

- Window 1 (103-125): 10 000 superseded-epoch events (`SessionId(1)`, `Generation(3)` vs current epoch 2) + 10 interleaved receipts, drained through the decision loop under the allocator guard — `assert_zero` over 10 001 decisions, then exact counts: every event stale, every receipt delivered. Any allocation in `is_stale_at_drain` (e.g., a clone to compare) fails the guard; any misclassification fails the counts.
- Window 2 (130-143): the all-fresh path (epoch 2, generation 0 == current) — zero allocations, all 10 000 pass. Guards the "flat when passing everything" half.
- The storm mixes kinds (`EndOfMedia`, `TimeTick`, `Buffering`, `Seeked` at 69-81) so kind-specific arms of the decision are all in the measured set; the doc's claim that seek terminals classify stale *without* the exemption in the cross-epoch case is thereby covered.
- Receipts-never-suppressed is asserted with data (124), not assumed.

## 3. Findings

### F-001 [S4] The same-epoch superseded-generation arm of the decision is not measured here
Where: crates/playback-engine/tests/epoch_staleness_alloc.rs:91-100 vs 41-46
Code (41-46):
```rust
fn current_pair() -> GenerationPair {
    GenerationPair {
        epoch: SessionEpoch(2),
        pipeline: PipelineGeneration(0),
    }
}
```
What is wrong: The storm pairs epoch 1/gen 3 against epoch 2/gen 0 (cross-epoch), and the fresh window pairs epoch 2/gen 0 against itself. The third arm — same epoch, older *pipeline generation* (e.g., epoch 2/gen 1 events vs a live gen 2) — the arm that governs seek-supersession within a session, is absent from both windows; its zero-alloc and classification behavior is unmeasured by this file.
Why it matters: The module doc bills the proof as covering "the M11 epoch-aware staleness decision" generally; one of its three decision arms is untested here. The arm's *correctness* is covered elsewhere (contract_suite's supersession test asserts suppression counts), but not its allocation behavior. Hygiene/scope.
What correct looks like: A third window with same-epoch, older-pipeline-generation events (mixed with a completed-generation seek terminal to also pin the exemption arm's allocation behavior).
Confidence: HIGH — Verification: read (window construction vs GenerationPair semantics).
## 4. Per-function table

| Function | Lines | Verdict | Notes |
|---|---|---|---|
| `current_pair` | 41-46 | REAL | The live pair; fixed. |
| `resident_event` | 48-56 | REAL | Storm/fresh event builder (setup-side). |
| `resident_receipt` | 58-66 | REAL | Receipt builder. |
| `storm_kind` | 68-81 | REAL | Four-kind rotation incl. `Seeked`. |
| the_epoch_staleness_decision_is_zero_allocation_under_storm | 84-144 | REAL | Both windows exact; F-001 scope. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| the_epoch_staleness_decision_is_zero_allocation_under_storm | YES | Allocation in the decision → guard fires; misclassification → count asserts fail; vacuous drain impossible (10 000/10/0/10 000 pinned). |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "allocates **nothing**, measured under a supersede-then-push storm and on the all-fresh common path" (4-6) | module doc | REAL | Both windows + assert_zero. |
| "10 001 decisions, zero allocation events, zero bytes" (25-26) | module doc | REAL | 10 000 events + 1 implied... window 1 pops 10 010 items (10 000 events + 10 receipts); the "10 001 decisions" phrasing undercounts receipts-as-decisions — cosmetic, the guard covers the whole window either way. |

## 7. Dead / duplicated / unreachable code

None.

## 8. Dependencies & flags

dev-deps: playback-engine, commands, testkit. No feature gates. Single-test-per-binary rationale documented (8-14).

## 9. Open questions for the next agent

- W03 colleague with playback-engine/src/events.rs (the hub): does `is_stale_at_drain`'s same-epoch arm share code with the cross-epoch arm (in which case F-001's allocation risk is nil), or does it have its own comparison path?
