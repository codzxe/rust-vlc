# AUDIT: crates/codecs/src/audio/aac/adts.rs
| Field | Value |
|---|---|
| Auditor | W07-A07 |
| Date | 2026-11-21 |
| Lines / functions | 347 / 8 items (2 consts, 1 enum + impl, `channel_count`, `AdtsHeader`, `parse_adts_header`, `AdtsFrame`, `walk_adts_frames`) + 4 tests + 1 test builder |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (two shipped lanes): (a) `lumen open foo.aac` → demux/src/adts/mod.rs:246/287 emits descriptor `aac-lc` → apps/cli/src/pipeline_host.rs:288-292 `"aac" \| "aac-lc" => self.seeded(track, config, AacDecoder::new)` → aac/decoder.rs:511 `parse_adts_header(data)` per packet + :526 `adts_engine(&header)`; (b) `lumen open foo.mp4` → demux/src/mp4/mod.rs:451 `b"mp4a" => Some("aac")` → same provider arm → ASC mode (adts.rs NOT on this lane — in-band headers absent). Also mod.rs:78 `is_adts` probe → `parse_adts_header(data).is_ok()`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full Read of the file · bit-layout hand-derivation against ISO 14496-3 §1.A.6.2 (all 14 fields, see §2) · `rg -n "parse_adts_header\|walk_adts_frames" crates apps` (callers: aac/decoder.rs:58,209,293,511; aac/fuzz.rs; aac/mod.rs:78) · `sed -n '445,470p' crates/demux/src/mp4/mod.rs` (mp4a→aac mapping) · `sed -n '470,580p' aac/decoder.rs` (ADTS vs raw/ASC mode split) · baseline test-workspace.log.txt:276-279 (all 4 unit tests ok) · `cat audit/runs/baseline/mp4-smoke-open-session-aac.txt` (the AAC-in-MP4 lane's shipped-binary evidence) |

## 1. One-paragraph verdict
A small, clean, byte-exact module: every ADTS fixed-header field was hand-derived against the ISO 14496-3 bit layout and is extracted correctly, the sampling-frequency and channel tables match the spec tables exactly, all error paths are typed with verbatim-equality-tested error values, the resync walker is bounded with correct end-of-stream partial-frame semantics, and all four unit tests execute in the committed baseline run. Zero findings after a hostile pass — the verification performed is listed in §2 so the zero is auditable, not lazy.

## 2. What actually works (verified)
- **Bit layout, field by field against ISO 14496-3 §1.A.6.2** (56-bit fixed header, MSB-first):
  - syncword 12 bits: `data[0] == 0xFF && data[1] >> 4 == 0xF` (adts.rs:121) ✓
  - ID bit 15: `data[1] & 0x08` (128), 0=MPEG-4/1=MPEG-2 per doc line 17 ✓
  - layer bits 13-14: `(data[1] >> 1) & 0x03`, must be 0 (129-136) ✓
  - protection_absent bit 16: `data[1] & 0x01` (137); header_len 7/9 (155) ✓
  - profile bits 16-17: `data[2] >> 6` (138-143) — 0=Main/1=LC/2=SSR/3=reserved ✓
  - sampling_frequency_index bits 18-21: `(data[2] >> 2) & 0x0F` (144) ✓; `SAMPLING_FREQUENCIES` (35-38) = {96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350} — exact ISO Table 1.16, 13 entries, 13-15 refused typedly (145-151) ✓
  - channel_configuration bits 23-25: `((data[2] & 0x01) << 2) | (data[3] >> 6)` (152) ✓; `channel_count` (69-75): 0→PCE-in-band(0), 1..=6→count, 7→8 — ISO Table 1.17 ✓ (channel_config is 3 bits, so `_ => 8` covers only 7 — no unreachable arm)
  - aac_frame_length bits 30-42 (13 bits): `((data[3] & 0x03) << 11) | (data[4] << 3) | (data[5] >> 5)` (153-154) — 2+8+3 = 13 bits, correct byte span ✓; max 8191 (bounded, no overflow: all operands ≤ usize)
  - number_of_raw_data_blocks bits 54-55: `data[6] & 0x03`, +1 (173) — precedence `as` before `+` verified ✓
  - fullness/original/home/copyright not extracted — informational, documented as such (24-26) ✓
- **Bounds/overflow**: truncation gate at 7 bytes (111-120); `frame_len < header_len` refused (156-162); walker gate `pos + 7 <= data.len()` before every parse (199) — no over-read; `pos.checked_add(frame_len)` (214) — no overflow path; `start.min(data.len())` (198) ✓.
- **Walker semantics** (196-228): byte-resync on any parse failure; a frame is accepted only when the full header validates AND `pos + frame_len <= len`; trailing partial left out (ffmpeg-matching, documented 189-191). Allocation = one `Vec<AdtsFrame>` ≤ len/7 entries — its only production caller is the whole-file surface `decode_stream` (aac/decoder.rs:293), not the per-packet contract path (which uses `parse_adts_header` at :511) — no hot-path cost.
- **Tests execute in baseline**: test-workspace.log.txt:276-279 — `channel_config_table_resolves_the_documented_counts`, `bad_sync_layer_index_and_short_buffers_are_typed`, `header_fields_parse_exactly`, `walk_resyncs_past_garbage_and_drops_the_trailing_partial` — all `ok` (inside the codecs lib suite, 506 tests).
- **Test builder bit-exactness**: `lc_stereo_441(64)` — hand-recomputed: h[3]=0x80 (chan 2, len hi 0), h[4]=8, h[5]=0x1F (len lo 0, fullness hi 0x1F), h[6]=0xFC (fullness lo 0x3F → VBR 0x7FF, blocks 0) → frame_len = 8<<3 | 0 = 64 ✓ matches the assert.

## 3. Findings
None. Hostile passes performed, all clean:
- False-sync acceptance: a header validating by chance on garbage also needs layer==0, index ≤ 12, frame_len ≥ 7, and (in `decode_stream`) profile/config gates + engine acceptance — no single-bit-crafted frame slips through untyped; the walker never panics on any input (all indexing gated).
- `frame_len == header_len` (zero-payload frame): accepted by the parse (156 only checks `<`), but the decode entry's empty-payload gate (decoder.rs:240-247, "a raw data block needs at least the element id bits") typed-refuses it — no wrong bytes emitted. Acceptable; the parse layer's job is geometry, the decode layer's is payload.
- CRC (protection_absent = 0): the 2 CRC bytes are skipped, never verified — ffmpeg-compatible behavior, documented header_len=9 handling is correct (decoder.rs:540 slices `header_len..frame_len`).
- `mpeg2` flag parsed but unused downstream — informational field, harmless.
- Overlong/adversarial `raw_blocks` > 1: parsed and exposed; multi-block frame handling is the decode entry's concern (see §9).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SAMPLING_FREQUENCIES` | 33-38 | REAL | — | exact ISO Table 1.16 |
| `AdtsProfile` + `name` | 41-63 | REAL | — | "profile − 1" encoding documented; names for typed refusals |
| `channel_count` | 65-75 | REAL | — | ISO Table 1.17; 3-bit domain so `_` arm covers only 7 |
| `AdtsHeader` | 77-100 | REAL | — | plain data, every field documented |
| `parse_adts_header` | 102-175 | REAL | — | bit-exact (§2); 5 typed error codes, no panic path |
| `AdtsFrame` | 177-184 | REAL | — | plain data |
| `walk_adts_frames` | 186-228 | REAL | — | bounded, resyncing, ffmpeg-matching partial-tail; `#[must_use]` |
| test `lc_stereo_441` builder | 236-248 | REAL | — | bit-verified by hand (§2) |
| test `header_fields_parse_exactly` | 250-267 | REAL | — | 10-field exact equality |
| test `channel_config_table_resolves_the_documented_counts` | 269-276 | REAL | — | pins table semantics |
| test `bad_sync_layer_index_and_short_buffers_are_typed` | 278-331 | REAL | — | full-error-value equality (strongest form) on 4 refusals |
| test `walk_resyncs_past_garbage_and_drops_the_trailing_partial` | 333-345 | REAL | — | leading garbage, inter-frame garbage, trailing partial — offsets exact |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| header_fields_parse_exactly | YES | every parsed field asserted by value |
| channel_config_table_resolves… | YES | 5-case table pin (near-tautological vs the match arms, but pins ISO semantics against refactors) |
| bad_sync_layer_index_and_short_buffers_are_typed | YES | asserts the ENTIRE error value (code + detail + message), so any taxonomy drift fails |
| walk_resyncs_past_garbage… | YES | mutation: stop-at-garbage → len 1; include-partial → len 3; wrong offsets fail |

## 6. Doc claims touching this file — incl. the wave-7 hostile question
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "status = **implemented** for AAC-LC (ADTS + ASC modes)" | docs/formats/audio-aac.md:8 | REAL (both lanes reachable + tested) / drift note: matrix row `codec-audio-aac-lc` is now `linux = "partial"` (feature-matrix.toml:673, 2026-11-10 triage: 142 concealed packets on valid corpus files; 1 aac-lc track refused) — the record predates the downgrade | reachability chains in header table; baseline log 276-279; mp4-smoke-open-session-aac.txt |
| **AAC-in-MP4 end to end in the shipped build** (wave-7 hostile question) | — | **REAL**: `lumen open sample-aac.mp4` → 1 track (Audio, `aac`) → Buffering → **Playing** → Stopped → Idle, EXIT 0, NO `decode.unsupported` — vs the H.264 sample's `error [decode.unsupported] … → Error`. The provider arm `"aac" \| "aac-lc"` (pipeline_host.rs:288-292) builds and configures AacDecoder with the esds ASC; adts.rs itself is NOT on the MP4 lane (ASC mode, decoder.rs:508-509) | audit/runs/baseline/mp4-smoke-open-session-aac.txt (quoted in §2 of this audit's command list); demux/src/mp4/mod.rs:451 |
| The engine-info literal "decoders: mjpeg, pcm_s16le" as build capability | baseline standing defect (INDEX.md CORRECTION v2) | FALSE as capability statement — under-reports; the whole audio tree (aac/alac/flac/mp3/mp2/opus/vorbis/adpcm) compiles in the default feature set (codecs/Cargo.toml:47-59) and is dispatched by the shipped provider | pipeline_host.rs:241-303 dispatch table |
| `lumen decode-bench` "audio track#1 aac — not benched (no decoder in this build…)" | baseline mp4-smoke-decode-bench.txt | MISLEADING (known standing defect) — bench lane hard-coded to MJPEG+PCM while `open` decodes AAC in the same binary | baseline INDEX.md CORRECTION v2 bullets 1-2 |
| "this is the codec-side mirror of `mp3::header`" (demux-side is 5.C2b's) | adts.rs:9-11 | REAL | demux/src/adts/** exists and is the parallel surface |

## 7. Dead / duplicated / unreachable code
None. Every item has a live caller (`parse_adts_header` ← decoder.rs:511 + mod.rs:78 probe; `walk_adts_frames` ← decoder.rs:293 `decode_stream` + fuzz.rs:85; `AdtsProfile`/tables ← decoder.rs:209 `adts_engine`).

## 8. Dependencies & flags
- Std-only, no engine imports (the symphonia edge lives in decoder.rs, not here). Compiled under `codec-aac` (default set) — shipped build includes it (baseline #1 CLEAN).
- No `unsafe`, no unwrap/expect/panic on any input path.

## 9. Open questions for the next agent
- W07-A02/A09 (aac/decoder.rs): (1) `raw_blocks > 1` frames — the ADTS parse exposes the count; does the engine path decode all blocks or only the first? Multi-block ADTS frames are encoder-rare but legal. (2) The compat-matrix "142 concealed packets on VALID corpus files" (feature-matrix.toml:682) — which layer conceals (demux damage accounting vs engine concealment) is undetermined from the notes.
- W07-A03 (aac/fuzz.rs): the fuzz harness calls both surfaces — confirm the walker's resync is fuzz-covered for the `frame_len == header_len` zero-payload frame.
- The `aac-lc` refused corpus track ("no decoder in this build", matrix notes) contradicts the provider's `"aac-lc"` arm — likely a demux-lane descriptor mismatch (es/aac.rs `RawAacDemuxer`?) — owner should reconcile with the "split the row's claimed descriptor set" next-step already recorded in the matrix.
