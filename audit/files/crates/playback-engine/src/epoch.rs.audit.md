# AUDIT: crates/playback-engine/src/epoch.rs
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-23 |
| Lines / functions | 367 / 16 (CancelToken 5, TokenMinter 5, SessionEpoch 3, PipelineGeneration 3, GenerationPair 2 + helpers, 4 unit tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: `lumen open` → `Engine::open` → `Session` construction `TokenMinter::new(epoch)` (session.rs:1439); every state-table `BumpPipeline` effect mints (session.rs:1612), teardown interrupts (session.rs:1591); the token is exposed at `Engine::cancel_token` (session.rs:1120). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/playback_engine-30ee2bde67c1e3d9 epoch` → 8 passed (4 epoch:: + 4 events:: cross-matched), 0 failed, exit 0. |

## 1. One-paragraph verdict
Correct, minimal two-level cancellation machinery exactly as documented: pair-wise staleness (different epoch OR different pipeline ⇒ stale, plus an interrupt flag that marks even the live pair stale at teardown), single-writer minting behind `&mut`, checked u64 arithmetic with a typed exhaustion terminal, and wire mappings that are the raw counters. The cross-epoch trap the design calls out (epoch 2's pipeline 0 must never satisfy epoch 1's token) is both implemented and unit-tested, and the property storm (generation_monotonicity.rs) hammers it across 64 seeds. Two S4 notes: `publish()` unconditionally clears the interrupt flag (a latent resurrect-the-dead-token footgun, unreachable today only through the state machine's effect ordering), and `interrupt()`'s braces are a stylistic relic.

## 2. What actually works (verified)
- `./target/debug/deps/playback_engine-30ee2bde67c1e3d9 epoch` → all pass, including `cross_epoch_equal_pipeline_counters_are_stale` (the motivating law) and `interrupt_marks_even_the_current_pair_stale`.
- Mint/interrupt ordering in the shipped path verified: the close row's `BumpPipeline` effects run before `DestroySession`'s `minter.interrupt()` (session.rs:1581-1606), and `closed = true` ends the session — no mint can follow an interrupt within an epoch.
- The generation-monotonicity storm (previous file) exercises this module end-to-end: exact mint deltas per receipt, cross-epoch staleness with equal counters (generation_monotonicity.rs:177-186), dead-handle typed refusals.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/src/epoch.rs:195-200 (with 176-184)
Code:
```
    fn publish(&self) {
        let mut state = self.token.lock();
        state.epoch = self.epoch.0;
        state.pipeline = self.pipeline;
        state.interrupted = false;
    }
``
What is wrong: `publish()` clears `interrupted` unconditionally, and `mint_pipeline()` (176-184) neither checks nor asserts the flag. A mint issued after an interrupt would resurrect the dead token — every stage holding it would see its last pair current again. Today this is unreachable through the session (the interrupt lives only in `DestroySession`, after which `closed = true` ends the minter's use; verified session.rs:1587-1606), so the safety rests entirely on caller discipline, not on the type. What correct looks like: `mint_pipeline` refusing (typed error) or at least `debug_assert!(!self.token.lock().interrupted)` when the flag is set — turning the ordering contract into an enforced invariant. Confidence: HIGH — Verification: read (reachability of the bad ordering disproven by tracing session.rs effect order; not executed).

F-002 [S4]
Where: crates/playback-engine/src/epoch.rs:188-193
Code:
```
    pub fn interrupt(&mut self) {
        {
            let mut state = self.token.lock();
            state.interrupted = true;
        }
    }
```
What is wrong: the redundant inner block is a stylistic relic (the guard is dropped at function end anyway); `&mut self` already grants exclusivity the extra scope pretends to need. Zero behavioral impact. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `TokenState` (struct) | 54-59 | REAL | — | Three words, Copy. |
| `CancelToken::snapshot` | 71-74 | REAL | — | Alias of pair(). |
| `CancelToken::is_current` | 78-81 | REAL | — | ¬stale. |
| `CancelToken::is_stale` | 91-94 | REAL | — | The doc 03 §5 comparison + interrupt. Correct: epoch OR pipeline mismatch OR interrupted. |
| `CancelToken::interrupted` | 100-102 | REAL | — | Teardown probe. |
| `CancelToken::lock` | 104-111 | REAL | — | Poison recovery with infallible-critical-section argument (same as packet-queue). |
| `CancelToken::pair` | 113-119 | REAL | — | Copy under lock. |
| `TokenMinter::new` | 138-151 | REAL | — | Publishes FIRST pair (epoch, pipeline 0, not interrupted). |
| `TokenMinter::token` / `pair` | 155-166 | REAL | — | Read-side clone / live pair. |
| `TokenMinter::mint_pipeline` | 176-184 | REAL | S4 | checked_add + typed exhaustion (Control/ResourceLimit/SessionGenerations); F-001 no interrupt guard. |
| `TokenMinter::interrupt` | 188-193 | REAL | S4 | F-002 braces; idempotent. |
| `TokenMinter::publish` | 195-200 | REAL | S4 | F-001 unconditional interrupt clear. |
| `generation_exhausted` | 203-213 | REAL | — | Typed terminal; budget/attempted = u64::MAX (contextual). |
| `SessionEpoch::FIRST` / `next` / `as_session_id` | 226-244 | REAL | — | Starts at 1 (documented PROPOSAL); checked next. |
| `PipelineGeneration::FIRST` / `as_wire` / `as_queue_generation` | 255-269 | REAL | — | Starts at 0 (doc 05 §4.2); queue space = pipeline counter of current epoch (per-epoch fresh queues make cross-epoch collisions unobservable — argument in module docs 35-45, sound). |
| `GenerationPair::session_id` / `wire_generation` | 287-295 | REAL | — | Wire mappings. |
| 4 unit tests | 302-366 | REAL | — | All executed; exact asserts (mint→1, stale old pair, interrupt invalidates, cross-epoch equal counters stale, wire mappings). |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| minter_publishes_and_stages_observe | YES | Stage-held token must flip stale on mint — a non-publishing or non-atomic mint fails. |
| interrupt_marks_even_the_current_pair_stale | YES | The interrupt flag's whole purpose. |
| cross_epoch_equal_pipeline_counters_are_stale | YES | The bare-counter bug the module exists to prevent. |
| wire_mappings_are_the_raw_counters | YES | Any off-by-one or transformation in the wire mapping fails. |
(Companion coverage in events.rs — `cross_epoch_is_stale_by_construction_whatever_the_counter`, `seek_terminal_exemption_is_intra_epoch_only` — also ran green under my `epoch` filter.)

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "different epoch OR different pipeline ⇒ stale" | epoch.rs:19-23 (doc 03 §5) | REAL | is_stale:91-94 + test 333-350. |
| "Only the session thread mints … the TokenMinter lives behind &mut in the session" | 27-29 | REAL | session.rs:1323 field, mint only at 1612 (BumpPipeline effect) under &mut. |
| "queues are constructed fresh per epoch (S4 — no cross-epoch queue reuse) … cross-epoch collisions in the queue space are therefore unobservable" | 35-45 | REAL | Pipeline::new per open (pipeline.rs:999), queues built in wire() per session (1104-1133); reopen cycles test (model_walk 1000×) would surface reuse as epoch/generation failures. |
| "minted on seek / track switch / final stop / close — the minimal machine mints on stop and close only" | 168-170 | PARTLY STALE DOC | The shipped machine mints on seek and A/V track switches too (session.rs BumpPipeline on those rows; the generation_monotonicity storm asserts exactly +1 per accepted seek/switch/stop). The doc comment describes the older minimal machine — the module docs at 13-17 say the same ("seek/track-switch joins with Phase 3" — Phase 3 landed). Cosmetic doc lag, not a code defect. |

## 7. Dead / duplicated / unreachable code
- None. All items used by session.rs / events.rs / lib.rs re-export (lib.rs:112).

## 8. Dependencies & flags
- `commands` (SessionId, Generation wire types), `media_types` (EngineError, GenerationId), std sync. No features, no unsafe. The `commands` dependency direction (engine → commands for wire types) is the documented wire-mapping pattern, not a layering violation.

## 9. Open questions for the next agent
- Harden F-001 (mint-after-interrupt) with a guard if the threaded Phase 3 ever lets effects interleave differently.
- events.rs owns the "exactly one terminal per generation" exemption bookkeeping built on these pairs — audit its cross-epoch exemption rules when assigned (its 4 epoch-adjacent tests ran green here).
