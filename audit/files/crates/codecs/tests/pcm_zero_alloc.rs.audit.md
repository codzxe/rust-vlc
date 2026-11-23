# AUDIT: crates/codecs/tests/pcm_zero_alloc.rs
| Field | Value |
|---|---|
| Auditor | W06-R3 (credit W06-A08; recovery of W06-A08's unfinished item 14) |
| Date | 2026-11-23 |
| Lines / functions | 201 / 1 test + 3 helpers + 1 sink impl + 2 consts + global allocator |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (dev-lane test). The surface it pins — `codecs::audio::pcm::PcmDecoder` decode/sink handoff — IS the CLI's pcm lane (pipeline_host `codec.starts_with("pcm_")` arm) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/pcm_zero_alloc-2085804e50b18c22 --nocapture` -> `[pcm_zero_alloc] 256 buffers per shape: 0 alloc events (passthrough + converted)`, 1 passed, exit 0; fixture + digest-pin cross-checks (tests/fixtures/generated/valid/avi_mjpeg_pcm.avi 425 516 B; pcm_fixtures.rs audio_stream_sha256 pins) |

## 1. One-paragraph verdict
A measured, not argued, zero-allocation proof — the strongest test design in this batch: a counting global allocator wraps the whole process, both measurement windows contain ONLY decode + sink handoff + drop (input packets and configure are built outside), and the pin is `alloc_calls == 0` over 256 steady-state buffers per shape. I executed it from the prebuilt binary: 1 passed, zero allocation events on both shapes (s16le zero-copy passthrough where the frame is the packet's own SharedBytes Arc clone; s16be pair-swapped conversion into the decoder's bounded CONVERT_SLOTS=4 pool rotation — decoder.rs:295). Methodology is correct for a process-global allocator (single test per binary + `serialize_tests()`), warm-up protocols are documented honestly (including the admission that the passthrough is zero-alloc even cold), and the fixture's audio digest is independently pinned by the pcm_fixtures suite. One scope note: the headline "zero-allocation proof for the PCM contract path" measures exactly two shapes of one fixture geometry (stereo s16le/s16be, 5 880-byte chunks) — other conversion geometries share the pool path but are unmeasured.

## 2. What actually works (verified)
- Executed: `./target/debug/deps/pcm_zero_alloc-2085804e50b18c22 --nocapture` -> `1 passed; 0 failed` in 0.03s with the run line `[pcm_zero_alloc] 256 buffers per shape: 0 alloc events (passthrough + converted)`; exit 0. Baseline :1345-1350 agrees (1 passed, 0.03s).
- Measurement window integrity (read-verified): `packet_for` copies happen in the `.collect()` BEFORE `ALLOC.state().guard()` (:139-145 and :175-181); the guard wraps only the decode loop; configure and warm-up precede it; the final eprintln is after.
- Both pins bit in the right place: `assert_eq!(delta.alloc_calls, 0, ...)` with byte counts in the failure message (:156-161, :192-197); frame-count cross-assert `sink.frames == warm + PACKETS` guarantees 256 real handoffs per shape.
- Fixture chain: `generated/valid/avi_mjpeg_pcm.avi` exists (425 516 bytes, repo-root tests/fixtures — common/mod.rs:29-31 resolves `../../tests/fixtures`); `extract_avi_pcm_chunks` walks `01wb` chunks; `chunks.len() == 60` asserted; the audio digest is pinned by pcm_fixtures.rs (`audio_stream_sha256`, :175/:199/:366) exactly as the doc claims.
- Pool claim: `CONVERT_SLOTS = 4` rotation exists in the decoder (crates/codecs/src/audio/pcm/decoder.rs:292-295), matching the doc's "bounded `CONVERT_SLOTS` rotation".

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/pcm_zero_alloc.rs:1-6 (scope) vs :37 (measured geometry)
Code:
```
//! Zero-allocation proof for the PCM contract path (card W5-19, the
//! D-1 audit): after warm-up, each steady-state `PcmDecoder::decode`
//! through the real contract surface (configure → decode → sink
//! handoff → drop) allocates **zero bytes** — measured by the counting
//! global allocator, not argued from code reading — on BOTH decode
//! shapes:
```
What is wrong: nothing in the execution — but the headline breadth exceeds the measured scope. The proof covers exactly one fixture geometry: stereo s16le (passthrough) and its byte-swapped s16be twin (conversion), 5 880-byte chunks. The PCM decoder's other conversion geometries (s24le/s32le/f32le intake, mono, odd-length tails) ride the same pooled path but are never measured; a per-buffer allocation introduced only on, say, the f32le conversion lane would not fail this test. The doc does scope itself ("on BOTH decode shapes", naming the shapes) — this finding is the headline-vs-scope gap only. What correct looks like: either a subtitle ("measured on the corpus s16 geometry") or one extra swapped-shape lane per width. Confidence: HIGH — Verification: read + ran.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `static ALLOC: CountingGlobalAlloc` | 20-22 | REAL | — | process-global counting allocator; state().guard()/delta() verified in testkit/alloc_count.rs (:77-101) |
| `const AVI` / `const PACKETS` | 34-39 | REAL | — | fixture path resolves to repo-root tests/fixtures; 256 > any plausible periodic-growth period short of 1/256 |
| `struct DroppingSink` + `FrameSink::accept` | 44-53 | REAL | — | drops frames immediately — the slot returns to the pool INSIDE the measured window (the honest hard case) |
| `fn configured` | 55-75 | REAL | — | loud `unreachable!` on descriptor/generation/configure failures; extra_data = the container's real strf |
| `fn packet_for` | 64-88 | REAL | — | SharedBytes::from_vec copies — all outside the guard windows |
| `pcm_contract_path_is_zero_allocation_per_buffer_after_warm_up` | 90-201 | REAL (executed) | — | both shapes, warm-up + 256-buffer windows, alloc_calls==0 pins; `serialize_tests()` guard; scope note F-001 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| pcm_contract_path_is_zero_allocation_per_buffer_after_warm_up | YES — this is a MEASURED pin: any allocation event anywhere in the process during either 256-buffer window (decoder regression, sink handoff regression, pool-growth regression) increments `alloc_calls` and fails the assert with the exact count and byte total. Verified executing green with 0 events | The counting allocator counts process-wide; window discipline keeps setup allocations out; the frame-count assert proves the decodes really happened |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "each steady-state decode ... allocates zero bytes — measured by the counting global allocator, not argued from code reading" | :1-5 | REAL (measured, executed) | my run: 0 alloc events over 256 buffers × 2 shapes, exit 0 |
| "the frame IS the packet's SharedBytes, an Arc refcount bump" | :8-10 | REAL | 0 allocs on the passthrough shape is the measurement of exactly this |
| "the whole-frame prefix is group-swapped into a pooled slot — the bounded CONVERT_SLOTS rotation" | :11-14 | REAL | CONVERT_SLOTS=4 at decoder.rs:295; conversion shape measured at 0 allocs after warm-up |
| "One #[test] per binary by design (the allocator is process-global...)" | :16-18 | REAL | single test + serialize_tests() :92; testkit/alloc_count.rs:64 |
| "60 chunks of 1470-frame stereo s16le" | :34-36 | REAL | assert chunks.len()==60 :108; 1470×2ch×2B = 5 880 B/chunk; digest pinned by pcm_fixtures.rs |
| "the audio stream digest is pinned by the pcm_fixtures suite" | :34-36 | REAL | pcm_fixtures.rs:175/:199/:366 audio_stream_sha256 |

## 7. Dead / duplicated / unreachable code
None. `mod common`'s extraction helpers are the shared fixtures_parent surface (audited by W06-A08, File 7 — extraction correctness machine-checked by writer digests).

## 8. Dependencies & flags
Test-only: codec_api prelude, codecs::audio::pcm::PcmDecoder (unconditional in-tree), testkit (alloc_count — the counting allocator + serialize_tests), common extraction helpers, committed repo-root fixture. No feature gates. Runs in the normal gate (not ignored — baseline :1345).

## 9. Open questions for the next agent
- If a future card adds f32le/s24le conversion lanes to the PCM decoder, extend this proof's shape 2 family — the pool path is shared, the measurement is not (F-001).
- The `warm + PACKETS` frame-count assert uses cycling chunks (60×256) — if a slot-recycling bug only manifests across FULL pool rotations at sizes > 256 packets, this window would miss it; a 1000-packet sweep is cheap and would close that gap.
