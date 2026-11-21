# AUDIT: crates/codecs/src/video/mjpeg/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2026-11-21 |
| Lines / functions | 436 / 12 (incl. 2 tests, 1 doctest) |
| File verdict | REAL (implemented, reachable, executed in baseline end-to-end) |
| Reachable from shipped entry point? | YES — `apps/cli/src/pipeline_host.rs:243-247` `"mjpeg" => { let mut decoder = codecs::video::mjpeg::MjpegDecoder::new(); decoder.configure(config)?; … }`. Executed: baseline AVI first-vertical decode-bench `v_frames=60 v_decoded=60` (INDEX.md), rc-smoke 6/6 engine checks, 98 mjpeg-related test mentions incl. contract-suite `per_frame_damage_leaves_the_decoder_usable` (test-workspace.log.txt:917). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read full file; baseline log greps (`video::mjpeg::tests::` :740-741, `video::mjpeg::decoder::tests::` :676-680+, `per_frame_damage…` :917, mjpeg_field_differential :1118); cross-ref pipeline_host dispatch (already quoted in log). |

## 1. One-paragraph verdict
The one lane in this wave that genuinely ships and works: the MJPEG `Decoder` impl is stateless per packet exactly as documented, its damage model (typed header errors, concealed entropy damage, per-frame isolation) is pinned by the contract suite that ran in baseline, and the two-phase pooled-slot decode is a real allocation optimization with the old path's cost documented. Two honest-but-real gaps: every frame is tagged a constant JFIF colorimetry regardless of what the stream carries (Adobe APP14 mislabels), and a dead SIMD stub module rides along behind an `#[allow(unused)]`.

## 2. What actually works (verified)
— `Decoder::decode` end-to-end in baseline: AVI MJPEG+PCM fixture `v_frames=60 v_decoded=60` (INDEX.md, decode-bench); decoder.rs tests `concealment_does_not_leak_across_frames`, `context_reuse_is_bit_exact_and_reallocates_only_on_change`, `avi1_interleaved_frame_is_metadata_only_bit_exact`, `avi1_single_field_decodes_at_field_height` all ok (:676-680); contract suite `per_frame_damage_leaves_the_decoder_usable` ok (:917) — the doc's central per-frame-isolation claim (:76-86) is executed proof, not prose.
— `configure`'s descriptor gate: typed `Unsupported` with the offending name in `FormatInfo` (:309-321); accepted vocabulary tested (`descriptor_gate_accepts_the_vertical_vocabulary`, :740 ok).
— `flush` returns `Drained` immediately (stateless — module doc :5 and impl :403-415 agree).
— Both module tests ran and passed for real (:740-741).

## 3. Findings
### F-001 [S3]
Where: crates/codecs/src/video/mjpeg/mod.rs:388 (+ const 214-219)
Code:
```rust
        let frame = Frame::Video(VideoFrame {
            format,
            size: (meta.width, meta.height),
            data: media_types::frame::FrameStorage::Cpu(data),
            pts: packet.pts,
            color: JFIF_COLOR,
```
What is wrong: every emitted frame carries the constant `JFIF_COLOR` (BT.601 matrix, Full range, :214-219) regardless of the stream's actual colorimetry markers. Module docs admit the scope (:28-32: "explicit color metadata (e.g. Adobe APP14) is not parsed in v0") — but the constant is applied to ALL streams, including Adobe APP14 JPEGs (conventionally BT.601 *limited*, or RGB when the APP14 transform flag is 0) and bare no-APP0 JFIFs from capture hardware.
Why it matters: downstream color management renders APP14 streams with wrong matrix/range (washed-out or clipped output); the tag is metadata the presentation layer trusts. Documented limitation caps severity, and the 4:4:4/4:2:2 conversion path (color.rs, W06-A09) bakes the same BT.601-full assumption into pixels — the two together make Adobe-family MJPEG systematically miscolored, not just mislabeled.
What correct looks like: parse APP0/APP14 presence in the parser (the marker walk already touches APP segments), map to `ColorDescription` per stream (not per frame), and refuse or tag-unknown when markers conflict.
Confidence: HIGH — Verification: read (const + call site + module-doc admission); color.rs cross-file for the conversion constant.
### F-002 [S4]
Where: crates/codecs/src/video/mjpeg/mod.rs:115-119
Code:
```rust
// (idct_simd: card 7-b stub — SIMD plan + dispatch skeleton, not yet
// wired; see that module's docs before implementing.)
pub(crate) mod idct;
#[allow(unused)]
pub(crate) mod idct_simd;
```
What is wrong: `idct_simd` is a 457-line stub module ("SIMD plan + dispatch skeleton, not yet wired" per its own gate comment) compiled into every build — the shipped `lumen` binary carries the dead code, and `#[allow(unused)]` suppresses exactly the lint that would report it.
Why it matters: dead weight plus an `#[allow]` hiding a real lint — the checklist's `#[allow]`-hiding pattern; blast radius zero at runtime (nothing calls it).
What correct looks like: cfg-gate it behind a `simd` feature until wired, or delete until card 7-b lands (git history keeps the plan).
Confidence: HIGH — Verification: read; module docs; idct_simd.rs is W06-A07's assignment for the internal audit.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module decls (`pub mod fuzz`, `pub(crate) mod …`) | 106-123 | REAL | S4 | fuzz is legitimately public (fuzz target + in-tree survival tests); idct_simd stub (F-002) |
| `decode_image` | 146-165 | REAL | — | fuzz entry; lossless mapping of decoder::decode's meta incl. all 4 conceal causes + field polarity |
| `ConcealReason` / `DecodedFormat` / `DecodedFrame` | 169-211 | REAL | — | data types; mirror decoder.rs's internal enum 1:1 |
| `JFIF_COLOR` const | 214-219 | REAL | S3 | F-001 — constant tag for variable reality |
| `MjpegDecoder::new` / `accepts` | 294-306 | REAL | — | `mjpeg`\|`jpeg`; tested |
| `Decoder::configure` | 309-325 | REAL | — | typed gate; stores generation + descriptor |
| `Decoder::decode` | 337-401 | REAL | S3 | not-configured typed; stale-generation via `drop_stale_packet`; two-phase pooled decode (context reused, slot refilled in place — the documented W6-02 fix); concealed → `Recovered{ConcealedFrame}`, clean → `Emitted{1}`; `interlaced` = AVI1 presence per docs; color tag F-001 |
| `Decoder::flush` | 403-415 | REAL | — | Drained (stateless); not-configured typed |
| tests ×2 | 422-435 | REAL | — | trivial but real; the heavy coverage lives in decoder.rs tests (:676-680+) and integration suites |
| doctest | 232-274 | REAL | — | executable: configure + empty packet → typed error; runs under `cargo test --doc` (baseline suite green) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| descriptor_gate_accepts_the_vertical_vocabulary | YES | positive/negative asserts (baseline :740 ok) |
| jfif_color_tag_is_bt601_full_range | YES (as a constant lock) | but it locks IN the F-001 behavior — the test pins the limitation, it does not expose it |
| (downstream) per_frame_damage_leaves_the_decoder_usable | YES | contract suite, baseline :917 ok — the module's core damage claim executed |
| (downstream) mjpeg_field_differential | YES | AVI1 lanes vs ffmpeg-pinned fixtures (:1118) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "stateless between packets … `flush` is immediately drained" | :4-6 | REAL | impl :403-415; contract suite |
| "4:2:0 and grayscale → I420 … 4:4:4/4:2:2/4:4:0 → Rgba8" | :16-27 | REAL | decoder.rs output-format tests (A03-A05's range; baseline ok) |
| "grayscale carries neutral (constant-128) chroma planes" | :20-23 | REAL | documented representation; decoder.rs fixtures |
| "explicit color metadata (e.g. Adobe APP14) is not parsed in v0" | :31-32 | REAL (honest) | F-001 is the cost of this admission |
| "no panic on any input (the `fuzz_codec_mjpeg` target's guarantee)" | :72-74 | REAL | fuzz target + in-tree survival tests (fuzz.rs is W06-A06's; baseline `video::mjpeg::fuzz…` suites ok) |
| AVI1 "decode as complete pictures with `interlaced` set … order-preserving" | :41-57 | REAL | `avi1_interleaved_frame_is_metadata_only_bit_exact`, `avi1_single_field_decodes_at_field_height` (:677-678 ok); ffmpeg-ignores-AVI1 claim cross-checks with the field differential |

## 7. Dead / duplicated / unreachable code
`idct_simd` stub (F-002). No duplication: `DecodedFrame` vs decoder.rs's internal `Image` is a deliberate public-contract mirror with explicit mapping (:148-164).

## 8. Dependencies & flags
`codec_api::prelude` (Decoder, DecodeOutcome, drop_stale_packet), media-types frame/packet types. No unsafe, no feature gates — unconditional in every build (including wasm `builtin-codecs` lane: apps/web/Cargo.toml:88-89 lists mjpeg as in-house decoder set). No external deps (in-house decision, module docs of codecs/src/lib.rs).

## 9. Open questions for the next agent
- W06-A09 (color.rs parts): confirm the BT.601-full fixed-point constants match the `JFIF_COLOR` tag (a mismatch there would be worse than F-001).
- W06-A04/A05 (decoder.rs parts 1-2): does `pooled_frame_slot`'s round-robin (FRAME_SLOTS=4) guard against a sink that retains more than 4 frames (overwrite-under-reader)?
- Does any demux lane mint the `jpeg` descriptor (accepted at :304) or is it dead vocabulary? (AVI/MP4 lanes use `mjpeg`; if nothing emits `jpeg`, the arm is harmless dead vocabulary.)
