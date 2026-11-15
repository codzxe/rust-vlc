// w31.4 (lane `ui-canvas`) — unit tests for the frame-channel client:
// the LUF1 decoder (golden bytes → pixels; the typed refusals) and the
// hand-rolled tauri channel shim (id allocation, window._<id>
// registration, index ordering, the raw-payload fetch dance with a
// mocked invoke). The mock posture mirrors tauri-transport.test.ts —
// no webview exists in this sandbox (INFRA-BLOCKED.md), so the
// internals surface is faked at the same seam the real one injects.

import { describe, expect, it, vi } from "vitest";
import {
  FETCH_CHANNEL_DATA_COMMAND,
  FrameChannelShim,
  FrameWireError,
  LUF1,
  decodeLuf1Record,
  detectFrameChannelBackend,
  parseLuf1Header,
  type FrameChannelBackend,
} from "./frame-channel";

// ---------------------------------------------------------------------------
// Fixtures: the LUF1 encoder (mirror of the prototype's encodeFrame —
// wire.rs's own encoder, byte-for-byte) + a hand-written golden record.
// ---------------------------------------------------------------------------

interface HeaderSpec {
  formatCode: number;
  repeat: boolean;
  width: number;
  height: number;
  frameId: number;
  ptsNs: number;
  emittedNs: number;
  headerLen?: number;
}

function encodeRecord(header: HeaderSpec, payload: Uint8Array): Uint8Array {
  const implied = header.width * header.height * 4;
  expect(payload.length).toBe(implied);
  const headerLen = header.headerLen ?? LUF1.MIN_HEADER_LEN;
  const out = new Uint8Array(headerLen + implied);
  out.set(LUF1.MAGIC, 0);
  const dv = new DataView(out.buffer);
  dv.setUint16(4, headerLen, true);
  dv.setUint16(6, LUF1.VERSION, true);
  out[8] = header.formatCode;
  out[9] = header.repeat ? 1 : 0;
  dv.setUint16(10, 0, true);
  dv.setUint32(12, header.width, true);
  dv.setUint32(16, header.height, true);
  dv.setUint32(20, implied, true);
  dv.setBigUint64(24, BigInt(header.frameId), true);
  dv.setBigInt64(32, BigInt(header.ptsNs), true);
  dv.setBigUint64(40, BigInt(header.emittedNs), true);
  out.set(payload, headerLen);
  return out;
}

/** A 2×2 RGBA test pattern: R, G, B, W (row-major). */
const RGBA_2X2 = new Uint8Array([
  255, 0, 0, 255, //
  0, 255, 0, 255, //
  0, 0, 255, 255, //
  255, 255, 255, 255,
]);

const HEADER_2X2: HeaderSpec = {
  formatCode: LUF1.FORMAT_RGBA8,
  repeat: false,
  width: 2,
  height: 2,
  frameId: 7,
  ptsNs: 1_000_000,
  emittedNs: 1_000_042,
};

/**
 * The hand-written golden record (independent of the encoder above —
 * the header fields spelled out byte by byte so a decoder regression
 * cannot hide behind an encoder bug). Same 2×2 RGBA frame.
 */
const GOLDEN_2X2 = new Uint8Array([
  // magic "LUF1"
  0x4c, 0x55, 0x46, 0x31,
  // header_len = 48, version = 1
  0x30, 0x00, 0x01, 0x00,
  // format = 1 (RGBA8), flags = 0 (no repeat)
  0x01, 0x00,
  // reserved u16 = 0
  0x00, 0x00,
  // width = 2, height = 2
  0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
  // payload_len = 2*2*4 = 16
  0x10, 0x00, 0x00, 0x00,
  // frame_id = 7 (u64 LE)
  0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
  // pts_ns = 1_000_000 (i64 LE)
  0x40, 0x42, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x00,
  // emitted_ns = 1_000_042 (u64 LE)
  0x6a, 0x42, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x00,
  // payload: R G B W
  255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
]);

/** Decode, asserting the typed kind (never a bare message match). */
function expectRefusal(record: Uint8Array, kind: string): void {
  expect(() => decodeLuf1Record(record)).toThrow(FrameWireError);
  try {
    decodeLuf1Record(record);
  } catch (error) {
    expect((error as FrameWireError).kind).toBe(kind);
  }
}

// ---------------------------------------------------------------------------
// The decoder.
// ---------------------------------------------------------------------------

describe("decodeLuf1Record (the wire.rs byte-mirror)", () => {
  it("decodes the hand-written golden record: header fields + pixels", () => {
    const frame = decodeLuf1Record(GOLDEN_2X2);
    expect(frame.header).toEqual({
      headerLen: 48,
      formatCode: 1,
      formatName: "rgba8",
      repeat: false,
      width: 2,
      height: 2,
      payloadLen: 16,
      frameId: 7,
      ptsNs: 1_000_000,
      emittedNs: 1_000_042,
    });
    expect(Array.from(frame.payload)).toEqual(Array.from(RGBA_2X2));
  });

  it("decodes a BGRA8 record (format code 2) with its payload verbatim", () => {
    const bgra = new Uint8Array([
      255, 0, 0, 255, //
      0, 255, 0, 255, //
      0, 0, 255, 255, //
      255, 255, 255, 255,
    ]);
    const record = encodeRecord(
      { ...HEADER_2X2, formatCode: LUF1.FORMAT_BGRA8 },
      bgra,
    );
    const frame = decodeLuf1Record(record);
    expect(frame.header.formatCode).toBe(LUF1.FORMAT_BGRA8);
    expect(frame.header.formatName).toBe("bgra8");
    // The decoder is format-honest: BGRA bytes cross UNTOUCHED (the
    // swizzle is the presenter's cost, not the decoder's).
    expect(Array.from(frame.payload)).toEqual(Array.from(bgra));
  });

  it("carries the repeat flag (bit 0) through", () => {
    const record = encodeRecord({ ...HEADER_2X2, repeat: true }, RGBA_2X2);
    expect(decodeLuf1Record(record).header.repeat).toBe(true);
  });

  it("ignores v1 extension space: header_len 48..=1024 is accepted", () => {
    // A 64-byte header: bytes 48..64 are v1-ignored extension space
    // (wire.rs's forward-compatibility window).
    const record = encodeRecord({ ...HEADER_2X2, headerLen: 64 }, RGBA_2X2);
    expect(decodeLuf1Record(record).header.headerLen).toBe(64);
    expect(Array.from(decodeLuf1Record(record).payload)).toEqual(Array.from(RGBA_2X2));
  });

  it("refuses a wrong magic", () => {
    const record = encodeRecord(HEADER_2X2, RGBA_2X2);
    record[0] = 0x58; // "XUF1"
    expectRefusal(record, "bad-magic");
  });

  it("refuses a version the decoder does not speak", () => {
    const record = encodeRecord(HEADER_2X2, RGBA_2X2);
    record[6] = 2;
    expectRefusal(record, "unsupported-version");
  });

  it("refuses header_len outside 48..=1024", () => {
    const record = encodeRecord(HEADER_2X2, RGBA_2X2);
    record[4] = 0x20; // header_len = 32
    expectRefusal(record, "bad-header-len");
  });

  it("refuses an unknown format code (the YUV family is not v1)", () => {
    const record = encodeRecord({ ...HEADER_2X2, formatCode: 3 }, RGBA_2X2);
    expectRefusal(record, "unknown-format-code");
  });

  it("refuses reserved flag bits and the reserved u16", () => {
    const flags = encodeRecord(HEADER_2X2, RGBA_2X2);
    flags[9] = 0x02; // bit 1 is reserved
    expectRefusal(flags, "reserved-not-zero");
    const reserved = encodeRecord(HEADER_2X2, RGBA_2X2);
    reserved[10] = 1;
    expectRefusal(reserved, "reserved-not-zero");
  });

  it("refuses impossible geometry (zero dims, cap violations, length mismatch)", () => {
    // Geometry violations are patched onto an otherwise-valid record
    // (the encoder refuses to EXPRESS them — the honesty rule the
    // decoder mirrors): width 0, width beyond the 65535 cap, and a
    // payload_len that disagrees with width·height·4.
    const zero = encodeRecord(HEADER_2X2, RGBA_2X2);
    zero[12] = 0; // width := 0 (zero dimension fires before the length checks)
    expectRefusal(zero, "bad-geometry");

    const wide = encodeRecord(HEADER_2X2, RGBA_2X2);
    const wideDv = new DataView(wide.buffer);
    wideDv.setUint32(12, 65_536, true); // width beyond the cap
    expectRefusal(wide, "bad-geometry");

    const mismatch = encodeRecord(HEADER_2X2, RGBA_2X2);
    mismatch[20] = 0x20; // payload_len := 32 (≠ 16)
    expectRefusal(mismatch, "payload-length-mismatch");
  });

  it("refuses truncated records (short header, short payload) and trailing bytes", () => {
    const record = encodeRecord(HEADER_2X2, RGBA_2X2);
    // Shorter than the fixed header.
    expectRefusal(record.subarray(0, 32), "truncated");
    // Header complete, payload missing its tail.
    expectRefusal(record.subarray(0, record.length - 4), "truncated");
    // A second record's bytes riding along violates the one-record
    // contract — refused, never silently half-read.
    const doubled = new Uint8Array(record.length * 2);
    doubled.set(record, 0);
    doubled.set(record, record.length);
    expectRefusal(doubled, "trailing-bytes");
  });

  it("refuses u64 fields beyond the JS safe-integer range (typed, not lossy)", () => {
    const record = encodeRecord(HEADER_2X2, RGBA_2X2);
    const dv = new DataView(record.buffer);
    dv.setBigUint64(40, BigInt(Number.MAX_SAFE_INTEGER) + 1n, true);
    expectRefusal(record, "unsafe-integer");
  });
});

describe("parseLuf1Header (the exported header parser)", () => {
  it("parses the golden header alone", () => {
    expect(parseLuf1Header(GOLDEN_2X2).frameId).toBe(7);
  });
});

// ---------------------------------------------------------------------------
// The shim.
// ---------------------------------------------------------------------------

/** The scripted fake internals (the tauri-transport.test.ts FakeBackend
 * posture: the seam the real webview injects, faked in tests). */
class FakeInternals implements FrameChannelBackend {
  readonly invokes: Array<{ command: string; payload?: unknown; options?: unknown }> = [];
  private nextId = 100;
  private nextFetchAnswer: { ok: unknown } | { reject: unknown } | null = null;
  unregisterCallback = vi.fn();

  transformCallback(callback: (response: unknown) => void): number {
    const id = this.nextId;
    this.nextId += 1;
    this.registered.set(id, callback);
    return id;
  }

  readonly registered = new Map<number, (response: unknown) => void>();

  invoke(command: string, payload?: unknown, options?: unknown): Promise<unknown> {
    this.invokes.push({ command, payload, options });
    const answer = this.nextFetchAnswer;
    this.nextFetchAnswer = null;
    if (!answer) {
      return Promise.reject(new Error("fake internals: no queued invoke answer"));
    }
    return "ok" in answer ? Promise.resolve(answer.ok) : Promise.reject(answer.reject);
  }

  queueFetch(answer: { ok: unknown } | { reject: unknown }): void {
    this.nextFetchAnswer = answer;
  }

  /** The shell delivers one channel message (either registration route). */
  deliver(id: number, frame: unknown): void {
    const callback = this.registered.get(id);
    if (!callback) {
      throw new Error(`fake internals: no callback registered for ${id}`);
    }
    callback(frame);
  }
}

function makeShim(
  options?: Partial<ConstructorParameters<typeof FrameChannelShim>[1]>,
): { shim: FrameChannelShim; internals: FakeInternals } {
  const internals = new FakeInternals();
  const shim = new FrameChannelShim(internals, {
    onRecord: options?.onRecord ?? (() => {}),
    onDiagnostic: options?.onDiagnostic,
  });
  return { shim, internals };
}

describe("FrameChannelShim (the hand-rolled tauri Channel)", () => {
  it("allocates the id through transformCallback and exposes window._<id>", () => {
    const { shim, internals } = makeShim();
    expect(shim.id).toBe(100);
    expect(internals.registered.has(100)).toBe(true);
    const prop = (globalThis as Record<string, unknown>)[`_${shim.id}`];
    expect(typeof prop).toBe("function");
    shim.close();
  });

  it("delivers in-order ArrayBuffer messages to onRecord", async () => {
    const records: Uint8Array[] = [];
    const { shim, internals } = makeShim({ onRecord: (record) => records.push(record) });
    const a = encodeRecord(HEADER_2X2, RGBA_2X2);
    internals.deliver(shim.id, { message: a.slice().buffer, index: 0 });
    internals.deliver(shim.id, { message: a.slice().buffer, index: 1 });
    await vi.waitFor(() => expect(records).toHaveLength(2));
    expect(Array.from(records[0]!)).toEqual(Array.from(a));
    expect(Array.from(records[1]!)).toEqual(Array.from(a));
    shim.close();
  });

  it("reorders out-of-order arrivals by index (the pendingMessages mechanism)", async () => {
    const order: number[] = [];
    const { shim, internals } = makeShim({
      onRecord: (record) => order.push(decodeLuf1Record(record).header.frameId),
    });
    const frame = (id: number, index: number): void => {
      const record = encodeRecord({ ...HEADER_2X2, frameId: id }, RGBA_2X2);
      internals.deliver(shim.id, { message: record.slice().buffer, index });
    };
    // Index 1 lands before 0 (the fetch path can resolve out of order)
    // — delivery order stays 10, 11, 12.
    frame(11, 1);
    frame(10, 0);
    frame(12, 2);
    await vi.waitFor(() => expect(order).toEqual([10, 11, 12]));
    shim.close();
  });

  it("drops stale duplicates with a diagnostic, never re-delivering", async () => {
    const records: Uint8Array[] = [];
    const diagnostics: string[] = [];
    const { shim, internals } = makeShim({
      onRecord: (record) => records.push(record),
      onDiagnostic: (message) => diagnostics.push(message),
    });
    internals.deliver(shim.id, { message: RGBA_2X2.slice().buffer, index: 0 });
    await vi.waitFor(() => expect(records).toHaveLength(1));
    internals.deliver(shim.id, { message: RGBA_2X2.slice().buffer, index: 0 });
    await vi.waitFor(() => expect(diagnostics.length).toBeGreaterThan(0));
    expect(records).toHaveLength(1);
    shim.close();
  });

  it("performs the fetch dance for token-string messages (the pinned invoke shape)", async () => {
    const records: Uint8Array[] = [];
    const { shim, internals } = makeShim({ onRecord: (record) => records.push(record) });
    const payload = encodeRecord(HEADER_2X2, RGBA_2X2);
    internals.queueFetch({ ok: payload.slice().buffer });
    // The token-string form: RAW payload above the eval threshold. The
    // shell's per-channel counter starts at 0, so the first message
    // carries index 0.
    internals.deliver(shim.id, { message: "17", index: 0 });

    await vi.waitFor(() => expect(records).toHaveLength(1));
    expect(Array.from(records[0]!)).toEqual(Array.from(payload));
    // The pinned shape: the ONE fetch command, the index payload, the
    // token in the protocol's Tauri-Channel-Id header.
    expect(internals.invokes).toHaveLength(1);
    expect(internals.invokes[0]?.command).toBe(FETCH_CHANNEL_DATA_COMMAND);
    expect(internals.invokes[0]?.payload).toEqual({ index: 0 });
    expect(internals.invokes[0]?.options).toEqual({
      headers: { "Tauri-Channel-Id": "17" },
    });
    shim.close();
  });

  it("skips a failed fetch with a diagnostic and keeps the channel alive", async () => {
    const diagnostics: string[] = [];
    const records: Uint8Array[] = [];
    const { shim, internals } = makeShim({
      onRecord: (record) => records.push(record),
      onDiagnostic: (message) => diagnostics.push(message),
    });
    internals.queueFetch({ reject: new Error("token expired") });
    internals.deliver(shim.id, { message: "41", index: 0 });
    await vi.waitFor(() => expect(diagnostics).toHaveLength(1));
    expect(diagnostics[0]).toContain("token expired");
    // The NEXT message still flows — a skip never stalls the channel.
    internals.queueFetch({ ok: RGBA_2X2.slice().buffer });
    internals.deliver(shim.id, { message: "42", index: 1 });
    await vi.waitFor(() => expect(records).toHaveLength(1));
    shim.close();
  });

  it("a throwing onRecord is a diagnostic, never a crash or a stall", async () => {
    const diagnostics: string[] = [];
    const seen: number[] = [];
    const { shim, internals } = makeShim({
      onRecord: (record) => {
        seen.push(decodeLuf1Record(record).header.frameId);
        if (seen.length === 1) {
          throw new Error("presenter exploded");
        }
      },
      onDiagnostic: (message) => diagnostics.push(message),
    });
    internals.deliver(shim.id, {
      message: encodeRecord({ ...HEADER_2X2, frameId: 1 }, RGBA_2X2).slice().buffer,
      index: 0,
    });
    internals.deliver(shim.id, {
      message: encodeRecord({ ...HEADER_2X2, frameId: 2 }, RGBA_2X2).slice().buffer,
      index: 1,
    });
    await vi.waitFor(() => expect(seen).toEqual([1, 2]));
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0]).toContain("presenter exploded");
    shim.close();
  });

  it("serves the 2.1.x frame shape ({ message, id }) through window._<id>", async () => {
    const records: Uint8Array[] = [];
    const { shim } = makeShim({ onRecord: (record) => records.push(record) });
    const evalTarget = (globalThis as Record<string, unknown>)[`_${shim.id}`] as (
      frame: unknown,
    ) => void;
    const record = encodeRecord(HEADER_2X2, RGBA_2X2);
    // The 2.1.1 shell evals window['_' + id]({ message, id }).
    evalTarget({ message: record.slice().buffer, id: 0 });
    await vi.waitFor(() => expect(records).toHaveLength(1));
    shim.close();
  });

  it("close() deregisters both routes and is idempotent", async () => {
    const { shim, internals } = makeShim();
    const prop = `_${shim.id}`;
    expect((globalThis as Record<string, unknown>)[prop]).toBeTypeOf("function");

    shim.close();
    expect((globalThis as Record<string, unknown>)[prop]).toBeUndefined();
    expect(internals.unregisterCallback).toHaveBeenCalledWith(shim.id);
    expect(internals.unregisterCallback).toHaveBeenCalledTimes(1);

    // Idempotent — a second close (React StrictMode double-unmount
    // shape) deregisters nothing twice.
    shim.close();
    expect(internals.unregisterCallback).toHaveBeenCalledTimes(1);

    // Late messages after close are dropped silently (no delivery, no
    // diagnostics — the consumer is gone).
    const records: Uint8Array[] = [];
    internals.deliver(shim.id, { message: RGBA_2X2.slice().buffer, index: 0 });
    await Promise.resolve();
    await Promise.resolve();
    expect(records).toHaveLength(0);
  });

  it("the end marker releases the registrations", async () => {
    const diagnostics: string[] = [];
    const { shim, internals } = makeShim({
      onDiagnostic: (message) => diagnostics.push(message),
    });
    internals.deliver(shim.id, { end: true, index: 0 });
    await vi.waitFor(() => expect(diagnostics).toHaveLength(1));
    expect((globalThis as Record<string, unknown>)[`_${shim.id}`]).toBeUndefined();
    expect(internals.unregisterCallback).toHaveBeenCalledWith(shim.id);
  });
});

describe("detectFrameChannelBackend (the shell probe)", () => {
  it("returns the internals inside the shell, null outside", () => {
    const globals = globalThis as { __TAURI_INTERNALS__?: unknown };
    const saved = globals.__TAURI_INTERNALS__;
    const internals: FrameChannelBackend = {
      invoke: () => Promise.resolve(null),
      transformCallback: () => 1,
    };
    globals.__TAURI_INTERNALS__ = internals;
    try {
      expect(detectFrameChannelBackend()).toBe(internals);
      globals.__TAURI_INTERNALS__ = { invoke: () => Promise.resolve(null) };
      expect(detectFrameChannelBackend()).toBeNull(); // no transformCallback
      delete globals.__TAURI_INTERNALS__;
      expect(detectFrameChannelBackend()).toBeNull();
    } finally {
      if (saved === undefined) {
        delete globals.__TAURI_INTERNALS__;
      } else {
        globals.__TAURI_INTERNALS__ = saved;
      }
    }
  });
});
