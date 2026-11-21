# AUDIT: ui/src/screens/settings/areas/extensions.css (quick check)
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 90 / 6 rule blocks (list well, toolbar search well, light sweep ×2, toolbar flex, actions cluster, placeholder ×2, find-action width) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (imported by areas/extensions.tsx:49; the Extensions area renders at /settings/extensions via areas/index.tsx registry; extensions.test.tsx regex-pins three of its rules). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Grep class selectors → all four (.settings-extensions-list :156, -toolbar :175, -toolbar__actions :184, -toolbar__find :192) used by extensions.tsx; Grep token definitions in tokens/generated/tokens.css → all six consumed tokens exist in both mode blocks (--color-surface #090909/:56+#131 dark, --color-surface-panel #191919, --color-surface-deep #0e0e0e dark / #bababa light :132, --color-content-secondary, --color-stroke-border #404040, --font-size-section 15px :72); read all 90 lines; read extensions.test.tsx:132-139 (regex pins for the placeholder token, light sweep, and surface-deep light rule). Baseline: ui build #22 OK, ui-vitest 1705 passed. |

## 1. One-paragraph verdict
Token-clean, both-theme-aware, fully-consumed area skin: every color rides a semantic token that exists in the generated sheet (dark and light blocks verified), raw px values are frame-measured and marked `frame:`, the light recessed-well sweep uses --color-surface-deep exactly as the token sheet documents it, and the placeholder grammar unifies with the topbar search (token + light sweep). No hardcoded hex outside comments, no unused selectors, no specificity hazards beyond the two documented deliberate bumps (search well one class deeper than form.css; find-action doubled selector to beat .settings-action's min-width).

## 2. What actually works (verified)
— verified by Grep: 4/4 class selectors consumed by extensions.tsx; 6/6 tokens defined in tokens/generated/tokens.css (dark :root + light sweep where applicable).
— verified by extensions.test.tsx:132-139: three of the rules (placeholder --color-stroke-border, light placeholder --color-content-secondary, light --color-surface-deep sweep) are regex-pinned so token drift fails CI.

## 3. Findings
None. Checked and clean: selector usage (4/4), token existence + light-mode coverage (6/6), no raw colors outside comments, no duplicate declarations, declaration order satisfies the documented stylelint constraint (search well :43-51 before light sweep :53-56), box-sizing on the bordered well, min-height (not fixed height) for the list well — a documented growth affordance, not a frame violation.

## 4. Per-function table
| Rule block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.settings-extensions-list` | 27-34 | REAL | — | Frame well: 192px min-height, 2px panel stroke, rx5, token surfaces. |
| `.settings-extensions-toolbar .lumen-field__control` | 43-51 | REAL | — | Search well re-skin; specificity bump documented. |
| light sweep (list + search) | 53-56 | REAL | — | --color-surface-deep recessed wells; test-pinned. |
| `.settings-extensions-toolbar` / `__actions` | 60-72 | REAL | — | Flex + 10px frame gaps + margin-left auto right-align. |
| placeholder rules | 76-82 | REAL | — | Topbar token + light sweep; test-pinned. |
| `__find` min-width | 87-89 | REAL | — | 185px frame width; doubled selector beats .settings-action. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| extensions.test.tsx:132-139 (3 regex pins on this file's rules) | YES | Source-text regex assertions on the exact token usage. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Colors map to semantic tokens (#090909 → --color-surface, #191919 → --color-surface-panel, #8F8F8F → --color-content-secondary)" | extensions.css:10-11 | REAL | tokens.css values match exactly (read). |
| "light … rides the token sheet's own recessed-wells step, --color-surface-deep (#BABABA)" | extensions.css:16-18 | REAL | tokens.css:132 `--color-surface-deep: #bababa` in the light block. |
| "the placeholder rides --color-stroke-border (dark #404040 …) + the topbar's light sweep" | extensions.css:21-23 | REAL | tokens.css dark #404040 / light sweep via --color-content-secondary; test-pinned. |

## 7. Dead / duplicated / unreachable code
None — every selector consumed; no duplicated rules.

## 8. Dependencies & flags
Semantic tokens only (6), the shared .lumen-field__control / .settings-action grammars (form.css / settings skins), no flags.

## 9. Open questions for the next agent
None.
