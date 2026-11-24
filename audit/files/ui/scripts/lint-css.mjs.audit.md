# AUDIT: ui/scripts/lint-css.mjs
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 65 / 1 (launcher) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — dev lint launcher (npm run lint:css) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read; wiring check via package.json scripts (lint:css — not re-run; baseline covers UI lanes) |

## 1. One-paragraph verdict
Small, correct launcher: prefers the offline local stylelint binary (X_OK probe), falls back to the historical pinned npx form with a LOUD notice (never silently passing), forwards argv verbatim, propagates the child exit status (2 on launch failure). The degraded-path notice explains the network tradeoff. Zero findings.

## 2. What actually works (verified)
— verified by read: exit-code propagation (L65 `child.status ?? 2`), loud fallback (L45-48), pinned versions matching the documented npx form.

## 3. Findings
None. Checked: no `|| true`, no swallowed status, no unquoted paths.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| top-level | 26-65 | REAL | — | offline-first, fail-loud |

## 5. Test honesty
No direct tests (a launcher); exercised by every `npm run lint:css` CI run (baseline: UI lanes pass; full_gate lint sections red are other lanes' lint-literals, not css — recorded in baseline §13).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "path 2 exists so the gate keeps working in a degraded checkout instead of silently passing" (L15-16) | header | REAL | L44-58 fallback + notice |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
node stdlib; stylelint 18.0.0 + config-standard 41.0.0 (pinned in fallback, matching devDependencies per header).

## 9. Open questions for the next agent
- None.
