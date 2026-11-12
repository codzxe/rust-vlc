import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Event } from "../../client/transport";
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
    // w26.gfix-B: interface.window-* went ui=false with the re-seed (the
    // interface frame shows no window-metric rows) — a live default_ref row
    // carries the proof (product.toml window.remember_geometry).
    expect(result.current.value("interface.remember-window-geometry")).toBe(true);
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
    act(() => result.current.setValue("interface.keep-above", true));
    act(() => result.current.apply());
    expect(result.current.dirtyCount).toBe(0);
    expect(result.current.persisted["interface.theme"]).toBe("light");
    expect(persistence.saved).toEqual([
      { "interface.theme": "light", "interface.keep-above": true },
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
    // w26.gfix-B: interface.theme lives on the GLOBAL screen now (g10-P1-2) —
    // an interface-screen key carries the area-reset proof.
    const { result } = setup({ "interface.start-in-compact-mode": true, "privacy.save-recent-items": false });
    act(() => result.current.setValue("interface.keep-above", true));
    act(() => result.current.resetArea("interface"));
    expect(result.current.value("interface.start-in-compact-mode")).toBe(false); // area reset
    expect(result.current.value("interface.keep-above")).toBe(false);
    expect(result.current.value("privacy.save-recent-items")).toBe(false); // other area untouched
  });

  it("records restart-required keys changed by apply", () => {
    const { result } = setup();
    // w26.gfix-B: interface.start-maximized went ui=false with the re-seed;
    // interface.remember-window-geometry is the live restart-required row.
    act(() => result.current.setValue("interface.remember-window-geometry", false)); // restart_required
    act(() => result.current.setValue("interface.keep-above", true)); // no restart
    expect(result.current.restartPending).toEqual([]);
    act(() => result.current.apply());
    expect(result.current.restartPending).toEqual(["interface.remember-window-geometry"]);
  });

  it("clears restart-pending when a later apply has no restart-required changes", () => {
    const { result } = setup();
    act(() => result.current.setValue("interface.remember-window-geometry", false));
    act(() => result.current.apply());
    act(() => result.current.setValue("interface.keep-above", true));
    act(() => result.current.apply());
    expect(result.current.restartPending).toEqual([]);
  });
});

describe("SettingsProvider — the live engine feed (issue #3, 4.H1 handoff d)", () => {
  /** A subscribable engine feed: captures the listener, spys the release. */
  const setupWithFeed = () => {
    const persistence = makePersistence();
    let listener: ((event: Event) => void) | null = null;
    const unsubscribe = vi.fn();
    const subscribe = (next: (event: Event) => void): (() => void) => {
      listener = next;
      return unsubscribe;
    };
    const hook = renderHook(() => useSettings(), {
      wrapper: ({ children }) => (
        <SettingsProvider persistence={persistence} subscribe={subscribe}>
          {children}
        </SettingsProvider>
      ),
    });
    return { persistence, hook, unsubscribe, emit: (event: Event) => act(() => listener?.(event)) };
  };

  it("a config.setting-changed event moves the persisted value and the clean draft", () => {
    const { hook, emit } = setupWithFeed();
    expect(hook.result.current.value("interface.theme")).toBe("dark");

    emit({
      seq: 1,
      kind: "config.setting-changed",
      area: "interface",
      key: "theme",
      layer: "cli-flag",
      value: "light",
    } satisfies Event);

    expect(hook.result.current.value("interface.theme")).toBe("light");
    expect(hook.result.current.persisted["interface.theme"]).toBe("light");
    expect(hook.result.current.dirtyCount).toBe(0);
  });

  it("a dirty draft wins: the engine's move does not clobber the user's edit", () => {
    const { hook, emit } = setupWithFeed();
    act(() => hook.result.current.setValue("interface.theme", "solarized"));

    emit({
      seq: 2,
      kind: "config.setting-changed",
      area: "interface",
      key: "theme",
      layer: "engine",
      value: "light",
    } satisfies Event);

    expect(hook.result.current.value("interface.theme")).toBe("solarized");
    expect(hook.result.current.isDirty("interface.theme")).toBe(true);
  });

  it("unknown keys and unrelated events are ignored (config-never-invents)", () => {
    const { hook, emit } = setupWithFeed();
    emit({
      seq: 3,
      kind: "config.setting-changed",
      area: "no-such",
      key: "key",
      layer: "engine",
      value: true,
    } satisfies Event);
    emit({ seq: 4, kind: "playback.end-of-media" } satisfies Event);
    expect(hook.result.current.dirtyCount).toBe(0);
    // The persisted store is seeded with every schema default — the proof
    // of "ignored" is that nothing MOVED off them.
    expect(hook.result.current.persisted["interface.theme"]).toBe("dark");
    expect(hook.result.current.value("interface.theme")).toBe("dark");
  });

  it("unmount releases the engine feed", () => {
    const { hook, unsubscribe } = setupWithFeed();
    hook.unmount();
    expect(unsubscribe).toHaveBeenCalled();
  });
});
