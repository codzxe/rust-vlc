// w26.k09 — player dialogs fixer lane. Genuinely new keys (CONTRACT §4.5):
// the Bookmarks + Go-to-Time dialog copy the j09 review found hardcoded as
// local COPY constants with no keys file behind them (review P2 — a §4.5
// violation: the Lead could not register what was never written down). The
// Lead registers these into ui/src/i18n/messages.ts after the wave merge;
// until then BookmarksDialog.tsx + GotoTimeDialog.tsx resolve them
// seam-first (central catalog → this file) via useDialogsMsg (the
// useToolsMsg / useNetworkStreamMsg pattern), so the copy renders
// identically before and after registration and pseudo-locale covers both
// dialogs the moment the keys are central.
//
// NOT duplicated here (reused through the same seams — grep first, §4.5):
//   * the tools shell's "Expand/Collapse Settings"/"Close" — already
//     registered central as player.tools.* (messages.ts) and recorded in
//     ui/src/i18n/keys/w26.a18.ts; ToolsDialogShell renders them through
//     useMsg directly since w26.k09;
//   * placeholder grammar follows customize.band.remove's "{item}" style:
//     single lowercase slots, filled with .replace() at the call site.
export const keys = {
  // Bookmarks dialog (issue #32, ui-dialog-bookmarks).
  "player.dialog.bookmarks.title": "Bookmarks",
  "player.dialog.bookmarks.subtitle": "Saved positions in the current media.",
  "player.dialog.bookmarks.label": "Label (optional)",
  "player.dialog.bookmarks.placeholder": "Name this bookmark",
  "player.dialog.bookmarks.add": "Add bookmark",
  // Accessible name of a row's jump action — includes the bookmark's label
  // so same-time rows stay distinguishable to screen readers (j09 P3).
  "player.dialog.bookmarks.jump": "Jump to {time} — {label}",
  "player.dialog.bookmarks.delete": "Delete bookmark at {time}",
  "player.dialog.bookmarks.unnamed": "Unnamed bookmark",
  "player.dialog.bookmarks.empty": "No bookmarks yet — add the current position.",
  "player.dialog.bookmarks.close": "Close",
  // Honest inline failures (j09 P2): the dialog's direct client sends
  // bypass the provider's dispatch, so the player's lastError surface never
  // sees them — these lines ARE the error surface for load/add/delete.
  "player.dialog.bookmarks.load-failed":
    "Couldn't load the bookmarks — try reopening the dialog.",
  "player.dialog.bookmarks.add-failed": "Couldn't add the bookmark — try again.",
  "player.dialog.bookmarks.delete-failed": "Couldn't delete the bookmark — try again.",
  // Go-to-Time dialog (issue #32, ui-dialog-goto-time).
  "player.dialog.goto.title": "Go to time",
  "player.dialog.goto.subtitle": "Jump to a specific time in the current media.",
  "player.dialog.goto.label": "Time",
  "player.dialog.goto.placeholder": "mm:ss or seconds (1:30, 90)",
  "player.dialog.goto.invalid": "Enter a time as seconds (90) or mm:ss (1:30)",
  "player.dialog.goto.beyond-end": "That time is past the end ({duration})",
  "player.dialog.goto.action": "Go",
  "player.dialog.goto.cancel": "Cancel",
  "player.dialog.goto.close": "Close",
} as const;
