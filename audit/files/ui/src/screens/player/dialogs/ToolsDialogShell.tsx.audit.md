# AUDIT: ui/src/screens/player/dialogs/ToolsDialogShell.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 254 / 4 (ToolsDialogShell, ToolsCard, ToolsRow, ToolsValueChip) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — ToolsDialog.tsx, tools/visualizer/VisualizerTool.tsx, tools/video + tools/dynamics barrels mount the shell/cards (rg consumers) -> player screen |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg SurfaceCloseReason union; rg consumers; rg dialogs.css order:-1; read ToolsDialogShell.test.tsx case list |

## 1. One-paragraph verdict
Generic, presentational dialog shell (chrome + collapsible card + row/chip
primitives) with genuinely thoughtful a11y: body-before-header DOM order with
CSS `order: -1` repainting (verified dialogs.css:77), aria-controls on hidden
bodies so the label resolves while collapsed, focus-trap delegation to the
shared Dialog. No backend surface by design (slots). One semantic mislabel:
the ✕ button reports close reason "escape".

## 2. What actually works (verified)
- Focus-order mechanism: dialogs.css:77 `order: -1` paints the header first
  while the body leads the DOM — matches the module header's claim (18-25).
- Tests drive real behavior: Escape closes, scrim click closes, ✕ closes,
  collapsed-by-default + expansion, defaultOpen, row slots
  (ToolsDialogShell.test.tsx:30-98).
- Consumers exist in three tool lanes + ToolsDialog (rg).

## 3. Findings
F-001 [S4]
Where: ToolsDialogShell.tsx:104-112
Code:
```
          <IconButton
            className="player-dialog__close"
            label={m("player.tools.close")}
            title={m("player.tools.close")}
            size="sm"
            onClick={() => onClose("escape")}
          >
```
What is wrong: the ✕ click reports reason "escape" — the union
`SurfaceCloseReason = "escape" | "outside"` (Surface.tsx:9) has no
"close-button" member, so a deliberate button click is indistinguishable
from a keyboard Escape for every consumer. Cosmetic today (no consumer found
branching on the reason), but the type is a lie about provenance.
What correct looks like: extend the union with "close-button".
Confidence: HIGH — Verification: read + rg.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ToolsDialogShell | 72-122 | REAL | S4 | see F-001 |
| ToolsCard | 147-193 | REAL | - | collapsed-by-default per frames; hidden-not-unmounted aria contract |
| ToolsRow | 212-230 | REAL | - | label/hint/control slots, htmlFor pass-through |
| ToolsValueChip | 245-254 | REAL | - | title tooltip for clipped values |

## 5. Test honesty
ToolsDialogShell.test.tsx (8+ cases): interaction-level (clicks, keys), would
fail if the Dialog wiring, focus trap or aria attributes regressed. Honest.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the surface trap's first-focusable policy lands on the first BODY control ... never on the ✕" | ToolsDialogShell.tsx:18-25 | REAL-UNPROVEN | DOM order + CSS order verified in code; actual focus landing not executed (jsdom focus trap test lives in Surface's own tests — not read) |
| "The ✕ stays keyboard reachable at the END of the tab cycle" | ToolsDialogShell.tsx:23-25 | REAL-UNPROVEN | follows from DOM order; not executed here |

## 7. Dead / duplicated / unreachable code
None. All four exports consumed (rg: ToolsDialog, visualizer, video, dynamics lanes + index barrels).

## 8. Dependencies & flags
IconButton, Dialog/Surface, i18n, dialogIcons, dialogs.css. No gates.

## 9. Open questions for the next agent
- Whether any onClose consumer differentiates "escape" vs "outside" today
  (if none does, F-001 stays cosmetic; if one does, it miscounts ✕ clicks).
