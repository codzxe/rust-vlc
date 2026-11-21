# AUDIT: ui/tools/eslint-plugin-local/package.json
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 13 / 6 keys (name, version, private, type, description, exports, engines) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (tooling manifest). The PLUGIN is reachable: eslint.config.js:14 imports it (path import) and registers its three rules at "error" (:108-110) — the `npm run lint` gate; its three rule test files provably ran in baseline #23 (133 src + 3 tools = 136 test files, arithmetic match with the vitest log) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read full file + dir listing (index.js + rules/{boundary-imports,i18n-no-literals,semantic-tokens-only}.js + 3 .test.js) + index.js head · `rg -n "eslint-plugin-local" --glob '!tools/**' .` (sole consumer: eslint.config.js, FILE-PATH import) · `rg -n "include" vite.config.ts` + sed :38-46 (the vitest glob covers tools/eslint-plugin-local/**/*.test.js) · test-file arithmetic: find src = 133, find tools = 3, baseline log "Test Files 136 passed" · baseline ui-vitest.log.txt (truncated to 18 lines — file list not committed; inclusion proven by the count, not the listing) |

## 1. One-paragraph verdict
The plugin behind this manifest is real and enforced: three promoted rules registered at "error" in the workspace ESLint config, each with a sibling test file, and the test files demonstrably executed in the baseline vitest run (133+3=136 file-count match). The manifest itself, however, is decorative for its only consumer: eslint.config.js imports `./tools/eslint-plugin-local/index.js` by explicit file path, so the package name, version, private flag, exports map, and engines range are never consulted by anything — no tool resolves "eslint-plugin-local" by name, npm never installs it (it is not a devDependency), and the exports map it declares points at a file nothing addresses through it. Harmless but misleading packaging: the directory is dressed as an installable package while living as a path-imported folder.

## 2. What actually works (verified)
— The plugin loads: eslint.config.js:14 `import local from "./tools/eslint-plugin-local/index.js"` + :108-110 registers `local/i18n-no-literals`, `local/boundary-imports`, `local/semantic-tokens-only` at "error" — the issue-#36 lint discipline the stylelint config's header calls its "TSX side".
— The three rules and their tests exist (rules/ listing) and ran green: vite.config.ts:41-44 includes `tools/eslint-plugin-local/**/*.test.js`, and baseline #23's "136 passed" equals 133 src + 3 tools test files exactly.
— `"type": "module"` matches the .js ESM sources; `exports."."` → ./index.js matches the real entry file.

## 3. Findings

### F-001 [S4]
Where: ui/tools/eslint-plugin-local/package.json:2-12
Code:
```
  "name": "eslint-plugin-local",
  "version": "0.1.0",
  "private": true,
```
```
  "exports": {
    ".": "./index.js"
  },
```
What is wrong: every resolution-oriented field is dead. The sole consumer (eslint.config.js:14) imports the entry by explicit relative path, so the exports map, package name, and version are never read; the package is not a devDependency of ui/package.json, so npm never installs it (and never enforces the `engines` range either). No file in the repo imports "eslint-plugin-local" by name (rg: zero matches outside the directory itself).
Why it matters: a manifest that promises package semantics (name resolution, installability, engine enforcement) that nothing exercises misleads the next contributor into `npm install`ing it, importing it by name, or trusting the engines gate — none of which work as the shape implies.
What correct looks like: either make it a real workspace package (add `"eslint-plugin-local": "file:tools/eslint-plugin-local"` to ui devDependencies and import by name — then exports/engines become live) or strip the manifest to a README-style comment and keep the folder as plain config-local modules.
Confidence: HIGH — Verification: read + exhaustive rg for by-name imports + devDependencies cross-check.

## 4. Per-function table (manifest keys)
| Key | Lines | Verdict | Notes |
|---|---|---|---|
| name/version/private | 2-4 | INERT | nothing resolves by name; never installed |
| type: module | 5 | REAL | matches the ESM sources |
| description | 6 | REAL (accurate) | the promotion + 'local' key claims verified |
| exports | 7-9 | INERT | F-001 — path import bypasses it |
| engines | 10-12 | INERT | npm never checks engines for a non-installed path import |

## 5. Test honesty
The plugin's own tests (rules/*.test.js) are the manifest's real proof of life — executed in baseline #23 (136-file arithmetic). NOTE: the committed ui-vitest.log.txt is truncated to 18 lines (5 sample file entries + summary), so per-file results for the tools tests are not individually citable — the count match is the evidence.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "promoted verbatim from the former inline definitions in ui/eslint.config.js. Rule names and semantics are unchanged" | description :6 | UNVERIFIABLE (no git in session-2) / names consistent | the three rule names match eslint.config.js:108-110 |
| "the consuming config registers this package under the 'local' plugin key" | :6 | REAL | eslint.config.js:14 + :108-110 |
| "Unit tests live next to each rule (rules/*.test.js, vitest) and are wired into npm run test" | index.js:12-13 | REAL | vite.config.ts:41-44; baseline 136-file count |

## 7. Dead / duplicated / unreachable code
- The manifest's resolution fields (F-001). The plugin code itself is live (lint gate + tests).

## 8. Dependencies & flags
- None declared (no dependencies block — the rules are dependency-free ESLint modules). Node engines range covers the repo's node 24 CI.

## 9. Open questions for the next agent
- The three rule implementations (boundary-imports.js, i18n-no-literals.js, semantic-tokens-only.js) are not in my file set — their audit should check the exemption lists (the phase-4 worklog mentions an unimplemented "technical-identifier exemption" for className/data-testid/icon/key/ref).
- If F-001's file: dependency route is taken, the lockfile gains an entry — re-run the lockfile audit.
