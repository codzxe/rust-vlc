# AUDIT: ui/src/components/menu/ContextMenu.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A10) |
| Date | 2026-11-24 |
| Lines / functions | 119 / 7 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — vitest baseline ui-test.txt:125 (7 tests, 231ms); subject mounted in PlaylistScreen/PlaylistPanel/DiscoveryScreen/playlist options MediaList |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | it-count 7 == baseline; consumer rg; sed quote verify |

## 1. One-paragraph verdict
Real behavioral suite for a LIVE component (unlike its dead MenuBar/Tooltip siblings): opens at
pointer coordinates (left/top pinned to clientX/Y), suppresses the native browser menu
(preventDefault observed via jsdom's dispatch-false), Escape with focus restoration to the
previously-focused element, arrow navigation across the full fixture (including a disabled-but-
focusable row), Enter activation + dismissal, outside-pointerDown dismissal, density class.
7/7 pass in the baseline. One small gap: the disabled item is navigated through but never
Enter-attacked to prove inertness.

## 2. What actually works (verified)
— verified by citation audit/runs/baseline/ui-test.txt:125 `✓ src/components/menu/
ContextMenu.test.tsx (7 tests) 231ms` (1705-passed run); 7 it() blocks on disk match.

## 3. Findings
F-001 [S4]
Where: ui/src/components/menu/ContextMenu.test.tsx:24,79-81 (fixture + navigation test)
Code:
  { id: "info", labelKey: "ctx.info", disabled: true },
...
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "Play" }), { key: "ArrowDown" });
    expect(screen.getByRole("menuitem", { name: "Media Information" })).toHaveFocus();
What is wrong: the disabled entry is exercised only as a waypoint — no test presses Enter on
the disabled "Media Information" to assert onSelect never fires. MenuBar.test.tsx:239-248 DOES
prove this for the sibling; ContextMenu's contract for disabled items is unguarded.
Why it matters: a regression making disabled rows activatable would pass this suite.
What correct looks like: mirror MenuBar's disabled-inertness test here.
Confidence: HIGH — Verification: read.
No S0-S3: every assertion binds to coordinates, roles, focus, call counts or classes produced
by the component.

## 4. Per-function table
| Test (line) | Verdict | Severity | Notes |
|---|---|---|---|
| opens at pointer coords (29) | REAL | — | style.left/top pinned |
| suppresses native menu (44) | REAL | — | preventDefault via dispatch false |
| Escape restores focus (56) | REAL | — | |
| arrows navigate incl. disabled row (71) | REAL | S4 | F-001 gap on the disabled row |
| Enter activates + dismisses (84) | REAL | — | onRemove once |
| outside pointerDown dismisses (99) | REAL | — | |
| density comfortable (110) | REAL | — | |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 7 | YES | coordinates/focus/call-counts are component-output-bound; e.g. drop preventDefault → :51-53 fails |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "opens at right-click coordinates" | header 7 | REAL | :39-40 |
| "Escape restores focus to the previously-focused element" | header 8-9 | REAL | :66-68 |

## 7. Dead / duplicated / unreachable code
None. Subject live (mounted by 4 screens).

## 8. Dependencies & flags
In-repo (ContextMenu, MenuEntry). No mocks of the subject.

## 9. Open questions for the next agent
Positioning here pins style.left/top but not clamping to viewport (menu at clientX 120 in a
1280-wide jsdom body — no overflow case tested; MenuPopup's flip is the live guard elsewhere).
