// ThemeProvider — card 4.A2, doc 08 §3.3 theming mechanics.
//
// Holds the user's theme selections (mode setting + accent id) and applies
// them as `data-theme` / `data-accent` attributes on the root <html> element.
// The generated CSS custom properties (src/tokens/generated/tokens.css)
// cascade off those attributes — no JS style recalculation beyond the
// attribute set (doc 08 §3.3). Switching is attribute-only by design.
//
// Persistence and the settings-schema integration ("Accent color" setting,
// doc 07 / doc 08 §3.3) land with the settings cards — this provider is the
// rendering-side seam only, honestly defaulting to the product defaults
// (dark + yellow, the Solstice Keyvox gold).

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { ACCENTS, type AccentId, type TokenMode } from "../tokens/generated/tokens";

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

function systemPrefersLight(): boolean {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return false; // no OS signal → dark (the product default, doc 08 §3.3)
  }
  return window.matchMedia("(prefers-color-scheme: light)").matches;
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [modeSetting, setModeSettingState] = useState<ThemeModeSetting>("dark");
  const [accent, setAccentState] = useState<AccentId>("yellow");
  const [systemLight, setSystemLight] = useState<boolean>(systemPrefersLight);

  // Track the OS preference only while the user is in "auto".
  useEffect(() => {
    if (modeSetting !== "auto" || typeof window === "undefined") {
      return;
    }
    if (typeof window.matchMedia !== "function") {
      return;
    }
    const query = window.matchMedia("(prefers-color-scheme: light)");
    const onChange = (event: MediaQueryListEvent) => setSystemLight(event.matches);
    query.addEventListener("change", onChange);
    setSystemLight(query.matches);
    return () => query.removeEventListener("change", onChange);
  }, [modeSetting]);

  const resolvedMode: TokenMode = modeSetting === "auto" ? (systemLight ? "light" : "dark") : modeSetting;

  // The single side effect that does the theming (doc 08 §3.3: attribute
  // set on the root element, nothing else).
  useEffect(() => {
    const root = document.documentElement;
    root.dataset.theme = resolvedMode;
    root.dataset.accent = accent;
  }, [resolvedMode, accent]);

  const setModeSetting = useCallback((setting: ThemeModeSetting) => {
    setModeSettingState(setting);
  }, []);

  const setAccent = useCallback((next: AccentId) => {
    if (!ACCENTS.includes(next)) {
      throw new Error(`unknown accent "${next}" — must be one of: ${ACCENTS.join(", ")}`);
    }
    setAccentState(next);
  }, []);

  const value = useMemo<ThemeContextValue>(
    () => ({ modeSetting, resolvedMode, accent, setModeSetting, setAccent }),
    [modeSetting, resolvedMode, accent, setModeSetting, setAccent],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

/** Access the theme selections (card 4.A2). Throws outside ThemeProvider. */
export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (ctx === null) {
    throw new Error("useTheme must be used inside <ThemeProvider>");
  }
  return ctx;
}
