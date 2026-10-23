// Onboarding state + persistence seam (card 4.F1).
//
// THE honest rule (MP §5, doc 07 §2.6/§5.4, card AC): privacy choices are
// opt-IN only and default OFF. A fresh run has every flag false; nothing is
// ever enabled implicitly — not by finishing, not by skipping, not by a
// corrupt/absent store (absent keys read as false, never as "unknown → on").
//
// Persistence mirrors E1's SettingsPersistence seam shape (interface +
// localStorage impl + memory test double). The store key is separate from
// `lumen.settings.overrides.v1` because SettingsProvider.apply() rewrites
// that key wholesale (schema overrides only) — an onboarding flag written
// there would be wiped by the next settings Apply. Handoff for the REAL
// config write path (config.set-setting over the client bridge, doc 05
// §2.11) is recorded in ui/NEXT-STEPS.md.

import { useCallback, useSyncExternalStore } from "react";

/** The opt-in-only privacy choices presented on the privacy step. */
export interface OnboardingChoices {
  /** Maps to schema key `privacy.telemetry` (doc 07 §5.4). Default false. */
  readonly telemetry: boolean;
  /** Maps to schema key `privacy.update-checks`. Default false. */
  readonly updateChecks: boolean;
  /**
   * Online suggestions (extension/store metadata lookups). [PROPOSAL] — not
   * yet in the settings schema; honest placeholder until a schema key lands.
   * Default false.
   */
  readonly onlineSuggestions: boolean;
}

export interface OnboardingState {
  /** Set by finishing OR skipping the wizard; gates first-run routing. */
  readonly complete: boolean;
  readonly choices: OnboardingChoices;
}

/** Every privacy flag OFF, nothing complete — the shipped default. */
export const ONBOARDING_DEFAULTS: OnboardingState = {
  complete: false,
  choices: { telemetry: false, updateChecks: false, onlineSuggestions: false },
};

export interface OnboardingPersistence {
  load(): Partial<OnboardingState> | null;
  save(state: OnboardingState): void;
}

const STORAGE_KEY = "lumen.onboarding.v1";

export const localStorageOnboardingPersistence: OnboardingPersistence = {
  load() {
    try {
      const raw = window.localStorage.getItem(STORAGE_KEY);
      if (raw === null) return null;
      const parsed: unknown = JSON.parse(raw);
      if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) return null;
      return parsed as Partial<OnboardingState>;
    } catch {
      return null; // corrupt store behaves like "never ran" — never crash
    }
  },
  save(state) {
    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
    } catch {
      // Storage unavailable (private mode/quota) — state stays in-memory.
    }
  },
};

export const memoryOnboardingPersistence: OnboardingPersistence = {
  load() {
    return null; // test double: always a fresh run
  },
  save() {
    /* test double */
  },
};

// --- external store (no provider needed; App-level gate reads it cheaply) ---

let persistence: OnboardingPersistence = localStorageOnboardingPersistence;
let state: OnboardingState = hydrate();
const listeners = new Set<() => void>();

/** Absent/partial/corrupt store folds back onto OFF defaults — never "on". */
function hydrate(): OnboardingState {
  const loaded = persistence.load();
  if (loaded === null) return ONBOARDING_DEFAULTS;
  return {
    complete: loaded.complete === true,
    choices: {
      telemetry: loaded.choices?.telemetry === true,
      updateChecks: loaded.choices?.updateChecks === true,
      onlineSuggestions: loaded.choices?.onlineSuggestions === true,
    },
  };
}

function commit(next: OnboardingState): void {
  state = next;
  persistence.save(state);
  for (const listener of listeners) listener();
}

const subscribe = (listener: () => void): (() => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};

const getSnapshot = (): OnboardingState => state;

/** Reactive read of the onboarding state (App gate + wizard). */
export function useOnboardingState(): OnboardingState {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}

export type OnboardingChoiceKey = keyof OnboardingChoices;

/** Flip one privacy choice (persisted immediately — an opt-in is explicit). */
export function setOnboardingChoice(key: OnboardingChoiceKey, value: boolean): void {
  if (state.choices[key] === value) return;
  commit({ ...state, choices: { ...state.choices, [key]: value } });
}

/**
 * Finish the wizard. Choices are committed exactly as they stand — the
 * wizard never mutates them here (finishing is not consent). Skipping calls
 * the same function with the defaults still in place.
 */
export function completeOnboarding(): void {
  if (state.complete) return;
  commit({ ...state, complete: true });
}

/** Test-only: swap the seam and rewind to a fresh run. */
export function resetOnboardingForTests(
  seam: OnboardingPersistence = localStorageOnboardingPersistence,
): void {
  persistence = seam;
  state = hydrate();
  for (const listener of listeners) listener();
}

/** Live choice editing inside the wizard (wraps the store actions). */
export function useOnboardingActions(): {
  setChoice: (key: OnboardingChoiceKey, value: boolean) => void;
  complete: () => void;
} {
  const setChoice = useCallback((key: OnboardingChoiceKey, value: boolean) => {
    setOnboardingChoice(key, value);
  }, []);
  const complete = useCallback(() => completeOnboarding(), []);
  return { setChoice, complete };
}
