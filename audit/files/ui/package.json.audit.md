# AUDIT: ui/package.json
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 59 / manifest (24 scripts, 3 runtime deps, 19 dev deps) |
| File verdict | REAL (lean, coherent manifest; every referenced script file exists; three generator/drift-guard pairs wired) |
| Reachable from shipped entry point? | YES — this manifest builds the shipped webview asset (`npm run build` → tsc + vite, baseline PASS) and defines the dev/test/visual lanes. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | existence check of all 9 referenced scripts/*.mjs (all present); baseline ui-npm-ci.txt (clean install), ui-build.txt (build PASS), ui-test.txt (1705/1) |

## 1. One-paragraph verdict
Three runtime dependencies total (react, react-dom, react-router-dom) — no state library, no i18n framework, no HTTP client: consistent with the audited architecture (homegrown i18n catalog, injected transport seam, local stores). The 24 scripts form coherent lanes (dev/build/typecheck/test/e2e, visual pr/nightly/update/seed, three generate+check drift-guard pairs for tokens/settings/hotkeys, l10n checks, dev-shots drift) and every referenced scripts/*.mjs exists on disk. Zero findings.

## 2. What actually works (verified)
— file-existence check: all 9 referenced scripts present (visual-run.mjs audited this wave).
— baseline: npm ci clean, build PASS (tsc && vite), vitest 1705 passed / 1 skipped — the manifest's lanes are the ones the baseline exercised.

## 3. Findings
None. Checked: referenced-file existence (all), dependency/runtime split (no accidental runtime deps in devDependencies or vice versa), pinned vs ranged versions (runtime pinned exact; dev mostly exact, ^stylelint/^types/node acceptable for tooling), scripts referencing real files, no unused scripts (each lane maps to a real command; visual:seed/visual:update map to the audited shim's flags).

## 4. Per-function table
n/a (manifest <100 LOC).

## 5. Test honesty
`npm test` = `vitest run` (baseline 1705 PASS). e2e lane (playwright) not re-run here.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "zero new npm deps" (visual-run.mjs's rationale) | visual-run.mjs:8 | TRUE | @playwright/test already in devDependencies (package.json:40) |

## 7. Dead / duplicated / unreachable code
No dead scripts found.

## 8. Dependencies & flags
React 19.3.1 / router 7.18.4 runtime; vite 8/vitest 5/typescript 6 toolchain. No feature flags at the manifest level.

## 9. Open questions for the next agent
- None.
