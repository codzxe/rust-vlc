// useTheme — the consumer hook for the theme selections (card 4.A2).
// It lives in its own module (not next to <ThemeProvider>) so the provider
// file only exports components (react-refresh/only-export-components,
// issue #36). The context itself stays in ThemeProvider.tsx, where the
// settings-sync seam (card 4.E2) imports it from; ThemeContext's own move
// to a context module is deferred until the guarded screens that import it
// (OnboardingScreen) can follow (see issue #36 notes).

import { useContext } from "react";
import { ThemeContext, type ThemeContextValue } from "./ThemeProvider";

/** Access the theme selections (card 4.A2). Throws outside ThemeProvider. */
export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (ctx === null) {
    throw new Error("useTheme must be used inside <ThemeProvider>");
  }
  return ctx;
}
