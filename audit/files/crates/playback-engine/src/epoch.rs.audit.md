# AUDIT: crates/playback-engine/src/epoch.rs
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-21 |
| Lines / functions | 367 / 19 prod items + 4 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES. `lumen` CLI → playback-engine session executor: session.rs:1439 `minter: TokenMinter::new(epoch)`, 1591 `self.minter.interrupt()`, 1612 `self.minter.mint_pipeline()`; CancelToken cloned to pipeline stages (`token()`). Also re-exported pub (lib.rs:112). Tests executed: orchrun log 3756-3760 — all 4 `epoch::tests::*` ok (playback_engine lib 83/83). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg `\.interrupt\(\)\|mint_pipeline\|TokenMinter::new` (callers: session.rs 1439/1591/1612, events.rs test oracle 1904-1906) · Read session.rs 1580-1620 (effect order: BumpPipeline before DestroySession) + 1182-1192 (closed flag read engine-side) · sed -n quotes verified · baseline log grep epoch::tests (4/4 ok) |

## 1. One-paragraph verdict
A small, tight cancellation core: the two-level (epoch, pipeline) pair
comparison is exactly the doc 03 §5 rule, minting is checked with a typed
exhaustion error, the shared token is a three-word mutex snapshot with poison
recovery, and the cross-epoch-equal-counters trap (the rule that motivates
per-epoch queues) has its own test. All four tests ran and passed in the
baseline. The single defect is a latent API trap: `publish()` silently clears
the interrupt flag, so a mint after an interrupt would revive a torn-down
token — the `interrupt()` doc ("every subsequent staleness check answers
stale") does not state the caveat, and safety currently rests on the
executor's effect ordering, not on the type.

## 2. What actually works (verified)
— Pair-wise staleness: `is_stale` (91-94) = interrupted || epoch≠ || pipeline≠ — exactly doc 03 §5's "different epoch OR different pipeline ⇒ stale" plus the teardown flag; `cross_epoch_equal_pipeline_counters_are_stale` (333-350) pins the motivating trap (epoch 2's pipeline 0 ≠ epoch 1's pipeline 0). ok in baseline.
— Mint/publish atomicity: `mint_pipeline` (176-184) checked_add → publish under the lock; the stage-held token observes the new pair and the old pair goes stale — asserted by `minter_publishes_and_stages_observe` (303-319).
— Interrupt semantics in the wired order: the table's close rows are `[BumpPipeline, DestroySession]` (state.rs 414-419, 647-664) and the executor runs effects in order (session.rs:1583 `for effect in row.effects`), so mint-then-interrupt always; after DestroySession the engine drops the session (session.rs:1187-1192 reads `closed`) — the revive path is unreachable in the shipped wiring. Verified by reading both call sites.
— Wire mappings: SessionId/Generation/GenerationId raw-counter mappings pinned by `wire_mappings_are_the_raw_counters` (353-366). ok in baseline.
— Exhaustion typing: `generation_exhausted` (203-213) — stage Control, kind ResourceLimit, honest context string; the executor converts it to a fatal + Error state (session.rs:1612-1620).

## 3. Findings

### F-001 [S4] — `publish()` silently clears the interrupt flag; `interrupt()`'s doc omits the caveat
Where: crates/playback-engine/src/epoch.rs:186-200
Code:
```rust
    /// Interrupt the token (doc 03 §5 teardown step 2). Idempotent;
    /// every subsequent staleness check answers stale.
    pub fn interrupt(&mut self) {
        {
            let mut state = self.token.lock();
            state.interrupted = true;
        }
    }

    fn publish(&self) {
        let mut state = self.token.lock();
        state.epoch = self.epoch.0;
        state.pipeline = self.pipeline;
        state.interrupted = false;
    }
```
What is wrong: `interrupt()` documents "every subsequent staleness check
answers stale" — but `publish()` (called by every `mint_pipeline`) sets
`interrupted = false`. A mint after an interrupt revives the token: the new
pair passes `is_current` even though teardown had marked the token dead.
In the current wiring this is unreachable: `interrupt` is called only inside
`Effect::DestroySession` (session.rs:1591), the close rows order
`BumpPipeline` before `DestroySession`, and the engine destroys the closed
session (session.rs:1187-1192). But `TokenMinter`/`CancelToken` are
`pub use`d (lib.rs:112) — the invariant lives in the executor's effect
ordering, not in the type, and nothing in epoch.rs states it. Also: the
redundant `{ }` block inside `interrupt()` is a leftover.
Why it matters: the type's own documentation overpromises; the next caller
(a host embedding playback-engine, or a new effect ordering) that mints after
a teardown-interrupt gets a zombie token that blesses new work. Silent
un-interrupt is the worst kind of teardown bug: it fails open.
What correct looks like: either `publish()` refuses to clear a set interrupt
(once interrupted, the minter is dead — mint returns the exhaustion-style
typed error), or the `interrupt()` doc states "until the next mint" and the
executor ordering is asserted by a test.
Confidence: HIGH — Verification: read (both functions + both call sites +
effect order); UNVERIFIED: none (revive path proven unreachable in current
wiring by call-site enumeration).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| struct TokenState | 54-59 | REAL | — | 3 words, Copy |
| struct CancelToken | 64-67 | REAL | — | Arc<Mutex<TokenState>> |
| CancelToken::snapshot | 70-74 | REAL | — | pub alias of pair() |
| CancelToken::is_current | 78-81 | REAL | — | !is_stale |
| CancelToken::is_stale | 91-94 | REAL | — | the doc 03 §5 rule + interrupt; tested |
| CancelToken::interrupted | 100-102 | REAL | — | teardown flag read |
| CancelToken::lock | 104-111 | REAL | — | poison recovery; sections are 3-word copies — claim verified by reading every lock body |
| CancelToken::pair | 113-119 | REAL | — | consistent 3-word read |
| TokenMinter::new | 138-151 | REAL | — | publishes initial pair at construction (state built with it — no publish needed) |
| TokenMinter::token | 153-157 | REAL | — | read-side clone |
| TokenMinter::pair | 159-166 | REAL | — | live pair from &mut authority |
| TokenMinter::mint_pipeline | 168-184 | REAL | — | checked_add, publish, typed exhaustion |
| TokenMinter::interrupt | 186-193 | REAL | S4 | F-001: doc overpromise + redundant braces |
| TokenMinter::publish | 195-200 | REAL | S4 | F-001: clears interrupted |
| generation_exhausted | 203-213 | REAL | — | typed error; budget/attempted u64::MAX is cosmetic (unreachable) |
| SessionEpoch (+FIRST/next/as_session_id) | 217-245 | REAL | — | FIRST=1 documented as PROPOSAL; next checked |
| PipelineGeneration (+FIRST/as_wire/as_queue_generation) | 249-270 | REAL | — | FIRST=0 per doc 05 §4.2; queue-space argument depends on per-epoch queues (executor discipline) |
| GenerationPair (+session_id/wire_generation) | 276-296 | REAL | — | pair identity |
| tests ×4 | 302-366 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| minter_publishes_and_stages_observe (303) | YES | old pair stale + new current after mint — breaks if publish/pair comparison is wrong |
| interrupt_marks_even_the_current_pair_stale (322) | YES | the current pair goes stale on interrupt |
| cross_epoch_equal_pipeline_counters_are_stale (333) | YES | the exact trap bare counters would miss |
| wire_mappings_are_the_raw_counters (353) | YES | raw-counter mappings pinned |
All 4 ok in baseline (orchrun log 3756-3760). Gap: no test pins the
mint-after-interrupt revive behavior either way (F-001) — the semantics are
untested and undocumented.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "different epoch OR different pipeline ⇒ stale" | epoch.rs:19-23 (module doc, doc 03 §5) | REAL | is_stale 91-94 + test 333 |
| "only the session thread mints" | epoch.rs:27-28 | REAL | TokenMinter behind &mut; sole construction session.rs:1439 |
| "Cross-epoch collisions in the queue space are … unobservable by construction: an old-epoch producer only ever holds old-epoch queue handles" | epoch.rs:43-45 | REAL-UNPROVEN (executor-side) | token rule tested; the per-epoch-queue discipline is session.rs/pipeline.rs territory (rg: queues built in the open path) — not proven here |
| "Interrupt … Idempotent; every subsequent staleness check answers stale" | epoch.rs:186-187 | FALSE (incomplete) | publish() clears the flag (F-001) — true only until the next mint |
| "the lock is never held across anything else" | epoch.rs:31-32 | REAL | every critical section is a 3-word copy/read (verified by reading) |

## 7. Dead / duplicated / unreachable code
- `snapshot()` (72-74) is a pub alias of private `pair()` — harmless API sugar.
- The `{ }` block in `interrupt()` — vestigial.
- No dead functions; everything else has live callers (session.rs).

## 8. Dependencies & flags
media-types + commands only; no external deps, no features, no unsafe.
Everything ships in the default CLI build; the module is `pub` (lib.rs:95,
112) so the F-001 trap is exposed to embedders.

## 9. Open questions for the next agent
- session.rs auditor: confirm no code path can run effects after
  `DestroySession` within one session (the `closed` flag is read only at the
  engine layer, 1187 — the executor itself does not gate on it).
- Whoever owns the executor: add the mint-after-interrupt semantics test (or
  make publish refuse to clear) — F-001.
