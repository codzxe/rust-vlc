// w26.f13 lane keys — the player app menu's frame-exact labels (menu.png /
// menu.svg, analysis §A1). Registered into ui/src/i18n/messages.ts by the
// Lead after the wave merge; until then ControlBar resolves them through its
// seam-first lane resolver (PlayerScreen's c6/f09 pattern), so the copy
// renders identically before and after.
//
// Reused catalog keys for the other rows: hotkeys.open-folder /
// hotkeys.open-network-stream / hotkeys.open-disc /
// hotkeys.open-capture-device / hotkeys.convert (frame-exact "Open Folder" /
// "Open Network Stream" / "Open Disc" / "Open Capture Device" / "Convert"),
// menu.view.settings ("Settings"), open.recent.empty (the honest empty
// reason row — the open-media dialog's own copy).
export const keys = {
  /** menu.svg row 1 — the frame's exact label ("hotkeys.open-file" reads "Open File"). */
  "player.appmenu.open-files": "Open File(s)",
  /** menu.svg row 7 — the recents flyout parent ("player.open.recent" reads "Recent"). */
  "player.appmenu.open-recent": "Open Recent",
  /** menu.svg row 8 — "Profile (Movies)" minus the fabricated profile name (no profile engine seam). */
  "player.appmenu.profile": "Profile",
  /** menu.svg row 9 — frame label ("menu.help.keyboard-shortcuts" reads "Keyboard shortcuts"). */
  "player.appmenu.keyboard-shortcuts": "Keyboard Shortcuts",
  /** menu.svg row 10 — the Customize Player panel trigger ("menu.tools.customize" reads "Customize player…"). */
  "player.appmenu.customize": "Customize Player",
  /** menu.svg row 12 — frame label ("menu.help.about" reads "About"). */
  "player.appmenu.about": "About VLC",
} as const;
