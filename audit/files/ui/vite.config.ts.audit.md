# AUDIT: ui/vite.config.ts
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 46 / 1 config |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as build config — drives every UI build/dev/test run (npm build passes per baseline §13) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read; `rg -ln "__UI_PACKAGE_"` (build-info.ts + updates-about.tsx consumers + globals declaration) |

## 1. One-paragraph verdict
Correct, tight build config: react plugin; host pinned to 127.0.0.1 with a cited CI failure (run 43370495290) justifying it; package name/version injected as defines and really consumed (build-info.ts -> the About panel, so the About version can never drift from package.json); vitest includes both src tests and the eslint-rule unit tests; JSON import attribute for the native config loader. Zero findings.

## 2. What actually works (verified)
— verified by consumer sweep for the defines and by baseline (npm ci/typecheck/test/build all pass; vitest 1705 passed).

## 3. Findings
None. Checked: no proxy/alias hacks, no test-glob exclusions that would hide tests, defines consumed.

## 4. Per-function table
| Block | Lines | Verdict | Notes |
|---|---|---|---|
| config | 17-46 | REAL | host pin, defines, test includes |

## 5. Test honesty
Config itself untested (standard); the include list governs the 1705-test suite that passes.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "UI code never imports outside ui/src ... never hardcodes a version" (L13-15) | comment | REAL | build-info.ts consumer (rg) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
@vitejs/plugin-react; Node ≥20.10/TS ≥5.3 syntax for the import attribute (documented).

## 9. Open questions for the next agent
- None.
