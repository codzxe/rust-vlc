import type { SettingValue } from "./generated/schema";

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

export const localStorageSettingsPersistence: SettingsPersistence = {
  load() {
    try {
      const raw = window.localStorage.getItem(STORAGE_KEY);
      if (raw === null) return {};
      const parsed: unknown = JSON.parse(raw);
      if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) return {};
      return parsed as Record<string, SettingValue>;
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

export const memorySettingsPersistence: SettingsPersistence = {
  load() {
    return {};
  },
  save() {
    /* test double */
  },
};
