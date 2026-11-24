# AUDIT: ui/tools/eslint-plugin-local/rules/i18n-no-literals.test.js
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 99 / 2 tests in 1 describe (RuleTester) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as lint-infrastructure for the shipped UI tree (rule enforced in CI lint-literals lane); runs under vitest |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -n "i18n-no-literals.test" audit/runs/baseline/ui-test.txt` → `146: ✓ tools/eslint-plugin-local/rules/i18n-no-literals.test.js (2 tests) 80ms`; full file read |

## 1. One-paragraph verdict
Real RuleTester coverage of the local i18n lint rule: valid cases cover the useMsg() seam, file-SCOPED allowlist exemption (same text allowed in CustomizePanels.tsx, rejected in PlaylistScreen.tsx — the scoping contract pinned from both sides), whitespace layout nodes, expression containers, resolved attributes, the data-placeholder marker exemption, and test-file exemption; invalid cases pin text nodes, attribute literals, template-literal placeholders (the live j20 regression), string-literal expression containers, and mis-scoped allowlist text. The rule module loads the REAL ui/i18n-allowlist.json, so the allowlist contract is exercised too.

## 2. What actually works (verified)
- Committed run: baseline ui-test.txt L146 `✓ ... (2 tests) 80ms` (inside 1705 passed).
- The scoping contract is tested BOTH directions (L35-38 valid with filename, L80-85 invalid with a different filename, same text) — the strongest form of the pin.

## 3. Findings
None. Zero-findings checklist: tautology scan — RuleTester asserts real rule verdicts on constructed code; the plugin-index registration test (L24-26) binds to the real plugin export. Mock-of-subject scan — none; the real rule + real allowlist are loaded. Skip scan — no it.skip; baseline 2 passed. Name-vs-body scan — verified. Mutation reasoning: dropping file-scoping makes L80-85 fail (the case becomes valid); dropping placeholder coverage makes L70-78 fail; over-blocking whitespace breaks L40.

## 4. Per-function table (>100 LOC not exceeded — 99 lines; still listed)
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| plugin registration | 24-26 | REAL | — | rule exported under historical name |
| valid/invalid matrix | 28-98 | REAL | — | 8 valid + 7 invalid incl. both-sides scoping pin |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| both | YES | RuleTester invalid-cases MUST report and valid-cases MUST NOT; verified green baseline L146 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the REAL ui/i18n-allowlist.json (loaded by the rule module itself), so the allowlist contract is covered too" | header L2-3 | REAL-UNPROVEN | rule impl not my file; the file-scoped cases only pass if the real allowlist carries the "00:02:10" entry scoped to CustomizePanels.tsx |
| "the '00:02:10' customize-band timecodes are the live fixture for that contract" | header L6-8 | REAL | used at L36-38 and L82-84 |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
eslint RuleTester, vitest, real plugin index + rule + allowlist.

## 9. Open questions for the next agent
- None.
