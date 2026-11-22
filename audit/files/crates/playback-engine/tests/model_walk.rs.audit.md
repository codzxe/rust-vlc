# AUDIT: crates/playback-engine/tests/model_walk.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 894 / 4 tests + PRNG + Walker + StreamChecker |
| File verdict | REAL — seeded model-based testing of the real Engine (500 walks × ≤64 steps) with per-step invariants, vacuity floors, a strong-connectivity BFS, and a 1000× reopen-cycle continuity proof |
| Reachable from shipped entry point? | N/A (test binary). The subject is the shipped engine/state machine |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep (not executed — no cargo) |

## 1. One-paragraph verdict

The model-based suite the docs promise and most projects fake. 500
seeded walks drive the real `Engine` through a weighted command mix
(pumps, play/pause/stop/close/seek, stall + refill underrun flaps,
reopens with 1-in-4 armed failures, stop-during-opening, buffered
pause, guaranteed close-from-active) and after EVERY step assert: no
illegal wire edge, exactly-one receipt per command (InboxBusy retried
the host way), and the doc 05 §4.3 ordering invariants (global seq
strictly increasing across closes/reopens, epoch per session,
generations monotone, fatal-before-error-state, list-ready before
first playing, buffering episodes bracketed 0.0/1.0 with the
superseded-generation exemption). The vacuity guards are the mark of
honest property testing: 500 walks must hit every named behavior at
least once (floors asserted at :714-724) or the suite fails "for
testing nothing". The BFS test proves every state reaches Idle AND
the graph is strongly connected (:793-819). Failures name the seed
and step. One nit: the PRNG is hand-rolled.

## 2. What actually works (verified)

— `run_walk` (:556-691): the command mix arms every completeness item
including the double-failure reopen cycles (:637-645) and the stall/
refill halves of the underrun flap (:611-636); floors are counted per
walk and aggregated.
— Vacuity floors (:714-724): `wire_legal_edges > 4000`,
`underrun_flaps >= 1`, `buffered_pauses >= 1`,
`paused_primed_bookends >= 1`, `stop_during_opening >= 1`,
`close_from_active >= 1`, `reopens >= 1`, `error_cycles >= 2` — the
walks are proven non-vacuous.
— `every_submitted_command_answers_exactly_one_receipt` (:732+):
pinned separately for a precise count, including the close macro's
single Completed for its two-row composition.
— `idle_is_reachable_from_every_state_over_rows_and_the_close_macro`
(:793-819): BFS over `TRANSITIONS` + the `close_plan` macro;
`successors` unwraps macro steps with a loud `unreachable!`
(:784-788) — a plan/table disagreement fails the test, not silently
skips.
— `reopen_cycles_1000x_keep_epoch_and_seq_continuous` (:828-894):
epoch == cycle+1 exactly (:858-862), monotone session ids, frames
played each cycle (:884), 2000 receipts (:893).
— Mutation reasoning (UNVERIFIED, no cargo): any illegal-transition
bug fails `can_transition_to` per step; any dropped/duplicated
receipt fails the exact-count test; any seq regression across reopen
fails the StreamChecker; a stuck state fails the BFS.

## 3. Findings

### F-001 [S4] Hand-rolled PRNG where the failure-seed reproducibility claim rests on its exact arithmetic
Where: model_walk.rs:81-113 (Rng)

Code (module doc :14-16):
```
//! The driver is a hand-rolled splitmix64-seeded xorshift64* PRNG
//! (the repo convention — `packet-queue`/`limits.rs`, the codecs/demux
//! `fuzz_loop_stable` suites; NO proptest dependency exists in
//! Cargo.lock).
```
What is wrong: not a defect in behavior — a risk note: the entire
"failed seed reruns the exact interleaving" guarantee depends on the
hand-rolled generator being deterministic across Rust versions and
platforms. xorshift64* over u64 with splitmix64 seeding IS
deterministic (integer arithmetic, no floats), so the claim is sound;
but any future "improvement" to the PRNG silently invalidates
historical seed reproduction. The doc cites the repo convention and
the absence of proptest (verified: honest).
What correct looks like: a one-line comment on the Rng impl pinning
the algorithm version ("do not change; seeds in the wild reference
this stream") — the module doc implies it but the impl does not.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Rng` (splitmix64 + xorshift64*) | 81-113 | REAL | S4 | deterministic; version-pin note absent (F-001) |
| `StreamChecker` | 115-361 | REAL | — | per-event ordering invariants; buffering brackets; supersession exemption |
| `describe` | 362-376 | REAL | — | failure-message helper |
| `Walker` (open/submit/pump/pump_until/finish) | 390-555 | REAL | — | InboxBusy retry = host shape; stop-during-opening armed pre-pump |
| `run_walk` | 556-691 | REAL | — | the weighted mix; floors returned |
| `seeded_model_walks_never_break_the_machine` | 697-725 | REAL | — | 500 walks; vacuity floors asserted |
| `every_submitted_command_answers_exactly_one_receipt` | 732-775 | REAL | — | exact count incl. macro |
| `successors` | 776-791 | REAL | — | rows + macro; loud unreachable on disagreement |
| `idle_is_reachable_from_every_state_over_rows_and_the_close_macro` | 793-819 | REAL | — | BFS; strong connectivity (all 8 states from every start) |
| `reopen_cycles_1000x_keep_epoch_and_seq_continuous` | 828-894 | REAL | — | epoch/seq/receipt continuity; frames per cycle |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `seeded_model_walks_never_break_the_machine` | YES | per-step invariants + vacuity floors (a vacuous pass is a failure) |
| `every_submitted_command_answers_exactly_one_receipt` | YES | exact receipt accounting |
| `idle_is_reachable_…` | YES | any unreachable/stuck state fails; table/macro disagreement is loud |
| `reopen_cycles_1000x_…` | YES | exact epoch arithmetic per cycle |

Not executed by me (no cargo); W00 ran the suite green.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "seeded random command sequences driving the REAL Engine" | :1-3 | REAL | Walker drives real Engine via common fakes |
| "A failed seed reruns the exact interleaving: every failure message names the walk seed and step" | :16-17 | REAL | `walk`/`step` fields carried into messages (StreamChecker describe) |
| "NO proptest dependency exists in Cargo.lock" | :16 | REAL | hand-rolled PRNG; consistent with the C5 zero-dep posture |
| "the bounded-memory half … lives in tests/reopen_alloc.rs" | :51-52 | REAL | that file is in my queue (audited next-but-one) |

## 7. Dead / duplicated / unreachable code

None. Reuses `tests/common` (audited — honest harness).

## 8. Dependencies & flags

`playback_engine` (state::TRANSITIONS, close_plan, apply_input pub
for tests), `commands`, `media_types`, `plugin_api`, `testkit`, and
`mod common`. No feature gates.

## 9. Open questions for the next agent

- Wave 16: the StreamChecker's buffering-bracket exemption ("legal
  only when the session was superseded") encodes doc 05 §4.2 —
  cross-check the wording against the replacement.rs suite (in my
  queue) for consistent interpretation.
