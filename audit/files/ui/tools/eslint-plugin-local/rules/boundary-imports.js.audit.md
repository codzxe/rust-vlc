# AUDIT: ui/tools/eslint-plugin-local/rules/boundary-imports.js
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 54 / 1 (boundaryImports rule object: meta, create) |
| File verdict | REAL |
| Reachable from shipped entry point? | TOOLING (not shipped): wired into eslint.config.js:102-110 scoped to `src/**/*.{ts,tsx}` with `"local/boundary-imports": "error"`; executed by `npm run lint` in CI (.github/workflows/ui.yml:102-105). Baseline did NOT run eslint (INDEX.md #21-24 — UNVERIFIED locally, CI-wired only) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | sed eslint.config.js:95-120 (rule scoping); rg boundary-imports in config (109); rg lint in .github/workflows/ui.yml (102-111); read rule body; path resolution traced by hand |

## 1. One-paragraph verdict
A correct, minimal boundary rule: flags absolute-path imports and relative
imports whose resolved target escapes ui/src, allows bare package
specifiers, anchors the src root relative to its own file location. The
scope claim ("files under src/") is enforced NOT by the rule but by the
eslint config block — the rule itself would flag any file it lints; and
only static ImportDeclarations are checked (dynamic import() and require
escape it). Enforcement runs in CI only; the baseline never executed eslint.

## 2. What actually works (verified)
— verified by read + hand-trace: `resolve(dirname(filename), spec)` vs
`srcRoot` (tools/eslint-plugin-local/rules/ → ../../../src = ui/src —
correct, matches the inline origin); the `target !== srcRoot` carve-out
allows importing exactly the root; absolute-path rejection precedes the
relative check.
— verified by rg: the rule is registered (eslint.config.js:14 plugin
import, :109 rule enable) scoped to src ts/tsx files.
— CI wiring verified: ui.yml step "2. lint" runs `npm run lint`
(102-105); the workflow header records a past "lint (0 errors…)" state.

## 3. Findings
### F-001 [S4] The rule enforces nothing about "files under src/" — scope lives entirely in eslint.config.js
Where: ui/tools/eslint-plugin-local/rules/boundary-imports.js:30-31
Code:
```
  create(context) {
    const filename = context.filename ?? "";
```
What is wrong: The header says "Files under src/ may not relatively import
outside ui/src", but `create` never checks that `filename` is under srcRoot
before reporting — the gate is the config's `files: ["src/**/*.{ts,tsx}"]`
block. Enabled globally (another config, a future refactor), the rule would
misfire on every non-src file with a relative import (e.g. tools/ or
scripts/ files legitimately importing siblings outside src).
Why it matters: Latent misuse hazard + a rule whose documented contract is
only half-implemented in itself.
What correct looks like: Early return when `!filename.startsWith(srcRoot)`
inside create (the rule then enforces its own header anywhere it is
enabled).
Confidence: HIGH — Verification: read (config scoping confirmed as the only
gate).
### F-002 [S4] Only static ImportDeclarations checked — dynamic import() escapes the boundary
Where: ui/tools/eslint-plugin-local/rules/boundary-imports.js:33-49
Code:
```
      ImportDeclaration(node) {
        const spec = node.source.value;
        if (typeof spec !== "string") {
          return;
        }
```
What is wrong: `import("...")` expressions (CallExpression with Import
callee) are not visited; a dynamic relative import escaping src would pass
the rule. require() likewise (moot in the ESM src tree, but the rule does
not know that).
Why it matters: A boundary rule with a trivial bypass; today no src file
uses dynamic import (rg UNVERIFIED — not exhaustively checked), so the gap
is latent.
What correct looks like: Also visit `CallExpression` with `callee.type ===
"Import"` and report non-conforming specs.
Confidence: HIGH — Verification: read (AST visitor list is exhaustive in
the file: one handler).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| boundaryImports.meta | 18-29 | REAL | — | type "problem", two messages, no schema |
| boundaryImports.create | 30-51 | REAL | S4 (F-001/F-002) | absolute check → bare-specifier allow → resolve+startsWith boundary check; lexical resolve only (symlinks not followed — acceptable) |

## 5. Test honesty
No test file for the rule (rg: none in tools/). Enforcement evidence is CI
only (ui.yml lint step); baseline INDEX.md has no eslint run — the rule's
effectiveness is UNVERIFIED in this audit's evidence set. Mutation check
(reasoned): a src file importing "../../foo" would be reported (resolve
escapes srcRoot) — logic traced by hand, not executed.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Files under src/ may not relatively import outside ui/src" | header 8-10 | REAL (via config) / SHALLOW (rule alone) | F-001 |
| "Rule semantics are unchanged" from the inline origin (issue #36) | header 1-4 | UNVERIFIED | the inline pre-move version is not in the tree to diff |
| doc 08 §8 "no Tauri APIs outside bindings" enforced by this rule | header 9-10 | SHALLOW | the rule bounds paths, not API usage — Tauri API leakage would need a different check (transport binding is centralized in client/, which IS inside src) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
node:path, node:url only. ESM module consumed by
tools/eslint-plugin-local/index.js (verified by config import path).

## 9. Open questions for the next agent
- REQUEST-ORCH-RUN candidate: `cd ui && npm run lint` to convert the CI
  claim (0 errors) into baseline-citable evidence for all three local rules.
- Do the sibling rules (i18n-no-literals, semantic-tokens-only) share the
  self-scoping gap (F-001)? Their agents should check.
