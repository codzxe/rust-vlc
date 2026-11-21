# AUDIT: ui/tools/eslint-plugin-local/rules/i18n-no-literals.js
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-11-21 |
| Lines / functions | 145 / 8 (allowlist loader, isTestFile, relToUi, create, check, literalText, 3 AST visitors) + export |
| File verdict | REAL |
| Reachable from shipped entry point? | Tooling (not the lumen binary). Executed in baseline via vitest: vite.config.ts:41-44 includes `tools/eslint-plugin-local/**/*.test.js`, and the RuleTester suite (i18n-no-literals.test.js, 15 cases incl. file-scoping positive+negative) ran green inside "npm test → 1705 passed / 1 skipped" (INDEX.md row 23). Repo-wide wiring: eslint.config.js:108 `"local/i18n-no-literals": "error"` over `src/**/*.{ts,tsx}`, run by `npm run lint` → CI .github/workflows/ui.yml:102-105 step "2. lint". No baseline row executes `npm run lint` → whole-tree pass UNVERIFIED. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:3 |
| Commands run | sed -n verification of :28-33, :54-55, :105-141; Read of index.js/package.json/eslint.config.js/vite.config.ts/i18n-allowlist.json/l10n-check.mjs; rg for fragment-literal `>\s*\{['"\`]` (3 hits, all non-literal: CallExpression/interpolated template); rg for `<input|option|optgroup|track … value=/label=/summary=` (1 hit, test file); rg `lint` in .github/workflows (ui.yml:102-105 runs it). |

## 1. One-paragraph verdict
The rule is real, executed, and honest about its documented limits: JSX text nodes, no-substitution template literals, `{"Copy"}` expression containers in element text position, and 7 user-visible attributes are checked against the useMsg()/MessageKey seam, with a v3 FILE-SCOPED allowlist enforced at load time (missing/empty `files` throws) and a test-file exemption. The RuleTester suite that ran in baseline vitest is genuinely adversarial: it pins the file-scoping contract in BOTH directions (the allowlisted "00:02:10" passes in CustomizePanels.tsx and FAILS in PlaylistScreen.tsx), which proves the cwd-dependent `relToUi` resolution and the scoping logic execute correctly. Three latent coverage/robustness holes, none currently exploitable by code in the tree: (1) expression containers inside shorthand fragments (`<>{'raw copy'}</>`) escape the parent check; (2) rendered copy attributes outside the 7-name list (`value` on submit/reset/button inputs, `option`/`optgroup label`, `summary`, `aria-roledescription`, `aria-keyshortcuts`) are unchecked; (3) duplicate `text` keys in the allowlist silently overwrite each other's file-scopes (Map.set) instead of merging — today's 6 entries are unique, and the failure direction is over-reporting (safe), so S4.

## 2. What actually works (verified)
- Rule registered under historical name: index.js:26 + test :24-26 (plugin.rules property assertion) — ran green in baseline row 23.
- File-scoped allowlist contract executed both ways: i18n-no-literals.test.js:35-38 (valid, "00:02:10" in CustomizePanels.tsx) and :81-85 (invalid, same text in PlaylistScreen.tsx) — 1705/1705 pass proves relToUi + Map/Set scoping work under vitest cwd (ui/).
- Load-time hardening: :44-53 throws on entry without non-empty `files` (v3 scoping) — verbatim quote verified; duplicated earlier by l10n-check.mjs:152-157 with file/line context (dual gate, real).
- Attribute unwrapping handles both `alt="..."` and `alt={"...""` shapes (:132-139) without double-reporting (container visitor skips attribute-position containers via the JSXElement parent check).
- Documented-limits honesty: header :15-17 admits interpolated templates and DOM-helper literals are NOT covered ("document, don't fake") — matches code (literalText returns null for substitution templates).
- Reachability wiring: eslint.config.js:108 error-level registration; CI ui.yml:102-105 `npm run lint`; test-exemption regex matches the repo's real test naming.

## 3. Findings
### F-001 [S4]
Where: ui/tools/eslint-plugin-local/rules/i18n-no-literals.js:110-118
Code:
```
      JSXExpressionContainer(node) {
        if (node.parent?.type !== "JSXElement") {
          return;
        }
```
What is wrong: The parent guard admits only `JSXElement`. An expression container that is a child of a shorthand JSX fragment (`<>{'Raw copy'}</>`) has parent type `JSXFragment` and is skipped — raw user-visible copy in a fragment child escapes the rule. (JSXText is NOT affected: that visitor has no parent check, so text nodes inside fragments are caught.)
Why it matters: The rule is the doc 07 §6.5 enforcement gate; a documented-as-covered shape ("string-literal expression containers in text position", header :9-10) silently excludes fragment text position. rg census: no current offender in ui/src (only CallExpression/interpolated-template hits) — latent, zero live violations today.
What correct looks like: `if (node.parent?.type !== "JSXElement" && node.parent?.type !== "JSXFragment") return;`
Confidence: HIGH — Verification: read + rg census of `>\s*\{['"\`]` over ui/src.
### F-002 [S4]
Where: ui/tools/eslint-plugin-local/rules/i18n-no-literals.js:119-131
Code:
```
        if (
          name !== "aria-label" &&
          name !== "alt" &&
          name !== "title" &&
          name !== "placeholder" &&
```
What is wrong: The attribute allowlist omits attributes the browser RENDERS as copy: `value` on `<input type=submit|reset|button>` (button label), `label` on `<option>`/`<optgroup>`/`<track>`, `summary` on `<table>`, plus `aria-roledescription`/`aria-keyshortcuts` (spoken by SR). `value="Save changes"` on a submit input sails through.
Why it matters: Same gate-bypass class as F-001; header documents the covered set as "user-visible attrs" without disclosing these omissions (unlike the honest `data-*` note). rg census: zero current offenders in ui/src (one `aria-label` test hit only) — latent.
What correct looks like: Add the rendered-copy attributes (value/label/summary) to the name check, or document the omission alongside the `data-*` note.
Confidence: HIGH — Verification: read + rg census `<input|option|optgroup|track [^>]*(value|label|summary)=`.
### F-003 [S4]
Where: ui/tools/eslint-plugin-local/rules/i18n-no-literals.js:54
Code:
```
  allowlistedTexts.set(entry.text, new Set(entry.files));
```
What is wrong: Two allowlist entries sharing one `text` overwrite each other: entry 2 replaces entry 1's file-set instead of merging, so every file named only by entry 1 loses its exemption and starts failing lint with a message that points at the allowlist contract rather than the duplicate.
Why it matters: The v3 file-scoping model is this rule's exemption mechanism; silent last-wins on duplicate texts is a config foot-gun. Current allowlist (ui/i18n-allowlist.json, 6 entries: "—", "(", ")", "/", "00:02:10", "00:25:40") has unique texts and neither validator (this rule nor l10n-check.mjs rule 6) detects duplicates — latent. Failure direction is over-reporting (CI fails loudly), not a bypass, hence S4 not S3.
What correct looks like: Merge on collision (`existing.addAll(entry.files)`) or throw on duplicate `text` (mirroring the load-time strictness three lines up).
Confidence: HIGH — Verification: read + allowlist read (6 unique texts).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module allowlist loader | 24-55 | REAL | S4 | Synchronous readFileSync at import; array + shape checks; throws on text-only entries (v3); F-003 last-wins set |
| isTestFile | 57 | REAL | — | Regex matches repo test naming (.test.tsx etc.); RuleTester's own fixtures are .tsx non-test → NOT exempt (correct: scoping test relies on that) |
| relToUi | 59-62 | REAL | — | cwd-dependent for relative filenames (RuleTester); proven working under vitest cwd=ui/ by baseline row 23; absolutizes + forward-slashes for real runs |
| create | 74-78 | REAL | — | context.filename fallback ""; test files return {} (full exemption) |
| check | 80-90 | REAL | — | Trim; empty→pass; file-scoped allowlist lookup; report single messageId |
| literalText | 92-104 | REAL | — | Literal string or no-substitution template; cooked ?? "" (invalid escape → skipped, benign) |
| JSXText visitor | 106-108 | REAL | — | No parent check → covers fragments too (contrast F-001) |
| JSXExpressionContainer visitor | 110-118 | REAL | S4 | F-001 fragment hole |
| JSXAttribute visitor | 119-140 | REAL | S4 | F-002 attribute list; handles literal + container values; JSXNamespacedName name → object compare → no match, no crash |
| default export | 145 | REAL | — | Consumed by index.js:26 |
## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| "is registered under its historical name" (:24) | YES | Identity check against plugin index |
| RuleTester valid set (:30-56): seam usage, scoped allowlist hit, whitespace-only, non-literal container, call-expr attrs, data-placeholder, test-file exemption | YES | Each pins a distinct branch; the scoped-allowlist case fails if relToUi or the files Set scoping breaks |
| RuleTester invalid set (:57-96): JSX text, alt literal, placeholder literal (the j20 live miss), template placeholder, allowlisted-text-in-wrong-file, {'Copy'} container, template title | YES | The wrong-file case (:81-85) is the scoping regression test — would pass silently if scoping regressed to v2 global matching. Mutation reasoning: delete `files.has(relFile)` → :81-85 fails; delete trim → whitespace case over-reports |
| (whole-suite reach) | YES | Included in baseline vitest via vite.config.ts:41-44; 1705 passed (row 23) is the execution proof |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "promoted verbatim from ui/eslint.config.js … semantics unchanged" | :1-5 | REAL-UNPROVEN | No git history access (session-2); semantics self-consistent with the registered config usage |
| "covers: JSX text nodes, no-substitution template literals, string-literal expression containers in text position, and … aria-label, alt, title, placeholder, aria-placeholder, aria-valuetext, aria-description" | :8-12 | REAL | Visitors :105-140; the test suite pins each shape |
| "data-* attrs are NOT covered on purpose" | :14-15 | REAL | Name check excludes them; test :50 pins it |
| "an entry without a non-empty files array is now a load-time config error" | :21-22 | REAL | :44-53 throw; mirrored by l10n-check.mjs:152-157 |
| "Still NOT covered: interpolated templates and DOM-helper literals — document, don't fake" | :15-17 | REAL | literalText returns null on substitution templates |
| (implied) fragment text position covered | :9-10 | FALSE-by-omission | F-001: expression containers in JSXFragment escape |
## 7. Dead / duplicated / unreachable code
- None. All three visitors carry live logic; the allowlist loader is shared source-of-truth with l10n-check.mjs (duplicated VALIDATION intentionally, different failure contexts — documented at l10n-check.mjs:148-151).
## 8. Dependencies & flags
node:fs/path/url only (zero npm deps). Consumers: index.js (plugin), eslint.config.js:108 (error, src/**/*.{ts,tsx}), CI ui.yml:102-105. Upstream data: ui/i18n-allowlist.json (6 entries, all file-scoped). Sibling validator: scripts/l10n-check.mjs rules 6/6b. Note: the rule only fires under `npm run lint`, which NO baseline row executed — the suite-level execution (vitest) is the only committed evidence.
## 9. Open questions for the next agent
- i18n-allowlist.json (my file 25) — cross-check each entry's text against its scoped files (l10n-check 6b does this at gate time; I will verify presence statically).
- If any agent finds raw copy in a `<>{'…'}</>` shape, promote F-001 to S3 with that file as the live proof.
