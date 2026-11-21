# AUDIT: crates/codecs/src/video/vp9/error.rs
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-11-21 |
| Lines / functions | 187 / 9 items (Vp9Error enum, into_engine_error, From, Detail+Display+Error, 2 constructors, 1 table-driven test) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open foo.ivf` → apps/cli/src/containers/mod.rs:55 `"ivf" => ivf::open` (and :57 `"matroska" => mkv::open` for WebM V_VP9) → track descriptor `vp9` → apps/cli/src/pipeline_host.rs:313-317 `"vp9" => { let mut decoder = Vp9Decoder::new(); … }` → decoder.rs:74 `use super::error::{Vp9Error, corrupt, unsupported}` — every variant and both constructors consumed (decoder.rs:174-374). EXECUTED: baseline test-workspace.log.txt:780 (`taxonomy_maps_each_variant_to_its_public_kind` ok) and :1286-1294 — the vp9 differential suite (4 ok in 1.58 s) including `tenbit_lane_is_a_typed_refusal_not_a_misdecode`, which drives THIS file's `Unsupported`/`vp9.bitdepth` mapping through the wired decoder, and the hostile lanes driving `Corrupt` → Recovered) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `Read` full file; `rg -n "unsupported(\|corrupt(\|Vp9Error::\|vp9\."` decoder.rs (all 3 variants + 8 dotted codes in live use); `grep -n "vp9" baseline test log` (unit :776-780 + differential :1286-1294, all ok); `sed -n '1286,1295p'` + `head -80 tests/vp9_differential.rs` (lane evidence: 5 MD5-pinned lanes vs ffmpeg libvpx-vp9, lane F = typed vp9.bitdepth refusal); `grep -n "ivf\|mkv" containers/mod.rs` (shipped container dispatch); `grep -n "struct Detail"` across codecs error modules (duplication evidence for F-001). |

## 1. One-paragraph verdict
A small, disciplined taxonomy file that does exactly what its docs claim: three variants (`Unsupported`/`Corrupt`/`BadState`) mapping one-to-one onto `ErrorKind::Unsupported`/`Corrupt`/`Decoder{BadState}` at stage `Decode`, all of them consumed by the wrapper (eight distinct `vp9.*` dotted codes in live use at decoder.rs:174-374), no panics, no unwraps, constructors total. The doc-comment mapping table (lines 37-41) matches the code arm-for-arm, the frame-scoped isolation claim is backed by executed tests (garbage-then-real recovery, both at unit level and in the ffmpeg-oracle differential suite), and the single test is genuinely content-sensitive — it pins the exact rendered `Display` string, the stage, and the error-source chain for all three variants. The only defect is hygiene-level: this is the Nth verbatim copy of the private `Detail(String)` error-carrier struct that every codec module re-declares. Note the reachability story is stronger than most of this wave: VP9 decode is genuinely wired into the shipped CLI (IVF + Matroska lanes) and differentially validated against ffmpeg's libvpx-vp9.

## 2. What actually works (verified)
— Mapping table verified arm-for-arm: `Unsupported` → `ErrorKind::Unsupported` + `FormatInfo{name:"vp9"}` + source `Detail` (75-85); `Corrupt` → `ErrorKind::Corrupt{detail}` + source `Detail(detail_text)` (86-95); `BadState` → `ErrorKind::Decoder{BadState}` + `vp9.not_configured` (96-102). Stage `Decode` in all three.
— Full consumption verified: decoder.rs uses `corrupt` at 174 (`vp9.frame.geometry`), 209 (`vp9.frame.planes`), 231 (`vp9.frame.plane_bounds`), 339 (`vp9.packet.push`); `unsupported` at 186 (`vp9.bitdepth`), 198 (`vp9.layout`), 282 (`vp9.codec_descriptor`); `Vp9Error::BadState` at 317/320/367/374. No dead variants.
— Executed: unit test ok (baseline :780) asserting the exact strings `"[decode] unsupported in vp9 (vp9.codec_descriptor)"`, `"[decode] corrupt structure (vp9.packet.decode)"`, `"[decode] decoder: invalid state transition (vp9.not_configured)"` + stage + source text for all three arms.
— The taxonomy's honest-boundary claim (10-bit/4:2:2+ = typed refusal, never mis-decode) is executed end-to-end: baseline :1292 `tenbit_lane_is_a_typed_refusal_not_a_misdecode` ok — the wired decoder refuses a profile-2 10-bit fixture through THIS file's `vp9.bitdepth` arm.
— The damage-isolation claim (module docs 19-24) is executed: `garbage_packet_is_recovered_not_fatal` (:777) and `hostile_garbage_is_recovered_and_the_stream_survives` (:1289) both ok — Corrupt maps to a per-frame Recovered outcome, not a session teardown.
— No `unwrap`/`expect`/`panic!`/`todo!` anywhere in the file (grep-verified) — the module-doc guarantee (27-28) holds.

## 3. Findings

### F-001 [S4] Nth verbatim copy of the private `Detail(String)` error-carrier pattern
Where: crates/codecs/src/video/vp9/error.rs:113-124
Code:
```
/// Chain-root carrier for the human-readable reason (the 2.B4
/// `FeatureDetail` pattern verbatim).
#[derive(Debug, Clone)]
struct Detail(String);

impl std::fmt::Display for Detail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Detail {}
```
What is wrong: The identical 10-line private carrier is re-declared in at least eight sibling modules (verified: video/{dnx,mpeg12,vp9}/error.rs, audio/{aac,alac,flac}/error.rs, mjpeg's `FeatureDetail`, and the pattern is self-described as "verbatim"). Each copy is trivially correct, but the family is pure copy-paste with zero shared definition — a fix to one (e.g. adding `source()` chaining semantics) will not propagate.
Why it matters: Checklist duplication class; blast radius is maintenance-only (each copy is 10 lines of obvious code), hence S4 despite the count.
What correct looks like: One `pub(crate) struct DetailString(String)` in `codec-api` (or media-types) re-exported by the codec modules — the "verbatim pattern" comment even names the right home.
Confidence: HIGH — Verification: grep across `crates/codecs/src/**/error.rs` (8+ identical declarations listed in Commands).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-28 | REAL | — | every claim traced: mapping table ✓, isolation ✓ (tests :777/:1289), no-panic guarantee ✓ (grep) |
| `Vp9Error::Unsupported` | 47-52 | REAL | — | code+detail; consumed ×3 |
| `Vp9Error::Corrupt` | 57-64 | REAL | — | structured CorruptDetail + text; consumed ×4 |
| `Vp9Error::BadState` | 65-67 | REAL | — | consumed ×4 (decode/flush before configure) |
| `into_engine_error` | 72-104 | REAL | — | arm-for-arm per doc table; stage Decode always |
| `From<Vp9Error>` | 107-111 | REAL | — | delegate |
| `Detail` + Display + Error | 115-124 | REAL | S4 | F-001 duplication |
| `unsupported` constructor | 127-132 | REAL | — | total |
| `corrupt` constructor | 135-145 | REAL | — | total |
| test `taxonomy_maps_each_variant_to_its_public_kind` | 151-186 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind (baseline :780 ok) | YES | asserts exact rendered Display strings for all three variants (a changed kind, stage, code, or source-wiring changes the string), the `PipelineStage::Decode` field, and the error-source text — table-driven over all arms |
Mutation check (reasoned): remap `Corrupt` → `ErrorKind::Unsupported` → rendered string mismatches → fail; drop `.with_source(Detail(…))` → source-text assertion fails; move stage to Demux → stage assertion fails.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| variant → kind/stage table (Unsupported/Corrupt/BadState) | error.rs:37-41 | REAL | code 75-102 matches arm-for-arm; pinned by the executed test |
| "a decoded frame whose bit depth is not 8 … typed refusal at the emit seam, never a mis-decode" | error.rs:8-14 | REAL | baseline :1292 tenbit-lane test ok (vp9.bitdepth through the wired decoder) |
| "Isolation is frame-scoped … the next packet decodes against it" | error.rs:19-24 | REAL | :777 + :1289 garbage-recovery tests ok |
| "Nothing panics; every constructor is total; no `unwrap` on any input path" | error.rs:27-28 | REAL | grep over the file: zero unwrap/expect/panic/todo |
| (vp9/mod.rs) "the end-to-end mkv and ivf decode lanes" | vp9/mod.rs:24-26 | REAL-UNPROVEN for the shipped-binary lane | container dispatch exists (containers/mod.rs:55/57) and the differential suite demuxes real IVF through `demux::ivf` + the wired decoder (:1286-1294 ok); no baseline `lumen open` smoke run on an .ivf file exists — the executed evidence is test-harness-level, not shipped-CLI-level |

## 7. Dead / duplicated / unreachable code
- No dead variants or constructors (all consumed — §2).
- Duplication: the `Detail` carrier family (F-001).

## 8. Dependencies & flags
- VP9 rides the `codec-vp9` Cargo feature (`dep:rusty_vp9`), which is in `codecs`' **default** set — so the shipped `lumen` CLI and `lumen-tauri` (built by release.yml with no feature overrides) both carry it; only `--no-default-features` subsets (wasm tree-shake) drop it.
- Depends on `media_types::error::{CorruptDetail, ErrorContext, FormatInfo}` and the standard error traits; no unsafe, no allocation beyond the two `String`s per error.

## 9. Open questions for the next agent
- decoder.rs owner: the `vp9.packet.push` corrupt arm (decoder.rs:339) — confirm the engine's rejection is distinguishable from geometry damage in the `detail` field (both use `InvalidStructure{offset:None}` in places); if every site uses the same class, the `CorruptDetail` structuring adds little reporting value.
- Whoever touches codec-api next: F-001's shared carrier is a 10-line consolidation the whole codecs tree can adopt.
