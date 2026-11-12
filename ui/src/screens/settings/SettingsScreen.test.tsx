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
    // w26.f03 (e05/e06 F6): the first App row is labeled "General" — the
    // Figma sidebar copy (area id + window title stay `global`/
    // "Global Settings", tools-settings §D2 pitfall 3).
    for (const area of [
      "General",
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
    // w26.f03 (e05 F9): the pill placeholder is the frame's "Search" — the
    // registered "Search settings" copy stays the input's accessible name.
    const search = screen.getByPlaceholderText("Search");
    expect(search).toHaveAccessibleName("Search settings");
    expect(screen.getByRole("button", { name: "Close settings" })).toBeInTheDocument();
    // w26.f03 (e05 F10): the top bar carries EXACTLY the frame's three
    // elements — search pill · window title · ✕; the extra "Reset all to
    // defaults" ghost is gone.
    expect(screen.queryByRole("button", { name: "Reset all to defaults" })).toBeNull();
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

  it("Escape closes the clean window to the player (w26.d1 — frame-implied dismissal)", async () => {
    renderAt("/settings/interface");
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(screen.getByTestId("player-home")).toBeInTheDocument());
  });

  it("Escape with unsaved changes routes through the same confirm dialog as the ✕", async () => {
    renderAt("/settings/interface");
    // w26.gfix-B: a live interface frame row (window metrics left the ui
    // registry with the re-seed — g10-P1-1).
    fireEvent.click(screen.getByRole("switch", { name: "Keep window above other windows" }));
    fireEvent.keyDown(window, { key: "Escape" });
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("Discard unsaved changes?");
    // Stay keeps the settings window (and the draft) open.
    fireEvent.click(screen.getByRole("button", { name: "Stay" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByTestId("settings-applybar")).toBeInTheDocument();
  });

  it("moves focus into the window on open and traps Tab inside it (e06 F8)", () => {
    const { container } = renderAt("/settings/interface");
    // Initial focus: the sidebar's ACTIVE area row (the window is a floating
    // surface — focus starts inside it, not on the app chrome behind).
    const active = screen.getByRole("button", { name: "Interface" });
    expect(active).toHaveFocus();
    const window_ = container.querySelector(".settings-window") as HTMLElement;
    // Tab from the LAST focusable stop wraps to the first (the search input).
    // Same focusable definition as the trap (aria-hidden pointer
    // affordances like the number steppers are not tab stops).
    const focusables = window_.querySelectorAll(
      'button:not([disabled]):not([aria-hidden="true"]), input:not([disabled]), [tabindex]:not([tabindex="-1"])',
    );
    const last = focusables[focusables.length - 1] as HTMLElement;
    const first = focusables[0] as HTMLElement;
    last.focus();
    fireEvent.keyDown(window_, { key: "Tab" });
    expect(first).toHaveFocus();
    // Shift+Tab from the first wraps to the last.
    fireEvent.keyDown(window_, { key: "Tab", shiftKey: true });
    expect(last).toHaveFocus();
    // The wrap stays INSIDE the window — focus never lands on the app chrome.
    expect(window_.contains(document.activeElement)).toBe(true);
  });

  it("renders interface rows FLAT — the frame carries no Advanced bar (e05 F7)", () => {
    renderAt("/settings/interface");
    // Settings - interface.svg has NO "Advanced Options" bar: every seeded
    // row (advanced-flagged or not) renders in its section (areas/interface).
    expect(screen.queryByRole("button", { name: "Advanced Options" })).toBeNull();
    // w26.gfix-B: the frame's 14th row (no window-metric rows exist anymore).
    expect(screen.getByText("Show controls in fullscreen")).toBeInTheDocument();
  });

  it("segregates GLOBAL advanced entries behind the in-flow disclosure (global-1, e05 F8)", () => {
    renderAt("/settings/global");
    const toggle = screen.getByRole("button", { name: "Advanced Options" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText("Show minor warnings")).toBeNull();
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("Show minor warnings")).toBeInTheDocument();
    expect(screen.getByText("Collect playback statistics")).toBeInTheDocument();
  });

  it("redirects an unknown area to the first schema area", async () => {
    renderAt("/settings/nope");
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "General" })).toHaveAttribute("aria-current", "page"),
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
    // w26.gfix-B: the interface frame's own row inventory (g10-P1-1) — bool,
    // enum, path, duration-ms slider.
    expect(screen.getByRole("switch", { name: "Keep window above other windows" })).toBeInTheDocument();
    const resize = screen.getByRole("button", { name: "Resize window to fit video" });
    expect(resize).toHaveAttribute("aria-haspopup", "listbox");
    expect(resize).toHaveTextContent("Never"); // frame value
    // Path row — input + the honest disabled Browse action (g10-P2-2).
    const skinFile = screen.getByRole("textbox", { name: "Skin file" }) as HTMLInputElement;
    expect(skinFile).toHaveAttribute("type", "text");
    expect(screen.getByRole("button", { name: /browse/i })).toBeDisabled();
    // Duration-ms row — the frame's slider with scale labels (g10-P1-1).
    const hideAfter = screen.getByRole("slider", { name: "Hide after" });
    expect(hideAfter).toHaveAttribute("aria-valuemin", "0");
    expect(hideAfter).toHaveAttribute("aria-valuemax", "10000");
    expect(screen.getByText("5 s")).toBeInTheDocument(); // scale center label
  });

  it("hides feature-gated unexposed entries (playback.hw-decode-policy)", () => {
    const { container } = renderAt("/settings/playback");
    expect(container.querySelector('[data-setting="playback.hw-decode-policy"]')).toBeNull();
  });

  it("shows the apply bar when dirty; apply persists and clears it", () => {
    renderAt("/settings/interface");
    expect(screen.queryByTestId("settings-applybar")).toBeNull();
    fireEvent.click(screen.getByRole("switch", { name: "Keep window above other windows" }));
    expect(screen.getByTestId("settings-applybar")).toHaveTextContent("Unsaved changes (1)");
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    expect(screen.queryByTestId("settings-applybar")).toBeNull();
    expect(JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? "{}")).toEqual({
      "interface.keep-above": true,
    });
  });

  it("revert discards draft edits without persisting", () => {
    renderAt("/settings/interface");
    fireEvent.click(screen.getByRole("switch", { name: "Remember window size and position" }));
    fireEvent.click(screen.getByRole("button", { name: "Revert" }));
    expect(screen.queryByTestId("settings-applybar")).toBeNull();
    expect(
      (screen.getByRole("switch", { name: "Remember window size and position" }) as HTMLElement).getAttribute("aria-checked"),
    ).toBe("true");
    expect(window.localStorage.getItem(STORAGE_KEY)).toBeNull();
  });

  it("badges restart-required fields while changed and after apply", () => {
    renderAt("/settings/interface");
    // w26.gfix-B: the live restart-required interface row (the frame relabel
    // of interface.remember-window-geometry — g10-P1-1).
    fireEvent.click(screen.getByRole("switch", { name: "Remember window size and position" }));
    expect(field("interface.remember-window-geometry")).toHaveTextContent("Restart required");
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    // still flagged: the applied change needs a restart to take effect
    expect(field("interface.remember-window-geometry")).toHaveTextContent("Restart required");
    expect(screen.getByTestId("settings-restart-notice")).toHaveTextContent("restart");
  });

  it("searches across areas from another area's screen", () => {
    renderAt("/settings/global");
    // w26.gfix-B: a live cross-area row (window metrics left the registry
    // with the re-seed); "Hide after" lives on interface, we are on global.
    fireEvent.change(screen.getByPlaceholderText("Search"), { target: { value: "hide after" } });
    const results = screen.getByTestId("settings-search-results");
    expect(results).toHaveTextContent("Hide after");
    expect(results).toHaveTextContent("Interface"); // cross-area hit
    expect(screen.queryByRole("heading", { name: "Appearance" })).toBeNull(); // current screen hidden while searching
  });

  it("shows an honest no-results state", () => {
    renderAt("/settings/global");
    fireEvent.change(screen.getByPlaceholderText("Search"), { target: { value: "zzzz" } });
    expect(screen.getByTestId("settings-search-empty")).toHaveTextContent("No settings match your search");
  });

  it("confirms before navigating away with unsaved changes (4.B4 Dialog)", async () => {
    renderAt("/settings/interface");
    fireEvent.click(screen.getByRole("switch", { name: "Keep window above other windows" }));
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
    fireEvent.click(screen.getByRole("switch", { name: "Keep window above other windows" }));
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
    // w26.gfix-B: the privacy frame's own rows render (the telemetry rows
    // left with the re-seed — g11-P2-1).
    expect(screen.getByRole("switch", { name: "Save recently played items" })).toBeInTheDocument();
    expect(screen.getByText("Clear history now")).toBeInTheDocument();
  });

  it("resets a single field to its schema default", () => {
    renderAt("/settings/interface");
    // w26.gfix-B: a live bool row (default OFF) — toggle on, then reset.
    const toggle = screen.getByRole("switch", { name: "Keep window above other windows" });
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-checked", "true");
    const resetButtons = screen.getAllByRole("button", { name: "Reset to default" });
    expect(resetButtons.length).toBeGreaterThan(0);
    fireEvent.click(resetButtons[0] as HTMLElement);
    expect(screen.getByRole("switch", { name: "Keep window above other windows" })).toHaveAttribute(
      "aria-checked",
      "false",
    ); // schema default
  });

  // w26.f03 (e05 F10): the top-bar "Reset all to defaults" ghost is gone
  // (frame-verbatim top bar); the reset-to-default contract stays reachable
  // through the per-field reset buttons above and the provider's resetArea
  // seam (covered by SettingsProvider.test.tsx "resetArea resets every field
  // of one area and nothing else").
});
