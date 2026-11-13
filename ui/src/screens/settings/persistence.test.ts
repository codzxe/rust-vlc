// w26.k12 — persistence.ts direct unit tests (j12 P3-11 + P3-6). The catch
// branches were uncovered (the shell tests exercise only the happy
// round-trip); these cover corrupt JSON, non-object stores, quota-refusing
// storage, unknown keys, and the NEW per-key value-type validation that
// drops a KNOWN key whose stored value no longer validates against the
// schema entry (a tampered {"interface.keep-above": "yes"} used to load and
// silently render as bool-false).

import { afterEach, describe, expect, it, vi } from "vitest";
import {
  localStorageSettingsPersistence,
  memorySettingsPersistence,
} from "./persistence";
import { SETTINGS_ENTRIES } from "./generated/schema";
import type { SettingValue } from "./generated/schema";

const STORAGE_KEY = "lumen.settings.overrides.v1";

/** The store's own key for a given schema entry (typing helper). */
const keyOf = (needle: string): string =>
  SETTINGS_ENTRIES.find((e) => e.key === needle)?.key ?? needle;

const seed = (store: Record<string, SettingValue>) => {
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(store));
};

afterEach(() => {
  window.localStorage.clear();
  vi.restoreAllMocks();
});

describe("localStorageSettingsPersistence.load (j12 P3-6/P3-11)", () => {
  it("returns {} for a missing store", () => {
    expect(localStorageSettingsPersistence.load()).toEqual({});
  });

  it("returns {} for corrupt JSON (the catch branch)", () => {
    window.localStorage.setItem(STORAGE_KEY, "{not json");
    expect(localStorageSettingsPersistence.load()).toEqual({});
  });

  it("returns {} for a non-object store (array / null / scalar)", () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(["interface.keep-above"]));
    expect(localStorageSettingsPersistence.load()).toEqual({});
    window.localStorage.setItem(STORAGE_KEY, "null");
    expect(localStorageSettingsPersistence.load()).toEqual({});
    window.localStorage.setItem(STORAGE_KEY, "42");
    expect(localStorageSettingsPersistence.load()).toEqual({});
  });

  it("drops unknown keys (config-never-invents, doc 05 §6.4)", () => {
    seed({ "no.such.key": true, "interface.keep-above": true });
    expect(localStorageSettingsPersistence.load()).toEqual({
      "interface.keep-above": true,
    });
  });

  it("round-trips a valid bool override", () => {
    seed({ "interface.keep-above": true });
    expect(localStorageSettingsPersistence.load()).toEqual({
      "interface.keep-above": true,
    });
  });

  it("drops a KNOWN key whose value fails the type validation (the tampered-store repro)", () => {
    // A bool row whose stored value is the string "yes" — used to load and
    // silently render as bool-false; now dropped, so the schema default wins.
    seed({ [keyOf("interface.keep-above")]: "yes" });
    expect(localStorageSettingsPersistence.load()).toEqual({});
  });

  it("drops an enum override whose value is not in the enumeration", () => {
    const entry = SETTINGS_ENTRIES.find((e) => e.type === "enum" && e.enumeration.length > 0);
    expect(entry).toBeDefined();
    seed({ [entry!.key]: "not-an-enum-value" });
    expect(localStorageSettingsPersistence.load()).toEqual({});
  });

  it("keeps a valid enum override (membership passes)", () => {
    const entry = SETTINGS_ENTRIES.find((e) => e.type === "enum" && e.enumeration.length > 0);
    seed({ [entry!.key]: entry!.enumeration[0]!.value });
    expect(localStorageSettingsPersistence.load()).toEqual({
      [entry!.key]: entry!.enumeration[0]!.value,
    });
  });

  it("drops a list<string> override carrying a non-string element", () => {
    const entry = SETTINGS_ENTRIES.find((e) => e.type === "list<string>");
    expect(entry).toBeDefined();
    seed({ [entry!.key]: ["en-US", 42] as unknown as SettingValue });
    expect(localStorageSettingsPersistence.load()).toEqual({});
    // All-string lists pass.
    seed({ [entry!.key]: ["en-US", "de-DE"] });
    expect(localStorageSettingsPersistence.load()).toEqual({
      [entry!.key]: ["en-US", "de-DE"],
    });
  });

  it("drops a numeric row whose stored value is a string (type mismatch)", () => {
    const entry = SETTINGS_ENTRIES.find((e) => e.type === "int");
    expect(entry).toBeDefined();
    seed({ [entry!.key]: "200" });
    expect(localStorageSettingsPersistence.load()).toEqual({});
  });
});

describe("localStorageSettingsPersistence.save", () => {
  it("writes the overrides as JSON", () => {
    localStorageSettingsPersistence.save({ "interface.keep-above": true });
    expect(JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? "null")).toEqual({
      "interface.keep-above": true,
    });
  });

  it("swallows quota/storage failures — never crashes (the catch branch)", () => {
    const setItem = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new DOMException("QuotaExceededError");
    });
    expect(() => localStorageSettingsPersistence.save({ "interface.keep-above": true })).not.toThrow();
    expect(setItem).toHaveBeenCalled();
  });
});

describe("memorySettingsPersistence (the labeled test double, j12 P3-6)", () => {
  it("loads {} and save is a no-op — mounting-only seam, never a store", () => {
    expect(memorySettingsPersistence.load()).toEqual({});
    expect(() => memorySettingsPersistence.save({ "interface.keep-above": true })).not.toThrow();
    expect(memorySettingsPersistence.load()).toEqual({});
  });
});
