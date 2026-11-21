# AUDIT: apps/web/audio-worklet.js
| Field | Value |
|---|---|
| Auditor | W12-A03 |
| Date | 2026-11-21 |
| Lines / functions | 233 / 7 (class constructor, queuedFrames, needsFrames, onPortMessage, push, process, registerProcessor registration; plus 4 module consts) |
| File verdict | UNREACHABLE (well-formed, parse-verified in one manual browser capture; logic never executed anywhere; nothing ships or gates it) |
| Reachable from shipped entry point? | NO. The ONLY in-repo loader is `ui/e2e/web/site/harness.js:713` (`await context.audioWorklet.addModule("audio-worklet.js")` — parse/load only, no `AudioWorkletNode` instantiation anywhere: Grep `AudioWorkletNode` repo-wide → only doc/comment mentions), staged by `ui/scripts/web-matrix-run.mjs:460` (`copyFileSync(workletPath, … "audio-worklet.js")` — answers the worker.js auditor's staging question). The web-matrix run is a manual dev lane (docs/web/browser-matrix-20261110.md: "the record you are reading is that capture … NOT re-run"); no CI workflow references this file or apps/web at all (workflow Grep: zero hits — the web-budget gate builds the lumen-web .wasm via xtask but stages/lints/checks no JS). The shipped `lumen` CLI never touches this file. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:3 |
| Commands run | Read audio-worklet.js 1-233 (full) · Read apps/web/src/audio/sink.rs 1-386 (the claimed tested model — line-by-line transcription comparison, see §2) · Grep `audio-worklet\|lumen-worklet-processor\|addModule` repo-wide (loader census: harness.js:713 only) · Grep `AudioWorkletNode` (no instantiation anywhere) · Grep `AudioWorkletNode\|type: "chunk"` (chunk producer: nobody) · `sed -n '690,751p' ui/e2e/web/site/harness.js` (the addModule-only scenario) · `sed -n '445,470p' ui/scripts/web-matrix-run.mjs` (staging copy) · `rg -n "worklet" docs/web/browser-matrix-20261110.{md,json}` (capture rows :81/:107 `workletModuleLoaded:true`, two isolation modes) · `rg -ln "worklet" ui/src ui/tests ui/e2e/web/web-matrix.spec.ts` (zero test/lint coverage) · `cat ui/eslint.config.js` head (lint scope = ui/ only; apps/web not linted) · baseline `orchrun-test-workspace-nofailfast.log.txt:3105-3109` (the Rust model's 5 tests ok). Quotes verified with the Read tool. |

## 1. One-paragraph verdict
The transcription claim survives hostile inspection: I compared every constant and every semantic against the natively-tested Rust model (sink.rs) — quantum 128, capacity 8192 with the same clamps, prebuffer 256, torn-chunk visible refusal, whole-chunk bound refusal, one-underrun-per-silent-quantum, counter bump by the full quantum through silence — and the JS is faithful, including subtleties like the truncating `Math.floor(samples.length / this.channels)` on torn refusals that mirrors Rust's integer division. But that is where the evidence ends: no `AudioWorkletNode` is instantiated anywhere in the repository, no chunk producer exists, no test runner, no linter (ESLint's scope is ui/; apps/web is unlinted), no CI lane — the ONLY verification this file has ever received is one manual 2026-11-10 browser capture asserting `workletModuleLoaded: true`, i.e. that it parses. The web target's entire audible-output path (engine → chunk message → worklet → speaker) has never executed; a logic typo in `process()` would pass every gate in the repo. Remaining defects: a wrong-typed chunk is silently dropped in direct contradiction of the file's own "never a silent drop" discipline, output channels beyond the queue's geometry are left unwritten (stale-buffer garbage risk), and two behaviors (`flush`, the 8-quantum report cadence, plus a `needsFrames` field the header's protocol spec omits) have no counterpart in the "tested model" the file claims to transcribe.

## 2. What actually works (verified)
— Verified by line-level comparison against `apps/web/src/audio/sink.rs` (the model), whose 5 tests ran green in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3105-3109`:
  - Constants: `QUANTUM=128` (js:56) = `WORKLET_QUANTUM=128` (sink.rs:52); `DEFAULT_CAPACITY_FRAMES=8192` (js:57) = `DEFAULT_QUEUE_FRAMES=8_192` (sink.rs:59); `PREBUFFER_TARGET=2*QUANTUM` (js:58) = `PREBUFFER_TARGET=2*WORKLET_QUANTUM` (sink.rs:121, test-pinned =256).
  - Clamps: `Math.max(1, channels|0)` (js:65) = `channels.max(1)` (sink.rs:136); `Math.max(QUANTUM, capacity)` (js:66-69) = `capacity_frames.max(WORKLET_QUANTUM)` (sink.rs:137, test-pinned).
  - Torn chunk: visible refusal + `overflowed+=1` + `chunk: Math.floor(len/channels)` (js:123-133) = `Err(QueueFull{chunk: chunk.len()/channels.max(1)})` + `overflowed+=1` (sink.rs:205-215) — including the truncation semantics.
  - Bound: `queued + frames > capacity` refuses the WHOLE chunk (js:135-146) = sink.rs:217-224.
  - Counter discipline: underrun counted once per quantum-with-silence, `silentFrames` accumulates the extent, `played += frames` (the full quantum) either way (js:208-214) = sink.rs:253-264.
— Verified by `docs/web/browser-matrix-20261110.md:81,107` (+ .json:306-312,590-596): the module loaded in real Chromium 157 in both isolation modes (`workletModuleLoaded: true`, `contextState: "running"`) — the file parses; nothing more is claimed by that capture.

## 3. Findings
### F-001 [S3]
Where: apps/web/audio-worklet.js:159-230 (with the loader census)
Code:
```
  process(inputs, outputs) {
    const output = outputs[0];
    if (!output || output.length === 0) {
      return true;
    }
```
What is wrong: The processor's logic has never executed. Grep repo-wide: no `AudioWorkletNode` instantiation exists (only doc mentions — sink.rs:6,44, doc 09:178); no code sends `{type:"chunk"…}` (Grep `type: "chunk"` → only this file's header); the harness scenario (`ui/e2e/web/site/harness.js:700-724`) calls `addModule` and asserts `workletModuleLoaded` — parse/load only — then closes the context. No unit test, no vitest file, no ESLint scope (ui/eslint.config.js covers ui/ only), no CI lane (no workflow references apps/web). The one manual capture (2026-11-10) is the file's entire verification history.
Why it matters: This is the web target's ONLY audio output path — doc 09's Phase-7 success criterion ("plays to canvas + WebAudio") terminates here. Every mutation of `process`/`push` logic (an off-by-one in the offset advance, a wrong channel loop, a broken compaction) passes every automated gate in the repository; the "audited transcription" is audited by eyeball only. The file's own header honestly labels browser-runtime behavior `[VERIFY]` — which keeps this out of fraud territory — but "card 7.I1's matrix owns those rows" points at a matrix whose audio row is exactly this parse-only check.
What correct looks like: Either instantiate the node in the harness (push N chunks, await a `report` message, assert `played`/`underruns` — the model's own ten-second simulation ported to the browser lane), or run the JS through a Node-side harness with a `process()` stub until the browser lane exists; at minimum add the file to some lint/CI surface.
Confidence: HIGH — Verification: repo-wide Greps (loader/producer/test/lint/CI all enumerated); harness read; browser-matrix capture read.
### F-002 [S4]
Where: apps/web/audio-worklet.js:119-122
Code:
```
  push(samples) {
    if (!(samples instanceof Float32Array) || samples.length === 0) {
      return;
    }
```
What is wrong: A chunk that is not a `Float32Array` (page sent a plain Array, a detached buffer, or a structurally wrong message) is dropped SILENTLY — no `overflowed` bump, no `queueFull` message, nothing. This contradicts the file's own discipline, stated three times in the header and enforced for every other malformed input: "a torn chunk is REFUSED whole, never a silent drop" (js:22-23), "the page's routing bug, visible, never a silent gap" (js:33-34). A wrong-typed chunk IS a routing bug and it IS a silent gap. The Rust model cannot express this case (`&[f32]` is typed), so the model's tests never covered it.
Why it matters: The one failure mode the visible-refusal design exists to surface is the one that vanishes.
What correct looks like: `this.overflowed += 1; this.port.postMessage({type:"queueFull", …, reason:"not-a-float32array"})` on the type mismatch (empty chunks may stay no-ops).
Confidence: HIGH — Verification: read; contrast with js:123-133 (torn) and js:135-146 (full).
### F-003 [S4]
Where: apps/web/audio-worklet.js:164 (with 181-183)
Code:
```
    const channels = Math.min(this.channels, output.length);
    const frames = output[0].length;
```
What is wrong: When the node's output exposes MORE channels than the queue's geometry (`output.length > this.channels`), the channels beyond `min()` are never written (the fill loops stop at `channels`, js:181-183, 198-200) — and AudioWorklet output buffers are not guaranteed zeroed between callbacks (implementations may reuse them), so the extra channel can carry stale or garbage samples. The consuming-a-partial-frame direction is handled deliberately and correctly (js:184-186 consumes the full interleaved frame); the surplus-output direction is not. The Rust model is interleaved-only and has no planar concept — a transcription gap the model could never catch.
Why it matters: A mono stream into a default stereo node (the classic page misconfiguration) yields possible noise in the right channel — the exact class of bug the "hand-audited shim" framing is supposed to preclude.
What correct looks like: After the frame loop, zero any `output[channel]` for `channel >= channels` (or construct the node with `outputChannelCount: [this.channels]` and document that the page must match).
Confidence: MEDIUM — Verification: read (the unwritten-channel case is structural); the actual audible artifact depends on browser buffer-reuse behavior the file itself marks [VERIFY].
### F-004 [S4]
Where: apps/web/audio-worklet.js:24-26, 36-38, 59, 109-115 (with sink.rs 1-386)
Code:
```
     *                      {type: "flush"} (drop the queue — the seek/park
     *                      path; the played counter is NOT touched: it is
     *                      the clock's truth and stays monotonic)
```
(and js:59 `const REPORT_EVERY_QUANTA = 8;`)
What is wrong: The header claims "a direct transcription of the tested Rust model in apps/web/src/audio/sink.rs" — but `WorkletQueue` (sink.rs:107-267) has no `flush` operation and no report cadence; both are JS-only behaviors with no tested twin (the cadence is at least marked `[VERIFY]` in the header; `flush` is presented as if it were part of the model). Additionally the header's protocol spec for the report message (js:27-28: `{type: "report", played, queuedFrames, underruns, silentFrames, overflowed}`) omits the `needsFrames` field the code actually sends (js:223) — a spec/code mismatch in the very contract a page author would implement against.
Why it matters: "Direct transcription of the tested model" is the file's entire verification story; three behaviors sit outside it, and one of the two documented message shapes is wrong.
What correct looks like: Add `flush()` to the Rust model with a test (drop-queue/keep-counter is exactly the kind of invariant the model exists to pin), fix the header's report shape, and note the cadence constant as JS-side policy.
Confidence: HIGH — Verification: read of both files side by side; flush/`REPORT_EVERY_QUANTA` absent from sink.rs; report fields compared.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module consts (QUANTUM/DEFAULT_CAPACITY_FRAMES/PREBUFFER_TARGET/REPORT_EVERY_QUANTA) | 56-59 | REAL | S4(F-004) | First three verified equal to sink.rs:52/59/121 (and test-pinned there, orchrun:3108); hand-maintained with no cross-language pin (the mod.rs audit's F-001 — nothing fails on divergence); REPORT_EVERY_QUANTA is JS-only policy. |
| `constructor` | 62-83 | REAL | — | Clamps mirror sink.rs:134-144 exactly (verified); port handler wired; no per-quantum allocation (queue/head/offset preallocated). |
| `queuedFrames` | 86-95 | REAL | — | O(remaining chunks) — bounded small (capacity 8192 / 1470-frame ships ⇒ ≤6 chunks + compaction); `Math.floor` division exact given the push-side whole-frame invariant. |
| `needsFrames` | 98-100 | REAL | — | = sink.rs:168-170 (`< PREBUFFER_TARGET`). |
| `onPortMessage` | 103-116 | REAL | S4(F-002 via push) | Type-gates the message shape; `flush` resets cursor only — counter monotonic (matches header; untested — F-004). |
| `push` | 119-148 | REAL | S4(F-002) | Torn + bound refusals are exact transcriptions (verified vs sink.rs:204-227 incl. truncating chunk-frames on torn); non-Float32Array case is the silent-drop hole (F-002). |
| `process` | 159-230 | REAL (never executed) | S3(F-001), S4(F-003) | Frame loop + cross-chunk pops + silence-on-underrun + one-underrun-per-quantum + full-quantum counter bump — all faithful to sink.rs:239-266 (verified); head compaction bounded (≥64); channel-surplus gap (F-003); zero execution evidence (F-001). Returns true forever — documented lifecycle choice. |
| `registerProcessor("lumen-worklet-processor", …)` | 233 | REAL | — | The name the harness's addModule resolves; loads (browser-matrix capture, both modes). |

## 5. Test honesty
No tests exist for this file in any runner (Grep: ui/src, ui/tests, e2e spec — zero worklet references; apps/web has no JS test tooling). The tests that exist are the RUST MODEL's (sink.rs:269-386, 5/5 green, orchrun:3105-3109): they verify the model this file transcribes, not this file. Any claim of coverage for audio-worklet.js that cites them is coverage-by-analogy. The browser-matrix `audio-context-and-worklet` scenario (harness.js:700-724) asserts constructible + module-loaded — parse only, and it was run once, manually.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "a tiny, fixed, hand-audited JS shim … a direct transcription of doc 03 §4.9(c)'s allowed operations; no engine code runs inside it" | doc 09 §5.1 (quoted in file header 4-8) | REAL (content) / REAL-UNPROVEN (execution) | Transcription verified constant-by-constant and semantic-by-semantic vs sink.rs (§2); "no engine code" holds (233 lines, pure queue/counter logic); but the shim itself never executed (F-001). |
| "The model's arithmetic is proven headless-native by this crate's tests" | file header 9-10 | REAL | sink.rs tests green (orchrun:3105-3109) — proves the MODEL, not the transcription (F-001/F-004). |
| "a torn chunk is REFUSED whole, never a silent drop" / "visible, never a silent gap" | file header 22-23, 33-34 | FALSE (one case) | Non-Float32Array chunks drop silently (F-002). |
| Report protocol `{type:"report", played, queuedFrames, underruns, silentFrames, overflowed}` | file header 27-28 | FALSE (shape) | Code also sends `needsFrames` (js:223) — header omits it (F-004). |
| "Browser-runtime behavior … is [VERIFY] — card 7.I1's matrix; no browser exists in this sandbox" | file header 48-51 | REAL (honest) | The one capture (browser-matrix 2026-11-10) checked parse/load only — the honest boundary is stated. |
| Web column "plays to canvas + WebAudio" (Phase 7 gate) | doc 09 §1 success criterion | PAPER-ONLY | No WebAudio output path has ever executed (F-001); feature-matrix web column correctly reports 0 implemented. |
| "worker.js 7,073 B + audio-worklet.js 9,012 B = 16,085 B raw" (size accounting) | docs/web/size-report.md:100 | REAL | File is 233 lines / 9,012 bytes (wave-00 manifest.tsv:103) — matches. |

## 7. Dead / duplicated / unreachable code
- The entire file is UNREACHABLE from shipped entry points (no product loads it; the harness loader is a manual lane).
- No dead code INSIDE the file: every message branch, counter, and clamp is live relative to the file's own contract (the absent consumers are the file-level problem, not internal orphans).
- Cross-language duplication (JS vs sink.rs) is deliberate and documented — the risk is unpinned divergence, already recorded as mod.rs F-001 by the audio/mod.rs auditor.

## 8. Dependencies & flags
- Zero dependencies: plain ES module + `AudioWorkletProcessor` global + `registerProcessor` — matches doc 09 §5.1's "no engine code" rule.
- Not covered by any lint config (ui/eslint.config.js scopes ui/ and e2e/web/site; apps/web/*.js is outside every tool's scope), not in any CI artifact list, not in the cargo workspace (JS file inside a Rust crate dir; only size-counted by size-report.md).
- Served-from-same-directory claim (worker.js:38-39) verified real: ui/scripts/web-matrix-run.mjs:460 stages it next to worker.js and the wasm.

## 9. Open questions for the next agent
- worker.js auditor's question ANSWERED (recorded here): web-matrix-run.mjs:460 does copy audio-worklet.js into the site dist — the serving claim holds for the manual lane.
- Whoever owns ui/e2e/web (browser-matrix lane): the `audio-context-and-worklet` scenario is one `new AudioWorkletNode` + two port messages away from actually exercising this file (F-001's fix); the model's ten-second simulation is directly portable.
- apps/web/src/audio/sink.rs owner (A02): add `flush()` to the model (F-004) — the JS already ships the semantics; the model is the half that's missing the operation.
- doc 09 §5.1 owner: the "[PROPOSAL — shim is generated from a template with the ring constants]" note is the structural fix for the unpinned-constants risk (mod.rs F-001 + this file's F-004); worth promoting before any browser-audio claim lands in the matrix.
