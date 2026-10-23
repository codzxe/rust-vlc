// Hotkey override persistence (card 4.E2) — same shape and honesty rules as
// the settings persistence seam (persistence.ts, card 4.E1): a MOCK
// localStorage round-trip today, the real write path (keymap config over the
// client bridge) is the app layer's handoff. Unknown/corrupt stores behave
// like "no overrides" — never crash, never invent bindings.

export type HotkeyOverrides = Readonly<Record<string, string>>;

export interface HotkeyOverridesStore {
  load(): HotkeyOverrides;
  save(overrides: HotkeyOverrides): void;
}

const STORAGE_KEY = "lumen.hotkeys.overrides.v1";

export const localStorageHotkeyOverrides: HotkeyOverridesStore = {
  load() {
    try {
      const raw = window.localStorage.getItem(STORAGE_KEY);
      if (raw === null) return {};
      const parsed: unknown = JSON.parse(raw);
      if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) return {};
      const out: Record<string, string> = {};
      for (const [id, combo] of Object.entries(parsed as Record<string, unknown>)) {
        if (typeof combo === "string" && combo !== "") out[id] = combo;
      }
      return out;
    } catch {
      return {};
    }
  },
  save(overrides) {
    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(overrides));
    } catch {
      // Storage unavailable — overrides stay in-memory for this session.
    }
  },
};

export const memoryHotkeyOverrides: HotkeyOverridesStore = {
  load() {
    return {};
  },
  save() {
    /* test double */
  },
};
