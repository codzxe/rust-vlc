// w26.b07 — Controls area tests (the b02 sibling contract's controls half):
// the hand Keyboard Shortcuts section + hotkeys fold, the embedded generated
// screen (skip amounts + mouse rows bound to their real schema keys), and
// the honest-surface rule for schema-less rows (Remotes renders nothing).

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { ClientProvider } from "../../../client/ClientProvider";
import { MockTransport } from "../../../client/transport";
import { SettingsScreen } from "../SettingsScreen";
import { SettingsProvider } from "../SettingsProvider";
import { ControlsArea } from "./controls";
import { SETTINGS_ENTRIES } from "../generated/schema";
import { HOTKEY_BINDINGS } from "./hotkeys/generated/bindings";
import { localStorageHotkeyOverrides } from "./hotkeys/overrides";

const renderRouted = (area: string) =>
  render(
    <ClientProvider transport={new MockTransport()}>
      <MemoryRouter initialEntries={[`/settings/${area}`]}>
        <Routes>
          <Route path="/settings/:area" element={<SettingsScreen />} />
        </Routes>
      </MemoryRouter>
    </ClientProvider>,
  );

const mountArea = () =>
  render(
    <SettingsProvider>
      <ControlsArea />
    </SettingsProvider>,
  );

const CONTROLS_ENTRIES = SETTINGS_ENTRIES.filter((entry) => entry.screen === "controls");

beforeEach(() => {
  window.localStorage.clear();
});

describe("ControlsArea — routing + the b02 sibling contract", () => {
  it("is routed at /settings/controls under the area registry", () => {
    renderRouted("controls");
    expect(document.querySelector('[data-settings-area="controls"]')).not.toBeNull();
    expect(screen.getByRole("button", { name: "Controls" })).toHaveAttribute("aria-current", "page");
  });

  it("renders exactly ONE area root (w26.i15, g14-P3-4 class — no nested duplicates)", () => {
    renderRouted("controls");
    expect(document.querySelectorAll('[data-settings-area="controls"]')).toHaveLength(1);
    // The single root is the embedded generated screen's (no-testid) root,
    // the recording/extensions pattern — the wrapper carries only the testid.
    expect(document.querySelector('.settings-sections[data-settings-area="controls"]:not([data-testid])')).not.toBeNull();
    expect(document.querySelector('[data-testid="controls-area"]')?.getAttribute("data-settings-area")).toBeNull();
  });

  it("hosts the retired hotkeys slug: /settings/hotkeys redirects here and mounts the fold host", () => {
    renderRouted("hotkeys");
    expect(screen.getByRole("button", { name: "Controls" })).toHaveAttribute("aria-current", "page");
    expect(document.querySelector('[data-testid="controls-area"]')).not.toBeNull();
    // w26.gfix-B (g13-P2-9): frame-verbatim title case.
    expect(screen.getByRole("button", { name: "Edit Keyboard Shortcuts" })).toBeInTheDocument();
  });

  it("embeds GENERATED_SCREENS.controls — every seeded schema row renders bound to its key", () => {
    mountArea();
    expect(CONTROLS_ENTRIES.length).toBeGreaterThan(0);
    for (const entry of CONTROLS_ENTRIES) {
      // All seeded controls rows are ungated (no feature gate) and
      // non-advanced → they must render in the normal flow.
      expect(entry.featureGate).toBe("");
      const field = document.querySelector(`[data-setting="${entry.key}"]`);
      expect(field, `controls entry ${entry.key} must render bound to its schema key`).not.toBeNull();
    }
  });
});

describe("ControlsArea — Keyboard Shortcuts section + the hotkeys fold", () => {
  it("renders the two frame rows with full-width action buttons", () => {
    mountArea();
    expect(screen.getByRole("heading", { name: "Keyboard Shortcuts" })).toBeInTheDocument();
    expect(screen.getByText("Defaults")).toBeInTheDocument();
    expect(screen.getByText("Reset all shortcuts")).toBeInTheDocument();
    const edit = screen.getByRole("button", { name: "Edit Keyboard Shortcuts" });
    expect(edit).toHaveAttribute("aria-expanded", "false");
    expect(edit.closest(".settings-field")).toHaveAttribute("data-controls-action", "edit-shortcuts");
    expect(
      screen.getByRole("button", { name: "Reset" }).closest(".settings-field"),
    ).toHaveAttribute("data-controls-action", "reset-shortcuts");
  });

  it("the editor DIALOG mounts on demand and closes (w26.i15, h11 #5 — no inline slab)", () => {
    mountArea();
    expect(screen.queryByTestId("hotkeys-table")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Edit Keyboard Shortcuts" }));
    expect(screen.getByRole("button", { name: "Edit Keyboard Shortcuts" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    // A real dialog: role + modal semantics + the frame-verbatim title.
    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(screen.getByText("Keyboard Shortcuts", { selector: ".hotkeys-dialog__title" })).toBeInTheDocument();
    expect(screen.getByTestId("hotkeys-table")).toBeInTheDocument();
    // Real keymap data, not a fixture: the editor's first binding renders.
    const first = HOTKEY_BINDINGS[0]!;
    expect(screen.getByTestId(`hotkeys-input-${first.id}`)).toBeInTheDocument();
    // Close via the ✕ — the dialog unmounts, the column never grew a slab.
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(screen.getByRole("button", { name: "Edit Keyboard Shortcuts" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    expect(screen.queryByTestId("hotkeys-table")).toBeNull();
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("Reset clears the persisted overrides and re-reads keymap defaults", () => {
    const first = HOTKEY_BINDINGS[0]!;
    localStorageHotkeyOverrides.save({ [first.id]: "Ctrl+Alt+Test" });
    mountArea();
    fireEvent.click(screen.getByRole("button", { name: "Edit Keyboard Shortcuts" }));
    const before = screen.getByTestId(`hotkeys-input-${first.id}`) as HTMLInputElement;
    expect(before.value).toBe("Ctrl+Alt+Test");
    fireEvent.click(screen.getByRole("button", { name: "Reset" }));
    // The editor re-mounted from the cleared store → the keymap default.
    const after = screen.getByTestId(`hotkeys-input-${first.id}`) as HTMLInputElement;
    expect(after.value).toBe(first.key);
    expect(localStorageHotkeyOverrides.load()).toEqual({});
  });
});

describe("ControlsArea — honest surface (config-never-invents)", () => {
  it("renders the frame's Remotes section as STAGED inert rows (g13-P2-8; w26.i15: tooltip, no note)", () => {
    mountArea();
    // The two frame rows render (frame C8 y762) — inert with the honest
    // reason on each row's tooltip; never dead-but-alive.
    const webRemote = screen.getByRole("switch", { name: "Web remote control" });
    expect(webRemote).toBeDisabled();
    expect(webRemote).toHaveAttribute("aria-checked", "false"); // frame: OFF
    const port = screen.getByRole("textbox", { name: "Port" }) as HTMLInputElement;
    expect(port).toBeDisabled();
    expect(port.value).toBe(""); // frame: empty
    // The honest reason rides each staged row's title — the 11px visible
    // note is gone (content the §C8 frame does not carry, h11 #3).
    const webRemoteRow = webRemote.closest("[data-staged]");
    expect(webRemoteRow).toHaveAttribute("data-staged", "control-server");
    expect(webRemoteRow?.getAttribute("title")).toContain(
      "Staged — the web remote activates when the control server ships",
    );
    expect(port.closest("[data-staged]")).toHaveAttribute("title", webRemoteRow?.getAttribute("title") ?? "");
    expect(screen.queryByRole("note")).toBeNull();
    // No live bindings: every Remotes row awaits its schema key.
    expect(document.querySelectorAll('[data-staged="control-server"]')).toHaveLength(2);
    expect(document.querySelectorAll('[data-setting^="controls.remote"]')).toHaveLength(0);
  });
});
