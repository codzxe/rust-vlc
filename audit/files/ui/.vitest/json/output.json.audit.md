# AUDIT: ui/.vitest/json/output.json
| Field | Value |
|---|---|
| Auditor | W15-A04 (recovered by W15-R3) |
| Date | 2026-11-24 |
| Lines / functions | 1 (single 32,112-byte minified JSON line) / n/a (data artifact) |
| File verdict | REAL as data (genuine vitest JSON reporter output) — UNREACHABLE as repo artifact (zero producers, zero consumers) = committed litter |
| Reachable from shipped entry point? | NO: nothing references the path (repo-wide rg for `.vitest` outside audit/ and the file itself: 0 hits) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | python3 json parse (exit 0): 19 testResults, 73/73 tests passed, 8 suites failed, success=false; rg `.vitest` repo-wide (exit 1 = no refs); rg baseline cross-check (exit 0) |

## 1. One-paragraph verdict
This is a committed vitest JSON-reporter dump of a targeted run of the settings subtree (all 19 result files live under src/screens/settings/), executed 2026-11-11T14:48:31Z in a checkout rooted /home/z/my-project/rust-vlc — NOT this workspace. It records a broken mid-development moment: 8 suites failed with unresolvable imports (`./controls`, `./recording` from areas/index.tsx — both files exist in the current tree), `success:false`. Nothing in the repo writes this file, reads it, or cites it; it is stale against a green baseline and misrepresents current test status to anyone who opens it. Verdict on the assignment question: litter, not evidence.

## 2. What actually works (verified)
— python3 json.load -> parseable, schema-consistent vitest JSON (numTotalTests 73 == numPassedTests 73, 19 testResults entries).
— Baseline cross-check: audit/runs/baseline/ui-test.txt:178-179 `Test Files 136 passed (136)` / `Tests 1705 passed | 1 skipped (1706)` — the current tree (with controls.tsx + recording.tsx present) is green where this artifact is red.

## 3. Findings
F-001 [S3]
Where: ui/.vitest/json/output.json (byte offsets 50, 454, 5958 of the single line)
Code (verbatim fragments):
```
"numFailedTestSuites":8,"numPendingTestSuites":0,"numTotalTests":73,"numPassedTests":73,"numFailedTests":0
"success":false,"testResults":[
Failed to resolve import \"./controls\" from \"src/screens/settings/areas/index.tsx\". Does the file exist?
```
What is wrong: the committed artifact says 8 settings suites fail and success:false; the current tree has controls.tsx/recording.tsx present (ls ui/src/screens/settings/areas/) and areas/index.tsx:49-50 imports resolve — baseline runs all 136 files green. The dump is an unlabeled stale snapshot of a WIP state. Why it matters: a committed test result that contradicts reality is worse than none — any reader (or future agent) citing it concludes the settings tests are broken. What correct looks like: delete it, or move it under docs/gates/logs/ with a provenance header (the repo's existing pattern, e.g. docs/gates/logs/phase4-exit-ui-vitest.log). Confidence: HIGH — Verification: ran (json parse + ls + baseline cite).
F-002 [S4]
Where: ui/.vitest/json/output.json (existence)
What is wrong: no producer (vite.config.ts test block has no reporters/outputFile; package.json:14 test = "vitest run", default reporter), no consumer, no reference (repo-wide rg `.vitest`: 0 hits outside the file; .gitignore does not exclude it, so it was committed deliberately-or-carelessly). Dead weight in the tree. Confidence: HIGH — Verification: ran.
F-003 [S4]
Where: testResults[].name fields
Code (verbatim):
```
"name":"/home/z/my-project/rust-vlc/ui/src/screens/settings/SettingsProvider.test.tsx"
```
What is wrong: leaks the dev machine's checkout layout (/home/z/my-project/rust-vlc ≠ this workspace root /home/z/my-project/audit-workspace) and embeds a wall-clock startTime (1794408511179 = 2026-11-11T14:48:31.179Z) — the artifact can never be byte-reproduced and proves it was generated in a different checkout than the one it sits in. Confidence: HIGH — Verification: ran (python3 timestamp conversion + path diff).

## 4. Per-function table
N/A (data artifact, not code).

## 5. Test honesty
N/A — this file IS test output. Honesty check of the run itself: the 8 "failed" suites report 0 assertionResults (suite-level import failure, no tests executed) while the header claims numPassedTests 73 == numTotalTests 73 — internally consistent, but a careless reader of just the counts would miss that 8 files never ran.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| (none — no doc, script, CI, or comment cites this file) | — | — | rg `.vitest` repo-wide: 0 hits |

## 7. Dead / duplicated / unreachable code
The file itself: fully dead data (F-002).

## 8. Dependencies & flags
None. Not covered by .gitignore (ui/.gitignore lists node_modules/dist/coverage/playwright-report/test-results/pseudo-probe/e2e/web/sitedist — not .vitest).

## 9. Open questions for the next agent
- UNVERIFIED: whether the 2026-11-11 broken state (missing controls.tsx/recording.tsx while index.tsx imported them) was ever a committed state, or only a local WIP tree — git history check required, banned for auditors.
- UNVERIFIED: who invoked the run and with what exact command line (no producer in tree; selection pattern suggests `vitest run src/screens/settings` or an IDE run).
