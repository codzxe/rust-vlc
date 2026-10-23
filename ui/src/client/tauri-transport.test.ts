import { describe, expect, it, vi } from "vitest";
import type { CommandRequest, CommandResponse, Event } from "../generated/commands";
import {
  LUMEN_DISPATCH_COMMAND,
  LUMEN_EVENT_CHANNEL,
  TauriTransport,
  TauriTransportUnavailableError,
  createTauriTransport,
  detectTauriBackend,
} from "./tauri-transport";
import type { TauriBackend, TauriEventFrame } from "./tauri-transport";

// A scriptable backend — the mockable seam. invoke answers from a queue
// (or rejects), listen records the handler and hands back a spied stop.
class FakeBackend implements TauriBackend {
  readonly invokes: Array<{ command: string; payload?: Record<string, unknown> }> = [];
  private readonly answers: Array<
    { ok: unknown } | { reject: unknown }
  > = [];
  private listenHandler: ((frame: TauriEventFrame) => void) | null = null;
  private readonly unlistenSpies: Array<() => void> = [];

  queue(response: unknown): void {
    this.answers.push({ ok: response });
  }

  queueRejection(rejection: unknown): void {
    this.answers.push({ reject: rejection });
  }

  invoke(command: string, payload?: Record<string, unknown>): Promise<unknown> {
    this.invokes.push({ command, payload });
    const answer = this.answers.shift();
    if (!answer) {
      return Promise.reject(new Error("fake backend: no queued answer"));
    }
    return "ok" in answer
      ? Promise.resolve(answer.ok)
      : Promise.reject(answer.reject);
  }

  listen(
    _event: string,
    handler: (frame: TauriEventFrame) => void,
  ): Promise<() => void> {
    this.listenHandler = handler;
    const stop = vi.fn();
    this.unlistenSpies.push(stop);
    return Promise.resolve(stop);
  }

  /** Test helper: the shell emits one event frame on the channel. */
  emit(payload: unknown): void {
    this.listenHandler?.({ event: LUMEN_EVENT_CHANNEL, id: 1, payload });
  }

  get lastUnlisten(): (() => void) | undefined {
    return this.unlistenSpies[this.unlistenSpies.length - 1];
  }
}

const OPEN_REQUEST: CommandRequest = {
  requestId: 7,
  cmd: "session.open",
  args: { source: { kind: "file", path: "/media/clip.mkv" } },
};

const OPEN_RESPONSE: CommandResponse = {
  gen: 3,
  requestId: 7,
  result: { completed: { kind: "unit" } },
};

const STATE_EVENT: Event = {
  seq: 11,
  kind: "session.state-changed",
  from: "idle",
  to: "opening",
};

describe("TauriTransport (mocked TauriBackend seam)", () => {
  it("routes every send through the single lumen_dispatch command", async () => {
    const backend = new FakeBackend();
    backend.queue(OPEN_RESPONSE);
    const transport = new TauriTransport(backend);

    await transport.send(OPEN_REQUEST);

    expect(backend.invokes).toHaveLength(1);
    expect(backend.invokes[0]?.command).toBe(LUMEN_DISPATCH_COMMAND);
    // The payload key is the Rust parameter name (`request`) — the
    // single-command rule means nothing else is ever addressed.
    expect(backend.invokes[0]?.payload).toEqual({ request: OPEN_REQUEST });
  });

  it("returns the shell's response verbatim", async () => {
    const backend = new FakeBackend();
    backend.queue(OPEN_RESPONSE);
    const transport = new TauriTransport(backend);

    await expect(transport.send(OPEN_REQUEST)).resolves.toEqual(OPEN_RESPONSE);
  });

  it("normalizes a typed CommandError rejection into the error envelope with the request echoed", async () => {
    const backend = new FakeBackend();
    backend.queueRejection({
      code: "not-found",
      details: {},
      message: "no such session",
      stage: "control",
    });
    const transport = new TauriTransport(backend);

    const response = await transport.send({
      requestId: 42,
      cmd: "playback.play",
    });

    expect(response.requestId).toBe(42);
    expect(response.result).toEqual({
      error: {
        code: "not-found",
        details: {},
        message: "no such session",
        stage: "control",
      },
    });
  });

  it("rethrows unexpected IPC failures instead of inventing a response", async () => {
    const backend = new FakeBackend();
    backend.queueRejection("channel closed");
    const transport = new TauriTransport(backend);

    await expect(transport.send(OPEN_REQUEST)).rejects.toThrow(
      /lumen_dispatch IPC failed/,
    );
  });

  it("forwards lumen://event payloads to subscribers as Events", () => {
    const backend = new FakeBackend();
    const transport = new TauriTransport(backend);
    const seen: Event[] = [];
    transport.subscribe((event) => seen.push(event));

    backend.emit(STATE_EVENT);

    expect(seen).toEqual([STATE_EVENT]);
  });

  it("unsubscribe stops the channel listener", async () => {
    const backend = new FakeBackend();
    const transport = new TauriTransport(backend);
    const unsubscribe = transport.subscribe(() => {});
    await Promise.resolve(); // let listen() resolve

    unsubscribe();

    expect(backend.lastUnlisten).toBeDefined();
    expect(backend.lastUnlisten).toHaveBeenCalled();
  });

  it("unsubscribe before the channel resolves still releases it exactly once", async () => {
    const backend = new FakeBackend();
    const transport = new TauriTransport(backend);
    const unsubscribe = transport.subscribe(() => {});
    unsubscribe(); // before listen() resolves

    await Promise.resolve();
    await Promise.resolve();

    expect(backend.lastUnlisten).toBeDefined();
    expect(backend.lastUnlisten).toHaveBeenCalledTimes(1);
  });

  it("surfaces channel-listen failures through the onListenFailure seam", async () => {
    const failing: TauriBackend = {
      invoke: () => Promise.reject(new Error("unused")),
      listen: () => Promise.reject(new Error("no channel")),
    };
    const onListenFailure = vi.fn();
    new TauriTransport(failing, onListenFailure).subscribe(() => {});

    await Promise.resolve();
    await Promise.resolve();

    expect(onListenFailure).toHaveBeenCalledWith(new Error("no channel"));
  });
});

describe("detectTauriBackend / createTauriTransport", () => {
  it("returns null and throws the typed unavailable error outside the shell", () => {
    const globals = globalThis as { __TAURI_INTERNALS__?: unknown };
    const saved = globals.__TAURI_INTERNALS__;
    delete globals.__TAURI_INTERNALS__;
    try {
      expect(detectTauriBackend()).toBeNull();
      expect(() => createTauriTransport()).toThrow(TauriTransportUnavailableError);
    } finally {
      if (saved !== undefined) {
        globals.__TAURI_INTERNALS__ = saved;
      }
    }
  });
});
