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
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { ACCENTS, type AccentId, type TokenMode } from "../tokens/generated/tokens";
import {
  ThemeContext,
  type ThemeContextValue,
  type ThemeModeSetting,
} from "./context";

// Compatibility re-exports (issue #36): the context, its value type and the
// mode-setting type moved to ./context.ts so this file exports only the
// component (react-refresh/only-export-components). Importers that still
// address them here — e.g. the guarded screens/ — keep working unchanged;
// new code imports from ./context directly.
export { ThemeContext } from "./context";
export type { ThemeContextValue, ThemeModeSetting } from "./context";

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

// useTheme lives in ./useTheme.ts and the context in ./context.ts (issue
// #36): this file exports only the component
// (react-refresh/only-export-components).
