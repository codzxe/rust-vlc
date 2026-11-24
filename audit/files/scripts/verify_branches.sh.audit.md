# AUDIT: scripts/verify_branches.sh
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Lines / functions | 21 / 1 (single loop over 4 worktrees) |
| File verdict | UNREACHABLE (host-fossil one-shot; sound method for its day) |
| Reachable from shipped entry point? | NO — no callers; part of the ED-DISPATCH-1 narrative (docs/worklog/archive-2026-10.md:1661+) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | Read (full); ls of the hardcoded paths (worktrees absent, $HOME/.cargo/env present); rg for narrative references |

## 1. One-paragraph verdict
A historical one-shot: runs `cargo test --workspace` sequentially in four
named worktrees (fix3 3b4 fix2 fix1), capturing rc + timestamps and
grepping result/error summaries per branch. The method was sound for its
purpose (no oversubscription, shared target dir), but every path is a
hardcoded host-absolute fossil — the worktrees do not exist on this
machine, and the script cannot run anywhere but its original host. It
survives only as narrative scaffolding for the ED-DISPATCH-1 merge wave.

## 2. What actually works (verified)
— The per-branch pattern is correct: rc captured, echoed with ISO
timestamps, and a `grep -E "^test result:|error[|error:|warning: unused"`
summary tail-25 of each log — the same evidence shape the committed gate
logs use.
— `source "$HOME/.cargo/env"` exists on the original host (verified
present at /home/z/my-project/.cargo/env here, amusingly); the worktree
paths do not (ls: no /home/z/my-project/worktrees).

## 3. Findings
F-001 [S3]
Where: scripts/verify_branches.sh:8-13
Code:
```
export CARGO_TARGET_DIR=/home/z/my-project/target
LOGROOT=/home/z/my-project/logs/verify-branches
```
What is wrong: host-absolute CARGO_TARGET_DIR, LOGROOT outside the repo,
and four hardcoded worktree paths — the script is a single-machine fossil
(unrunnable on any other checkout, including this tree), same defect class
as scripts/corpus_supervisor.sh. Its logs were never committed, so the
"4 salvaged branches verified" narrative has no in-tree evidence.
Confidence: HIGH — Verification: read + ls.

## 4. Per-section table
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| loop body | 12-20 | UNREACHABLE | S3 | correct rc/summary capture; host-fossil paths |

## 5. Test honesty
N/A (one-shot). The method (full workspace test per branch, rc logged)
was the right verification for a merge-intent decision.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "verify the 4 salvaged directives-fix branches" | :2 | REAL-UNPROVEN | the loop visits exactly 4 named worktrees; whether the runs completed is recorded only in uncommitted host logs |

## 7. Dead / duplicated / unreachable code
Entire file unreachable off its host. Same family as the other one-shot
fossils (corpus_supervisor.sh, resolve_*.py).

## 8. Dependencies & flags
cargo/rustup env; shared target dir; 2-core considerate sequential runs.

## 9. Open questions
None for the next agent beyond the systemic one: this repo's scripts/
mixes maintained tooling (quality/*, packaging/*) with machine-fossil
one-shots (this, corpus_supervisor, resolve_*). A curation pass exists
(worklog.md:19 — 9 resolve scripts deleted) but these two remain.
