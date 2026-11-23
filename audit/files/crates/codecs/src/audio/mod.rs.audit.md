# AUDIT: crates/codecs/src/audio/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A03 |
| Date | 2026-11-24 |
| Lines / functions | 63 / 0 (module root: 9 `pub mod` declarations + docs) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — every declared module is dispatched by apps/cli/src/pipeline_host.rs:241-380 (mjpeg/mp3/mp2/vorbis/opus/aac/alac/adpcm_*/pcm_*/...) and apps/web/src/host.rs:639-641; all audio `codec-*` features are default-on (Cargo.toml:47-59), included in the release lane |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | cross-verification reads: Cargo.toml features (47-81), apps/cli/src/pipeline_host.rs dispatch, per-module audits (vorbis/decoder.rs, mp2/header.rs, adpcm/ms.rs, mp3/fuzz.rs, opus/error.rs — this wave); rg for engine-info decoder list |

## 1. One-paragraph verdict
REAL. The module root does exactly what it says: nine audio modules, the seven external-engine ones behind their `codec-*` Cargo features (each cfg attribute matches the feature list in Cargo.toml:47-81 exactly — verified one-to-one), the two in-house ones (adpcm, pcm) unconditional, matching the documented 7.K1 feature-gate history. Every engine attribution in the docs is true (claxon for FLAC, symphonia-bundle-mp3 for MP3/MP2, symphonia-codec-vorbis/aac/alac, ruopus for Opus — all pinned in Cargo.toml with exact versions). One claim is overstated: "bit-exact against the ffmpeg 7.1.5 reference" for the ADPCM family is proven false in the loud-delta regime for adpcm_ms (this wave's ms.rs audit, S2: the 32_767 delta clamp diverges from live ffmpeg 7.1.5 on legal loud input — empirically demonstrated).

## 2. What actually works (verified)
— verified by reading Cargo.toml:47-81 against the cfg attributes: `codec-aac`→mod aac (28-29), `codec-alac`→alac (38-39), `codec-flac`→flac (40-41), `codec-mp2`→mp2 (48-49), `codec-mp3`→mp3 (50-51), `codec-opus`→opus (57-58), `codec-vorbis`→vorbis (62-63); adpcm (37) and pcm (61) unconditional. Exact match, no orphan features, no ungated external engines.
— verified by the CLI dispatch (pipeline_host.rs:241-380): every module has a live arm; the "landed:" error string there (376-377) matches this module list.
— the engine attributions were verified per-module by this wave's audits (vorbis: symphonia-codec-vorbis =0.6.1 doing the decode; mp2: family engine reuse, no new dep; opus: ruopus behind our own head gate).

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/mod.rs:30-35
Code:
```rust
/// the WAV-carried MS/IMA-WAV pair plus Yamaha, OKI/Dialogic, SWF,
/// Apple IMA4 and Westwood IMA — one `AdpcmDecoder`, seven flavors,
/// bit-exact against the ffmpeg 7.1.5 reference. In-house (zero external
```
What is wrong: "bit-exact against the ffmpeg 7.1.5 reference" is stated as an unqualified property of the whole seven-flavor family. It is proven false for adpcm_ms in the loud-delta regime: ms.rs:105 clamps the delta ladder at 32_767 while ffmpeg 7.1.5 (live, this sandbox) lets `idelta` grow past it — a crafted legal block (header delta 32767 + nibble `0x88`) decodes `-1, 29438` here vs ffmpeg's `32767, 32767` (full evidence in audit/files/crates/codecs/src/audio/adpcm/ms.rs.audit.md F-001, S2). The claim is true for the committed fixtures and past-session corpus (verified byte-exact on adpcm_ms_mono.wav) but the blanket wording hides a proven divergence class. Why it matters: this module doc is the registry-facing summary; downstream readers inherit a false guarantee. What correct looks like: "bit-exact on the fixture/corpus lanes; loud-delta content bounded by a documented concealment clamp that diverges from the reference (ms.rs:98-105)". Confidence: HIGH — Verification: ran (live ffmpeg differential, this wave).

## 4. Per-function table
Not applicable (63 LOC, zero functions — module declarations only; every declaration listed in §2).

## 5. Test honesty
Not applicable (no tests in this file). The per-module suites are audited in their own files this wave: vorbis (5+21+1 tests ran), mp2 (27 ran), adpcm (55+2+4+4 ran), mp3 fuzz (2+2 ran), opus error (1 ran), zero_alloc_resample (1×2 ran) — all green.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| pcm = in-house unpacked-PCM decoder (first vertical) | mod.rs:3-4 | REAL | pcm mod unconditional; dispatched `pcm_*` arm (pipeline_host.rs:347-359) |
| flac through purity-pinned claxon | mod.rs:4-6 | REAL | Cargo.toml:86 `claxon = { version = "=0.4.3", optional = true }` |
| mp3 "through the purity-pinned symphonia-bundle-mp3 engine with our own ID3v2/Xing/VBRI/gapless/seek surface" | mod.rs:7-9 | REAL | Cargo.toml:90; the mp3 module's own headers/id3/xing/seek files (mp2/header.rs audit confirmed the family split) |
| vorbis "through the purity-pinned symphonia-codec-vorbis engine with our own header-chain, Vorbis-comment and granule-arithmetic surface" | mod.rs:10-13 | REAL | decoder.rs:107 import + Cargo.toml:103 — verified end-to-end this wave (hostile focus answered) |
| aac "HE-AAC honestly partial" | mod.rs:15-16 | REAL-UNPROVEN (cross-file) | the honesty note points into aac's module docs — not audited this wave (other agent) |
| external-engine modules ride codec-* features; in-house stay unconditional (7.K1) | mod.rs:20-26 | REAL | §2 one-to-one verification |
| adpcm "one AdpcmDecoder, seven flavors, bit-exact against the ffmpeg 7.1.5 reference" | mod.rs:30-35 | OVERSTATED | F-001: bit-exactness proven false for adpcm_ms loud-delta content (live-ffmpeg probe) |
| mp2 "family engine … no new dependency … our own Layer II gate, CBR geometry/seek arithmetic" | mod.rs:42-47 | REAL | mp2/header.rs audit this wave |
| opus "our own OpusHead validation (the pre-constructor panic gate), pre-skip/end-trim granule arithmetic and RFC 7845 → house channel-order mapping" | mod.rs:52-56 | REAL-UNPROVEN (cross-file) | opus/error.rs + decoder.rs:123-133 spot-checks consistent; full verification belongs to the opus decoder audit |

## 7. Dead / duplicated / unreachable code
None. All nine modules dispatched by shipped hosts. (Cross-reference for the apps/cli owner: engine-info's hard-coded two-decoder list at apps/cli/src/host.rs:1643-1658 understates this module's real surface — the "which registry lies" question from the baseline resolves to: the error string tells the truth, the info command is stale.)

## 8. Dependencies & flags
See §2 — the feature/module matrix is exact. All audio codec features default-on; the release lane builds them (baseline: lumen-cli + desktop clean).

## 9. Open questions for the next agent
- aac (HE-AAC "honestly partial"), flac, alac, pcm, and the non-assigned vorbis files (headers.rs, comments.rs, seek.rs, error.rs, fuzz.rs) and mp2/decoder.rs+seek.rs are other agents' assignments — this root's claims about them were only spot-checked.
- If the ms.rs delta-clamp divergence (S2) is fixed, revisit this file's F-001 wording.
