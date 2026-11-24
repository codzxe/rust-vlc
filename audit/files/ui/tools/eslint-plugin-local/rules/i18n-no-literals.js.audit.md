# AUDIT: ui/tools/eslint-plugin-local/rules/i18n-no-literals.js
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 145 / 5 (allowlist loader, isTestFile, relToUi, i18nNoLiterals.create + literalText) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — dev lint tooling (wired as "error" in eslint.config.js:108; not shipped) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n "i18n-no-literals" eslint.config.js` (L14 import, L108 "error"); `rg -n "it\(" i18n-no-literals.test.js` (2 tests w/ RuleTester valid+invalid) |

## 1. One-paragraph verdict
Real, disciplined lint rule: enforces useMsg()-sourced copy over raw literals in JSX text, no-substitution templates, text-position expression containers, and 7 user-visible attributes; file-scoped allowlist with load-time shape enforcement (empty `files` array = config error — closes the v2 global-match hole); test files exempt; limits (interpolated templates, DOM-helper strings) documented rather than faked. Zero findings.

## 2. What actually works (verified)
— verified by wiring check (eslint.config.js:108 "error") and RuleTester-based tests at i18n-no-literals.test.js:24-28 (registration + reject/allowlist/test-exemption semantics). Baseline: UI npm lanes pass (§13).

## 3. Findings
None. Checked: allowlist validation fail-loud (L44-53); file-soping via Set.has(relFile) (L85-88); attribute list matches documented set (L122-128); trimmed-empty skip (L82-84).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| allowlist loader | 32-55 | REAL | — | load-time throw on shape violations |
| isTestFile / relToUi | 57-62 | REAL | — | path normalization for RuleTester vs real runs |
| create/handlers | 74-141 | REAL | — | JSXText, expression containers (parent-gated), 7 attrs |
| literalText | 92-104 | REAL | — | Literal + no-substitution TemplateLiteral only (disclosed) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| :24 plugin registration | YES | property assertion |
| :28 reject + allowlist + exemption | YES | RuleTester invalid/valid cases |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v3 ... covers: JSX text nodes, no-substitution template literals, ... and literal/expression values of the user-visible attrs" (L8-12) | header | REAL | handlers L105-140 match |
| "Still NOT covered: interpolated templates ... document, don't fake" (L16-17) | header | REAL | literalText returns null for expressions>0 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
node:fs/path/url only. Allowlist at ui/i18n-allowlist.json (not audited — outside assignment; a hostile pass could check whether allowlist entries are stale).

## 9. Open questions for the next agent
- ui/i18n-allowlist.json contents: are all entries still justified (owner/reason current)?
