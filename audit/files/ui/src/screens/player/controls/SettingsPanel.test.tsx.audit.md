# AUDIT: ui/src/screens/player/controls/SettingsPanel.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A10) |
| Date | 2026-06-14 |
| Lines / functions | 106 / 1 Harness + 1 describe block (4 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — baseline ui-test.txt:132 `✓ src/screens/player/controls/SettingsPanel.test.tsx (4 tests) 210ms`; subject SettingsPanel.tsx (401 L) ships in the player ControlBar panel popover |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `grep -n SettingsPanel.test audit/runs/baseline/ui-test.txt` -> :132; `wc -l` SettingsPanel.tsx -> 401 L; read-through of all 4 tests |

## 1. One-paragraph verdict
Direct keyboard suite filling a real documented gap (header: the panel's keyboard interaction previously had zero direct coverage). The Harness reproduces the host's real wiring (controlled activeTab), and the tests drive genuine key paths: focus-on-open (panel takes focus, tabindex -1), Tab/Shift+Tab wrap within the trap (footer is last focusable — forward and backward both asserted), ArrowRight/ArrowLeft tab switching with focus-follows-selection AND aria-labelledby id rewiring, ring wrap in both directions, and roving tabindex (only the active tab is a Tab stop). No assertions on mocks; the panel is presentational so no providers are needed — that's honest scope, not shortcut.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:132 (4/4 pass; lane 1705 passed :178-179). Mutation reasoning: a broken focus trap fails :59-64; a focus-not-following-selection arrow handler fails :76/:94-97; a stale tabpanel label fails :78-81; a missing roving tabindex fails :102-104; focus not landing on open fails :43.

## 3. Findings
None. Checked for: tautologies (none — real focus movement asserted via document.activeElement), subject mocks (none — the Harness wraps the real panel with real state), skipped tests (none — 4/4 run), narrow coverage claims contradicting body (none — the header's claimed 45 lines of interaction map to the four tested behaviors: focus-on-open, trap, arrow tablist, roving tabindex). Zero findings is the honest result.

## 4. Per-function table (file >100 LOC)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| TABS / Harness | 14-37 | REAL | — | controlled activeTab like PanelPopover; footer + body buttons as focusables |
| focus-on-open test | 40-47 | REAL | — | panel focused at mount, tabindex -1 |
| trap wrap test | 49-65 | REAL | — | forward + backward wrap with real key events |
| arrow tablist test | 67-98 | REAL | — | both directions, focus-follows-selection, id rewiring, ring wrap |
| roving tabindex test | 100-105 | REAL | — | active tab 0, others -1 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| trap wrap :49-65 | YES | activeElement tracked through both wraps |
| arrows :67-98 | YES | aria-selected + focus + label wiring all checked |
| roving :100-105 | YES | exact tabindex per tab |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 4 tests run | baseline ui-test.txt:132 | REAL | `✓ ... (4 tests) 210ms` |

## 7. Dead / duplicated / unreachable code
None. onRequestClose stubbed to a no-op — the one seam this suite deliberately does not cover (ControlBar suite owns the Escape-restore pin per header :4-5).

## 8. Dependencies & flags
Imports resolve (SettingsPanel + SettingsPanelTab type from the real component).

## 9. Open questions for the next agent
None.
