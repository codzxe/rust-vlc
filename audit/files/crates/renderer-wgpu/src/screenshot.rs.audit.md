# AUDIT: crates/renderer-wgpu/src/screenshot.rs
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-21 |
| Lines / functions | 245 / 4 functions + 5 tests |
| File verdict | REAL (encoder verified against the PNG/zlib spec by trace; unit tests pass; but the lane's committed-hash acceptance is currently RED — F-001) |
| Reachable from shipped entry point? | YES. `apps/cli/src/pipeline_host.rs:918` `screenshot::encode_rgba(extent, &rgba).map_err(\|err\| err.to_engine_error())` — the CLI screenshot lane. The first-vertical evidence run (`run-to-end` → `final-frame.png`) consumes it end-to-end. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `sed -n` full read + quote verification (:81-86, :160-165) · spec cross-check by trace (PNG signature/IHDR field semantics, zlib 0x7801 header validity `30721 % 31 == 0`, stored-block LEN/NLEN one's-complement framing, Adler-32 BE placement, CRC-32 reflected poly) · `rg -n 'final_frame_is_current' -A 25 tests/integration/first-vertical/tests/evidence.rs` (:153-175 — the consuming test) · `rg -n 'final-frame' orchrun…log` (:2502 assertion text, :6587 orchestrator failing-test list) · baseline renderer_wgpu 60/60 unit pass (orchrun :4767-4781) |

## 1. One-paragraph verdict
A correct, dependency-free PNG writer: I verified every layer against the formats — signature, IHDR (8-bit RGBA, filter 0, no interlace), sRGB chunk, a zlib stream (valid 0x78/0x01 header, checksum-trick verified) of stored deflate blocks with correct LEN/NLEN one's-complement framing and a final-block bit, Adler-32 and CRC-32 implementations pinned by their classic test vectors, and big-endian chunk length/checksum placement; the block-splitting test's arithmetic (4 blocks for 200,050 raw bytes, IDAT length exactly 2+20+200,050+4) checks out by hand. The unit suite is honest (known vectors, byte-stability, typed mismatch errors, block splitting). Two findings: the committed-hash acceptance story this module exists for is currently failing — the first-vertical `final_frame_is_current` test asserts a fresh run's PNG equals the committed `final-frame.png` and the no-fail-fast baseline shows it DRIFTED (one of the 6 failures the fail-fast Wave-0 baseline hid) — so the end-to-end lane is red, not proven, with the drift unattributed (upstream pixels or stale fixture, not provably the encoder); and the `raw.is_empty()` branch is unreachable dead code given the zero-extent guard.

## 2. What actually works (verified)
— PNG/zlib format correctness: traced every byte layer (see Commands run); the zlib header 0x7801 satisfies the `% 31 == 0` check; stored blocks ≤ 65,535 bytes with correct BFINAL ordering; Adler-32 over the *uncompressed* raw scanline stream, big-endian — RFC 1950 shape.
— CRC-32 (reflected 0xEDB88320, init/final 0xFFFFFFFF) and Adler-32 (mod 65,521) pinned by external vectors in `known_crc32_and_adler32_vectors` (:141-143: 0xCBF43926 for "123456789", 0x11E60398 for "Wikipedia") — passed in baseline (renderer_wgpu 60/60, orchrun :4767-4781).
— `large_streams_split_into_stored_blocks` (:230-244): hand-verified — raw = 50×(1+4000) = 200,050; blocks 65535×3 + 3445 = 4; IDAT = 2 + 4×5 + 200,050 + 4 = 200,076; the length field's offset 46 (8 sig + 25 IHDR + 13 sRGB) is exact.
— Typed refusals: zero extent → `ZeroExtent { from_frame: false }` (surface semantics — correct for a readback extent); short buffer → `EncodeSizeMismatch` — and note `found > expected` is accepted (extra bytes ignored), so the error's `Truncated` fold in error.rs is direction-correct on this leg.
— Byte-stability: two calls, identical output (:176-195) — the stored-block design really is heuristic-free.

## 3. Findings

### F-001 [S3]
Where: crates/renderer-wgpu/src/screenshot.rs:5-8 (claim) / tests/integration/first-vertical/tests/evidence.rs:160-165 (consumer)
Code (from the consuming test, verbatim):
```
    let committed = std::fs::read(ev::final_frame_path())
        .unwrap_or_else(|error| unreachable!("read final-frame.png: {error}"));
    assert_eq!(
        png, committed,
        "the committed final-frame.png drifted from a fresh run"
    );
```
What is wrong: The module's stated purpose — "byte-stable by construction… exactly what a committed-hash acceptance test needs" (:5-8) — is contradicted by the committed evidence: `final_frame_is_current` FAILED in the no-fail-fast workspace run (orchrun log :2502 `assertion 'left == right' failed: the committed final-frame.png drifted from a fresh run`; :6587 lists it as hidden-failure #3 of the fail-fast baseline). A fresh `run-to-end` through the shipped CLI produces a PNG that no longer equals the committed `final-frame.png`.
Why it matters: Either the renderer's pixel output changed since the fixture was committed (compositor drift — the screenshot lane faithfully records it, which is the test working as designed), or the fixture is simply stale. From this file's seat the cause is unattributed, but the module's acceptance story is currently RED: the one committed-hash proof that exercises `encode_rgba` end-to-end does not pass. The encoder's own determinism test passes, so this is not (necessarily) an encoder defect — it is a broken evidence chain on the lane this module was built to prove.
What correct looks like: Regenerate the fixture via the suite's snapshot writer (:177 "Regenerate every committed artifact from a fresh run") with an owner-attributed commit naming what changed upstream, or bisect the drift to the compositor change and fix the regression. Until then, "byte-stable by construction" describes the encoder, not the proof.
Confidence: HIGH (the failure is committed evidence) / drift cause UNVERIFIED — Verification: read + orchrun baseline cite.

### F-002 [S4]
Where: crates/renderer-wgpu/src/screenshot.rs:81-86
Code:
```
    if raw.is_empty() {
        // Degenerate but legal: one empty final stored block.
        idat.push(1);
        idat.extend_from_slice(&0u16.to_le_bytes());
        idat.extend_from_slice(&(!0u16).to_le_bytes());
    }
```
What is wrong: The branch is unreachable. `encode_rgba` rejects `width == 0 || height == 0` at :29-34, so `width, height ≥ 1`, `stride = width*4 ≥ 4`, and `raw_len = (stride + 1) * height ≥ 5` — `raw` can never be empty; the while-loop at :72-80 always executes at least once and always emits a final block.
Why it matters: Dead defensive code that can never run implies a case that can occur — a reader will hunt for the degenerate input that reaches it and find none. It also silently encodes a false belief (that a legal call can produce zero scanlines).
What correct looks like: Delete the branch, or replace the comment with "unreachable: the zero-extent guard at :29 makes raw non-empty; kept only if the guard ever relaxes" — an honest unreachable marker (`debug_assert!(!raw.is_empty())`) would do both.
Confidence: HIGH — Verification: read (guard at :29-34 + raw_len arithmetic).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `encode_rgba` | 28-92 | REAL | — | Format-correct by trace; checked-mul size with u64::MAX saturate (:35-38 — w*h*4 can't overflow u64 for u32 dims anyway); capacity hint under-estimates for extreme aspect ratios (filter-byte overhead up to 25% at width 1) — a realloc hint, not a correctness issue. |
| `push_chunk` | 95-103 | REAL | — | Length BE + type + data + CRC(type+data) BE — correct PNG framing; one small Vec per chunk (non-hot path, fine). |
| `crc32` | 106-119 | REAL | — | Reflected bitwise implementation; vector-pinned; on-the-fly table rationale documented. |
| `adler32` | 122-131 | REAL | — | mod-65,521, vector-pinned; (b<<16)\|a correct. |
| tests (5) | 133-245 | REAL | — | All would fail under the mutations they target (see §5); all pass in baseline. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `known_crc32_and_adler32_vectors` (:139) | YES | External check values (0xCBF43926 / 0x11E60398) — kills any checksum regression. |
| `encoded_stream_has_the_png_shape` (:150) | YES | Byte-offset assertions on signature/IHDR/sRGB/IEND — kills framing bugs. |
| `encoding_is_byte_stable_across_calls` (:176) | YES | Two-run equality — kills nondeterminism. |
| `size_mismatches_are_typed_errors` (:198) | YES | Exact typed variants asserted (found<expected and zero-extent). |
| `large_streams_split_into_stored_blocks` (:230) | YES | Exact IDAT length — kills block-splitting/final-bit bugs. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "byte-stable by construction… exactly what a committed-hash acceptance test needs" | screenshot.rs:5-8 | REAL (encoder) / **RED (proof)** | Encoder determinism test passes; but the consuming committed-hash test `final_frame_is_current` FAILS in the no-fail-fast baseline (F-001, orchrun :2502/:6587). |
| "~140 lines, no dependencies" | screenshot.rs:13-14 | REAL | 245 lines total including 112 lines of tests; implementation ≈131 lines — honest. |
| "sRGB (perceptual intent) matching the surface semantics" | screenshot.rs:9-10 | REAL | sRGB chunk intent 0 emitted (:57). |

## 7. Dead / duplicated / unreachable code
F-002: the `raw.is_empty()` branch (:81-86) is unreachable. Nothing else; the module deliberately mirrors xtask's std-only JPEG encoder design (documented cross-reference, not duplication of code).

## 8. Dependencies & flags
`crate::error`, `crate::params::Extent2D` only — genuinely std-only. No feature gates, no unsafe. Consumed by apps/cli (pipeline_host.rs:918) and exercised by the first-vertical evidence lane.

## 9. Open questions for the next agent
- Whoever owns the final-frame drift (first-vertical evidence agent / compositor owner): F-001's root cause — did the compositor's pixel output change, or is the fixture stale? The regen writer is at evidence.rs:177+.
- `renderer.rs`/`staging.rs` owners: the readback that feeds `encode_rgba` — confirm the RGBA byte order (PNG is RGBA; a BGRA surface readback would produce silently color-swapped screenshots that CRC-identical goldens would never catch — only visual inspection or an independent decoder would).
