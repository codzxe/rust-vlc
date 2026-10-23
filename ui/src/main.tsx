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
