# AUDIT: crates/playback-engine/src/epoch.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 367 / TokenState, CancelToken::{snapshot, is_current, is_stale, interrupted, lock, pair}, TokenMinter::{new, token, pair, mint_pipeline, interrupt, publish}, generation_exhausted, SessionEpoch::{FIRST, next, as_session_id}, PipelineGeneration::{FIRST, as_wire, as_queue_generation}, GenerationPair::{session_id, wire_generation} + 4 tests (298-367) |
| File verdict | **REAL** — the live cancellation-generation machinery behind every open/seek/switch/stop/close |
| Reachable from shipped entry point? | YES — `session.rs:1439` (`minter: TokenMinter::new(epoch)` in `Session::new`); `Effect::BumpPipeline` → `mint_pipeline` (session.rs:1612) on the seek/switch/stop/close rows (state.rs:418/542/553/566/574/583/651); `Effect::DestroySession` → `interrupt` (session.rs:1591); the W00 smoke's `gen=0` in `ok: accepted session=1 gen=0` is this module's wire form. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg (mint/interrupt/epoch call sites in session.rs + state.rs rows; source-swap 3.A2 status); sed reads of session.rs:1580-1640/1175-1196, state.rs:800-830. No cargo (static). |

## 1. One-paragraph verdict

This is the one file so far that earns a clean bill — with the proof listed, not asserted: 367 lines, zero unsafe, zero allocation in steady state (a `Mutex<TokenState>` of three words), poison recovery documented (epoch.rs:104-111), checked counter exhaustion with a typed error (176-184, 233-238), and the doc 03 §5 pair rule implemented exactly (`is_stale`: interrupted OR epoch differs OR pipeline differs — line 93). It is wired into the live command path (every gen-bumping row mints through it) and the W00 smoke shows the wire form working (`session=1 gen=0`). The two defects are documentation staleness (both named "future" cards — 3.A2 source replacement and Phase-3 seek/switch minting — have landed, per docs/gates/phase3-exit.md:103 and the state-table rows) and one latent ordering trap: `publish()` clears the interrupt flag, so a mint after an interrupt would silently revoke teardown cancellation — defended today only by row ordering (`[BumpPipeline, DestroySession]`, state.rs:418/651), Idle-state rejections (state.rs:810), and session destruction at poll end (session.rs:1187-1196), not by any guard in this module.

## 2. What actually works (verified)

- `is_stale`/`is_current` (79-94): the doc 03 §5 pair comparison plus the interrupt flag; cross-epoch equal counters are stale (test at 333-350 proves it — the exact trap the module exists to prevent).
- `mint_pipeline` (176-184): checked_add → typed `ResourceLimit` error at u64 exhaustion; publish is a three-word critical section.
- `interrupt` (188-193): idempotent; even the current pair goes stale (test 322-330).
- Wire mappings (242-244, 259-261, 267-269): raw counter passthroughs, asserted by test 353-366.
- Reachability: session.rs:1439/1591/1612; `SessionEpoch::FIRST` at session.rs:990/1204; the events.rs staleness stamp (this agent's first file) differential-checks against this module's token (events.rs:1901-1922).
- W00 smoke: `ok: accepted session=1 gen=0` — the epoch/pipeline pair on the wire from the shipped binary.

## 3. Findings

### F-001 [S4] — stale deferral comments: both "future" cards have landed
Where: crates/playback-engine/src/epoch.rs:7-8 and 15-16, 169-170
Code (epoch.rs:15-17):
```rust
//! that **resets each epoch** and is minted on the gen-bumping
//! events — in the minimal machine, `stop` (final) and `close`
//! (terminal); seek/track-switch joins with Phase 3. On the wire
```
Code (epoch.rs:7-8):
```rust
//! - **Epoch** (session generation, [`SessionEpoch`]): minted on
//!   `Open` (and on source replacement once card 3.A2 wires it).
```
What is wrong: both deferrals describe the past. Card 3.A2 (source replacement) is COMPLETE — docs/gates/phase3-exit.md:103 ("W5R-03 `tests/source_swap.rs` 4/4 green … the epoch bump (fresh session id, dead old handle)") and the suite exists (crates/playback-engine/tests/source_swap.rs). Phase 3 landed: the seek rows (state.rs:542, 553) and track-switch rows (state.rs:566, 574, 583) all carry `Effect::BumpPipeline`, so "the minimal machine mints on `stop` and `close` only" (also at 169-170) is false today.
Why it matters: the module doc misdescribes the minting surface — a reader auditing generation flow would look for two mint sites and miss seven.
What correct looks like: update both comments to the landed reality.
Confidence: HIGH — Verification: read (state.rs rows cited; gate doc cited).

### F-002 [S4] — `publish()` clears the interrupt flag; a mint after an interrupt silently revokes teardown cancellation
Where: crates/playback-engine/src/epoch.rs:188-200
Code:
```rust
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
What is wrong: `mint_pipeline` has no guard against minting while interrupted, and `publish` unconditionally writes `interrupted: false`. The invariant "an interrupted token stays interrupted" is enforced nowhere in this module; it survives only because (a) transition rows order `BumpPipeline` before `DestroySession` (state.rs:418, 651), (b) after close the state is Idle where Play/Pause/Stop/Seek/SwitchTrack are rejected (`(S::Idle, I::Play | …) => R::NoSession`, state.rs:810), and (c) the engine destroys the closed session at poll end (session.rs:1187-1196). None of those defenses lives here, and (c) leaves a same-poll window: commands queued behind a close are applied to the still-live session object before the destruction check.
Why it matters: latent trap for any future mint site (or a new row ordering) — the failure mode is silent revocation of cancellation, the worst kind. Not currently reachable in a way that matters (post-close mints would hit Idle rejections; Open-on-closed-session routing is the engine's business, session.rs owner's file).
What correct looks like: `mint_pipeline` returns the exhausted-style typed error (or asserts) when `interrupted`; or `publish` keeps the flag.
Confidence: MEDIUM — Verification: read (control flow here + session.rs/state.rs defenses); the same-poll window is reasoned, not executed.
Severity: S4 because every current path is defended; it is a missing local invariant, not a live bug.

## 4. Per-function table

| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| TokenState | 54-59 | REAL | — | three words, Copy |
| CancelToken::snapshot | 71-74 | REAL | — | public alias of `pair()` — two names, one operation (API-surface redundancy, harmless) |
| CancelToken::is_current | 78-81 | REAL | — | negation of is_stale |
| CancelToken::is_stale | 90-94 | REAL | — | the doc 03 §5 rule + interrupt; lock + 3 compares; poison-recovered lock (104-111) |
| CancelToken::interrupted | 99-102 | REAL | — | flag read |
| CancelToken::lock / pair | 104-119 | REAL | — | poison → into_inner, documented |
| TokenMinter::new | 138-151 | REAL | — | publishes epoch + pipeline 0 + not-interrupted atomically |
| TokenMinter::token | 154-157 | REAL | — | Arc clone for stages |
| TokenMinter::pair | 161-166 | REAL | — | live pair (no lock — session-thread authority) |
| TokenMinter::mint_pipeline | 176-184 | REAL | S4 | checked_add; no interrupted-guard (F-002) |
| TokenMinter::interrupt | 188-193 | REAL | — | idempotent |
| TokenMinter::publish | 195-200 | REAL | S4 | clears the interrupt flag (F-002) |
| generation_exhausted | 203-213 | REAL | — | typed ResourceLimit error, context code `generation.exhausted` |
| SessionEpoch::{FIRST, next, as_session_id} | 226-244 | REAL | — | FIRST=1 documented PROPOSAL; next checked |
| PipelineGeneration::{FIRST, as_wire, as_queue_generation} | 255-269 | REAL | — | queue space = pipeline counter within epoch (per-epoch queues, module doc 35-45) |
| GenerationPair::{session_id, wire_generation} | 287-295 | REAL | — | passthroughs |

## 5. Test honesty (4 tests, lines 298-367)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| minter_publishes_and_stages_observe (303) | YES | publish-observe semantics; old pair stale after mint |
| interrupt_marks_even_the_current_pair_stale (322) | YES | the teardown invalidation |
| cross_epoch_equal_pipeline_counters_are_stale (333) | YES | the masquerade case — the module's core property |
| wire_mappings_are_the_raw_counters (353) | YES | all three mappings |

Small but exact. The heavier proofs live cross-file: tests/epoch_staleness_alloc.rs (zero-alloc drain decisions, cited by events.rs) and tests/source_swap.rs (50 epoch bumps with in-flight seeks). UNVERIFIED by run here.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "different epoch OR different pipeline ⇒ stale" | epoch.rs:19-23, doc 03 §5 | REAL | is_stale:91-94 + test 333 |
| "Only the session thread mints" | epoch.rs:27-28 | REAL | TokenMinter behind &mut in session.rs:1439; rg: no other constructor of TokenMinter in production |
| "source replacement once card 3.A2 wires it" | epoch.rs:7-8 | FALSE (stale) | 3.A2 COMPLETE (gate doc :103; tests/source_swap.rs) — F-001 |
| "the minimal machine mints on `stop` and `close` only" | epoch.rs:169-170 | FALSE (stale) | seek/switch rows also mint (state.rs:542-583) — F-001 |
| "Cross-epoch collisions in the queue space are therefore unobservable" | epoch.rs:43-45 | REAL-UNPROVEN | per-epoch queue construction verified in session teardown; adversarial proof is source_swap.rs's job (not run) |

## 7. Dead / duplicated / unreachable code

- `snapshot()` (71-74) duplicates `pair()` as public API — one caller set would do. Trivial.
- Nothing dead: every item has production callers (session.rs, pipeline stages via `token()`).

## 8. Dependencies & flags

- Depends on `commands::{Generation, SessionId}`, `media_types::{EngineError, GenerationId}`. No features, no unsafe, std Mutex only. The lock is never held across anything but three-word copies (verified 92-93, 113-118, 190-199).

## 9. Open questions for the next agent

- session.rs / Engine owner: confirm the same-poll command-after-close window (F-002's reasoning) — does `Engine::poll` apply queued commands to a session whose `closed` flag is already set before the destruction check at 1187-1196? If yes, the trap is one new transition row away from live.
- state.rs owner: the `[BumpPipeline, DestroySession]` ordering is load-bearing for F-002 — worth a comment there saying so.
