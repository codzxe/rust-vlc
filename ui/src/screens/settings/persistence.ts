import { SETTINGS_ENTRIES } from "./generated/schema";
import type { SettingEntry, SettingValue } from "./generated/schema";

// MOCK persistence seam (card 4.E1). The framework never talks to the
// engine directly: Apply() hands the changed values to this interface.
// Today it round-trips localStorage for development; the REAL write path is
// the app layer's — `config.set-setting` over the client bridge (doc 05
// §2.11), which validates against the embedded schema before applying.
// Handoff recorded in ui/NEXT-STEPS.md.
export interface SettingsPersistence {
  /** Persisted user overrides; unknown keys are dropped by the loader. */
  load(): Record<string, SettingValue>;
  save(overrides: Record<string, SettingValue>): void;
}

const STORAGE_KEY = "lumen.settings.overrides.v1";

/**
 * w26.k12 (j12 P3-6): per-key VALUE-type validation on load. The provider
 * already drops unknown keys (key membership); this additionally drops KNOWN
 * keys whose stored value no longer validates against the schema entry — a
 * tampered `{"interface.keep-above": "yes"}` used to load and silently
 * render as bool-false. Types + enum membership + list element types are
 * checked; min/max ranges are NOT enforced here (range drift is a schema
 * evolution concern, the wells clamp on edit, and the engine bridge
 * validates its own side). The desktop seam (config-settings.ts) is the
 * engine's own validated path — this guards the browser localStorage seam.
 */
function isValidValue(entry: SettingEntry, value: SettingValue): boolean {
  switch (entry.type) {
    case "bool":
      return typeof value === "boolean";
    case "enum":
      return typeof value === "string" && entry.enumeration.some((o) => o.value === value);
    case "string":
    case "path":
    case "url":
      return typeof value === "string";
    case "list<string>":
      return Array.isArray(value) && value.every((v) => typeof v === "string");
    default:
      // int / float / duration-ms / percent: finite numbers.
      return typeof value === "number" && Number.isFinite(value);
  }
}

export const localStorageSettingsPersistence: SettingsPersistence = {
  load() {
    try {
      const raw = window.localStorage.getItem(STORAGE_KEY);
      if (raw === null) return {};
      const parsed: unknown = JSON.parse(raw);
      if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) return {};
      const store = parsed as Record<string, SettingValue>;
      const out: Record<string, SettingValue> = {};
      for (const entry of SETTINGS_ENTRIES) {
        const value = store[entry.key];
        if (value !== undefined && isValidValue(entry, value)) out[entry.key] = value;
      }
      return out;
    } catch {
      return {}; // corrupt store behaves like "no overrides" — never crash
    }
  },
  save(overrides) {
    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(overrides));
    } catch {
      // Storage unavailable (private mode/quota) — settings stay in-memory.
    }
  },
};

/**
 * Test double (NOT a production seam): an always-empty persistence used by
 * three picker tests + settingsThemeSync's test to mount providers without
 * touching localStorage. w26.k12 (j12 P3-6): kept here on the review's
 * honest-label option (Lead taste) — moving it to a test util would break
 * four imports in other lanes' test files; the label above stays the truth.
 */
export const memorySettingsPersistence: SettingsPersistence = {
  load() {
    return {};
  },
  save() {
    /* test double */
  },
};
