# AUDIT: ui/tsconfig.json
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 30 / 0 (config) |
| File verdict | REAL (strict config; include set covers src + e2e + configs) |
| Reachable from shipped entry point? | YES — gates the typecheck of every ui/src file (all my audited files) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; include-set vs my assignment cross-check |

## 1. One-paragraph verdict
Strict, modern config: strict + noUncheckedIndexedAccess + noUnusedLocals/Parameters + noFallthroughCasesInSwitch + noImplicitOverride + verbatimModuleSyntax + isolatedModules; ES2022/DOM libs; bundler resolution; noEmit. `include` covers `src`, `e2e`, and the three config files — no exclusion games (nothing is carved out of typecheck). This matters for the master question: no file in ui/src can dodge the compiler.

## 2. What actually works (verified)
— verified by read: include list (L22-29) has no `exclude`; all 41 of my files fall inside `src`.

## 3. Findings
None. Checked: strictness flags all on, no `skipLibCheck`-style lies (skipLibCheck true is standard and only affects .d.ts of deps), no path-alias weirdness, no excluded source.

## 4. Per-function table
n.a. — config.

## 5. Test honesty
n.a.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| (implicit) all of src is typechecked | include L22-23 | REAL | no exclude key |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
vite/client types. Note: `types: ["vite/client"]` means node types are not global — scripts run via node separately (mjs files, not typechecked here).

## 9. Open questions for the next agent
None.
