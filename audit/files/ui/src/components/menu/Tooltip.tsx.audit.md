# AUDIT: ui/src/components/menu/Tooltip.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 127 / 1 (Tooltip) |
| File verdict | UNREACHABLE — real, tested component with zero production consumers |
| Reachable from shipped entry point? | NO — `rg -ln "<Tooltip" src --glob '*.tsx'` returns only Tooltip.test.tsx; the only other mention (SettingField.tsx:114) is a comment citing "k02's Tooltip pattern", not an import |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg -ln "<Tooltip" src --glob '*.tsx'`; `rg -ln "Tooltip" src --glob '*.tsx' --glob '!*test*'`; `rg -n "Tooltip" components/menu/index.ts` (exported L8-9); `rg -c "it\(" Tooltip.test.tsx` (10) |

## 1. One-paragraph verdict
Well-built tooltip primitive (hover-intent show/hide timers, immediate focus show, Escape consumption with stopPropagation, aria-describedby merged onto the child trigger via cloneElement — each behavior carries a fix-citation) with 10 tests — and not a single shipped screen uses it. The staged settings rows use native `title` attributes instead (staged.tsx L132); SettingField references the "Tooltip pattern" only in a comment. Exported API (menu/index.ts:8-9) with no consumer: dead primitive, tree-shaken from any production bundle but alive in source and test maintenance cost.

## 2. What actually works (verified)
— verified by Tooltip.test.tsx (10 cases; baseline vitest 1705 passed). Component logic reviewed line-by-line: timer cleanup on unmount (L52), show/hide races handled, describedby merge/preserve (L62-72).

## 3. Findings
F-001 [S3]
Where: ui/src/components/menu/Tooltip.tsx:31 (export) vs repo-wide usage
Code:
```
export function Tooltip({
  content,
  children,
```
What is wrong: zero production consumers — the component exists, is exported, and is tested, but no shipped route renders it (rg proof above). Dead primitive. Why it matters: maintenance + test cost for nothing; also signals an inconsistency — tooltip affordances elsewhere use raw `title` (staged rows) while this a11y-better primitive sits unused. What correct looks like: either adopt it at the `title`-using sites or delete it. Confidence: HIGH — Verification: rg sweeps (quoted).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Tooltip | 31-127 | REAL (unreachable) | S3 | F-001 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| Tooltip.test.tsx (10) | YES | timers, focus, escape, describedby merge covered |
Tests are honest — they prove a component nobody uses.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Tooltip (doc 08 §259 primitives list)" (L20) | docstring | REAL as a list entry; UNREACHABLE as shipped surface | rg consumer sweep |

## 7. Dead / duplicated / unreachable code
The entire component — see F-001.

## 8. Dependencies & flags
timings constants, menu.css. No backend coupling (correct).

## 9. Open questions for the next agent
- Was Tooltip consumed by a surface that was later restyled to native title (the staged rows)? A git-blame would date the orphaning (orchestrator side; agents never run git).
