import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { App } from "./App";
import { ClientProvider } from "./client/ClientProvider";
import { MockTransport } from "./client/transport";

const rootElement = document.getElementById("root");
if (rootElement === null) {
  throw new Error("index.html is missing its #root mount point");
}

// Scaffold wiring: the browser dev shell uses the mock transport. The real
// hosts (apps/desktop, apps/web) inject their own CommandTransport here.
createRoot(rootElement).render(
  <StrictMode>
    <ClientProvider transport={new MockTransport()}>
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </ClientProvider>
  </StrictMode>,
);
