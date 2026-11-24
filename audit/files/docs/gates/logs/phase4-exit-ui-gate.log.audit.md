# AUDIT: docs/gates/logs/phase4-exit-ui-gate.log (data audit)
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines | 76 (no header, no footer/exit line) |
| File verdict | REAL — a captured FAILING eslint run, honestly cited by its gate doc |
| Reachable from shipped entry point? | N/A (gate evidence log; worktree p4-gate historical) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | grep counts of error/warning lines; cross-check docs/gates/phase4-exit.md:51,200; fix presence check in ui/eslint.config.js |

## 1. One-paragraph verdict
The log records a genuinely failing UI lint run: 6 `no-undef` errors in `ui/scripts/l10n-pseudo-probe.mjs` (browser globals under a node-globals config) plus 26 react-refresh warnings, closing with `✖ 32 problems (6 errors, 26 warnings)`. Its citing doc claims exactly this ("FAIL — 6 errors / 26 warnings, exit 1") and uses the log to expose a contradiction in an earlier "lint clean" claim — evidence recorded, not hidden. The post-fix (browser-globals block scoped to that one file) is present in the current tree's ui/eslint.config.js:66-74. The only defect is the doc's description of the log as "typecheck+lint+test first run" — the log contains no test output at all.

## 2. What actually works (verified)
— Detail/summary consistency: 6 error lines (L11-16) + 26 warning lines = the summary at L74 `✖ 32 problems (6 errors, 26 warnings)`; L75 `0 errors and 2 warnings potentially fixable`.
— Fix lineage verified: `ls ui/scripts/l10n-pseudo-probe.mjs` -> exists; ui/eslint.config.js:74 `files: ["scripts/l10n-pseudo-probe.mjs"]` (the row-2b browser-globals block landed on main).

## 3. Findings
F-001 [S4] — Cited-log description overstates contents.
Where: docs/gates/phase4-exit.md:200 vs the 76-line log
Code (doc side):
```
- `phase4-exit-ui-gate.log` — typecheck+lint+test first run (lint FAIL at 6
```
What is wrong: the log contains typecheck (L2-3) and lint output ONLY — no test section, no exit-code footer, no date/command header. A reader sent to this log for "test" output finds none. Confidence: HIGH — Verification: read (full 76 lines).
(Also inherent: no exit code captured in the log; exit 1 is the doc's inference from eslint's non-zero-on-error contract — reasonable but unrecorded in-log.)

## 4. Claim cross-references
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "FAIL — 6 errors / 26 warnings, exit 1" | docs/gates/phase4-exit.md:51 | REAL | log L11-16, L74 |
| "all 6 errors are `no-undef` in `ui/scripts/l10n-pseudo-probe.mjs`" | phase4-exit.md:51 | REAL | log L10-16 (same file, same offsets) |
| "the be4e4c6 'lint clean' claim is contradicted by this run — recorded, not hidden" | phase4-exit.md:51 | consistent-with-log | the log's 6 errors are the contradicting evidence; be4e4c6 itself is out of git-lane (UNVERIFIED) |
| "Re-run: exit 0 — 0 errors / 26 warnings" (post-fix) | phase4-exit.md:52 | REAL for the current tree | the fix block is in ui/eslint.config.js:66-74 now |
| "typecheck+lint+test first run" | phase4-exit.md:200 | overstated | F-001 |

## 5. Open questions for the next agent
- None blocking. The lint-postfix log (phase4-exit-ui-lint-postfix.log) belongs to another agent's lane; if unassigned, verify its 0-error claim matches current `npm run lint` behavior.
