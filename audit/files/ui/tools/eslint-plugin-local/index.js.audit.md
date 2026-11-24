# AUDIT: ui/tools/eslint-plugin-local/index.js
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 30 / 1 default export (plugin manifest) |
| File verdict | REAL — thin, correct plugin manifest; registered and rule-tested |
| Reachable from shipped entry point? | NO (dev tooling: eslint.config.js:14 imports it; never in ui/dist) — reachable from the LINT lane |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | ls tools/eslint-plugin-local + rules/ (exit 0 — three rules + three test files exist); rg `eslint-plugin-local` eslint.config.js (exit 0 — :14 imports and registers) |

## 1. One-paragraph verdict
Exactly what it claims: a manifest re-exporting the three promoted rules (boundary-imports, i18n-no-literals, semantic-tokens-only). Verified the rule modules AND their co-located tests exist (ls), the plugin is registered in eslint.config.js:14 under the `local` key, and the header's claims match the tree. The three rules are the enforcement half of several claims I audited elsewhere (semantic-tokens-only backs every CSS audit's "tokens only" claim; i18n-no-literals backs the no-raw-copy claims). Zero findings: checked registration, rule-name addressing (`local/<name>`), test wiring (vitest include per header — plausible, not re-run: UNVERIFIED).

## 2. What actually works (verified)
— verified by ls + rg: all three rule files + tests present; registration at eslint.config.js:14.

## 3. Findings
None. Checked: exports match the documented rule names; no dead rules (all three registered via the manifest consumers); no path/escaping hazards (ESM imports).

## 4. Per-function table
| Item | Lines | Verdict | Notes |
|---|---|---|---|
| default export (rules map) | 23-29 | REAL | 3 rules, addressed local/<name> |

## 5. Test honesty
Each rule has a co-located .test.js (ls verified). Baseline `npm test` (vitest 1705 passed) includes them per the header's include-glob claim. UNVERIFIED: I did not run vitest (npm budget).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ui/eslint.config.js imports this module and registers it under the `local` plugin key" (L5-7) | header | REAL | eslint.config.js:14 (rg) |
| "Unit tests live next to each rule ... wired into npm run test" (L16-17) | header | REAL-UNPROVEN | files exist (ls); vitest wiring not re-run |

## 7. Dead / duplicated / unreachable code
None (this file IS the deduplication: rules promoted from inline config).

## 8. Dependencies & flags
Three sibling rule modules. Dev-only; not bundled.

## 9. Open questions for the next agent
- rules/ owners (not my assignment): i18n-no-literals passes while six "VLC" strings ship — is the identity leak in that rule's scope or does it need a fourth rule (identity-gate copy check)?
