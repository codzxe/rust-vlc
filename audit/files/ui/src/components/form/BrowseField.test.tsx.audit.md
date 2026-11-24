# AUDIT: ui/src/components/form/BrowseField.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A01) |
| Date | 2026-06-14 |
| Lines / functions | 81 / 1 css read + 2 describe blocks (9 tests incl. 1 snapshot) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — baseline ui-test.txt:147 `✓ src/components/form/BrowseField.test.tsx (9 tests) 142ms`; subject BrowseField.tsx (68 L) ships in settings screens (skin/log path rows) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `grep -n BrowseField audit/runs/baseline/ui-test.txt` -> :147; `wc -l` subject -> 68 L; read-through of all tests |

## 1. One-paragraph verdict
Honest small component suite: label wiring via getByLabelText, change/click handlers fire, disabled truthfully disables BOTH input and action with no handler, error state with a real aria-describedby id round-trip, the j01-8 hideLabel mechanism pinned (aria-label + NO label text node — would catch a second text node regression), ref forwarding, snapshot; plus two CSS token pins for the action button's states and frame geometry. The only defect is the usual raw-regex CSS pin brittleness.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:147 (9/9 pass; lane 1705 passed :178-179). Mutation reasoning: a broken label association fails :14; a firing disabled action fails :34; a describedBy pointing nowhere fails :42; a hideLabel that also renders a text node fails :52.

## 3. Findings
F-001 [S4]
Where: ui/src/components/form/BrowseField.test.tsx:69-75
Code:
```
    expect(formCss).toMatch(
      /\.lumen-browse__action\s*\{[^}]*--color-content-secondary[^}]*--color-surface-panel[^}]*--radius-control/,
    );
```
What is wrong: property-order-coupled regex over raw CSS (`[^}]*--color-content-secondary[^}]*--color-surface-panel…` requires those custom properties in that textual order inside one rule). Why it matters: any property reorder or move-to-`:root` refactor breaks the pin without a semantic change; conversely semantic drift that preserves order passes. What correct looks like: parse rule bodies and assert a set of properties per selector, or computed styles. Confidence: HIGH — Verification: read (passing baseline proves current text matches; brittleness structural).

## 4. Per-function table
File <100 LOC — 7 behavior tests (:11-64) + 2 css pins (:67-81), all REAL; the j01-8 test (:48-53) is the notable anti-drift pin.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| disabled no-fire :27-35 | YES | both controls disabled + spy silent |
| error wiring :37-43 | YES | id equality both directions |
| hideLabel :48-53 | YES | absence of text node asserted |
| css pins :67-81 | YES (text drift only) | F-001 caveat |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 9 tests run | baseline ui-test.txt:147 | REAL | `✓ ... (9 tests) 142ms` |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
form.css read via import.meta.dirname (same pattern as Toggle/IconButton suites; exercised green in baseline).

## 9. Open questions for the next agent
None.
