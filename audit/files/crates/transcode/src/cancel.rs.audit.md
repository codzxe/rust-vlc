# AUDIT: crates/transcode/src/cancel.rs
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-11-21 |
| Lines / functions | 85 / 4 production items + 2 tests |
| File verdict | REAL (correct, minimal) — UNREACHABLE from shipped entry points; one documented cancel-contract promise falsified by the runner |
| Reachable from shipped entry point? | NO. Consumers: job.rs (JobShared/JobHandle), queue.rs, runner.rs, record.rs — all inside the transcode crate, which no shipped binary depends on (proof in my runner.rs#part1 F-002: zero dependents in every app/Cargo.toml; product.toml:165 `transcode = false # Phase 7`). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 0 |
| Commands run | read 1-85 in full (during runner.rs cross-reference and again for this audit) · rg CancelToken callers (job.rs/queue.rs/runner.rs/record.rs — crate-internal only) · sed -n 16,19p / 44,51p (quote verification) · cross-verified the doc's checking-claims against runner.rs:378/509/518 and queue.rs:225/238-240/256/260-262 · baseline #3 (both tests pass) |

## 1. One-paragraph verdict
Eighty-five lines of exactly what a cooperative cancel token should be:
one Arc'd atomic flag, SeqCst on both operations, idempotent fire with
an informative return value, honest docs about what firing does NOT do
(wake parked waiters — the queue's interrupt owns that). The
implementation is flawless; the documentation makes one promise the
runner does not keep: "A cancelled job still finalizes its sink (best
effort) so the partial output is structurally valid" — the runner's
pump can mint the Cancelled terminal first, and the worker then exits
without the finalize (my runner.rs#part1 F-001 traces the interleaving;
this file owns the promise). The whole thing is dead code in every
shipped binary with the rest of the crate.

## 2. What actually works (verified)
— `CancelToken` semantics: `Clone` shares one `Arc<AtomicBool>` flag
(30-33); `cancel()` swaps true SeqCst and returns the previous state
(49-51) — first fire reports false, repeats report true; `is_cancelled()`
SeqCst load (55-57). Both tests pin the sharing and independence
behavior, and both pass in baseline #3.
— The doc's operational claims verified against the code it describes:
  * "no stage is interrupted mid-call" — runner calls decode/sink to
    completion between checks (runner.rs:519).
  * "before each demux event, between each decoded packet, inside every
    bounded queue park" — runner.rs:378 (loop top), 509 (worker loop
    top), queue.rs:225/238-240 (producer) and 256/260-262 (consumer).
  * "Firing does not wake parked queue waiters — the owning job calls
    interrupt" — verified in queue.rs: interrupt (178-182) is what
    notifies; the token alone is inert.
— `#[must_use]` on `new()` and `is_cancelled()`; `Default` yields a
non-fired token (AtomicBool::default = false) — consistent with the
docs.

## 3. Findings

### F-001 [S2] — same defect as runner.rs#part1 F-001 (dedupe at rollup: one root defect, two contract surfaces)
Where: crates/transcode/src/cancel.rs:16-19 (the promise), falsified by runner.rs:509-517/568-574
Code:
```rust
//! A cancelled job still **finalizes its sink** (best effort) so the
//! partial output is structurally valid — sizes tell the truth about
//! the bytes that made it; the outcome distinguishes `Cancelled` from
//! `Completed` so no caller mistakes a prefix for the whole.
```
What is wrong: This module-level contract is not guaranteed by the
pipeline that implements it. On user cancel both threads race to mint
the `Cancelled` terminal; when the PUMP's stop path wins
(runner.rs:1101-1105 via pump_body 379-380/400-401), the worker's
subsequent `is_stopping()` check finds the outcome already set and
returns WITHOUT `sink.finish()` (runner.rs:516, and the Interrupted arm
at 573) — the partial WAV keeps placeholder sizes: not structurally
valid, sizes NOT telling the truth. The full interleaving trace is in
my runner.rs#part1 F-001; this file is audited because the promise
lives here and is the crate's advertised cancel semantics.
Why it matters: The honest-prefix contract is the crate's answer to
"what does a cancelled conversion leave on disk"; a caller relying on
this doc gets a corrupt prefix depending on thread scheduling. (The
crate is unreachable today — practical impact deferred until Phase 7.)
What correct looks like: Either the runner guarantees the worker-side
finalize on every Cancelled outcome (pump never mints Cancelled while
a sink is owned), or this doc is downgraded to "the worker finalizes
best-effort when it observes the cancel first".
Confidence: HIGH — Verification: read (interleaving traced in the
runner audit; quotes verified by sed).

### F-002 [S3]
Where: crates/transcode/src/cancel.rs:30-33 (the type)
Code:
```rust
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
}
```
What is wrong: Nothing in the code — the crate is dead in every
shipped binary (transcode has zero dependents; product.toml Phase-7
gate; proof in runner.rs#part1 F-002).
Why it matters: Same crate-level deadness already recorded (W01-A03
F-002); this file's quality (and its broken promise, F-001) is
invisible to users until the Phase-7 wiring lands.
What correct looks like: Phase-7 wiring, with F-001 fixed first.
Confidence: HIGH — Verification: read (dependency greps in the
runner.rs#part1 audit).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-19 | REAL | S2 | Checking-points and no-mid-call claims verified TRUE; the finalize-on-cancel promise is FALSE (F-001). |
| imports | 21-22 | REAL | — | std only. |
| `CancelToken` | 30-33 | REAL / UNREACHABLE | S3 | F-002; Arc<AtomicBool>, the shape the streaming event hubs use (doc claim, consistent). |
| `new` | 36-41 | REAL | — | must_use. |
| `cancel` | 44-51 | REAL | — | SeqCst swap; returns previous state; docs honest that firing alone wakes nobody. |
| `is_cancelled` | 53-57 | REAL | — | SeqCst load; must_use. |
| tests `clones_share_the_flag_and_firing_is_idempotent` | 64-75 | REAL | — | Pins clone-sharing + swap return semantics. |
| tests `independent_tokens_do_not_interfere` | 77-84 | REAL | — | Separate Arcs. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| clones_share_the_flag_and_firing_is_idempotent (64) | YES | A regression to an independent-flag Clone or a non-swap cancel breaks the assertions (the swap return values are checked in both directions). |
| independent_tokens_do_not_interfere (77) | YES | Cross-token isolation pinned. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A cancelled job still finalizes its sink (best effort) so the partial output is structurally valid" | cancel.rs:16-19 | FALSE (racy) | F-001 / runner.rs#part1 F-001 interleaving. |
| "no stage is interrupted mid-call" | cancel.rs:4-6 | REAL | runner.rs:519 — decode runs to completion between checks. |
| "before each demux event, between each decoded packet, inside every bounded queue park" | cancel.rs:11-14 | REAL | runner.rs:378/509; queue.rs:225/238-240/256/260-262. |
| "Firing does not wake parked queue waiters — the owning job calls interrupt" | cancel.rs:46-48 | REAL | queue.rs:178-182 (interrupt notifies; the token does not). |

## 7. Dead / duplicated / unreachable code
The file is unreachable with its crate (F-002). No internal dead code;
both tests exercise the full public surface.

## 8. Dependencies & flags
std only (Arc, AtomicBool). No unsafe, no features, no I/O. SeqCst is
stronger than strictly required (Release/Acquire would suffice for a
boolean flag) — a deliberate conservative choice, not a defect.

## 9. Open questions for the next agent
- A06 (job.rs): `JobHandle`'s cancel path — confirm it fires the token
  AND calls `PipelineThreads::interrupt()` (the wake contract this file
  documents depends on both being called).
- F-001's fix belongs in runner.rs; when it lands, the
  `cancellation_is_prompt_terminal_and_leaves_a_truthful_prefix` test
  should gain a deterministic pump-wins variant.
