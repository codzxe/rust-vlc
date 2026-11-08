// GENERATED FILE — DO NOT EDIT.
//
// Produced by ui/scripts/gen-tokens.mjs (card 4.A2) from
// themes/generated/tokens.css — the committed output of the Rust
// crates/theme-tokens pipeline (card 1.B4, `cargo xtask tokens`).
// Regenerate with `npm run tokens:generate` in ui/. Drift is caught by
// src/tokens/tokens.test.ts (pinned sha256 + structure) and
// `npm run tokens:check`.
//
// Components consume SEMANTIC tokens only, via `var(--…)` in CSS or
// `tokenVar("…")` for inline styles (doc 08 §3.2). Raw hex/rgb/px values
// are rejected by the local/semantic-tokens-only lint rule.

/** SHA-256 of themes/generated/tokens.css at generation time. */
export const TOKENS_CSS_SHA256 = "4f6045e209a2d2a89aa8512c3f15839d68d760e4ec8db5168c7cad42d8f12c38";

/** The full generated token CSS — same bytes as tokens.css, carried here so
 * tests can pin structure + hash without the filesystem (vitest stubs raw
 * .css imports). */
export const TOKENS_CSS = "/* GENERATED FILE — DO NOT EDIT.\n *\n * Produced by `cargo xtask tokens` (crates/theme-tokens `tokens-gen`,\n * card 1.B4) from themes/tokens.json — doc 08 §3. Regeneration is\n * idempotent; the committed copy is diff-tested by\n * crates/theme-tokens/tests/diff.rs.\n *\n * Solstice design language (2026-11-08): warm neutrals, Keyvox-gold default\n * accent (yellow), sunset/note/tape/keycap semantic groups, Nunito face.\n * NOTE: this revision of the bridge was hand-maintained in an environment\n * without cargo (documented in the redesign worklog); `cargo xtask tokens`\n * must reproduce it when the Rust toolchain returns — diff.rs will verify.\n *\n * Components consume SEMANTIC tokens only, via `var(--<name>)`\n * (doc 08 §3.2). The primitive and alias layers are never emitted\n * here; they exist for re-derivation from the Figma variables.\n *\n * Block layout (doc 08 §3.3 — attributes on the root element):\n *   :root                                 mode-independent values,\n *                                        the default mode (dark) and\n *                                        the default accent (yellow)\n *   :root[data-theme=\"<mode>\"]            per-mode values\n *   :root[data-accent=\"<id>\"]             alternate-accent values,\n *                                        default mode\n *   :root[data-theme=\"<mode>\"][data-accent=\"<id>\"]\n *                                        ...and their per-mode values\n *\n * Modes (2): dark (default), light.\n * Accents (9): yellow (default), orange, red, green, cyan, blue, violet, pink, gray.\n * Tokens: 125 primitive / 91 alias / 91 semantic (source:\n * themes/tokens.json).\n */\n\n:root {\n  --blur-backdrop: 25px;\n  --blur-backdrop-small: 13.75px;\n  --color-accent: #f2c41d;\n  --color-accent-bright: #f2c41d;\n  --color-accent-icon: #f2c41d;\n  --color-accent-strong: #c89a10;\n  --color-accent-tinted: #3b2f08;\n  --color-content-bright: #f5eedc;\n  --color-content-on-accent: #201a10;\n  --color-content-primary: #cbbda1;\n  --color-content-secondary: #b7a98d;\n  --color-content-tertiary: #8c7c60;\n  --color-keycap: #2a2417;\n  --color-keycap-edge: #4a3e28;\n  --color-keycap-ink: #f2e9d5;\n  --color-keycap-shadow: #14100a;\n  --color-note: #262019;\n  --color-note-edge: #3b3122;\n  --color-note-ink: #f2e9d5;\n  --color-note-ink-muted: #b9ab8f;\n  --color-record: #ff0000;\n  --color-stroke-border: #352c1d;\n  --color-stroke-icon: #cbbda1;\n  --color-stroke-warm: #241c11;\n  --color-sunset-1: #191631;\n  --color-sunset-2: #2e3e80;\n  --color-sunset-3: #6c69ba;\n  --color-sunset-4: #a479ad;\n  --color-sunset-5: #e68373;\n  --color-sunset-6: #ef9b4c;\n  --color-sunset-7: #7a3b1e;\n  --color-surface: #241d13;\n  --color-surface-chrome: #282013;\n  --color-surface-deep: #12100b;\n  --color-surface-hover: #2f2617;\n  --color-surface-muted: #5f5036;\n  --color-surface-panel: #1a150e;\n  --color-surface-raised: #4c3f28;\n  --color-surface-selected: #382d1b;\n  --color-tape: #dfc27f;\n  --color-tape-edge: #c6a968;\n  --font-family-sans: \"Nunito\", \"Inter\", system-ui, sans-serif;\n  --font-family-time: ui-monospace, \"SFMono-Regular\", \"Cascadia Mono\", Menlo, Consolas, monospace;\n  --font-size-body: 14px;\n  --font-size-caption: 11px;\n  --font-size-display: 32px;\n  --font-size-label: 16px;\n  --font-size-micro: 6px;\n  --font-size-time: 15px;\n  --font-size-title: 18px;\n  --font-weight-bold: 700;\n  --font-weight-extrabold: 800;\n  --font-weight-medium: 500;\n  --font-weight-regular: 400;\n  --font-weight-semibold: 600;\n  --gradient-visualizer-1: #ff5e5b;\n  --gradient-visualizer-2: #f2c41d;\n  --gradient-visualizer-3: #ff9e4a;\n  --gradient-visualizer-4: #6c69ba;\n  --motion-duration-fast: 120ms;\n  --motion-duration-slow: 480ms;\n  --motion-duration-step: 280ms;\n  --opacity-chrome: 1;\n  --opacity-disabled: 0.5;\n  --opacity-emphasis: 0.7;\n  --opacity-muted: 0.3;\n  --opacity-panel: 0.95;\n  --opacity-scrim: 0.2;\n  --opacity-scrim-strong: 0.24;\n  --opacity-scrim-subtle: 0.14;\n  --opacity-swatch-rest: 0.7;\n  --radius-app: 19px;\n  --radius-card: 16px;\n  --radius-card-lg: 30px;\n  --radius-control: 10px;\n  --radius-hairline: 1px;\n  --radius-inset: 6px;\n  --radius-menu: 12px;\n  --radius-panel: 12px;\n  --radius-ring: 21px;\n  --radius-thumbnail: 8px;\n  --radius-window: 16px;\n  --shadow-card: 0px 5px 16px 0px rgba(20, 13, 5, 0.38);\n  --shadow-note: 0px 14px 36px 0px rgba(12, 8, 3, 0.5);\n  --shadow-overlay: 0px 12px 36px 0px rgba(12, 8, 3, 0.6);\n  --space-grid: 4px;\n  --space-inset: 8px;\n  --space-item-pitch: 48px;\n  --space-track-inset: 2px;\n  --stroke-icon: 1.5px;\n  --stroke-icon-max: 3px;\n  --stroke-icon-strong: 2px;\n  --stroke-ring: 4px;\n  --stroke-window: 20px;\n}\n\n:root[data-theme=\"light\"] {\n  --color-accent-bright: #ffd84d;\n  --color-accent-icon: #d9a912;\n  --color-accent-tinted: #e8d39a;\n  --color-content-bright: #1f1b14;\n  --color-content-primary: #4a3e2a;\n  --color-content-secondary: #3e3320;\n  --color-keycap: #fffdf4;\n  --color-keycap-edge: #d9cca8;\n  --color-keycap-ink: #1f1b16;\n  --color-keycap-shadow: #c9bb98;\n  --color-note: #efe6d2;\n  --color-note-edge: #e0d2b0;\n  --color-note-ink: #1f1b16;\n  --color-note-ink-muted: #6e6250;\n  --color-stroke-border: #ddd0b2;\n  --color-stroke-icon: #4a3e2a;\n  --color-sunset-1: #fbf3e2;\n  --color-sunset-2: #f9d9a0;\n  --color-sunset-3: #f4b183;\n  --color-sunset-4: #e58f87;\n  --color-sunset-5: #c8739e;\n  --color-sunset-6: #8e6fb5;\n  --color-sunset-7: #5d5a9c;\n  --color-surface: #efe3c6;\n  --color-surface-chrome: #f3ead2;\n  --color-surface-deep: #f6efdd;\n  --color-surface-hover: #e5d8b8;\n  --color-surface-muted: #c9b98f;\n  --color-surface-panel: #f8f1e0;\n  --color-surface-raised: #e8dbbd;\n  --color-surface-selected: #e0d0a8;\n  --opacity-chrome: 0.75;\n  --opacity-panel: 0.8;\n}\n\n:root[data-accent=\"orange\"] {\n  --color-accent: #d37918;\n  --color-accent-bright: #d37918;\n  --color-accent-icon: #d37918;\n  --color-accent-strong: #ab6314;\n  --color-accent-tinted: #371f06;\n  --color-content-on-accent: #161616;\n}\n\n:root[data-theme=\"light\"][data-accent=\"orange\"] {\n  --color-accent-bright: #e8851a;\n  --color-accent-icon: #c2711a;\n  --color-accent-tinted: #d8ba9a;\n}\n\n:root[data-accent=\"red\"] {\n  --color-accent: #e15032;\n  --color-accent-bright: #e15032;\n  --color-accent-icon: #e15032;\n  --color-accent-strong: #c83a1d;\n  --color-accent-tinted: #351008;\n  --color-content-on-accent: #161616;\n}\n\n:root[data-theme=\"light\"][data-accent=\"red\"] {\n  --color-accent-bright: #e66044;\n  --color-accent-icon: #db4728;\n  --color-accent-tinted: #d6a69c;\n}\n\n:root[data-accent=\"green\"] {\n  --color-accent: #19a234;\n  --color-accent-bright: #19a234;\n  --color-accent-icon: #19a234;\n  --color-accent-strong: #137a27;\n  --color-accent-tinted: #083511;\n  --color-content-on-accent: #161616;\n}\n\n:root[data-theme=\"light\"][data-accent=\"green\"] {\n  --color-accent-bright: #1ab839;\n  --color-accent-icon: #1a9231;\n  --color-accent-tinted: #9cd6a7;\n}\n\n:root[data-accent=\"cyan\"] {\n  --color-accent: #18aad3;\n  --color-accent-bright: #18aad3;\n  --color-accent-icon: #18aad3;\n  --color-accent-strong: #1389aa;\n  --color-accent-tinted: #062c37;\n  --color-content-on-accent: #161616;\n}\n\n:root[data-theme=\"light\"][data-accent=\"cyan\"] {\n  --color-accent-bright: #1abbe8;\n  --color-accent-icon: #1a9dc2;\n  --color-accent-tinted: #9acad8;\n}\n\n:root[data-accent=\"blue\"] {\n  --color-accent: #185dd3;\n  --color-accent-bright: #185dd3;\n  --color-accent-icon: #185dd3;\n  --color-accent-strong: #134baa;\n  --color-accent-tinted: #061837;\n  --color-content-on-accent: #eaeaea;\n}\n\n:root[data-theme=\"light\"][data-accent=\"blue\"] {\n  --color-accent-bright: #1a66e8;\n  --color-accent-icon: #1a58c2;\n  --color-accent-tinted: #9ab1d8;\n}\n\n:root[data-accent=\"violet\"] {\n  --color-accent: #8e42ff;\n  --color-accent-bright: #8e42ff;\n  --color-accent-icon: #8e42ff;\n  --color-accent-strong: #7314ff;\n  --color-accent-tinted: #19003d;\n  --color-content-on-accent: #ffffff;\n}\n\n:root[data-theme=\"light\"][data-accent=\"violet\"] {\n  --color-accent-bright: #9c59ff;\n  --color-accent-icon: #8536fc;\n  --color-accent-tinted: #b192e0;\n}\n\n:root[data-accent=\"pink\"] {\n  --color-accent: #f052b3;\n  --color-accent-bright: #f052b3;\n  --color-accent-icon: #f052b3;\n  --color-accent-strong: #ec28a0;\n  --color-accent-tinted: #380524;\n  --color-content-on-accent: #161616;\n}\n\n:root[data-theme=\"light\"][data-accent=\"pink\"] {\n  --color-accent-bright: #f365bd;\n  --color-accent-icon: #ec47ac;\n  --color-accent-tinted: #da98c0;\n}\n\n:root[data-accent=\"gray\"] {\n  --color-accent: #a99c82;\n  --color-accent-bright: #a99c82;\n  --color-accent-icon: #a99c82;\n  --color-accent-strong: #8f8268;\n  --color-accent-tinted: #241f14;\n  --color-content-on-accent: #161616;\n}\n\n:root[data-theme=\"light\"][data-accent=\"gray\"] {\n  --color-accent-bright: #b5aa90;\n  --color-accent-icon: #9c9077;\n  --color-accent-tinted: #c4ba9f;\n}\n";

/** Theming modes (doc 08 §3.3). `dark` is the product default. */
export type TokenMode = "dark" | "light";
export const TOKEN_MODES: readonly TokenMode[] = ["dark", "light"];

/** The 9 accent selections (doc 08 §3.3). `orange` is the product default. */
export type AccentId = "blue" | "cyan" | "gray" | "green" | "orange" | "pink" | "red" | "violet" | "yellow";
export const ACCENTS: readonly AccentId[] = ["blue", "cyan", "gray", "green", "orange", "pink", "red", "violet", "yellow"];

/** Every semantic CSS custom property name emitted in the `:root` block. */
export type SemanticTokenName = "--blur-backdrop" | "--blur-backdrop-small" | "--color-accent" | "--color-accent-bright" | "--color-accent-icon" | "--color-accent-strong" | "--color-accent-tinted" | "--color-content-bright" | "--color-content-on-accent" | "--color-content-primary" | "--color-content-secondary" | "--color-content-tertiary" | "--color-keycap" | "--color-keycap-edge" | "--color-keycap-ink" | "--color-keycap-shadow" | "--color-note" | "--color-note-edge" | "--color-note-ink" | "--color-note-ink-muted" | "--color-record" | "--color-stroke-border" | "--color-stroke-icon" | "--color-stroke-warm" | "--color-sunset-1" | "--color-sunset-2" | "--color-sunset-3" | "--color-sunset-4" | "--color-sunset-5" | "--color-sunset-6" | "--color-sunset-7" | "--color-surface" | "--color-surface-chrome" | "--color-surface-deep" | "--color-surface-hover" | "--color-surface-muted" | "--color-surface-panel" | "--color-surface-raised" | "--color-surface-selected" | "--color-tape" | "--color-tape-edge" | "--font-family-sans" | "--font-family-time" | "--font-size-body" | "--font-size-caption" | "--font-size-display" | "--font-size-label" | "--font-size-micro" | "--font-size-time" | "--font-size-title" | "--font-weight-bold" | "--font-weight-extrabold" | "--font-weight-medium" | "--font-weight-regular" | "--font-weight-semibold" | "--gradient-visualizer-1" | "--gradient-visualizer-2" | "--gradient-visualizer-3" | "--gradient-visualizer-4" | "--motion-duration-fast" | "--motion-duration-slow" | "--motion-duration-step" | "--opacity-chrome" | "--opacity-disabled" | "--opacity-emphasis" | "--opacity-muted" | "--opacity-panel" | "--opacity-scrim" | "--opacity-scrim-strong" | "--opacity-scrim-subtle" | "--opacity-swatch-rest" | "--radius-app" | "--radius-card" | "--radius-card-lg" | "--radius-control" | "--radius-hairline" | "--radius-inset" | "--radius-menu" | "--radius-panel" | "--radius-ring" | "--radius-thumbnail" | "--radius-window" | "--shadow-card" | "--shadow-note" | "--shadow-overlay" | "--space-grid" | "--space-inset" | "--space-item-pitch" | "--space-track-inset" | "--stroke-icon" | "--stroke-icon-max" | "--stroke-icon-strong" | "--stroke-ring" | "--stroke-window";
export const SEMANTIC_TOKEN_NAMES: readonly SemanticTokenName[] = [
  "--blur-backdrop",
  "--blur-backdrop-small",
  "--color-accent",
  "--color-accent-bright",
  "--color-accent-icon",
  "--color-accent-strong",
  "--color-accent-tinted",
  "--color-content-bright",
  "--color-content-on-accent",
  "--color-content-primary",
  "--color-content-secondary",
  "--color-content-tertiary",
  "--color-keycap",
  "--color-keycap-edge",
  "--color-keycap-ink",
  "--color-keycap-shadow",
  "--color-note",
  "--color-note-edge",
  "--color-note-ink",
  "--color-note-ink-muted",
  "--color-record",
  "--color-stroke-border",
  "--color-stroke-icon",
  "--color-stroke-warm",
  "--color-sunset-1",
  "--color-sunset-2",
  "--color-sunset-3",
  "--color-sunset-4",
  "--color-sunset-5",
  "--color-sunset-6",
  "--color-sunset-7",
  "--color-surface",
  "--color-surface-chrome",
  "--color-surface-deep",
  "--color-surface-hover",
  "--color-surface-muted",
  "--color-surface-panel",
  "--color-surface-raised",
  "--color-surface-selected",
  "--color-tape",
  "--color-tape-edge",
  "--font-family-sans",
  "--font-family-time",
  "--font-size-body",
  "--font-size-caption",
  "--font-size-display",
  "--font-size-label",
  "--font-size-micro",
  "--font-size-time",
  "--font-size-title",
  "--font-weight-bold",
  "--font-weight-extrabold",
  "--font-weight-medium",
  "--font-weight-regular",
  "--font-weight-semibold",
  "--gradient-visualizer-1",
  "--gradient-visualizer-2",
  "--gradient-visualizer-3",
  "--gradient-visualizer-4",
  "--motion-duration-fast",
  "--motion-duration-slow",
  "--motion-duration-step",
  "--opacity-chrome",
  "--opacity-disabled",
  "--opacity-emphasis",
  "--opacity-muted",
  "--opacity-panel",
  "--opacity-scrim",
  "--opacity-scrim-strong",
  "--opacity-scrim-subtle",
  "--opacity-swatch-rest",
  "--radius-app",
  "--radius-card",
  "--radius-card-lg",
  "--radius-control",
  "--radius-hairline",
  "--radius-inset",
  "--radius-menu",
  "--radius-panel",
  "--radius-ring",
  "--radius-thumbnail",
  "--radius-window",
  "--shadow-card",
  "--shadow-note",
  "--shadow-overlay",
  "--space-grid",
  "--space-inset",
  "--space-item-pitch",
  "--space-track-inset",
  "--stroke-icon",
  "--stroke-icon-max",
  "--stroke-icon-strong",
  "--stroke-ring",
  "--stroke-window",
];

/** Build a `var(--…)` reference for a semantic token (doc 08 §3.2). */
export function tokenVar(name: SemanticTokenName): string {
  return `var(${name})`;
}
