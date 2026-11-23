# AUDIT: crates/codecs/src/video/dnx/decoder.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| Lines / functions | 1067 / 16 prod + 14 test helpers/tests |
| File verdict | UNREACHABLE (real, working code; no shipped demux lane can mint its descriptor) |
| Reachable from shipped entry point? | NO — wired in apps/cli/src/pipeline_host.rs:326 and apps/web/src/host.rs:674, but zero container lanes mint `dnxhd` |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | rg filmcraft/dnxhd/demux wiring (exit 0); sed/od quote verification; baseline cite audit/runs/baseline/cargo-test-workspace.txt:632-643,1032-1044 |

## 1. One-paragraph verdict
This is a REAL DNxHD/DNxHR decoder wrapper, not parse-only. The engine `filmcraft-dnx =0.9.0` (crates.io, verified on disk at ~/.cargo/registry/src/*/filmcraft-dnx-0.9.0, 4376 LOC) implements full VC-3 entropy decode (VLC tables in tables.rs), dequantization, f32 IDCT (dct.rs `idct8x8`), macroblock color conversion — `decode_frame_with` walks scan lines, decodes macroblocks, and returns full planar u16 planes. The wrapper adds a CBR truncation gate, plane-geometry validation, lossless YUYV/I420 repack, typed refusals for everything the 8-bit vocabulary cannot express, and a pooled-slot allocator. Baseline run proves it decodes real ffmpeg-generated DNxHD bit-exact-ish (6/6 differential tests, audit/runs/baseline/cargo-test-workspace.txt:1032-1044: lanes L1/L2/L3/L4 + 1080p10bit, tolerance ≤2 LSB vs ffmpeg 7.1.5). The killer: NOTHING in the shipped product can reach it — no demuxer in crates/demux emits the `dnxhd` codec descriptor, and the CLI wiring comment admits it.

## 2. What actually works (verified)
- Engine is real decode, not parse-only: read ~/.cargo/registry/src/index.crates.io-*/filmcraft-dnx-0.9.0/src/decode.rs (decode_unit → decode_row → VLC + idct8x8) -> real macroblock pipeline confirmed.
- Baseline: audit/runs/baseline/cargo-test-workspace.txt:632-643 (11 unit tests ok) and :1032-1044 (`dnx_differential`: lane_l1_1080p_175m, lane_l2_1080p_36m, lane_l3_1080p_145m, lane_l4_720p_90m, lane_l5_1080p10_185m, truncated_real_frame_refuses_typed — 6 passed, 6.08s) — a real ffmpeg cross-differential through this exact `DnxDecoder`.

## 3. Findings
F-001 [S2]
Where: apps/cli/src/pipeline_host.rs:320-326 (wiring), crates/demux (absence)
Code:
```
            // raw VC-3 ES decodes statelessly. No container lane
            // mints the `dnxhd` descriptor yet (the mkv V_MS/FOURCC
```
What is wrong: The only shipped entry into `DnxDecoder` is the CLI/web `"dnxhd"` match arm, but `rg -n "dnxhd|avdn|AVdn" crates/demux/src/` returns ZERO matches — no MP4/MKV/AVI/MXF/ASF/TS demuxer produces the `dnxhd` descriptor, and apps/cli/src/containers/es.rs hosts only mpeg-audio/annexb-h264-hevc/flac raw-ES lanes. The decoder is compiled in (codec-dnx is in codecs' `default` features, crates/codecs/Cargo.toml:51) but dead weight in every shipped binary. Why it matters: apps/web/src/capabilities.rs:220 advertises `"dnxhd"` in `decoder_descriptors` — a capability snapshot claim no file-open path can exercise; users cannot play a single DNxHD file. What correct looks like: either an MXF/MP4(AVdn)/MKV lane that mints the descriptor, or dnxhd removed from the advertised capability set. Confidence: HIGH — Verification: ran rg over crates/demux and apps/.

F-002 [S3]
Where: crates/codecs/src/video/dnx/decoder.rs:412-421
Code:
```
    if picture.bit_depth != 8 {
        return Err(unsupported(
            "codecs.dnx.output.bit_depth",
```
What is wrong: the `Decoder` trait path refuses every 10/12-bit stream — the dominant professional DNxHD/DNxHR class (the suite's own lane l5 is 10-bit; DNxHR HQX is 10-bit 4:2:2). The full-depth `decode_picture` seam exists and the refusal is typed and documented (module doc table, lines 59-67), so this is an honest capability gap, not silent corruption — but the trait path emits no frame for the format's most common profile. Confidence: HIGH — Verification: read + baseline lane_l5 result.

F-003 [S4]
Where: crates/codecs/src/video/dnx/decoder.rs:1038-1065
Code:
```
        // A same-size reuse takes the rotation, not a new allocation
        // (observable as a differing fill on the recycled slot).
        let small2 = synthetic_422(8, 2);
        let c = packed_slot(&mut slots, &mut next, &small2, len_small);
        assert_eq!(c.len(), len_small);
```
What is wrong: the test named `pooled_slots_reuse_exact_sizes_without_stale_bytes` never proves reuse: it asserts only `c.len()`. The comment promises "observable as a differing fill on the recycled slot" but no such assertion exists. Additionally `assert_ne!(a.as_slice(), b.as_slice())` (line 1060) compares a 32-byte vs a 64-byte buffer — trivially unequal, proves nothing about independence of fills. Would it fail if recycling were broken (always fresh alloc)? No. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/codecs/src/video/dnx/decoder.rs:554-555
Code:
```
        // 4:4:4 never reaches packing (typed refusal first).
        DnxChroma::Yuv444 => {}
```
What is wrong: silent no-op arm. The invariant holds today (packed_output refuses 4:4:4 at line 479), but if that gate ever regressed, pack_into would hand the sink an all-zero frame with no error. A `debug_assert!` or unreachable-but-typed arm is the honest shape. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DnxChroma::chroma_width/height | 120-138 | REAL | — | div_ceil correct for 420/422/444 |
| DnxPicture::chroma_width/height | 195-207 | REAL | — | delegation |
| DnxDecoder::new / accepts / decoded_pictures | 258-277 | REAL | — | accepts only "dnxhd" |
| DnxDecoder::decode_picture | 279-301 | REAL | — | gate + engine + counter (saturating) |
| decode_picture_ungated | 306-367 | REAL | — | CBR gate before engine; units math matches engine field_coded logic; u64 widening correct |
| color_description | 378-398 | REAL | — | conservative bt709 fill; BT.2020CL typed refusal |
| packed_output | 411-485 | REAL | S3 | see F-002; checked_mul on packed len; odd-422 width refused |
| limit_error | 490-496 | REAL | — | defensive, unreachable on ≤16384 rasters |
| validate_planes | 501-522 | REAL | — | exact geometry equality, typed corrupt on mismatch |
| pack_into | 527-557 | REAL | S4 | YUYV macropixel order verified by test; Yuv444 dead arm (F-004) |
| packed_slot | 563-593 | REAL | — | rotation bounded at FRAME_SLOTS=4; eviction safe (SharedBytes refcount) |
| Decoder::configure | 596-614 | REAL | — | resets slots/counters; extra_data deliberately ignored (documented, VC-3 self-describing) |
| Decoder::decode | 625-650 | REAL | — | stale-gen drop first; typed errors out |
| Decoder::flush | 652-659 | REAL | — | Drained (intra-only, stateless) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| configure_refuses_foreign_descriptors | YES | asserts code + kind |
| decode_and_flush_before_configure_are_typed_bad_state | YES | asserts "codecs.dnx.not_configured" (mapping verified in error.rs:93-99) |
| stale_packets_are_dropped_on_sight | YES | asserts NeedMoreInput + counter 0 |
| flush_after_configure_is_immediately_drained | weakly | only happy path |
| cbr_gate_refuses_short_packets_with_exact_numbers | YES | engine writer round-trip pinned; exact 917504/640 numbers |
| yuyv_repack_is_byte_exact | YES | hand-computed 32-byte golden |
| i420_repack_handles_odd_rasters | YES | 5×3 odd raster, exact segments |
| inexpressible_outputs_are_typed_refusals | YES | 7 cases each asserting exact code |
| plane_geometry_mismatch_surfaces_as_corrupt_data | YES | popped Cb sample |
| color_volumes_map_conservatively | YES | enum equality vs ColorDescription::BT709_LIMITED |
| pooled_slots_reuse_exact_sizes_without_stale_bytes | NO | see F-003 — len-only assertions |
Note: no test in THIS file decodes a real VC-3 frame through the happy path; that proof lives in tests/dnx_differential.rs (baseline: 6/6 ok, ffmpeg-cross-checked).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "engine admission ... rayon-free, threads: false" | decoder.rs:8-22 | REAL | Cargo.toml:158 `default-features = false`; engine decode.rs has cfg(not(feature="threads")) fallback |
| "CBR truncation gate ... engine never sees the short buffer" | decoder.rs:34-49 | REAL | gate at 309-340 precedes decode_frame_with at 341; baseline truncated_real_frame_refuses_typed ok |
| "YUYV ... byte-identical samples" | decoder.rs:61 | REAL | yuyv_repack_is_byte_exact golden |
| "No container lane mints the `dnxhd` descriptor yet" | apps/cli/src/pipeline_host.rs:320-322 | TRUE (honest admission) | rg over crates/demux: zero matches |
| web capabilities advertise dnxhd as landed | apps/web/src/capabilities.rs:220 | MISLEADING | decoder real but unreachable by any openable file (F-001) |

## 7. Dead / duplicated / unreachable code
The entire decoder is compiled into shipped binaries but unreachable at runtime (F-001). No intra-file dead code found.

## 8. Dependencies & flags
`codec-dnx` in codecs default features (Cargo.toml:51,67,158) → compiled in every default consumer (playback-engine → apps/cli). Engine filmcraft-dnx =0.9.0 exact pin, default-features off. No rayon added.

## 9. Open questions for the next agent
- Does any planned MXF host (comment points at "mkv V_MS/FOURCC and ASF futures") exist in a branch? If not, the honest capability snapshot should drop dnxhd.
- dnx_differential.rs + tests/dnx/mod.rs are separate files (likely another agent's) — their ffmpeg regeneration requires ffmpeg on PATH; baseline shows them passing, so regeneration ran.
