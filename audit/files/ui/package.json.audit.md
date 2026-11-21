# AUDIT: ui/package.json
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 59 / 19 scripts + 3 deps + 21 devDeps |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (manifest) — consumed by every ui gate; baseline runs #21-24 execute 8 of its 19 scripts; ui.yml executes 6 more |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read full file + scripts/ dir listing (all 10 script targets exist) + eslint.config.js:14,108-110 (the local-plugin wiring) + `rg -o 'npm run …' .github/workflows/ui.yml` (CI script coverage) + `rg 'webmatrix\|dev-shots\|pseudo-probe' .github/ scripts/ docs/gates/` (unwired scripts) + package.json↔lockfile cross-check (file 23: 24/24 consistent) + baseline ui-*.log.txt runs |

## 1. One-paragraph verdict
A real, well-organized manifest: every script's target file exists, the build gate correctly chains `tsc --noEmit && vite build`, the check/generate pairs (tokens/settings/hotkeys) match the drift-guard pattern the baseline proves green (runs #22-24), the dependency set is lean (3 runtime deps, 21 dev), and the lockfile cross-check (file 23) confirms all 24 entries resolve version-consistently. Two hygiene defects: the pinning policy is mixed — three of 24 deps carry caret ranges in an otherwise exactly-pinned manifest, with scripts/lint-css.mjs even claiming "the exact versions" for a `^18.0.0` range; and the script inventory outruns its enforcement — 9 of 19 scripts have no baseline execution (notably `lint`, whose last recorded repo-wide run FAILED at the phase-4 exit gate), and `webmatrix` + `dev-shots:check` are wired into no CI workflow at all.

## 2. What actually works (verified)
— Script targets all exist: scripts/{lint-css,gen-tokens,gen-settings-screens,gen-hotkeys-data,l10n-check,l10n-pseudo-probe,web-matrix-run,visual-run,dev-shots-drift-check}.mjs (ls verified).
— Baseline-proven scripts (runs #21-24): npm ci, build (tsc+vite, 386 ms), test (vitest 1705 passed), tokens:check, settings:check, hotkeys:check, l10n:check — 4/4 checks PASS.
— CI-proven wiring (ui.yml): typecheck, lint, lint:css, test, test:e2e (playwright), visual (pr + nightly lanes) — the gates named in the workflow match these script names exactly.
— Dependency hygiene: 3 runtime deps only (react/react-dom/react-router-dom, exactly pinned), no unused-vulnerable-looking entries; @types match the react versions; the local ESLint plugin (tools/eslint-plugin-local) is wired via eslint.config.js:14 with its three rules at "error" (:108-110) despite NOT appearing in devDependencies (it's a workspace-local path import — see file 25).
— `"private": true` + `"type": "module"` — correct for an app workspace.

## 3. Findings

### F-001 [S4]
Where: ui/package.json:38, 44, 52
Code:
```
    "@axe-core/playwright": "^4.14.0",
```
```
    "@types/node": "^26.6.5",
```
```
    "stylelint": "^18.0.0",
```
What is wrong: mixed pinning policy. 21 of 24 dependencies are exact-pinned; these three carry caret ranges. scripts/lint-css.mjs:10-11 compounds it by claiming "stylelint 18.0.0 + config-standard 41.0.0 — the exact versions the npx form pinned, from package.json devDependencies" — but the manifest says `^18.0.0`; it is the LOCKFILE that pins 18.0.0.
Why it matters: under `npm ci` the impact is nil (lock honored); under `npm update`/`install` on a fresh minor, exactly these three drift while the rest stay frozen — the "exact versions" reasoning in lint-css.mjs silently stops being true.
What correct looks like: pin all three (`4.14.0`, `26.6.5`, `18.0.0`) to match the other 21 and the script's own claim.
Confidence: HIGH — Verification: read + lockfile cross-check (file 23).

### F-002 [S4]
Where: ui/package.json:12-30 (scripts) vs baseline/CI coverage
Code:
```
    "webmatrix": "node scripts/web-matrix-run.mjs",
```
```
    "dev-shots:check": "node scripts/dev-shots-drift-check.mjs"
```
What is wrong: the script inventory outruns enforcement. No baseline run exists for: lint, lint:css (already REQUEST-ORCH-RUN, log entry 4), test:e2e, webmatrix, visual/visual:pr/visual:nightly/visual:update/visual:seed, l10n:pseudo-probe, dev-shots:check. CI covers lint/lint:css/test:e2e/visual:pr/visual:nightly (ui.yml) but webmatrix and dev-shots:check are wired into NO workflow — local-only commands. And `lint`'s last recorded repo-wide run FAILED (docs/gates/phase4-exit.md:51: 6 no-undef errors in l10n-pseudo-probe.mjs, exit 1; the doc records the fix landing later) — current HEAD status unproven.
Why it matters: scripts that nothing executes are claims, not gates; the phase-4 record proves this exact class of gate regresses between sessions.
What correct looks like: add webmatrix + dev-shots:check to ui.yml (or delete them), and add `npm run lint` to the baseline run set next to lint:css.
Confidence: HIGH on the coverage map — Verification: baseline INDEX + ui.yml grep + workflows grep; HEAD lint status UNVERIFIED.

## 4. Per-function table (scripts)
| Script | Target exists | Baseline | CI | Notes |
|---|---|---|---|---|
| dev / build:fast | n/a (vite) | — | — | dev-only, fine |
| build | ✓ | ✓ #22 | ✓ (implied by artifacts) | tsc --noEmit chained — the type gate rides the build |
| lint | eslint.config.js ✓ | ✗ | ✓ ui.yml | last recorded run FAILED (phase4-exit:51); HEAD status UNVERIFIED |
| lint:css | ✓ | ✗ | ✓ ui.yml | REQUEST-ORCH-RUN standing (log entry 4); stylelint audit F-002 |
| typecheck | ✓ | ✓ | ✓ | build covers it too |
| test | ✓ | ✓ #23 (1705 passed) | ✓ | |
| test:e2e | playwright.config ✓ | ✗ | ✓ ui.yml | single-browser chromium per phase-4 record |
| webmatrix | ✓ | ✗ | ✗ | wired NOWHERE (F-002) |
| visual / :pr / :nightly / :update / :seed | ✓ | ✗ | :pr + :nightly in ui.yml | update/seed are operator commands, fine |
| tokens / settings / hotkeys :generate+:check | ✓ | :check ✓ #24 (4/4 PASS) | — | :generate is operator-side; drift guards proven |
| l10n:check / l10n:pseudo-probe | ✓ | :check ✓ #24 | :check in ui.yml | probe is dev-only; its historical lint errors recorded at phase-4 exit |
| dev-shots:check | ✓ | ✗ | ✗ | wired NOWHERE (F-002) |

## 5. Test honesty
N/A (manifest). The check-scripts' honesty is evidenced by baseline #24 (all four PASS) and by the drift-guard design audited at file 1 (gen-settings-screens).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "build": tsc --noEmit && vite build — typecheck rides the build | :9 | REAL | baseline #22 exit 0 |
| lint-css.mjs "the exact versions … from package.json devDependencies" | lint-css.mjs:10-11 | FALSE for stylelint | `^18.0.0` is a range; the lockfile pins 18.0.0 (F-001) |
| ui.yml "2. lint" runs the ESLint + stylelint gates | ui.yml:102-110 | REAL | script names match exactly |
| (implied) all scripts are enforceable | the manifest itself | PARTIAL | webmatrix + dev-shots:check unwired; 9/19 without baseline runs (F-002) |

## 7. Dead / duplicated / unreachable code
- No dead deps found (all 24 lockfile-consistent, file 23; the runtime trio matches the app's actual imports — react/react-dom/react-router-dom). Dead-adjacent: two unwired scripts (F-002), and tools/eslint-plugin-local is a path-import not a devDependency (consistent with a workspace-local plugin; see file 25).

## 8. Dependencies & flags
- Runtime: react 19.3.1, react-dom 19.3.1, react-router-dom 7.18.4 (exact). Dev: toolchain only — eslint/typescript-eslint, stylelint pair, vite/vitest/jsdom/testing-library, playwright pair, types. `"type": "module"` (ESM) — all scripts are .mjs node scripts.

## 9. Open questions for the next agent
- The lint gate's HEAD status (F-002) — the REQUEST-ORCH-RUN basket already covers lint:css; consider adding `npm run lint`.
- web-matrix-run.mjs and dev-shots-drift-check.mjs contents were not audited here (not in my file set) — if they remain unwired, they are dead tooling regardless of their internals.
