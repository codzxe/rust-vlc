# AUDIT: scripts/quality/rules/sleep-in-test.yml
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 23 / 1 ast-grep rule (3-pattern any + inside constraint) |
| File verdict | REAL (executed via pinned ast-grep 0.51.3: 26 true hits) |
| Reachable from shipped entry point? | YES as tooling — default rule set of scripts/quality/03_astgrep.sh (J-tier reporter) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## 1. One-paragraph verdict
A working ast-grep rule that I EXECUTED with the suite's own pinned tool version (`npx -p @ast-grep/cli@0.51.3 ast-grep scan --rule ... --json crates apps tools`): **26 real hits** — genuine `sleep` calls inside test modules (packet-queue/tests/concurrency.rs x10, backpressure.rs, and more). The rule's structure is correct for the ast-grep YAML schema (any-of three sleep forms; `inside` a `mod tests` or `#[test]`-attributed function, `stopBy: end`); the message is actionable (mock clock / condition variable, D-10 reference). The one environmental defect discovered while executing: the suite's `sg_run` fallback in common.sh probes `command -v sg` — on this (and most) Linux systems `/usr/bin/sg` is NEWGRP, a setgroups utility, so the fallback chain false-positives and never reaches the working pinned npx path.

## 2. What actually works (verified)
— EXECUTED (stdout-only, no repo writes): pinned ast-grep 0.51.3 + this rule -> 26 hits, first 12 enumerated (packet-queue/tests/concurrency.rs:204,254,272,276,344,465,542,697,843,857; backpressure.rs:247,315). Real rule, real detection.
— `ast-grep --version` -> 0.51.3 (the same version common.sh pins for the npx fallback — pin honored in my run).
— Mutation reasoning: remove the `inside` constraint and the rule would flag sleeps in production code (it deliberately does not); the `#[test]`-attribute branch catches `#[tokio::test]` too (regex 'test' substring).

## 3. Findings

F-001 [S4 — cross-file, observed during execution]
Where: scripts/quality/common.sh sg_run (context for this rule's runner)
Code (from common.sh):
```
sg_run() {
  if command -v ast-grep >/dev/null 2>&1; then
    ast-grep "$@"
  elif command -v sg >/dev/null 2>&1; then
    sg "$@"
```
What is wrong: `/usr/bin/sg` on this host is a symlink to newgrp (`ls -la /usr/bin/sg` -> `sg -> newgrp`, ran) — and that is the standard Linux layout. On any host without a cargo-installed ast-grep, sg_run selects newgrp, whose CLI rejects `--rule` (observed: "sg: unrecognized option '--rule'"), so the scan fails and records not_run — while the WORKING pinned npx path sits unreached below the false-positive branch. This rule therefore reports nothing on such hosts even though the pinned npx tool works there (proven: I ran it). Owner fix: drop the `sg` branch or probe `sg --help` for ast-grep semantics. Confidence: HIGH — Verification: ran (ls symlink; sg invocation rejected; npx path succeeded with the same args). Logged as cross-file observation; common.sh is not my assigned file.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| rule (any + inside) | 7-23 | REAL | - | executed: 26 hits; scope constraint verified by design |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the executed scan | YES | 26 true positives prove the matcher fires; a broken pattern yields 0 hits (a silent-empty risk only if patterns drift from API — but hits exist today) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Sleep inside a test: flaky by construction ... (D-10)" | rule message | REAL | 26 detected sleeps confirm the described practice exists in-tree (the finding the rule exists to surface) |

## 7. Dead / duplicated / unreachable code
None in the rule. (The sg fallback branch of common.sh is effectively dead-wrong on stock Linux — see F-001.)

## 8. Dependencies & flags
ast-grep 0.51.3 (pinned npx package — I executed exactly the pinned version).

## 9. Open questions for the next agent
1. Are the 26 sleep hits triaged anywhere (docs/audit/FINDINGS.md)? 26 flaky-by-construction sleeps in packet-queue tests is a real test-stability exposure.
2. common.sh sg_run fix (F-001) — handoff to the quality-lane owner.
