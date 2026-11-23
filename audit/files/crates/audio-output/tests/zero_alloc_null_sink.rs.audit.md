# AUDIT: crates/audio-output/tests/zero_alloc_null_sink.rs
| Field | Value |
|---|---|
| Auditor | W08-A07 |
| Date | 2026-11-23 |
| Lines / functions | 79 / 1 test + f32_format helper |
| File verdict | REAL (executed: passes; the measurement is sound and would fail on per-callback allocation) |
| Reachable from shipped entry point? | Tests of REAL shipped code: audio-output is a lumen-cli dependency (apps/cli/Cargo.toml) — the NullSink write path is reachable |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `cargo test -p audio-output --test zero_alloc_null_sink` (exit 0, 1 passed); `cargo test -p audio-output` (exit 0, all suites green incl. sine_sink_hash 3, callback_budget 4); rg alloc-scan of ring.rs/render.rs/null.rs |

## 1. One-paragraph verdict
This is a real zero-allocation proof, not a tautology: a process-global counting
allocator wraps System, the measured window covers 256 cycles of BOTH write
entry points plus the device render, and `assert_zero` fails on any allocation
event OR byte. Executed: passes. The hostile question — does the null sink
allocate per callback? — answers NO on the measured path, verified three ways:
the test run, a static scan of `write_bytes` (fixed 256-sample stack chunk;
heap only when one frame exceeds 256 samples, i.e. channels > 256), and
`render_into` (borrow-only signature; the ring allocates solely at
construction). The instrument itself is sound: realloc counts as an allocation
event (grow OR shrink), deallocation is legal, and whole-binary serialization
prevents sibling-thread noise. The three S4s: the comment claims the
"pump→sink→device hand-off" is measured but no PcmPump code runs in the
window; the capture-enabled configuration (which does allocate per render,
driver-side) is excluded without saying so; and the documented channels>256
heap fallback is a per-call allocation hole the suite never pins.

## 2. What actually works (verified)
— `cargo test -p audio-output --test zero_alloc_null_sink` -> 1 passed,
exit 0 (0.07s).
— `cargo test -p audio-output` -> every suite green (33 unit, 4
callback_budget, 1 clock_truth, 4 pcm_pipeline, 3 sine_sink_hash, 1
zero_alloc), exit 0 — the cross-referenced capture/sine-hash ACs the header
cites are real passing tests.
— Mutation reasoning (no edit): reintroducing the "old per-quantum Vec<f32>
materialization" in write_bytes would add ≥1 alloc event per write → 512
events over the window → assert_zero panics with byte/call deltas. The test
cannot pass against an allocating sink.

## 3. Findings
F-001 [S4]
Where: crates/audio-output/tests/zero_alloc_null_sink.rs:68-70
Code:
```
    // The measured window: the pump→sink→device hand-off, both entry
    // points, N cycles. Each render drains exactly one quantum, so
    // the ring never fills (backpressure is a different, typed path).
```
What is wrong: no PcmPump (audio-output/src/pump.rs) code runs inside the
measured window — the test drives `stream.sink` directly. The pump layer's
own allocations (its queue, its policy bookkeeping) are NOT measured by this
binary, yet the comment says the "pump→sink→device hand-off" is what is
measured. The write+render halves are genuinely covered; the pump is not.
What correct looks like: either a pump-driven variant in the window or a
comment that says "the sink entry points the pump calls".
Confidence: HIGH — Verification: read (no PcmPump token in the file).
F-002 [S4]
Where: crates/audio-output/tests/zero_alloc_null_sink.rs:46 + null.rs:302-306
Code:
```
    let mut stream = NullBackend::open_stream(&format, 8192, NullConfig { capture: false }).must();
```
(and in null.rs:)
```
        if let Some(capture) = self.capture.as_mut() {
            for sample in out.iter() {
                capture.extend_from_slice(&sample.to_le_bytes());
            }
```
What is wrong: only `capture: false` is measured. With capture enabled,
`NullDevice::render` grows the capture Vec via `extend_from_slice` — real
allocation events on the render path (amortized, but nonzero: a fresh Vec
doubling from empty over 256 cycles allocates ~log2 times). The header claims
"the null sink's write path" is zero-allocation, which stays true, but the
file nowhere states that the capture configuration is excluded from the
claim. Driver-side capture is documented (null.rs: "capture never runs under
the real-time rules") — the omission is a documentation gap in the test, not
a hidden defect. Confidence: HIGH — Verification: read + ring growth
arithmetic.
F-003 [S4]
Where: crates/audio-output/src/null.rs:207-212 (the code under test)
Code:
```
        let pushed_total = if step <= 256 {
            let mut chunk = [0f32; 256];
            Self::push_aligned(&self.shared.ring, interleaved, &mut chunk, step)
        } else {
            let mut chunk = vec![0f32; step];
```
What is wrong: the heap fallback (channels > 256) allocates a fresh Vec on
EVERY write_bytes call — a per-callback allocation on a geometrically valid
input. It is documented as the "cold absurd-geometry path" and no container
parses that wide, but the zero-allocation contract silently has this hole and
no test pins either its existence or its cold-ness. Confidence: HIGH —
Verification: read (test uses 2 channels, step=256, stack path).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| f32_format (helper) | 30-37 | REAL | — | stereo 44.1k interleaved f32 |
| null_sink_write_path_is_zero_allocation_after_warm_up | 39-79 | REAL | S4 x3 | executed, passes; scope caveats F-001..F-003 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| null_sink_write_path_is_zero_allocation_after_warm_up | YES | any alloc/alloc_zeroed/realloc event or byte in 256 cycles of both write paths + render trips assert_zero with a diagnostic; counting allocator has no pause/noise hole (serialized binary, process-global counter) |
Warm-up design is correct: one cycle per entry point before the guard absorbs
lazy first-call state (Arc/ring construction happens in open_stream, outside
the window by construction).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "after one warm-up cycle, N × (write_bytes + AudioSink::write + device render) allocates zero bytes" | header 1-4 | REAL | test executed, passes |
| "The old per-quantum Vec<f32> materialization is gone (fixed stack chunks)" | header 4-6 | REAL | null.rs:207-209 stack chunk; measured window proves it |
| "the ring bytes are byte-identical (pinned by the capture ACs … and the sine-hash AC)" | header 6-7 | REAL | sine_sink_hash 3/3 and callback_budget 4/4 pass (ran) |
| "the pump→sink→device hand-off" is the measured window | lines 68-69 | FALSE (overstated) | no pump code in the window (F-001) |

## 7. Dead / duplicated / unreachable code
None in this file. The subject path is reachable: audio-output is a direct
dependency of apps/cli (verified in apps/cli/Cargo.toml dependencies).

## 8. Dependencies & flags
dev-deps: testkit (CountingGlobalAlloc, serialize_tests, Must), audio-engine,
media-types. The `#[global_allocator]` static is the one-test-per-binary
pattern (only 1 test here — discipline held). No feature gates.

## 9. Open questions for the next agent
- pump.rs (PcmPump) deserves its own zero-alloc measurement if it runs on the
  real-time-adjacent path — this file does not cover it (F-001).
- cpal.rs (the real-device backend) has no zero-alloc proof at all; the null
  sink's proof must not be quoted as covering the cpal path in any matrix row.
