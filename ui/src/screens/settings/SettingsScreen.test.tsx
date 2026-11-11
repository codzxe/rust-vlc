import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import { MockTransport } from "../../client/transport";
import { SettingsScreen } from "./SettingsScreen";

const STORAGE_KEY = "lumen.settings.overrides.v1";

// SettingsScreen reads :area via useParams — mount it through a real
// route. Issue #3 wiring: the screen now consumes the client context
// (the desktop config seam), so every mount supplies one — the honest
// MockTransport (config.* answers feature-not-exposed, exactly the
// browser-fallback behavior these jsdom tests exercise).
const renderAt = (path: string) =>
  render(
    <ClientProvider transport={new MockTransport()}>
      <MemoryRouter initialEntries={[path]}>
        <Routes>
          <Route path="/settings/:area" element={<SettingsScreen />} />
          {/* Close-to-player stub: the window's ✕ routes to /player. */}
          <Route path="/player" element={<div data-testid="player-home" />} />
        </Routes>
      </MemoryRouter>
    </ClientProvider>,
  );

const field = (key: string): HTMLElement | null => document.querySelector(`[data-setting="${key}"]`);

describe("SettingsScreen shell (card 4.E1)", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("renders the schema-generated area navigation", () => {
    renderAt("/settings/interface");
    // The b02 registry = exactly the 13 Figma areas (App 4 / Media 5 / System 4).
    for (const area of [
      "Global",
      "Interface",
      "Privacy",
      "Controls",
      "Playback",
      "Video",
      "Audio",
      "Subtitles",
      "Library",
      "Network",
      "Performance",
      "Recording",
      "Extensions",
    ]) {
      expect(screen.getByRole("button", { name: area })).toBeInTheDocument();
    }
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument();
  });

  it("renders the floating window shell (W26 §D2: top bar + search pill + window title)", () => {
    const { container } = renderAt("/settings/interface");
    expect(container.querySelector(".settings-window")).not.toBeNull();
    expect(screen.getByPlaceholderText("Search settings")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Close settings" })).toBeInTheDocument();
    // The Solstice backdrop veil is gone for good — the window floats on the
    // plain app surface (assertion replaces the old data-backdrop test 1:1:
    // it fails if any tier ever comes back).
    expect(document.documentElement.dataset.backdrop).toBeUndefined();
  });

  it("anchors the column with the area's window title in the top bar (W26 §D2)", () => {
    renderAt("/settings/interface");
    expect(screen.getByRole("heading", { name: "Interface Settings" })).toBeInTheDocument();
  });

  it("closes the window back to the player via the top-bar ✕", async () => {
    renderAt("/settings/interface");
    fireEvent.click(screen.getByRole("button", { name: "Close settings" }));
    await waitFor(() => expect(screen.getByTestId("player-home")).toBeInTheDocument());
  });

  it("segregates advanced entries behind the in-flow Advanced Options disclosure", () => {
    renderAt("/settings/interface");
    const toggle = screen.getByRole("button", { name: "Advanced Options" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByLabelText("Minimum window width")).toBeNull();
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByLabelText("Minimum window width")).toBeInTheDocument();
  });

  it("redirects an unknown area to the first schema area", async () => {
    renderAt("/settings/nope");
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Global" })).toHaveAttribute("aria-current", "page"),
    );
  });

  it("canonicalises retired deep-link slugs to their merged home (b02 registry)", async () => {
    renderAt("/settings/hotkeys");
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Controls" })).toHaveAttribute("aria-current", "page"),
    );
  });

  it("renders schema fields with the right primitives per type", () => {
    renderAt("/settings/interface");
    expect(screen.getByRole("switch", { name: "Start maximized" })).toBeInTheDocument();
    expect(screen.getByRole("radiogroup", { name: "Theme" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "Dark" })).toBeInTheDocument();
    const width = screen.getByLabelText("Window width") as HTMLInputElement;
    expect(width).toHaveValue(1280);
    // NumberField receives the schema range (min 480 / max 3840) for
    // stepper clamping; the input itself is a native number control.
    expect(width).toHaveAttribute("type", "number");
  });

  it("hides feature-gated unexposed entries (playback.hw-decode-policy)", () => {
    const { container } = renderAt("/settings/playback");
    expect(container.querySelector('[data-setting="playback.hw-decode-policy"]')).toBeNull();
  });

  it("shows the apply bar when dirty; apply persists and clears it", () => {
    renderAt("/settings/interface");
    expect(screen.queryByTestId("settings-applybar")).toBeNull();
    fireEvent.click(screen.getByRole("switch", { name: "Start maximized" }));
    expect(screen.getByTestId("settings-applybar")).toHaveTextContent("Unsaved changes (1)");
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    expect(screen.queryByTestId("settings-applybar")).toBeNull();
    expect(JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? "{}")).toEqual({
      "interface.start-maximized": true,
    });
  });

  it("revert discards draft edits without persisting", () => {
    renderAt("/settings/interface");
    fireEvent.click(screen.getByRole("switch", { name: "Remember window geometry" }));
    fireEvent.click(screen.getByRole("button", { name: "Revert" }));
    expect(screen.queryByTestId("settings-applybar")).toBeNull();
    expect((screen.getByRole("switch", { name: "Remember window geometry" }) as HTMLElement).getAttribute("aria-checked")).toBe("true");
    expect(window.localStorage.getItem(STORAGE_KEY)).toBeNull();
  });

  it("badges restart-required fields while changed and after apply", () => {
    renderAt("/settings/interface");
    fireEvent.click(screen.getByRole("switch", { name: "Start maximized" }));
    expect(field("interface.start-maximized")).toHaveTextContent("Restart required");
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    // still flagged: the applied change needs a restart to take effect
    expect(field("interface.start-maximized")).toHaveTextContent("Restart required");
    expect(screen.getByTestId("settings-restart-notice")).toHaveTextContent("restart");
  });

  it("searches across areas from another area's screen", () => {
    renderAt("/settings/global");
    fireEvent.change(screen.getByPlaceholderText("Search settings"), { target: { value: "window width" } });
    const results = screen.getByTestId("settings-search-results");
    expect(results).toHaveTextContent("Window width");
    expect(results).toHaveTextContent("Interface"); // cross-area hit
    expect(screen.queryByRole("radiogroup", { name: "Language" })).toBeNull(); // current screen hidden while searching
  });

  it("shows an honest no-results state", () => {
    renderAt("/settings/global");
    fireEvent.change(screen.getByPlaceholderText("Search settings"), { target: { value: "zzzz" } });
    expect(screen.getByTestId("settings-search-empty")).toHaveTextContent("No settings match your search");
  });

  it("confirms before navigating away with unsaved changes (4.B4 Dialog)", async () => {
    renderAt("/settings/interface");
    fireEvent.click(screen.getByRole("switch", { name: "Start maximized" }));
    fireEvent.click(screen.getByRole("button", { name: "Privacy" }));
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("Discard unsaved changes?");
    fireEvent.click(screen.getByRole("button", { name: "Stay" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByRole("button", { name: "Interface" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByTestId("settings-applybar")).toBeInTheDocument(); // edits kept
  });

  it("discarding the dialog reverts edits and navigates", async () => {
    renderAt("/settings/interface");
    fireEvent.click(screen.getByRole("switch", { name: "Start maximized" }));
    fireEvent.click(screen.getByRole("button", { name: "Privacy" }));
    fireEvent.click(await screen.findByRole("button", { name: "Discard changes" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Privacy" })).toHaveAttribute("aria-current", "page"),
    );
    expect(screen.queryByTestId("settings-applybar")).toBeNull(); // reverted
    expect(window.localStorage.getItem(STORAGE_KEY)).toBeNull(); // never persisted
  });

  it("navigates without a dialog when clean", async () => {
    renderAt("/settings/interface");
    fireEvent.click(screen.getByRole("button", { name: "Privacy" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Privacy" })).toHaveAttribute("aria-current", "page"));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByRole("switch", { name: "Share anonymous usage data" })).toBeInTheDocument();
  });

  it("resets a single field to its schema default", () => {
    renderAt("/settings/interface");
    const width = screen.getByLabelText("Window width") as HTMLInputElement;
    fireEvent.change(width, { target: { value: "800" } });
    expect(width).toHaveValue(800);
    const resetButtons = screen.getAllByRole("button", { name: "Reset to default" });
    expect(resetButtons.length).toBeGreaterThan(0);
    fireEvent.click(resetButtons[0] as HTMLElement);
    expect(width).toHaveValue(1280); // product.toml window.width
  });

  it("resets the whole screen to schema defaults (vs applied values)", () => {
    renderAt("/settings/interface");
    fireEvent.click(screen.getByRole("switch", { name: "Start maximized" }));
    fireEvent.click(screen.getByRole("button", { name: "Apply" })); // persisted: true
    fireEvent.click(screen.getByRole("button", { name: "Reset all to defaults" }));
    expect((screen.getByRole("switch", { name: "Start maximized" }) as HTMLElement).getAttribute("aria-checked")).toBe("false");
    expect(screen.getByLabelText("Window height")).toHaveValue(720);
    // reset is a draft edit vs the applied value — dirty again; the restart
    // badge stays: the APPLIED value still needs a restart until re-applied.
    expect(screen.getByTestId("settings-applybar")).toHaveTextContent("Unsaved changes (1)");
    expect(field("interface.start-maximized")).toHaveTextContent("Restart required");
  });
});
