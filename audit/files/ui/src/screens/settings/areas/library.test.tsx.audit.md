# AUDIT: ui/src/screens/settings/areas/library.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | 135 / 6 it() tests, 2 describe blocks |
| File verdict | REAL (6/6 pass in baseline; pins the area's honest inertness) |
| Reachable from shipped entry point? | YES via test lane — baseline ui-test.txt line 108 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full file read; baseline cite |

## 1. One-paragraph verdict
Honest test of a STAGED settings area: it pins that the 7 §C9 frame rows render visible-but-
DISABLED with "Staged" tooltips + data-staged="schema-seed" markers, that ZERO live
data-setting rows exist ("nothing pretends to be wired"), that the routed area mounts under
the registry, and a schema-truth assertion (no library [[setting]] rows seeded -> no
GENERATED_SCREENS.library). This is the anti-fakeness pattern done right: the test proves
the UI discloses its own unwired state.

## 2. What actually works (verified)
- Baseline: ui-test.txt line 108 `✓ src/screens/settings/areas/library.test.tsx (6 tests) 294ms`.
- Read-verified strong assertions: `expect(rows).toHaveLength(7);` with per-row
  `data-staged="schema-seed"` + title contains "Staged";
  `expect(container.querySelectorAll("[data-setting], [data-setting-capability]")).toHaveLength(0);`
  (nothing wired); `expect(screen.getByRole("switch", { name: "Only allow one VLC window" })).toBeDisabled();`.
- The ⓘ reveal interaction is asserted both ways (click -> hint appears with the staged
  reason, click again -> gone).

## 3. Findings
F-001 [S4]
Where: line 118 `fireEvent.click(dots[0]!);` (and the `fold!` at line 47)
What is wrong: `!` non-null assertions — a missing element throws an opaque TypeError
instead of a clean failure. Cosmetic. Confidence: HIGH — read.

## 4. Per-function table
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| renderRouted / mountArea | 24-37 | REAL | routed + bare-provider mounts |
| describe routing + b02 sibling contract | 39-79 | REAL | fold containment; schema-truth data pin |
| describe honest surface (§C9 rows) | 81-135 | REAL | staged inertness census (7/7), dim markers |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| renders the §C9 frame rows STAGED inert | YES | 7-row census + disabled + markers |
| binds zero live settings rows | YES | empty selector set |
| schema truth: no library rows seeded | YES | generated-schema data equality |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 6 tests pass | baseline ui-test.txt:108 | REAL | vitest line |
| "the honest-surface rule for the §C9 frame rows" (header line 5) | file itself | REAL | disabled+marker assertions |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
Uses GENERATED_SCREENS + SETTINGS_ENTRIES as oracles (legit: they are the generated truth).

## 9. Open questions
- The playlist-services fold is owned by another lane and "never edited here" — when that
  lane seeds rows, the fold-emptiness assertion flips. Documented in-file; fine.
