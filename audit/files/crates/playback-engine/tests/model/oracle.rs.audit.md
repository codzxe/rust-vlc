# AUDIT: crates/playback-engine/tests/model/oracle.rs
| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-17 |
| Lines / functions | 376 / 3 (ModelAnswer enum, model_answer fn, module) |
| File verdict | REAL (the oracle is genuine, total, and consumed by a passing model check — but its "documented state machine" framing is stale, see F-001) |
| Reachable from shipped entry point? | Test-support module (tests/model/), consumed by tests/model_check.rs; the STATE MACHINE it pins (playback-engine/src/state.rs) is the shipped engine's core. Baseline model_check: 5 tests pass (cargo-test-workspace.txt:4265-4276). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 0 |
| Commands run | sed doc 03 §4.6 table (docs/03-engine-plan.md:450-515); sed state.rs:86-98 (enum docs); baseline citation; sed quote verification. |

## 1. One-paragraph verdict
This is a real, total (no wildcard), citation-carrying imperative oracle over the engine's 8-state × 13-input machine, and the three-way proof that consumes it passes (model_check: 5 tests, incl. `table_oracle_and_artifact_agree_on_every_pair`). But the module's headline claim — that it "encodes the **documented** state machine — doc 03 §4.6's rows" — is false against the actual document: doc 03 §4.6 (docs/03-engine-plan.md:460-479) specifies a NINE-state machine (`Probing`, `Demuxing` included), `SwitchTrack → Buffering`, `Ended | Seek → Buffering`, `SetSource → Opening`, and stop ending in "engine → Idle". The oracle encodes none of those: SwitchTrack is a self-transition, Ended+Seek is a typed rejection (doc 05 contradiction, honestly flagged as "Lead-owned"), Open-on-active is a `SourceReplacementPhase3` rejection, Stop lands in a retained `Stopped`. The machine it encodes is the ENGINE's (whose own enum docs admit it "absorbs doc 03's Probing and Demuxing sub-phases in the minimal machine" — state.rs:89-90), so the "two authors, one spec" story is really "two authors, one engine header": doc 03 §4.6 is the stale party, and this file's citations reference rows that do not exist in the doc it names.

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/cargo-test-workspace.txt:4265-4276` -> model_check: "5 passed; 0 failed" including `table_oracle_and_artifact_agree_on_every_pair` and `the_artifact_reasons_match_the_total_rejection_function`.
— verified by read: the match at lines 52-375 has NO wildcard arm; every (state, input) pair is spelled out (compiles ⇒ total over the shared enums).
— verified by sed docs/03-engine-plan.md:450-515: the doc's table diverges as described in F-001.

## 3. Findings
F-001 [S2]
Where: crates/playback-engine/tests/model/oracle.rs:1-11 vs docs/03-engine-plan.md:460-479
Code:
```
//! author): `model_answer` encodes the **documented** state machine —
//! doc 03 §4.6's rows and doc 05 §3.3's wire conformance list — as
```
What is wrong: The header asserts the oracle encodes doc 03 §4.6's rows; the doc's actual table specifies: (a) states `Probing` and `Demuxing` (absent from the engine enum the oracle matches on — state.rs:89-90 admits the absorption); (b) `Playing/Paused/Buffering | SwitchTrack | Buffering` — the oracle answers a same-state work-carrying self-transition (lines 237-248); (c) `Ended | Seek | Buffering` — the oracle rejects (`EndedNeedsWireEdge`, lines 346-349); (d) `any active | SetSource | Opening` — the oracle rejects with `SourceReplacementPhase3`; (e) `any active | Stop | … engine → Idle` — the oracle lands in a retained `Stopped`. Each deviation is documented at its arm, but the arm citations quote "doc 03 §4.6" rows that are NOT in the doc. Why it matters: the three-way proof is advertised as docs ↔ table ↔ oracle; in reality the doc side is stale and the proof is engine-table ↔ oracle. A reader trusting doc 03 §4.6 would expect behaviors the product does not have. What correct looks like: either update doc 03 §4.6 to the minimal machine or retitle the header "encodes the engine's state machine (doc 03 §4.6 v0 reduction)". Confidence: HIGH — Verification: read + sed of both files.
F-002 [S3]
Where: crates/playback-engine/tests/model/oracle.rs:24
Code:
```
use playback_engine::state::{Effect, RejectionReason, SessionInput, SessionState};
```
What is wrong: The "echo chamber kill" (directive D-5) only covers the transition MAPPING. The oracle shares the engine's entire vocabulary (states, inputs, effects, rejection reasons), so a vocabulary-level defect — an Effect variant that should exist per the docs but doesn't, or an input silently removed from the engine enum — compiles cleanly here and shrinks the proof surface without any failure. The totality discipline catches ADDITIONS (new enum variant ⇒ compile error in the oracle) but not removals. Why it matters: the doc 03 §4.6 side-effect vocabulary (e.g. `FirstFrame`, `SessionClosed`, `ClockMasterChanged`) has no Effect counterpart and the oracle cannot notice. Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ModelAnswer (enum) | 28-38 | REAL | — | Row{to, effects} / Reject(reason). |
| model_answer | 47-376 | REAL | S2/S3 | Total 8×13 match, per-arm citations; F-001 stale-doc framing, F-002 shared vocabulary. Notable honest arms: Buffering+Underrun rejected as steady state (183-189); Paused+Primed episode bookend (280-287); Ended+Seek doc-05 conflict (346-349); Error exit-only-Close (365-369). |

## 5. Test honesty
This file IS the oracle (no tests of its own here); honesty attaches to model_check.rs (not assigned to me): baseline shows its 5 tests pass, including pair-exhaustiveness and artifact/table/oracle agreement. The oracle would fail the check if the engine table changed without the oracle (or vice versa) — that is its purpose and it is wired.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "encodes the documented state machine — doc 03 §4.6's rows" | oracle.rs:1-4 | FALSE | doc 03 §4.6 table (docs/03-engine-plan.md:460-479) has 9 states / SwitchTrack→Buffering / Ended-Seek row / SetSource row / stop→Idle — none encoded; F-001. |
| "the match has no wildcard — every (state, input) pair is spelled out" | oracle.rs:13-16 | REAL | Verified by read: no `_` arm; compiles ⇒ total. |
| "Ended | seek … the reconciliation is Lead-owned" | oracle.rs:346-349 | REAL (honest conflict record) | Doc 03 vs doc 05 contradiction surfaced in code; unresolved. |

## 7. Dead / duplicated / unreachable code
None (module is wholly consumed by tests/model_check.rs — rg confirms `model_answer` referenced there).

## 8. Dependencies & flags
playback_engine::state types only; no feature gates; no unsafe.

## 9. Open questions for the next agent
- Whoever audits docs/03-engine-plan.md: §4.6's state machine is stale vs the shipped engine (9 states advertised, 7+Opening absorbed) — ledger it as a doc claim.
- The doc 03 vs doc 05 `ended → buffering` contradiction (oracle.rs:346-349) is marked "Lead-owned" — is there a tracking issue, or is it parked indefinitely?
