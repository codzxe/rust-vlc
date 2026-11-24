# AUDIT: ui/src/components/button/IconButton.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A03) |
| Date | 2026-06-14 |
| Lines / functions | 144 / 2 helpers + 2 describe blocks (12 tests incl. 1 snapshot) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — baseline ui-test.txt:117 `✓ src/components/button/IconButton.test.tsx (12 tests) 132ms`; subject IconButton.tsx (58 L) ships across the player chrome/toolbars |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `grep -n IconButton.test audit/runs/baseline/ui-test.txt` -> :117; `wc -l` IconButton.tsx -> 58 L; snapshot file present (`ls __snapshots__`) |

## 1. One-paragraph verdict
Behavioral suite with honest a11y/behavior pins: accessible name from label, class composition (base + size modifier + icon-host protocol + consumer extra), click firing, disabled truthfulness (state + no handler), loading inertness (aria-busy + disabled + no handler), DOM snapshot; then six CSS-token pins for the W26 skin incl. the j01-12 loading-spinner regression (14px currentcolor ring, glyph visibility-hidden not display:none, reduced-motion gate shared with Button). One hygiene defect: the skin pins are raw-regex over button.css.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:117 (12/12 pass; lane 1705 passed :178-179). Mutation reasoning: a lost accessible name fails :28; a firing disabled handler fails :63; a loading button that stays clickable fails :77; a removed spinner rule fails :133-135; a broken reduced-motion extension fails :140-142.

## 3. Findings
F-001 [S4]
Where: ui/src/components/button/IconButton.test.tsx:10-13
Code:
```
const cssRule = (selector: string): string => {
  const match = buttonCss.match(new RegExp(`${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}[^{]*\\{([^}]*)\\}`));
  return match?.[1] ?? "";
};
```
What is wrong: `[^{]*` allows anything between the selector and `{` — a `.lumen-icon-button` query can silently match a LONGER selector that merely starts with it (e.g. `.lumen-icon-button--sm` has its own rule; today the first-occurrence order happens to be right, but a reordered stylesheet changes which rule the helper resolves while the test keeps passing on the wrong body). Also returns "" on no-match, so a selector typo degrades to empty-rule assertions that fail with unclear cause. Why it matters: pins whose target is order-dependent are not really pinned. What correct looks like: match the full selector list (`selector\s*\{`) or parse rules. Confidence: MEDIUM — Verification: read (current pass per baseline; the ambiguity is structural).

## 4. Per-function table (file >100 LOC)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| cssRule helper | 10-13 | REAL | S4 | F-001 first-occurrence ambiguity |
| Glyph helper | 15-19 | REAL | — | decorative svg child |
| behavior suite | 21-88 | REAL | — | 6 tests incl. disabled/loading inertness |
| skin suite | 90-144 | REAL | — | 6 token pins; j01-12 spinner regression guarded |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| accessible name :22-29 | YES | role+name query |
| disabled/loading inert :53-78 | YES | spy + disabled semantics |
| spinner pin :128-143 | YES | rule content + reduced-motion regex |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 12 tests run | baseline ui-test.txt:117 | REAL | `✓ ... (12 tests) 132ms` |

## 7. Dead / duplicated / unreachable code
None. Snapshot exercised (:80-87).

## 8. Dependencies & flags
Imports resolve (IconButton 58 L); snapshot file exists.

## 9. Open questions for the next agent
None.
