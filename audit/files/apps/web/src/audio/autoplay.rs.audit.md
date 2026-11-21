# AUDIT: apps/web/src/audio/autoplay.rs
| Field | Value |
|---|---|
| Auditor | W12-A05 |
| Date | 2026-11-21 |
| Lines / functions | 197 / 10 (1 struct + 1 enum + 4 methods/impls + 4 tests) |
| File verdict | REAL (clean two-bit state machine, exactly-once surfacing, all 4 tests green and discriminating) |
| Reachable from shipped entry point? | NO from shipped binaries (lumen-web crate UNREACHABLE, W01-A10 F-001). Reachable within the crate: `WebHost` owns `AutoplayGate` (host.rs:813, construction 849) and drives it from `handle(AudioContext)` (1064) + `observe_autoplay`/`fill_audio` (1171-1215) — the same paths the crate's tests execute. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read autoplay.rs 1-198 (full); `rg -n "AutoplayGate|GateChange|autoplay" apps/web/src/host.rs apps/web/src/audio/mod.rs apps/web/src/protocol.rs` (sole consumer = WebHost; re-exports in audio/mod.rs:46); baseline `orchrun-test-workspace-nofailfast.log.txt:3095-3098` (autoplay::tests 4/4 ok) + `:3124` (host-level `the_autoplay_gate_parks_freezes_and_resumes` ok — the integration through WebHost). Quotes verified with the Read tool. |

## 1. One-paragraph verdict
This is the rare file with nothing to indict in its logic: two bits (parked, surfaced), three transitions (`report` arms/clears, `observe` surfaces exactly once per parking cycle when playback wants audio), and every arrival-order combination (report-then-play, play-then-report, duplicates, resume, re-arm) pinned by tests that assert exact `GateChange` values — all green in the repair run, plus a host-level integration test that proves the freeze actually stops the sample-counter clock. The host discards the `Resumed` instruction (host.rs:1064 `let _change =`), which is sound because unfreezing is structural (`is_parked()` gates `fill_audio`), not event-driven. The only findings are documentation-shaped: the module doc promises a `playback.gesture-required` catalog row that the catalog does not have (it is a transport-level event by the Lead's `[PROPOSAL]`, honestly labeled), and the gate has no `reset()` even though its state deliberately outlives sessions — correct behavior, but nothing states that a suspended context parked before a session close stays parked into the next session.

## 2. What actually works (verified)
— Verified by baseline `orchrun-test-workspace-nofailfast.log.txt:3095-3098` → `audio::autoplay::tests::{both_arrival_orders_surface_once, observation_surfaces_once_per_cycle, suspension_cycles_arm_and_clear, the_default_gate_is_open}` all `ok`; `:3124` → `host::tests::the_autoplay_gate_parks_freezes_and_resumes ... ok` (the gate wired through WebHost, pump, fill_audio and the wire event).
— Verified by exhaustive state reasoning: `report`/`observe` over {parked, surfaced} × {running, wants_audio} — 16 combinations traced by hand; every output matches the doc's matrix; no combination double-fires `Parked` and none can surface on an open gate.
— Verified by mutation reasoning: remove the `surfaced` latch (117: always return Parked when parked&&wants) and `observation_surfaces_once_per_cycle` fails at line 163 ("once per cycle"); swap the resume-clear order in `report` and `suspension_cycles_arm_and_clear` fails at 143-150.

## 3. Findings

### F-001 [S4]
Where: apps/web/src/audio/autoplay.rs:25-26 (module doc) vs the served command vocabulary
Code:
```
//! - [`AutoplayGate::report`] applies the page's context-state
//!   report ([`crate::protocol::HostToWorker::AudioContext`]);
```
(with the honest caveat at 35-40):
```
//! wanting audio surfaces the event then). The gate does not know
//! about the command layer's event catalog — the
//! `playback.gesture-required` catalog row is the Lead's additive
//! decision (doc 09 §12 OQ-3, `[PROPOSAL]`); the transport-level
//! event carries the same condition without editing the shared
//! catalog.
```
What is wrong: the doc names a `playback.gesture-required` catalog row as the intended command-layer surface, but no such catalog id exists in the served or unserved command vocabulary (`SERVED` at host.rs:762-775 excludes it; the `GestureRequired` worker message is transport-level only). The doc's own caveat admits the row is a `[PROPOSAL]`, so this is drift-risk, not a false claim — but a reader of this file alone would expect a catalog row to find.
Why it matters: doc/impl vocabulary drift around the ONE user-facing autoplay surface; a UI author reading this module would search the command catalog for a row that is not there.
What correct looks like: state the present truth first ("the only surface today is the transport-level `GestureRequired` worker message; the catalog row is doc 09 §12 OQ-3 `[PROPOSAL]`").
Confidence: HIGH — Verification: read (SERVED + protocol vocabulary checked via rg).
### F-002 [S4]
Where: apps/web/src/audio/autoplay.rs:84-104
Code:
```
    /// Apply the page's context-state report. A fresh suspension
    /// arms the gate; a resume after a park clears it (and re-arms
    /// the once-per-cycle event latch for the next cycle).
    pub fn report(&mut self, running: bool) -> GateChange {
```
What is wrong: the gate has no `reset()` and its docs never state that parked state deliberately outlives a session: `WebHost::teardown_session_scoped` (host.rs:1567-1577) does not touch `autoplay`, so a context that parked during session 1 enters session 2 still parked (correct — the AudioContext is page-scoped, not session-scoped — and it makes the freeze apply immediately to the new session), but this cross-session semantics is nowhere written and no test covers it.
Why it matters: the behavior is right; the silence is the defect — the next maintainer could "fix" the leak by resetting the gate at open and break the parked-invariant for the second session.
What correct looks like: one doc line on the struct ("page-scoped: survives session teardown by design — the AudioContext does") + one host-level test (park, close, open, assert still frozen).
Confidence: HIGH — Verification: read (teardown fields enumerated at host.rs:1567-1577; autoplay absent).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| struct AutoplayGate (2 fields) | 44-51 | REAL | — | Copy + Debug/PartialEq — value semantics suit the host's single owner |
| enum GateChange (3 variants) | 53-64 | REAL | — | None/Parked/Resumed; `Resumed` intentionally unconsumed by the host (unfreeze is structural via `is_parked`) |
| AutoplayGate::new | 67-75 | REAL | — | open gate; Default delegates (121-125) |
| is_parked | 77-82 | REAL | — | the freeze predicate `fill_audio` reads (host.rs:1213) |
| report | 87-104 | REAL | S4 | fresh suspension arms + re-arms latch; duplicate suspended reports keep `surfaced` (no re-fire); resume clears both and answers `Resumed` once; cross-session semantics undocumented = F-002 |
| observe | 111-118 | REAL | — | parked ∧ wants_audio ∧ ¬surfaced ⇒ Parked once; the join point for both arrival orders |
| Default impl | 121-125 | REAL | — | delegates to new |
| tests: suspension_cycles_arm_and_clear | 133-151 | REAL | — | full report matrix incl. duplicate + re-arm; discriminating |
| tests: observation_surfaces_once_per_cycle | 156-171 | REAL | — | once-per-cycle, not-wanting, next cycle re-surfaces; discriminating |
| tests: both_arrival_orders_surface_once | 175-188 | REAL | — | the two orderings the doc promises; discriminating |
| tests: the_default_gate_is_open | 191-196 | REAL | — | headless-lane truth; value equality with new() |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| suspension_cycles_arm_and_clear | YES | exact GateChange equality on every step; a lost `Resumed` or a re-firing arm breaks an assert |
| observation_surfaces_once_per_cycle | YES | the second `observe(true)` must be `None` — latch removal fails it |
| both_arrival_orders_surface_once | YES | order-2 (play first) catches an observe-only-on-report bug |
| the_default_gate_is_open | WEAK but honest | pins the default; trivially discriminating for a flipped default |
| (host-level) the_autoplay_gate_parks_freezes_and_resumes | YES | proves the gate actually freezes the clock through fill_audio and ships exactly one wire event (executed ok, repair run 3124) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "parked + playback wanting audio + not yet surfaced ⇒ surface the event once … both arrival orders fire exactly one event" | autoplay.rs:106-110 | REAL | both_arrival_orders_surface_once + observation_surfaces_once_per_cycle green |
| "never fake playing: … the audio lane stops rendering … the sample-counter clock freezes" | autoplay.rs:13-17 | REAL | host.rs fill_audio 1213-1215 park gate + host test 3124 green |
| "A report that arrives while nothing wants to flow parks silently (the gate is armed; the first observation … surfaces the event then)" | autoplay.rs:33-35 | REAL | report returns None on arm (103); observe is the join (111-118) |
| "the `playback.gesture-required` catalog row is the Lead's additive decision (doc 09 §12 OQ-3, `[PROPOSAL]`)" | autoplay.rs:35-40 | REAL-UNPROVEN | no catalog row exists (SERVED/unserved checked) — the doc itself marks it a proposal; F-001 notes the drift risk |

## 7. Dead / duplicated / unreachable code
None. `GateChange::Resumed` is produced and intentionally discarded by the only consumer (host.rs:1064) — not dead code (tests assert it), and the discard is documented inline.

## 8. Dependencies & flags
None beyond std derives. No feature gates. Re-exported via `crate::audio::{AutoplayGate, GateChange}` (audio/mod.rs:46).

## 9. Open questions for the next agent
- W12-A03 (audio-worklet.js): the worklet is documented as the sink MODEL's transcription (audio/mod.rs docs) — does the worklet also mirror the gate's park semantics, or does the page's AudioContext state report reach the worker at all in the JS glue (worker.js, W12-A08)? If the glue never sends `AudioContext { running }`, this gate is armed but never driven in the browser — everything stays headless-honest, but the doc 09 §5.3 flow would be browser-dead.
