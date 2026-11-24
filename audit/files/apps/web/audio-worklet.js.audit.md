# AUDIT: apps/web/audio-worklet.js
| Field | Value |
|---|---|
| Auditor | W12-A07 |
| Date | 2026-11-14 |
| Lines / functions | 233 / 8 (constructor, queuedFrames, needsFrames, onPortMessage, push, process, registerProcessor call, header protocol) |
| File verdict | REAL (executed: 28/28 invariant checks against the real file) but UNREACHABLE — no counterparty exists: nothing constructs the node, nothing sends `chunk`, nothing reads `report` |
| Reachable from shipped entry point? | NO. Only touch in the repo: `await context.audioWorklet.addModule("audio-worklet.js")` (ui/e2e/web/site/harness.js:713) — a parse/load check followed by `context.close()`. `rg "AudioWorkletNode|connect\(.*destination" --glob '!*audit*'` -> 0 hits repo-wide. The product UI never loads it (staged into the e2e site by ui/scripts/web-matrix-run.mjs:460). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | `node --check apps/web/audio-worklet.js` -> exit 0; EXECUTED the real file via `vm.runInThisContext` with stubbed AudioWorkletProcessor/port globals — audit/scratch/W12-A07/worklet-probe.js -> 28 passed, 0 failed, exit 0 (first run 26/2 — the 2 fails were probe indexing bugs, fixed, re-run clean; debug cross-check in transcript). |

## 1. One-paragraph verdict
The transcription claim is TRUE and now executed: I loaded the real file in node and ran its actual processor class against the Rust model's own test invariants — the 10-second filled simulation lands on exactly 480 000 played frames, underruns render whole silent quanta with the counter still bumping, capacity and torn chunks are refused whole with typed `queueFull` messages, frames flow in order across chunk boundaries, flush keeps the counter monotonic, and the report cadence fires every 8 quanta. The file is nonetheless dead code in the product: its port protocol (`chunk`/`flush` in, `report`/`queueFull` out) has no producer and no consumer anywhere in the repository — the engine's audio terminates in the null device inside wasm memory, and the only browser code that touches this file parses it and closes the context. One real transcription hole exists: a wrong-TYPE chunk (plain Array instead of Float32Array) is silently dropped with no counter bump and no message, contradicting the file's own "never a silent drop" rule.

## 2. What actually works (verified)
— EXECUTED: `node audit/scratch/W12-A07/worklet-probe.js` -> `28 passed, 0 failed`, exit 0. The probe stubs `AudioWorkletProcessor`/`registerProcessor`, loads `apps/web/audio-worklet.js` verbatim, and asserts: T1 (10 s @48 kHz stereo, 3750 quanta, 1470-frame ships) `played === 480000`, `underruns === 0`; T2 underrun discipline (5 empty quanta → played 640, underruns 5; half-refill → ONE underrun, played 768); T3 typed refusals (`queueFull {queued:256, chunk:128, capacity:256}` + queue intact + torn chunk refused); T4 in-order service across a 100+28-frame chunk boundary; T5 flush empties queue, keeps `played`; T6 exactly one `report` per 8 quanta with all six fields; T7 the silent-drop hole (below); T8 mono-output consumes full stereo geometry.
— `node --check` exit 0 (syntax), matching the gate's own check.
— Constants match sink.rs exactly: 128 / 8192 / 256 (verified against sink.rs:52/59/121 by Read).

## 3. Findings
F-001 [S2]
Where: apps/web/audio-worklet.js:17-34 (the port protocol) vs the repo
Code:
```
 * Port protocol (the page routes chunks in; the processor posts
 * evidence out — v1 is `threading: single`, no SharedArrayBuffer, so
 * chunks cross as transferred Float32Array messages):
 *   page -> processor   {type: "chunk", samples: Float32Array}
```
What is wrong: the protocol's other end does not exist. No code in the repository constructs an `AudioWorkletNode` (rg over apps/, ui/, scripts/ — 0 non-audit hits), so `registerProcessor("lumen-worklet-processor", ...)` (line 233) registers a processor nobody instantiates; no code posts `{type:"chunk"}` or `{type:"flush"}`; no code reads the `report`/`queueFull` messages that would build "the wallclock anchor — doc 09 §5.2". The engine half ends at the null device (host.rs:220 `sink: NullSink`); this file is the second half of an apparatus with no middle. Why it matters: this file is the repo's entire answer to "how does the web produce sound", and it is unreachable, unloaded (except one parse check), and unconnected. What correct looks like: the pump-side PCM drain export + page-side node wiring (card 7.H3's actual landing), or an explicit `[UNWIRED — card 7.H3]` marker in the header instead of a protocol description that reads as live. Confidence: HIGH — Verification: repo-wide rg + read (browser runtime UNVERIFIED — no browser in sandbox; the absence is structural, in source).

F-002 [S3]
Where: apps/web/audio-worklet.js:120-122
Code:
```
    if (!(samples instanceof Float32Array) || samples.length === 0) {
      return;
    }
```
What is wrong: a chunk that arrives as a plain Array (the shape any JSON relay — the ABI's own JSON-over-words lane! — produces) fails the `instanceof` check and returns with NO `overflowed` bump and NO `queueFull` post: a silent drop, in the file whose header promises "a torn chunk is REFUSED whole, never a silent drop" (line 22-23). Executed proof (probe T7): after `onPortMessage({type:"chunk", samples:[0.5,0.5,0.5,0.5]})` — `queuedFrames() === 0`, `overflowed === 0`, `port.messages.length === 0`. A page wiring bug would manifest as silence with zero diagnostic evidence — exactly the failure mode the typed-refusal design exists to prevent. Why it matters: the one realistic integration mistake (JSON relay loses the typed-array) is the one input shape the file swallows whole. What correct looks like: count it and post `queueFull` (or a dedicated `badChunk` message). Confidence: HIGH — Verification: ran (probe T7, output pasted above).

F-003 [S4]
Where: apps/web/audio-worklet.js:27-28 vs 219-227
Code:
```
 *   processor -> page   {type: "report", played, queuedFrames,
 *                        underruns, silentFrames, overflowed}
```
vs the actual message (line 223): `needsFrames: this.needsFrames(),` — the header's protocol contract omits the `needsFrames` field the code actually sends. Doc/code drift inside one file. Confidence: HIGH — Verification: read + probe T6 asserts the six-field shape.

F-004 [S4]
Where: apps/web/audio-worklet.js:123-131
Code:
```
    if (samples.length % this.channels !== 0) {
      // A torn chunk is a routing bug: refuse it whole, visibly.
      this.overflowed += 1;
      this.port.postMessage({
        type: "queueFull",
        queued: this.queuedFrames(),
        chunk: Math.floor(samples.length / this.channels),
```
What is wrong: the torn-chunk refusal reports `chunk: Math.floor(len/channels)` — 3 stereo samples report "chunk: 1" when 1.5 torn frames were refused. Faithful transcription of the Rust model's identical wart (sink.rs:210-214), so the halves do match — but the diagnostic both halves emit is arithmetically false. Confidence: HIGH — Verification: read + probe T3 (torn path exercised).

F-005 [S4]
Where: apps/web/audio-worklet.js:164 + 198-201
Code:
```
    const channels = Math.min(this.channels, output.length);
```
(with the silence branch writing only `channels` outputs.)
What is wrong: when the output exposes MORE channels than the queue geometry (node misconfigured mono-queue on a stereo output), the extra channels are left at whatever the output buffer holds — the Rust model writes every sample of `out` unconditionally (sink.rs:242-252). Whether WebAudio zero-fills worklet outputs between quanta is runtime behavior this sandbox cannot verify (UNVERIFIED); if it does not, stale samples loop. Transcript asymmetry, not an executed defect (probe T8 covers the fewer-channels direction, which IS handled deliberately at 184-186). Confidence: MEDIUM — Verification: read; runtime fill policy UNVERIFIED.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| constructor | 62-83 | REAL (executed) | — | clamps channels≥1, capacity≥128; stub-port driven in probe |
| queuedFrames | 86-95 | REAL (executed) | — | whole-frame invariant; O(chunks) scan per call (not per frame) — fine at ≤6 chunks typical |
| needsFrames | 98-100 | REAL (executed) | — | <256 target, matches model |
| onPortMessage | 103-116 | REAL (executed) | — | flush keeps counter (T5); non-object ignored |
| push | 119-148 | REAL (executed) | S3/S4 | F-002 silent drop; F-004 torn-chunk chunk field |
| process | 159-230 | REAL (executed) | S4 | F-005 unfilled extra channels; boundary continuity verified; bounded compaction at head≥64 |
| registerProcessor call | 233 | REAL | S2 | registers a processor no code instantiates (F-001) |
| header protocol doc | 17-34 | PAPER | S4 | F-003 omits needsFrames; protocol has no counterparty (F-001) |

## 5. Test honesty
No tests in this file. External evidence:
| Evidence | Honest? | Reason |
|---|---|---|
| worklog.md:743 "node harness (not committed — /home/z/audioworklet-harness) … 25/25 checks PASS" | UNVERIFIABLE | the harness lives outside the repo; the number is not reproducible from the tree. My committed probe (audit/scratch/W12-A07/worklet-probe.js, 28 checks) now provides in-repo execution evidence. |
| e2e harness `addModule` + assert loaded (harness.js:713-722) | WEAK | proves the file PARSES in a browser; says nothing about process(), the port protocol, or audio reaching a destination |
| `node --check` in the gate (worklog) | WEAK | syntax only — re-run here, exit 0 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a direct transcription of the tested Rust model … WorkletQueue" | audio-worklet.js:5-6 | REAL | executed probe T1-T6 matches sink.rs's five tests' invariants exactly (constants, counter, refusals, boundary, flush) |
| "a torn chunk is REFUSED whole, never a silent drop" | audio-worklet.js:22-23 | FALSE for wrong-type chunks | F-002: plain-Array chunk drops silently (probe T7) |
| "the counter evidence the page pairs with AudioContext.currentTime to build the wallclock anchor — doc 09 §5.2" | audio-worklet.js:29-31 | PAPER-ONLY | no page code reads `report` messages (rg) |
| "chunks cross as transferred Float32Array messages" | audio-worklet.js:19 | PAPER-ONLY | no sender exists (F-001) |
| README: "the audited transcription of the tested WorkletQueue model" | README.md:29 | REAL (arithmetic) / UNREACHABLE (deployment) | probe + rg |

## 7. Dead / duplicated / unreachable code
The whole file is unreachable in the product (F-001). No duplication beyond the intended model/twin pairing — and the pairing is genuinely synchronized (constants + semantics verified equal by execution).

## 8. Dependencies & flags
Zero dependencies, no imports — runs only inside an AudioWorklet global scope (AudioWorkletProcessor, registerProcessor, sampleRate). Plain JS, no build step. Served by the e2e site builder only (web-matrix-run.mjs:451,460).

## 9. Open questions for the next agent
- Who owns card 7.H3's landing? The missing seams are: (a) a PCM drain export on the wasm ABI (the frame lane's twin), (b) worker.js audio routing, (c) the page-side AudioWorkletNode + destination wiring, (d) the wallclock anchor consumer of `report`. None exist; no issue/TODO in apps/web tracks them (rg "7.H3" -> capability strings only).
- If a JSON relay ever feeds this processor, F-002 guarantees silent audio loss — fix before wiring anything.
