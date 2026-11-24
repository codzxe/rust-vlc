# AUDIT: ui/src/screens/player/customize/index.ts
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 43 / 1 barrel (24 re-exports) |
| File verdict | REAL (over-exported barrel: 1 of 24 exports externally consumed) |
| Reachable from shipped entry point? | YES (PlayerScreen.tsx:109 imports CustomizePanels from this barrel — the only external import site) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | grep barrel import sites (exactly 1 external); per-export external usage scan |

## 1. One-paragraph verdict
Barrel for the customize surface. Hostile finding: of the 24 re-exported
symbols, exactly ONE (CustomizePanels) is imported from outside the customize
directory (PlayerScreen.tsx:109). The rest are consumed only internally via
direct module imports — the barrel re-exports exist as dead public API. One
export, bandTiles, is consumed by NOTHING at all (defined
toolbarLayout.ts:102, re-exported index.ts:28, zero call sites).

## 2. What actually works (verified)
— grep for barrel-path imports outside the customize dir: PlayerScreen.tsx
  only. The module itself is reachable and load-bearing for the customize modal.
— Internal cross-imports bypass the barrel (direct ./cycleState, ./panels
  paths) — no cycle risk.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/customize/index.ts:28 (bandTiles)
Code: `  bandTiles,`
What is wrong: bandTiles is dead — defined (toolbarLayout.ts:102), re-exported
here, and called nowhere in the tree (rg over all src: definition + barrel
only). Ships as unused public API of the customize surface.
What correct looks like: delete the function or consume it where the band
preview was meant to render.
Confidence: HIGH — Verification: rg bandTiles across src.
F-002 [S4]
Where: ui/src/screens/player/customize/index.ts:5-43 (whole barrel)
What is wrong: 23 of 24 re-exports have no external consumer (applyCycle*/
CYCLE_PAYLOADS/deepEqual/toolbar helpers/etc. are internal-only). Over-wide
public surface: every internal refactor churns this file and invites
accidental external coupling.
Confidence: HIGH — Verification: grep import sites + per-export scan.

## 4. Per-function table
Not applicable (<100 LOC barrel; entries are re-exports).

## 5. Test honesty
No tests in file.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the retired gating helpers (tileVisible/visibleTiles) left the barrel" | :3-4 | REAL | neither symbol appears in the barrel or tree |

## 7. Dead / duplicated / unreachable code
bandTiles (F-001) — fully dead. 23 barrel re-exports externally unused
(F-002), internally alive.

## 8. Dependencies & flags
Re-exports only: CustomizePanels, toolbarItems, capabilities, cycleState,
panels, toolbarLayout, types. No flags.

## 9. Open questions for the next agent
toolbarLayout.ts owner (not my file): was bandTiles the toolbar-band preview's
helper whose consumer was deleted in a redesign?
