// w31.4 (lane `ui-canvas`) — the frame-canvas consumer tests: the
// surface mounts ONLY inside the desktop shell (the __TAURI_INTERNALS__
// probe), binds the shell's shipper through the app's own transport
// (session.set-frame-channel with the allocated id), and renders
// decoded LUF1 records into the canvas via a mocked 2D context
// (jsdom ships no canvas rasterizer — the presenter's putImageData
// calls are asserted at the seam). The mock posture mirrors
// tauri-transport.test.ts (the internals surface faked where the real
// webview injects it).

import { render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import type { CommandRequest, CommandResponse, CommandTransport } from "../../client/transport";
import { LUF1 } from "../../client/frame-channel";
import { FrameCanvasSurface, MediaSurface } from "./MediaSurface";

// ---------------------------------------------------------------------------
// Fixtures: a recording transport + the fake internals + the LUF1
// encoder (the frame-channel.test.ts mirror).
// ---------------------------------------------------------------------------

/** Records every command; answers the bind's Unit completion. */
class RecordingTransport implements CommandTransport {
  readonly sent: CommandRequest[] = [];

  async send(request: CommandRequest): Promise<CommandResponse> {
    this.sent.push(request);
    return {
      gen: 0,
      requestId: request.requestId,
      result: { completed: { kind: "unit" } },
    };
  }

  subscribe(): () => void {
    return () => {};
  }
}

/** The fake injected internals (transformCallback + the fetch invoke). */
class FakeInternals {
  readonly registered = new Map<number, (response: unknown) => void>();
  readonly unregisterCallback = vi.fn();
  private nextId = 41;

  transformCallback(callback: (response: unknown) => void): number {
    const id = this.nextId;
    this.nextId += 1;
    this.registered.set(id, callback);
    return id;
  }

  invoke(): Promise<unknown> {
    return Promise.reject(new Error("the surface test never fetches (ArrayBuffer form)"));
  }

  /** The shell delivers one channel message. */
  deliver(id: number, frame: unknown): void {
    this.registered.get(id)?.(frame);
  }
}

function installInternals(internals: FakeInternals): void {
  (globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = internals;
}

function clearInternals(): void {
  delete (globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

interface HeaderSpec {
  formatCode: number;
  repeat: boolean;
  width: number;
  height: number;
  frameId: number;
}

function encodeRecord(header: HeaderSpec, payload: Uint8Array): Uint8Array {
  const implied = header.width * header.height * 4;
  const out = new Uint8Array(LUF1.MIN_HEADER_LEN + implied);
  out.set(LUF1.MAGIC, 0);
  const dv = new DataView(out.buffer);
  dv.setUint16(4, LUF1.MIN_HEADER_LEN, true);
  dv.setUint16(6, LUF1.VERSION, true);
  out[8] = header.formatCode;
  out[9] = header.repeat ? 1 : 0;
  dv.setUint32(12, header.width, true);
  dv.setUint32(16, header.height, true);
  dv.setUint32(20, implied, true);
  dv.setBigUint64(24, BigInt(header.frameId), true);
  out.set(payload, LUF1.MIN_HEADER_LEN);
  return out;
}

/** A 3×2 RGBA pattern with distinguishable pixels. */
const RGBA_3X2 = new Uint8Array([
  255, 0, 0, 255, //
  0, 255, 0, 255, //
  0, 0, 255, 255, //
  255, 255, 0, 255, //
  0, 255, 255, 255, //
  255, 0, 255, 255,
]);

interface PutCall {
  width: number;
  height: number;
  data: Uint8ClampedArray;
}

/** A 2D-context fake recording putImageData calls (jsdom has none). */
function mockCanvas2d(): PutCall[] {
  const puts: PutCall[] = [];
  const ctx = {
    createImageData: (width: number, height: number): ImageData =>
      ({
        width,
        height,
        data: new Uint8ClampedArray(width * height * 4),
        colorSpace: "srgb",
      }) as unknown as ImageData,
    putImageData: (image: ImageData): void => {
      puts.push({ width: image.width, height: image.height, data: image.data });
    },
  };
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(
    ctx as unknown as CanvasRenderingContext2D,
  );
  return puts;
}

afterEach(() => {
  clearInternals();
  vi.restoreAllMocks();
});

describe("MediaSurface — the shell-mode split (w31.4)", () => {
  it("browser mode: no canvas, no bind — the honest empty surface stays", async () => {
    clearInternals();
    const transport = new RecordingTransport();
    render(
      <ClientProvider transport={transport}>
        <MediaSurface source={null} playing={false} />
      </ClientProvider>,
    );
    // The video half mounts (src-less, honest); the frame canvas NEVER
    // renders outside the desktop shell.
    expect(document.querySelector("video")).not.toBeNull();
    expect(document.querySelector("canvas")).toBeNull();
    // And nothing was dispatched — the consumer touches no transport in
    // a browser (no set-frame-channel can leak to a demo transport).
    await Promise.resolve();
    expect(transport.sent).toHaveLength(0);
  });

  it("shell mode: mounts the canvas, binds via the app transport, deregisters on unmount", async () => {
    const internals = new FakeInternals();
    installInternals(internals);
    const transport = new RecordingTransport();
    const { unmount } = render(
      <ClientProvider transport={transport}>
        <MediaSurface source={null} playing={false} />
      </ClientProvider>,
    );

    const canvas = document.querySelector("canvas");
    expect(canvas).not.toBeNull();
    expect(canvas).toHaveAttribute("data-media-surface", "frames");
    expect(canvas).toHaveAttribute("aria-hidden", "true");
    // The video half still mounts beside it (the url-source path).
    expect(document.querySelector("video")).not.toBeNull();

    // The bind: the allocated channel id rides the ONE command envelope
    // through the app's own transport.
    await vi.waitFor(() => expect(transport.sent).toHaveLength(1));
    expect(transport.sent[0]).toMatchObject({
      cmd: "session.set-frame-channel",
      args: { channel: 41 },
    });

    // Registration shape: window._<id> exists while mounted.
    expect((globalThis as Record<string, unknown>)["_41"]).toBeTypeOf("function");

    unmount();
    expect((globalThis as Record<string, unknown>)["_41"]).toBeUndefined();
    expect(internals.unregisterCallback).toHaveBeenCalledWith(41);
  });

  it("renders decoded RGBA frames into the canvas (sized to the frame)", async () => {
    const internals = new FakeInternals();
    installInternals(internals);
    const puts = mockCanvas2d();
    render(
      <ClientProvider transport={new RecordingTransport()}>
        <FrameCanvasSurface />
      </ClientProvider>,
    );

    const canvas = document.querySelector("canvas") as HTMLCanvasElement;
    internals.deliver(41, { message: encodeRecord(
      { formatCode: LUF1.FORMAT_RGBA8, repeat: false, width: 3, height: 2, frameId: 1 },
      RGBA_3X2,
    ).slice().buffer, index: 0 });

    await vi.waitFor(() => expect(puts).toHaveLength(1));
    // The bitmap matches the frame (putImageData is 1:1); CSS scales
    // it into the surface (object-fit: contain semantics).
    expect(canvas.width).toBe(3);
    expect(canvas.height).toBe(2);
    expect(puts[0]).toMatchObject({ width: 3, height: 2 });
    expect(Array.from(puts[0]!.data)).toEqual(Array.from(RGBA_3X2));
  });

  it("swizzles BGRA8 frames to RGBA draw-side", async () => {
    const internals = new FakeInternals();
    installInternals(internals);
    const puts = mockCanvas2d();
    render(
      <ClientProvider transport={new RecordingTransport()}>
        <FrameCanvasSurface />
      </ClientProvider>,
    );

    // BGRA payload: the R and B channels swap on the way to the 2D
    // context (the recorded 2D-path cost).
    const bgra = new Uint8Array([
      0, 0, 255, 255, //
      0, 255, 0, 255, //
      255, 0, 0, 255,
    ]);
    internals.deliver(41, { message: encodeRecord(
      { formatCode: LUF1.FORMAT_BGRA8, repeat: false, width: 3, height: 1, frameId: 2 },
      bgra,
    ).slice().buffer, index: 0 });

    await vi.waitFor(() => expect(puts).toHaveLength(1));
    expect(Array.from(puts[0]!.data)).toEqual([
      255, 0, 0, 255, //
      0, 255, 0, 255, //
      0, 0, 255, 255,
    ]);
  });

  it("a malformed record is a console diagnostic, never a crash — the next frame still renders", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const internals = new FakeInternals();
    installInternals(internals);
    const puts = mockCanvas2d();
    render(
      <ClientProvider transport={new RecordingTransport()}>
        <FrameCanvasSurface />
      </ClientProvider>,
    );

    // A corrupted record (magic destroyed) + a truncated one.
    const bad = encodeRecord(
      { formatCode: LUF1.FORMAT_RGBA8, repeat: false, width: 3, height: 2, frameId: 1 },
      RGBA_3X2,
    );
    bad[0] = 0x58;
    internals.deliver(41, { message: bad.slice().buffer, index: 0 });
    internals.deliver(41, { message: bad.slice(0, 30).buffer, index: 1 });
    await vi.waitFor(() => expect(warn).toHaveBeenCalledTimes(2));

    // The channel keeps flowing: the NEXT record renders.
    internals.deliver(41, { message: encodeRecord(
      { formatCode: LUF1.FORMAT_RGBA8, repeat: false, width: 3, height: 2, frameId: 3 },
      RGBA_3X2,
    ).slice().buffer, index: 2 });
    await vi.waitFor(() => expect(puts).toHaveLength(1));
    expect(warn.mock.calls[0]?.[0]).toContain("skipping a record");
  });
});
