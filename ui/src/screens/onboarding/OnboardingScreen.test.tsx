import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactElement } from "react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import { beforeEach, describe, expect, it } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import { MockTransport } from "../../client/transport";
import { ThemeProvider } from "../../theme/ThemeProvider";
import { OnboardingEntry, OnboardingScreen, RequireOnboarding } from "./OnboardingScreen";
import {
  ONBOARDING_DEFAULTS,
  completeOnboarding,
  resetOnboardingForTests,
  setOnboardingChoice,
  useOnboardingState,
} from "./onboardingState";

// Card 4.F1 + W26 Figma redesign (lane w26.a12) — the 8-step flow
// (welcome → library → appearance → playback → subtitles → privacy →
// shortcuts → done). The honest-rule proofs are unchanged: a fresh run
// leaves every privacy flag false, skipping never enables anything, and
// only explicit opt-ins persist.

const STORAGE_KEY = "lumen.onboarding.v1";
const SETTINGS_KEY = "lumen.settings.overrides.v1";

const readStore = (): Record<string, unknown> =>
  JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? "{}") as Record<string, unknown>;

const readSettings = (): Record<string, unknown> =>
  JSON.parse(window.localStorage.getItem(SETTINGS_KEY) ?? "{}") as Record<string, unknown>;

/** Location probe for redirect assertions. */
function Where(): ReactElement {
  const location = useLocation();
  return <span data-testid="where">{location.pathname}</span>;
}

const renderWizard = (): ReturnType<typeof render> =>
  render(
    <ClientProvider transport={new MockTransport()}>
      <MemoryRouter initialEntries={["/onboarding"]}>
        <Where />
        <Routes>
          <Route path="/onboarding" element={<OnboardingScreen />} />
          <Route path="/player" element={<span data-testid="player-stub" />} />
        </Routes>
      </MemoryRouter>
    </ClientProvider>,
  );

const goToStep = (n: number): void => {
  for (let i = 0; i < n; i += 1) fireEvent.click(screen.getByTestId("onboarding-next"));
};

const goToPrivacy = (): void => goToStep(5); // welcome→library→appearance→playback→subtitles→privacy

describe("OnboardingScreen (card 4.F1, W26 flow)", () => {
  beforeEach(() => {
    window.localStorage.clear();
    resetOnboardingForTests();
  });

  // --- THE AC: fresh run, every privacy flag false ---

  it("renders the welcome step with progress indication on a fresh run", () => {
    renderWizard();
    expect(screen.getByRole("heading", { name: "Welcome to Lumen" })).toBeInTheDocument();
    const progress = screen.getByTestId("onboarding-progress");
    expect(progress).toHaveAttribute("aria-label", "Onboarding step 1 of 8");
    expect(progress.querySelector('[aria-current="step"]')).toBeInTheDocument();
  });

  it("fresh run: every privacy choice is OFF and none pre-checked", () => {
    renderWizard();
    goToPrivacy();
    for (const label of [
      "Send anonymous usage statistics",
      "Check for updates automatically",
      "Online suggestions",
    ]) {
      expect(screen.getByRole("switch", { name: label })).toHaveAttribute("aria-checked", "false");
    }
    expect(readStore()).toEqual({}); // nothing persisted on a fresh run
  });

  it("privacy step shows the honest off-by-default note", () => {
    renderWizard();
    goToPrivacy();
    expect(screen.getByText(/stay off unless you enable them/i)).toBeInTheDocument();
  });

  // --- skip semantics ---

  it("skip completes the wizard with every privacy flag still false", () => {
    renderWizard();
    fireEvent.click(screen.getByTestId("onboarding-skip"));
    expect(readStore()).toEqual({
      complete: true,
      choices: { telemetry: false, updateChecks: false, onlineSuggestions: false },
    });
  });

  it("skip never flips a flag the user did not touch", () => {
    renderWizard();
    goToPrivacy();
    fireEvent.click(screen.getByRole("switch", { name: "Send anonymous usage statistics" }));
    goToStep(1); // privacy → shortcuts (skip lives on every step but done)
    fireEvent.click(screen.getByTestId("onboarding-skip"));
    expect(readStore()).toEqual({
      complete: true,
      choices: { telemetry: true, updateChecks: false, onlineSuggestions: false },
    });
  });

  it("skip navigates to the player", async () => {
    renderWizard();
    fireEvent.click(screen.getByTestId("onboarding-skip"));
    await waitFor(() => expect(screen.getByTestId("where")).toHaveTextContent("/player"));
  });

  // --- opt-in persistence ---

  it("explicit opt-ins persist (and only those)", () => {
    renderWizard();
    goToPrivacy();
    fireEvent.click(screen.getByRole("switch", { name: "Send anonymous usage statistics" }));
    fireEvent.click(screen.getByRole("switch", { name: "Online suggestions" }));
    goToStep(2); // privacy → shortcuts → done
    fireEvent.click(screen.getByTestId("onboarding-get-started"));
    expect(readStore()).toEqual({
      complete: true,
      choices: { telemetry: true, updateChecks: false, onlineSuggestions: true },
    });
  });

  it("opt-in toggles round-trip through a store re-hydration", () => {
    setOnboardingChoice("updateChecks", true);
    completeOnboarding();
    resetOnboardingForTests(); // re-hydrate module state from localStorage
    const probe = renderProbe();
    expect(probe.complete).toBe(true);
    expect(probe.choices).toEqual({
      telemetry: false,
      updateChecks: true,
      onlineSuggestions: false,
    });
  });

  it("corrupt store folds back to all-off defaults (never 'on')", () => {
    window.localStorage.setItem(STORAGE_KEY, "{not json");
    resetOnboardingForTests();
    const { choices, complete } = renderProbe();
    expect(complete).toBe(false);
    expect(choices).toEqual(ONBOARDING_DEFAULTS.choices);
  });

  // --- step navigation ---

  it("Back/Next walk the steps and update the progress label", () => {
    renderWizard();
    fireEvent.click(screen.getByTestId("onboarding-next"));
    const progress = screen.getByTestId("onboarding-progress");
    expect(progress).toHaveAttribute("aria-label", "Onboarding step 2 of 8");
    expect(screen.getByRole("heading", { name: "Media library" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(screen.getByRole("heading", { name: "Welcome to Lumen" })).toBeInTheDocument();
  });

  it("Back is hidden on the welcome step (geometry kept, no dead control)", () => {
    renderWizard();
    expect(screen.getByRole("button", { name: "Back" })).toHaveClass("onboarding__hidden");
  });

  it("Next through all steps reaches done, where Skip disappears and Get started appears", () => {
    renderWizard();
    goToStep(7);
    expect(screen.getByRole("heading", { name: "You are set" })).toBeInTheDocument();
    expect(screen.queryByTestId("onboarding-skip")).toBeNull();
    fireEvent.click(screen.getByTestId("onboarding-get-started"));
    expect(readStore()["complete"]).toBe(true);
  });

  // --- library step: the real open-path funnel ---

  it("library step opens a pasted path through the one funnel (session.open)", async () => {
    const transport = new MockTransport();
    render(
      <ClientProvider transport={transport}>
        <MemoryRouter initialEntries={["/onboarding"]}>
          <Routes>
            <Route path="/onboarding" element={<OnboardingScreen />} />
          </Routes>
        </MemoryRouter>
      </ClientProvider>,
    );
    goToStep(1);
    fireEvent.change(screen.getByTestId("onboarding-open-path"), {
      target: { value: "/media/demo/big-buck-bunny.mkv" },
    });
    fireEvent.click(screen.getByTestId("onboarding-open-submit"));
    expect(transport.sent).toHaveLength(1);
    expect(transport.sent[0]).toMatchObject({
      cmd: "session.open",
      args: { source: { kind: "file", path: "/media/demo/big-buck-bunny.mkv" } },
    });
    // The mock transport answers with the typed error envelope — the
    // status line reports the honest failure.
    await waitFor(() =>
      expect(screen.getByTestId("onboarding-open-notice")).toHaveTextContent(
        /could not be opened/i,
      ),
    );
  });

  it("library step rejects an empty path with the field's error copy", () => {
    renderWizard();
    goToStep(1);
    fireEvent.click(screen.getByTestId("onboarding-open-submit"));
    expect(screen.getByText(/enter a file path or an http/i)).toBeInTheDocument();
    expect(screen.queryByTestId("onboarding-open-notice")?.textContent ?? "").toBe("");
  });

  // --- appearance step: live preview through ThemeProvider ---

  it("appearance step live-previews the theme through ThemeProvider", () => {
    render(
      <ThemeProvider>
        <ClientProvider transport={new MockTransport()}>
          <MemoryRouter initialEntries={["/onboarding"]}>
            <Routes>
              <Route path="/onboarding" element={<OnboardingScreen />} />
            </Routes>
          </MemoryRouter>
        </ClientProvider>
      </ThemeProvider>,
    );
    goToStep(2); // welcome → library → appearance
    // Dot row order is the frame's (Onboarding-1.svg: orange first — the
    // default accent — then the band set), not the generated alphabetical one.
    const accents = screen.getByTestId("onboarding-accents");
    expect([...accents.children].map((dot) => (dot as HTMLElement).dataset.accent)).toEqual([
      "orange",
      "red",
      "yellow",
      "green",
      "cyan",
      "blue",
      "violet",
      "pink",
      "gray",
    ]);
    fireEvent.click(screen.getByRole("button", { name: "Light" }));
    expect(document.documentElement.dataset.theme).toBe("light");
    fireEvent.click(screen.getByTestId("onboarding-accent-cyan"));
    expect(document.documentElement.dataset.accent).toBe("cyan");
    expect(screen.getByRole("button", { name: "Light" })).toHaveAttribute("aria-pressed", "true");
  });

  // --- playback / subtitles steps: real settings-schema writes ---

  it("playback step writes the real settings overrides store", async () => {
    renderWizard();
    goToStep(3);
    fireEvent.click(screen.getByRole("switch", { name: "Start paused" }));
    await waitFor(() => expect(readSettings()).toMatchObject({ "playback.start-paused": true }));
  });

  it("subtitles step commits the subtitle mode through the segmented control", async () => {
    renderWizard();
    goToStep(4);
    fireEvent.click(screen.getByRole("radio", { name: "Off" }));
    await waitFor(() => expect(readSettings()).toMatchObject({ "playback.subtitle-mode": "off" }));
  });

  // --- shortcuts step: the real keymap tour ---

  it("shortcuts step shows the real keymap tour and the full-list note", () => {
    renderWizard();
    goToStep(6);
    // Caps are keymap data: Space (play/pause), F (fullscreen), Shift+S
    // (snapshot) renders as Shift + S caps.
    expect(screen.getByText("Play Pause")).toBeInTheDocument();
    expect(screen.getByText("Space")).toBeInTheDocument();
    expect(screen.getByText("F")).toBeInTheDocument();
    expect(screen.getAllByText("Shift")).toHaveLength(2); // Shift+S / Shift+R combos
    expect(screen.getByText("Snapshot")).toBeInTheDocument();
    expect(screen.getByText("Find the full list in the menu.")).toBeInTheDocument();
  });

  // --- first-run gating ---

  it("gate: /player redirects to /onboarding while incomplete", async () => {
    render(
      <MemoryRouter initialEntries={["/player"]}>
        <Where />
        <Routes>
          <Route
            path="/player"
            element={
              <RequireOnboarding>
                <span data-testid="player-stub" />
              </RequireOnboarding>
            }
          />
          <Route path="/onboarding" element={<OnboardingEntry />} />
        </Routes>
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getByTestId("where")).toHaveTextContent("/onboarding"));
    expect(screen.getByRole("heading", { name: "Welcome to Lumen" })).toBeInTheDocument();
  });

  it("gate: after completion, /player renders and /onboarding bounces back", async () => {
    completeOnboarding();
    const utils = render(
      <MemoryRouter initialEntries={["/player"]}>
        <Where />
        <Routes>
          <Route
            path="/player"
            element={
              <RequireOnboarding>
                <span data-testid="player-stub" />
              </RequireOnboarding>
            }
          />
          <Route path="/onboarding" element={<OnboardingEntry />} />
        </Routes>
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getByTestId("player-stub")).toBeInTheDocument());
    utils.unmount();
    render(
      <MemoryRouter initialEntries={["/onboarding"]}>
        <Where />
        <Routes>
          <Route path="/onboarding" element={<OnboardingEntry />} />
          <Route path="/player" element={<span data-testid="player-stub" />} />
        </Routes>
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getByTestId("where")).toHaveTextContent("/player"));
  });
});

// --- tiny helpers -----------------------------------------------------------

interface ProbeSnapshot {
  complete: boolean;
  choices: Record<string, boolean>;
}

/** Renders the reactive store read so re-hydration is observable. */
function Probe(): ReactElement {
  const { complete, choices } = useOnboardingState();
  return (
    <span
      data-testid="onboarding-probe"
      data-complete={String(complete)}
      data-telemetry={String(choices.telemetry)}
      data-update-checks={String(choices.updateChecks)}
      data-online-suggestions={String(choices.onlineSuggestions)}
    />
  );
}

function renderProbe(): ProbeSnapshot {
  render(<Probe />);
  const probe = screen.getByTestId("onboarding-probe");
  return {
    complete: probe.dataset.complete === "true",
    choices: {
      telemetry: probe.dataset.telemetry === "true",
      updateChecks: probe.dataset.updateChecks === "true",
      onlineSuggestions: probe.dataset.onlineSuggestions === "true",
    },
  };
}
