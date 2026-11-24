# AUDIT: docs/gates/logs/phase4-exit-playwright.log (DATA AUDIT — log)
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Lines | 799 |
| File verdict | REAL (authentic playwright run log; internally consistent) |
| Reachable from shipped entry point? | N/A (log data; cited by docs/gates/phase4-exit.md:56,208) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg/sed/wc (see log W17-A10) |

## 1. One-paragraph verdict
Genuine `npx playwright test` run log: 68 tests, **37 passed / 31 failed (2.3m)**.
Internal arithmetic checks out (37+31=68). The 31 failures decompose as 30 visual
screenshot misses ("A snapshot doesn't exist … writing actual") plus 1 real smoke
failure ("boot lands on the player placeholder"). The log itself is honest data —
it records a RED run. Any doc citing it as a green gate is lying; phase4-exit.md:56
cites it accurately as 37/31 and diagnoses both failure classes. No exit-code line
is embedded in the log (Playwright non-verbose list reporter) — UNVERIFIED: exact
process exit code, though 31 failures guarantee non-zero under default config.

## 2. What actually works (verified)
- `sed -n '767,799p'` -> "31 failed" list + "37 passed (2.3m)". Sum = 68 = "Running 68 tests using 1 worker" (line 7).
- `sed -n '141,155p'` -> smoke failure verbatim: `Expected substring: "Player"` / `Received string: "Welcome to Lumen"` — real onboarding redirect, not a flake.
- `ls ui/e2e/visual/` -> baselines/ exists; `visual.spec.ts-snapshots/` does NOT exist in tree — corroborates the "snapshot doesn't exist … writing actual" failure mode for the default config.

## 3. Findings
F-001 [S4]
Where: docs/gates/logs/phase4-exit-playwright.log:799
Code:
```
  37 passed (2.3m)
```
What is wrong: final summary line omits the exit status and elapsed-detail present in CI-grade logs; the "31 failed" block is 33 lines earlier (767). A skimmer reading only the last line sees "37 passed". Why it matters: log-tail greps misreport this run as green. What correct looks like: trailing `Exit code: 1` or `failed` on the summary line. Confidence: HIGH — Verification: read.

## 4. Data integrity checks
- Test count math: 68 total = 37 passed + 31 failed. CONSISTENT.
- Failure taxonomy: 30 × visual.spec.ts:44 "matches pinned baseline (pr)" + 1 × smoke.spec.ts:6. All 30 visual failures are "A snapshot doesn't exist at …/visual.spec.ts-snapshots/…-linux.png, writing actual." (e.g. lines 389, 429, 449, 749) — missing-dir artifact, not pixel regressions.
- The single smoke failure is a real stale test (onboarding first-run redirect landed; test still expects "Player" heading) — corroborated by phase4-exit.md:56 which claims a post-fix rerun in `phase4-exit-playwright-postfix.log` (file exists, UNVERIFIED contents — assigned to another agent's domain).

## 5. Provenance
Log path references `/home/z/my-project/worktrees/p4-gate/ui/...` — produced in a
worktree, not the main tree. Cannot re-execute (browser budget, 2-core sandbox);
data audit only, no re-run claim made.
