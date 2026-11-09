// useTheme — the consumer hook for the theme selections (card 4.A2).
// It lives in its own module (not next to <ThemeProvider>) and reads the
// context from ./context.ts so neither file mixes component and
// non-component exports (react-refresh/only-export-components, issue #36).

import { useContext } from "react";
import { ThemeContext, type ThemeContextValue } from "./context";

/** Access the theme selections (card 4.A2). Throws outside ThemeProvider. */
export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (ctx === null) {
    throw new Error("useTheme must be used inside <ThemeProvider>");
  }
  return ctx;
}
