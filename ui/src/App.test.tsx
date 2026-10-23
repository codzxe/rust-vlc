import { render, screen } from "@testing-library/react";
import { ClientProvider } from "./client/ClientProvider";
import { MockTransport } from "./client/transport";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { App } from "./App";

function renderAt(path: string): void {
  render(
    <ClientProvider transport={new MockTransport()}>
      <MemoryRouter initialEntries={[path]}>
        <App />
      </MemoryRouter>
    </ClientProvider>,
  );
}

// Route smoke over the doc 08 §1.6 skeleton: each design route renders its
// honest placeholder heading.
describe("App routing skeleton (doc 08 §1.6)", () => {
  it("redirects / to the player screen", () => {
    renderAt("/");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Player");
  });

  it("renders the music-player route", () => {
    renderAt("/player/music");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Music player");
  });

  it("renders the settings route via the schema-generated framework (4.E1); unknown areas redirect to the first schema area", async () => {
    renderAt("/settings/audio"); // "audio" has no schema entries yet (4.E2–4.E4)
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent("Settings");
    expect(screen.getByRole("button", { name: "Global" })).toHaveAttribute("aria-current", "page");
  });

  it("renders the onboarding route", () => {
    renderAt("/onboarding");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Onboarding");
  });

  it("renders unknown paths as the not-found screen", () => {
    renderAt("/definitely-not-a-route");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("not found");
  });
});
