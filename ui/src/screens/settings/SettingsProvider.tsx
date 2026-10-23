import { createContext } from "react";
import { useContext, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { SETTINGS_ENTRIES } from "./generated/schema";
import type { SettingEntry, SettingValue } from "./generated/schema";
import { localStorageSettingsPersistence } from "./persistence";
import type { SettingsPersistence } from "./persistence";

// SettingsProvider — card 4.E1 framework state (doc 07 §5):
//   persisted  = schema defaults + user overrides from the persistence seam
//   draft      = the values the controls edit
//   dirty keys = draft values that differ from persisted (the Apply/Revert bar)
//   restart    = schema-flagged keys changed by the last successful Apply
// Reset-to-default rewrites draft values from the schema (never invents).

export interface SettingsContextValue {
  /** Values the controls render + edit. */
  readonly draft: Readonly<Record<string, SettingValue>>;
  /** Last applied values (revert target). */
  readonly persisted: Readonly<Record<string, SettingValue>>;
  /** Keys whose draft differs from persisted, in schema order. */
  readonly dirtyKeys: readonly string[];
  /** Schema entries changed by the last Apply that require a restart. */
  readonly restartPending: readonly string[];
  readonly isDirty: (key: string) => boolean;
  readonly dirtyCount: number;
  setValue(key: string, value: SettingValue): void;
  /** Commit draft → persisted (+ persistence seam); records restart keys. */
  apply(): void;
  /** draft ← persisted. No key = revert everything. */
  revert(key?: string): void;
  /** draft[key] ← schema default. */
  resetField(key: string): void;
  /** Every field of one settings area ← schema defaults. */
  resetArea(area: string): void;
  /** Current value of a setting (draft). */
  value(key: string): SettingValue;
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

const defaultsFor = (): Record<string, SettingValue> => {
  const out: Record<string, SettingValue> = {};
  for (const entry of SETTINGS_ENTRIES) out[entry.key] = entry.defaultValue;
  return out;
};

export function SettingsProvider({
  children,
  persistence = localStorageSettingsPersistence,
}: {
  children: ReactNode;
  persistence?: SettingsPersistence;
}) {
  // Lazy state: defaults overlaid with persisted overrides (unknown keys
  // dropped — config-never-invents, doc 05 §6.4).
  const [persisted, setPersisted] = useState<Record<string, SettingValue>>(() => {
    const base = defaultsFor();
    const overrides = persistence.load();
    for (const entry of SETTINGS_ENTRIES) {
      const override = overrides[entry.key];
      if (entry.key in overrides && override !== undefined) base[entry.key] = override;
    }
    return base;
  });
  const [draft, setDraft] = useState<Record<string, SettingValue>>(() => ({ ...persisted }));
  const [restartPending, setRestartPending] = useState<string[]>([]);

  const value = useMemo<SettingsContextValue>(() => {
    const dirtyKeys = SETTINGS_ENTRIES.filter((e) => draft[e.key] !== persisted[e.key]).map((e) => e.key);
    // draft/persisted are seeded with every schema key, so indexed access is
    // total; the assertions only satisfy noUncheckedIndexedAccess.
    const total = (v: SettingValue | undefined): SettingValue => v as SettingValue;
    return {
      draft,
      persisted,
      dirtyKeys,
      restartPending,
      isDirty: (key) => draft[key] !== persisted[key],
      dirtyCount: dirtyKeys.length,
      setValue(key, next) {
        setDraft((prev) => ({ ...prev, [key]: next }));
      },
      apply() {
        const changed: SettingEntry[] = SETTINGS_ENTRIES.filter((e) => draft[e.key] !== persisted[e.key]);
        if (changed.length === 0) return;
        setPersisted((prev) => {
          const next = { ...prev };
          for (const e of changed) next[e.key] = total(draft[e.key]);
          // Persist only overrides that differ from the schema default —
          // the store stays minimal and "reset to default" truly clears.
          const overrides: Record<string, SettingValue> = {};
          for (const entry of SETTINGS_ENTRIES) {
            const current = total(next[entry.key]);
            if (current !== entry.defaultValue) overrides[entry.key] = current;
          }
          persistence.save(overrides);
          return next;
        });
        setRestartPending(changed.filter((e) => e.restartRequired).map((e) => e.key));
      },
      revert(key) {
        setDraft((prev) => {
          if (key === undefined) return { ...persisted };
          const next = { ...prev };
          next[key] = total(persisted[key]);
          return next;
        });
      },
      resetField(key) {
        const entry = SETTINGS_ENTRIES.find((e) => e.key === key);
        if (!entry) return;
        setDraft((prev) => ({ ...prev, [key]: entry.defaultValue }));
      },
      resetArea(area) {
        const areaEntries = SETTINGS_ENTRIES.filter((e) => e.screen === area);
        if (areaEntries.length === 0) return;
        setDraft((prev) => {
          const next = { ...prev };
          for (const e of areaEntries) next[e.key] = e.defaultValue;
          return next;
        });
      },
      value: (key) => total(draft[key]),
    };
  }, [draft, persisted, restartPending, persistence]);

  return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>;
}

export function useSettings(): SettingsContextValue {
  const ctx = useContext(SettingsContext);
  if (ctx === null) throw new Error("useSettings must be used inside <SettingsProvider>");
  return ctx;
}
