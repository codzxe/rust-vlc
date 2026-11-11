// w26.a18 — player dialogs lane. Genuinely new keys introduced this lane
// (CONTRACT §4.5): the tools shell's card-toggle + close copy. The Lead
// registers these into ui/src/i18n/messages.ts after the wave merge; the
// components meanwhile read their local TOOLS_DIALOG_COPY module copy
// (the established dialogs COPY-NOTE pattern).
export const keys = {
  "player.tools.expand-settings": "Expand Settings",
  "player.tools.collapse-settings": "Collapse Settings",
  "player.tools.close": "Close",
} as const;
