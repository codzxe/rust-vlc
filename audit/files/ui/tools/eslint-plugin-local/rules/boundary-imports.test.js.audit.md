# AUDIT: ui/tools/eslint-plugin-local/rules/boundary-imports.test.js
| Field | Value |
|---|---|
| Auditor | W16-A04 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / tests | 60 / 1 (RuleTester case set: 4 valid + 2 invalid) |
| File verdict | REAL (baseline-verified green; the rule is wired as "error" in the shipped eslint config) |
| Reachable from shipped entry point? | N/A — lint tooling; consumer = ui/eslint.config.js:109 `"local/boundary-imports": "error"` (rg verified) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg baseline ui-test.txt:166; rg eslint wiring |

## 1. Verdict
RuleTester coverage of the boundary rule with real absolute filenames under ui/src (so
relative-specifier resolution runs against the real tree): in-boundary relative import,
bare specifier, deep climb that stays inside, up-and-back-in normalization; invalid cases:
escape outside ui/src (messageId outsideSrc) and absolute-path import (messageId absolute).
Baseline green: ui-test.txt:166 `✓ …boundary-imports.test.js (1 test) 67ms`.

## 2. What actually works (verified)
- Baseline 1/1 (cite above).
- Mutation reasoning: boundary math regression fails the valid case 3/4 or the invalid
  outsideSrc; absolute-path acceptance fails messageId "absolute"; overreach (rejecting bare
  'react') fails valid case 2.
- The rule is enforced at "error" level in the real config — this is not a dead local rule.

## 3. Findings
F-001 [S4]
Where: ui/tools/eslint-plugin-local/rules/boundary-imports.test.js:44-57
Code:
      invalid: [
        // Relative import escaping ui/src (…/themes is Rust-owned territory).
        {
          code: "import { t } from '../../themes/tokens';",
What is wrong: no invalid case for a URL/protocol specifier (e.g. 'https://…' or 'data:'),
and no case for Windows-style absolute paths — the rule's refusal vocabulary is pinned only
for POSIX absolute + relative escape. If the rule mishandles url: or windows forms, nothing
in this suite notices.
Confidence: MEDIUM — Verification: read (case list exhausted above).
Otherwise checked: the test's own import resolution (uiSrc computed from import.meta.url —
real path math), RuleTester API usage correct, sibling rules' tests exist and pass
(:144/:146), wiring at eslint.config.js:109.

## 4. Test honesty
| Case | Would it fail if rule were broken? | Reason |
|---|---|---|
| 4 valid | YES | RuleTester errors on any produced report |
| escape → outsideSrc | YES | messageId pinned |
| absolute → "absolute" | YES | messageId pinned |

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "resolves relative specifiers against context.filename and compares with the REAL ui/src root" | :1-3 | REAL | valid case 3/4 depend on that resolution |

## 6. Dead / duplicated / unreachable code
None.

## 7. Dependencies & flags
eslint RuleTester under vitest; no mocks.

## 8. Open questions for the next agent
None blocking.
