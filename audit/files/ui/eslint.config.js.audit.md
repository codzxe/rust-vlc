# AUDIT: ui/eslint.config.js
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 116 / 6 config blocks |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (dev tooling) — exercised by `npm run lint` in CI (.github/workflows/ui.yml:102-111, chained with lint:css) and by the plugin's own vitest suite; NOT part of the audit baseline runs (INDEX #21-24 has no lint step) — lint pass is REAL-UNPROVEN on baseline evidence alone. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | ls tools/eslint-plugin-local (3 rules + tests, 613 LOC — exists); Read .github/workflows/ui.yml:70-144 (lint + lint:css steps); grep 'lint' .github/workflows (ui.yml only); cross-check scripts globals vs dev-shots.mjs/l10n-pseudo-probe usage. |

## 1. One-paragraph verdict
A coherent flat config: the three custom rules live in a tested in-repo plugin (promoted from this file, issue #36), generated sources get a documented exemption from unused-directive hygiene because the generators own their output, and the per-script global blocks match what each script actually uses (dev-shots' `window` handled by its inline `/* global */`, the probe's browser globals by its block, web-matrix's URL by its own). Two nits: the plugin is imported by relative path rather than as a workspace dependency, and the browser-globals declaration mechanism is split between config blocks and inline comments across scripts — two mechanisms for one concern.

## 2. What actually works (verified)
— Plugin exists with unit tests (rules/*.test.js, 613 LOC total); CI declares `npm run lint` + `npm run lint:css` (ui.yml:102-111) with an honest comment recording why lint:css was added (the fonts.css @font-face slip that a never-run gate cannot catch). Baseline-adjacent: vitest 1705 passed includes the plugin's rule tests (ui-vitest.log.txt — 136 files).

## 3. Findings
### F-001 [S4]
Where: ui/eslint.config.js:14
Code:
```
import local from "./tools/eslint-plugin-local/index.js";
```
What is wrong: the plugin is imported by relative path (it has its own package.json but is not a ui dependency), so the config depends on the tools/ directory layout rather than package resolution.
Why it matters: moving tools/ or publishing the plugin breaks the config at lint time with a resolution error — no package.json entry documents the relationship.
What correct looks like: `"eslint-plugin-local": "file:./tools/eslint-plugin-local"` in devDependencies + a normal import.
Confidence: HIGH — read both package.json files.
### F-002 [S4]
Where: ui/eslint.config.js:33,41-55,62,75-85 vs ui/scripts/dev-shots.mjs:2
Code:
```
    languageOptions: { globals: { console: "readonly", process: "readonly" } },
```
```
/* global window: readonly -- the addInitScript callbacks below evaluate in the PAGE context; ... */
```
What is wrong: browser-globals for page-context callbacks are declared THREE ways across the scripts tree: config blocks for e2e/web/site, a dedicated block for l10n-pseudo-probe (with its own URL case for web-matrix-run), and an inline comment for dev-shots. Same concern, three mechanisms, each with a prose justification.
Why it matters: the next Playwright driver script has to pick a mechanism and may pick wrong (no-undef false positives or missing a global); the pattern scales poorly.
What correct looks like: one shared "playwright-driver" globals block (console/process/setTimeout/document/window) applied to the driver scripts that need it.
Confidence: HIGH — all four sites read.

## 4. Per-function table (config blocks)
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ignores | 17-29 | REAL | — | dist/node_modules/coverage/reports + sitedist (documented as build output) |
| scripts/**/*.mjs globals | 30-34 | REAL | S4 (F-002) | console/process |
| e2e/web/site globals | 35-56 | REAL | — | browser globals enumerated |
| web-matrix-run globals | 57-64 | REAL | — | +URL |
| l10n-pseudo-probe globals | 65-86 | REAL | — | history note: pre-existing no-undef on main blocked lint (P4-GATE) |
| generated exemption | 89-100 | REAL | — | reportUnusedDisableDirectives off — drift-checked by the three *:check scripts instead |
| src rules block | 101-115 | REAL | S4 (F-001 via import) | 3 local errors + hooks/refresh warns |

## 5. Test honesty
The plugin's rules have real unit tests (rules/*.test.js). Mutation: break i18n-no-literals' literal detection → its test fails. The config file itself is only exercised by running lint — declared in CI, absent from baseline logs.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "promoted verbatim from this file, unit-tested via vitest" | eslint.config.js:4-6 | REAL | plugin + tests exist; vitest baseline includes them |
| "today's output triggers no rules, so the blanket directive reads as 'unused'" | eslint.config.js:94-95 | REAL-UNPROVEN | would need a lint run (REQUEST-ORCH-RUN below if the orchestrator wants it proven) |
| CI runs lint + lint:css | ui.yml:102-111 | REAL (workflow text) | not executed in baseline |

## 7. Dead / duplicated / unreachable code
F-002 (three globals mechanisms). No dead config blocks — every files: pattern matches real files (verified per directory).

## 8. Dependencies & flags
@eslint/js, eslint-plugin-react-hooks, eslint-plugin-react-refresh, typescript-eslint, tools/eslint-plugin-local (F-001). ESLint 10.13.0 per package.json.

## 9. Open questions for the next agent
- tools/eslint-plugin-local owner (asset-group? another wave): do the three rules' tests cover the CSS-token rule against actual violations, or only happy paths? (613 LOC skimmed, not audited — not my assignment.)
- REQUEST-ORCH-RUN (optional, to close the REAL-UNPROVEN): `cd ui && npm run lint && npm run lint:css`
