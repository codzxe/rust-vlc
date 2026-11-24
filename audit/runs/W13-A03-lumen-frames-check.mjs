// W13-A03: substitute conformance check for the MISSING prototype/harness.mjs.
// Golden bytes hand-derived from apps/desktop/src/wire.rs:720-756 (golden_bytes_match_the_spec_table).
import { FrameDecoder, encodeFrame, parseHeader, PROTOCOL } from "/home/z/my-project/audit-workspace/apps/desktop/prototype/lumen-frames.mjs";

function goldenBytes() {
  const b = new Uint8Array(56);
  const dv = new DataView(b.buffer);
  b.set([0x4c, 0x55, 0x46, 0x31], 0);           // magic LUF1
  dv.setUint16(4, 48, true);                     // header_len
  dv.setUint16(6, 1, true);                      // version
  b[8] = 1;                                      // format Rgba8
  b[9] = 0b0000_0001;                            // flags repeat
  dv.setUint16(10, 0, true);                     // reserved
  dv.setUint32(12, 2, true);                     // width
  dv.setUint32(16, 1, true);                     // height
  dv.setUint32(20, 8, true);                     // payload_len
  dv.setBigUint64(24, 0x1122334455667788n, true);// frame_id
  dv.setBigInt64(32, -1000n, true);              // pts_ns
  dv.setBigUint64(40, 0x0102030405060708n, true);// emitted_ns
  b.set([10, 20, 30, 40, 50, 60, 70, 80], 48);   // payload
  return b;
}

let failures = 0;
function check(name, cond, detail) {
  if (!cond) { failures++; console.log(`FAIL ${name} ${detail ?? ""}`); }
  else console.log(`ok   ${name}`);
}

// 1. Golden decode (frame_id AND emitted_ns swapped to SAFE values: the
// Rust golden's 0x1122334455667788 (1.23e18) and 0x0102030405060708
// (7.26e16) BOTH exceed 2^53=9.007e15 and are REFUSED by the JS
// safe-integer guard — mirror divergences, checked separately in 1b).
{
  const bytes = goldenBytes();
  new DataView(bytes.buffer).setBigUint64(24, 42n, true);
  new DataView(bytes.buffer).setBigUint64(40, 0x01020304n, true);
  const d = new FrameDecoder();
  d.feed(bytes);
  const f = d.nextFrame();
  check("golden: frame present", f !== null);
  check("golden: headerLen", f.header.headerLen === 48, JSON.stringify(f.header));
  check("golden: format rgba8", f.header.formatName === "rgba8");
  check("golden: repeat", f.header.repeat === true);
  check("golden: dims 2x1", f.header.width === 2 && f.header.height === 1);
  check("golden: payloadLen 8", f.header.payloadLen === 8);
  check("golden: frameId 42", f.header.frameId === 42, String(f.header.frameId));
  check("golden: ptsNs -1000", f.header.ptsNs === -1000, String(f.header.ptsNs));
  check("golden: emittedNs safe", f.header.emittedNs === 0x01020304, String(f.header.emittedNs));
  check("golden: payload bytes", Array.from(f.payload).join(",") === "10,20,30,40,50,60,70,80");
  check("golden: drained", d.nextFrame() === null);
  d.finish(); // no throw
}

// 1b. The Rust golden's own frame_id (0x1122334455667788 = 1.23e18) AND
// emitted_ns (0x0102030405060708 = 7.26e16) are each refused typed by the
// JS decoder — the committed Rust golden vector is UNDECODABLE by the JS
// mirror (the "byte for byte mirror" claim fails on the committed vector).
expectThrow("divergence: Rust golden frame_id refused", "unsafe-integer", () => {
  const d = new FrameDecoder();
  d.feed(goldenBytes());
  d.nextFrame();
});
{
  const onlyEmitted = goldenBytes();
  new DataView(onlyEmitted.buffer).setBigUint64(24, 42n, true);
  expectThrow("divergence: Rust golden emitted_ns refused", "unsafe-integer", () => {
    const d = new FrameDecoder();
    d.feed(onlyEmitted);
    d.nextFrame();
  });
}

// 2. Chunked feed at EVERY split point (the Rust twin's test).
{
  const bytes = goldenBytes();
  new DataView(bytes.buffer).setBigUint64(24, 42n, true);
  new DataView(bytes.buffer).setBigUint64(40, 0x01020304n, true);
  let ok = true;
  for (let split = 0; split <= bytes.length; split++) {
    const d = new FrameDecoder();
    d.feed(bytes.subarray(0, split));
    let f = d.nextFrame();
    if (f === null && split < bytes.length) {
      d.feed(bytes.subarray(split));
      f = d.nextFrame();
    }
    if (f === null || f.header.frameId !== 42) { ok = false; break; }
  }
  check("chunked: every split point decodes", ok);
}

// 3. Malformed refusals.
function expectThrow(name, kind, fn) {
  try { fn(); check(name, false, "no throw"); }
  catch (e) { check(name, e instanceof Error && e.kind === kind, `kind=${e.kind} (${e.message})`); }
}
{
  const base = goldenBytes();
  const badMagic = base.slice(); badMagic[0] = 0x58;
  expectThrow("malformed: bad magic", "bad-magic", () => { const d = new FrameDecoder(); d.feed(badMagic); d.nextFrame(); });
  const badVer = base.slice(); new DataView(badVer.buffer).setUint16(6, 2, true);
  expectThrow("malformed: version 2", "unsupported-version", () => { const d = new FrameDecoder(); d.feed(badVer); d.nextFrame(); });
  const badFlags = base.slice(); badFlags[9] = 0x03;
  expectThrow("malformed: reserved flag bit", "reserved-not-zero", () => { const d = new FrameDecoder(); d.feed(badFlags); d.nextFrame(); });
  const badRes = base.slice(); new DataView(badRes.buffer).setUint16(10, 1, true);
  expectThrow("malformed: reserved u16", "reserved-not-zero", () => { const d = new FrameDecoder(); d.feed(badRes); d.nextFrame(); });
  const badLen = base.slice(); new DataView(badLen.buffer).setUint32(20, 9, true);
  expectThrow("malformed: payload_len mismatch", "payload-length-mismatch", () => { const d = new FrameDecoder(); d.feed(badLen); d.nextFrame(); });
  const zeroDim = base.slice(); new DataView(zeroDim.buffer).setUint32(12, 0, true); new DataView(zeroDim.buffer).setUint32(20, 0, true);
  expectThrow("malformed: zero width", "bad-geometry", () => { const d = new FrameDecoder(); d.feed(zeroDim); d.nextFrame(); });
  const truncBase = base.slice();
  new DataView(truncBase.buffer).setBigUint64(24, 42n, true);
  new DataView(truncBase.buffer).setBigUint64(40, 0x01020304n, true);
  expectThrow("truncated: finish throws", "truncated", () => { const d = new FrameDecoder(); d.feed(truncBase.subarray(0, 50)); d.finish(); });
  // Note: with an UNSAFE frame_id, finish() on the same partial record throws
  // "unsafe-integer" (header parse runs before the truncation accounting) —
  // diagnostic-order divergence vs the Rust twin, noted in the audit.
  const huge = base.slice(); new DataView(huge.buffer).setUint32(12, 65535, true); new DataView(huge.buffer).setUint32(16, 65535, true);
  new DataView(huge.buffer).setUint32(20, 65535 * 65535 * 4, true);
  expectThrow("malformed: implied payload over 1GiB cap", "bad-geometry", () => { const d = new FrameDecoder(); d.feed(huge); d.nextFrame(); });
}

// 4. Encoder round-trip.
{
  const payload = new Uint8Array(2 * 2 * 4).fill(7);
  const bytes = encodeFrame({ formatCode: 2, repeat: false, width: 2, height: 2, frameId: 5, ptsNs: 12345, emittedNs: 999 }, payload);
  const d = new FrameDecoder();
  d.feed(bytes);
  const f = d.nextFrame();
  check("encode: round-trip bgra8", f !== null && f.header.formatName === "bgra8" && f.header.frameId === 5 && f.payload.length === 16);
}

console.log(failures === 0 ? "ALL CHECKS PASSED" : `${failures} FAILURES`);
process.exit(failures === 0 ? 0 : 1);
