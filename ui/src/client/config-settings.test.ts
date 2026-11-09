import { describe, expect, it, vi } from "vitest";
import type { CommandRequest, CommandResponse, Event } from "../generated/commands";
import type { CommandTransport } from "./transport";
import type { SettingEntry, SettingValue } from "../screens/settings/generated/schema";
import {
  applySettingChanged,
  createConfigSettingsPersistence,
  splitSettingKey,
} from "./config-settings";

// Issue #3 (4.H1 handoff d): the REAL settings write path — the engine's
// config layer over the command transport. These tests pin the wire
// round-trip (config.get-setting hydrate, config.set-setting write with
// the engine's applied value read back, config.reset-setting for reverted
// keys), the write-only-what-changed discipline, and the
// `config.setting-changed` event fold. No webview is needed: the
// transport is scripted at the seam.

/** A transport that records every request and answers from a script. */
class ScriptedTransport implements CommandTransport {
  readonly sent: CommandRequest[] = [];
  private readonly listeners = new Set<(event: Event) => void>();
  private readonly script: Array<(request: CommandRequest) => CommandResponse> = [];

  /** Queue the next answer (sync or promise-wrapped both work). */
  answer(handler: (request: CommandRequest) => CommandResponse): void {
    this.script.push(handler);
  }

  send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    const handler = this.script.shift();
    if (handler === undefined) {
      return Promise.reject(new Error("scripted transport: no answer queued"));
    }
    return Promise.resolve(handler(request));
  }

  subscribe(listener: (event: Event) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  /** Test helper: the engine emits one event. */
  emit(event: Event): void {
    for (const listener of this.listeners) listener(event);
  }

  get listenerCount(): number {
    return this.listeners.size;
  }
}

/** The wire's setting value (generated commands.ts): mutable arrays. */
type WireValue = boolean | number | string | string[];

const settingResult = (value: WireValue): CommandResponse => ({
  gen: 1,
  requestId: 0,
  result: {
    completed: { kind: "setting-result", schemaMeta: {}, value },
  },
});

const settingApplied = (applied: WireValue): CommandResponse => ({
  gen: 1,
  requestId: 0,
  result: {
    completed: { kind: "setting-applied", applied, restartRequired: false },
  },
});

const notExposed: CommandResponse = {
  gen: 1,
  requestId: 0,
  result: {
    error: {
      code: "feature-not-exposed",
      details: { cmd: "config.get-setting" },
      message: "not served by this host",
      stage: "control",
    },
  },
};

/** A minimal-but-real schema entry (the generator's SettingEntry shape). */
const entry = (
  key: string,
  defaultValue: SettingValue,
  extra: Partial<SettingEntry> = {},
): SettingEntry => ({
  key,
  type: typeof defaultValue === "boolean" ? "bool" : "string",
  description: `settings.${key}`,
  defaultValue,
  defaultRef: "",
  restartRequired: false,
  advanced: false,
  featureGate: "",
  featureExposed: true,
  enumeration: [],
  screen: "interface",
  section: "general",
  order: 1,
  cliFlag: "",
  since: "0.1.0",
  ...extra,
});

const ENTRIES: readonly SettingEntry[] = [
  entry("interface.theme", "dark"),
  entry("interface.start-maximized", false),
];

describe("splitSettingKey", () => {
  it("splits dotted schema keys; refuses degenerate shapes", () => {
    expect(splitSettingKey("interface.theme")).toEqual({ area: "interface", key: "theme" });
    expect(splitSettingKey("theme")).toBeNull();
    expect(splitSettingKey(".theme")).toBeNull();
    expect(splitSettingKey("interface.")).toBeNull();
  });
});

describe("createConfigSettingsPersistence — hydration (config.get-setting)", () => {
  it("reads every schema key and keeps only engine values that differ from the schema default", async () => {
    const transport = new ScriptedTransport();
    // Theme answers "light" (≠ default "dark"); start-maximized answers
    // its default false → NOT an override (the provider's store shape).
    transport.answer(() => settingResult("light"));
    transport.answer(() => settingResult(false));
    const settings = createConfigSettingsPersistence(transport, ENTRIES);

    await settings.hydrate();

    expect(settings.hydrated).toBe(true);
    expect(settings.load()).toEqual({ "interface.theme": "light" });
    const cmds = transport.sent.map((r) => r.cmd);
    expect(cmds).toEqual(["config.get-setting", "config.get-setting"]);
    const areas = transport.sent.map(
      (r) => r.cmd === "config.get-setting" && r.args,
    );
    expect(areas).toContainEqual({ area: "interface", key: "theme" });
    expect(areas).toContainEqual({ area: "interface", key: "start-maximized" });
  });

  it("a host that does not serve config.* leaves defaults standing, with one honest log line", async () => {
    const transport = new ScriptedTransport();
    transport.answer(() => notExposed);
    transport.answer(() => notExposed);
    const log = vi.fn();
    const settings = createConfigSettingsPersistence(transport, ENTRIES, { log });

    await settings.hydrate();

    expect(settings.load()).toEqual({});
    expect(log).toHaveBeenCalledTimes(1);
    expect(log).toHaveBeenCalledWith(expect.stringMatching(/no readable settings/));
  });
});

describe("createConfigSettingsPersistence — writes (config.set-setting / reset-setting)", () => {
  it("save writes ONLY changed keys and resets what reverted", async () => {
    const transport = new ScriptedTransport();
    // Hydration: theme "light" (override), start-maximized default false.
    transport.answer(() => settingResult("light"));
    transport.answer(() => settingResult(false));
    const settings = createConfigSettingsPersistence(transport, ENTRIES);
    await settings.hydrate();

    // Apply: theme stays "light" (unchanged → never rewritten),
    // start-maximized flips true (→ set), and the theme override is
    // dropped entirely (→ reset, because it reverted to the default).
    transport.answer(() => settingApplied(true));
    transport.answer(() => settingResult("dark"));
    settings.save({ "interface.start-maximized": true });

    const setCalls = transport.sent.filter((r) => r.cmd === "config.set-setting");
    const resetCalls = transport.sent.filter((r) => r.cmd === "config.reset-setting");
    expect(setCalls).toHaveLength(1);
    expect(setCalls[0]?.args).toEqual({
      area: "interface",
      key: "start-maximized",
      value: true,
    });
    expect(resetCalls).toHaveLength(1);
    expect(resetCalls[0]?.args).toEqual({ area: "interface", key: "theme" });
  });

  it("a write round-trips: the engine's applied value is reported through onWriteResult", async () => {
    const transport = new ScriptedTransport();
    const onWriteResult = vi.fn();
    const settings = createConfigSettingsPersistence(transport, ENTRIES, { onWriteResult });
    // No hydrate needed: save() diffs against the empty last-known set.
    transport.answer(() => settingApplied("light"));
    settings.save({ "interface.theme": "light" });
    // The write is dispatched asynchronously — let it settle.
    await vi.waitFor(() => expect(onWriteResult).toHaveBeenCalledTimes(1));

    expect(onWriteResult).toHaveBeenCalledWith({
      kind: "applied",
      key: "interface.theme",
      value: "light",
    });
    expect(settings.pendingWrites).toBe(0);
  });

  it("a typed engine refusal is reported as failed with the error code — never swallowed", async () => {
    const transport = new ScriptedTransport();
    const onWriteResult = vi.fn();
    const settings = createConfigSettingsPersistence(transport, ENTRIES, { onWriteResult });
    transport.answer(() => notExposed);
    settings.save({ "interface.theme": "light" });
    await vi.waitFor(() => expect(onWriteResult).toHaveBeenCalledTimes(1));

    expect(onWriteResult).toHaveBeenCalledWith({
      kind: "failed",
      key: "interface.theme",
      reason: "feature-not-exposed",
    });
  });

  it("an unreachable transport is a failed write, not a crash", async () => {
    const transport = new ScriptedTransport(); // nothing queued
    const onWriteResult = vi.fn();
    const settings = createConfigSettingsPersistence(transport, ENTRIES, { onWriteResult });

    settings.save({ "interface.theme": "light" });
    await vi.waitFor(() => expect(onWriteResult).toHaveBeenCalledTimes(1));

    expect(onWriteResult).toHaveBeenCalledWith({
      kind: "failed",
      key: "interface.theme",
      reason: "transport unreachable",
    });
  });
});

describe("createConfigSettingsPersistence — the live engine feed", () => {
  it("config.setting-changed events fold into the snapshot (engine-side moves are not rewritten later)", async () => {
    const transport = new ScriptedTransport();
    const settings = createConfigSettingsPersistence(transport, ENTRIES);
    transport.emit({
      seq: 1,
      kind: "config.setting-changed",
      area: "interface",
      key: "theme",
      layer: "cli-flag",
      value: "light",
    });

    expect(settings.load()).toEqual({ "interface.theme": "light" });

    // A later Apply that keeps "light" writes NOTHING (the diff base
    // already knows the engine moved it).
    settings.save({ "interface.theme": "light" });
    expect(transport.sent).toHaveLength(0);
  });

  it("dispose releases the event subscription (test hygiene)", () => {
    const transport = new ScriptedTransport();
    const settings = createConfigSettingsPersistence(transport, ENTRIES);
    expect(transport.listenerCount).toBe(1);

    settings.dispose();

    expect(transport.listenerCount).toBe(0);
  });
});

describe("applySettingChanged — the pure fold", () => {
  it("a value equal to the schema default clears the override row", () => {
    const next = applySettingChanged(
      { "interface.theme": "light" },
      {
        seq: 2,
        kind: "config.setting-changed",
        area: "interface",
        key: "theme",
        layer: "engine",
        value: "dark",
      },
      ENTRIES,
    );
    expect(next).toEqual({});
  });

  it("unknown keys and unrelated events are ignored — never invented", () => {
    expect(
      applySettingChanged(
        {},
        {
          seq: 3,
          kind: "config.setting-changed",
          area: "no-such",
          key: "key",
          layer: "engine",
          value: true,
        },
        ENTRIES,
      ),
    ).toEqual({});
    expect(applySettingChanged({}, { seq: 4, kind: "playback.end-of-media" }, ENTRIES)).toEqual(
      {},
    );
  });
});
