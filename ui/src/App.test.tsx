import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { ClientProvider } from "./client/ClientProvider";
import { MockTransport } from "./client/transport";
import { MemoryRouter } from "react-router-dom";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "./App";
import { resetOnboardingForTests } from "./screens/onboarding/onboardingState";
// w26.f08: the app-level theme hydration cases compose the providers exactly
// like main.tsx does (ThemeProvider is the mount that owns the bootstrap).
import { ThemeProvider } from "./theme/ThemeProvider";

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
// screen (app-shell workstream, 2026-11-08). w26.i20 (h20 top-5 #3): the
// first-run gate wraps EVERY AppShell route (the whole product, contract
// §1) — seeded profiles (lumen.onboarding.v1 complete, the e2e/dev-shots
// shape) render real screens; ?onboarding=1 is the per-navigation bypass.
const ONBOARDING_DONE = JSON.stringify({
  complete: true,
  choices: { telemetry: false, updateChecks: false, onlineSuggestions: false },
});

/** Seed a completed wizard (the e2e a11y/dev-shots shape) + re-hydrate the
 * module store, so the test exercises its screen — not the first-run gate. */
function seedOnboardingDone(): void {
  window.localStorage.setItem("lumen.onboarding.v1", ONBOARDING_DONE);
  resetOnboardingForTests(); // re-hydrate the module store from localStorage
}

/** Drop the seed + re-hydrate so later tests start on a fresh run again. */
function clearOnboardingSeed(): void {
  window.localStorage.clear();
  resetOnboardingForTests();
}

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

  // w26.k20 (j20 P2-3): non-player routes are React.lazy chunks, so their
  // first paint suspends for a microtask — assertions on lazy screens await
  // findBy*/waitFor (same matchers, same strength); player + onboarding stay
  // eager (home screen + first-run gate) and keep synchronous getbys.
  it("w26.i20 (h20 #3): first run, the gate guards the WHOLE app — every shell route redirects into the wizard", async () => {
    // Fresh profile (no onboarding store): pre-onboarding, the product IS the
    // wizard — settings, playlists, dialogs, working screens and the 404 are
    // no longer reachable by typing a URL (h20 walked them live; the gate
    // used to wrap PlayerRoute only).
    for (const path of [
      "/player",
      "/player/music",
      "/miniplayer",
      "/playlist",
      "/media-info",
      "/settings",
      "/settings/global",
      "/settings/not-a-real-area",
      "/discovery",
      "/convert",
      "/keyboard-shortcuts",
      "/open-network-stream",
      "/definitely-not-a-route",
    ]) {
      cleanup();
      renderAt(path);
      expect(screen.getByRole("heading", { level: 1 }), `fresh run at ${path}`).toHaveTextContent(
        "Welcome to Lumen",
      );
    }
    // /onboarding itself renders the wizard directly (no gate loop).
    cleanup();
    renderAt("/onboarding");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Welcome to Lumen");
    // The ungated dev fixture keeps rendering (full-bleed route by design).
    // TokenSandbox reads the theme context — mount the same provider stack
    // main.tsx uses (ThemeProvider), per the theme-hydration cases below.
    // (w26.k20: /dev/tokens is a lazy chunk — await its first paint.)
    cleanup();
    render(
      <ThemeProvider>
        <ClientProvider transport={new MockTransport()}>
          <MemoryRouter initialEntries={["/dev/tokens"]}>
            <App />
          </MemoryRouter>
        </ClientProvider>
      </ThemeProvider>,
    );
    await waitFor(() => expect(document.querySelector(".ts-sandbox")).not.toBeNull());
  });

  it("renders the settings route via the schema-generated framework (4.E1); unknown areas redirect to the first schema area", async () => {
    // w26.i20: settings sits behind the app-wide first-run gate now — seed
    // the completed wizard (e2e/dev-shots shape) so this test exercises the
    // area redirect, not the gate.
    seedOnboardingDone();
    renderAt("/settings/not-a-real-area"); // genuinely unknown slug (audio is real since w26.b02)
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent("Settings");
    expect(screen.getByRole("button", { name: "General" })).toHaveAttribute("aria-current", "page");
    clearOnboardingSeed();
  });

  it("renders the onboarding route (real 4.F1 wizard)", () => {
    renderAt("/onboarding");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Welcome to Lumen");
  });

  it("renders unknown paths as the not-found screen", async () => {
    // w26.i20: the 404 lives inside the gated shell — seed the completed
    // wizard so the catch-all renders its real screen, not the wizard.
    // w26.k20: the not-found screen is a lazy chunk → findBy.
    seedOnboardingDone();
    renderAt("/definitely-not-a-route");
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent("Page not found");
    clearOnboardingSeed();
  });

  it("completed profile: / lands on the player home — the wizard never shows again", () => {
    window.localStorage.setItem("lumen.onboarding.v1", ONBOARDING_DONE);
    resetOnboardingForTests(); // re-hydrate the module store from localStorage
    renderAt("/");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Player");
    window.localStorage.clear();
    resetOnboardingForTests(); // leave a fresh run for the later route tests
  });

  it("?onboarding=1 bypasses the first-run gate for tests/screenshots, per navigation only", async () => {
    // Fresh run (no persisted store): the param satisfies the gate…
    renderAt("/player?onboarding=1");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Player");
    // …on ANY shell route now (w26.i20 moved the gate to the shell layout,
    // so the bypass covers the whole product, not just the player family).
    // (w26.k20: settings + not-found are lazy chunks → findBy.)
    cleanup();
    renderAt("/settings/global?onboarding=1");
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent("Settings");
    cleanup();
    renderAt("/definitely-not-a-route?onboarding=1");
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent("Page not found");
    // …and writes nothing to the onboarding store: the next param-less
    // navigation is gated again — player family AND the rest of the app.
    cleanup();
    renderAt("/player");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Welcome to Lumen");
    cleanup();
    renderAt("/playlist");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Welcome to Lumen");
  });
});

describe("App — persisted theme + accent hydration (w26.f08, e03-F1/e19-F1)", () => {
  /** Renders the App under the same provider stack main.tsx mounts. */
  function renderApp(path: string): void {
    render(
      <ThemeProvider>
        <ClientProvider transport={new MockTransport()}>
          <MemoryRouter initialEntries={[path]}>
            <App />
          </MemoryRouter>
        </ClientProvider>
      </ThemeProvider>,
    );
  }

  /** Seeds a persisted LIGHT theme choice (the settings screen's own shape). */
  function seedLightTheme(): void {
    window.localStorage.setItem(
      "lumen.settings.overrides.v1",
      JSON.stringify({ "interface.theme": "light" }),
    );
  }

  afterEach(() => {
    cleanup();
    window.localStorage.clear();
    resetOnboardingForTests(); // leave a fresh run for the route tests above
    delete document.documentElement.dataset.theme;
    delete document.documentElement.dataset.accent;
  });

  it("a fresh load of /player boots LIGHT when interface.theme=light is persisted (used to boot dark — e03-F1)", () => {
    window.localStorage.setItem("lumen.onboarding.v1", ONBOARDING_DONE);
    seedLightTheme();
    resetOnboardingForTests();
    renderApp("/player");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Player");
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("a fresh load of /discovery (non-settings, non-player) boots LIGHT too — every route hydrates (e19-F1)", async () => {
    // (w26.k20: discovery is a lazy chunk → findBy; hydration is asserted
    // after the screen paints, which is the honest seam anyway.)
    window.localStorage.setItem("lumen.onboarding.v1", ONBOARDING_DONE);
    seedLightTheme();
    resetOnboardingForTests();
    renderApp("/discovery");
    expect(await screen.findByRole("heading", { level: 1 })).toHaveTextContent("Discovery");
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("the full-bleed /onboarding gate route hydrates as well (no AppShell there — the provider init is the seam)", () => {
    // Fresh profile (no onboarding store) + a persisted light choice: the
    // wizard itself must render light.
    seedLightTheme();
    renderApp("/onboarding");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Welcome to Lumen");
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("a persisted accent (global.accent-color) boots with the app too — data-accent hydrates on every route (w26.f08 round 2)", () => {
    window.localStorage.setItem("lumen.onboarding.v1", ONBOARDING_DONE);
    window.localStorage.setItem(
      "lumen.settings.overrides.v1",
      JSON.stringify({ "global.accent-color": "violet" }),
    );
    resetOnboardingForTests();
    renderApp("/player");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Player");
    expect(document.documentElement.dataset.accent).toBe("violet");
    // The theme stays at its own default when only the accent is persisted.
    expect(document.documentElement.dataset.theme).toBe("dark");
  });
});
