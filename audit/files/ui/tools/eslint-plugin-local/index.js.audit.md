# AUDIT: ui/tools/eslint-plugin-local/index.js
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-11-21 |
| Lines / functions | 29 / 1 default export (plugin object with 3 rule registrations) |
| File verdict | REAL — every wiring claim in the header verified true against the tree and the baseline run |
| Reachable from shipped entry point? | YES as toolchain: eslint.config.js:12 imports it, registers under `local`, and enables all three rules at "error" for `src/**/*.{ts,tsx}` (eslint.config.js:101-108); `npm run lint` = `eslint .` (package.json:11); CI ui.yml:102-111 runs `npm run lint`. Its 3 unit-test files run inside `npm test` (baseline #23: 136 files = 133 src + 3 tools, verified by count). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `ls tools/eslint-plugin-local/rules/` (3 rules + 3 tests exist); read eslint.config.js:12,101-108 (import + enablement); read vite.config.ts:41-44 (include glob `tools/eslint-plugin-local/**/*.test.js`); `find src tools -name "*.test.*" | wc -l` → 136 = baseline "Test Files 136 passed"; read rules/boundary-imports.js + its RuleTester test; `rg -n "lint" package.json .github/workflows/ui.yml` |

## 1. One-paragraph verdict
The plugin index does exactly what its header says and nothing else: it re-exports the three promoted rules under their canonical names, eslint.config.js registers them as `local/<name>` at "error" severity, the unit tests run in the same vitest pass as the app tests (proven by the file-count match with the baseline's 136), and CI wires `npm run lint` (ui.yml:102-111). The rule bodies belong to A05/A10's audits; from this file's side the registrations are sound (each rule file exports a `{meta, create}` object — spot-verified on boundary-imports.js). The only blemish is a provenance claim that cannot be checked from the tree, plus the fact that the baseline run set contains no lint execution (only lane logs and CI config evidence the rules pass on real code).

## 2. What actually works (verified)
— verified by read eslint.config.js:12 + :101-108 → `import local from "./tools/eslint-plugin-local/index.js"`, `plugins: { local, … }`, `"local/i18n-no-literals": "error"`, `"local/boundary-imports": "error"`, `"local/semantic-tokens-only": "error"` — the header's "addressed as `local/<name>` exactly as before" is true.
— verified by read vite.config.ts:41-44 → include glob covers `tools/eslint-plugin-local/**/*.test.js`; `find` count = 136 test files = baseline #23's "Test Files 136 passed (136)" — the 3 rule test files demonstrably ran green in the baseline.
— verified by read rules/boundary-imports.js:17-30 + boundary-imports.test.js:15-59 → real ESLint rule shape (`meta` + `create`) and real RuleTester cases (valid: inside-boundary relative, bare specifier, deep climb, up-and-back-in; invalid: escape + absolute path with messageId assertions) — the registration the index performs lands on working rules.
— verified by ui.yml:102-111 → CI runs `npm run lint` and `npm run lint:css`; w26.c1/w26.k10 lane logs record live `npx eslint … → clean` runs over the tool dirs.

## 3. Findings

### F-001 [S4]
Where: ui/tools/eslint-plugin-local/index.js:3-4
Code:
```
// The three scaffold-local rules that used to live inline in
// ui/eslint.config.js, promoted to a tested package (single source of
```
What is wrong: the "promoted verbatim" provenance claim (here and in package.json/rules headers) is unverifiable from the tree — the former inline definitions no longer exist anywhere in the repo, and this audit session runs no git archaeology (§7.4 deviation). Nothing contradicts it either; but a reader cannot confirm the semantics are "unchanged" from any in-tree witness. Additionally, the baseline run set (audit/runs/baseline/) contains no `npm run lint` execution — the rules' "error" pass on real code is evidenced only by lane logs (w26.c1, w26.k10 `npx eslint → clean`) and the CI workflow's config, not by a committed run output.
Why it matters: provenance/evidence bookkeeping for a toolchain gate; the rules' correctness on live code is otherwise well-proven (unit tests in baseline + CI wiring).
What correct looks like: cite the promoting commit/issue in the header (issue #36 is named — sufficient), and add a lint run to the baseline evidence set at the next gate.
Confidence: HIGH (that it is unverifiable) — Verification: repo-wide read; baseline INDEX.md table (#21-24 covers npm ci/build/test/checks only).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| default export `{ rules: {…} }` | 23-29 | REAL | — | flat-config plugin shape (rules-only object is valid); 3 registrations, each backed by an existing rule module (verified by ls + read of one) |
| `rules["boundary-imports"]` | 25 | REAL | — | rule body = A10's audit; shape spot-verified |
| `rules["i18n-no-literals"]` | 26 | REAL | — | rule body = A05's audit |
| `rules["semantic-tokens-only"]` | 27 | REAL | — | rule body = A05's audit |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| boundary-imports.test.js (RuleTester, 1 it → 4 valid + 2 invalid cases) | YES | messageId-pinned invalid cases incl. two edge geometries (up-and-back-in, deep climb); runs in baseline (count-verified) |
| i18n-no-literals.test.js / semantic-tokens-only.test.js | YES (A05's audit owns depth) | same RuleTester pattern; both in the baseline 136 |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "ui/eslint.config.js imports this module and registers it under the `local` plugin key" | index.js:4-7 | REAL | eslint.config.js:12,104 |
| "rule names below are addressed as `local/<name>`" | index.js:7-8 | REAL | eslint.config.js:106-108 (all three at "error") |
| "Unit tests … wired into `npm run test` via the vitest include glob in vite.config.ts" | index.js:16-17 | REAL | vite.config.ts:43; 136-file baseline count match |
| "promoted verbatim … semantics unchanged" | index.js:3-6, package.json description | UNVERIFIED | former inline definitions absent from tree; no git archaeology this session (F-001) |

## 7. Dead / duplicated / unreachable code
None. All three rules enabled; tests run in the default test pass.

## 8. Dependencies & flags
No runtime deps (rules use node:path/node:url internally); package.json (A04's audit) declares `type: module` + engines — the .js extension imports in index.js:19-21 are correct for ESM.

## 9. Open questions for the next agent
- eslint.config.js enables the local rules ONLY for `src/**/*.{ts,tsx}` — the rules never see `e2e/web/site/*.js` or `scripts/**`; if i18n literals or raw colors matter in the web-matrix harness page, that is a config-scope decision worth a row in whoever owns eslint.config.js.
