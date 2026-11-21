# AUDIT: ui/src/screens/player/dialogs/ToolsDialogShell.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 254 / 4 components (+4 prop interfaces) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — ToolsDialog.tsx mounts the shell with all 9 tool cards (EQ, compressor, spatializer, widener, visualizer, colors, crop, essential, HDR); opened via the `lumen:open-tools` event / openToolsDialog from the player's ControlBar. W14 caveat: no shipped desktop binary (build-desktop-tauri.log.txt exit 101). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | rg 'SurfaceCloseReason' Surface.tsx (type = "escape" \| "outside" only); rg consumers (ToolsDialog.tsx, VisualizerTool.tsx compose ToolsCard/Row/ValueChip); rg i18n keys (player.tools.close/collapse-settings/expand-settings registered messages.ts:2174-2175); rg dialogs.css (order:-1 at 77; all tools-card/tools-row/tools-dialog classes present). |

## 1. One-paragraph verdict
A clean generic dialog shell: correct focus-order trick (body before header in DOM, `order: -1` repaint — verified present in dialogs.css:73-77), correct aria wiring (labelledBy on the title id, aria-expanded/aria-controls on the card toggle, hidden body stays in the DOM so the trap and aria references stay coherent), and an honest composition API. Two nits: the ✕ button reports close reason "escape" because the shared SurfaceCloseReason type has no third value, and ToolsRow renders a `<label>` that is a no-op whenever `htmlFor` is omitted and the control lives in the sibling slot. The heavy honesty debt of the tools dialog lives in the mounted tool cards (all local state; EQ explicitly staged on `audio.set-eq` — the W8 nonexistent backend), not in this shell.

## 2. What actually works (verified)
— dialogs.css:77 carries the `order: -1` header rule the module header's focus-order claim depends on; classes tools-card__toggle/chevron/body (388-435+), tools-dialog__cards/staged (315-378) all exist. i18n keys registered. Baseline: ToolsDialog.test.tsx passes (ui-vitest.log.txt); `tsc --noEmit` clean (ui-build.log.txt).

## 3. Findings
### F-001 [S4]
Where: ui/src/screens/player/dialogs/ToolsDialogShell.tsx:104-112
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
What is wrong: the ✕ click reports `onClose("escape")`. `SurfaceCloseReason` (Surface.tsx:9) has exactly two values — "escape" | "outside" — so a close-button click is indistinguishable from an Escape keypress at every consumer.
Why it matters: naming that lies at the API level — any consumer reacting differently to explicit dismissal vs. Escape (telemetry, "don't show again" flows) cannot; the type needs a "close" value (a Surface-level gap this shell inherits, not introduces).
What correct looks like: extend SurfaceCloseReason with "close" and pass it here.
Confidence: HIGH — type read.
### F-002 [S4]
Where: ui/src/screens/player/dialogs/ToolsDialogShell.tsx:220-227
Code:
```
      <div className="tools-row__label">
        <label className="tools-row__label-text" htmlFor={htmlFor}>
          {label}
        </label>
```
What is wrong: when `htmlFor` is omitted (it is optional, ToolsDialogShell.tsx:203) and the control sits in the sibling `tools-row__control` slot, the `<label>` wraps only text — clicking it focuses nothing (labels associate by nesting OR by for; neither holds). Every visualizer/dynamics row that forgets htmlFor ships a dead label.
Why it matters: a11y affordance that silently no-ops; the API documents the requirement but nothing enforces it.
What correct looks like: make htmlFor required when the control isn't nested, or render a `<span>` when undefined (a bare label with no target is invalid-ish HTML semantics).
Confidence: HIGH — read; the visualizer rows pass htmlFor (types/lane files), so no live defect found in current mounts.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ToolsDialogShellProps` (interface) | 45-66 | REAL | — | |
| `ToolsDialogShell` | 72-122 | REAL | S4 (F-001) | Dialog wrap; body-first DOM + subtitle/cards slots; footer optional |
| `ToolsCardProps` (interface) | 124-137 | REAL | — | |
| `ToolsCard` | 147-193 | REAL | — | uncontrolled open; hidden body keeps aria-controls resolvable; 24px chevron box documented (h14 P3) |
| `ToolsRowProps` (interface) | 195-206 | REAL | — | |
| `ToolsRow` | 212-230 | REAL | S4 (F-002) | label/hint left, control right |
| `ToolsValueChipProps` (interface) | 232-239 | REAL | — | |
| `ToolsValueChip` | 245-254 | REAL | — | title tooltip for clipped values |

## 5. Test honesty
ToolsDialog.test.tsx belongs to another agent. The shell's behaviors asserted there (focus order, aria wiring) would fail on real mutations per its header; not executed by me.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the body renders BEFORE the header in the DOM and the header paints first via `order: -1` (dialogs.css)" | ToolsDialogShell.tsx:18-21 | REAL | dialogs.css:73-77 verified |
| "The ✕ stays keyboard reachable at the END of the tab cycle" | ToolsDialogShell.tsx:23-25 | REAL | DOM order: body (96-99) → header/✕ (100-113) |
| "the expanded body stays in the DOM (`hidden`) so `aria-controls` always resolves" | ToolsDialogShell.tsx:142-145 | REAL | line 187 `hidden={!open}` |
| "the player.tools.* keys are already registered central" | ToolsDialogShell.tsx:31-33 | REAL | messages.ts:2174-2176 |

## 7. Dead / duplicated / unreachable code
None in this file. (The mounted tools' staged lanes are other files' findings — see visualizer.css audit F-001 and the EQ wrapper's data-pending-command="audio.set-eq" at ToolsDialog.tsx:125-132.)

## 8. Dependencies & flags
Dialog/IconButton/Surface (SurfaceCloseReason), dialogIcons (ChevronGlyph/CloseGlyph), dialogs.css, useMsg. All verified present.

## 9. Open questions for the next agent
- Surface.tsx owner: is there a reason SurfaceCloseReason has no "close" value (F-001)? Do Go-to-Time/Bookmarks dialogs mislabel the same way?
- Does any tool row in the dynamics/video lanes omit htmlFor (F-002 live defect)?
