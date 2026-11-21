# AUDIT: apps/web/src/audio/sink.rs
| Field | Value |
|---|---|
| Auditor | W12-A02 |
| Date | 2026-11-21 |
| Lines / functions | 386 / 16 (2 consts + 2 types-with-impls + 11 methods + 5 tests; several trivial getters grouped) |
| File verdict | UNREACHABLE (real, native-tested arithmetic spec of an unwired browser lane) |
| Reachable from shipped entry point? | NO. In-crate wiring stops at re-exports (`audio/mod.rs:48`, `lib.rs:71`); `host.rs` runs the `NullSink` for the session clock (`host.rs:220 sink: NullSink`, `host.rs:1845` "the AudioWorklet sink is card 7.H3 (the null device is the only output)"); `worker.js` contains no `addModule`/`AudioWorkletNode`/chunk routing (grep → only an `audioContext` state mention at :28). No test or production code outside this module instantiates `WorkletQueue`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | grep `WorkletQueue\|WORKLET_QUANTUM\|PREBUFFER` across apps/web → only re-exports + the JS twin; read audio-worklet.js (233 lines) side-by-side — constants/semantics compared line-by-line; grep worker.js `worklet\|AudioContext\|addModule` → no wiring; grep host.rs `NullSink\|audio.null-device` → null device is the shipped clock; baseline: `audio::sink::tests::*` 5/5 ok (orchrun-test-workspace-nofailfast.log.txt:3105-3109). |

## 1. One-paragraph verdict
The WorkletQueue is a clean, honestly-labeled arithmetic model: bounded queue with typed whole-chunk refusals, silence-on-underrun with a monotonic played-counter, no per-quantum allocation, and five exact-value tests that all ran green in baseline. Its JS "twin" (apps/web/audio-worklet.js) genuinely matches the constants (128/8192/256), the refusal discipline, and the counter semantics — but the twin already carries an operation (`flush`: drop the queue, keep the counter) that the Rust spec has no method for, breaking the "direct transcription / when either changes, both change" invariant both headers stake their story on, with nothing mechanical enforcing it. And none of it is wired: the v1 wasm host runs the null device as its clock, the worker never instantiates the worklet, so this module is a tested spec for a lane that does not exist in any shipped artifact.

## 2. What actually works (verified)
— Baseline `audio::sink::tests::{ten_second_filled_playback_simulation_counter_is_exact, the_bound_refuses_typed_and_counts, the_prebuffer_target_is_two_quanta, the_shared_constants, underruns_render_silence_and_keep_the_counter_running} ... ok` (orchrun-test-workspace-nofailfast.log.txt:3105-3109) — the arithmetic (480,000 instants = 10 s at 48 kHz; underrun counts; typed refusals) is proven native.
— Twin constants verified equal: sink.rs:52,59,121 (128 / 8_192 / 256) vs audio-worklet.js:55-58 (`QUANTUM=128`, `DEFAULT_CAPACITY_FRAMES=8192`, `PREBUFFER_TARGET=2*QUANTUM`).
— Twin refusal semantics verified: torn chunk refused whole + counted (sink.rs:205-215 vs audio-worklet.js:120-132); bound refusal whole-chunk (sink.rs:217-224 vs js:133-146); zero-length push is a silent no-op in both.

## 3. Findings

### F-001 [S3] The twin invariant is already broken: the JS processor's `flush` operation has no counterpart in this spec model
Where: apps/web/src/audio/sink.rs:15-18 (the claim); apps/web/audio-worklet.js:109-114 (the divergent operation)
Code:
```rust
//! This module is that transcription's **spec**: the same constants
//! ([`WORKLET_QUANTUM`], the bounded queue), the same fill and
//! counter semantics, proven headless-native. The JS file transcribes
//! it; when either changes, both change (the file headers say so).
```
What is wrong: The JS processor accepts `{type: "flush"}` (audio-worklet.js:109-114: `this.queue = []; this.head = 0; this.offset = 0;` with the comment "Drop the queued audio; the counter stays monotonic") — the seek/park path. `WorkletQueue` has no `flush`/`clear`/`drop` method (function inventory: new/channels/capacity/queued_frames/needs_frames/played/underruns/silent_frames/overflowed_pushes/push/process). The JS header's "a direct transcription of the tested Rust model" and this file's "when either changes, both change" are both false in this detail, and the flush semantics (queue drops, counter and underrun/silence accounting unaffected) are exactly the kind of arithmetic rule this module exists to prove — but no test can, because the operation does not exist on the model.
Why it matters: The two-file discipline is the entire correctness story for the real-time JS half ("no engine code runs inside it"); the first divergence appears while both halves are still unwired, and nothing (no shared test, no CI pin) detects the next one.
What correct looks like: `pub fn flush(&mut self)` on `WorkletQueue` (drop queue, keep counters) + a test asserting counter monotonicity across a flush, and the JS calling out the twin method in its header.
Confidence: HIGH — Verification: read both files in full; method inventory exhaustive.
### F-002 [S4] `process()` pops sample-wise, so a misaligned `out` slice permanently breaks the queue's whole-frame invariant — and the doc's "FULL quantum" counter claim is wrong for the short-`out` case it explicitly allows
Where: apps/web/src/audio/sink.rs:239-253
Code:
```rust
    pub fn process(&mut self, out: &mut [f32]) -> ProcessReport {
        let wanted = out.len() / self.channels.max(1);
        let mut filled = 0;
        for sample in out.iter_mut() {
            match self.queue.pop_front() {
```
What is wrong: (a) The loop pops one f32 per output sample; if `out.len() % channels != 0` (channels=2, out.len()=3), the queue is left holding a partial trailing frame — permanently misaligning every later frame (channel swap for the rest of the session), contradicting the `queued_frames` doc "a partial trailing frame is impossible — pushes validate" (sink.rs:158-159): pushes validate, `process` can still break it. (b) The doc at 232-233 says the counter bumps "by the FULL quantum either way", but the code bumps by `wanted` — for the doc-allowed short `out` ("`out` shorter than one quantum processes the prefix it was given"), the bump is the prefix, not a quantum.
Why it matters: The model advertises itself as total/safe for arbitrary shapes; the actual contract is "out must be a multiple of channels" and that precondition is neither documented nor enforced. Production shapes (quantum × channels) are safe; a twin-porting developer reading only this spec would not learn the constraint.
What correct looks like: Iterate frames, not samples (`for frame in out.chunks_mut(self.channels)`), which makes misalignment impossible, and fix the counter doc (or bump by `WORKLET_QUANTUM` for full quanta only and document the prefix rule).
Confidence: HIGH — Verification: read (both traces worked through by hand: 3-sample out on a 2-channel queue leaves 1 sample queued).
### F-003 [S4] The whole module is an unwired lane: the v1 audio path is the null device; nothing instantiates WorkletQueue
Where: apps/web/src/audio/sink.rs:101-115 (the type), wiring evidence host.rs:220/:1845, worker.js (no worklet wiring)
Code:
```rust
pub struct WorkletQueue {
    channels: usize,
    capacity_frames: usize,
    queue: VecDeque<f32>,
```
What is wrong: Nothing outside tests constructs a `WorkletQueue`: `host.rs` runs `NullSink` for the session clock (`host.rs:220`, feature tag `audio.null-device` at :720, and the typed refusal at :1845 "the AudioWorklet sink is card 7.H3 (the null device is the only output)"); `worker.js` never loads `audio-worklet.js` (no `addModule`, no `AudioWorkletNode`, no chunk routing). The JS processor's `registerProcessor` target is loaded by no page in the repo.
Why it matters: The wave question — "is there a real audio output path on web?" — answers NO: v1 wasm emits no sound, by the capability snapshot's own admission (capabilities.rs:290-300, "audioworklet-output: Blocked … v1 runs the null device"). The dead-ness is honestly documented; the cost is two maintained-but-unwired halves whose only enforcement is prose (F-001).
What correct looks like: Card 7.I1 wiring (context creation, chunk routing, counter feedback) or a stub marker on the lane.
Confidence: HIGH — Verification: grep wiring surfaces (worker.js, src/) → absent; capability row cross-checked.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `WORKLET_QUANTUM` | 49-52 | REAL | — | 128; pinned by test + JS twin. |
| `DEFAULT_QUEUE_FRAMES` | 54-59 | REAL | — | 8192 (~170 ms @48 kHz); `[PROPOSAL]` labeled. |
| `ProcessReport` + `underran` | 61-77 | REAL | — | filled/silent; `underran` = silent>0. |
| `QueueFull` + `Display` | 79-99 | REAL | — | typed refusal with queued/chunk/capacity. |
| `WorkletQueue` (struct) | 101-115 | REAL | F-003 | unwired lane. |
| `PREBUFFER_TARGET` | 117-121 | REAL | — | 256 = 2× quantum; matches JS. |
| `new` | 123-144 | REAL | — | clamps channels≥1, capacity≥quantum; mirrors JS constructor (verified). |
| getters: `channels`/`capacity`/`queued_frames`/`needs_frames`/`played`/`underruns`/`silent_frames`/`overflowed_pushes` | 146-195 | REAL | — | all trivial, correct; clamped-pinning tested (:383-384). |
| `push` | 197-227 | REAL | — | torn-chunk whole refusal (counted as overflow, documented at 206-208); bound refusal whole; empty chunk no-op. Matches JS twin. |
| `process` | 229-266 | REAL | F-002 | sample-wise popping; counter/underrun accounting correct for aligned `out`. |
| `tests::ten_second_filled_playback_simulation_counter_is_exact` | 280-305 | REAL | — | exact 480,000 counter; would fail on any arithmetic drift. |
| `tests::underruns_render_silence_and_keep_the_counter_running` | 312-337 | REAL | — | full+partial underrun shapes, exact counts. |
| `tests::the_bound_refuses_typed_and_counts` | 341-361 | REAL | — | refusal fields + queue intact + torn refusal. |
| `tests::the_prebuffer_target_is_two_quanta` | 365-375 | REAL | — | boundary at 255/256. |
| `tests::the_shared_constants` | 379-385 | REAL | F-001 | pins Rust constants only — cannot see the JS twin; "both change together" has no mechanical check. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ten_second_filled_playback_simulation_counter_is_exact | YES | exact counter/underrun/queue-bound asserts over 3,750 quanta. |
| underruns_render_silence_and_keep_the_counter_running | YES | exact per-quantum underrun counts incl. the half-filled quantum. |
| the_bound_refuses_typed_and_counts | YES | exact refusal payload + queue integrity. |
| the_prebuffer_target_is_two_quanta | YES | boundary equality. |
| the_shared_constants | YES (Rust side) | pins 128/8192/clamps; blind to the JS half (F-001) and to process() misalignment (F-002 — no test passes a non-multiple `out`). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the same constants, the same fill and counter semantics … when either changes, both change" | module doc 15-18 | FALSE (flush divergence) | F-001; constants/fill/counter verified equal, flush is JS-only. |
| "process never allocates (the VecDeque pops are O(1))" | doc 103-105 | REAL | read: pops only; no allocation in process. |
| "the played counter counts rendered instants — filled AND silent … Pause freezes it" | doc 30-37 | REAL (model) | tests :313-337; pause behavior lives in the autoplay gate/context handling (host.rs:1057), not here — consistent with the doc's pointer. |
| "The arithmetic here is proven by this crate's native tests (simulation) … browser behavior is `[VERIFY]`" | doc 39-45 | REAL | baseline :3105-3109; browser-matrix-20261110.md lists audio scenarios as refused-by-design (no worklet wiring). |
| capability row `audioworklet-output: Blocked` ("v1 runs the null device") | capabilities.rs:290-300 | REAL | host.rs:220/:1845 (NullSink is the only output); worker.js has no worklet wiring. |

## 7. Dead / duplicated / unreachable code
- `WorkletQueue`/`ProcessReport`/`WORKLET_QUANTUM` re-exported (`lib.rs:71`, `audio/mod.rs:48`) but constructed only by this module's tests — production-dead until card 7.I1 (F-003).
- No duplicated logic within Rust; the JS twin is the deliberate copy (see F-001 for its un-twin'd operation).

## 8. Dependencies & flags
- std-only (`VecDeque`), `testkit` (dev). No feature gates, no unsafe, no unwrap/expect/panic (workspace lints honored; `#[must_use]` on all public getters). No external deps.

## 9. Open questions for the next agent
- A03 (audio-worklet.js owner): (a) the flush operation (js:109-114) — file the twin-divergence from your side too (F-001); (b) js:172 `const channels = Math.min(this.channels, output.length)` — outputs with MORE channels than the queue geometry leave the extra planar channels unwritten (relies on browser pre-zeroing — is that guaranteed?); (c) REPORT_EVERY_QUANTA=8 has no Rust counterpart — wiring decision, fine, but note it.
- A05/A06 (audio/mod.rs, autoplay, host.rs AudioContext handling): does the AudioContext state report (protocol.rs:103) actually pause the null-device clock, and is that tested? The sink doc defers pause semantics there.
- Card 7.I1's [VERIFY] matrix owns the browser wiring — if it never lands, this module + the JS processor are permanent dead code and should be marked as such.
