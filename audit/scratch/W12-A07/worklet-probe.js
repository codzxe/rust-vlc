// W12-A07 execution probe: run apps/web/audio-worklet.js's REAL processor
// class against the Rust model's own invariants (sink.rs tests), plus a
// demonstration of the silent-drop hole for non-Float32Array chunks.
// Stubs the AudioWorkletProcessor globals; loads the real file verbatim.
"use strict";
const fs = require("fs");
const vm = require("vm");

const registered = {};
class FakePort {
  constructor() { this.messages = []; }
  postMessage(m) { this.messages.push(m); }
  onmessage(_) {}
}
global.AudioWorkletProcessor = class {
  constructor() { this.port = new FakePort(); }
};
global.registerProcessor = (name, cls) => { registered[name] = cls; };
global.sampleRate = 48000; // worklet global (unused by the file, set for parity)

const src = fs.readFileSync("/home/z/my-project/audit-workspace/apps/web/audio-worklet.js", "utf8");
vm.runInThisContext(src, { filename: "audio-worklet.js" });

const P = registered["lumen-worklet-processor"];
if (!P) { console.log("FAIL: processor not registered"); process.exit(1); }
console.log("registered names: " + Object.keys(registered).join(","));

function makeOutputs(channels, frames) {
  const outs = [];
  for (let c = 0; c < channels; c++) outs.push(new Float32Array(frames));
  return [outs];
}

let pass = 0, fail = 0;
function check(name, cond) {
  if (cond) { pass++; console.log("ok   " + name); }
  else { fail++; console.log("FAIL " + name); }
}

// --- Test 1: 10-second filled stereo sim (the Rust test's twin) ---
{
  const p = new P({ processorOptions: { channels: 2, capacityFrames: 8192 } });
  const ship = new Float32Array(1470 * 2).fill(0.5);
  let quanta = 0;
  for (let s = 0; s < 10; s++) {
    for (let q = 0; q < 375; q++) {
      if (p.needsFrames() || p.queuedFrames() < 128) p.push(ship);
      const out = makeOutputs(2, 128);
      p.process([], out);
      quanta++;
    }
  }
  check("T1 quanta == 3750", quanta === 3750);
  check("T1 played == 480000 (exactly ten seconds)", p.played === 480000);
  check("T1 underruns == 0", p.underruns === 0);
  check("T1 silentFrames == 0", p.silentFrames === 0);
  check("T1 overflowed == 0", p.overflowed === 0);
  check("T1 queue <= 8192", p.queuedFrames() <= 8192);
}

// --- Test 2: underrun discipline ---
{
  const p = new P({ processorOptions: { channels: 2 } });
  const out = makeOutputs(2, 128);
  for (let i = 0; i < 5; i++) p.process([], out);
  check("T2 5 empty quanta: played == 640", p.played === 5 * 128);
  check("T2 underruns == 5", p.underruns === 5);
  check("T2 silentFrames == 640", p.silentFrames === 640);
  check("T2 outputs are silence", out[0][0].every((v) => v === 0));
  p.push(new Float32Array(64 * 2).fill(0.5)); // half refill
  p.process([], out);
  check("T2 half refill: filled==64 silent==64 counted as ONE underrun -> underruns == 6", p.underruns === 6);
  check("T2 played == 768", p.played === 6 * 128);
}

// --- Test 3: capacity refusal (typed, whole-chunk) ---
{
  const p = new P({ processorOptions: { channels: 2, capacityFrames: 256 } });
  const chunk = new Float32Array(128 * 2).fill(0.5);
  p.push(chunk); p.push(chunk); // 256 frames = full
  p.push(chunk); // refused
  check("T3 overflowed == 1", p.overflowed === 1);
  const last = p.port.messages[p.port.messages.length - 1];
  check("T3 queueFull posted {queued:256, chunk:128, capacity:256}",
    last && last.type === "queueFull" && last.queued === 256 && last.chunk === 128 && last.capacity === 256);
  check("T3 queue intact at 256", p.queuedFrames() === 256);
  // torn chunk refused too
  p.push(new Float32Array(3).fill(0.5));
  check("T3 torn chunk -> overflowed == 2", p.overflowed === 2);
  check("T3 queue still 256", p.queuedFrames() === 256);
}

// --- Test 4: chunk-boundary continuity across a quantum ---
{
  const p = new P({ processorOptions: { channels: 2 } });
  // A 100-frame chunk followed by a 28-frame chunk = one full quantum.
  const a = new Float32Array(100 * 2); for (let i = 0; i < a.length; i++) a[i] = i;
  const b = new Float32Array(28 * 2).fill(-1);
  p.push(a); p.push(b);
  const out = makeOutputs(2, 128);
  p.process([], out);
  let ok = true;
  for (let f = 0; f < 128; f++) {
    const want = f < 100 ? f * 2 : -1;
    if (out[0][0][f] !== want) { ok = false; break; }
  }
  check("T4 frames served across the chunk boundary in order", ok);
  check("T4 underruns == 0 after boundary fill", p.underruns === 0);
}

// --- Test 5: flush keeps the counter monotonic ---
{
  const p = new P({ processorOptions: { channels: 2 } });
  p.push(new Float32Array(128 * 2).fill(0.5));
  p.process([], makeOutputs(2, 128));
  const before = p.played;
  p.onPortMessage({ type: "flush" });
  check("T5 flush empties the queue", p.queuedFrames() === 0);
  check("T5 flush keeps played", p.played === before);
}

// --- Test 6: report cadence (every 8 quanta) ---
{
  const p = new P({ processorOptions: { channels: 2 } });
  for (let i = 0; i < 8; i++) p.process([], makeOutputs(2, 128));
  const reports = p.port.messages.filter((m) => m.type === "report");
  check("T6 exactly one report after 8 quanta", reports.length === 1);
  check("T6 report carries played/queuedFrames/needsFrames/underruns/silentFrames/overflowed",
    reports[0] && reports[0].played === 1024 && "queuedFrames" in reports[0] &&
    "needsFrames" in reports[0] && "underruns" in reports[0] && "silentFrames" in reports[0] && "overflowed" in reports[0]);
}

// --- Test 7: THE HOLE — a plain-Array chunk is SILENTLY dropped ---
{
  const p = new P({ processorOptions: { channels: 2 } });
  // A page that posts a JSON-round-tripped chunk (plain array) instead of
  // a Float32Array — structured clone preserves typed arrays, but a naive
  // JSON.parse relay does not.
  p.onPortMessage({ type: "chunk", samples: [0.5, 0.5, 0.5, 0.5] });
  check("T7 plain-Array chunk: queue still empty (silently dropped)", p.queuedFrames() === 0);
  check("T7 plain-Array chunk: overflowed NOT incremented", p.overflowed === 0);
  check("T7 plain-Array chunk: NO queueFull message posted", p.port.messages.length === 0);
  // Same for a zero-length Float32Array (accepted as no-op by the Rust model too)
  p.push(new Float32Array(0));
  check("T7 empty Float32Array: no overflow counted", p.overflowed === 0);
}

// --- Test 8: fewer output channels than geometry ---
{
  const p = new P({ processorOptions: { channels: 2 } });
  p.push(new Float32Array([1, 2, 3, 4])); // 2 stereo frames
  const out = makeOutputs(1, 2); // mono output exposes 1 channel
  p.process([], out);
  check("T8 mono output serves both frames, consuming the stereo geometry",
    out[0][0][0] === 1 && out[0][0][1] === 3 && p.queuedFrames() === 0);
}

console.log(`\n${pass} passed, ${fail} failed`);
process.exit(fail === 0 ? 0 : 1);
