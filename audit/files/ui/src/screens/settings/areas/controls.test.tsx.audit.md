# AUDIT: ui/src/screens/settings/areas/controls.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A10) |
| Date | 2026-06-14 |
| Lines / functions | 158 / 2 helpers + 3 describe blocks (8 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — baseline ui-test.txt:96 `✓ src/screens/settings/areas/controls.test.tsx (8 tests) 659ms`; subject ControlsArea ships on /settings/controls and the retired /settings/hotkeys redirect |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `grep -n controls.test audit/runs/baseline/ui-test.txt` -> :96; `ls` hotkeys/ + generated/ (overrides.ts, bindings exist); read-through of all 8 tests |

## 1. One-paragraph verdict
Solid area suite: routing + single-area-root regression pin, the retired hotkeys slug redirecting into the fold host, the generated controls screen bound row-by-row to real schema keys (with the no-feature-gate invariant asserted :72), the hotkeys editor dialog lifecycle with REAL keymap data and REAL localStorage round-trip (save override -> visible value -> Reset -> default restored + store cleared, :119-131), and the staged Remotes rows pinned as honestly-inert (disabled, OFF, empty port, tooltip reason, zero live `controls.remote*` bindings). One trivial-assertion defect: a `queryByRole("note")` absence check.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:96 (8/8 pass; lane 1705 passed :178-179). Mutation reasoning: a duplicated area root fails :51; a schema row not rendering fails :73-74; a broken reset fails :129-130 (store cleared asserted); a live-but-dead remote toggle fails :140/:156.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/settings/areas/controls.test.tsx:153
Code:
```
    expect(screen.queryByRole("note")).toBeNull();
```
What is wrong: "note" is not an implicit ARIA role any element carries — the query can only ever fail if someone literally writes `role="note"`; it does not pin the intended property (no visible 11px note text next to the rows). Why it matters: the assertion reads like coverage of the h11 #3 note-removal but proves almost nothing; a `<p>` note would pass. What correct looks like: assert absence of the note's actual text (e.g. `queryByText(/control server/i)` scoped outside the title attribute). Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| renderRouted / mountArea | 18-36 | REAL | — | real SettingsScreen route mount + provider-only mount |
| routing + sibling contract suite | 42-77 | REAL | — | routed area, one-root pin, hotkeys redirect, schema binding loop |
| Keyboard Shortcuts suite | 79-132 | REAL | — | fold expansion, dialog modal semantics, real bindings, localStorage round-trip |
| honest-surface suite | 134-158 | REAL | S4 | staged inert rows + tooltip reason; F-001 weak query |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| schema binding :66-76 | YES | loops every controls entry against DOM |
| reset round-trip :119-131 | YES | persisted value read from input, then cleared store asserted |
| staged remotes :135-157 | YES | disabled+OFF+empty+title+zero bindings pinned (except the weak :153 probe) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 8 tests run | baseline ui-test.txt:96 | REAL | `✓ ... (8 tests) 659ms` |

## 7. Dead / duplicated / unreachable code
None. beforeEach clears localStorage (:38-40) — real hygiene for the persistence tests.

## 8. Dependencies & flags
HOTKEY_BINDINGS/generated/bindings + overrides module exist (ls verified).

## 9. Open questions for the next agent
None.
