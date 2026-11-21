# AUDIT: ui/tsconfig.json
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 30 / 19 compilerOptions keys + 1 include array (6 entries) |
| File verdict | REAL (one evidence-integrity gap on its gate, two hygiene debts) |
| Reachable from shipped entry point? | N/A (build config) — consumed by `tsc --noEmit` via package.json:9 `"build": "tsc --noEmit && vite build"` and :13 `"typecheck"`. Its include set covers src, e2e (both spec trees verified on disk), vite.config.ts, and all three playwright configs (all exist). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `Read` tsconfig.json (complete) + vite.config.ts (vitest config location); `ls e2e tools` (all include targets exist; tools/eslint-plugin-local real); `rg -n "from \"node:" playwright.web.config.ts` (:13-15 node:fs/path/url imports — resolve via @types/node devDep); `rg -n '"@types/node"\|"typescript"' package.json` (typescript 6.0.3, @types/node ^26.6.5); `sed -n '93,98p' src/screens/settings/SettingsProvider.tsx` (noUncheckedIndexedAccess corroborated by the `total` helper's own comment); `rg -l 'import\.meta as ImportMeta' src \| wc -l` → **18 files** (F-002 evidence); `rg -l 'from "\.\./\.\./\.\./\.\./' src \| wc -l` → 26 files (F-003 evidence); full read of ALL baseline ui logs — including the raw first-attempt `ui-typecheck.log` / `ui-build.log` / `ui-test.log` (17:12, pre-install: `sh: 1: tsc: not found`) vs the rerun `*.log.txt` (17:20, post `npm ci`) — and `ls ui/node_modules` (ABSENT in this workspace → tsc cannot be re-run here). |

## 1. One-paragraph verdict
A tight, modern, honestly strict single tsconfig: `strict` plus seven additional hardening flags (`noUncheckedIndexedAccess`, `noUnusedLocals/Parameters`, `noFallthroughCasesInSwitch`, `noImplicitOverride`, `verbatimModuleSyntax`, `isolatedModules`), whose effects are corroborated in the codebase itself (the `total()` helper in SettingsProvider exists only to satisfy `noUncheckedIndexedAccess`; `import type` discipline matches `verbatimModuleSyntax`). The include list is honest — every named file exists, and the vitest config (inside vite.config.ts) is covered. Three defects: the heaviest is evidentiary, not configurational — the ONLY direct tsc evidence in the baseline is a FAILED run (`tsc: not found`, pre-install), and the successful build log that everything cites shows vite output with the npm banner trimmed, so "tsc --noEmit ran and passed" rests on the operator's CMD header + `&&` semantics, not on visible output (tsc is silent on success); tsc cannot be re-run here (no node_modules in this workspace). Second, `types: ["vite/client"]` excludes Node's types while the test suite runs on Node and reads files — forcing the `import.meta as ImportMeta & { dirname?: string }` cast in 18 test files. Third, no path aliases, with 26 files doing four-level relative imports as the measurable consequence.

## 2. What actually works (verified)
— Verified by disk checks: all six include targets exist (src, e2e/{a11y,smoke,theme,visual,web}, vite.config.ts, playwright.{,web.,visual.}config.ts); no phantom includes, no missing tsconfig.node.json/app.json split to reconcile (single-file config).
— Verified by sed (SettingsProvider.tsx:95-97): the `total` helper's own comment ("the assertions only satisfy noUncheckedIndexedAccess") proves the flag is active and felt in the code — the config is not aspirational.
— Verified by package.json + read: typescript 6.0.3 + @types/node ^26.6.5 (devDeps) make `moduleResolution: "bundler"`, `verbatimModuleSyntax`, and the node: imports in playwright.web.config.ts:13-15 all resolvable under this config.
— Verified by baseline #24 + my own direct node run (i18n-dead-keys audit): the tooling scripts around this config are reproducible; the config-file ecosystem (package.json scripts ↔ configs on disk) is internally consistent.

## 3. Findings

### F-001 [S3]
Where: ui/package.json:9 (`"build": "tsc --noEmit && vite build"`) + audit/runs/baseline/ui-build.log.txt (evidence) + audit/runs/baseline/ui-typecheck.log (counter-evidence)
Code:
```
CMD: npm run build (tsc --noEmit && vite build)
DATE: 2026-11-21T06:18:47Z
- Using dynamic import() to code-split the application
```
What is wrong: the committed evidence that TypeScript type-checking passes at HEAD is ambiguous. (a) The raw `ui-typecheck.log` (17:12, first attempt, pre-`npm ci`) records `sh: 1: tsc: not found` — the only DIRECT tsc invocation in the baseline, and it FAILED on environment, not on types. (b) The rerun `ui-build.log.txt` (17:20) documents `npm run build (tsc --noEmit && vite build)` → EXIT=0 but contains ONLY vite output: the npm script banner (`> lumen-ui@0.1.0 build`) is absent — the log is demonstrably trimmed (its sibling ui-vitest.log.txt shows 5 of 136 test files with the summary intact) — and `tsc --noEmit` prints NOTHING on success. So "tsc ran" is inferred from the operator's CMD header plus `&&` exit semantics, not from visible output. (c) tsc cannot be re-run in this audit workspace: `ui/node_modules` does not exist (verified). If the header were inaccurate (e.g. a direct `vite build` after the pre-install failure), NO baseline evidence would show the tree type-checks — and vite/esbuild strips types without checking them.
Why it matters: this config's entire value is the gate `tsc --noEmit && vite build`; every UI audit this wave (including my own "compiler-enforced exhaustiveness" claims in StateRouter.tsx and the exhaustiveness arguments in error-display consumers) cites baseline #22 as proof the strict flags pass at HEAD. That proof currently has a trimmed-banner hole.
What correct looks like: REQUEST-ORCH-RUN (logged): `cd ui && npm ci && npm run typecheck` — one run with visible banner+exit closes the gap permanently; future baseline logs should not trim banners of silent-on-success commands.
Confidence: HIGH (that the evidence is ambiguous), MEDIUM (that tsc actually failed — the balance of evidence, INDEX #22 + header + `&&`, says it ran).

### F-002 [S4]
Where: ui/tsconfig.json:20
Code:
```json
    "types": ["vite/client"]
```
What is wrong: the types array admits only vite/client globals while the test suite (included via `include: ["src", …]`, executed by vitest on Node) uses Node runtime APIs. The measurable cost: 18 test files carry the cast workaround `const { dirname } = import.meta as ImportMeta & { dirname?: string };` (rg count; examples: StateRouter.test.tsx:17, Switch.test.tsx:7, Button.test.tsx, List.test.tsx) — an optional-property cast on a value that ALWAYS exists at runtime under Node ≥20.11, repeated 18 times purely because the type world excludes Node.
Why it matters: 18 copies of a lie-by-type-omission (`dirname?: string` — it is never undefined in the vitest runtime); every new file-reading test must rediscover the hack.
What correct looks like: `"types": ["vite/client", "node"]` (the @types/node devDep already exists for module resolution) — the casts then delete; or a shared test-utils `readSibling(path)` helper owning the one cast.
Confidence: HIGH — Verification: rg count 18 + read of two instances + package.json devDep check.

### F-003 [S4]
Where: ui/tsconfig.json:2-21 (absence of `baseUrl`/`paths`)
Code:
```json
  "compilerOptions": {
    "target": "ES2022",
```
What is wrong: no path aliases are configured; the measurable consequence is 26 files importing through four-level relative chains (`../../../../`) — e.g. the states lane reaching `../../../client/transport` (StateRouter's context-bridge.ts:21-22) and player components climbing out of four directories to reach components/i18n.
Why it matters: structure debt with a rename tax: moving any intermediate directory rewrites import lines across dozens of files (mechanical but noisy diffs that pollute blame); the chains also make the UI boundary rule ("UI code never imports outside ui/src", vite.config.ts:13-14) harder to eyeball.
What correct looks like: `"paths": { "@/*": ["src/*"] }` (or the lane prefix convention of the codebase's choice) + codemod; the boundary rule then becomes one readable pattern.
Confidence: HIGH — Verification: rg count 26 for the four-level pattern; deep chains observed directly in every player-lane file I audited this wave.

## 4. Per-function (config-key) table
| Key | Value | Verdict | Severity | Notes |
|---|---|---|---|---|
| target | ES2022 | REAL | — | Matches lib; browsers OK. |
| lib | ES2022, DOM, DOM.Iterable | REAL | — | Consistent with target; no stale DOM libs. |
| module | ESNext | REAL | — | Bundler-native; matches Vite. |
| moduleResolution | bundler | REAL | — | Requires TS ≥5.0; typescript 6.0.3 ✓. |
| jsx | react-jsx | REAL | — | Automatic runtime (no `import React` anywhere in the audited lanes ✓). |
| strict | true | REAL | — | Baseline-dependent proof only (F-001). |
| noUncheckedIndexedAccess | true | REAL | — | Corroborated: SettingsProvider.tsx:95-97's `total` helper exists to satisfy it. |
| noUnusedLocals / noUnusedParameters | true | REAL | — | Consistent with the codebase's clean parameter discipline (ignored params via `_`-prefix not needed in audited files). |
| noFallthroughCasesInSwitch | true | REAL | — | Load-bearing for StateRouter.tsx's two intentional fall-through groups (playing/paused/stopped/ended) — legal only because the flag demands explicit empty-case grouping, which the file does correctly. |
| noImplicitOverride | true | REAL | — | No class hierarchies observed in UI lanes; dormant but harmless. |
| verbatimModuleSyntax | true | REAL | — | Corroborated: `import type` discipline is uniform across every audited file. |
| isolatedModules | true | REAL | — | Required by Vite/esbuild per-file transpilation; consistent. |
| useDefineForClassFields | true | REAL | — | ES2022 semantics; matches target. |
| skipLibCheck | true | REAL | — | Standard; hides dependency .d.ts errors (accepted industry tradeoff, noted). |
| resolveJsonModule | true | REAL | — | Used by vite.config.ts:7 (`import pkg … with { type: "json" }`). |
| noEmit | true | REAL | — | Typecheck-only; emit is Vite's job. |
| types | ["vite/client"] | REAL (with debt) | F-002 | Excludes Node globals; 18 test-file casts result. |
| include | src, e2e, vite.config.ts, 3× playwright configs | REAL | — | All targets exist on disk; vitest config (in vite.config.ts) covered; tools/ (plain .js, no allowJs) intentionally outside tsc — consistent. |

## 5. Test honesty
No tests target tsconfig.json. Its "test" is the build gate — see F-001 for why that gate's evidence is currently weaker than the tree deserves. The config's own strictness flags are corroborated by in-code artifacts (F-002's casts, SettingsProvider's helper) — the code visibly LIVES under these flags.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| (implied by build script) "the tree passes tsc --noEmit under this strict config" | package.json:9 + baseline INDEX #22 | REAL-UNPROVEN | EXIT=0 of `tsc --noEmit && vite build` per operator header; no visible tsc output either way; no re-run possible here (F-001). |
| vite.config.ts:5-6 "Node ≥20.10 / TS ≥5.3 syntax, both satisfied by the toolchain" | vite.config.ts | REAL | typescript 6.0.3 (package.json), import attributes used and covered by this config. |

## 7. Dead / duplicated / unreachable code
- No dead keys. No duplicate configs (single file — verified `ls tsconfig*`).
- tools/eslint-plugin-local (.js) is deliberately outside tsc scope (no allowJs) but inside vitest scope (vite.config.ts:43) — consistent split, noted for its owner.

## 8. Dependencies & flags
- typescript 6.0.3, @types/node ^26.6.5 (module-resolution only today — F-002 proposes promoting it to the types array), vite 8.3.5, vitest 5.0.5. No feature flags.

## 9. Open questions for the next agent
- F-001's REQUEST-ORCH-RUN is the single highest-value cheap run for the whole UI lane: one `npm run typecheck` with banner converts every "compiler-enforced" claim in this wave's audits from REAL-UNPROVEN to REAL.
- If aliases land (F-003), the codemod must respect the boundary rule's eyeball-checkability (vite.config.ts:13-14) — an `@/` prefix keeps "outside ui/src" violations greppable.
