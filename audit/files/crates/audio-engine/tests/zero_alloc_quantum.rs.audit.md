# AUDIT: crates/audio-engine/tests/zero_alloc_quantum.rs
| Field | Value |
|---|---|
| Auditor | W07-A04 |
| Date | 2026-11-23 |
| Lines / functions | 105 / 2 (1 test) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (proof test) — it exercises the shipped `AudioPipeline` (apps/cli/src/pipeline_host.rs:1600) directly; instrument = `testkit::alloc_count::CountingGlobalAlloc` (verified sound, see below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p audio-engine --test zero_alloc_quantum` → 1 passed (0.21s); same with `-- --test-threads=1` → 1 passed (0.28s); baseline cargo-test-workspace.txt:121 lists the binary |

## 1. One-paragraph verdict
This is a real zero-allocation proof, not a syntactic one: a counting global allocator wraps `System`, the window (`ALLOC.state().guard()`) is opened only after warm-up, 512 full quanta run through `process → mix → take_output → drop`, and `assert_zero` panics with exact byte/event deltas if even one allocation (including `realloc`/`alloc_zeroed`, both wrapped in the instrument) occurs. I verified the instrument's semantics by reading `crates/testkit/src/alloc_count.rs:194-203` — `assert_zero` requires `alloc_calls == 0 && total_bytes == 0` while explicitly permitting deallocations (pool returns) — and the realloc path is counted (alloc_count.rs:262), so a hidden `Vec::push` growth inside the window would fail loudly. The test also characterizes the geometry-change cold path (one half-size quantum outside the window, then 8 quanta at the new geometry asserted zero). Executed both default and `--test-threads=1` (the module doc's both-modes claim): PASS. The single coverage caveat: the proof models the immediate-drop consumer (the null-sink shape); a real device holding >4 quantum buffers forces the bounded cold path (OUT_SLOTS rotation), which is allocation-per-quantum by design — bounded, not zero.

## 2. What actually works (verified)
— `cargo test -p audio-engine --test zero_alloc_quantum` → `1 passed` (0.21s); `-- --test-threads=1` → `1 passed` (0.28s). Mutation reasoning: breaking `pooled_slot`'s warm path (forcing the cold path) or any per-quantum `Vec` growth trips `assert_zero` with the byte count printed.

## 3. Findings
F-001 [S4]
Where: crates/audio-engine/tests/zero_alloc_quantum.rs:79-83
Code:
```
        // The pump's consumption shape: take, hand to the sink, drop —
        // the slot returns to refcount 1 inside the window (a drop is
        // a dealloc, never an alloc).
        drop(pipeline.take_output().must());
```
What is wrong (coverage limitation, honestly labeled): the zero-alloc proof holds for the immediate-drop consumer only. A sink that retains more than `OUT_SLOTS`(4) quantum buffers (an async real-device callback, the Wave-8 cpal shape) drives `pooled_slot` into its cold arm — one fresh `SharedBytes` allocation per quantum, bounded by the rotation, never zero. The test comment calls this "the pump's consumption shape" which is true for today's null sink (`NullSink::write_bytes` copies synchronously and drops) but should not be read as a device-independent guarantee. What correct looks like: a second window asserting the retention shape's bound (alloc_calls ≤ N per N quanta), or a doc note. Confidence: HIGH — Verification: read (pooled_slot cold arm, pipeline.rs:431-446).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| quantum_frame (helper) | 32-51 | REAL | — | per-quantum-varying s16 payload |
| audio_quantum_path_is_zero_allocation_after_warm_up | 53-105 | REAL | S4 | strong measured proof; immediate-drop shape only (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| audio_quantum_path_is_zero_allocation_after_warm_up | YES | counting-allocator guard panics on any alloc/realloc/alloc_zeroed event in the window (instrument verified: alloc_count.rs:194-203, 238-262); warm-up and geometry-change cold path correctly excluded via window placement |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "allocates **zero bytes** — measured by the counting global allocator, not argued from code reading" (L2-5) | zero_alloc_quantum.rs | REAL | executed: 1 passed, both thread modes |
| "The binary runs identically under `--test-threads=1` and the default parallel runner" (L10-11) | zero_alloc_quantum.rs | REAL | executed both → 1 passed each |
| "a drop is a dealloc, never an alloc" (L81) | zero_alloc_quantum.rs | REAL | instrument permits deallocs in the window (alloc_count.rs:187-189) |

## 7. Dead / duplicated / unreachable code
None. (`serialize_tests()` is redundant for a one-test binary but harmless and future-proof.)

## 8. Dependencies & flags
audio-engine, media-types, testkit (alloc_count + Must). Standalone test binary with its own `#[global_allocator]` — one test per binary by design (module doc L7-9); this file honors that (exactly one `#[test]`).

## 9. Open questions for the next agent
- Wave 8 (cpal): when a real device holds render buffers asynchronously, re-prove the pump's allocation bound under retention (F-001) — the OUT_SLOTS=4 cap is the design's answer, but no test asserts the retained-shape allocation rate.
- testkit auditor: the counting allocator's e2e proof lives at `crates/testkit/tests/alloc_count_e2e.rs` (referenced alloc_count.rs:281) — verify it pins realloc-as-event, else the quantum test could miss a `Vec` growth.
