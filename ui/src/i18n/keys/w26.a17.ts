// w26.a17 lane keys (player customize panel) — the "Customize Player"
// surface copy that has no existing catalog entry. Registered into
// ui/src/i18n/messages.ts by the Lead after the wave merge (contract §4.5);
// until then the customize renderer resolves these via its seam-first local
// resolver (see CustomizePanels.tsx `useCustomizeMsg`), so the copy renders
// identically before and after registration.
export const keys = {
  // Panel chrome — frame "Player - customize.svg" header strip.
  "customize.title": "Customize Player",
  "customize.instruction":
    "Drag items around and out of the box below to re-order them. Drag items into the box to remove them from the player.",
  "customize.close": "Close customize panel",
  "customize.save": "Save",
  // The drag-and-drop toolbar palette (the frame's dashed-tile box).
  "customize.box.label": "Toolbar items",
  // Quick-action tile labels — the frame's palette rows read "Forward 10s",
  // "Backward 10s", "Forward 15s", "Backward 15s" (doc 08 §4 button catalog
  // "±10s/15s"); the old "+10 s" chips stay for the control-bar buttons.
  "customize.tile.forward-10": "Forward 10s",
  "customize.tile.backward-10": "Backward 10s",
  "customize.tile.forward-15": "Forward 15s",
  "customize.tile.backward-15": "Backward 15s",
} as const;
