# AUDIT: ui/src/components/menu/MenuItem.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A07) |
| Date | 2026-11-24 |
| Lines / functions | 66 / 4 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — vitest baseline ui-test.txt:154 (4 tests, 127ms); subject rendered by MenuPopup.tsx:329 (the live popup used by PlayerScreen/PlaylistPanel/ControlBar) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | it-count 4 == baseline; consumer rg (MenuPopup.tsx:3,329) |

## 1. One-paragraph verdict
Small, honest, presentational: pins the W26 figma grammar slots — decorative leading icon
(aria-hidden, label stays the accessible name), icon-accent modifier class, check slot
retained for both checkable and plain icon-less rows (with a negative assertion: no icon slot
in the plain case), accelerator text + submenu chevron + aria-haspopup/expanded. The header
explicitly fences the scope ("keyboard behavior is owned by MenuPopup … these tests pin the
new visual structure only") — no scope inflation. All 4 pass in the baseline; subject is live
(rendered inside MenuPopup).

## 2. What actually works (verified)
— verified by citation audit/runs/baseline/ui-test.txt:154 `✓ src/components/menu/MenuItem.test.tsx
(4 tests) 127ms` (1705-passed run); 4 it() blocks on disk match.

## 3. Findings
Zero numbered findings. Checked, and how: no tautologies (each test asserts roles, aria
attributes, class presence/absence, text content of the rendered component — e.g. :49's
negative `querySelector(".lumen-menu__icon")).toBeNull()`); no mock of the subject; scope
fence is truthful (keyboard coverage genuinely lives in MenuBar/ContextMenu suites, verified
by reading their keydown tests); no dead fixtures (both dict entries consumed).

## 4. Per-function table
| Test (line) | Verdict | Severity | Notes |
|---|---|---|---|
| leading decorative icon (18) | REAL | — | aria-hidden pinned |
| icon-accent flag (28) | REAL | — | modifier class |
| check slot kept / no icon slot plain (41) | REAL | — | positive + negative assertions |
| accelerator + chevron (52) | REAL | — | haspopup/expanded state |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 4 | YES | slot removal/class rename/aria regression all fail the queries |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Keyboard behavior is owned by MenuPopup … visual structure only" | header 10-11 | REAL | no keydown tests here; MenuBar/ContextMenu suites carry them |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
In-repo (MenuItem, MenuEntryBase). No timers, no mocks.

## 9. Open questions for the next agent
The `active` prop is passed in every render here — the inactive styling branch (active=false)
is untested; MenuPopup's suite presumably covers it. Confirm before relying on it.
