// The REAL settings write path (issue #3 — the 4.H1 handoff d): a
// `SettingsPersistence` whose store is the ENGINE's config layer over
// the command transport — `config.get-setting` to hydrate,
// `config.set-setting` / `config.reset-setting` to write (doc 05
// §2.11). This replaces the localStorage MOCK (persistence.ts) in the
// desktop shell; the browser dev shell keeps localStorage (the demo
// transport honestly answers `feature-not-exposed` for config.*).
//
// Round-trip discipline: the provider's Apply hands the FULL override
// set (values that differ from the schema default), so this module
// diffs against its own last-known set and writes ONLY the changed
// keys — an unchanged setting is never rewritten, and a key that
// reverted to default is RESET (`config.reset-setting`), not written.
// Engine-side changes (`config.setting-changed` events) fold into the
// same snapshot, so a later Apply never rewrites a key the engine
// already moved.
//
// Sync/async asymmetry (stated, not hidden): `SettingsPersistence`
// load() is synchronous, so hydration is a separate AWAITED step
// (hydrate()); the settings screen gates its provider mount on it so
// the UI never renders schema defaults over engine values it has not
// read yet. Writes are dispatched asynchronously; every write's
// outcome is reported through onWriteResult (success carries the
// engine's applied value — the round-trip's read-back half).

import type {
  CommandRequest,
  CommandResponse,
  CommandTransport,
  Event,
} from "./transport";
import type { SettingEntry, SettingValue } from "../screens/settings/generated/schema";
import type { SettingsPersistence } from "../screens/settings/persistence";

/** The wire's setting value (generated commands.ts): no readonly arrays. */
type WireSettingValue = boolean | number | string | string[];

const toWireValue = (value: SettingValue): WireSettingValue =>
  Array.isArray(value) ? [...value] : value;

/** Split a dotted schema key ("interface.theme") into the catalog's area+key. */
export function splitSettingKey(
  key: string,
): { area: string; key: string } | null {
  const dot = key.indexOf(".");
  if (dot <= 0 || dot === key.length - 1) return null;
  return { area: key.slice(0, dot), key: key.slice(dot + 1) };
}

/** One write's outcome, reported through onWriteResult. */
export type ConfigWriteResult =
  | { readonly kind: "applied"; readonly key: string; readonly value: SettingValue }
  | { readonly kind: "reset"; readonly key: string }
  | { readonly kind: "failed"; readonly key: string; readonly reason: string };

export interface ConfigSettingsOptions {
  /** Diagnostic surface for every write's outcome (never swallowed). */
  readonly onWriteResult?: (result: ConfigWriteResult) => void;
  /** Log sink for the one hydration-gap line (defaults to console.debug). */
  readonly log?: (message: string) => void;
}

export interface ConfigSettings extends SettingsPersistence {
  /** Fetch every schema key's engine value (bounded by the schema). */
  hydrate(): Promise<void>;
  /** True once hydrate() has completed (any outcome). */
  readonly hydrated: boolean;
  /** Writes dispatched but not yet settled (diagnostics). */
  readonly pendingWrites: number;
  /** Release the setting-changed subscription (test hygiene). */
  dispose(): void;
}

/**
 * Build the engine-backed settings persistence. `entries` is the
 * generated settings schema (the caller owns that import site).
 */
export function createConfigSettingsPersistence(
  transport: CommandTransport,
  entries: readonly SettingEntry[],
  options: ConfigSettingsOptions = {},
): ConfigSettings {
  const log = options.log ?? ((message: string) => console.debug(message));
  // The last-known override set — the diff base for writes.
  let overrides: Record<string, SettingValue> = {};
  let hydrated = false;
  let pending = 0;
  let nextRequestId = 0;
  const requestId = (): number => {
    nextRequestId += 1;
    return nextRequestId;
  };

  const send = (request: CommandRequest): Promise<CommandResponse> =>
    transport.send(request);

  /** Query one key; null = not readable (error/not-exposed/shape). */
  const readKey = async (
    entry: SettingEntry,
  ): Promise<SettingValue | null> => {
    const split = splitSettingKey(entry.key);
    if (split === null) return null;
    let response: CommandResponse;
    try {
      response = await send({
        requestId: requestId(),
        cmd: "config.get-setting",
        args: { area: split.area, key: split.key },
      });
    } catch {
      return null; // transport unreachable — the honest unknown
    }
    if (
      "completed" in response.result &&
      response.result.completed.kind === "setting-result"
    ) {
      return response.result.completed.value;
    }
    return null; // typed error (e.g. feature-not-exposed in the browser)
  };

  /** Write one key; reports the outcome through onWriteResult. */
  const writeKey = (key: string, value: SettingValue): void => {
    const split = splitSettingKey(key);
    if (split === null) {
      options.onWriteResult?.({
        kind: "failed",
        key,
        reason: "key is not in area.key form",
      });
      return;
    }
    pending += 1;
    void send({
      requestId: requestId(),
      cmd: "config.set-setting",
      args: { area: split.area, key: split.key, value: toWireValue(value) },
    }).then(
      (response) => {
        pending -= 1;
        if (
          "completed" in response.result &&
          response.result.completed.kind === "setting-applied"
        ) {
          // The engine's applied value is the round-trip's read-back.
          options.onWriteResult?.({
            kind: "applied",
            key,
            value: response.result.completed.applied,
          });
        } else if ("error" in response.result) {
          options.onWriteResult?.({
            kind: "failed",
            key,
            reason: response.result.error.code,
          });
        }
      },
      () => {
        pending -= 1;
        options.onWriteResult?.({
          kind: "failed",
          key,
          reason: "transport unreachable",
        });
      },
    );
  };

  /** Reset one key to its layer default (a reverted override). */
  const resetKey = (key: string): void => {
    const split = splitSettingKey(key);
    if (split === null) {
      options.onWriteResult?.({
        kind: "failed",
        key,
        reason: "key is not in area.key form",
      });
      return;
    }
    pending += 1;
    void send({
      requestId: requestId(),
      cmd: "config.reset-setting",
      args: { area: split.area, key: split.key },
    }).then(
      (response) => {
        pending -= 1;
        if ("completed" in response.result) {
          options.onWriteResult?.({ kind: "reset", key });
        } else if ("error" in response.result) {
          options.onWriteResult?.({
            kind: "failed",
            key,
            reason: response.result.error.code,
          });
        }
      },
      () => {
        pending -= 1;
        options.onWriteResult?.({
          kind: "failed",
          key,
          reason: "transport unreachable",
        });
      },
    );
  };

  // Engine-side changes fold into the snapshot (the same fold the
  // settings store applies to its own state — one rule, two stores).
  const unsubscribe = transport.subscribe((event: Event) => {
    overrides = applySettingChanged(overrides, event, entries);
  });

  const persistence: ConfigSettings = {
    load() {
      // The hydrated snapshot. Before hydrate() completes this is {} —
      // the settings screen never mounts the provider that early.
      return { ...overrides };
    },
    save(next) {
      // Diff against the last-known set: write only what changed,
      // reset what reverted. The snapshot updates immediately (the
      // writes are already dispatched); outcomes surface via
      // onWriteResult.
      const changed: string[] = [];
      const reverted: string[] = [];
      for (const key of Object.keys(next)) {
        if (overrides[key] !== next[key]) changed.push(key);
      }
      for (const key of Object.keys(overrides)) {
        if (!(key in next)) reverted.push(key);
      }
      overrides = { ...next };
      for (const key of changed) {
        const value = next[key];
        if (value !== undefined) writeKey(key, value);
      }
      for (const key of reverted) resetKey(key);
    },
    async hydrate() {
      const results = await Promise.all(
        entries.map(async (entry) => [entry, await readKey(entry)] as const),
      );
      const loaded: Record<string, SettingValue> = {};
      let readable = 0;
      for (const [entry, value] of results) {
        if (value === null) continue;
        readable += 1;
        // Override = engine value that differs from the schema default
        // (the provider's own store shape).
        if (value !== entry.defaultValue) loaded[entry.key] = value;
      }
      overrides = loaded;
      hydrated = true;
      if (readable === 0) {
        log(
          "lumen config settings: engine reported no readable settings (host does not serve config.*) — schema defaults stand",
        );
      }
    },
    get hydrated() {
      return hydrated;
    },
    get pendingWrites() {
      return pending;
    },
    dispose() {
      unsubscribe();
    },
  };
  return persistence;
}

/**
 * Fold one `config.setting-changed` event into an override set (pure;
 * both the write-path snapshot and the settings store consume it, so
 * the fold rule exists exactly once). A value equal to the schema
 * default clears the override row; unknown keys are ignored — never
 * invented.
 */
export function applySettingChanged(
  overrides: Readonly<Record<string, SettingValue>>,
  event: Event,
  entries: readonly SettingEntry[],
): Record<string, SettingValue> {
  if (event.kind !== "config.setting-changed") return { ...overrides };
  const full = `${event.area}.${event.key}`;
  const entry = entries.find((candidate) => candidate.key === full);
  if (entry === undefined) return { ...overrides };
  const next = { ...overrides };
  if (event.value === entry.defaultValue) {
    delete next[full];
  } else {
    next[full] = event.value;
  }
  return next;
}
