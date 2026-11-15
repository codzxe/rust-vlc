// w31.4 (lane `ui-canvas`) — the webview half of w31.1's frame channel:
// the hand-rolled Tauri channel shim + the LUF1 record decoder.
//
// WHAT THIS MODULE IS
//
// The shell (w31.1) taps the engine's presented frames and ships ONE
// COMPLETE LUF1 record per pacer tick as a RAW BINARY channel message
// over the tauri IPC (apps/desktop-tauri/src/frame_channel.rs sends
// `InvokeResponseBody::Raw`; the format spec is apps/desktop/src/wire.rs
// — this decoder is its byte-mirror, sibling of the reference
// `apps/desktop/prototype/lumen-frames.mjs`).
//
// THE SHIM (why hand-rolled): the npm `@tauri-apps/api` package is not
// in the dependency allowlist (records/dependencies/records/
// tauri-wry-2.x.md — "exception-ADR-004, NOT imported"), so — exactly
// like `tauri-transport.ts` — this module binds to the always-injected
// `window.__TAURI_INTERNALS__` surface directly: same wire, no new
// dependency. The ported behavior is the @tauri-apps/api `Channel`
// class (studied at tauri-v2.1.1 `packages/api/src/core.ts`, the
// mechanism's ~60 lines): id allocation via `transformCallback`, the
// `window._<id>` eval target, and index-based message ordering. On top
// of the port sits the raw-payload fetch dance: a message that arrives
// as a fetch-token string is fetched through
// `plugin:__TAURI_CHANNEL__|fetch` into an ArrayBuffer (see
// `FrameChannelShim` — both tauri generations' shapes are served, and
// neither can crash the app: every skip is a typed diagnostic).
//
// HONEST LIMITS: verified against tauri 2.1.1 and 2.12.5 (the shipped
// Cargo.lock version) channel sources — in 2.12.x the shell's own
// eval'd script performs the fetch before `runCallback` hands the
// callback an ArrayBuffer, so the token-string branch is the DEFENSIVE
// path (a shell that hands the token to the callback instead); the
// ArrayBuffer branch is the one the real shell exercises. No webview
// exists in this sandbox (apps/desktop-tauri/INFRA-BLOCKED.md), so the
// runtime shapes are pinned by unit tests against mocked internals,
// same posture as tauri-transport.ts.

/** The v1 protocol constants (byte-mirror of wire.rs's pub consts). */
export const LUF1 = {
  /** Stream magic: "LUF1" (Lumen Frame, version 1 in the magic itself). */
  MAGIC: [0x4c, 0x55, 0x46, 0x31],
  VERSION: 1,
  MIN_HEADER_LEN: 48,
  MAX_HEADER_LEN: 1024,
  /** Payload cap: 1 GiB (bigger means corruption, not a frame). */
  MAX_PAYLOAD_LEN: 1 << 30,
  /** Largest accepted width/height — no real display exceeds it. */
  MAX_DIMENSION: 65_535,
  /** Pixel-format codes (the v1 table is RGBA/BGRA only). */
  FORMAT_RGBA8: 1,
  FORMAT_BGRA8: 2,
} as const;

// The u64 fields cross into JS Numbers through a safe-integer guard
// (2^53 - 1 ceiling): frame ids and timestamps beyond it are a typed
// error, not silent precision loss (u64 ns ≈ 285 years; the guard
// documents where the f64 world ends).

/** Typed protocol error (mirror of desktop::WireError / the reference
 * decoder's kind strings — a decode failure is NEVER a guess). */
export class FrameWireError extends Error {
  readonly kind: string;

  constructor(kind: string, message: string) {
    super(`frame stream: ${message}`);
    this.name = "FrameWireError";
    this.kind = kind;
  }
}

/** The decoded fixed v1 header (wire.rs's layout table, little-endian). */
export interface Luf1Header {
  readonly headerLen: number;
  readonly formatCode: number;
  readonly formatName: "rgba8" | "bgra8";
  /** Bit 0 of the flags byte: the record re-presents the previous slot
   * (the scheduler's bounded repeat — presenters must honor it). */
  readonly repeat: boolean;
  readonly width: number;
  readonly height: number;
  readonly payloadLen: number;
  readonly frameId: number;
  readonly ptsNs: number;
  readonly emittedNs: number;
}

/** One complete decoded record: the validated header + its payload. */
export interface DecodedLuf1Frame {
  readonly header: Luf1Header;
  /** `width * height * 4` bytes, row-major, in the header's format. */
  readonly payload: Uint8Array;
}

function toSafeNumber(value: bigint, field: string): number {
  const n = Number(value);
  if (!Number.isSafeInteger(n)) {
    throw new FrameWireError(
      "unsafe-integer",
      `${field} ${value.toString()} exceeds the JS safe-integer range`,
    );
  }
  return n;
}

/**
 * Parse + validate the fixed 48-byte header (throws on every malformed
 * field, exactly like the Rust parser and the reference JS decoder —
 * unknown magic/version/format, reserved bits, and impossible geometry
 * are typed refusals, never guesses). `view` must span at least
 * `LUF1.MIN_HEADER_LEN` bytes.
 */
export function parseLuf1Header(view: Uint8Array): Luf1Header {
  for (let i = 0; i < 4; i++) {
    if (view[i] !== LUF1.MAGIC[i]) {
      throw new FrameWireError(
        "bad-magic",
        `magic 0x${Array.from(view.slice(0, 4), (b) => b.toString(16).padStart(2, "0")).join("")} is not LUF1`,
      );
    }
  }
  const dv = new DataView(view.buffer, view.byteOffset, view.byteLength);
  const headerLen = dv.getUint16(4, true);
  if (headerLen < LUF1.MIN_HEADER_LEN || headerLen > LUF1.MAX_HEADER_LEN) {
    throw new FrameWireError("bad-header-len", `header_len ${headerLen} outside 48..=1024`);
  }
  const version = dv.getUint16(6, true);
  if (version !== LUF1.VERSION) {
    throw new FrameWireError("unsupported-version", `version ${version} (decoder speaks 1)`);
  }
  const formatCode = view[8] ?? -1;
  if (formatCode !== LUF1.FORMAT_RGBA8 && formatCode !== LUF1.FORMAT_BGRA8) {
    throw new FrameWireError("unknown-format-code", `format code ${formatCode}`);
  }
  const flags = view[9] ?? 0;
  if ((flags & 0xfe) !== 0) {
    throw new FrameWireError("reserved-not-zero", `flags 0x${flags.toString(16)} uses reserved bits`);
  }
  if (dv.getUint16(10, true) !== 0) {
    throw new FrameWireError("reserved-not-zero", "reserved u16 is not zero");
  }
  const width = dv.getUint32(12, true);
  const height = dv.getUint32(16, true);
  const payloadLen = dv.getUint32(20, true);
  if (width === 0 || height === 0) {
    throw new FrameWireError("bad-geometry", "zero dimension");
  }
  if (width > LUF1.MAX_DIMENSION || height > LUF1.MAX_DIMENSION) {
    throw new FrameWireError("bad-geometry", "dimension exceeds 65535");
  }
  const implied = width * height * 4;
  if (implied > LUF1.MAX_PAYLOAD_LEN) {
    throw new FrameWireError("bad-geometry", "payload exceeds the 1 GiB cap");
  }
  if (payloadLen !== implied) {
    throw new FrameWireError(
      "payload-length-mismatch",
      `payload_len ${payloadLen} != w*h*4 (${implied})`,
    );
  }
  return {
    headerLen,
    formatCode,
    formatName: formatCode === LUF1.FORMAT_RGBA8 ? "rgba8" : "bgra8",
    repeat: (flags & 0x01) !== 0,
    width,
    height,
    payloadLen,
    frameId: toSafeNumber(dv.getBigUint64(24, true), "frame_id"),
    ptsNs: toSafeNumber(dv.getBigInt64(32, true), "pts_ns"),
    emittedNs: toSafeNumber(dv.getBigUint64(40, true), "emitted_ns"),
  };
}

/**
 * Decode ONE COMPLETE record (the channel contract: the shell ships
 * exactly one record per message, so the UI needs the record-at-a-time
 * form of the decoder, not the stream form). A short record is the
 * typed `truncated` refusal; trailing bytes beyond `header_len +
 * payload_len` are a contract violation (two records in one message)
 * and are refused too — strictness is the wire.rs ethos, and the
 * shell-side encoder guarantees the exact form.
 *
 * @throws {FrameWireError} on any malformed field (never returns a guess).
 */
export function decodeLuf1Record(record: Uint8Array): DecodedLuf1Frame {
  if (record.length < LUF1.MIN_HEADER_LEN) {
    throw new FrameWireError(
      "truncated",
      `${LUF1.MIN_HEADER_LEN - record.length} header bytes missing; feed more input`,
    );
  }
  const header = parseLuf1Header(record.subarray(0, LUF1.MIN_HEADER_LEN));
  const total = header.headerLen + header.payloadLen;
  if (record.length < total) {
    throw new FrameWireError("truncated", `${total - record.length} bytes missing; feed more input`);
  }
  if (record.length > total) {
    throw new FrameWireError(
      "trailing-bytes",
      `${record.length - total} bytes trail the record (one record per message is the channel contract)`,
    );
  }
  return { header, payload: record.subarray(header.headerLen, total) };
}

// ---------------------------------------------------------------------------
// The channel shim — the hand-rolled @tauri-apps/api Channel port.
// ---------------------------------------------------------------------------

/** The tauri command that fetches a raw channel payload by token
 * (tauri's ipc/channel.rs `FETCH_CHANNEL_DATA_COMMAND`). */
export const FETCH_CHANNEL_DATA_COMMAND = "plugin:__TAURI_CHANNEL__|fetch";

/** The header tauri's fetch command addresses the queued payload by
 * (`CHANNEL_ID_HEADER_NAME` in ipc/channel.rs). */
const CHANNEL_ID_HEADER_NAME = "Tauri-Channel-Id";

/**
 * The always-injected internals surface (mirrors tauri-transport.ts's
 * `TauriInternals`; the third `invoke` argument carries the fetch
 * headers the channel protocol needs).
 */
export interface FrameChannelBackend {
  invoke(command: string, payload?: unknown, options?: unknown): Promise<unknown>;
  transformCallback(callback: (response: unknown) => void, once?: boolean): number;
  /** The 2.12.x cleanup seam; absent on 2.1.x (there the window prop IS
   * the registry — deleting it is the deregistration). */
  unregisterCallback?(id: number): void;
}

function isFrameChannelBackend(value: unknown): value is FrameChannelBackend {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const candidate = value as Partial<FrameChannelBackend>;
  return typeof candidate.invoke === "function" && typeof candidate.transformCallback === "function";
}

/**
 * Probe the webview host for the channel seam (the same presence probe
 * `detectTauriBackend`/`isDesktopShell` use — the consumer and the
 * transport decision can never drift). Null outside the desktop shell.
 */
export function detectFrameChannelBackend(): FrameChannelBackend | null {
  const internals = (
    globalThis as { __TAURI_INTERNALS__?: unknown }
  ).__TAURI_INTERNALS__;
  return isFrameChannelBackend(internals) ? internals : null;
}

/** What the registered callback receives per message (the tauri channel
 * protocol frame): `{ message, index }` on 2.12.x, `{ message, id }` on
 * 2.1.x, plus the terminal `{ end: true, index }` marker. */
interface ChannelFrame {
  message?: unknown;
  index?: unknown;
  id?: unknown;
  end?: unknown;
}

export interface FrameChannelShimOptions {
  /** Receives each resolved record (bytes) in channel order. */
  onRecord: (record: Uint8Array) => void;
  /** Diagnostic sink for skipped messages (defaults to console.warn —
   * a skip is never silent, and never a crash). */
  onDiagnostic?: (message: string) => void;
}

/**
 * The hand-rolled tauri channel (the webview's receive side of
 * w31.1's `session.set-frame-channel`):
 *
 * 1. **Id allocation + registration** — the constructor registers one
 *    callback through `internals.transformCallback` (what
 *    @tauri-apps/api's Channel does; on tauri 2.12.x the shell's
 *    `runCallback(<id>, …)` eval reaches it) AND exposes the SAME
 *    handler as `window._<id>` (the 2.1.x eval target
 *    `window['_' + id]({ message, id })`; also the registration shape
 *    w31.1's contract pins). Both routes feed one ordering queue.
 * 2. **Ordering** — messages carry a monotonic index; out-of-order
 *    arrivals (the raw-fetch path can resolve out of order) park in a
 *    pending map until their turn, exactly the api Channel's
 *    pendingMessages mechanism. Stale duplicates are dropped.
 * 3. **The fetch dance** — a record arrives either as an ArrayBuffer
 *    (the direct form: small payloads eval'd as a byte array, or a
 *    shell that already performed the fetch) or as a fetch-token
 *    string, which the shim fetches via
 *    `invoke('plugin:__TAURI_CHANNEL__|fetch', { index }, { headers:
 *    { 'Tauri-Channel-Id': token } })` into an ArrayBuffer.
 * 4. **Lifecycle** — `close()` deregisters both registrations (delete
 *    `window._<id>`; `unregisterCallback(id)` where the seam exists).
 *    No leak.
 *
 * The handler is throw-safe: a failing `onRecord`/fetch is a
 * diagnostic, never a crash and never a stall (the index counter keeps
 * advancing — the same contract the 2.12.x api class documents).
 */
export class FrameChannelShim {
  readonly id: number;

  private readonly backend: FrameChannelBackend;
  private readonly onRecord: (record: Uint8Array) => void;
  private readonly onDiagnostic: (message: string) => void;
  private readonly propName: string;
  private closed = false;
  private nextIndex = 0;
  private readonly pending = new Map<number, unknown>();
  private draining = false;

  constructor(backend: FrameChannelBackend, options: FrameChannelShimOptions) {
    this.backend = backend;
    this.onRecord = options.onRecord;
    this.onDiagnostic = options.onDiagnostic ?? ((message) => console.warn(message));
    // The id the shell rehydrates (`__CHANNEL__:<id>` — frame_channel.rs).
    this.id = backend.transformCallback((response) => {
      this.handleCallback(response);
    });
    this.propName = `_${this.id}`;
    // The 2.1.x eval target. In a 2.1.x shell `transformCallback` has
    // already defined this prop (configurable: true) — redefining it to
    // the same behavior is the hand-rolled equivalent; in a 2.12.x
    // shell nothing evals it, and defining it is inert.
    Object.defineProperty(globalThis, this.propName, {
      value: (frame: ChannelFrame): void => {
        this.handleCallback(frame);
      },
      writable: false,
      configurable: true,
    });
  }

  /** Deregister both registrations (idempotent). */
  close(): void {
    if (this.closed) {
      return;
    }
    this.closed = true;
    this.pending.clear();
    try {
      delete (globalThis as Record<string, unknown>)[this.propName];
    } catch {
      // A frozen/stubbed host — the prop dies with the page anyway.
    }
    this.backend.unregisterCallback?.(this.id);
  }

  /** One tauri channel message (either registration route). */
  private handleCallback(response: unknown): void {
    if (this.closed) {
      return;
    }
    if (typeof response !== "object" || response === null) {
      this.onDiagnostic(`frame channel: callback payload is ${typeof response}, not a frame`);
      return;
    }
    const frame = response as ChannelFrame;
    // 2.12.x stamps `index`; 2.1.x stamped `id` — serve both (the
    // shipped shell is 2.12.x; the older key keeps the port honest).
    const index =
      typeof frame.index === "number" ? frame.index : typeof frame.id === "number" ? frame.id : null;
    if (index === null) {
      this.onDiagnostic("frame channel: message carries no index — skipped");
      return;
    }
    if (frame.end === true) {
      // The terminal marker (tauri's channel cleanup callback): the
      // shell will send nothing after it — release the registrations.
      this.onDiagnostic(`frame channel: end marker at index ${index} — closing`);
      this.close();
      return;
    }
    if (index < this.nextIndex) {
      this.onDiagnostic(`frame channel: stale message index ${index} (already at ${this.nextIndex})`);
      return;
    }
    this.pending.set(index, frame.message);
    void this.drain();
  }

  /**
   * Deliver parked messages in index order. The loop is guarded so
   * concurrent callback storms coalesce into one pass; the only
   * suspension point is the (rare) token fetch, after which the loop
   * re-checks the pending map — messages that arrived during the fetch
   * are picked up by the same pass.
   */
  private async drain(): Promise<void> {
    if (this.draining) {
      return;
    }
    this.draining = true;
    try {
      while (!this.closed) {
        const message = this.pending.get(this.nextIndex);
        if (message === undefined) {
          break;
        }
        const index = this.nextIndex;
        this.pending.delete(index);
        this.nextIndex += 1;
        try {
          const record = await this.resolveBytes(message, index);
          this.onRecord(record);
        } catch (error) {
          // A failed fetch/decode for ONE message must neither crash
          // the app nor stall the channel — skip it, say so, continue.
          this.onDiagnostic(
            `frame channel: message ${index} skipped: ${error instanceof Error ? error.message : String(error)}`,
          );
        }
      }
    } finally {
      this.draining = false;
    }
  }

  /** Resolve one message to record bytes (the fetch dance lives here). */
  private async resolveBytes(message: unknown, index: number): Promise<Uint8Array> {
    if (message instanceof ArrayBuffer) {
      return new Uint8Array(message);
    }
    if (ArrayBuffer.isView(message)) {
      // A shell that evals `new Uint8Array([...])` directly hands the
      // view, not the buffer — accept both byte carriers.
      return new Uint8Array(message.buffer, message.byteOffset, message.byteLength);
    }
    if (typeof message === "string") {
      // The fetch-token form (w31.1's contract for RAW payloads above
      // the eval threshold): fetch the bytes through the channel
      // plugin. The token rides the protocol's Tauri-Channel-Id header;
      // the message index rides the payload (the pinned contract
      // shape — a future shell build drifts loudly, not silently).
      const fetched = await this.backend.invoke(
        FETCH_CHANNEL_DATA_COMMAND,
        { index },
        { headers: { [CHANNEL_ID_HEADER_NAME]: message } },
      );
      if (fetched instanceof ArrayBuffer) {
        return new Uint8Array(fetched);
      }
      if (ArrayBuffer.isView(fetched)) {
        return new Uint8Array(fetched.buffer, fetched.byteOffset, fetched.byteLength);
      }
      throw new Error(`the channel fetch answered ${typeof fetched}, not an ArrayBuffer`);
    }
    throw new Error(`channel message is ${typeof message}, not binary`);
  }
}
