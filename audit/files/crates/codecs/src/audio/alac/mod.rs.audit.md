# AUDIT: crates/codecs/src/audio/alac/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A01 |
| Date | 2026-11-27 |
| Lines / functions | 84 / 0 own fns (module organizer: docs + re-exports + 1 test) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: the re-exported `AlacDecoder` is dispatched by apps/cli/src/pipeline_host.rs:293-296 (`"alac"` → `self.seeded(track, config, AlacDecoder::new)`, cookie rides `extra_data`) and apps/web host; `parse_alac_config`/`decode_frame` additionally feed fuzz.rs and the differential suites |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p codecs --lib audio::alac` → 10 passed / 0 failed (exit 0); `cargo test -p codecs --test alac_bitexact` → 1 passed / 0 failed in 0.14s (exit 0); rg/sed verifications incl. dependency record |

## 1. One-paragraph verdict
This is a thin, honest module organizer: every item in its module map exists and is re-exported (L53-55), its doc claims check out against the code and records, and I executed both evidence chains it advertises — the 10 in-crate ALAC lib tests and the ffmpeg-differential `alac_bitexact` suite (1 test, byte-equality against pinned ffmpeg MD5 oracle digests, passed in 0.14s). The one real defect is inherited through its public re-export: `AlacConfig.max_frame_bytes` is parsed at the wrong cookie offset (be32(14) instead of Apple's 12..16 window) and is then never used — a lying, dead field on the public surface this module exposes. The single in-module test is a smoke that would not catch that offset bug.

## 2. What actually works (verified)
— verified by `cargo test -p codecs --lib audio::alac` → `test result: ok. 10 passed; 0 failed` (includes mod.rs's `output_format_matches_the_decode_mapping` and the fuzz smoke tests).
— verified by `cargo test -p codecs --test alac_bitexact` → `test fixtures_bitexact_vs_ffmpeg ... ok; 1 passed` — the module doc's bit-exactness AC is a real, running, byte-equality suite.
— verified by read: decoder.rs:103 `use symphonia_codec_alac::AlacDecoder as AlacEngine;` + Cargo.toml exact pin `symphonia-codec-alac = { version = "=0.6.1" … }`; dependency record docs/dependencies/records/symphonia-alac-0.6.1.md:29 "src/lib.rs:10 declares `#![forbid(unsafe_code)]`"; rg `unsafe` over alac/*.rs → doc-comment mentions only, zero unsafe tokens.

## 3. Findings
F-001 [S3] (cross-file; owner is decoder.rs — flagged here because mod.rs re-exports AlacConfig into the public surface)
Where: crates/codecs/src/audio/alac/decoder.rs:211, surfaced via mod.rs:53-54 (`pub use decoder::{AlacConfig, …}`)
Code:
```rust
    let config = AlacConfig {
        frame_length: be32(0),
        bit_depth: body[5],
        rice_pb_mb_kb: (body[6], body[7], body[8]),
        num_channels: body[9],
        max_frame_bytes: be32(14),
        sample_rate: be32(20),
    };
```
What is wrong: Apple's ALACSpecificConfig is frameLength 0..4, compatibleVersion 4, bitDepth 5, pb/mb/kb 6/7/8, numChannels 9, maxRun 10..12, maxFrameBytes 12..16, avgBitRate 16..20, sampleRate 20..24. `max_frame_bytes: be32(14)` reads the middle of maxFrameBytes/avgBitRate (upper 16 bits of maxFrameBytes glued to the lower 16 of avgBitRate) — the field does NOT contain maxFrameBytes. It is never read anywhere else in the tree (rg: only the struct def at decoder.rs:127 and this parse), so today it is a dead field whose name lies; the first future consumer gets a silently wrong number (a DoS gate keyed on it would misfire in both directions). Confidence: HIGH on the offset (layout cross-checked against the parse's own sibling offsets which match Apple exactly — bitDepth 5, numChannels 9, sampleRate 20; the 24-byte total with maxRun+maxFrameBytes+avgBitRate forces 12/16); Verification: read. What correct looks like: `be32(12)` or delete the field.
F-002 [S4]
Where: crates/codecs/src/audio/alac/mod.rs:61-82
Code:
```rust
    #[test]
    fn output_format_matches_the_decode_mapping() {
```
What is wrong (coverage note): the module's only test exercises the happy cookie → format mapping and the empty-packet refusal. It does not touch the version gate, the 48-byte `chan`-block shape, the frma/alac prefix stripping, or `max_frame_bytes` — exactly the field that is misparsed (F-001): this test could not catch it. Real coverage lives in decoder.rs's tests and the bitexact suite (both pass), so this is a hygiene note about the module-local smoke, not a gap in the module's evidence chain. Confidence: HIGH — Verification: read + ran.

## 4. Per-function table
File <100 LOC and owns no functions — table N/A. Re-exported surface audited: `AlacConfig` (F-001 field), `AlacDecoder` (REAL, shipped via CLI/web hosts), `DecodedAlac`/`decode_frame` (REAL, fuzz+bitexact), `output_format` (REAL, pinned by the mod test), `parse_alac_config` (REAL — typed gates length/version/bitDepth 1..=32/channels 1..=8/frameLength 1..=65536/rate≠0 verified at decoder.rs:189-249).

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| output_format_matches_the_decode_mapping (mod.rs) | YES for the fields it touches | asserts S16/2ch/44.1k/Interleaved from hand-built cookie bytes at the real offsets; decode_frame(&[]) must error. NO for max_frame_bytes (never asserted — F-001 slips past it) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "frame decode through the pinned `symphonia-codec-alac` 0.6.1 engine" | mod.rs:9-10 | REAL | decoder.rs:103 + Cargo.toml `=0.6.1` |
| differential tests "require byte equality against ffmpeg's own decoded PCM … oracle digests recorded" | mod.rs:40-47 | REAL | ran `--test alac_bitexact` → 1 passed; suite header records ffmpeg 7.1.5 MD5s (alac_bitexact.rs:19-24) |
| "No `unsafe` on this module's own paths (the engine is `#![forbid(unsafe_code)]`)" | mod.rs:32-33 | REAL (own paths: rg-verified zero unsafe tokens) / REAL-UNPROVEN (engine's forbid — taken from dependency record symphonia-alac-0.6.1.md:29; I did not read the crate's source) |
| "no panic on any input — every decode path is bounds-checked" | mod.rs:33-36 | REAL-UNPROVEN | 10 lib tests + fuzz smoke + bitexact pass and the parse gates are typed; exhaustive no-panic is the fuzz campaign's claim (nightly CI tier, not executable in this sandbox) |

## 7. Dead / duplicated / unreachable code
`AlacConfig.max_frame_bytes` — dead + misparsed (F-001). Nothing else in this file.

## 8. Dependencies & flags
`symphonia-codec-alac` =0.6.1 + `symphonia-core` =0.6.1 behind default feature `codec-alac` (enabled in shipped build); module glue is std + codec-api + media-types.

## 9. Open questions for the next agent
- decoder.rs owner: confirm F-001's offset against ffmpeg's libavcodec/alac.c `alac_decode_specific_config` field order, and decide fix-vs-delete for `max_frame_bytes`.
- The 48-byte cookie shape ("config + chan block (parsed by the engine — we only need the leading 24)") — is the `chan` block actually honored by symphonia 0.6.1, or is 48 just tolerated? Owner: decoder.rs.
