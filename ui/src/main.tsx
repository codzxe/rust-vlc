// Bundled type FIRST (assets/fonts/fonts.css): the face must be loaded
// before any component paints — the visual baselines pin Inter's
// glyphs, and font-display:block in the @font-face holds first paint
// until it is ready (the host-parity fix, 2026-11-13).
import "./assets/fonts/fonts.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { App } from "./App";
import { ClientProvider } from "./client/ClientProvider";
// App-layer transport wiring (issue #3 — the 4.H1 handoff a): ONE host
// probe decides the transport. Inside the Lumen desktop shell the
// webview host surface is present and the app runs on the REAL
// TauriTransport — every command through the shell's single
// `lumen_dispatch` command, engine events on `lumen://event`. In a
// plain browser (dev shell, visual harness) the DemoTransport
// simulation drives the UI instead: a friendly simulated engine that
// speaks the real 1.D4 wire protocol, badged "Demo" in the chrome.
// The choice is announced once via console.debug — never silent.
// MockTransport stays exported from ./client/transport for tests that
// want the honest feature-not-exposed envelope.
import { selectAppTransport } from "./client/app-transport";
import { setActiveLocale } from "./i18n/messages";
import { ThemeProvider } from "./theme/ThemeProvider";
// Generated semantic tokens (card 4.A2): the CSS custom-property cascade the
// whole UI consumes. Import stays inside ui/src (boundary-imports rule).
import "./tokens/generated/tokens.css";
// Lumen base stylesheet (W25 redesign, wave 26): root type, scrollbars,
// selection, focus floor, shared panel/row/separator primitives.
// Composes the semantic tokens — never overrides them. (Also imported by
// <ThemeProvider> so the theme layer is self-contained; Vite dedupes.)
import "./theme/lumen.css";
// Card 4.F4: global token-driven :focus-visible floor for focusables that
// live outside the B1 primitives (which carry their own identical rule),
// and the body-level surface paint floor (never raw browser white).
import "./a11y/focus.css";
import "./a11y/page.css";

const rootElement = document.getElementById("root");
if (rootElement === null) {
  throw new Error("index.html is missing its #root mount point");
}

// Card 4.H3 debug hook: `?locale=pseudo` renders the deterministic pseudo
// locale (i18n/pseudo.ts) for clipping/hardcode proofs. This is the ONLY
// supported override point — the visual harness (G1/G2) passes the query
// param from outside, never editing harness internals. Any other tag is
// ignored (falls back to en-US, doc 07 §6.3).
const requestedLocale = new URLSearchParams(window.location.search).get("locale");
if (requestedLocale !== null) {
  setActiveLocale(requestedLocale);
}

// The app-layer transport choice (see the import note above): probe the
// host once, log the decision once, run the whole UI on the result.
const appTransport = selectAppTransport();

createRoot(rootElement).render(
  <StrictMode>
    <ThemeProvider>
      <ClientProvider transport={appTransport.transport}>
        <BrowserRouter>
          <App />
        </BrowserRouter>
      </ClientProvider>
    </ThemeProvider>
  </StrictMode>,
);
