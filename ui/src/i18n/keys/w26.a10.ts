// w26.a10 lane keys (appshell) — the shell-chrome copy that has no existing
// catalog entry. Registered into ui/src/i18n/messages.ts by the Lead after
// the wave merge (contract §4.5); until then the appshell resolves these via
// its local seam-first resolver (see AppShell.tsx `useShellMsg`), so the
// copy renders identically before and after registration.
export const keys = {
  // w26.k18 (j18 F4): shell.sidebar.create.hint, shell.playlist.cool-songs/anime-to-watch pruned from this lane file — verified dead
  // (unreferenced by any source; grandfathered in ui/i18n-dead-keys.json)
  // so a future Lead prune of the catalog cannot resurrect them via re-merge.
  // Wordmark / app identity (product.toml identity.name is the source of
  // truth for hosts; the catalog key is the UI's config-driven seam).
  "app.name": "Lumen",
  // Title-bar window controls — in-app actions, NOT OS chrome (web shell):
  // minimize collapses the app into the miniplayer card, maximize toggles
  // the document fullscreen, close stops playback and returns home.
  "shell.window.group": "Window controls",
  "shell.window.minimize": "Minimize to miniplayer",
  "shell.window.maximize": "Toggle fullscreen",
  "shell.window.close": "Stop and return to the player",
  // Title bar left cluster.
  "shell.title.toggle-sidebar": "Toggle playlists sidebar",
  // Icon rail pinned bottom stop (opens the shell's open-media dialog).
  "shell.rail.add": "Add media",
  // Playlists sidebar (sidebar.svg reference).
  "shell.sidebar.title": "Playlists",
  "shell.sidebar.back": "Back",
  "shell.sidebar.more": "More playlist options",
  "shell.sidebar.more.hint": "Playlist actions are not exposed by this build yet.",
  "shell.sidebar.search": "Search playlists",
  "shell.sidebar.search.placeholder": "Search playlists…",
  "shell.sidebar.search.empty": "No playlists match your search.",
  "shell.sidebar.count-suffix": "Items",
  "shell.sidebar.create": "Create playlist",
  "shell.sidebar.sort": "Sort playlists alphabetically",
  "shell.sidebar.view-list": "List view",
  "shell.sidebar.view-grid": "Grid view",
  "shell.sidebar.empty": "No playlists yet.",
  "shell.sidebar.list-label": "Playlists",
} as const;
