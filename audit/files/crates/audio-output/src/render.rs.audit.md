# AUDIT: crates/audio-output/src/render.rs
| Field | Value |
|---|---|
| Auditor | W08-A04 |
| Date | 2026-11-21 |
| Lines / functions | 311 / 10 (incl. 4 tests + 1 test helper) |
| File verdict | REAL (core `render_into`+`RenderStats`; components `RenderCallback`/`RingRenderer` UNREACHABLE) |
| Reachable from shipped entry point? | YES for `render_into`/`RenderStats` (call chain: `lumen` CLI `open` → apps/cli/src/host.rs `advance_time` host.rs:613-617 / drain host.rs:823-826 / `discard_ring_prefix` host.rs:1273-1277 → `NullDevice::render` null.rs:292-306 → `render_into` render.rs:89). NO for `RenderCallback` trait + `RingRenderer` (zero consumers). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "NullBackend|builtin_backends|RingRenderer|render_into|RenderCallback|PcmPump|UnsupportedBackend|cpal_backend"` (caller census — established: only NullDevice calls render_into in production; RingRenderer/RenderCallback/builtin_backends/PcmPump have no non-test callers) · `rg -n "dyn RenderCallback|: RenderCallback|RenderCallback>" --glob *.rs` (0 matches — trait never used as a bound anywhere) · `rg -n "builtin_backends"` (definition only + worklog mention) · `sed -n` quote verification for every finding · baseline `test-workspace.log.txt` (audio-output: 33 unit + 13 integration tests, all ok) · read docs/03-engine-plan.md §4.8–§4.9 (claim sources) · read tests/{callback_budget,zero_alloc_null_sink}.rs (AC evidence cited in module docs) |

## 1. One-paragraph verdict
The reachable core of this file is real, small, and correct: `render_into` is a genuinely
allocation-free, lock-free, bounded real-time callback core whose no-blocking contract is
both structurally true (traced call graph: one `SpscRing::read`, one `SampleCounter::advance`,
one `fetch_add`, one slice fill) and measured (`tests/zero_alloc_null_sink.rs` traps the render
path under a counting global allocator, zero bytes over 256 cycles, baseline 1/1 ok). The rest
of the file is a dead seam: the `RenderCallback` trait is used as a bound by nothing in the
workspace, and `RingRenderer` — documented as "The production RenderCallback" — is constructed
only by its own unit test. The module docs also under-claim the delivered enforcement state
(the "owed to testkit" fault-injecting allocator already exists and already covers this path).
No OS device thread will ever call this code as shipped: the only consumer is the null device,
whose output the CLI drops (see §9).

## 2. What actually works (verified)
- `render_into` executes and passes 4/4 in-module tests + 4/4 callback-budget ACs + 1/1
  zero-alloc proof — verified by baseline `audit/runs/baseline/test-workspace.log.txt`:
  `test render::tests::full_read_copies_everything_and_advances_once ... ok`,
  `render::tests::underrun_fills_silence_and_counts_one_event ... ok`,
  `render::tests::ring_segments_stay_within_the_power_of_two_wrap_budget ... ok`,
  `render::tests::renderer_trait_callback_produces_the_same_output ... ok`;
  `test result: ok. 33 passed` (unit) plus `callback_budget` 4 passed,
  `zero_alloc_null_sink` 1 passed (`null_sink_write_path_is_zero_allocation_after_warm_up ... ok`).
- No-allocation claim of the module docs: verified structurally (no owned value constructed;
  call graph traced into ring.rs `read`, counter.rs `advance`) and measured
  (`tests/zero_alloc_null_sink.rs:71-78` guard window includes `stream.device.render(&mut out)`
  → `render_into`, asserts zero over 256 cycles).
- Arithmetic audit of `render_into` (read-only reasoning): `channels ≥ 1` by ring construction
  (ring.rs:71 refuses 0), so no div-by-zero; `frames_requested * channels ≤ out.len()` (no
  overflow); `read.frames ≤ frames_requested` (ring read is `wanted.min(available)`), so
  `silence` cannot underflow; fill slice bounds in range. No unwrap/expect/panic path, no
  unsafe, no lock, no syscall in the graph.
- Underrun policy "output silence, never repeat old audio" (doc 03 §4.8 d) implemented at
  render.rs:102-105 and pinned by `callback_budget.rs:251-254` ("pure silence, never old
  audio", baseline ok).

## 3. Findings

### F-001 [S3]
Where: crates/audio-output/src/render.rs:51-57 and 122-134
Code:
```rust
pub trait RenderCallback: Send {
    /// Fill `out` with the next `out.len()` interleaved samples. The
    ...
    fn render(&mut self, out: &mut [f32]);
}
```
```rust
/// The production [`RenderCallback`]: a shared ring + counters view.
///
/// Constructed at open; moved into (or handed to) whatever drives the
/// device.
```
What is wrong: `RenderCallback` is consumed by nothing. A workspace-wide search for any use
of the trait as a bound (`dyn RenderCallback`, `: RenderCallback`, `RenderCallback>`) returns
zero matches in all `*.rs`; the only mentions outside this file are doc comments
(apps/web/src/audio/sink.rs:7, crates/audio-engine/src/convert.rs:15). `RingRenderer` is
constructed exactly once outside its own definition — by its own unit test (render.rs:297).
Calling it "The production RenderCallback" is a name that lies: no production path can reach it.
Why it matters: this is the file's answer to doc 03 §4.9(b)'s device-thread shape, and it is
pure scaffolding — the hostile question ("all seams?") is literally true for half this file.
Dead public API also rots silently: no compile error will fire when the RT contract drifts,
because nothing exercises it.
What correct looks like: either delete the trait + `RingRenderer` until a real backend exists,
or wire the trait into a device-thread emulation the session layer actually drives. Rename to
make the not-yet-wired status explicit (the crate's own style: `[PROPOSAL]`).
Confidence: HIGH — Verification: read (rg caller census: 0 bound-usages; 1 test-only constructor).

### F-002 [S3]
Where: crates/audio-output/src/render.rs:85-88 vs 148-154
Code:
```rust
/// `out.len()` must be a multiple of `channels` — a device buffer is
/// always whole frames; an unaligned tail is left **untouched** (the
/// driver-side wrapper validates and reports; the RT core stays
/// branch-cheap).
```
```rust
impl RenderCallback for RingRenderer {
    fn render(&mut self, out: &mut [f32]) {
        // The trait shape returns nothing; the ledger is discardable
        // by design (the RT path stays allocation- and branch-cheap).
        render_into(&self.ring, &self.counter, &self.underruns, out);
    }
}
```
What is wrong: the documented contract says the driver-side wrapper "validates and reports"
unaligned buffers. The one driver that validates is `NullDevice::render` (null.rs:293-295,
`audio.render_buffer_unaligned`). The file's own advertised production wrapper,
`RingRenderer::render`, performs no validation and cannot report — the trait signature returns
`()`. An unaligned `out` would leave its tail holding whatever bytes were there before
(stale/garbage samples in a real device buffer), silently. Additionally, `RenderStats` — the
module's central "operation ledger" — is unobservable through the trait: `render` discards it,
so a real backend wired via `RenderCallback` loses all budget observability the module docs
are built around.
Why it matters: when ADR-005 lands and a platform adapter adopts `RingRenderer` as designed,
it inherits a silent-contract hole exactly where the docs promise a check. Blast radius today
is zero only because F-001 makes the type unreachable.
What correct looks like: validate alignment in `RingRenderer::render` (fill unaligned tail
with silence, or debug_assert) or change the trait to return the ledger / a typed result;
keep `render_into` pure as documented.
Confidence: HIGH — Verification: read (both sites + null.rs:292-306 compared).

### F-003 [S4]
Where: crates/audio-output/src/render.rs:24-33
Code:
```rust
//! counter advance, and (driver-side, outside the callback) zero
//! storage growth across any number of renders. What this v0 gate
//! cannot do is *trap* an allocation the way doc 03 §4.9 c's
//! fault-injecting allocator would — that hook needs a
//! `#[global_allocator]` (i.e. `unsafe`, platform-adapter territory
//! per doc 01 §2.7) and is `[PROPOSAL: tooling]` owed to testkit
//! hardening.
```
What is wrong: factually outdated. The `#[global_allocator]` hook already exists —
`testkit::alloc_count::CountingGlobalAlloc` (crates/testkit/src/alloc_count.rs:236-237,
`unsafe impl GlobalAlloc` under Lead decision ED-b) — and this crate already uses it:
`tests/zero_alloc_null_sink.rs:12-14` installs it and its measured window
(tests/zero_alloc_null_sink.rs:71-78) includes `device.render` → `render_into`, asserting
zero allocations over 256 cycles (baseline: 1 passed). The claim "cannot do is *trap* an
allocation" is therefore false of the current tree; it also hides real delivered evidence
(under-claiming — the opposite direction of inflation, but still doc rot). Relatedly,
line 23-24 asserts the AC checks "`silence_fills ≤ 1`" — the ledger carries
`silence_filled` *frames*, not fill *operations*; no counter of fill ops exists.
Why it matters: the module docs are the "code review" half of doc 03 §4.9(c)'s gate; a gate
document that misstates which half of the enforcement exists misdirects the next auditor
and the ADR-005 admission decision.
What correct looks like: rewrite the paragraph to the actual state: counting-allocator
trap exists and covers the render path; the *panicking* fault-injector and lock-tracer are
the still-owed `[PROPOSAL]` parts; fix `silence_fills ≤ 1` → "at most one silence-fill
operation (structural; the ledger reports the filled frame count)".
Confidence: HIGH — Verification: read alloc_count.rs + zero_alloc_null_sink.rs + baseline log.

### F-004 [S4]
Where: crates/audio-output/src/render.rs:128-134 and 137-140
Code:
```rust
pub struct RingRenderer {
    ring: std::sync::Arc<SpscRing>,
    counter: std::sync::Arc<SampleCounter>,
    underruns: std::sync::Arc<AtomicU64>,
    channels: u32,
}
```
```rust
    pub fn channels(&self) -> u32 {
        self.channels
    }
```
What is wrong: `channels` duplicates state the ring already owns (`ring.channels()`); the
copy is set only by the in-module test (fields are private, no public constructor exists) and
nothing asserts it equals the ring's channel count. `render_into` uses the ring's value, the
getter returns the copy — a future constructor that lets them diverge produces a renderer
that reports one channel count and renders at another.
Why it matters: duplication of invariants across SPSC halves is exactly the class of bug the
no-blocking contract cannot afford; the type is the template for the real backend (ADR-005).
What correct looks like: drop the field; `channels()` returns `self.ring.channels()`; or add
a `debug_assert_eq!` in any future constructor.
Confidence: HIGH — Verification: read (no public constructor exists; only test literal at
render.rs:297-302 sets it, matching values).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `trait RenderCallback` (decl) | 51-57 | UNREACHABLE | S3 | matches doc 03 §4.9(b) shape verbatim; used as a bound by nothing in the workspace |
| `RenderCallback::render` (trait method) | 56 | UNREACHABLE | — | signature returns `()` → cannot report errors or the ledger (F-002) |
| `struct RenderStats` | 60-72 | REAL | — | plain Copy ledger; fields consistent with what `render_into` produces; consumed by NullDevice::render, tests, `fully_served` |
| `RenderStats::fully_served` | 74-79 | REAL | — | `frames_read == frames_requested`; trivial, correct; note: a 0-frame request reports `true` (vacuous, contract-impossible input) |
| `render_into` | 89-120 | REAL | — | the RT core; traced graph: 1× `SpscRing::read`, 1× `advance`, 1× `fetch_add`, 1× slice fill; no alloc/lock/syscall/panic; bounds checked (§2); silence policy per doc 03 §4.8 d; `let _ =` on fetch_add is idiomatic discard of a return value, not a swallowed error |
| `struct RingRenderer` | 128-134 | UNREACHABLE | S3/S4 | private fields, no public constructor; only in-module test constructs it (F-001, F-004) |
| `RingRenderer::channels` | 137-140 | UNREACHABLE | S4 | duplicated state (F-004) |
| `RingRenderer::ring` | 142-146 | UNREACHABLE | — | getter with no callers |
| `<RingRenderer as RenderCallback>::render` | 148-154 | UNREACHABLE | S3 | no alignment validation, discards the ledger (F-002); delegates correctly to `render_into` |
| test helper `setup` | 163-169 | — | — | builds ring/counter/underruns; uses `testkit::Must` |
| test `full_read_copies_everything_and_advances_once` | 171-190 | PASS (baseline) | — | would fail under mutation: misordered ring copy, wrong stats fields, counter not advanced |
| test `underrun_fills_silence_and_counts_one_event` | 192-214 | PASS (baseline) | — | kills the advance-by-read mutation (`count()==3`), the no-silence-fill mutation (NAN prefill), the no-underrun-count mutation |
| test `ring_segments_stay_within_the_power_of_two_wrap_budget` | 216-289 | PASS (baseline) | — | strongest in file: parks cursors at non-round alignments, request sizes 1..16 incl. > capacity; pins exact wrapped values `[2.0,3.0,4.0,5.0]`; would fail on any wrap/segment miscount |
| test `renderer_trait_callback_produces_the_same_output` | 291-310 | PASS (baseline) | — | would fail if `RingRenderer::render` stopped delegating (data half `[0.5,-0.5]` pinned) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| full_read_copies_everything_and_advances_once | YES | out is prefilled NAN and compared element-wise; counter/underrun asserted exactly |
| underrun_fills_silence_and_counts_one_event | YES | second empty-call block pins counter==5 and underruns==2 — catches advance-by-read and per-frame underrun counting |
| ring_segments_stay_within_the_power_of_two_wrap_budget | YES | exact wrapped-sample expectations + segment bound over 9 request sizes incl. oversized; any index-mask error breaks `[2.0,3.0,4.0,5.0]` |
| renderer_trait_callback_produces_the_same_output | YES | out prefilled 0.0; the data half `[0.5,-0.5]` distinguishes delegation from no-op |
All four ran and passed in baseline (`test-workspace.log.txt`, `33 passed` for the crate's
unit suite). No `#[ignore]`, no tautologies, no mocks of the subject under test. The
integration ACs that the module docs cite (`callback_budget.rs` 4/4, `zero_alloc_null_sink.rs`
1/1) also ran and passed in baseline.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Inside `render` it is forbidden: any lock …; allocation; logging; syscalls; unbounded loops" | doc 03 §4.9(c) | REAL | render_into traced graph (§2); zero-alloc measured by tests/zero_alloc_null_sink.rs (baseline 1 passed); segments ≤ 2 asserted in callback_budget.rs (baseline 4 passed) |
| "The callback only: reads ring indices, `memcpy`s frames, fills silence on underrun, increments the sample counter" | doc 03 §4.9(c) | REAL | render.rs:98-112 does exactly these four things, nothing else |
| "code review plus these tests are the gate" (allocator hook is `[PROPOSAL: tooling detail]`) | doc 03 §4.9(c) | REAL (hook) but render.rs:27-33 misstates it as entirely owed | testkit already ships the counting `#[global_allocator]` (alloc_count.rs:236); the *panicking* fault-injector/lock-tracer remain proposals (F-003) |
| "audible glitch policy: output silence, never repeat old audio" | doc 03 §4.8(d) | REAL | render.rs:102-105; pinned by callback_budget.rs:251-254 ("pure silence, never old audio") |
| `pub trait RenderCallback { fn render(&mut self, out: &mut [f32]); }` runs "on the device's high-priority thread" | doc 03 §4.9(b) | UNREACHABLE | no OS device thread exists in any shipped build; the only implementor is dead code (F-001); in the shipped CLI the "device thread" is the session loop calling `NullDevice::render` on the caller's thread |

## 7. Dead / duplicated / unreachable code
- `RenderCallback` (trait), `RingRenderer`, `RingRenderer::channels`, `RingRenderer::ring`,
  `<RingRenderer as RenderCallback>::render` — UNREACHABLE (F-001/F-002/F-004); re-exported at
  lib.rs:141 so they compile into the shipped crate as dead public API.
- Crate-level dead seams observed while tracing (not this file, for the domain ledger):
  `builtin_backends()` (lib.rs:152, zero callers), `PcmPump`/`SinePattern` (test-only consumers).

## 8. Dependencies & flags
- Crate deps: audio-engine, media-clock, media-types (path deps); dev-dep testkit. No external
  deps, no unsafe, std-only — matches Cargo.toml header claims (verified by reading both).
- No feature flags gate anything in this file; the cpal gate (`AUDIO_ADR005_ADMITTED=false`,
  cpal.rs:43) is a different module and cannot reach this file.
- Workspace lints inherited (`[lints] workspace = true`, Cargo.toml:29-30); no `#[allow]` in
  this file.

## 9. Open questions for the next agent
- apps/cli/src/host.rs render loops allocate a fresh `vec![0.0_f32; RENDER_QUANTUM * channels]`
  per quantum (host.rs:613, 822, 1274) — outside the RT callback but on the pump path; doc 03
  §4.8(c) says "The pump never allocates per frame". CLI-domain agent should classify.
- The shipped CLI discards every rendered buffer (host.rs:613-826): audible output is
  impossible by construction; the null device's only real effect is counter/clock pacing.
  Confirm no other shipped binary (tauri is UNVERIFIED-BUILD per baseline) wires a real sink.
- apps/web/src/audio/sink.rs + apps/web/audio-worklet.js: the only potentially audible path in
  the repo (browser AudioWorklet, hand-written JS). Unverified in sandbox; not reachable from
  the `lumen` CLI. Web-domain agent should audit the JS shim against this file's semantics.
- `SinePattern`/`PcmPump` (test-only today) are the intended engine-side pump; when the engine
  adopts them, re-audit `DropPolicy::Drop` interaction with `NullSink::write_bytes` all-or-nothing
  refusal (`audio.ring_full` when `high_water` > ring capacity — pump.rs docs flag it as a
  wiring bug; nothing enforces `high_water ≤ ring capacity`).
