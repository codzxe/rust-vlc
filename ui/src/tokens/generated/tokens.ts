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
export const TOKENS_CSS_SHA256 = "6edc89dfdbab0f439b081088b2314e70d326c3de2123fbe23e44ae7e33901f03";

/** The full generated token CSS — same bytes as tokens.css, carried here so
 * tests can pin structure + hash without the filesystem (vitest stubs raw
 * .css imports). */
export const TOKENS_CSS = "/* GENERATED FILE — DO NOT EDIT.\n *\n * Produced by `cargo xtask tokens` (crates/theme-tokens `tokens-gen`,\n * card 1.B4) from themes/tokens.json — doc 08 §3. Regeneration is\n * idempotent; the committed copy is diff-tested by\n * crates/theme-tokens/tests/diff.rs.\n *\n * Components consume SEMANTIC tokens only, via `var(--<name>)`\n * (doc 08 §3.2). The primitive and alias layers are never emitted\n * here; they exist for re-derivation from the Figma variables.\n *\n * Block layout (doc 08 §3.3 — attributes on the root element):\n *   :root                                 mode-independent values,\n *                                        the default mode (dark) and\n *                                        the default accent (orange)\n *   :root[data-theme=\"<mode>\"]            per-mode values\n *   :root[data-accent=\"<id>\"]             alternate-accent values,\n *                                        default mode\n *   :root[data-theme=\"<mode>\"][data-accent=\"<id>\"]\n *                                        ...and their per-mode values\n *\n * Modes (2): dark (default), light.\n * Accents (9): orange (default), red, yellow, green, cyan, blue, violet, pink, gray.\n * Tokens: 106 primitive / 80 alias / 80 semantic (source:\n * themes/tokens.json).\n */\n\n:root {\n  --blur-backdrop: 25px;\n  --blur-backdrop-small: 13.75px;\n  --color-accent: #d37918;\n  --color-accent-bright: #d37918;\n  --color-accent-icon: #d37918;\n  --color-accent-strong: #ab6314;\n  --color-accent-tinted: #371f06;\n  --color-content-bright: #eaeaea;\n  --color-content-on-accent: #090909;\n  --color-content-primary: #c6c6c6;\n  --color-content-secondary: #8f8f8f;\n  --color-content-tertiary: #828282;\n  --color-eq-1: #e15032;\n  --color-eq-2: #c49e23;\n  --color-eq-3: #19a234;\n  --color-eq-4: #18aad3;\n  --color-eq-5: #185dd3;\n  --color-eq-6: #8e42ff;\n  --color-eq-7: #f052b3;\n  --color-record: #e15032;\n  --color-stroke-border: #404040;\n  --color-stroke-icon: #8f8f8f;\n  --color-surface: #090909;\n  --color-surface-chrome: #161616;\n  --color-surface-deep: #0e0e0e;\n  --color-surface-hover: #262626;\n  --color-surface-muted: #151515;\n  --color-surface-panel: #191919;\n  --color-surface-raised: #222222;\n  --color-surface-selected: #1f1f1f;\n  --font-family-sans: \"Inter\", \"-apple-system\", \"Segoe UI\", \"Roboto\", sans-serif;\n  --font-family-time: \"Inter\", \"-apple-system\", \"Segoe UI\", \"Roboto\", sans-serif;\n  --font-size-body: 14px;\n  --font-size-caption: 11px;\n  --font-size-display: 32px;\n  --font-size-label: 16px;\n  --font-size-micro: 6px;\n  --font-size-section: 15px;\n  --font-size-sidebar: 13px;\n  --font-size-time: 15px;\n  --font-size-title: 18px;\n  --font-weight-bold: 700;\n  --font-weight-extrabold: 800;\n  --font-weight-medium: 500;\n  --font-weight-regular: 400;\n  --font-weight-semibold: 600;\n  --motion-duration-fast: 120ms;\n  --motion-duration-slow: 480ms;\n  --motion-duration-step: 280ms;\n  --opacity-chrome: 1;\n  --opacity-disabled: 0.5;\n  --opacity-emphasis: 0.7;\n  --opacity-muted: 0.3;\n  --opacity-panel: 0.95;\n  --opacity-scrim: 0.2;\n  --opacity-scrim-strong: 0.24;\n  --opacity-scrim-subtle: 0.14;\n  --opacity-swatch-rest: 0.7;\n  --radius-app: 19px;\n  --radius-card: 14px;\n  --radius-card-lg: 30px;\n  --radius-control: 6px;\n  --radius-hairline: 1px;\n  --radius-inset: 4px;\n  --radius-menu: 12px;\n  --radius-panel: 12px;\n  --radius-ring: 21px;\n  --radius-thumbnail: 8px;\n  --radius-window: 14px;\n  --shadow-card: 0px 4.76px 4.76px 0px rgba(0, 0, 0, 0.38);\n  --shadow-overlay: 0px 4.76px 5.9px 0px rgba(0, 0, 0, 0.7);\n  --space-grid: 4px;\n  --space-inset: 8px;\n  --space-item-pitch: 48px;\n  --space-track-inset: 2px;\n  --stroke-icon: 1.5px;\n  --stroke-icon-max: 3px;\n  --stroke-icon-strong: 2px;\n  --stroke-ring: 4px;\n  --stroke-window: 20px;\n}\n\n:root[data-theme=\"light\"] {\n  --color-accent-bright: #b35f0d;\n  --color-accent-icon: #c2711a;\n  --color-accent-tinted: #d8ba9a;\n  --color-content-bright: #161616;\n  --color-content-primary: #2b2b2b;\n  --color-content-secondary: #6e6e6e;\n  --color-content-tertiary: #767676;\n  --color-stroke-border: #d9d9d9;\n  --color-stroke-icon: #9a9a9a;\n  --color-surface: #f0f0f0;\n  --color-surface-chrome: #f5f5f5;\n  --color-surface-deep: #dedede;\n  --color-surface-hover: #e6e6e6;\n  --color-surface-muted: #d4d4d4;\n  --color-surface-panel: #efefef;\n  --color-surface-raised: #ffffff;\n  --color-surface-selected: #e0e0e0;\n  --opacity-chrome: 0.75;\n  --opacity-panel: 0.8;\n}\n\n:root[data-accent=\"red\"] {\n  --color-accent: #e15032;\n  --color-accent-bright: #e15032;\n  --color-accent-icon: #e15032;\n  --color-accent-strong: #c83a1d;\n  --color-accent-tinted: #351008;\n  --color-content-on-accent: #161616;\n}\n\n:root[data-theme=\"light\"][data-accent=\"red\"] {\n  --color-accent-bright: #e66044;\n  --color-accent-icon: #db4728;\n  --color-accent-tinted: #d6a69c;\n}\n\n:root[data-accent=\"yellow\"] {\n  --color-accent: #c49e23;\n  --color-accent-bright: #c49e23;\n  --color-accent-icon: #c49e23;\n  --color-accent-strong: #9d7f1c;\n  --color-accent-tinted: #342a09;\n  --color-content-on-accent: #161616;\n}\n\n:root[data-theme=\"light\"][data-accent=\"yellow\"] {\n  --color-accent-bright: #daaf24;\n  --color-accent-icon: #b49224;\n  --color-accent-tinted: #d4c79e;\n}\n\n:root[data-accent=\"green\"] {\n  --color-accent: #19a234;\n  --color-accent-bright: #19a234;\n  --color-accent-icon: #19a234;\n  --color-accent-strong: #137a27;\n  --color-accent-tinted: #083511;\n  --color-content-on-accent: #161616;\n}\n\n:root[data-theme=\"light\"][data-accent=\"green\"] {\n  --color-accent-bright: #1ab839;\n  --color-accent-icon: #1a9231;\n  --color-accent-tinted: #9cd6a7;\n}\n\n:root[data-accent=\"cyan\"] {\n  --color-accent: #18aad3;\n  --color-accent-bright: #18aad3;\n  --color-accent-icon: #18aad3;\n  --color-accent-strong: #1389aa;\n  --color-accent-tinted: #062c37;\n  --color-content-on-accent: #161616;\n}\n\n:root[data-theme=\"light\"][data-accent=\"cyan\"] {\n  --color-accent-bright: #1abbe8;\n  --color-accent-icon: #1a9dc2;\n  --color-accent-tinted: #9acad8;\n}\n\n:root[data-accent=\"blue\"] {\n  --color-accent: #185dd3;\n  --color-accent-bright: #185dd3;\n  --color-accent-icon: #185dd3;\n  --color-accent-strong: #134baa;\n  --color-accent-tinted: #061837;\n  --color-content-on-accent: #eaeaea;\n}\n\n:root[data-theme=\"light\"][data-accent=\"blue\"] {\n  --color-accent-bright: #1a66e8;\n  --color-accent-icon: #1a58c2;\n  --color-accent-tinted: #9ab1d8;\n}\n\n:root[data-accent=\"violet\"] {\n  --color-accent: #8e42ff;\n  --color-accent-bright: #8e42ff;\n  --color-accent-icon: #8e42ff;\n  --color-accent-strong: #7314ff;\n  --color-accent-tinted: #19003d;\n  --color-content-on-accent: #ffffff;\n}\n\n:root[data-theme=\"light\"][data-accent=\"violet\"] {\n  --color-accent-bright: #9c59ff;\n  --color-accent-icon: #8536fc;\n  --color-accent-tinted: #b192e0;\n}\n\n:root[data-accent=\"pink\"] {\n  --color-accent: #f052b3;\n  --color-accent-bright: #f052b3;\n  --color-accent-icon: #f052b3;\n  --color-accent-strong: #ec28a0;\n  --color-accent-tinted: #380524;\n  --color-content-on-accent: #161616;\n}\n\n:root[data-theme=\"light\"][data-accent=\"pink\"] {\n  --color-accent-bright: #f365bd;\n  --color-accent-icon: #ec47ac;\n  --color-accent-tinted: #da98c0;\n}\n\n:root[data-accent=\"gray\"] {\n  --color-accent: #afafaf;\n  --color-accent-bright: #afafaf;\n  --color-accent-icon: #afafaf;\n  --color-accent-strong: #989898;\n  --color-accent-tinted: #1e1e1e;\n  --color-content-on-accent: #161616;\n}\n\n:root[data-theme=\"light\"][data-accent=\"gray\"] {\n  --color-accent-bright: #bababa;\n  --color-accent-icon: #a7a7a7;\n  --color-accent-tinted: #b9b9b9;\n}\n";

/** Theming modes (doc 08 §3.3). `dark` is the product default. */
export type TokenMode = "dark" | "light";
export const TOKEN_MODES: readonly TokenMode[] = ["dark", "light"];

/** The 9 accent selections (doc 08 §3.3). `orange` is the product default. */
export type AccentId = "blue" | "cyan" | "gray" | "green" | "orange" | "pink" | "red" | "violet" | "yellow";
export const ACCENTS: readonly AccentId[] = ["blue", "cyan", "gray", "green", "orange", "pink", "red", "violet", "yellow"];

/** Every semantic CSS custom property name emitted in the `:root` block. */
export type SemanticTokenName = "--blur-backdrop" | "--blur-backdrop-small" | "--color-accent" | "--color-accent-bright" | "--color-accent-icon" | "--color-accent-strong" | "--color-accent-tinted" | "--color-content-bright" | "--color-content-on-accent" | "--color-content-primary" | "--color-content-secondary" | "--color-content-tertiary" | "--color-eq-1" | "--color-eq-2" | "--color-eq-3" | "--color-eq-4" | "--color-eq-5" | "--color-eq-6" | "--color-eq-7" | "--color-record" | "--color-stroke-border" | "--color-stroke-icon" | "--color-surface" | "--color-surface-chrome" | "--color-surface-deep" | "--color-surface-hover" | "--color-surface-muted" | "--color-surface-panel" | "--color-surface-raised" | "--color-surface-selected" | "--font-family-sans" | "--font-family-time" | "--font-size-body" | "--font-size-caption" | "--font-size-display" | "--font-size-label" | "--font-size-micro" | "--font-size-section" | "--font-size-sidebar" | "--font-size-time" | "--font-size-title" | "--font-weight-bold" | "--font-weight-extrabold" | "--font-weight-medium" | "--font-weight-regular" | "--font-weight-semibold" | "--motion-duration-fast" | "--motion-duration-slow" | "--motion-duration-step" | "--opacity-chrome" | "--opacity-disabled" | "--opacity-emphasis" | "--opacity-muted" | "--opacity-panel" | "--opacity-scrim" | "--opacity-scrim-strong" | "--opacity-scrim-subtle" | "--opacity-swatch-rest" | "--radius-app" | "--radius-card" | "--radius-card-lg" | "--radius-control" | "--radius-hairline" | "--radius-inset" | "--radius-menu" | "--radius-panel" | "--radius-ring" | "--radius-thumbnail" | "--radius-window" | "--shadow-card" | "--shadow-overlay" | "--space-grid" | "--space-inset" | "--space-item-pitch" | "--space-track-inset" | "--stroke-icon" | "--stroke-icon-max" | "--stroke-icon-strong" | "--stroke-ring" | "--stroke-window";
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
  "--color-eq-1",
  "--color-eq-2",
  "--color-eq-3",
  "--color-eq-4",
  "--color-eq-5",
  "--color-eq-6",
  "--color-eq-7",
  "--color-record",
  "--color-stroke-border",
  "--color-stroke-icon",
  "--color-surface",
  "--color-surface-chrome",
  "--color-surface-deep",
  "--color-surface-hover",
  "--color-surface-muted",
  "--color-surface-panel",
  "--color-surface-raised",
  "--color-surface-selected",
  "--font-family-sans",
  "--font-family-time",
  "--font-size-body",
  "--font-size-caption",
  "--font-size-display",
  "--font-size-label",
  "--font-size-micro",
  "--font-size-section",
  "--font-size-sidebar",
  "--font-size-time",
  "--font-size-title",
  "--font-weight-bold",
  "--font-weight-extrabold",
  "--font-weight-medium",
  "--font-weight-regular",
  "--font-weight-semibold",
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
