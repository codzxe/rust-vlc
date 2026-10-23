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
  | "tokens.accent"
  // --- player screen shell (card 4.C1) ---
  | "player.canvas.label"
  | "player.canvas.placeholder"
  | "player.region.title-bar"
  | "player.region.controls-bar"
  | "player.region.library"
  | "player.region.playlist"
  | "player.region.media-info"
  | "player.panel.placeholder"
  | "player.controls.placeholder"
  | "player.menu.app"
  | "player.empty.title"
  | "player.empty.body"
  | "player.empty.open";

const messages: Readonly<Record<MessageKey, string>> = {
  "screen.player": "Player",
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
  // --- player screen shell (card 4.C1) ---
  "player.canvas.label": "Video canvas",
  "player.canvas.placeholder": "Placeholder: video output is not connected yet — engine wiring lands with card 4.H1.",
  "player.region.title-bar": "Player title bar",
  "player.region.controls-bar": "Playback controls",
  "player.region.library": "Library",
  "player.region.playlist": "Playlist",
  "player.region.media-info": "Media information",
  "player.panel.placeholder": "Placeholder panel container — content is implemented by a later card.",
  "player.controls.placeholder": "Placeholder region — transport controls land with card 4.C2.",
  "player.menu.app": "Application menu",
  "player.empty.title": "No media open",
  "player.empty.body": "Open media to start playback.",
  "player.empty.open": "Open media",
};

export type Msg = (key: MessageKey) => string;

/** Runtime guard for config-provided label keys (doc 07 §7.2 validator rule:
 * every `label_key` must resolve — the consumption side fails loudly on an
 * unknown key instead of rendering an empty label). */
export function isMessageKey(key: string): key is MessageKey {
  return Object.prototype.hasOwnProperty.call(messages, key);
}

// Scaffold stand-in for the generated i18n hook. Hook-shaped on purpose so
// call sites do not change when the real locale-pack implementation lands.
export function useMsg(): Msg {
  return (key) => messages[key];
}
