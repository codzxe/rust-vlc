# AUDIT: apps/desktop/tests/zero_alloc_transport.rs
| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-23 |
| Lines / functions | 141 / 4 (2 tests + FixedClock + consts; note: assignments.tsv spells it `zeroalloc_transport.rs`, the real path is `zero_alloc_transport.rs`) |
| File verdict | REAL (genuine counting-allocator proof, both tests pass in baseline) — but the headline claim "the desktop frame path allocates zero bytes" is true only for the SEND half; the production drain half allocates per frame and is outside the measured window |
| Reachable from shipped entry point? | N/A (integration test) — runs via `cargo test -p desktop`; executed in baseline cargo-test-workspace.txt:2616-2620 (both `ok`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg (multiple, exit 0 — note: one rg invocation used a stray `-r` replace flag, mangling DISPLAYED names only; all file/path facts re-verified with plain rg); sed -n quotes (exit 0); baseline citations |

## 1. One-paragraph verdict
This is a real allocation proof, not a tautology: a `#[global_allocator]` counting allocator, whole-suite serialization via `serialize_tests()`, a warm-up cycle that sizes the scratch/pool/queue, then 256 measured cycles ending in `guard.assert_zero(...)` — any steady-state allocation on the measured path fails the test, and per-cycle content asserts (advancing `pts_ns`, incrementing `frame_id`) kill memoization shortcuts. The hostile catch is scope: the measured window is `present -> serialize -> send -> decode_record -> offer -> draw` — it stops at the mailbox. The path the shipped GUI ticker would actually run continues with `FrameTap::drain_records` -> `encode_record`, which allocates a fresh `Vec::with_capacity(payload_len + MIN_HEADER_LEN)` per drained record (frame_tap.rs:126) plus the `Vec<Vec<u8>>` push — so the module doc's opening claim "Zero-allocation proof for the desktop frame path" proves half the frame path. The wire.rs audit (F-003, prior agent) flagged the same gap from the other side; this test is the evidence string that half-covers it.

## 2. What actually works (verified)
— verified by audit/runs/baseline/cargo-test-workspace.txt:2616-2620 -> `Running tests/zero_alloc_transport.rs` then `test streaming_decoder_path_is_zero_allocation_after_warm_up ... ok` and `test mailbox_frame_path_is_zero_allocation_after_warm_up ... ok`.
— verified by read: `assert_eq!(frame.header.frame_id, i as u64 + 1)` (:93) is correct arithmetic (warm-up consumed id 0) and catches id-freeze; pts varies per cycle (:86-87) so identical-output caching cannot pass test 1; warm-up cycle (:80-81) legitimately sizes bridge scratch, decoder pool, mailbox queue before the guard.

## 3. Findings
F-001 [S3]
Where: apps/desktop/tests/zero_alloc_transport.rs:1-5 and :84-101
Code:
```
//! Zero-allocation proof for the desktop frame path (audit row C7,
//! ED-FIX-2 completion — W2-03): after one warm-up cycle, the
//! per-frame transport path allocates **zero bytes**, measured by the
//! counting global allocator (the `zero_alloc_null_sink.rs` /
//! `zero_alloc_quantum.rs` discipline).
```
```
    let guard = ALLOC.state().guard();
    for i in 0..CYCLES as i64 {
        let pts = i * FRAME_30;
        request.when = media_types::time::Timestamp::new(pts, video_engine::NANO_BASE);
        bridge.present(&request).must();
        let frame = bridge.transport_mut().mailbox_mut().draw_next().must();
```
What is wrong: "the desktop frame path" ends at `draw_next` inside the measured window; the production consumer continuation — `FrameTap::drain_records` (frame_tap.rs:72-89) -> `encode_record` (frame_tap.rs:125-134, `Vec::with_capacity(frame.header.payload_len as usize + MIN_HEADER_LEN)` per record + `out.push(record)`) — allocates per frame and is never measured. A reader of row C7 / this doc concludes the whole per-frame desktop path is allocation-free; the drain half is not. Why it matters: the drain runs once per pacer tick in the real shell — it is the other half of the hot path; the evidence string over-covers the claim (spec 6.4: evidence that does not prove the row's claim). What correct looks like: extend the measured loop with `tap.drain_records(&mut out)` and either prove the re-encode zero-alloc too (it cannot be, as written) or narrow the claim to "send path". Confidence: HIGH — Verification: read of this file + frame_tap.rs:125-134; wire.rs audit F-003 corroborates.
F-002 [S4]
Where: apps/desktop/tests/zero_alloc_transport.rs:132-139
Code:
```
    let guard = ALLOC.state().guard();
    for _ in 0..CYCLES {
        decoder.feed(&record);
        let frame = decoder.next_frame().must().must();
        assert_eq!(frame.header.frame_id, 7);
```
What is wrong: test 2 feeds the byte-identical record 256 times and asserts the constant `frame_id == 7` — the anti-memoization rationale declared at :53-54 ("varies the records so nothing can 'pass' by memoizing identical inputs") is honored by test 1 but not here; a decoder that cached the first decode by record content would pass test 2. The alloc assertion is unaffected (caching would itself likely allocate), so blast radius is small. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FixedClock (+impl) | 61-67 | REAL | — | deterministic stamps, no clock in window |
| mailbox_frame_path_is_zero_allocation_after_warm_up | 70-102 | REAL | S3 | genuine assert_zero over 256 cycles; window excludes the drain half (F-001) |
| streaming_decoder_path_is_zero_allocation_after_warm_up | 105-141 | REAL | S4 | assert_zero genuine; identical-input weakness (F-002) |
| (consts EXTENT/FRAME_30/CYCLES) | 48-57 | REAL | — | 64x48, 30 fps spacing, 256 cycles |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mailbox_frame_path_is_zero_allocation_after_warm_up | YES | any steady-state alloc on the measured path trips assert_zero; pts/frame_id content asserts catch wrong-record returns |
| streaming_decoder_path_is_zero_allocation_after_warm_up | YES (allocs) / NO (memoization) | assert_zero genuine; identical record + constant frame_id cannot catch a content-caching decoder |
| GAP | — | No measured window includes drain_records/encode_record (F-001); repeat path (present_repeat) unmeasured (same alloc shape as present — low risk). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "after one warm-up cycle, the per-frame transport path allocates **zero bytes**" | zero_alloc_transport.rs:2-3 | REAL (for the measured transport window) | baseline :2619-2620 both ok |
| "Zero-allocation proof for the desktop frame path" | zero_alloc_transport.rs:1 | SHALLOW (half the path) | drain half allocates per record (frame_tap.rs:126, F-001) |
| "the old path built a fresh `WireDecoder` per `send` and paid a full-frame `to_vec` + an O(n) `drain` per record — 2 allocs + 2 copies" | zero_alloc_transport.rs:18-21 | REAL-UNPROVEN (history claim) | the fix (one decoder, transport.rs:184-231) is verifiable; the "old path" numbers are unverifiable history — no git access (rulebook §2) |

## 7. Dead / duplicated / unreachable code
None. (Note: assignments.tsv names this file `zeroalloc_transport.rs`; the on-disk name is `zero_alloc_transport.rs` — the orchestrator's manifest should record the corrected path.)

## 8. Dependencies & flags
desktop (bridge/transport/wire/mailbox), renderer_wgpu::params::Extent2D (extent only — no GPU), testkit (Must + alloc_count + serialize_tests), video_engine (Presenter/NANO_BASE), media-types. Dev-deps only; no features.

## 9. Open questions for the next agent
1. If row C7 of the audit/matrix ledger claims "zero-alloc desktop frame path", downgrade it to "zero-alloc send path" or extend this test to cover drain_records. 2. testkit::alloc_count::CountingGlobalAlloc is load-bearing for every zero-alloc claim in the repo — its own audit should confirm the guard window counts all threads (serialize_tests implies process-global serialization).
