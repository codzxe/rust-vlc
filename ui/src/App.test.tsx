import { cleanup, render, screen } from "@testing-library/react";
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

// Route smoke over the doc 08 §1.6 inventory — every route renders a real
// screen (app-shell workstream, 2026-11-08). On a fresh run the first-run
// gate owns / and the player routes until the wizard completes/skips
// (card 4.F1); ?onboarding=1 is the per-navigation test bypass of that gate.
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
    renderAt("/settings/not-a-real-area"); // genuinely unknown slug (audio is real since w26.b02)
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

  it("completed profile: / lands on the player home — the wizard never shows again", () => {
    window.localStorage.setItem("lumen.onboarding.v1", ONBOARDING_DONE);
    resetOnboardingForTests(); // re-hydrate the module store from localStorage
    renderAt("/");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Player");
    window.localStorage.clear();
    resetOnboardingForTests(); // leave a fresh run for the later route tests
  });

  it("?onboarding=1 bypasses the first-run gate for tests/screenshots, per navigation only", () => {
    // Fresh run (no persisted store): the param satisfies the gate…
    renderAt("/player?onboarding=1");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Player");
    // …and writes nothing to the onboarding store: the next param-less
    // navigation is gated again.
    cleanup();
    renderAt("/player");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Welcome to Lumen");
  });
});
