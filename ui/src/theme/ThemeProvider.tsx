// ThemeProvider — card 4.A2, doc 08 §3.3 theming mechanics.
//
// Holds the user's theme selections (mode setting + accent id) and applies
// them as `data-theme` / `data-accent` attributes on the root <html> element.
// The generated CSS custom properties (src/tokens/generated/tokens.css)
// cascade off those attributes — no JS style recalculation beyond the
// attribute set (doc 08 §3.3). Switching is attribute-only by design.
//
// Persistence and the settings-schema integration ("Accent color" setting,
// doc 07 / doc 08 §3.3) land with the settings cards — the settings→theme
// CONSUMPTION seam lives in ./settingsThemeSync (mounted by the settings
// screens; card 4.E2). w26.f08 (e03-F1 / e19-F1): the provider additionally
// BOOTSTRAPS its mode from the persisted `interface.theme` override via that
// seam's synchronous read, so every route starts on the user's saved mode
// (previously the default below won on all fresh loads outside /settings).
// The accent has no persisted setting yet (no schema key — recorded gap in
// settingsThemeSync.ts), so it honestly defaults to the product default
// (orange, the W25 Figma redesign accent; the Solstice gold default
// is retired). The base stylesheet for the skin is lumen.css, imported
// below so any host mounting <ThemeProvider> gets the whole theme layer.

import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { ACCENTS, type AccentId, type TokenMode } from "../tokens/generated/tokens";
// Base stylesheet of the Lumen skin (wave 26 theme lane): root type,
// scrollbars, selection, focus floor, shared panel/row/separator
// primitives. Composes the generated tokens; replaces solstice.css.
import "./lumen.css";
import {
  ThemeContext,
  type ThemeContextValue,
  type ThemeModeSetting,
} from "./context";
// w26.f08: synchronous bootstrap read of the persisted interface.theme
// override (same persistence seam the settings screens hydrate from).
import { readPersistedThemeMode } from "./settingsThemeSync";

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
  // Lazy init (w26.f08, e03-F1/e19-F1): the persisted `interface.theme`
  // override — when valid — wins over the dark default on the FIRST render
  // of every route, so a saved LIGHT/system choice never boots dark. The
  // read is synchronous (localStorage seam) and total (absent/corrupt store
  // → null → product default); it never throws and never guesses.
  const [modeSetting, setModeSettingState] = useState<ThemeModeSetting>(
    () => readPersistedThemeMode() ?? "dark",
  );
  const [accent, setAccentState] = useState<AccentId>("orange");
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
  // set on the root element, nothing else). LAYOUT effect (w26.f08): it
  // commits before the browser paints, so the very first painted frame of
  // the app already carries the hydrated/resolved mode — a passive effect
  // could flash the pre-React dark default (:root) for one frame.
  useLayoutEffect(() => {
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
