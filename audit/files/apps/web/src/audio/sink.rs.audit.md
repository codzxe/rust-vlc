# AUDIT: apps/web/src/audio/sink.rs
| Field | Value |
|---|---|
| Auditor | W12-A07 |
| Date | 2026-11-14 |
| Lines / functions | 386 / 22 (2 consts, ProcessReport+underran, QueueFull+Display, WorkletQueue x11 methods, 5 tests) |
| File verdict | UNREACHABLE MODEL — real, exactly-tested arithmetic that NO production code path ever instantiates; the "sink" of the shipped web lane is the null device |
| Reachable from shipped entry point? | NO, twice over: (a) apps/web is outside the lumen-cli/desktop release lane and the product UI never loads the wasm; (b) even inside apps/web, `WorkletQueue` has zero production constructors — `rg WorkletQueue apps/ crates/ tests/` -> only sink.rs itself, mod.rs re-export, lib.rs re-export, README.md:29. The host's audio lane writes to `NullSink` (host.rs:220 `sink: NullSink`, host.rs:233 `self.sink.write_bytes(...)`). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | Read of sink.rs (full) + host.rs audio lane (205-245, 1720-1855) + rg WorkletQueue/WORKLET_QUANTUM/AutoplayGate repo-wide + rg addModule/AudioWorkletNode/destination repo-wide; baseline test citation. |

## 1. One-paragraph verdict
The arithmetic twin of a sink that is not wired to anything. Every rule the module documents — bounded queue with typed whole-chunk refusal, 128-frame quantum, silence-on-underrun with the counter still bumping, played-counter-as-clock — is implemented, pinned by five tests that pass in the baseline, and consumed by nobody: the wasm host's audio path terminates in `NullSink`, no code anywhere pushes decoded PCM into a `WorkletQueue`, no JS drains one, `worker.js` contains no audio handling at all, and `AudioWorkletNode`/`destination` appear in ZERO non-audit files in the repository. The file is honest about being a spec ("This module is that transcription's spec"), but the transcription it specifies (audio-worklet.js) is itself loaded only by an e2e `addModule` parse check — so the entire audible-output apparatus is two well-tested halves with no producer, no consumer, and no connection to the engine between them.

## 2. What actually works (verified)
— 5/5 tests pass (baseline cargo-test-workspace.txt lumen-web block; `audio::sink::tests::*`). Not re-run (cargo budget).
— The math claims are true by inspection: DEFAULT_QUEUE_FRAMES 8192 = 170.7 ms at 48 kHz (matches "~170 ms" doc); PREBUFFER_TARGET 256 = 2×128; `process` bumps `played` by the FULL quantum on underrun (state.rs:264 `self.played = self.played.saturating_add(wanted as u64)`), pinned by `underruns_render_silence_and_keep_the_counter_running` (filled=0 → played += 128).
— Underrun + counter semantics match the native null device's documented discipline (cross-checked against crates/audio-output render.rs audit's "verify the JS twin" note).

## 3. Findings
F-001 [S2]
Where: apps/web/src/audio/sink.rs:101-115 (the model) vs apps/web/src/host.rs:212-240 (the real lane) and repo-wide absence of the bridge
Code (sink.rs's own contract):
```
/// The processor's queue + counter state (the shim's model).
///
/// Interleaved f32 frames, `channels` wide. `process` never
/// allocates (the `VecDeque` pops are O(1); D-1's hot-path rule for
/// what is, on the browser, the real-time audio thread).
```
Code (what the engine actually does — host.rs:218-233):
```
pub struct AudioLane {
    pipeline: AudioPipeline,
    sink: NullSink,
}
...
        if let Some(buffer) = self.pipeline.take_output() {
            self.sink.write_bytes(buffer.data.as_slice())?;
        }
```
What is wrong: the entire audible-output story of the web lane is this file + audio-worklet.js, and NEITHER is connected to the engine. The decoded audio goes to the null device's ring inside wasm memory; `WorkletQueue` is never constructed outside tests; the "chunks cross as transferred `Float32Array` messages routed into the node's port" transport (sink.rs:22-25) exists in no code — `rg "Float32Array|audio" apps/web/worker.js` -> 0 hits; `rg "AudioWorkletNode|destination" --glob '!*audit*'` -> 0 hits in the whole repo; the one browser touch is `await context.audioWorklet.addModule("audio-worklet.js")` (ui/e2e/web/site/harness.js:713) followed by `context.close()` — a parse check, not playback. Why it matters: any claim that the web build produces audible output (AudioWorklet → destination) is false TODAY; the honest statement is host.rs:1845 "the AudioWorklet sink is card 7.H3 (the null device is the only output)". What correct looks like: either wire the pump's audio fill to a queue-drain export (the frame lane's twin for PCM) + a page-side node, or mark the module `[card 7.H3 — unwired]` in lib.rs instead of re-exporting it as product API. Confidence: HIGH — Verification: read + repo-wide rg (all quoted greps re-run for this audit); browser runtime UNVERIFIED (no browser in sandbox — but the absence is in source, not runtime). Severity: S2 because the module is the load-bearing "web audio sink" of the matrix/docs narrative while the shipped lane provably renders nothing audible.

F-002 [S3]
Where: apps/web/src/lib.rs:71
Code:
```
pub use audio::{AutoplayGate, WORKLET_QUANTUM, WorkletQueue};
```
What is wrong: the crate's public API re-exports `WorkletQueue` and `WORKLET_QUANTUM` as if they were the wasm module's audio surface, next to `AutoplayGate` (which IS wired — host.rs:813/849). A consumer of the crate (the future `@lumen/wasm-client` the worker.js comment names) would reasonably conclude the queue is the live sink. It is dead weight in the cdylib (tree-shaken only if unused — it IS unused, so the wasm size cost is zero, but the API claim is misleading). Confidence: HIGH — Verification: read + rg.

F-003 [S4]
Where: apps/web/src/audio/sink.rs:204-215
Code:
```
        if !chunk.len().is_multiple_of(self.channels) {
            // A torn chunk is a routing bug: count it as an overflow
            // refusal (the typed path) rather than inventing a
            // partial-frame semantic the real port never produces.
            self.overflowed = self.overflowed.saturating_add(1);
```
What is wrong: a torn chunk (a ROUTING bug — producer error) increments the same `overflowed_pushes()` counter as a capacity refusal (a CONSUMER pace bug), and the `QueueFull.chunk` field it returns reports `chunk.len() / channels.max(1)` — 3 samples / 2 channels = "1 frame" for what is actually 1.5 torn frames. The two failure classes are indistinguishable in the only observables the shim reports. Comment admits the choice; the conflation is still a diagnostic lie-by-counter. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: apps/web/src/audio/sink.rs:239-254
Code:
```
        let wanted = out.len() / self.channels.max(1);
        let mut filled = 0;
        for sample in out.iter_mut() {
```
What is wrong: when `out.len()` is not a multiple of `channels` (the doc's "processes the prefix it was given" case), the loop still WRITES the trailing partial sample but `wanted` rounds down — the written tail sample is rendered yet uncounted (played bumps by `wanted`). Unreachable when the shim always passes exactly quantum×channels, which the doc says is the real shape. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: apps/web/src/audio/sink.rs:54-59
Code:
```
/// The default queue bound (frames): absorbs the v1 ship granularity
/// (the pump's 1470-frame slices) with headroom while staying a
/// bounded ~170 ms at 48 kHz.
```
What is wrong: 8192 frames / 1470-frame chunks = 5.57 chunks — the bound is NOT chunk-aligned, so the 6th consecutive ship is refused whole mid-flight whenever the consumer stalls for >170 ms; combined with PREBUFFER_TARGET=256 (~5.3 ms) the producer would be shipping every ~5 ms while the bound only absorbs 170 ms of stall. Not wrong arithmetic (170.7 ms is honest), but the two `[PROPOSAL — number]` knobs are unpinned against each other; no test exercises the ship/stall interleaving at the real granularity. Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| WORKLET_QUANTUM / DEFAULT_QUEUE_FRAMES | 49-59 | REAL | S4 | constants pinned by test; F-005 knob interplay |
| ProcessReport::underran | 71-77 | REAL | — | |
| QueueFull + Display | 81-99 | REAL | S4 | torn-chunk chunk field misreports (F-003) |
| WorkletQueue::new | 133-144 | REAL | — | clamps channels≥1, capacity≥quantum (tested) |
| WorkletQueue::channels / capacity / queued_frames | 147-163 | REAL | — | whole-frame invariant by construction |
| WorkletQueue::needs_frames | 168-170 | REAL | — | <256 target (tested) |
| WorkletQueue::played / underruns / silent_frames / overflowed_pushes | 175-195 | REAL | — | observables for the shim's port reports |
| WorkletQueue::push | 204-227 | REAL | S4 | F-003 counter conflation |
| WorkletQueue::process | 239-266 | REAL | S4 | F-004 partial-`out` accounting; O(1) per pop as documented |
| (all of the above) | — | UNREACHABLE | S2 | F-001: no production constructor |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ten_second_filled_playback_simulation_counter_is_exact | YES | 3750 quanta → played == 480 000 exactly; push cadence at the real 1470 granularity |
| underruns_render_silence_and_keep_the_counter_running | YES | filled=0 → played still +128 (pins bump-by-wanted, the counter rule) |
| the_bound_refuses_typed_and_counts | YES | whole-chunk refusal + intact queue + torn-chunk refusal + counter |
| the_prebuffer_target_is_two_quanta | YES | 255/256 boundary |
| the_shared_constants | YES | 128/8192/clamps pinned (the both-halves pin with audio-worklet.js) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the arithmetic twin of the hand-written JS processor" | sink.rs:1-3 | REAL (twinship) | audio-worklet.js audit (next file) compares constants/fill/counter line-by-line |
| "chunks cross as transferred `Float32Array` messages routed into the node's port" | sink.rs:22-25 | FALSE (no such code exists) | rg Float32Array/audio in worker.js -> 0; no producer/consumer anywhere (F-001) |
| "the same fill and counter semantics, proven headless-native" | sink.rs:15-18 | REAL | the 5 tests |
| "AudioWorkletNode + AudioContext" mapping (doc 09 §5.1) | sink.rs:5-6 | PAPER-ONLY | no AudioWorkletNode construction in repo (rg, 0 hits) |
| README: audio-worklet.js = "the audited transcription of the tested WorkletQueue model" | README.md:29 | REAL (file parses) / UNREACHABLE (never runs) | harness.js:713 addModule + close; nothing constructs the node |

## 7. Dead / duplicated / unreachable code
The entire `WorkletQueue` is dead-in-production (F-001/F-002): reachable only from its own tests. `AutoplayGate` (same audio/ module, NOT this file) is the wired sibling — the contrast confirms the sink half never landed.

## 8. Dependencies & flags
std only (VecDeque) — zero deps, as the module claims. No feature gates. wasm32-clean by construction.

## 9. Open questions for the next agent
- audio-worklet.js (my next file): transcription fidelity vs this model — quantum 128, bound, torn-chunk refusal shape, counter-on-silence, report cadence.
- host.rs auditor: the pump's `fill_audio` drives the NULL ring; when card 7.H3 lands, the drain export (PCM bytes out of linear memory, the frame lane's twin) is the missing seam — no issue tracks it in apps/web (rg "7.H3" shows only capability strings).
