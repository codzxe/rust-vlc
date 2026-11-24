# AUDIT: ui/tools/eslint-plugin-local/package.json
| Field | Value |
|---|---|
| Auditor | W15-R2 (credited: A06) |
| Date | 2026-11-23 |
| Lines / functions | 13 / 0 (manifest) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (dev toolchain, not runtime) — ui/eslint.config.js:14 `import local from "./tools/eslint-plugin-local/index.js"` registered as plugin at :107, rules enforced as "error" at :108-110; `npm run lint` = `eslint .` (ui/package.json:11) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | ls plugin dir (index.js + 3 rules + 3 tests exist); rg eslint.config.js (exit 0); sed vite.config.ts:38-46; grep baseline ui-test.txt (3 rule test files ran and passed) |

## 1. One-paragraph verdict
Manifest for a real, tested, wired local ESLint plugin. Everything it declares
exists: exports "." -> ./index.js (present, exports the 3 rules), engines node
^22||^24||>=26 (sandbox runs v24.22.3 — in range). The plugin is consumed by
eslint.config.js:14 under the `local` key with all three rules at "error"
(:108-110), and its unit tests are wired into the vitest include glob
(vite.config.ts:43 `"tools/eslint-plugin-local/**/*.test.js"`). Baseline
ui-test.txt:144-166 shows all 3 rule test files PASSED in the real run. One
hygiene finding: nothing imports the package by its name — the sole consumer
uses a relative path, so `name`/`exports` are decorative; the load-bearing
field is `type: module` (which the parent ui/package.json:5 already provides).

## 2. What actually works (verified)
— verified by `grep -n "eslint-plugin-local" audit/runs/baseline/ui-test.txt` ->
  semantic-tokens-only.test.js (1 test), i18n-no-literals.test.js (2 tests),
  boundary-imports.test.js (1 test) — all ✓ in the 136-file run.
— verified by `sed -n '100,115p' ui/eslint.config.js` -> all three local rules
  registered at "error".

## 3. Findings
F-001 [S4]
Where: ui/tools/eslint-plugin-local/package.json:2-9
Code: (verbatim quote)
```
  "name": "eslint-plugin-local",
  "version": "0.1.0",
  "private": true,
  "type": "module",
```
What is wrong: No consumer resolves this package by name ("eslint-plugin-local"
appears in zero import statements; eslint.config.js:14 imports by relative
path). The manifest's identity fields do no work; only "type": "module" has an
effect and ui/package.json:5 already declares it for the tree. Why it matters:
cosmetic packaging debt; a reader may assume a workspace dependency exists (it
does not — no "file:" reference in ui/package.json). What correct looks like:
either drop the manifest or add it as a proper workspace dependency. Harmless
to lint behavior either way. Confidence: HIGH — Verification: rg (exit 0/1
checks above).

## 4. Per-function table
Not applicable — JSON manifest, 4 keys.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| rules/*.test.js (3 files, 4 tests total) | YES | Ran in baseline (ui-test.txt:144-166); they execute the real rule implementations |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "promoted verbatim from the former inline definitions in ui/eslint.config.js" | package.json:6 | UNVERIFIED | Requires git history (forbidden); eslint.config.js:5-8 comment repeats the claim |
| "the consuming config registers this package under the 'local' plugin key" | package.json:6 | REAL | eslint.config.js:14 + :107 `plugins: { local, ... }` |
| "Unit tests ... wired into npm run test via the vitest include glob" (index.js comment) | index.js:15-16 | REAL | vite.config.ts:43; baseline run proves execution |

## 7. Dead / duplicated / unreachable code
`name`/`version`/`exports` fields unused by any resolver (see F-001). Rules
themselves all live (all three enforced as errors).

## 8. Dependencies & flags
No dependencies declared — correct for a pure-ESLint-rule package. Engines
range satisfied by sandbox node v24.22.3.

## 9. Open questions for the next agent
Whether full_gate/lint CI steps would behave identically if the manifest were
deleted (likely yes — relative import only).
