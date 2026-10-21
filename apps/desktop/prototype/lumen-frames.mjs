// lumen-frames.mjs — the canvas-side reference implementation of the
// Lumen frame-streaming wire protocol v1 (card 2.D3, ADR-003
// variant B).
//
// This module mirrors apps/desktop/src/wire.rs BYTE FOR BYTE (the
// conformance proof is prototype/harness.mjs over the Rust-emitted
// golden stream). It is deliberately dependency-free and
// environment-agnostic: the parser runs in Node and every browser;
// the canvas presenter feature-detects its backend.
//
// PROTOCOL v1 (all integers little-endian) — see wire.rs for the
// authoritative table:
//   off  len  field
//    0    4   magic "LUF1"
//    4    2   header_len (48..=1024; 48..header_len is v1-ignored
//                     extension space)
//    6    2   version (1)
//    8    1   format code (1 = Rgba8, 2 = Bgra8)
//    9    1   flags (bit 0 = repeat; bits 1..=7 must be zero)
//   10    2   reserved (must be zero)
//   12    4   width  (1..=65535)
//   16    4   height (1..=65535)
//   20    4   payload_len (== width*height*4)
//   24    8   frame_id (monotonic per stream)
//   32    8   pts_ns (scheduled slot, media-clock ns base)
//   40    8   emitted_ns (serialize instant on the bridge's emit
//                     clock — the transport-latency origin)
//   48   ...   payload (width*height*4 bytes, row-major RGBA)
//
// Status honestly labeled: the 2D and WebGL2 paths are implemented
// but UNVERIFIED IN A BROWSER in the build sandbox (no browser
// exists there — docs/prototypes/video-in-webview.md §5); the
// WebGPU rung of doc 08 §7.4's chain is a typed stub in this
// revision (writing untestable upload code would be worse than an
// honest stub).

export const PROTOCOL = {
  MAGIC: [0x4c, 0x55, 0x46, 0x31], // "LUF1"
  VERSION: 1,
  MIN_HEADER_LEN: 48,
  MAX_HEADER_LEN: 1024,
  MAX_PAYLOAD_LEN: 1 << 30,
  MAX_DIMENSION: 65535,
  FORMAT_RGBA8: 1,
  FORMAT_BGRA8: 2,
};

// 2^53 - 1: the JS Number safe-integer ceiling. Frame ids and
// timestamps beyond it are a typed error, not silent precision loss
// (u64 ns ≈ 285 years; the guard documents where the f64 world
// ends).
const MAX_SAFE = Number.MAX_SAFE_INTEGER;

/** Typed protocol error (mirror of desktop::WireError). */
export class WireError extends Error {
  /** @param {string} kind @param {string} message */
  constructor(kind, message) {
    super(`frame stream: ${message}`);
    this.name = "WireError";
    this.kind = kind;
  }
}

function toSafeNumber(bigint, field) {
  const n = Number(bigint);
  if (!Number.isSafeInteger(n)) {
    throw new WireError(
      "unsafe-integer",
      `${field} ${bigint.toString()} exceeds the JS safe-integer range`,
    );
  }
  return n;
}

/** Growable byte queue with a consume cursor (amortized O(1) appends). */
class ByteQueue {
  constructor() {
    this.buf = new Uint8Array(1 << 16);
    this.start = 0;
    this.end = 0;
  }
  get length() {
    return this.end - this.start;
  }
  /** @param {Uint8Array} chunk */
  feed(chunk) {
    const need = this.end + chunk.length;
    if (need > this.buf.length) {
      const live = this.length;
      let size = this.buf.length;
      while (size < need) size *= 2;
      const next = new Uint8Array(size);
      next.set(this.buf.subarray(this.start, this.end), 0);
      this.buf = next;
      this.end = live;
      this.start = 0;
    }
    this.buf.set(chunk, this.end);
    this.end += chunk.length;
  }
  /** A view of the next `n` bytes (valid until the next feed/skip). */
  view(n) {
    return this.buf.subarray(this.start, this.start + n);
  }
  /** @param {number} n */
  skip(n) {
    this.start += n;
    if (this.start === this.end) {
      this.start = 0;
      this.end = 0;
    } else if (this.start > this.buf.length / 2) {
      this.buf.copyWithin(0, this.start, this.end);
      this.end -= this.start;
      this.start = 0;
    }
  }
}

/**
 * The incremental record decoder (mirror of desktop::wire::
 * WireDecoder): feed arbitrary chunks, pull complete frames.
 * Malformed records throw WireError — sticky, no resync (the host
 * drops the stream and re-establishes; the engine-side twin is the
 * generation purge).
 */
export class FrameDecoder {
  constructor() {
    this.pending = new ByteQueue();
    this.fed = 0;
    this.consumed = 0;
  }
  /** @param {Uint8Array} chunk */
  feed(chunk) {
    this.pending.feed(chunk);
    this.fed += chunk.length;
  }
  /**
   * @returns {{header: object, payload: Uint8Array} | null} null =
   * need more input.
   */
  nextFrame() {
    if (this.pending.length < PROTOCOL.MIN_HEADER_LEN) return null;
    const header = parseFrameHeader(this.pending.view(PROTOCOL.MIN_HEADER_LEN));
    const total = header.headerLen + header.payloadLen;
    if (this.pending.length < total) return null;
    const payload = this.pending.view(total).slice(header.headerLen, total);
    this.pending.skip(total);
    this.consumed += total;
    return { header, payload };
  }
  /** Throws WireError("truncated") if a partial record remains. */
  finish() {
    if (this.pending.length === 0) return;
    const remaining = this.pending.length;
    const missing =
      remaining < PROTOCOL.MIN_HEADER_LEN
        ? PROTOCOL.MIN_HEADER_LEN - remaining
        : (() => {
            const header = parseFrameHeader(this.pending.view(PROTOCOL.MIN_HEADER_LEN));
            return header.headerLen + header.payloadLen - remaining;
          })();
    throw new WireError("truncated", `${missing} bytes missing; feed more input`);
  }
}

/**
 * Parse + validate the fixed header (throws on every malformed
 * field, exactly like the Rust parser).
 * @param {Uint8Array} view
 */
function parseFrameHeader(view) {
  for (let i = 0; i < 4; i++) {
    if (view[i] !== PROTOCOL.MAGIC[i]) {
      throw new WireError(
        "bad-magic",
        `magic 0x${Array.from(view.slice(0, 4), (b) => b.toString(16).padStart(2, "0")).join("")} is not LUF1`,
      );
    }
  }
  const dv = new DataView(view.buffer, view.byteOffset, view.byteLength);
  const headerLen = dv.getUint16(4, true);
  if (headerLen < PROTOCOL.MIN_HEADER_LEN || headerLen > PROTOCOL.MAX_HEADER_LEN) {
    throw new WireError("bad-header-len", `header_len ${headerLen} outside 48..=1024`);
  }
  const version = dv.getUint16(6, true);
  if (version !== PROTOCOL.VERSION) {
    throw new WireError("unsupported-version", `version ${version} (decoder speaks 1)`);
  }
  const formatCode = view[8];
  if (formatCode !== PROTOCOL.FORMAT_RGBA8 && formatCode !== PROTOCOL.FORMAT_BGRA8) {
    throw new WireError("unknown-format-code", `format code ${formatCode}`);
  }
  const flags = view[9];
  if ((flags & 0xfe) !== 0) {
    throw new WireError("reserved-not-zero", `flags 0x${flags.toString(16)} uses reserved bits`);
  }
  if (dv.getUint16(10, true) !== 0) {
    throw new WireError("reserved-not-zero", "reserved u16 is not zero");
  }
  const width = dv.getUint32(12, true);
  const height = dv.getUint32(16, true);
  const payloadLen = dv.getUint32(20, true);
  if (width === 0 || height === 0) {
    throw new WireError("bad-geometry", "zero dimension");
  }
  if (width > PROTOCOL.MAX_DIMENSION || height > PROTOCOL.MAX_DIMENSION) {
    throw new WireError("bad-geometry", "dimension exceeds 65535");
  }
  const implied = width * height * 4;
  if (implied > PROTOCOL.MAX_PAYLOAD_LEN) {
    throw new WireError("bad-geometry", "payload exceeds the 1 GiB cap");
  }
  if (payloadLen !== implied) {
    throw new WireError("payload-length-mismatch", `payload_len ${payloadLen} != w*h*4 (${implied})`);
  }
  return {
    headerLen,
    formatCode,
    formatName: formatCode === PROTOCOL.FORMAT_RGBA8 ? "rgba8" : "bgra8",
    repeat: (flags & 0x01) !== 0,
    width,
    height,
    payloadLen,
    frameId: toSafeNumber(dv.getBigUint64(24, true), "frame_id"),
    ptsNs: toSafeNumber(dv.getBigInt64(32, true), "pts_ns"),
    emittedNs: toSafeNumber(dv.getBigUint64(40, true), "emitted_ns"),
  };
}

// The header parser is exported for harnesses and hosts that want
// to inspect a record without committing to its payload.
export { parseFrameHeader as parseHeader };

/**
 * Encode one record (the JS mirror of wire.rs's encoder — used by
 * the in-page synthetic generator and available to future hosts).
 * @param {{formatCode: number, repeat: boolean, width: number,
 *          height: number, frameId: number, ptsNs: number,
 *          emittedNs: number}} header
 * @param {Uint8Array} payload
 */
export function encodeFrame(header, payload) {
  const implied = header.width * header.height * 4;
  if (payload.length !== implied) {
    throw new WireError("payload-length-mismatch", `payload ${payload.length} != w*h*4 (${implied})`);
  }
  const out = new Uint8Array(PROTOCOL.MIN_HEADER_LEN + implied);
  out.set(PROTOCOL.MAGIC, 0);
  const dv = new DataView(out.buffer);
  dv.setUint16(4, PROTOCOL.MIN_HEADER_LEN, true);
  dv.setUint16(6, PROTOCOL.VERSION, true);
  out[8] = header.formatCode;
  out[9] = header.repeat ? 1 : 0;
  dv.setUint16(10, 0, true);
  dv.setUint32(12, header.width, true);
  dv.setUint32(16, header.height, true);
  dv.setUint32(20, implied, true);
  dv.setBigUint64(24, BigInt(header.frameId), true);
  dv.setBigInt64(32, BigInt(header.ptsNs), true);
  dv.setBigUint64(40, BigInt(header.emittedNs), true);
  out.set(payload, PROTOCOL.MIN_HEADER_LEN);
  return out;
}

/**
 * The latency ledger (mirror of desktop::mailbox::LatencyAccountant):
 * exact min/max/floor-mean over recorded samples.
 */
export class LatencyLedger {
  constructor() {
    this.samples = 0;
    this.min = null;
    this.max = null;
    this.sum = 0;
  }
  /** @param {number} latencyNs */
  record(latencyNs) {
    this.samples += 1;
    this.min = this.min === null ? latencyNs : Math.min(this.min, latencyNs);
    this.max = this.max === null ? latencyNs : Math.max(this.max, latencyNs);
    this.sum += latencyNs;
  }
  summary() {
    return {
      samples: this.samples,
      minNs: this.min,
      maxNs: this.max,
      meanNs: this.samples === 0 ? null : Math.floor(this.sum / this.samples),
    };
  }
}

/**
 * The canvas presenter: doc 08 §7.4's fallback chain B(WebGPU) →
 * B(WebGL2) → B(2D ImageData). Backend detection is cached and
 * reported truthfully (ADR-003 decision 4).
 */
export class CanvasPresenter {
  /** @param {HTMLCanvasElement} canvas */
  constructor(canvas) {
    this.canvas = canvas;
    this.backend = null; // 'webgl2' | '2d'
    this.detectedGpu = false; // navigator.gpu present?
    this.gl = null;
    this.ctx2d = null;
    this.offscreen = null;
    this.drawn = 0;
    this.repeatsSkipped = 0;
  }

  /** Detect + cache the backend (the fallback chain, honestly). */
  detect() {
    if (this.backend !== null) return this.backend;
    if (typeof navigator !== "undefined" && navigator.gpu) {
      this.detectedGpu = true; // present but a typed stub this revision
    }
    this.gl = this.canvas.getContext("webgl2");
    if (this.gl) {
      this.backend = "webgl2";
      this.initWebgl2();
    } else {
      this.ctx2d = this.canvas.getContext("2d");
      this.backend = this.ctx2d ? "2d" : "none";
    }
    return this.backend;
  }

  /** The truthful one-line capability report. */
  status() {
    const backend = this.detect();
    const gpu = this.detectedGpu ? "WebGPU detected (stub in this revision — falls through)" : "no WebGPU";
    return `backend=${backend} (${gpu}); frames drawn=${this.drawn}`;
  }

  initWebgl2() {
    const gl = this.gl;
    const vs = gl.createShader(gl.VERTEX_SHADER);
    gl.shaderSource(vs, "attribute vec2 p; varying vec2 t; void main(){ t = p * 0.5 + 0.5; gl_Position = vec4(p, 0.0, 1.0); }");
    gl.compileShader(vs);
    const fs = gl.createShader(gl.FRAGMENT_SHADER);
    gl.shaderSource(fs, "precision mediump float; varying vec2 t; uniform sampler2D tex; void main(){ gl_FragColor = texture2D(tex, t); }");
    gl.compileShader(fs);
    const prog = gl.createProgram();
    gl.attachShader(prog, vs);
    gl.attachShader(prog, fs);
    gl.linkProgram(prog);
    gl.useProgram(prog);
    const buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
    const loc = gl.getAttribLocation(prog, "p");
    gl.enableVertexAttribArray(loc);
    gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
    this.texture = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, this.texture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  }

  /**
   * Draw one decoded frame (scaling to the canvas; aspect-fit is
   * the caller's CSS job — `object-fit: contain` on the canvas).
   * @param {{header: object, payload: Uint8Array}} frame
   */
  draw(frame) {
    const backend = this.detect();
    if (backend === "none") {
      throw new WireError("no-backend", "no canvas backend available (2d, webgl2 all refused)");
    }
    if (frame.header.formatCode === PROTOCOL.FORMAT_BGRA8 && backend === "2d") {
      // The 2D path takes RGBA; BGRA frames swizzle draw-side
      // (recorded cost; WebGL2 uploads BGRA natively where the
      // extension exists [VERIFY on desktop]).
      swizzleBgraToRgba(frame.payload);
    }
    if (backend === "webgl2") {
      const gl = this.gl;
      gl.bindTexture(gl.TEXTURE_2D, this.texture);
      gl.texImage2D(
        gl.TEXTURE_2D,
        0,
        gl.RGBA,
        frame.header.width,
        frame.header.height,
        0,
        gl.RGBA,
        gl.UNSIGNED_BYTE,
        frame.payload,
      );
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    } else {
      const w = frame.header.width;
      const h = frame.header.height;
      if (this.offscreen === null || this.offscreen.width !== w || this.offscreen.height !== h) {
        this.offscreen = document.createElement("canvas");
        this.offscreen.width = w;
        this.offscreen.height = h;
      }
      const octx = this.offscreen.getContext("2d");
      octx.putImageData(new ImageData(new Uint8ClampedArray(frame.payload.buffer, frame.payload.byteOffset, frame.payload.length), w, h), 0, 0);
      this.ctx2d.drawImage(this.offscreen, 0, 0, this.canvas.width, this.canvas.height);
    }
    this.drawn += 1;
  }
}

/** In-place BGRA→RGBA swizzle (the 2D fallback's recorded cost). */
function swizzleBgraToRgba(payload) {
  for (let i = 0; i + 3 < payload.length; i += 4) {
    const b = payload[i];
    payload[i] = payload[i + 2];
    payload[i + 2] = b;
  }
}

/**
 * The synthetic test pattern (the JS mirror of renderer-wgpu's
 * `synthetic.rs` spec: two-axis gradient, 4x4 red corner blocks,
 * white every-16th grid line, and the (5,5) frame-index marker) —
 * so the standalone page can generate its OWN stream with zero
 * host wiring.
 */
export function syntheticPattern(width, height, markerIndex) {
  const out = new Uint8Array(width * height * 4);
  const grad = (i, n) => (n <= 1 ? 128 : Math.round((i * 255 + (n - 1) / 2) / (n - 1)));
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const at = (y * width + x) * 4;
      let pixel;
      if ((x < 4 && y < 4) || (x + 4 >= width && y + 4 >= height)) {
        pixel = [255, 0, 0, 255];
      } else if (x % 16 === 0 || y % 16 === 0) {
        pixel = [255, 255, 255, 255];
      } else {
        pixel = [grad(x, width), grad(y, height), (x + y) % 256, 255];
      }
      out[at] = pixel[0];
      out[at + 1] = pixel[1];
      out[at + 2] = pixel[2];
      out[at + 3] = pixel[3];
    }
  }
  const at = (5 * width + 5) * 4;
  out[at] = markerIndex & 0xff;
  out[at + 1] = 0xee;
  out[at + 2] = 0x77;
  out[at + 3] = 0xff;
  return out;
}
