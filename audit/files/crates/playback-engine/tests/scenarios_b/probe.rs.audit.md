# AUDIT: crates/playback-engine/tests/scenarios_b/probe.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-23 |
| Lines / functions | 50 / 1 (single test) |
| File verdict | STUB (self-declared temporary debug scaffolding, committed anyway) |
| Reachable from shipped entry point? | NO — test-only module (`mod probe;` at scenarios_b/main.rs:56), runs in every `cargo test -p playback-engine --test scenarios_b` invocation; ships in no binary. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Direct run `./target/debug/deps/scenarios_b-bee4b59599013918 probe_read` -> 1 passed / 0 failed (exit 0, prints eprintln diagnostics); baseline audit/runs/baseline/cargo-test-workspace.txt:4389 (ok). |

## 1. One-paragraph verdict
This file's own first line says everything: "Temporary probe — measure the scripted source's read pattern (to be deleted before commit)." It was not deleted before commit; it is still here, still running in the suite, still asserting nothing (the only assert is that the open succeeds — the read-pattern measurements it exists to print go to stderr for a human, with no pass/fail meaning), and still swallowing its close result. As a test it is vacuous: it would pass identically if the scripted source's read pattern were completely broken, because no read-stat invariant is ever asserted. Its one saving grace is honesty — it does not pretend to be a proof; it labels itself a probe. Verdict: committed debug scaffolding that its own header declares should not exist.

## 2. What actually works (verified)
— `./target/debug/deps/scenarios_b-bee4b59599013918 probe_read` -> `test result: ok. 1 passed` (exit 0); the probe genuinely drives the real host (open `script://avi-parent`, settle, 40 pumps, close) and prints real read counters — as a diagnostic tool it functions. Nothing else is verified because nothing else is asserted.

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/tests/scenarios_b/probe.rs:1-2
Code:
```
//! Temporary probe — measure the scripted source's read pattern (to
//! be deleted before commit).
```
What is wrong: the file declares itself temporary and not-for-commit, and it is committed and wired into the suite (`mod probe;` main.rs:56; runs in baseline and in every gate run). It is dead-weight debug scaffolding: no invariant, no regression value — its assertions cannot fail for any read-pattern defect (the only assert is `open.result.is_ok()`, line 24). It also holds the process-global `serialize_tests()` lock for its whole body, serializing it against every real test in the binary for no evidentiary gain. Why it matters: committed "temporary" files rot into permanent noise; a future reader cannot tell which tests carry meaning. What correct looks like: delete the file (as its header instructs), or — if the read-pattern shape matters — promote it to a real test asserting the read-count/byte invariants it currently only prints. Confidence: HIGH — Verification: read + ran.
Confidence: HIGH — Verification: read / ran

F-002 [S4]
Where: crates/playback-engine/tests/scenarios_b/probe.rs:49
Code:
```
    let _ = dispatch(&mut host, Command::SessionClose);
```
What is wrong: the close result is explicitly discarded — a failed close (the very teardown the suite's scenario 16 measures carefully) would be silent here, and the final state after close is never checked. Consistent with being a probe, but this is the exact swallowed-error shape (checklist 6.2) the rest of the suite is scrupulous about. Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| probe_read_pattern | 11-50 | STUB (as a test) | S3/S4 | self-declared temporary; one vacuous assert; swallowed close (F-001, F-002) |

(50 LOC — per-function table trivially complete with the single test.)

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| probe_read_pattern | NO (for anything it nominally measures) | read_stats are printed, never asserted; only the open's success is asserted; close result discarded |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "to be deleted before commit" | probe.rs:2 | FALSE (as a statement about the repo's state) | the file is committed, wired via main.rs:56, and executes in the baseline gate run (baseline:4389) and my run |

## 7. Dead / duplicated / unreachable code
The entire file is the dead code (F-001). No #[ignore], no todo!/unimplemented!.

## 8. Dependencies & flags
commands catalog, testkit (Must, serialize_tests), crate::host. `#[global_allocator]`-sharing binary (hence the serialize lock). No feature gates.

## 9. Open questions for the next agent
- Was the read-pattern question this probe was measuring ever answered/promoted into a real test (scenario_11 interrupted-network reads?), or did the investigation die with the probe left behind? If the former, deletion is trivially safe; if the latter, the measurement should be re-run and asserted.
