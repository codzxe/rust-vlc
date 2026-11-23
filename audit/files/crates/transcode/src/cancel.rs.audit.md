# AUDIT: crates/transcode/src/cancel.rs
| Field | Value |
|---|---|
| Auditor | W10-A10 |
| Date | 2026-11-23 |
| Lines / functions | 85 / 3 production fns (CancelToken new/cancel/is_cancelled) + 2 tests |
| File verdict | UNREACHABLE (real, correct, tested — transcode crate island) |
| Reachable from shipped entry point? | NO — callers are crates/transcode/src/runner.rs (job lifecycle) and queue.rs, none reachable from any shipped binary (no manifest depends on `transcode`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p transcode --lib cancel::` -> 2 passed / 0 failed (exit 0); grep of interrupt/cancel wiring in runner.rs/queue.rs; sed quote verification |

## 1. One-paragraph verdict
A 58-line cooperative cancellation token done right: `Arc<AtomicBool>` shared by `Clone`, idempotent one-way fire (`swap(true, SeqCst)` returning the previous state), honest doc that firing alone does not wake parked waiters (the owning job must call `PacketStageQueue::interrupt` — and runner.rs:225/queue.rs:178 actually do, with queue.rs:450 proving a parked consumer wakes). Both tests pass under my run. The only defects are the crate's island status (nothing shipped can ever fire this token) and a stronger-than-needed memory ordering.

## 2. What actually works (verified)
— verified by `cargo test -p transcode --lib cancel::` -> `test result: ok. 2 passed; 0 failed` (clone-shares-flag + idempotent fire; token independence).
— verified by grep: runner.rs:225 `queue.interrupt()`, queue.rs:178 `fn interrupt`, queue.rs:450 `interrupt_wakes_a_parked_consumer`, runner.rs:1016 `self.cancel.is_cancelled()` in the pump loop — the module doc's wiring claims are real.

## 3. Findings
F-001 [S3]
Where: crates/transcode/src/cancel.rs:31-33
Code:
```
pub struct CancelToken {
    flag: Arc<AtomicBool>,
}
```
What is wrong: reachability only — the token, its queue-interrupt wiring, and the cancelled-job-finishes-with-valid-partial-output contract live entirely inside a crate no shipped binary depends on (see wav.rs/convert.rs audits). Cancel of a transcode job is not a user-reachable action.
Confidence: HIGH — Verification: ran (manifest grep, prior files).
F-002 [S4]
Where: crates/transcode/src/cancel.rs:49-57
Code:
```
    pub fn cancel(&self) -> bool {
        self.flag.swap(true, Ordering::SeqCst)
    }

    /// Whether the stop request has fired.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
```
What is wrong: `SeqCst` on both sides is stronger than a one-way boolean flag needs (`Release` store / `Acquire` load — or even `Relaxed`, since no data is published through the flag) — no correctness bug, pure over-synchronization on a path checked per packet. Cosmetic.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CancelToken::new | 38-42 | REAL | — | Fresh flag |
| CancelToken::cancel | 49-51 | REAL | S4 | Idempotent swap, returns previous; F-002 ordering |
| CancelToken::is_cancelled | 55-57 | REAL | S4 | Load; F-002 |
| tests (2) | 64-84 | REAL | — | Clone sharing, idempotence, independence |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| clones_share_the_flag_and_firing_is_idempotent | YES | Cross-clone observation both directions + previous-state returns |
| independent_tokens_do_not_interfere | YES | Two fresh tokens isolated |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Clone shares the same underlying flag" | cancel.rs:26-28 | REAL | Test 1 |
| "Firing does not by itself wake parked queue waiters — the owning job calls interrupt" | cancel.rs:46-48 | REAL | runner.rs:225 + queue.rs:178,450 |
| "every loop iteration, every bounded park and every queue hand-off checks this token" | cancel.rs:6-8 | REAL-UNPROVEN | runner.rs:1016 shows the pump check; the full per-stage enumeration lives in runner.rs/queue.rs (not my files) — spot-checked only |

## 7. Dead / duplicated / unreachable code
- Whole module unreachable from shipped entry points (F-001). No duplication.

## 8. Dependencies & flags
- std-only (Arc + AtomicBool). No feature gates.

## 9. Open questions for the next agent
- runner.rs/queue.rs auditors: confirm the "cancelled job still finalizes its sink" claim (cancel.rs:16-19) — the finalize path on cancellation is the one contract this token's docs promise that I could not execute.
