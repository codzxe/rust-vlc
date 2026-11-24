# AUDIT: ui/src/components/form/TextField.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A10) |
| Date | 2026-06-14 |
| Lines / functions | 75 / 1 css read + 2 describe blocks (8 tests incl. 1 snapshot) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — baseline ui-test.txt:148 `✓ src/components/form/TextField.test.tsx (8 tests) 131ms`; subject TextField.tsx (55 L) ships across settings/open-network-stream forms |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `grep -n TextField.test audit/runs/baseline/ui-test.txt` -> :148; `wc -l` subject -> 55 L; read-through of all tests |

## 1. One-paragraph verdict
Compact honest suite: label wiring, change firing, error state with real aria-describedby id round-trip, NO-error default (absence pinned), a notably HONEST disabled test — it asserts the native disabled attribute and explicitly explains why it does not fake a fireEvent dispatch (fireEvent bypasses the browser gate; asserting the attribute is the real contract, :35-41), hideLabel keeping the accessible name with no text node, ref forwarding + snapshot; plus CSS token state pins for the control matrix.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:148 (8/8 pass; lane 1705 passed :178-179). Mutation reasoning: broken label association fails :14; describedBy pointing nowhere fails :26; an always-on aria-invalid fails :31; hideLabel losing the name fails :47-48.

## 3. Findings
F-001 [S4]
Where: ui/src/components/form/TextField.test.tsx:64-74
Code:
```
    expect(formCss).toMatch(/\.lumen-field__control:hover\s*\{[^}]*--color-surface-hover/);
```
What is wrong: raw-regex CSS pins over form.css (same brittleness family as Toggle/BrowseField/IconButton: order- and formatting-sensitive, blind to semantic drift that preserves text). Why it matters: five state pins here churn under innocuous CSS refactors. What correct looks like: computed-style assertions or a normalized rule parser. Confidence: HIGH — Verification: read.

## 4. Per-function table
File <100 LOC — 6 behavior tests (:11-56) + 2 css pins (:59-75). The disabled test (:35-41) and hideLabel test (:43-49) are the two anti-fabrication pins worth copying.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| error wiring :20-27 | YES | id equality both directions |
| no-error default :29-33 | YES | absence assertions |
| disabled :35-41 | YES | attribute contract, honestly reasoned (no fake dispatch) |
| hideLabel :43-49 | YES | role+name + no text node |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 8 tests run | baseline ui-test.txt:148 | REAL | `✓ ... (8 tests) 131ms` |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
form.css via import.meta.dirname (exercised green in baseline).

## 9. Open questions for the next agent
None.
