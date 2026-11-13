// w26.k07 (wave K fixer, lane 07/20 — j07 findings close-out, control bar).
// New message keys per CONTRACT §4.5 — the Lead registers them into
// ui/src/i18n/messages.ts post-wave; until then the ControlBar queue tab
// resolves them through a lane seam typed over this WHOLE file (the honest
// pre-registration pattern — unlike the dead post-merge resolvers j07 P1
// removed).
//
// Content source: Controls.svg queue variant — the meta row "4 items - 2h
// 3m" (the composition was hardcoded English unit grammar in ControlBar.tsx,
// j07 P2-4). The Go-to-Time dialog copy j07 flagged as P2-6 lives in
// ui/src/i18n/keys/w26.k09.ts (the dialogs fixer lane landed it in this
// same wave — NOT duplicated here; grep-first rule, §4.5).
export const keys = {
  /** Queue panel meta row — count-only branch ("4 items"); {count} composed in code. */
  "player.controls.queue-meta": "{count} items",
  /** Queue panel meta row — total branch ("4 items - 2h 3m"); {duration} is the localized queue-total. */
  "player.controls.queue-meta-total": "{count} items - {duration}",
  /** Queue total duration unit grammar ("2h 3m"); {hours}/{minutes} composed in code. */
  "player.controls.queue-total": "{hours}h {minutes}m",
} as const;
