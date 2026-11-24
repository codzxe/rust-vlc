// W13-A03: substitute for the MISSING prototype/harness.mjs — decode the
// Rust-emitted golden stream (emit_stream --out) with lumen-frames.mjs's
// FrameDecoder and check every expectation in expected.json.
import { FrameDecoder } from "/home/z/my-project/audit-workspace/apps/desktop/prototype/lumen-frames.mjs";
import { readFileSync } from "node:fs";

const dir = process.argv[2] ?? "/home/z/my-project/audit-workspace/audit/runs/proto-test1";
const stream = new Uint8Array(readFileSync(`${dir}/stream.bin`));
const expected = JSON.parse(readFileSync(`${dir}/expected.json`, "utf8"));

function fnv1a64(bytes) {
  let hash = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  for (const b of bytes) {
    hash ^= BigInt(b);
    hash = BigInt.asUintN(64, hash * prime);
  }
  return hash.toString(16).padStart(16, "0");
}

let failures = 0;
function check(name, cond, detail) {
  if (!cond) { failures++; console.log(`FAIL ${name} ${detail ?? ""}`); }
  else console.log(`ok   ${name}`);
}

check("protocol block", expected.protocol.magic === "LUF1" && expected.protocol.version === 1 && expected.protocol.headerLen === 48);
check("streamBytes count", expected.streamBytes === stream.length, `${expected.streamBytes} vs ${stream.length}`);
check("frameCount", expected.frameCount === expected.frames.length);

// Chunked feed mirroring emit_stream's LCG split (1..4096-byte chunks).
let lcg = 0x853c49e6748fea9bn;
const decoder = new FrameDecoder();
let offset = 0;
const t0 = Date.now();
while (offset < stream.length) {
  lcg = BigInt.asUintN(64, lcg * 6364136223846793005n + 1442695040888963407n);
  const chunk = Math.min(Number((lcg >> 33n) & 0xfffn) + 1, stream.length - offset);
  decoder.feed(stream.subarray(offset, offset + chunk));
  offset += chunk;
}
const frames = [];
for (let f; (f = decoder.nextFrame()) !== null; ) frames.push(f);
const ms = Date.now() - t0;

check("decoded frame count", frames.length === expected.frames.length, `${frames.length} vs ${expected.frames.length}`);

let allHeaders = true, allDigests = true, allHeadTail = true;
for (let i = 0; i < frames.length; i++) {
  const f = frames[i], e = expected.frames[i];
  const hOk = f.header.frameId === e.frameId && f.header.width === e.width && f.header.height === e.height
    && f.header.formatCode === e.formatCode && f.header.repeat === e.repeat && f.header.ptsNs === e.ptsNs
    && f.header.emittedNs === e.emittedNs && f.header.payloadLen === e.payloadLen;
  if (!hOk) { allHeaders = false; console.log(`  header mismatch frame ${i}: ${JSON.stringify(f.header)} vs ${JSON.stringify(e)}`); break; }
  const digest = fnv1a64(f.payload);
  if (digest !== e.fnv1a64) { allDigests = false; console.log(`  digest mismatch frame ${i}: ${digest} vs ${e.fnv1a64}`); break; }
  const head = Array.from(f.payload.slice(0, 8), (b) => b.toString(16).padStart(2, "0")).join("");
  const tail = Array.from(f.payload.slice(-8), (b) => b.toString(16).padStart(2, "0")).join("");
  if (head !== e.head || tail !== e.tail) { allHeadTail = false; console.log(`  head/tail mismatch frame ${i}`); break; }
}
check("all 90 headers match expected.json", allHeaders);
check("all 90 FNV-1a payload digests match", allDigests);
check("all 90 head/tail hex windows match", allHeadTail);

console.log(`decoded ${frames.length} frames / ${stream.length} bytes through the JS decoder in ${ms} ms (LCG-chunked feed)`);
console.log(failures === 0 ? "SUBSTITUTE HARNESS: ALL CHECKS PASSED" : `${failures} FAILURES`);
process.exit(failures === 0 ? 0 : 1);
