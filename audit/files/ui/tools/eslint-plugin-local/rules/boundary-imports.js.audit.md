# AUDIT: ui/tools/eslint-plugin-local/rules/boundary-imports.js
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-24 |
| Lines / functions | 54 / 1 ESLint rule |
| File verdict | REAL (working boundary rule on POSIX; two portability/coverage gaps) |
| Reachable from shipped entry point? | YES as tooling — loaded by eslint.config.js:14 (`local`) and enabled at :109 (`local/boundary-imports: "error"`); runs in `npm run lint`. Not shipped in the bundle. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg plugin wiring (eslint.config.js:14,109); read; baseline full_gate.txt lint-literal section checked (different checker; eslint run UNVERIFIED — node_modules absent) |

## 1. One-paragraph verdict
The rule does what it says on POSIX: absolute paths rejected, bare specifiers allowed, relative imports resolved and required to stay under ui/src (srcRoot resolution via the module's own location is correct: tools/eslint-plugin-local/rules → ../../../src = ui/src). Two gaps: the containment check appends a hardcoded forward slash, so on Windows (backslash separators from path.resolve) every in-src relative import fails `startsWith(srcRoot + "/")` and the rule would flood false positives — breaking `npm run lint` on a platform the repo explicitly targets; and only static `ImportDeclaration` is visited, so dynamic `import("…")` escapes the boundary unflagged.

## 2. What actually works (verified)
— rg: rule wired into eslint.config.js:14 + enabled :109 "error".
— path math verified by hand: resolve(tools/eslint-plugin-local/rules, ../../../src) = ui/src.

## 3. Findings
F-001 [S4]
Where: ui/tools/eslint-plugin-local/rules/boundary-imports.js:46
Code:
```
        if (target !== srcRoot && !target.startsWith(srcRoot + "/")) {
          context.report({ node, messageId: "outsideSrc" });
        }
```
What is wrong: `srcRoot + "/"` assumes POSIX separators. On Windows, `path.resolve` produces backslash paths, so even `./foo` inside src fails the startsWith test and is reported — the lint lane becomes all-noise on a supported platform (rc-smoke tracks a windows lane). Fix: compare against `path.sep` or use `path.relative(srcRoot, target)` and check for `..`/absolute. Confidence: HIGH — Verification: read + separator reasoning; not executed on Windows (UNVERIFIED executionally).
F-002 [S4]
Where: ui/tools/eslint-plugin-local/rules/boundary-imports.js:33
Code:
```
      ImportDeclaration(node) {
```
What is wrong: Dynamic imports (`import("…")`, ImportExpression) are not visited — a boundary escape via lazy import passes the rule. The repo does use dynamic imports (App.tsx route-splitting, per the vite INEFFECTIVE_DYNAMIC_IMPORT note in the build baseline). Confidence: HIGH — Verification: read; node type not handled.

## 4. Per-function table
n/a (<100 LOC).

## 5. Test honesty
No unit tests for the rule found in my files; eslint execution UNVERIFIED here (node_modules absent — npm install banned).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Files under src/ may not relatively import outside ui/src" | boundary-imports.js:8-9 | TRUE on POSIX, UNRELIABLE on Windows | F-001 |
| "Rule semantics are unchanged" (from the promotion) | boundary-imports.js:1-2 | TRUE | logic matches the described inline original |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
node:path, node:url only. Loaded via tools/eslint-plugin-local/index.js.

## 9. Open questions for the next agent
- Does CI ever run `npm run lint` on a windows runner? If yes, F-001 would have been caught — check .github/workflows.
