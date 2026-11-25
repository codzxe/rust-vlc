// FX6-A05 (wave FX6, FIX-PLAN §3 F1.2) — the webview half of the frame
// lane, verified + locked. Two halves, mirroring the subject file:
//
// 1. THE putImageData LANE (the W15-verified canvas, pinned further):
//    the geometry discipline (resize-on-dimension-change — the presenter
//    sizes the bitmap to the FRAME, putImageData is 1:1) and the REPEAT
//    record's honest draw (the record carries the full payload;
//    re-presenting the identical frame IS honoring the flag) — the two
//    behaviors the existing MediaSurface.test.tsx five pins leave open.
//
// 2. THE FRAME-COUNTER READOUT (F1.2's "a frame counter", honestly
//    scoped): presented + skipped are THIS webview's own counts at the
//    draw call and the refusal path; the last frame id is the
//    engine-assigned id riding the LUF1 record header (engine truth
//    through the existing transport). The engine's produced/shipped/
//    dropped-at-mailbox counters do NOT ride the transport in this wave
//    (the FX6-A03 seam) — these tests pin that nothing fabricates them
//    client-side: the readout shows only what the webview can see.
//
// Fixture posture mirrors MediaSurface.test.tsx (the same fake
// __TAURI_INTERNALS__ injection + the recorded 2D context — jsdom ships
// no canvas rasterizer; the putImageData calls are asserted at the seam).
// The LUF1 byte-layout encoder is duplicated per-file on purpose — the
// per-file inline fixtures are the established convention here (the
// MediaSurface.test.tsx header documents the same mirror posture; the
// audit's shared-encoder followup stands, see the report).

import { render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ClientProvider } from "../../client/ClientProvider";
import type { CommandRequest, CommandResponse, CommandTransport } from "../../client/transport";
import { LUF1 } from "../../client/frame-channel";
import { FrameCanvasSurface, MediaSurface } from "./MediaSurface";

// ---------------------------------------------------------------------------
// Fixtures (the MediaSurface.test.tsx mirror — see the header note).
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

/** Current readout value for one counter (null when the readout is absent). */
function statValue(name: string): string | null {
  const target = document.querySelector(`[data-frame-stat-value="${name}"]`);
  return target === null ? null : (target.textContent ?? "");
}

/** Deliver one shell channel message (record bytes, index-sequenced). */
function deliverRecord(internals: FakeInternals, index: number, record: Uint8Array): void {
  internals.deliver(41, { message: record.slice().buffer, index });
}

describe("MediaSurface — FX6-A05: the putImageData lane, verified + locked", () => {
  it("resizes the canvas bitmap when the frame's dimensions change (1:1, never scaled)", async () => {
    const internals = new FakeInternals();
    installInternals(internals);
    const puts = mockCanvas2d();
    render(
      <ClientProvider transport={new RecordingTransport()}>
        <FrameCanvasSurface />
      </ClientProvider>,
    );

    const canvas = document.querySelector("canvas") as HTMLCanvasElement;
    // Two frames, two geometries (3×2 then 2×3 — both 6 pixels, so the
    // same payload serves both: only the header's geometry differs).
    deliverRecord(
      internals,
      0,
      encodeRecord({ formatCode: LUF1.FORMAT_RGBA8, repeat: false, width: 3, height: 2, frameId: 0 }, RGBA_3X2),
    );
    await vi.waitFor(() => expect(puts).toHaveLength(1));
    expect(canvas.width).toBe(3);
    expect(canvas.height).toBe(2);

    deliverRecord(
      internals,
      1,
      encodeRecord({ formatCode: LUF1.FORMAT_RGBA8, repeat: false, width: 2, height: 3, frameId: 1 }, RGBA_3X2),
    );
    await vi.waitFor(() => expect(puts).toHaveLength(2));

    // The bitmap follows the FRAME's dimensions (putImageData is 1:1 — the
    // CSS class scales into the surface, the rasterizer never does), and
    // each ImageData is sized to its own frame's geometry.
    expect(canvas.width).toBe(2);
    expect(canvas.height).toBe(3);
    expect(puts[0]).toMatchObject({ width: 3, height: 2 });
    expect(puts[1]).toMatchObject({ width: 2, height: 3 });
    // The 1:1 discipline: the blitted byte count is exactly w×h×4.
    expect(puts[1]!.data.length).toBe(2 * 3 * 4);
    expect(Array.from(puts[1]!.data)).toEqual(Array.from(RGBA_3X2));
  });

  it("draws REPEAT records like any other (the record carries the full payload)", async () => {
    const internals = new FakeInternals();
    installInternals(internals);
    const puts = mockCanvas2d();
    render(
      <ClientProvider transport={new RecordingTransport()}>
        <FrameCanvasSurface />
      </ClientProvider>,
    );

    // A repeat-flagged record re-presents the previous slot — the flag
    // bit rides the header (bit 0), the payload is complete, and the
    // presenter blits it exactly like a fresh frame.
    deliverRecord(
      internals,
      0,
      encodeRecord({ formatCode: LUF1.FORMAT_RGBA8, repeat: true, width: 3, height: 2, frameId: 2 }, RGBA_3X2),
    );
    await vi.waitFor(() => expect(puts).toHaveLength(1));
    expect(puts[0]).toMatchObject({ width: 3, height: 2 });
    expect(Array.from(puts[0]!.data)).toEqual(Array.from(RGBA_3X2));

    // And a fresh record after the repeat still renders — the repeat
    // never wedges the lane.
    deliverRecord(
      internals,
      1,
      encodeRecord({ formatCode: LUF1.FORMAT_RGBA8, repeat: false, width: 3, height: 2, frameId: 3 }, RGBA_3X2),
    );
    await vi.waitFor(() => expect(puts).toHaveLength(2));
    expect(Array.from(puts[1]!.data)).toEqual(Array.from(RGBA_3X2));
  });
});

describe("MediaSurface — FX6-A05: the frame-counter readout, surfaced honestly", () => {
  it("mounts only in the desktop shell, zeroed — the last-id cell stays empty before the first frame", () => {
    // Browser mode first: NO readout — the frame lane (and its counters)
    // never mounts outside the shell; the browser's honest empty surface
    // stays exactly as it was.
    clearInternals();
    const { unmount } = render(
      <ClientProvider transport={new RecordingTransport()}>
        <FrameCanvasSurface />
      </ClientProvider>,
    );
    expect(document.querySelector("[data-frame-stats]")).toBeNull();
    unmount();

    // Shell mode: the readout mounts beside the canvas through the full
    // MediaSurface composite (the video half's own mount path).
    const internals = new FakeInternals();
    installInternals(internals);
    render(
      <ClientProvider transport={new RecordingTransport()}>
        <MediaSurface source={null} playing={false} />
      </ClientProvider>,
    );
    const readout = document.querySelector("[data-frame-stats]");
    expect(readout).not.toBeNull();
    expect(readout).toHaveAttribute("data-frame-stats", "webview");
    // Zeroed is the honest pre-bind state: 0 frames drawn, 0 refused.
    expect(statValue("presented")).toBe("0");
    expect(statValue("skipped")).toBe("0");
    // The last-id cell is EMPTY, never "0": the engine assigns id 0 to
    // the FIRST record (apps/desktop/src/bridge.rs next_frame_id starts
    // at 0), so a "0" here would claim a frame was drawn.
    expect(statValue("last-id")).toBe("");
    // The labels flow through the lane's pre-registration i18n seam.
    expect(readout?.textContent).toContain("Frames presented");
    expect(readout?.textContent).toContain("Last frame id");
    expect(readout?.textContent).toContain("Skipped records");
  });

  it("surfaces the presented count and the engine-assigned last frame id", async () => {
    const internals = new FakeInternals();
    installInternals(internals);
    mockCanvas2d();
    render(
      <ClientProvider transport={new RecordingTransport()}>
        <FrameCanvasSurface />
      </ClientProvider>,
    );

    deliverRecord(
      internals,
      0,
      encodeRecord({ formatCode: LUF1.FORMAT_RGBA8, repeat: false, width: 3, height: 2, frameId: 7 }, RGBA_3X2),
    );
    await vi.waitFor(() => expect(statValue("presented")).toBe("1"));
    expect(statValue("last-id")).toBe("7");

    deliverRecord(
      internals,
      1,
      encodeRecord({ formatCode: LUF1.FORMAT_RGBA8, repeat: false, width: 3, height: 2, frameId: 8 }, RGBA_3X2),
    );
    await vi.waitFor(() => expect(statValue("presented")).toBe("2"));
    // The last frame id is the LATEST record's id (a 0-assignment
    // engine counter visible through the transport — engine truth, not
    // a client-side estimate).
    expect(statValue("last-id")).toBe("8");
    expect(statValue("skipped")).toBe("0");
  });

  it("counts skipped records without counting them presented (the drift-defense path)", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const internals = new FakeInternals();
    installInternals(internals);
    mockCanvas2d();
    render(
      <ClientProvider transport={new RecordingTransport()}>
        <FrameCanvasSurface />
      </ClientProvider>,
    );

    // One good record, one corrupted (magic destroyed), one good again.
    deliverRecord(
      internals,
      0,
      encodeRecord({ formatCode: LUF1.FORMAT_RGBA8, repeat: false, width: 3, height: 2, frameId: 5 }, RGBA_3X2),
    );
    const bad = encodeRecord(
      { formatCode: LUF1.FORMAT_RGBA8, repeat: false, width: 3, height: 2, frameId: 6 },
      RGBA_3X2,
    );
    bad[0] = 0x58;
    deliverRecord(internals, 1, bad);
    deliverRecord(
      internals,
      2,
      encodeRecord({ formatCode: LUF1.FORMAT_RGBA8, repeat: false, width: 3, height: 2, frameId: 9 }, RGBA_3X2),
    );

    await vi.waitFor(() => expect(statValue("presented")).toBe("2"));
    // The refusal is counted where it happened (the webview's own skip
    // — never silently absorbed into the presented count).
    expect(statValue("skipped")).toBe("1");
    // The last frame id is the last DRAWN record's id (the corrupted
    // record's id never reached the counter).
    expect(statValue("last-id")).toBe("9");
    expect(warn).toHaveBeenCalledTimes(1);
    expect(warn.mock.calls[0]?.[0]).toContain("skipping a record");
  });
});
