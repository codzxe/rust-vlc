import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { App } from "./App";
import { ClientProvider } from "./client/ClientProvider";
import { MockTransport } from "./client/transport";
import { ThemeProvider } from "./theme/ThemeProvider";
// Generated semantic tokens (card 4.A2): the CSS custom-property cascade the
// whole UI consumes. Import stays inside ui/src (boundary-imports rule).
import "./tokens/generated/tokens.css";
// Card 4.F4: global token-driven :focus-visible floor for focusables that
// live outside the B1 primitives (which carry their own identical rule),
// and the body-level surface paint floor (never raw browser white).
import "./a11y/focus.css";
import "./a11y/page.css";

const rootElement = document.getElementById("root");
if (rootElement === null) {
  throw new Error("index.html is missing its #root mount point");
}

// Scaffold wiring: the browser dev shell uses the mock transport. The real
// hosts (apps/desktop, apps/web) inject their own CommandTransport here.
createRoot(rootElement).render(
  <StrictMode>
    <ThemeProvider>
      <ClientProvider transport={new MockTransport()}>
        <BrowserRouter>
          <App />
        </BrowserRouter>
      </ClientProvider>
    </ThemeProvider>
  </StrictMode>,
);
