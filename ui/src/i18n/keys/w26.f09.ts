// w26.f09 lane keys (player composition fixes) — the canvas context menu's
// "Customize interface…" trigger row (e04 F1: the customize surface had no
// user path; w26.f09 wires the trigger + the lumen:open-customize seam).
// VLC 4 grammar ("Customize Interface…" — the toolbar editor); the design
// frames title the SURFACE "Customize Player" (customize.title, catalog).
// Registered into ui/src/i18n/messages.ts by the Lead after the wave merge;
// until then PlayerScreen resolves it through its lane seam-first resolver,
// so the copy renders identically before and after.
export const keys = {
  // Player canvas context menu — the customize trigger (right-click menu.svg
  // carries 14 frame rows; this Lead-approved 15th row opens the surface).
  "player.canvas.menu.customize-interface": "Customize interface…",
} as const;
