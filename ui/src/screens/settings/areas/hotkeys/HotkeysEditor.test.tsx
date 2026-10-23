import { beforeEach, describe, expect, it } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { HotkeysEditor } from "./HotkeysEditor";
import { memoryHotkeyOverrides } from "./overrides";
import type { HotkeyBinding } from "./generated/bindings";

// Card 4.E2 hotkeys editor: render from fixture data, edit flow, conflict
// detection fires + clears, reset-to-default per binding, apply persists.

const FIXTURE: readonly HotkeyBinding[] = [
  { id: "media.play-pause", section: "media", key: "Space", labelMsgid: "hotkeys.play-pause", command: "playback.toggle-play-pause", mode: "player-focused", disabled: false },
  { id: "media.stop", section: "media", key: "S", labelMsgid: "hotkeys.stop", command: "playback.stop", mode: "player-focused", disabled: false },
  { id: "window.quit", section: "window", key: "Ctrl+Q", labelMsgid: "hotkeys.quit", uiAction: "quit", mode: "always", disabled: false },
  { id: "media.open-disc", section: "media", key: "Ctrl+D", labelMsgid: "hotkeys.open-disc", uiAction: "open-disc", mode: "player-focused", disabled: true },
];

const saved: Record<string, unknown> = {};
const recordingStore = {
  load: () => ({} as Record<string, string>),
  save: (o: Record<string, string>) => Object.assign(saved, o),
};

beforeEach(() => {
  window.localStorage.clear();
});

const mount = () => render(<HotkeysEditor bindings={FIXTURE} store={recordingStore} />);

describe("HotkeysEditor (render + edit + conflicts + reset)", () => {
  it("renders one row per binding with real keymap data (action, combo, command id)", () => {
    mount();
    for (const b of FIXTURE) {
      const row = screen.getByTestId(`hotkeys-input-${b.id}`).closest("tr");
      expect(row).toBeTruthy();
      expect((screen.getByTestId(`hotkeys-input-${b.id}`) as HTMLInputElement).value).toBe(b.key);
    }
    expect(screen.getByTestId("hotkeys-input-media.play-pause").closest("tr")?.textContent).toContain("playback.toggle-play-pause");
    const quitRow = screen.getByTestId("hotkeys-input-window.quit").closest("tr");
    expect(quitRow?.textContent).toContain("quit"); // ui_action as command id
  });

  it("shows the recipe/command/ui_action id in the command column, — when none", () => {
    const noCommand: HotkeyBinding = { ...FIXTURE[0]!, id: "media.bare", command: undefined, recipe: undefined, uiAction: undefined };
    render(<HotkeysEditor bindings={[noCommand]} store={memoryHotkeyOverrides} />);
    expect(screen.getByTestId("hotkeys-input-media.bare").closest("tr")?.textContent).toContain("—");
  });

  it("edit writes draft state (value changes, marked changed)", async () => {
    mount();
    const input = screen.getByTestId("hotkeys-input-media.stop") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "X" } });
    expect(input.value).toBe("X");
    expect(input.closest("tr")?.dataset.changed).toBe("true");
    expect(screen.getByTestId("hotkeys-applybar")).toBeTruthy();
  });

  it("conflict detection fires on BOTH rows of a duplicate combo and clears when fixed", async () => {
    mount();
    // Make media.stop collide with media.play-pause's Space.
    const stop = screen.getByTestId("hotkeys-input-media.stop") as HTMLInputElement;
    fireEvent.change(stop, { target: { value: "Space" } });
    const ppRow = screen.getByTestId("hotkeys-input-media.play-pause").closest("tr");
    const stopRow = stop.closest("tr");
    expect(ppRow?.dataset.conflict).toBe("true");
    expect(stopRow?.dataset.conflict).toBe("true");
    expect(screen.getAllByRole("alert")).toHaveLength(2); // inline warning on both rows
    // Fix it → warnings clear.
    fireEvent.change(stop, { target: { value: "K" } });
    expect(ppRow?.dataset.conflict).toBeUndefined();
    expect(stopRow?.dataset.conflict).toBeUndefined();
  });

  it("reset-to-default restores the keymap default for one binding", async () => {
    mount();
    const input = screen.getByTestId("hotkeys-input-media.stop") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "K" } });
    fireEvent.click(screen.getByTestId("hotkeys-reset-media.stop"));
    expect(input.value).toBe("S");
    expect(input.closest("tr")?.dataset.changed).toBeUndefined();
  });

  it("Apply persists overrides via the store; Revert discards the draft", async () => {
    mount();
    const input = screen.getByTestId("hotkeys-input-media.stop") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "K" } });
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    await waitFor(() => expect(saved["media.stop"]).toBe("K"));
    // New edit + revert → back to the persisted override, not the default.
    fireEvent.change(input, { target: { value: "L" } });
    fireEvent.click(screen.getByRole("button", { name: "Revert" }));
    expect(input.value).toBe("K");
  });

  it("renders disabled-from-data bindings inert (feature-gated source)", () => {
    mount();
    const disc = screen.getByTestId("hotkeys-input-media.open-disc") as HTMLInputElement;
    expect(disc.disabled).toBe(true);
    expect(disc.closest("tr")?.dataset.disabled).toBe("true");
  });
});

describe("HotkeysEditor (real generated data)", () => {
  it("renders all bindings from keymaps/default.toml with unique ids", async () => {
    const { HOTKEY_BINDINGS } = await import("./generated/bindings");
    const ids = new Set(HOTKEY_BINDINGS.map((b) => b.id));
    expect(ids.size).toBe(HOTKEY_BINDINGS.length);
    expect(HOTKEY_BINDINGS.length).toBeGreaterThan(100);
    render(<HotkeysEditor store={memoryHotkeyOverrides} />);
    const rows = document.querySelectorAll(".hotkeys-row");
    expect(rows.length).toBe(HOTKEY_BINDINGS.length);
  });
});
