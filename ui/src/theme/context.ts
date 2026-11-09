// ThemeContext — the React context behind <ThemeProvider> (card 4.A2).
//
// It lives in its own module (issue #36) so ThemeProvider.tsx exports only
// the component (react-refresh/only-export-components) and the consumer-side
// seams (useTheme, settingsThemeSync) can import the context without
// dragging in the provider file. ThemeProvider.tsx re-exports it for
// compatibility with importers that still address it there.

import { createContext } from "react";
import type { AccentId, TokenMode } from "../tokens/generated/tokens";

/** What the user picks: a fixed mode, or follow the OS. */
export type ThemeModeSetting = TokenMode | "auto";

export interface ThemeContextValue {
  /** The user's selection ("auto" follows the OS). */
  readonly modeSetting: ThemeModeSetting;
  /** What actually renders (auto resolved against the OS preference). */
  readonly resolvedMode: TokenMode;
  /** The active accent (one of the 9, doc 08 §3.3). */
  readonly accent: AccentId;
  setModeSetting(setting: ThemeModeSetting): void;
  setAccent(accent: AccentId): void;
}

// Exported for the settings-consumption seam (card 4.E2): the sync hook
// reads the context optionally so settings screens render (inert) even
// without a ThemeProvider host (tests, embeds).
export const ThemeContext = createContext<ThemeContextValue | null>(null);
