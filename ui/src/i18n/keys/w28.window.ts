// w28.window lane keys (the frameless-title-bar fix, w28 fix 2) — the OS
// window-control copy. The banner is now the real title bar
// (decorations:false in tauri.conf.json), so its right edge carries the
// REAL window controls: − (minimize), □/restore (toggle maximize),
// × (close). These labels name the OS action exactly — never an in-app
// action (the in-app trio keeps its own honest labels: "Minimize to
// miniplayer", "Toggle fullscreen", "Stop and return to the player").
export const keys = {
  // The OS chrome group (desktop shell only; never rendered in a browser).
  "shell.window.os.group": "Window controls",
  "shell.window.os.minimize": "Minimize",
  "shell.window.os.maximize": "Maximize",
  "shell.window.os.restore": "Restore",
  "shell.window.os.close": "Close",
} as const;
