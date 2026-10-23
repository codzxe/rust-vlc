// Message catalog + useMsg() for the Phase-4 scaffold.
//
// Doc 07 §6.5 / ui README: user-visible text must flow through useMsg() +
// MessageKey — the `lumen/i18n-no-literals` lint rule (eslint.config.js)
// bans raw JSX literals. The real generated locale packs arrive with the i18n
// pipeline (doc 07); this hand-written table is the scaffold stand-in and
// every entry honestly labels its screen as a placeholder.

export type MessageKey =
  | "screen.player"
  | "screen.player.music"
  | "screen.miniplayer"
  | "screen.playlist"
  | "screen.settings"
  | "screen.onboarding"
  | "screen.keyboard-shortcuts"
  | "screen.open-network-stream"
  | "screen.not-found"
  | "screen.placeholder.body"
  | "screen.dev.tokens"
  | "tokens.mode"
  | "tokens.accent";

const messages: Readonly<Record<MessageKey, string>> = {
  "screen.player": "Player (placeholder)",
  "screen.player.music": "Music player (placeholder)",
  "screen.miniplayer": "Miniplayer (placeholder)",
  "screen.playlist": "Playlist (placeholder)",
  "screen.settings": "Settings (placeholder)",
  "screen.onboarding": "Onboarding (placeholder)",
  "screen.keyboard-shortcuts": "Keyboard shortcuts (placeholder)",
  "screen.open-network-stream": "Open network stream (placeholder)",
  "screen.not-found": "Screen not found",
  "screen.placeholder.body":
    "Route skeleton only — the Figma-exact composition lands in later Phase-4 cards (doc 08 §1.6).",
  "screen.dev.tokens": "Token sandbox (dev fixture — not a product screen)",
  "tokens.mode": "Mode",
  "tokens.accent": "Accent",
};

export type Msg = (key: MessageKey) => string;

// Scaffold stand-in for the generated i18n hook. Hook-shaped on purpose so
// call sites do not change when the real locale-pack implementation lands.
export function useMsg(): Msg {
  return (key) => messages[key];
}
