import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { SettingValue } from "./generated/schema";
import { SettingsProvider } from "./SettingsProvider";
import { useSettings } from "./SettingsProvider";
import type { SettingsPersistence } from "./persistence";

const makePersistence = (initial: Record<string, SettingValue> = {}): SettingsPersistence & {
  saved: Record<string, SettingValue>[];
} => {
  const saved: Record<string, SettingValue>[] = [];
  return {
    saved,
    load: () => initial,
    save: (overrides) => {
      saved.push(overrides);
    },
  };
};

const setup = (initial: Record<string, SettingValue> = {}) => {
  const persistence = makePersistence(initial);
  const hook = renderHook(() => useSettings(), { wrapper: ({ children }) => <SettingsProvider persistence={persistence}>{children}</SettingsProvider> });
  return { persistence, ...hook };
};

describe("SettingsProvider (card 4.E1 framework state)", () => {
  it("starts clean: draft = persisted = schema defaults", () => {
    const { result } = setup();
    expect(result.current.dirtyCount).toBe(0);
    expect(result.current.value("interface.theme")).toBe("dark"); // product.toml defaults.theme
    expect(result.current.value("interface.window-width")).toBe(1280);
  });

  it("loads persisted overrides through the seam and drops unknown keys", () => {
    const { result } = setup({
      "interface.theme": "light",
      "not.a.real.key": true, // config-never-invents (doc 05 §6.4)
    });
    expect(result.current.value("interface.theme")).toBe("light");
    expect(result.current.draft["not.a.real.key"]).toBeUndefined();
  });

  it("tracks dirty state per key and reverts everything", () => {
    const { result } = setup();
    act(() => result.current.setValue("interface.theme", "light"));
    expect(result.current.dirtyCount).toBe(1);
    expect(result.current.isDirty("interface.theme")).toBe(true);
    act(() => result.current.revert());
    expect(result.current.dirtyCount).toBe(0);
    expect(result.current.value("interface.theme")).toBe("dark");
  });

  it("reverts a single key only", () => {
    const { result } = setup();
    act(() => result.current.setValue("interface.theme", "light"));
    act(() => result.current.setValue("playback.start-paused", true));
    act(() => result.current.revert("interface.theme"));
    expect(result.current.dirtyCount).toBe(1);
    expect(result.current.value("interface.theme")).toBe("dark");
    expect(result.current.value("playback.start-paused")).toBe(true);
  });

  it("apply commits draft to persisted and saves only non-default overrides", () => {
    const { result, persistence } = setup();
    act(() => result.current.setValue("interface.theme", "light"));
    act(() => result.current.setValue("interface.window-width", 1920));
    act(() => result.current.apply());
    expect(result.current.dirtyCount).toBe(0);
    expect(result.current.persisted["interface.theme"]).toBe("light");
    expect(persistence.saved).toEqual([
      { "interface.theme": "light", "interface.window-width": 1920 },
    ]);
  });

  it("revert after apply restores the APPLIED value, not the pre-edit one", () => {
    const { result } = setup();
    act(() => result.current.setValue("interface.theme", "light"));
    act(() => result.current.apply());
    act(() => result.current.setValue("interface.theme", "dark"));
    act(() => result.current.revert());
    expect(result.current.value("interface.theme")).toBe("light");
  });

  it("resetField restores the schema default into the draft", () => {
    const { result } = setup({ "playback.default-rate": 2.0 });
    expect(result.current.value("playback.default-rate")).toBe(2.0);
    act(() => result.current.resetField("playback.default-rate"));
    expect(result.current.value("playback.default-rate")).toBe(1.0); // product.toml playback.default_rate
    expect(result.current.isDirty("playback.default-rate")).toBe(true); // still draft vs persisted
  });

  it("resetArea resets every field of one area and nothing else", () => {
    const { result } = setup({ "interface.theme": "light", "privacy.telemetry": true });
    act(() => result.current.setValue("interface.window-width", 800));
    act(() => result.current.resetArea("interface"));
    expect(result.current.value("interface.theme")).toBe("dark"); // area reset
    expect(result.current.value("interface.window-width")).toBe(1280);
    expect(result.current.value("privacy.telemetry")).toBe(true); // other area untouched
  });

  it("records restart-required keys changed by apply", () => {
    const { result } = setup();
    act(() => result.current.setValue("interface.start-maximized", true)); // restart_required
    act(() => result.current.setValue("privacy.telemetry", true)); // no restart
    expect(result.current.restartPending).toEqual([]);
    act(() => result.current.apply());
    expect(result.current.restartPending).toEqual(["interface.start-maximized"]);
  });

  it("clears restart-pending when a later apply has no restart-required changes", () => {
    const { result } = setup();
    act(() => result.current.setValue("interface.start-maximized", true));
    act(() => result.current.apply());
    act(() => result.current.setValue("privacy.telemetry", true));
    act(() => result.current.apply());
    expect(result.current.restartPending).toEqual([]);
  });
});
