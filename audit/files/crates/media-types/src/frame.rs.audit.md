# AUDIT: crates/media-types/src/frame.rs
| Field | Value |
|---|---|
| Auditor | W02-A06 |
| Date | 2026-11-27 |
| Lines / functions | 370 / 5 prod fns/methods + 15 types + 5 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen` bin -> lumen_cli (apps/cli/Cargo.toml deps media-types) -> containers/{wav,ogg,mp4,ts,adts}.rs use `AudioFormat::frames_in`; codecs (mjpeg/mod.rs:386, av1/decoder.rs:604, ...) construct `FrameStorage::Cpu` VideoFrames; renderer-wgpu consumes them |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `cargo test -p media-types --lib frame` -> 5 passed, EXIT=0; baseline cargo-test-workspace.txt media-types section -> 76 passed / 1 ignored |

## 1. One-paragraph verdict
A clean, real vocabulary module: decoded audio/video/subtitle frame types with sample-format byte math, minimal colorimetry, a zero-copy `SharedBytes` CPU storage variant and a platform-neutral `OpaqueHandle(u64)` hardware escape hatch. `AudioFormat::frames_in` is load-bearing in the shipped CLI (muxing containers compute sample counts through it), and the math is correct: `frame_bytes = channels × bytes_per_sample`, `frames_in` returns `None` on a partial frame instead of rounding — the honest choice, pinned by its own test. No panics, no unwraps, no stubs. What is missing: no invariant tying `VideoFrame.size`/`format` to `data.len()` (consumers must defend, and the ones I checked do), no shared plane-size helper for the pixel formats (NV12 `w*h*3/2` math is re-derived in tests and consumers), and two trivial test pins. This is the type layer the RFC-6455 frame file is not — different "frame" concept, no wire parsing here by design (doc 01 §2.2: no format-specific parsing in media-types).

## 2. What actually works (verified)
- verified by `cargo test -p media-types --lib frame` -> `test result: ok. 5 passed; 0 failed` EXIT=0.
- verified by `rg frames_in` -> production call sites in apps/cli/src/containers/{wav,ogg,mp4,ts,adts}.rs (L161-166 wav, L265 ogg, L466 mp4, L576 ts, L191 adts) — the type is not dead vocabulary.
- Baseline cross-check: cargo-test-workspace.txt media-types: 76 passed / 1 ignored, exit_code=0.

## 3. Findings
F-001 [S4]
Where: frame.rs:209-226
Code:
```
pub struct VideoFrame {
    /// Pixel format.
    pub format: PixelFormat,
    /// `(width, height)` in pixels.
    pub size: (u32, u32),
    /// CPU memory or an opaque hardware handle.
    pub data: FrameStorage,
```
What is wrong: nothing enforces that `data` is `w×h`-sized for `format` — a `VideoFrame { size: (64,64), format: Nv12, data: Cpu(5 bytes) }` is constructible. This is a deliberate vocabulary-crate stance (no parsing/validation here, doc 01 §2.2), and the consumers I checked defend themselves (renderer-wgpu/src/surface.rs:242 bounds-checks rects, :294 checks `needed` bytes), but every new consumer must remember. Correct look: a `debug_assert`-style checked constructor or a `try_from_parts` in the codec layer.
Confidence: HIGH — Verification: read + targeted rg of consumers.

F-002 [S4]
Where: frame.rs:86-103 (PixelFormat) vs frame.rs:328 and codecs/tests/vp9_differential.rs:272
Code:
```
            data: FrameStorage::Cpu(SharedBytes::from_vec(vec![0; 64 * 64 * 3 / 2])),
```
What is wrong: no plane/size math on `PixelFormat` in the vocabulary, so NV12 buffer size is re-derived ad hoc (unit test here, vp9/av1/h264 differential tests, decoders). Divergent copies of `w*h*3/2` are a standing off-by-one risk when a format is added. Correct look: `PixelFormat::bytes_needed(w, h) -> Option<u64>` next to the enum.
Confidence: HIGH — Verification: read + rg `* 3 / 2`.
Why S4 not S3: every current copy I found computes the same expression; no divergence found.

F-003 [S4]
Where: frame.rs:364-369 and 77-83
Code:
```
        let c = ColorDescription::BT709_LIMITED;
        assert_eq!(c.primaries, ColorPrimaries::Bt709);
        assert_eq!(c.range, ColorRange::Limited);
```
What is wrong: the default-const test pins 2 of 4 fields (transfer, matrix unasserted); `frames_in`'s `frame == 0` guard (channels=0 format) is untested. Trivial pins.
Confidence: HIGH — Verification: read.

## 4. Per-function table (production code)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SampleFormat::bytes_per_sample | 32-40 | REAL | — | 1/2/3/4/8 table, pinned by test |
| AudioFormat::frame_bytes | 70-72 | REAL | — | channels × bps, u64 math — no overflow for real inputs |
| AudioFormat::frames_in | 77-83 | REAL | S4(F-003) | zero-guard + strict multiple; None on partial (honest); used by 5 CLI containers |
| ColorDescription::BT709_LIMITED | 161-166 | REAL | S4(F-003) | const, correct values (709/709/709/limited) |
| Frame::pts / generation / is_stale | 249-269 | REAL | — | total match over 3 kinds, tested |

Types (all REAL, plain data): SampleFormat, AudioLayout, AudioFormat, PixelFormat (Nv12/I420/Yuyv422/Rgba8/Bgra8), ColorPrimaries, TransferCharacteristics (Pq/Hlg/Srgb present), ColorMatrix (Bt2020Ncl present), ColorRange, ColorDescription, FrameStorage, SubtitleCue, AudioFrame, VideoFrame, SubtitleFrame, Frame.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| audio_format_frame_math | YES | 4-byte frame, 1000 frames, partial→None; planar F32 2ch = 8 |
| sample_format_byte_widths | YES | pins U8/S24/F64 widths |
| frame_accessors_cover_all_kinds | YES | all three Frame kinds with real Stamp/GenerationId; is_stale both directions |
| opaque_handle_names_no_platform | WEAK | string-contains checks on the Debug output of a variant literally named `OpaqueHandle` — pins the naming contract only |
| color_description_default_is_bt709_limited | WEAK | 2 of 4 fields (F-003) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Depth and signedness are explicit — no 'whatever the decoder felt like'" | frame.rs:11-13 | REAL | enum with 6 explicit formats |
| "partial frame ... is corruption at the decode boundary — data, not a guess" | frame.rs:74-77 | REAL | frames_in returns None; wav.rs:161 cites "the honest count" |
| "hardware-frame escape hatch that names no platform" | frame.rs:2-3, 171-175 | REAL | OpaqueHandle(u64); consumers never dereference (renderer test asserts no platform words) |
| "Std-only by design" (crate) | media-types/Cargo.toml:4 | REAL | Cargo.toml has no [dependencies] (see Cargo.toml audit) |
| "no sentinel numbers (MP §8)" (crate) | media-types/Cargo.toml:10 | REAL | no magic values in this file; TimeSpan::UNKNOWN in the test comes from time.rs (other file's scope) |

## 7. Dead / duplicated / unreachable code
None dead. Duplication: NV12 size math re-derived in tests/consumers (F-002). `AudioLayout` affects no math here (frame_bytes is layout-independent — correct for planar too since total bytes = channels×samples×bps either way).

## 8. Dependencies & flags
Std-only + crate-internal (ids::GenerationId, packet::SharedBytes, time::{Duration, Stamp, TimeSpan}). No features, no cfg gates. Zero external deps — consistent with the crate's std-only claim.

## 9. Open questions for the next agent
- Where is `AudioLayout::Planar` actually produced? All decoder call sites I saw emit interleaved PCM ( pcm/mjpeg first vertical). If planar is produced nowhere yet, the layout field is a forward-looking extension (fine, but check for dead-planar handling in the mixer).
- The rate==0 defense lives in consumers (pcm/decoder.rs:118/417/626, audio-output/null.rs:80) — a `Rate` newtype or a checked constructor would centralize it.
- Confirm renderer-wgpu rejects (not panics on) a size/data-length-mismatched VideoFrame end-to-end (F-001); surface.rs looked defensive but the upload path wasn't fully traced.
