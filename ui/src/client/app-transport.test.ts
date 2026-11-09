import { afterEach, describe, expect, it, vi } from "vitest";
import type { CommandRequest, CommandResponse } from "../generated/commands";
import { DemoTransport } from "./demo-transport";
import type { CommandTransport } from "./transport";
import { isDesktopShell, selectAppTransport } from "./app-transport";
import { TauriTransport } from "./tauri-transport";

// Issue #3 (4.H1 handoff a): the ONE host probe that decides which
// transport the app runs on. These tests pin the selection rule and the
// honesty rule (exactly one visible log line stating the choice) — the
// real webview cells remain ADR-004/INFRA-BLOCKED, so the desktop half
// drives the same injected-internals surface Tauri 2.x provides.

/** The webview-host surface Tauri 2.x injects (the probe's only input).
 * Structural stand-in for the module-private TauriInternals shape. */
interface FakeInternals {
  invoke(command: string, payload?: unknown, options?: unknown): Promise<unknown>;
  transformCallback(callback: (response: unknown) => void, once?: boolean): number;
}

type Globals = { __TAURI_INTERNALS__?: FakeInternals };

const OPEN_RESPONSE: CommandResponse = {
  gen: 1,
  requestId: 7,
  result: { completed: { kind: "unit" } },
};

/** Install a minimal internals surface; record every invoke. */
function installInternals(): { invokes: Array<{ command: string; payload?: unknown }> } {
  const invokes: Array<{ command: string; payload?: unknown }> = [];
  const internals: FakeInternals = {
    invoke: (command: string, payload?: unknown) => {
      invokes.push({ command, payload });
      return Promise.resolve(OPEN_RESPONSE);
    },
    transformCallback: () => 1,
  };
  const globals = globalThis as Globals;
  globals.__TAURI_INTERNALS__ = internals;
  return { invokes };
}

const globals = globalThis as Globals;
const savedInternals = globals.__TAURI_INTERNALS__;

afterEach(() => {
  // jsdom has no internals by default; restore whatever was there.
  if (savedInternals === undefined) {
    delete globals.__TAURI_INTERNALS__;
  } else {
    globals.__TAURI_INTERNALS__ = savedInternals;
  }
});

describe("selectAppTransport — the host probe (4.H1 handoff a)", () => {
  it("outside the shell: browser fallback runs the demo simulation, announced once", () => {
    delete globals.__TAURI_INTERNALS__;
    const log = vi.fn();

    const app = selectAppTransport({ log });

    expect(app.origin).toBe("browser-fallback");
    expect(app.transport).toBeInstanceOf(DemoTransport);
    expect(app.label).toMatch(/browser fallback/);
    expect(app.label).toMatch(/simulated engine/);
    // The honesty rule: exactly ONE line, stating what is live.
    expect(log).toHaveBeenCalledTimes(1);
    expect(log).toHaveBeenCalledWith(app.label);
  });

  it("a custom browserFallback factory is honored (the dev-shell seam)", () => {
    delete globals.__TAURI_INTERNALS__;
    const fallback: CommandTransport = new DemoTransport();

    const app = selectAppTransport({ browserFallback: () => fallback, log: () => {} });

    expect(app.origin).toBe("browser-fallback");
    expect(app.transport).toBe(fallback);
  });

  it("inside the shell: the REAL TauriTransport over the single lumen_dispatch command", async () => {
    const { invokes } = installInternals();

    const app = selectAppTransport({ log: () => {} });

    expect(app.origin).toBe("desktop-shell");
    expect(app.transport).toBeInstanceOf(TauriTransport);
    expect(app.label).toMatch(/desktop shell/);
    expect(app.label).toMatch(/real engine/);
    // The chosen transport really is the dispatch-path transport: a send
    // crosses the injected internals as `lumen_dispatch` with the request
    // under the Rust parameter name.
    const request: CommandRequest = { requestId: 7, cmd: "session.close" };
    await app.transport.send(request);
    const dispatch = invokes.find((call) => call.command === "lumen_dispatch");
    expect(dispatch?.payload).toEqual({ request });
  });

  it("the choice is announced exactly once on the desktop path too", () => {
    installInternals();
    const log = vi.fn();

    const app = selectAppTransport({ log });

    expect(log).toHaveBeenCalledTimes(1);
    expect(log).toHaveBeenCalledWith(app.label);
  });

  it("the desktop probe sees the same backend it hands the transport (no drift)", async () => {
    const { invokes } = installInternals();
    const app = selectAppTransport({ log: () => {} });

    // The probe ran once (the selection); a send must NOT re-probe — the
    // backend captured at selection is the one used.
    expect(invokes).toHaveLength(0);
    await app.transport.send({ requestId: 1, cmd: "system.list-commands" });
    expect(invokes).toHaveLength(1);
    expect(invokes[0]?.command).toBe("lumen_dispatch");
  });
});

describe("isDesktopShell — the chrome's host read", () => {
  it("matches the probe both ways", () => {
    delete globals.__TAURI_INTERNALS__;
    expect(isDesktopShell()).toBe(false);

    installInternals();
    expect(isDesktopShell()).toBe(true);
  });
});
