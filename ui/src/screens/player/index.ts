// Card 4.C1 — player screen shell public surface.
export { PlayerScreen } from "./PlayerScreen";
export type { PlayerMediaContext, PlayerScreenProps } from "./PlayerScreen";
export { useChromeAutoHide, CHROME_AUTO_HIDE_DELAY_MS } from "./useChromeAutoHide";
export type { ChromeAutoHide, ChromeAutoHideOptions, ChromeVisibility } from "./useChromeAutoHide";
export { useFullscreen, FullscreenUnsupportedError } from "./useFullscreen";
export type { FullscreenController } from "./useFullscreen";
export {
  DEFAULT_KEYMAP,
  DEFAULT_KEYMAP_TOML,
  matchBindings,
  normalizeBindingKey,
  parseKeymap,
  shellActionFor,
} from "./keymap";
export type { KeyBinding, MatchableKeyboardEvent, NormalizedKey, PlayerShellAction } from "./keymap";
export {
  DEFAULT_LAYOUT,
  DEFAULT_LAYOUT_TOML,
  LayoutParseError,
  evaluateCondition,
  parseLayoutConfig,
  parseTomlSubset,
  resolveLayout,
} from "./layout";
export type {
  LayoutAnchor,
  LayoutConditionContext,
  LayoutConditionToken,
  LayoutConfig,
  LayoutMenu,
  LayoutMenuItem,
  LayoutRegion,
} from "./layout";
