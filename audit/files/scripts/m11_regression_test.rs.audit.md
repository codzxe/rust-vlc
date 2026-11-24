# AUDIT: scripts/m11_regression_test.rs
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Date | 2026-11-24 |
| Lines / functions | 93 / 3 (`open_on`, `prime_playing`, `superseded_epoch_events_never_leak_into_the_next_session`) |
| File verdict | UNREACHABLE (as a test: never compiled — scripts/ is not a cargo member; content REAL: byte-equivalent to the landed, baseline-green test) |
| Reachable from shipped entry point? | NO — no build target includes scripts/; the only reference is a worklog line (docs/worklog/archive-2026-10.md:4236). The LIVE copy of this exact test is crates/playback-engine/tests/session_flow.rs:1966-2017, which the orchestrator baseline ran green (cargo-test-workspace.txt:4472). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | grep for references (1 hit: worklog); read + compare landed copy at session_flow.rs:1933-2017; baseline grep `... ok`; ls scripts/*.rs (this is the only stray .rs) |

## 1. One-paragraph verdict
This file is a scratch duplicate of a real regression test, left behind in scripts/ where no compiler ever sees it. The test it contains is genuine and good — it reproduced the M11 bug class (stale-epoch event leak), asserts the fix counts its suppressions, AND asserts the fix does not overreach (dead-session receipts survive). The live copy landed in playback-engine's session_flow suite and passed in the orchestrator baseline. The defect is the location: a "regression test" file that is not wired anywhere is a trap for the next maintainer, and its existence lets a reader cite scripts/m11_regression_test.rs as coverage that does not exist.

## 2. What actually works (verified)
— verified by comparison: the test body (L40-92) is line-for-line the same logic as crates/playback-engine/tests/session_flow.rs:1966-2017 (same asserts, same messages, same RequestId(80)/RequestId(81) receipt assertions); helper `open_on` matches session_flow.rs:1933, `prime_playing` matches :1955.
— verified by baseline: audit/runs/baseline/cargo-test-workspace.txt:4472 `test superseded_epoch_events_never_leak_into_the_next_session ... ok` — the LIVE copy executed green on the current tree.

## 3. Findings
F-001 [S3]
Where: scripts/m11_regression_test.rs:1-93 (whole file; scripts/ is not a cargo member)
Code:
```
#[test]
fn superseded_epoch_events_never_leak_into_the_next_session() {
```
What is wrong: nothing compiles or runs this file. scripts/ holds shell tools; no Cargo target includes it; the only textual reference is docs/worklog/archive-2026-10.md:4236 ("- Applied W1-10's Lead wiring verbatim (... the verified M11 regression test; scripts/m11_regression_test.rs) — playback-engine 111/0."). Why it matters: (a) editing this file changes no behavior — a maintainer fixing the M11 guard here would ship nothing; (b) citing it as the regression evidence points at a file with no execution path, while the real evidence is session_flow.rs:1966 + the baseline run. What correct looks like: delete the scripts/ copy (history is in the worklog) or move it under a docs/evidence path named as an artifact. Confidence: HIGH — Verification: read + grep (no other references).
F-002 [S4]
Where: scripts/m11_regression_test.rs:1-28 (bare fragment)
Code:
```
fn open_on(engine: &mut Engine, script_len: u64) -> SessionHandle {
```
What is wrong: the file is a fragment, not a standalone test: no `mod`, no `use` statements, unresolvable bare identifiers (`Engine`, `DemuxEvent`, `FakeDemuxer`...) — it cannot compile even if dropped verbatim into a crate. It only makes sense pasted into session_flow.rs's context. Confidence: HIGH — Verification: read.

## 4. Per-function table (file <100 LOC but table kept for the 3 items)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open_on` | 8-28 | UNREACHABLE copy (REAL twin at session_flow.rs:1933) | S3 | fragment; identical semantics |
| `prime_playing` | 30-38 | UNREACHABLE copy (twin :1955) | S3 | 40-poll loop + assert Playing |
| `superseded_epoch_events_never_leak_into_the_next_session` | 40-92 | UNREACHABLE copy (twin :1966-2017, baseline-green :4472) | S3 | strong test: no-leak assert + suppressed>0 assert + receipts-survive assert |

## 5. Test honesty
The test itself is honest and would fail if the epoch guard regressed (stale_leaked non-empty => assert fails; suppression counter 0 => assert fails) AND if the guard overreached (missing Receipt(80)/81 => fails). Mutation-worthy both directions. The PROBLEM is purely that this copy never executes — the live twin does (baseline evidence).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the verified M11 regression test; scripts/m11_regression_test.rs — playback-engine 111/0" | docs/worklog/archive-2026-10.md:4236 | SHALLOW (the test is real but the scripts/ path is a non-executing copy; the 111/0 score belongs to the landed session_flow copy) | session_flow.rs:1966 twin; baseline :4472 ok |

## 7. Dead / duplicated / unreachable code
The entire file is dead-by-location (only stray .rs in scripts/). Duplication: full copy of session_flow.rs:1933-2017.

## 8. Dependencies & flags
None reachable — the file references playback-engine/testkit types it never imports.

## 9. Open questions for the next agent
- Why was the scratch copy kept? No comment explains it. If it was meant as "evidence," the worklog line should cite the session_flow.rs path instead.
