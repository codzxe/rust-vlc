import { render, screen } from "@testing-library/react";
import { ClientProvider } from "./client/ClientProvider";
import { MockTransport } from "./client/transport";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { App } from "./App";
import { resetOnboardingForTests } from "./screens/onboarding/onboardingState";

function renderAt(path: string): void {
  render(
    <ClientProvider transport={new MockTransport()}>
      <MemoryRouter initialEntries={[path]}>
        <App />
      </MemoryRouter>
    </ClientProvider>,
  );
}

// Route smoke over the doc 08 §1.6 inventory — every route now renders a
// real screen (Solstice app-shell workstream, 2026-11-08). On a fresh run
// the first-run gate owns / and the player routes until the wizard
// completes/skips (card 4.F1).
const ONBOARDING_DONE = JSON.stringify({
  complete: true,
  choices: { telemetry: false, updateChecks: false, onlineSuggestions: false },
});

describe("App routing skeleton (doc 08 §1.6)", () => {
  it("first run: / redirects through /player into the onboarding gate", () => {
    renderAt("/");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Welcome to Lumen");
  });

  it("renders the music-player route as the real player in music variant", () => {
    window.localStorage.setItem("lumen.onboarding.v1", ONBOARDING_DONE);
    resetOnboardingForTests(); // re-hydrate the module store from localStorage
    renderAt("/player/music");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Player");
    expect(document.querySelector(".lumen-player")).toHaveAttribute("data-variant", "music");
    window.localStorage.clear();
    resetOnboardingForTests(); // leave a fresh run for the later route tests
  });

  it("renders the settings route via the schema-generated framework (4.E1); unknown areas redirect to the first schema area", async () => {
    renderAt("/settings/audio"); // "audio" has no schema entries yet (4.E2–4.E4)
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent("Settings");
    expect(screen.getByRole("button", { name: "Global" })).toHaveAttribute("aria-current", "page");
  });

  it("renders the onboarding route (real 4.F1 wizard)", () => {
    renderAt("/onboarding");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Welcome to Lumen");
  });

  it("renders unknown paths as the not-found screen", () => {
    renderAt("/definitely-not-a-route");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Lost the thread");
  });
});
