# AUDIT: crates/media-types/src/frame.rs (full file, 370 lines)

| Field | Value |
|---|---|
| Auditor | W02-A07 |
| Date | 2026-11-21 |
| Lines / functions | 370 / 16 units + 5 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI open path → demux AVI → `crates/codecs/src/video/mjpeg/mod.rs:383` constructs `Frame::Video(VideoFrame { … })` → video-engine/packet-queue consume; audio lane: `crates/codecs/src/audio/pcm/decoder.rs` emits `AudioFrame`. Baseline executed proof: `audit/runs/baseline/mp4-smoke-bench-avi-mjpeg-pcm.txt` → `v_frames=60 v_decoded=60 a_frames=60 a_decoded=60` — 60 real `VideoFrame`s and 60 `AudioFrame`s flowed through the shipped binary. `Frame::is_stale` used in production at `crates/codec-api/src/encode.rs:114`; `FrameStorage::OpaqueHandle` consumed by h264/hevc hw paths (`crates/codecs/src/video/h264/decode.rs:466,779`).) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | `sed -n` verbatim verification (70-83, 364-369); `rg` consumers of `VideoFrame`/`AudioFrame`/`FrameStorage`/`is_stale`/`OpaqueHandle` (full consumer map below); read doc 01 §3.3 frame sketches (field-by-field match); read doc 03 §4 sketch (superseded preliminary enums); `grep -c '#[test]'` across media-types src (77 tests); baseline log forensics: `grep media_types test-workspace.log.txt` → only a `Compiling` line (:38), NO test-binary run; `grep "error: test failed"` → run #3 aborted at `-p desktop --test zero_alloc_transport` (:2411), run #4 aborted at `-p first-vertical --test evidence` (allfeatures :2500) — see F-001. |

## 1. One-paragraph verdict
Clean, small vocabulary file: pure data types plus four arithmetic/accessor methods, zero panics, zero `unsafe`, no I/O, no feature gates — exactly what its crate contract (doc 01 §2.2 "data types and pure functions only") demands. Every struct field matches the doc 01 §3.3 sketch verbatim; the audio frame math is correct for both interleaved and planar layouts and refuses partial frames as data (`None`), not rounded guesses; zero-channel formats are rejected before division. The types are proven live end-to-end by the shipped MJPEG+PCM lane (60/60 frames decoded in the baseline bench). The two findings are not in this file's logic: (1) an evidence-integrity defect — **media-types' 77 unit tests (5 in this file) were never executed in ANY baseline run**, because both baseline `cargo test --workspace` runs failed fast (run #3 stopped at `apps/desktop`, run #4 at `first-vertical` — every package sorting after those never ran), while INDEX.md presents the partial totals (1681/1751 passed) as the workspace result; (2) the BT.709 default test pins only 2 of the 4 fields it names.

## 2. What actually works (verified)
— Types executed in the shipped binary: baseline `mp4-smoke-bench-avi-mjpeg-pcm.txt` → `v_frames=60 v_decoded=60 a_frames=60 a_decoded=60`; the constructor site is `crates/codecs/src/video/mjpeg/mod.rs:383-391` (`Frame::Video(VideoFrame { format, size, data: FrameStorage::Cpu(data), pts, color: JFIF_COLOR, interlaced, generation })`) — every field of `VideoFrame` is populated by the shipped decoder.
— `bytes_per_sample` (1/2/3/4/4/8) and `frame_bytes` = channels × bytes: verified by hand for all six `SampleFormat` variants; layout-independent formula is correct for planar too (total bytes = channels × frames × bytes_per_sample in both layouts).
— `frames_in` division safety: `frame == 0` guard (zero channels → `None`, no div-by-zero); `bytes as u64` widening cannot truncate; `is_multiple_of` + exact division — partial buffers yield `None`.
— `Frame::{pts,generation,is_stale}` accessors: total over the three variants; `is_stale` uses strict `<` (matches packet-queue's "stale is strictly past" semantics, stamped.rs:142-148).
— Doc 01 §3.3 conformance: `Frame`, `AudioFrame`, `VideoFrame`, `SubtitleFrame`, `SubtitleCue`, `FrameStorage` field sets match the doc sketch exactly (doc adds nothing the code lacks and vice versa; `AudioLayout` is an additive refinement the doc's "sample format, channels, rate" comment anticipated).
— `OpaqueHandle` consumers are real: h264/hevc hw decoders mint it (`decode.rs:466`), conformance tests reject it on CPU paths (`codecs/tests/fixtures_parent.rs:125`) — the platform-neutral contract is enforced both ways.

## 3. Findings

### F-001 [S2]
Where: audit/runs/baseline/INDEX.md rows 3–4 (evidence for this file's tests) vs the logs they summarize — test-workspace.log.txt:2411-2412, test-workspace-allfeatures.log.txt:2500-2501
Code (log tails, verbatim):
```
error: test failed, to rerun pass `-p desktop --test zero_alloc_transport`
TEST_EXIT=101
```
```
error: test failed, to rerun pass `-p first-vertical --test evidence`
EXIT=101
```
What is wrong: Both baseline workspace test runs aborted at the first failing test binary (cargo fail-fast; no `--no-fail-fast`). Run #3 stopped inside `apps/desktop`; run #4 got three packages further and stopped at `first-vertical`. Cargo runs packages alphabetically, so **every package sorting after those never executed a single test in any baseline run**: `input`, `localization`, `media-clock`, `media-probe`, **`media-types`** (this file's 5 tests among the crate's 77), `packet-queue`, `playback-engine`, `platform-{linux,macos,windows}`, `playlists`, `plugin-api`, `plugin-host-wasm`, `product-config`, `renderer-wgpu`, `settings-schema`, `streaming`, `subtitles`, `testkit`, `theme-tokens`, `transcode`, `video-engine`, `web`, `mem-audit`, `sweep` — verified: `grep media_types test-workspace.log.txt` finds only the `Compiling` line (:38); no `Running` line, no test names (`audio_format_frame_math` etc. appear in neither log); `playback_engine`/`video_engine`/`renderer_wgpu` likewise absent from both logs. INDEX.md rows 3–4 present the partial totals ("1681 passed / 1 failed / 24 ignored", "1751 passed / 2 failed / 25 ignored") as the result of `cargo test --workspace` without disclosing the abort — the natural reading (and the reading every wave-1 citation adopted) is full-workspace coverage, which is false for ~25 packages.
Why it matters: This is the audit program's evidence substrate: "cite baseline instead of running builds." For this file, the unit tests' passing status is UNKNOWN — no run ever executed them. The same hole covers the entire engine core (playback-engine, video-engine, renderer-wgpu, streaming, subtitles) — the crates at the heart of the "MP4 cannot be opened" question have zero executed test evidence in the baseline. Any wave-1/2 agent who cited baseline #3 as proof that these crates' tests pass cited evidence that does not exist.
What correct looks like: `cargo test --workspace --no-fail-fast --locked` (baseline rerun) and an INDEX.md correction listing exactly which packages' tests ran vs. not; until then, every file audit in the never-run set must mark its tests REAL-UNPROVEN (this audit does).
Confidence: HIGH — Verification: read of both log tails + exhaustive greps; REQUEST-ORCH-RUN filed (see log).

### F-002 [S4]
Where: crates/media-types/src/frame.rs:364-369
Code:
```
    fn color_description_default_is_bt709_limited() {
        let c = ColorDescription::BT709_LIMITED;
        assert_eq!(c.primaries, ColorPrimaries::Bt709);
        assert_eq!(c.range, ColorRange::Limited);
    }
```
What is wrong: The test named "default is bt709 limited" asserts only `primaries` and `range`. `BT709_LIMITED`'s `transfer` and `matrix` fields (frame.rs:161-166) are unpinned — mutating `transfer: Srgb` or `matrix: Bt601` into the const leaves this test green.
Why it matters: The const is the fallback colorimetry for every frame that doesn't carry its own (mjpeg pins its own JFIF_COLOR; other decoders may not), so a silent field mutation would mis-tag all default-colored output and no test would notice.
What correct looks like: Assert all four fields, or derive the expectation from a second construction site so drift is impossible.
Confidence: HIGH — Verification: read; test never executed in baseline (F-001) so this is inspection, not observed pass/fail.

## 4. Per-function table
| Function / unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SampleFormat` enum | 14-28 | REAL | — | 6 variants, explicit depth/signedness; doc-comment notes planar handled by layout (correct design). |
| `SampleFormat::bytes_per_sample` | 30-41 | REAL | — | 1/2/3/4/4/8 — hand-verified all arms; exhaustive match (no `_` arm — adding a variant is a compile error here ✓). |
| `AudioLayout` enum | 44-50 | REAL | — | Interleaved/Planar. |
| `AudioFormat` struct | 54-65 | REAL | — | sample/channels/layout/rate; doc 01 §3.3 "sample format, channels, rate" + layout refinement. |
| `AudioFormat::frame_bytes` | 67-72 | REAL | — | u64 math, no overflow (≤2^32 × 8); layout-independent (correct for planar: total = channels × frames × bps). |
| `AudioFormat::frames_in` | 74-83 | REAL | — | Zero-channel guard before division ✓; partial buffer → `None` (data, not guess) ✓; usize→u64 widening only. |
| `PixelFormat` enum | 90-103 | REAL | — | Nv12/I420/Yuyv422/Rgba8/Bgra8 — covers everything the shipped decoders emit (mjpeg: I420+Rgba8 at codecs mjpeg/mod.rs:378-381) and GPU inputs; minimal by documented design. |
| `ColorPrimaries` | 106-114 | REAL | — | 709/601/2020. |
| `TransferCharacteristics` | 117-127 | REAL | — | 709/Pq/Hlg/Srgb. |
| `ColorMatrix` | 131-138 | REAL | — | 709/601/2020Ncl (2020Cl omitted — minimal set, doc-blessed). |
| `ColorRange` | 141-147 | REAL | — | Limited/Full. |
| `ColorDescription` + `BT709_LIMITED` | 151-167 | REAL | S4 | F-002 (test pins 2/4 fields). Const itself correct. |
| `FrameStorage` enum | 175-181 | REAL | — | Cpu(SharedBytes) zero-copy / OpaqueHandle(u64) platform-neutral; hw consumers verified (h264/decode.rs:466). |
| `SubtitleCue` | 184-192 | REAL | — | span + plain-text payload (v1 scope documented). |
| `AudioFrame` | 195-207 | REAL | — | Matches doc 01 §3.3 exactly; `duration: Option` (no sentinel) ✓; no discontinuity flag (packet has one — inferable from PTS gap; open question, not a defect). |
| `VideoFrame` | 210-226 | REAL | — | Matches doc 01 §3.3 exactly (size:(u32,u32), color, interlaced, generation). No stride field — doc-blessed omission (tightly-packed CPU frames only; hw frames ride OpaqueHandle). `interlaced: bool` cannot express field polarity (mjpeg discards `field_polarity` value at mod.rs:389) — open question for the deinterlace phase. |
| `SubtitleFrame` | 229-237 | REAL | — | Cues + pts + generation. `Clone` deep-copies `Vec<SubtitleCue>` incl. `String`s — subtitle updates are low-rate; no hot-path clone found (rg over video-engine/packet-queue: no frame clones). |
| `Frame` enum + `pts`/`generation`/`is_stale` | 240-270 | REAL | — | Total accessors; strict-`<` staleness consistent with packet-queue semantics; production consumer codec-api/encode.rs:114. |
| `tests` mod | 272-370 | REAL-UNPROVEN | S2/S4 | 5 tests, honest assertions (see §5) — but NEVER EXECUTED in any baseline run (F-001). |

## 5. Test honesty (5 tests; none executed in baseline — analysis by inspection)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `audio_format_frame_math` (277-298) | YES | Real math: 2ch S16 → 4 B/frame; 4000 B → 1000 frames; 4001 B → None; planar F32 2ch → 8. Mutation of `frame_bytes` or the modulo logic breaks it. |
| `sample_format_byte_widths` (301-305) | YES | Pins 1/3/8 (the non-obvious widths). Partially redundant with the match arms being exhaustive, but pins values. |
| `frame_accessors_cover_all_kinds` (308-348) | YES | Builds all three Frame kinds with real SharedBytes; asserts pts/generation identity and is_stale in BOTH directions (not stale at g, stale at g.next). Mutation of any accessor arm fails. |
| `opaque_handle_names_no_platform` (351-362) | YES (weakly) | Enforces the naming contract via Debug output — catches a rename to a platform word; does not (cannot) test behavior (there is none — it's a vocabulary type). Gimmicky but honest for its stated purpose. |
| `color_description_default_is_bt709_limited` (365-369) | PARTIALLY | F-002: transfer/matrix mutations escape. |
No tautologies; no mocks; no `#[ignore]`; no cfg-gating.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| Frame/Packet/AudioFrame/VideoFrame/SubtitleFrame field sets | doc 01 §3.3 | REAL | Field-by-field match with the code (both directions). |
| "`OpaqueHandle` carries hardware frames without naming any platform [PROPOSAL]" | doc 01 §3.3 | REAL | frame.rs:176-181; hw producers (h264/hevc decode.rs) + CPU-path rejecting tests. |
| "data types and pure functions only … no I/O, no sentinel numbers" | doc 01 §2.2 (crate contract) | REAL | Whole file is types + 4 pure methods; no sentinels (Missing/None are typed options). |
| SampleFormat `{ I16, F32, PlanarF32 }` | doc 03 §4 sketch | SUPERSEDED | Code has U8/S16/S24/S32/F32/F64 + separate AudioLayout — cleaner and strictly more expressive; doc 03 is the older plan sketch ("candidate", "… enumerated"), doc 01 §3.3 governs. Not drift — documented supersession. |
| media-types tests pass (implied by INDEX rows 3-4) | baseline INDEX.md | FALSE (as evidence) | F-001: never executed; run aborted before the package. |

## 7. Dead / duplicated / unreachable code
- None. Every type has production consumers (codecs crate: 20+ files; video-engine; codec-api; packet-queue via SharedBytes).
- `PixelFormat::Yuyv422` and `Bgra8` — no current producer found in codecs (rg shows decoders emit I420/Rgba8/Nv12); they are renderer-facing vocabulary (Bgra8 "the desktop-compositor favorite") — vocabulary completeness, not dead code; note for the renderer auditor to confirm consumers there.

## 8. Dependencies & flags
- Zero runtime deps (crate contract: std-only ✓, Cargo.toml "No dependencies. Deliberately boring."). dev-dep: testkit (direction boundary enforced per Cargo.toml comment).
- No `unsafe`, no panics, no unwraps in production code (tests use `unwrap_or_else(|e| unreachable!)` — test-only panics). No feature gates. `u64::is_multiple_of` needs Rust ≥1.87 — repo pins 1.99.1 ✓.

## 9. Open questions for the next agent
- ORCHESTRATOR (urgent): rerun baseline tests with `--no-fail-fast` (REQUEST-ORCH-RUN in my log) — the never-run set includes playback-engine/video-engine/renderer-wgpu/streaming/subtitles, i.e., the engine core at the center of this audit's reason for existing.
- W02-A08 (media-types/src/error.rs, already committed on their branch): if that audit cited baseline run #3 as execution evidence for error.rs's 6 unit tests, the citation is void — media-types never ran (F-001).
- Wave-16-A07 (tests/shared_bytes_alloc.rs) and A04 (packet.rs): same evidence hole applies to their files' test claims.
- Video-frame auditor: field polarity is lossy (`interlaced: bool`); stride is absent — both doc-blessed today, both real constraints for the deinterlace/zoom cards.
