# AUDIT: crates/codecs/src/audio/aac/adts.rs
| Field | Value |
|---|---|
| Auditor | W07-A01 |
| Date | 2026-11-27 |
| Lines / functions | 346 / 4 non-test fns (+1 test helper, 4 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: aac/mod.rs:78 (`adts::parse_adts_header(data).is_ok()` — the codec-family probe) and aac/decoder.rs:58,293,307,513 (`parse_adts_header`/`walk_adts_frames` in both the free surface and the contract `Decoder::decode` ADTS path); AacDecoder is dispatched by apps/cli/src/pipeline_host.rs:288-291 (`"aac" | "aac-lc"`) and apps/web/src/host.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p codecs --lib audio::aac::adts` → 4 passed / 0 failed (exit 0); rg/sed caller + bit-layout verification (exit 0) |

## 1. One-paragraph verdict
A correct, panic-free, byte-exact ADTS fixed-header parser and resync walker. I hand-verified every field extraction against ISO 14496-3 §1.A.6.2 bit positions (sync 0xFFF, ID bit 11, layer bits 12-13, protection bit 15, profile bits 16-17, sampling index bits 18-21, channel config bits 23-25, 13-bit frame length across bytes 3-5, raw-block count bits 54-55): all correct, all reserved sampling indices (13-15) refused typedly via table lookup, frame_len < header_len refused, and the walk's resync/overflow handling is sound (`checked_add`, trailing partial frame dropped, `pos` provably ≤ len so `pos + 7` cannot overflow). The only defects are test-coverage gaps: the 9-byte CRC header shape and the MPEG-2 ID bit are never exercised. Note: AAC-in-MP4 does NOT go through this file — the esds ASC lane is decoder.rs's `sniff_asc_*` + `symphonia_codec_aac::AacDecoder::try_new` (see §6).

## 2. What actually works (verified)
— verified by `cargo test -p codecs --lib audio::aac::adts` → `test result: ok. 4 passed; 0 failed` (field-exact parse, channel table, typed refusals for sync/layer/index/short, garbage-resync walk with trailing-partial drop).
— verified by manual bit-layout check against the ISO layout documented at adts.rs:13-28 (read, all 10 fields).

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/aac/adts.rs:236-247 (test helper) and 152-155
Code:
```rust
    fn lc_stereo_441(len: usize) -> Vec<u8> {
        let mut h = vec![0xFF, 0xF1]; // sync | MPEG-4, layer 0, no CRC
```
What is wrong: every test frame is MPEG-4, protection_absent=1. The 9-byte CRC header (`header_len = 9`, the `frame_len < 9` boundary) and the `mpeg2` flag (adts.rs:128) are never executed by any test; a regression in the CRC branch ships unnoticed. Blast radius small (the branch is one constant + one comparison). Confidence: HIGH — Verification: read (no other test in crate exercises protection_absent=0 — rg over aac tests). 
F-002 [S4]
Where: crates/codecs/src/audio/aac/adts.rs:156-162
Code:
```rust
    let header_len = if protection_absent { 7 } else { 9 };
    if frame_len < header_len {
```
What is wrong: `frame_len == header_len` (a 7- or 9-byte frame with zero payload bytes and ≥1 claimed raw block) parses Ok. ISO-wise a raw_data_block cannot be empty, so this is degenerate input accepted at the parse layer; downstream the engine refuses the empty payload (aac/decoder.rs error path, not re-verified here — UNVERIFIED: downstream behavior of a zero-payload frame). No panic, no OOB (walk only slices `data[pos..]` for the 7-byte header). Confidence: MEDIUM — Verification: read + walk logic traced.
Zero findings would be dishonest for a shipped-parser file, but nothing above S4 was found: no overflow, no panic path, no allocation beyond the frame list, O(N) resync.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| channel_count | 69-75 | REAL | — | Table 1.17 exact: 0→0 (PCE), 1..=6→n, 7→8; config is 3 bits so `_` covers only 7 |
| parse_adts_header | 110-175 | REAL | F-002 | all field extractions hand-verified vs ISO; typed errors for truncated/sync/layer/index/frame_len |
| walk_adts_frames | 196-228 | REAL | — | O(N) resync; checked_add; trailing partial dropped; pos bounded by len |
| (tests ×4) | 250-345 | REAL | — | honest, would fail if broken |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| header_fields_parse_exactly | YES | asserts all 10 fields incl. derived channels/frame_len/raw_blocks |
| channel_config_table_resolves_the_documented_counts | YES | boundary 6 vs 7→8 pinned |
| bad_sync_layer_index_and_short_buffers_are_typed | YES | exact error equality incl. context codes |
| walk_resyncs_past_garbage_and_drops_the_trailing_partial | YES | offsets asserted numerically (2, 2+64+5) |
Gap: no CRC-shape / MPEG-2 test (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "0 is PCE in band (refused typedly upstream)" | adts.rs:66-67 | REAL | aac/decoder.rs:188-197 `gate_channels` refuses `aac.channel_config_pce`, called at decoder.rs:307 and 513 |
| "a trailing partial frame is left out … trailing garbage never fails the walk" | adts.rs:189-191 | REAL | walk L217-220 `break` + test |
| (context) AAC-in-MP4 routing: esds ASC → decoder | Cargo.toml codec-aac + aac/decoder.rs:433-470 | REAL | `use symphonia_codec_aac::AacDecoder as AacEngine` (decoder.rs:65); configure sniffs ASC object type/channel config in-house (`sniff_asc_*`), gates, then `AacEngine::try_new(&params, …)` with `params.extra_data = asc` (decoder.rs:459-470); the ASC blob itself is harvested by apps/cli/src/containers/mp4.rs:373-381 (`asc_of_entry` → esds DecoderSpecificInfo) |

## 7. Dead / duplicated / unreachable code
None. All three functions have live callers (probe + decoder + fuzz.rs of the aac module).

## 8. Dependencies & flags
`super::error` (AacError/corrupt), `media_types::error::CorruptDetail`. std-only. Behind default feature `codec-aac` (Cargo.toml) — enabled in the shipped default build.

## 9. Open questions for the next agent
- What does aac/decoder.rs do with a zero-payload ADTS frame (F-002) — typed refusal or engine panic? (Its file, not mine.)
- The ADTS *demuxer* lane (5.C2b, crates/demux ADTS packetizer) re-implements header walking — check duplication drift against this file.
