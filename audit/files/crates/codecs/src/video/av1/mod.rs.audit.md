# AUDIT: crates/codecs/src/video/av1/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-R2 (recovery; credit W06-A04) |
| Date | 2026-11-23 |
| Lines / functions | 86 / 0 (module root: docs, 3 mod decls, 2 re-export groups, 1 doctest) |
| File verdict | REAL (structural claims verified; doctest body verified by read only — never executed against current tree, see F-001) |
| Reachable from shipped entry point? | YES — `apps/cli/src/pipeline_host.rs:63` `use codecs::video::av1::Av1Decoder;` → `pipeline_host.rs:304-305` `"av1" \| "av01"` arm constructs `Av1Decoder::new()` (chain head `apps/cli/src/containers/mod.rs:55` `"ivf" => ivf::open` → `crates/demux/src/ivf/mod.rs:211` `b"AV01" => Some("av1")`, per W06-A09 decoder.rs audit, import+arm re-verified by me) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg` traces (exit 0); `sed -n` quote verifications (exit 0); `cargo test -p codecs --doc av1/mod` → 0 tests, 11 filtered out (exit 0); `cargo test -p codecs --doc -- --list` → 11 doctests listed (exit 0); scratch-crate rustdoc attribution experiment under audit/tmp/attr-test (exit 0) |

## 1. One-paragraph verdict
Module root for the shipped AV1 decoder: 51 lines of module docs, three module
declarations (decoder/obu/fuzz — all exist and each carries a completed sibling
audit: W06-A09 REAL, W06-A05 REAL, W06-A01 REAL), one consumed re-export
(`Av1Decoder`) and one dead re-export block. Every structural claim I could
check against the tree is TRUE: the rav1d pin in the doc is byte-identical to
Cargo.toml:125 (asm off), the Ogg "no AV1 mapping" claim holds (zero `av1`
matches in crates/demux/src/ogg/), MKV/MP4 `AV01` correctly described as future
container cards (no mapping in tree), and the `av1.bitdepth`/`av1.layout`
typed-refusal vocabulary exists at decoder.rs:537-542. The one real defect
class: the module's `# Example` doctest has provably never run against the
current file — the workspace's cached doctest binary (and the baseline's
"11 passed" Doc-tests codecs section) was compiled from a different revision of
this file, proven by rustdoc line-number attribution (F-001).

## 2. What actually works (verified)
- rav1d pin claim (mod.rs:8-11) == `crates/codecs/Cargo.toml:125`:
  `rav1d = { version = "=1.1.0", optional = true, default-features = false, features = ["bitdepth_8", "bitdepth_16"] }` — asm OFF confirmed. Record `docs/dependencies/records/rav1d-1.1.0.md` exists (13,052 bytes).
- Ogg claim (mod.rs:34-37): `rg -n -i "av1" crates/demux/src/ogg/` → 0 hits. TRUE.
- Container claims: `rg "AV01" crates/demux/src/mkv/ crates/demux/src/mp4/` → 0 hits; "follow in their container cards" is honest-future, not false-present.
- Typed-refusal vocabulary: decoder.rs:537-542 quotes `av1.layout` / `av1.bitdepth` Unsupported paths — matches mod.rs:46-48.
- Doctest semantics by read: `accepts()` (decoder.rs:293-295) is `matches!(descriptor, "av1" | "av01")`, so `configure` with `"vp9"` returns Err (`av1.codec_descriptor`) and the example's `assert!(refused.is_err())` holds; all names resolve (prelude at crates/codec-api/src/lib.rs:129-140 exports DecoderConfig/ThreadBudget/GenerationId/CodecDescriptor; `GenerationId::FIRST` ids.rs:445, `.next()` ids.rs:464; `DecoderConfig::new` 5-arg signature codec-api config.rs:100-106).
- — executed: nothing for THIS file's doctest (see F-001); module reachability verified by rg trace above.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/video/av1/mod.rs:53-74 (doctest) vs audit/runs/baseline/cargo-test-workspace.txt:1413
Code:
```
//! # Example
//!
//! ```
//! use codec_api::prelude::*;
//! use codecs::video::av1::Av1Decoder;
```
and baseline line:
```
test crates/codecs/src/video/mod.rs - video::av1 (line 76) ... ok
```
What is wrong: The doctest in THIS file has never been executed against the current tree, and the baseline "ok" at :1413 is evidence for a different revision. Proof: rustdoc attributes module-doc doctests to the declaring file with the child file's code-block line number — I reproduced this in a scratch crate (`audit/tmp/attr-test`: block at av1/mod.rs line 7 → listed as `src/video/mod.rs - video::av1 (line 7)`). The current block starts at mod.rs:55, but the cached doctest binary and the baseline both list `video::av1 (line 76)`; vp9 shows the same skew (line 87 listed vs line 47 in the current 71-line file). So the entire "Doc-tests codecs ... 11 passed" baseline section (cargo-test-workspace.txt:1401-1418) predates the current av1/mod.rs, vp9/mod.rs and video/mod.rs (43 lines, no doctests). `cargo test -p codecs --doc av1/mod` (ran, exit 0) matched 0 of 11 cached doctests; rebuilding is forbidden here (rav1d C toolchain, cargo-build-budget). Why it matters: the module's only executable claim rests on an evidence string that proves nothing about the current bytes (spec §6.4 class). What correct looks like: re-run doctests from a clean target dir and re-baseline. Confidence: HIGH — Verification: ran (attribution experiment + doctest --list) + read (assertion semantics).
F-002 [S4]
Where: crates/codecs/src/video/av1/mod.rs:82-86
Code:
```
pub use obu::{
    MAX_OBUS_PER_TU, OBU_FRAME, OBU_FRAME_HEADER, OBU_METADATA, OBU_PADDING,
    OBU_REDUNDANT_FRAME_HEADER, OBU_SEQUENCE_HEADER, OBU_TEMPORAL_DELIMITER, OBU_TILE_GROUP,
    OBU_TILE_LIST, ObuInfo, TemporalUnit,
};
```
What is wrong: All 12 re-exported names have zero consumers outside crates/codecs/src/video/av1/ (`rg "OBU_FRAME|OBU_TILE|MAX_OBUS_PER_TU|TemporalUnit|ObuInfo|OBU_SEQUENCE|OBU_PADDING|OBU_METADATA|OBU_REDUNDANT|OBU_TEMPORAL" crates/ apps/` → no hits outside the av1 module; the one external OBU caller uses the full path `codecs::video::av1::obu::parse_temporal_unit`, av1_differential.rs:538). Only the sibling `pub use decoder::Av1Decoder` is consumed (pipeline_host.rs:63, av1_differential.rs:39). Dead API surface presented as the module's public vocabulary. Confidence: HIGH — Verification: ran (rg).
F-003 [S4]
Where: crates/codecs/src/video/av1/mod.rs:23-26
Code:
```
//! - [`decoder`] — the engine seam (the repo's second sanctioned
//!   `unsafe` exception: rav1d 1.1.0's only public surface is its
//!   `dav1d_*` C ABI), the I420 copy-out, pooled output slots, and
//!   the `Decoder` impl.
```
What is wrong: "the repo's second sanctioned `unsafe` exception" is false as a repo-scope count: `#![allow(unsafe_code)]` / unsafe FFI exists in at least 8 other files — crates/codecs/src/video/h264/hw/vaapi/{dl,ffi,session}.rs (Lead-admitted HW lane, 60+ unsafe blocks per W06-A07's audit), apps/web/src/abi.rs, apps/desktop-tauri/src/main.rs, and 4 crates/platform-macos files. The claim is only true scoped to the codecs crate's decode engines (decoder.rs:10-11 says "first" = mjpeg/idct_simd.rs, which does contain unsafe — verified). Why it matters: doc asserts a repo-wide policy fact the tree contradicts. What correct looks like: "the second decode-engine unsafe seam in codecs". Confidence: MEDIUM (the word "sanctioned" may invoke a Lead ledger I cannot read; the raw file count is HIGH) — Verification: read + rg.

## 4. Per-function table (file <100 LOC; inventory instead)
| Item | Lines | Verdict | Notes |
|---|---|---|---|
| Module docs (claims) | 1-74 | REAL | All structural claims verified TRUE except F-001/F-003 |
| doctest | 55-74 | REAL-UNPROVEN | Semantics verified by read; never executed vs current tree (F-001) |
| `pub mod decoder` | 76 | REAL | decoder.rs exists; W06-A09 audit REAL, shipped |
| `pub mod obu` | 77 | REAL | obu.rs exists; W06-A05 audit REAL, shipped |
| `pub mod fuzz` | 79 | REAL | fuzz.rs exists; W06-A01 audit REAL (dev tooling) |
| `pub use decoder::Av1Decoder` | 81 | REAL | Consumed: pipeline_host.rs:63, av1_differential.rs:39 |
| `pub use obu::{...}` (12 names) | 82-86 | DEAD | Zero external consumers (F-002) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| doctest (mod.rs:55-74) | UNVERIFIED — never ran vs current tree | Asserts vp9-descriptor refusal; `accepts()` gate (decoder.rs:293-295) makes it pass by read; stale doctest cache (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| rav1d 1.1.0, `default-features = false, features = ["bitdepth_8","bitdepth_16"]`, asm OFF | mod.rs:7-11 | REAL | Cargo.toml:125 byte-identical (sed-verified) |
| OBU validator "ours, typed av1.obu.* damage before the engine" | mod.rs:19-22 | REAL | obu.rs:169 `parse_temporal_unit`, `av1.obu.count_limit` etc. (W06-A05 executed 11/11) |
| "second sanctioned unsafe exception" | mod.rs:23-26 | FALSE as repo-scope count | F-003: ≥8 other unsafe-permitting files |
| IVF `AV01` fourcc → descriptor `av1` → one TU per packet | mod.rs:30-33 | REAL | ivf/mod.rs:206-211 `b"AV01" => Some("av1")`; av1_differential.rs:41 uses IvfDemuxer |
| Ogg: "no AV1 mapping exists there today" | mod.rs:34-37 | REAL | rg -i av1 crates/demux/src/ogg/ → 0 hits |
| MKV/WebM/MP4 AV01 "follow in their container cards" | mod.rs:40-42 | REAL (honest future) | rg AV01 in demux mkv/mp4 → 0 hits |
| 8-bit 4:2:0 → I420; 10/12-bit, 4:2:2/4:4:4, monochrome typed Unsupported `av1.bitdepth`/`av1.layout` | mod.rs:44-48 | REAL | decoder.rs:537-542 (sed-verified); 10-bit refusal pinned by differential suite per W06-A09 |
| Example: wrong codec is typed refusal | mod.rs:53-74 | REAL-UNPROVEN | accepts() gate verified by read; doctest never executed vs current bytes (F-001) |

## 7. Dead / duplicated / unreachable code
F-002's 12-name re-export block. Nothing else: the module itself is the root of
a shipped decoder (reachability chain in header).

## 8. Dependencies & flags
`codec-av1` feature (Cargo.toml:66 `codec-av1 = ["dep:rav1d"]`) is in `default`
(Cargo.toml:50) — the shipped release lane carries this module (W06-A09 cites
cargo-build-release-lumen-cli.txt:98,162). Gate: `#[cfg(feature = "codec-av1")]`
at video/mod.rs:22 (wasm subsets exclude it — honest, documented).

## 9. Open questions for the next agent
- ORCHESTRATOR: the baseline Doc-tests codecs section (cargo-test-workspace.txt:1401-1418) provably predates the current av1/mod.rs, vp9/mod.rs, video/mod.rs (line-number skew 76→55, 87→47). Any wave agent citing "video::av1 (line 76) ... ok" or "video::vp9 (line 87) ... ok" as evidence is citing a stale tree. A clean-target doctest re-run is needed to re-baseline.
- Is there a Lead "sanctioned unsafe" ledger (F-003)? If yes, does vaapi count?
