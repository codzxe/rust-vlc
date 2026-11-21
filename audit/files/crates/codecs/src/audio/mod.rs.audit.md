# AUDIT: crates/codecs/src/audio/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A08 |
| Date | 2026-11-21 |
| Lines / functions | 63 / 9 module declarations (no functions — registry file) |
| File verdict | REAL (correct, consistent registry; one doc-wording defect) |
| Reachable from shipped entry point? | YES for ALL nine modules in the native `lumen` CLI: apps/cli/Cargo.toml:51 `codecs = { path = "../../crates/codecs" }` (default features) and crates/codecs/Cargo.toml:47-59 `default` = the full codec set, so aac/adpcm/alac/flac/mp2/mp3/opus/pcm/vorbis all compile and all are dispatched (apps/cli/src/pipeline_host.rs:253-303). Contrast: the wasm module ships decoder-less (apps/web/Cargo.toml:83-90 — codecs is optional, `default-features = false`, "The SHIPPED module enables none of" the external codecs). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read crates/codecs/Cargo.toml [features] (default set, per-codec deps, mp2⇒mp3 implication) · read apps/cli/Cargo.toml:51 + apps/web/Cargo.toml:79-121 (who gets which features — the wave-7 reachability split) · `rg -n "cfg\(feature" apps/web/src/host.rs` (web dispatch mirrors the gates 1:1) · read crates/codecs/src/lib.rs:58 (`pub mod audio`) · `ls audio/` (9 dirs = 9 declarations, none missing) · `sed -n` quote verification |

## 1. One-paragraph verdict
This is the answer key to the wave-7 hostile question, and it is internally honest: every external-engine audio decoder (aac, alac, flac, mp2, mp3, opus, vorbis) rides its `codec-*` Cargo feature with `default` enabling the full set, and the two in-house modules (adpcm, pcm) are unconditional. The shipped native `lumen` CLI depends on `codecs` with default features, so the entire audio tree is compiled and dispatched — the `lumen engine-info` "decoders: mjpeg, pcm_s16le" literal (a standing Wave-0 defect) under-reports a registry that actually contains nine audio modules; the wasm web module, by contrast, genuinely ships decoder-less (opt-in features only). The file's gate list matches the Cargo.toml feature list exactly, the mp2⇒mp3 implication is honored (mp2/header.rs imports crate::audio::mp3 only under the gate), and all nine audio directories are declared. The only defect is a doc word: "HE-AAC honestly partial" misdescribes a hard typed refusal.

## 2. What actually works (verified)
— Gate↔module consistency: the 7 `#[cfg(feature = "codec-*")]` declarations (aac:28-29, alac:38-39, flac:40-41, mp2:48-49, mp3:50-51, opus:57-58, vorbis:62-63) correspond 1:1 to Cargo.toml:60-81 features; adpcm:37 and pcm:61 unconditional as documented (24-26).
— `default` = full set verified (Cargo.toml:47-59) — the doc's "the unconditional pre-7.K1 graph is unchanged" claim holds for every default-features consumer.
— Baseline execution: the AAC lane plays end-to-end in the shipped binary (`sample-aac.mp4` → Playing; `sample-420.mp4` H.264+AAC plays — INDEX.md corrected MP4 matrix); the AVI PCM lane decodes 60/60 frames in decode-bench — two of the nine modules proven live in the shipped build, the rest REAL-UNPROVEN at the binary level but compiled+dispatched (per-file audits this wave).
— Web dispatch mirrors the gates exactly (apps/web/src/host.rs:622-673, one `#[cfg]` per codec arm).

## 3. Findings
### F-001 [S4] "HE-AAC honestly partial" misdescribes a hard refusal
Where: crates/codecs/src/audio/mod.rs:13-16
Code:
```
//! [`aac`] is the AAC-LC decoder through the purity-pinned
//! `symphonia-codec-aac` engine with our own ADTS
//! header parse and profile/HE gates (card 5.C2a — the decoder half
//! of 5.C2; HE-AAC honestly partial, see its module docs).
```
What is wrong: "HE-AAC honestly partial" reads as partial HE-AAC support. The aac module's own docs say the opposite: "HE-AAC (SBR/PS), honestly absent — an ASC that signals object type 5/29 is refused typedly, never silently downgraded" (aac/error.rs:10-12; refusal site aac/decoder.rs:159 `aac.he_sbr`). There is no partial decode; there is a typed refusal.
Why it matters: registry-level doc is the first thing a reader/consumer consults; "partial" invites expectations (downsampled SBR output?) the code deliberately refuses to fake — the exact honesty inversion the module's own design fights.
What correct looks like: "HE-AAC honestly absent (typed refusal), see its module docs".
Confidence: HIGH — Verification: read both sites; refusal code path confirmed by rg (decoder.rs:159-198).

## 4. Per-function table (registry file: per-module rows)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc | 1-26 | REAL | F-001 | gate inventory and history accurate otherwise |
| `pub mod aac` (codec-aac) | 28-29 | REAL | — | dispatched: pipeline_host.rs:288-292 "aac"/"aac-lc" |
| `pub mod adpcm` (unconditional) | 30-37 | REAL | — | dispatched: pipeline_host.rs:298-303 `adpcm_*` |
| `pub mod alac` (codec-alac) | 38-39 | REAL | — | dispatched: :293-297 |
| `pub mod flac` (codec-flac) | 40-41 | REAL | — | dispatched: :253-260 |
| `pub mod mp2` (codec-mp2 ⇒ codec-mp3) | 42-49 | REAL | — | dispatched: :266-270; header.rs's cross-import gated consistently |
| `pub mod mp3` (codec-mp3) | 50-51 | REAL | — | dispatched: :261-265 ("mp3"/"mp1") |
| `pub mod opus` (codec-opus) | 52-58 | REAL | — | dispatched: :276-282 |
| `pub mod pcm` (unconditional) | 59-61 | REAL | — | the first vertical's audio half (AVI 60/60 baseline) |
| `pub mod vorbis` (codec-vorbis) | 62-63 | REAL | — | dispatched: :271-275 |

## 5. Test honesty
No tests in this file (pure registry). The gate consistency it declares is exercised by every baseline build: `cargo build --release --locked --workspace` CLEAN (INDEX.md #1) compiles the full set; the wasm size-report lane builds the subset (docs/web/size-report.md).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the EXTERNAL-engine modules ride their `codec-*` Cargo features … `default` enables the full set" | mod.rs:20-24 | REAL | Cargo.toml:47-59 |
| "the in-house modules ([`adpcm`], [`pcm`]) stay unconditional" | mod.rs:24-26 | REAL | mod.rs:37,61 |
| "[`aac`] … HE-AAC honestly partial" | mod.rs:16 | FALSE (wording) | F-001: the module refuses HE-AAC typedly |
| "mp2 … Rides `codec-mp2` (which implies `codec-mp3`)" | mod.rs:46-47 | REAL | Cargo.toml:77 |
| "[`opus`] … RFC 7845 → house channel-order mapping" | mod.rs:54-56 | REAL | opus/order.rs exists (module dir listing) |

## 7. Dead / duplicated / unreachable code
None in-file. Cross-cutting answer for the wave question: NOTHING in this tree is dead in the native shipped build (all compiled + dispatched); on wasm, all seven external-engine modules are dead-by-build (compiled out unless `builtin-codecs*` opted in — apps/web/Cargo.toml:97-121, the honest capability snapshot).

## 8. Dependencies & flags
The feature graph: codec-aac/alac/mp3/vorbis each pull `dep:symphonia-core` (shared node); codec-flac → claxon =0.4.3; codec-opus → ruopus =0.1.2 (default-features off, binding condition); codec-mp2 → codec-mp3. Exact pins throughout (bump re-triggers the dependency records per doc 11 §3.7). No license conflicts (cargo-deny: licenses ok — INDEX.md #25).

## 9. Open questions for the next agent
- The `lumen engine-info` under-reporting (hard-coded `decoders: mjpeg, pcm_s16le`, both `active: false` — INDEX.md standing defects) now has a second consequence: it hides nine live audio modules. Whichever agent owns apps/cli/host.rs `engine_info()` should derive the list from the real registry (or the DecoderProvider match arms).
- apps/web capabilities.rs — does the web module's reported capability snapshot agree with the decoder-less default (no phantom codec claims)? (apps/web agent.)
- codec-h264-sw is in `default` but not in this file's audio tree (video side) — the video mod.rs agent should confirm the same default-features story holds there.
