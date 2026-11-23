# AUDIT: crates/codecs/src/video/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Lines / functions | 42 / 0 (module registry: 7 `pub mod` gates, no code) |
| File verdict | REAL (accurate module registry; every gate claim verified against Cargo.toml and the CLI provider) |
| Reachable from shipped entry point? | YES — mjpeg/h264/hevc/mpeg12 unconditional; av1/dnx/vp9 behind `codec-*` features that are ALL in `default` (crates/codecs/Cargo.toml:47-58), so the release lane compiles them; mjpeg/av1/vp9/dnxhd/h264 lanes registered in the CLI provider (pipeline_host.rs:242-346) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | sed/rg on crates/codecs/Cargo.toml (features 42-81) and apps/cli/src/pipeline_host.rs; ran codec suites covering mjpeg (fixtures_parent 9/9), h264 (slice 9/9, h264_sw_differential 2/2), hevc (parser 19/19), mpeg12 (facts 6/6 + corpus 3/3) earlier in this session |

## 1. One-paragraph verdict
A 42-line module list whose every factual claim checks out: the external engines (av1/rav1d, dnx/filmcraft-dnx, vp9/rusty_vp9) ride `codec-*` features, all included in `default`, so the shipped release lane carries them; the in-house modules (h264, hevc, mjpeg, mpeg12) are unconditional as documented, with only h264's `sw` lane behind default-on `codec-h264-sw`; the wasm exclusion claim for av1 matches the recorded rav1d wasm32 compile failure note in Cargo.toml:62-65; and the CLI provider registers av1/vp9/dnxhd/h264/mjpeg lanes (pipeline_host.rs:304-346). One documentation nit: the header still narrates "v0 carried a single codec: mjpeg" in the historical present-tense chain, which under-describes the current six-module surface until the reader reaches the wave-7 sentence — harmless, accurate in order.

## 2. What actually works (verified)
- Feature graph: `default = [codec-aac, codec-alac, codec-av1, codec-dnx, codec-flac, codec-h264-sw, codec-mp2, codec-mp3, codec-opus, codec-vorbis, codec-vp9]` (Cargo.toml:47-58) — release lane = full set, no UNREACHABLE-by-feature video code.
- h264 mod.rs:55/60 gate `sw` behind `codec-h264-sw` — consistent with this file's doc.
- Downstream execution this session: mjpeg (fixtures_parent 9/9), h264 sw differential (2/2 lane_d), hevc parser (19/19), mpeg12 (6/6 + 3/3) — every in-house module's tests ran green through this registry's modules.

## 3. Findings
F-001 [S4]
Where: video/mod.rs:3-9
Code:
//! v0 carried a single codec: [`mjpeg`] — the Motion-JPEG /
//! baseline JPEG decoder behind [`codec_api::Decoder`] (card 2.B4,
//! the first vertical's video path, doc 03 §7). Wave 4 added the
What is wrong: pure narration nit — the module-doc leads with the historical v0 state and wave history before stating the current surface; a reader auditing "what video codecs exist" must synthesize six sentences to learn there are seven modules. The information is accurate; the structure buries the present. Confidence: HIGH — Verification: read.

## 4. Per-function table (n/a — no functions; per-module gate table instead)
| Module | Gate | In shipped release lane? | Verified by |
|---|---|---|---|
| av1 | `codec-av1` (default) | YES | Cargo.toml:50; CLI lane pipeline_host.rs:304 |
| dnx | `codec-dnx` (default) | YES | Cargo.toml:51; CLI lane :326 |
| h264 | unconditional (sw lane: `codec-h264-sw`, default) | YES | h264/mod.rs:55; CLI lane :346 |
| hevc | unconditional | YES (compiled; no CLI decode lane — matches "no decoder for hevc") | rg CLI; baseline mp4-smoke exit 65 |
| mjpeg | unconditional | YES | CLI lane :242; rc-smoke headless-decode PASS |
| mpeg12 | unconditional | YES (facts/demux; decode = typed refusal) | mpeg12_corpus 3/3 |
| vp9 | `codec-vp9` (default) | YES | Cargo.toml:58; CLI lane :313 |

## 5. Test honesty
No tests in this file (registry only). Covered transitively by every module's suites (multiple executed this session, all green).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "rav1d does not compile for wasm32-unknown-unknown (39 errors, measured 2026-11-10)" | mod.rs:18-21 + Cargo.toml:62-65 | REAL-UNPROVEN | consistent recorded claim; no wasm build run here (UNVERIFIED, would need wasm32 target) |
| "the EXTERNAL-engine modules ride their codec-* Cargo features … `default` enables the full set" | mod.rs:10-13 | REAL | Cargo.toml:47-58 read |
| "the in-house modules stay unconditional until the doc 01 §2.5(a) registration card" | mod.rs:14-16 | REAL | no cfg on h264/hevc/mjpeg/mpeg12 mod decls |

## 7. Dead / duplicated / unreachable code
None: all seven modules compiled in the default release lane; hevc compiles but has no shipped decode lane (the typed "no decoder" refusal is the honest current state, matching the baseline).

## 8. Dependencies & flags
External engines: rav1d (=1.1.0), filmcraft-dnx (=0.9.0), rusty_vp9 (=0.1.3), rust_h264 (=0.6.1) — all exact pins behind default-on features.

## 9. Open questions for the next agent
- av1/vp9/dnx decoder files (video/av1/decoder.rs, video/vp9/*, video/dnx/*) were NOT in my assignment; their admission claims (bit-exactness etc.) are unaudited by me — the mod.rs verdict says nothing about those modules' internals.
