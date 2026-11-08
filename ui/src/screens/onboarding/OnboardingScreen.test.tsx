import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactElement } from "react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import { beforeEach, describe, expect, it } from "vitest";
import { ThemeProvider } from "../../theme/ThemeProvider";
import { OnboardingEntry, OnboardingScreen, RequireOnboarding } from "./OnboardingScreen";
import {
  ONBOARDING_DEFAULTS,
  completeOnboarding,
  resetOnboardingForTests,
  setOnboardingChoice,
  useOnboardingState,
} from "./onboardingState";

// Card 4.F1 — the honest-rule proofs: a fresh run leaves every privacy flag
// false, skipping never enables anything, and only explicit opt-ins persist.

const STORAGE_KEY = "lumen.onboarding.v1";

const readStore = (): Record<string, unknown> =>
  JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? "{}") as Record<string, unknown>;

/** Location probe for redirect assertions. */
function Where(): ReactElement {
  const location = useLocation();
  return <span data-testid="where">{location.pathname}</span>;
}

const renderWizard = (): ReturnType<typeof render> =>
  render(
    <MemoryRouter initialEntries={["/onboarding"]}>
      <Where />
      <Routes>
        <Route path="/onboarding" element={<OnboardingScreen />} />
        <Route path="/player" element={<span data-testid="player-stub" />} />
      </Routes>
    </MemoryRouter>,
  );

const goToPrivacy = (): void => {
  fireEvent.click(screen.getByTestId("onboarding-next"));
};
const goToStep = (n: number): void => {
  for (let i = 0; i < n; i += 1) fireEvent.click(screen.getByTestId("onboarding-next"));
};

describe("OnboardingScreen (card 4.F1)", () => {
  beforeEach(() => {
    window.localStorage.clear();
    resetOnboardingForTests();
  });

  // --- THE AC: fresh run, every privacy flag false ---

  it("renders the welcome step with progress indication on a fresh run", () => {
    renderWizard();
    expect(screen.getByRole("heading", { name: "Welcome to Lumen" })).toBeInTheDocument();
    const progress = screen.getByTestId("onboarding-progress");
    expect(progress).toHaveAttribute("aria-label", "Onboarding step 1 of 5");
    // v2 keycap indicator: the current chip also wears its step number.
    expect(progress.querySelector('[aria-current="step"]')).toHaveTextContent("1");
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
    goToStep(2); // past appearance + shortcuts to done
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
    goToStep(3); // privacy → appearance → shortcuts → done
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
    goToPrivacy();
    const progress = screen.getByTestId("onboarding-progress");
    expect(progress).toHaveAttribute("aria-label", "Onboarding step 2 of 5");
    expect(progress.querySelector('[aria-current="step"]')).toHaveTextContent("2");
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(screen.getByRole("heading", { name: "Welcome to Lumen" })).toBeInTheDocument();
  });

  it("Back is hidden on the welcome step (geometry kept, no dead control)", () => {
    renderWizard();
    expect(screen.getByRole("button", { name: "Back" })).toHaveClass("onboarding__hidden");
  });

  it("Next through all steps reaches done, where Skip disappears and Get started appears", () => {
    renderWizard();
    goToStep(4);
    expect(screen.getByRole("heading", { name: "You are set" })).toBeInTheDocument();
    expect(screen.queryByTestId("onboarding-skip")).toBeNull();
    fireEvent.click(screen.getByTestId("onboarding-get-started"));
    expect(readStore()["complete"]).toBe(true);
  });

  it("appearance step live-previews the theme through ThemeProvider", () => {
    render(
      <ThemeProvider>
        <MemoryRouter initialEntries={["/onboarding"]}>
          <Routes>
            <Route path="/onboarding" element={<OnboardingScreen />} />
          </Routes>
        </MemoryRouter>
      </ThemeProvider>,
    );
    goToStep(2); // welcome → privacy → appearance
    fireEvent.click(screen.getByRole("button", { name: "Light" }));
    expect(document.documentElement.dataset.theme).toBe("light");
    fireEvent.click(screen.getByTestId("onboarding-accent-cyan"));
    expect(document.documentElement.dataset.accent).toBe("cyan");
    expect(screen.getByRole("button", { name: "Light" })).toHaveAttribute("aria-pressed", "true");
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

// --- tiny helpers ------------------------------------------------------------

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
