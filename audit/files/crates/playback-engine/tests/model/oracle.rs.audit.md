# AUDIT: crates/playback-engine/tests/model/oracle.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 376 / 1 function (`model_answer`, 324 lines) + 1 enum |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | N/A — test-support module (tests/model/) consumed by `tests/model_check.rs` (not in my assignment; the three-way proof driver). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | `cat` full file (exit 0); arm-by-arm cross-diff against crates/playback-engine/src/state.rs TRANSITIONS (sed 371-665) and `rejection_reason` (sed 700-860). No cargo. |

## 1. One-paragraph verdict

A total, wildcard-free imperative encoding of the documented state machine — 104 (state, input) pairs classified as 30 rows + typed rejections, every arm carrying a doc citation. I diffed it arm-by-arm against the engine's `TRANSITIONS` table and `rejection_reason` (state.rs): they agree exactly, including effect ordering, the C8 self-rows, the 3.C1 additions (Opening+Stop, Buffering+Pause/Stop, Paused+Primed bookend), the 3.A1 seek rows, the 3.A3 switch rows, and every rejection family. As a test artifact it is honest and would catch any engine-side table drift. The two caveats are inherent to the genre: the "two authors, independently written" anti-echo-chamber claim (directive D-5, cited in the module doc) is a process claim no code artifact can verify — a faithful transcription of TRANSITIONS would look identical; and the oracle reuses the engine's own `Effect`/`RejectionReason` enum types, so the proof covers the (state,input)→(to,effects) mapping but inherits the engine's vocabulary wholesale — a mislabeled or misordered Effect enum would pass the three-way check untouched.

## 2. What actually works (verified)

- Totality: no wildcard arm exists (read the whole match); every one of the 8×13 pairs is spelled — adding a state or input is a compile error here, exactly as documented (13-16).
- Exact agreement with the engine table: all 30 rows match state.rs TRANSITIONS (spot-verified every row: Open, Idle+Close, Opening+OpenReady/Failed/Close/Stop, Buffering+Primed/EofReached/Play/Pause/Stop/Failed/Seek/SwitchTrack, Playing+Play/Pause/Underrun/Drained/Seek/SwitchTrack/Stop/Failed, Paused+Play/Pause/Primed/Drained/Seek/SwitchTrack/Stop/Failed, Stopped/Ended/Error+Close). All rejection reasons match `rejection_reason`'s arms.
- Citations present per arm (doc 03 §4.6 row / doc 05 §3.3 edge) — the arms are diffable against the prose, as claimed.

## 3. Findings

### F-001 [S4] The "independently authored" anti-echo claim is unverifiable, and the shared Effect vocabulary is outside the proof
Where: crates/playback-engine/tests/model/oracle.rs:5-11 (claim), 24 (the imports)
Code (5-11, verbatim):
```rust
//! It is deliberately NOT generated from
//! (or compared against, in its authorship) the engine's
//! [`TRANSITIONS`] data table: the whole point of the three-way proof
//! (`tests/model_check.rs`) is that two authors writing the same
//! spec-derived truth must agree, killing the self-referential echo
//! chamber where a table is only ever checked against itself
//! (directive D-5).
```
Code (24, verbatim):
```rust
use playback_engine::state::{Effect, RejectionReason, SessionInput, SessionState};
```
What is wrong: nothing checkable — but the claim of independent authorship is a process fact (agent history), not a code fact, and the artifact cannot distinguish "two authors reading the same docs" from "one author transcribing the table". More substantively: the oracle's answers are expressed in the engine's own `Effect` and `RejectionReason` types, so if the engine's effect list for a row were wrong in the same way the oracle spells it, the three-way proof still passes — the proof bounds the mapping, not the vocabulary or the semantics of each effect (those live in session.rs's interpreter, audited separately).
Confidence: HIGH — Verification: read + full cross-diff.
Severity: S4 — this is the honest limit of the method, disclosed here for the wave-20 adjudicators.

### F-002 [S4] The `(S::Buffering, I::Close)` arm documents an unreachable-by-client path with a reason that exists only for table totality
Where: crates/playback-engine/tests/model/oracle.rs:177-181
What is wrong: nothing — the arm correctly mirrors state.rs's `SettleFirst` rejection, and the comment says so ("no client path can reach it"). Recorded because the same totality rationale was the subject of my session.rs F-004 (the executor composes close through stop and re-stamps generations): the oracle pins the *table's* refusal, not the executor's composition, so the three-way proof says nothing about `close_plan`'s macro behavior. That half is pinned only by the session-level contract suites.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ModelAnswer` enum | 28-38 | REAL | — | Row{to, effects} / Reject(reason) |
| `model_answer` | 47-376 | REAL | S4 | total, wildcard-free, doc-cited; 1:1 with TRANSITIONS (verified); independence claim F-001 |

## 5. Test honesty

This file IS the oracle (no assertions of its own). Its consumer `tests/model_check.rs` (another agent) must be checked to confirm it actually drives all 104 pairs against both the table and the oracle — if it samples, the totality above is only latent.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "two authors writing the same spec-derived truth must agree" (D-5) | oracle.rs:5-11 | UNVERIFIABLE (process claim) | F-001 |
| "the match has no wildcard" | oracle.rs:13-16 | REAL | read the whole match — no `_` arm |
| "one match arm per documented rule, each arm citing its source" | oracle.rs:3-5 | REAL | citations present in every arm |

## 7. Dead / duplicated / unreachable code

The deliberate duplication of the table as imperative code is the file's purpose (the anti-echo discipline); not a defect.

## 8. Dependencies & flags

playback-engine (state types) only. No features. Part of the tests/ tree — compiles under `cargo test` only.

## 9. Open questions for the next agent

- Does `tests/model_check.rs` exhaustively enumerate `SessionState::ALL × SessionInput::ALL` (104 pairs) against BOTH `apply_input` and `model_answer`, or sample? The three-way proof's strength depends on it (file: crates/playback-engine/tests/model_check.rs).
