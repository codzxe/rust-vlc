# AUDIT: crates/playback-engine/src/session.rs#part3 (chunk: lines 1220-1834 of 3059)
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-21 |
| Lines / functions | 615 audited (1220-1834) / 18 units |
| File verdict (chunk) | REAL (implemented, reachable from the shipped `lumen` CLI, executed in baseline smoke runs) |
| Reachable from shipped entry point? | YES — `apps/cli/src/host.rs:92` imports `Engine`; host.rs:231 `Engine::new()`; `CliHost::pump` calls `engine.poll()` → `Session::apply_command`/`Session::apply` (this chunk); `apps/web/src/host.rs:96` and apps/desktop use the same crate. Baseline execution: audit/runs/baseline/avi-firstvertical-open-session.txt (full Idle→Opening→Buffering→Playing→Stopped→Idle trail), mp4-smoke-open-session-420.txt (plays). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `sed -n 1220-1840p` (full chunk, read twice); `rg -n "^(pub )?(fn|struct|enum|impl…)"` (function inventory); `sed -n 1061-1219p` (Engine::open/poll — cross-chunk verification of R5-R6 and closed-session drop); `sed -n 781-900p state.rs` (apply_input/rejection_reason/close_plan — verified every table row landing outside the buffering family and the total rejection match); `rg` over tests/ (session_flow.rs, replacement.rs, table.rs); `sed -n 475-543p tests/session_flow.rs`; Read+od verification of all quotes (Bash `#[m` display-mangling). |

NOTE: quotes verified with the Read tool (true bytes) — the Bash tool display
eats `[m` after `#` (e.g. `#[must_use]` shows as `#ust_use]`).

## 1. One-paragraph verdict
This chunk is the session core — the effect interpreter (`Session::apply`),
the emission helpers, the session/runtime/emission state structs, and the
engine's session-replacement flow — and it is real, disciplined code: typed
rejections instead of unwraps, saturated counters, episode bookends owed and
paid, totals merged before drops, receipts-before-events ordering. It is
reachable from the shipped CLI and executed by the baseline smoke runs. I found
no correctness defect with a realistic trigger; the four findings are hygiene
edges on practically-unreachable paths (u64 generation exhaustion, v0 effect
aliasing, placeholder error numbers, a dead defensive default). The state
machine's bookkeeping invariants (episode 1.0 bookend, damage flush before
settling state-changed, starvation episode counting) are each covered by an
honest integration test that fails under the obvious mutations.

## 2. What actually works (verified)
— Full open lifecycle through this code: audit/runs/baseline/avi-firstvertical-open-session.txt → `Buffering → Playing → Stopped → Idle`, EXIT=0; and rc-smoke-engine-only.log.txt → 6/6 engine checks.
— Workspace test suite executes this crate green: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt (4534 passed / 7 failed; none of the 7 in playback-engine).
— Episode bookend ordering: tests/session_flow.rs:475 `stop_while_buffering_closes_the_episode` asserts the 1.0 precedes `→ stopped` and exactly one exit — fails if the bookend block (session.rs:1700-1723) is removed.

## 3. Findings

### F-001 [S4]
Where: crates/playback-engine/src/session.rs:1612-1624
Code:
```
                    if let Err(error) = self.minter.mint_pipeline() {
                        outputs.push(self.emit(EngineEventKind::Fatal { error }, origin));
                        let from = self.state;
                        self.state = SessionState::Error;
                        outputs.push(self.emit(
                            EngineEventKind::StateChanged {
                                from,
                                to: SessionState::Error,
                            },
                            origin,
                        ));
                        return Ok(outputs);
```
What is wrong: the mint-exhaustion early return bypasses the executor's
postlude — the buffering-episode bookend (lines 1700-1723) and the damage
flush (1725-1734) never run for this transition, and `episode_open`/pending
damage survive into the Error state. They self-heal at the *next* settling
transition (e.g. the Close from Error), so the 1.0 is deferred, not lost —
but the "closes at the settling transition" discipline is violated on this
path.
Why it matters: an invariant documented as absolute ("an episode that never
closes strands a 0.0 on the wire") has one path where the close is late. In
practice unreachable: it requires 2^64 pipeline generations minted in one
session.
What correct looks like: route the mint failure through the same postlude
(set episode_open=false + flush damage) before returning, or document the
deferral in the invariant.
Confidence: HIGH — Verification: read.

### F-002 [S4]
Where: crates/playback-engine/src/session.rs:1648-1653
Code:
```
                Effect::FreezeClock | Effect::StopClock => {
                    // Underrun/pause entry freezes; Ended freezes at
                    // the end position (v0: pause — see the effect's
                    // docs).
                    self.runtime.clock.pause();
                }
```
What is wrong: `StopClock` is aliased to `clock.pause()` — a "stop" that is
indistinguishable from a freeze. The two effects exist as distinct vocabulary
but share one behavior (admitted inline: "v0: pause").
Why it matters: vocabulary that promises a distinction the code does not
deliver is exactly the naming-lies class this audit hunts; if the media clock
ever grows stop-specific semantics (e.g. reset-on-resume), this arm is a
trap. Today the observable behavior is correct (both freeze position).
What correct looks like: implement StopClock against the clock's stop surface
when it exists, or collapse the two into one effect until then.
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/playback-engine/src/session.rs:1285-1299 (quote 1290-1293)
Code:
```
                limit: media_types::error::LimitKind::SessionGenerations,
                budget: u64::MAX,
                attempted: u64::MAX,
```
What is wrong: the epoch-exhaustion error fills `budget`/`attempted` with
`u64::MAX` placeholders — numbers that assert a measurement that was never
made. The real fact is "the u64 epoch counter space is exhausted".
Why it matters: a typed error carrying fabricated numbers is a small lie in a
codebase whose whole discipline is honest diagnostics; a consumer rendering
"budget: 18446744073709551615" learns nothing true.
What correct looks like: attempted = the real counter value, budget = the
counter's true bound, or an error kind without numeric fields.
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/playback-engine/src/session.rs:1826-1829
Code:
```
        let mut resolved_switch: Option<(WiredKind, ResolvedSelection)> = None;
        if let Some(kind) = command.kind.selection_kind() {
            let selector = command.selector.unwrap_or(TrackSelector::Auto);
```
What is wrong: dead defensive default. `needs_payload` (lines 1805-1826)
already rejected every Select command whose `selector` is None
(`RejectionReason::MissingPayload`), and `selection_kind()` is `Some` only
for the three Select kinds — so the `unwrap_or` arm is unreachable. (The
same pattern repeats at line 2231 inside `subtitle_select_step`, part4's
range.)
Why it matters: unreachable defaults mask future guard regressions — if the
payload guard were ever weakened, a None selector would silently resolve to
`Auto` instead of failing loudly.
What correct looks like: the honest form is a debug_assert or restructuring
so the type system carries the invariant (an `expect` would violate the
no-panic discipline).
Confidence: HIGH — Verification: read (both guards traced).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Engine::current_pair` | 1220-1223 | REAL | — | live pair or last_pair; documented "live (or last)" semantics |
| `Engine::replace_live_session` | 1245-1277 | REAL | — | implicit close via pinned table rows; rejection honored as Invariant (never unwrapped); totals merged before drop; last_pair claim verified against Engine::open:1088-1089 (R5-R6) — on the open-*failure* path last_pair stays stale, but that matches the documented "or last" accessor semantics |
| `impl Default for Engine` | 1279-1283 | REAL | — | delegates |
| `SessionError::generation_exhausted_boxed` | 1285-1299 | REAL | S4 | F-003 placeholder numbers |
| `struct InputContext` | 1304-1328 | REAL | — | plain payload carrier, Default |
| `struct Session` | 1330-1367 | REAL | — | 17 fields, each documented; single-writer invariant stated |
| `struct EmissionState` | 1373-1417 | REAL | — | policy baselines; steady-state no-alloc claim consistent with emit paths |
| `struct SessionRuntime` | 1400-1418 | REAL | — | retains config parts; injected MonotonicSource for tests |
| `Session::new` | 1420-1464 | REAL | — | fresh Idle session; emission state zeroed |
| `Session::pair` | 1466-1468 | REAL | — | delegate to minter |
| `Session::emit` | 1470-1472 | REAL | — | seq fetch_add+1 — engine-wide monotonic; single-writer so Relaxed is sound |
| `Session::emit_hook` | 1475-1477 | REAL | — | hotplug seam shares the emission path |
| `Session::emit_raw` | 1487-1505 | REAL | — | borrow-splitting static form |
| `Session::emit_terminal` | 1519-1543 | REAL | — | stamps completed generation, not live pair; divergence case documented |
| `Session::apply` | 1553-1752 | REAL | S4 | the effect interpreter: every effect traced — fatal fallback constructs typed internal error (no panic); DestroySession leaves in-flight slots set (harmless: poll step 3, lines 1192-1198, drops the closed session — verified); BumpPipeline failure path F-001; FreezeClock\|StopClock alias F-002; pause-entry immediate tick condition correct (from==Playing && to==Paused excludes C8 self-transitions); Underrun episode counted post-row-lookup (rejections not counted) |
| `Session::flush_damage` | 1760-1782 | REAL | — | only nonzero stages emitted; Some((0,0)) emits nothing (correct) |
| `Session::position_fraction` | 1787-1795 | REAL | — | duration<=0 → 0.0 honest floor; clamp 0..1; negative positions clamp to 0 |
| `Session::apply_command` (start; continues in part4) | 1805-1834 | REAL | S4 | MissingPayload guard rejects payload-less commands typed; class-S subtitle delegation; TrackNotFound rejection with no mint (documented asymmetry); F-004 dead default at 1828 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| session_flow.rs:475 `stop_while_buffering_closes_the_episode` | YES | asserts exit_at < stopped_at and exactly one 1.0 — breaks if the bookend block (1700-1723) is deleted or mis-ordered |
| session_flow.rs:252 `starvation_metrics_count_the_underrun_episode_in_engine_stats` | YES | asserts the engine-stats episode count — breaks if the `saturating_add` at 1572 is removed |
| session_flow.rs:1093 `receipts_precede_the_events_they_cause` | YES | index-ordering assertion over the receipt/event stream — breaks if poll pushes receipt after outputs (session.rs:1164-1169) |
| session_flow.rs:940 `a_second_open_from_playing_replaces_the_session` | YES | replacement flow drives `replace_live_session` — breaks if the implicit close's event trail changes |
| session_flow.rs:67/354/395/417 etc. | YES | event-chain exact-sequence assertions through `apply` |
No tautologies found in the tests read; fixtures are hand-built demuxer/decoder doubles implementing the real traits (the sanctioned test double), not mocks of `Session` itself.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "an episode that never closes strands a 0.0 on the wire" (never-lose-the-last-fact) | session.rs:1693-1700 comment | REAL with one exception | bookend block + tests/session_flow.rs:475; exception: F-001 mint-exhaustion path defers the close |
| "receipt is returned separately so the caller can order it before the events" (doc 05 §4.3 g2) | session.rs:1801-1803 | REAL | poll:1164-1169 pushes receipt first; test receipts_precede… |
| "exactly one terminal event per completed generation" (doc 05 §4.3 inv 4) | session.rs:1519-1531 | REAL-UNPROVEN (this chunk) | emit_terminal implements the stamping; the "exactly one" half is enforced by callers in parts 4/5 (seek/switch steps) — outside this chunk |
| "last_pair needs no update — R5-R6 overwrite it before any drain can observe the gap" | session.rs:1262-1264 | REAL | Engine::open:1088-1089 sets last_pair/session before returning; drains happen in later polls (verified: no drain call between) |
| R2-R3 replacement = shared close_plan macro composition | session.rs:1227-1243 | REAL | close_plan (state.rs:886-897) returns STOP_THEN_CLOSE/CLOSE_ONLY; rejection_reason table (state.rs:796-878) confirms every plan step is a row |

## 7. Dead / duplicated / unreachable code
- None dead in this chunk. The `unwrap_or(TrackSelector::Auto)` default (F-004)
  is dead-but-defensive. Cross-chunk duplication note: the
  selector-resolution preamble pattern appears at both 1826-1840 (class-A) and
  2231 (class-S) — near-identical shape, different effects; acceptable.

## 8. Dependencies & flags
- No feature flags in this chunk; pure crate-internal types plus media-types /
  media-clock vocabulary. `pub(crate)` discipline holds (Session, InputContext,
  SessionRuntime, emit_hook, apply).

## 9. Open questions for the next agent
- part4 (W03-A09): does `seek_pipeline_step`/`switch_pipeline_step` uphold
  "exactly one terminal per completed generation" (the emit_terminal callers)?
- Does any path apply `SessionInput::Open` to a live session (the
  `SourceReplacementPhase3` rejection suggests replacement is engine-level
  only)? If a future path does, `Session::new` state assumptions hold but
  `opening_attempted` (set where?) deserves a look in part4/5.
