# AUDIT: crates/codecs/src/audio/flac/metadata.rs
| Field | Value |
|---|---|
| Auditor | W07-A10 |
| Date | 2026-11-21 |
| Lines / functions | 472 / 11 prod items + 6 tests |
| File verdict | REAL (split: STREAMINFO parse reachable+executed; chain walk/SEEKTABLE index UNREACHABLE) |
| Reachable from shipped entry point? | PARTIAL. `parse_streaminfo`: YES — `lumen open *.mkv` (h264+FLAC fixture lane): `apps/cli/src/containers/mkv.rs:320-336` harvests the 34-byte body → `pipeline_host.rs:253` seeded `FlacDecoder` → `flac/decoder.rs:540` `parse_streaminfo(body)`; executed in baseline (`mkv.rs:484` `flac_fixture_opens_and_decodes_the_full_stream` among the 1681 passing tests, run #3). `parse_stream`/`is_flac`/`SeekIndex::from_body`: NO — only callers are the flac fuzz harness (`fuzz.rs:69,107`) and tests; native `.flac` open is a typed stub refusal (`apps/cli/src/containers/es.rs:258-271`). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `rg -n 'parse_stream\|is_flac\|SeekIndex\|SeekPlacer…'` caller maps across crates/apps; `sed -n` quote checks; cross-examined `apps/cli/src/containers/ogg.rs:180-200`, `containers/mkv.rs:240-336`, `containers/es.rs:258-271`, `crates/demux/src/ogg/mod.rs:166-182, 304-314, 960-995, 1273-1282`, `flac/decoder.rs:528-545`; matrix rows `container-ogg` (:162-166), `codec-audio-flac` (:699-709), `container-flac-native` (:233-243). |

## 1. One-paragraph verdict
The STREAMINFO parser is spec-correct, panic-free (every index guarded by the 34-byte length check), honestly typed, and genuinely reachable through the shipped Matroska lane — the packed 20+3+5+36-bit field decode, the 24-bit frame-size reads, and the validation set were all hand-verified against the FLAC spec and are pinned by exact-value tests. But the file's second half (the metadata-chain walk and SEEKTABLE index that the format doc and matrix advertise as "SEEKTABLE seeking") has no shipped caller, and the audit uncovered a hard integration break one seam downstream of this file's contract: the Ogg FLAC harvest passes the full 38-byte block-header-prefixed STREAMINFO packet where `FlacDecoder::configure`→`parse_streaminfo` requires the bare 34-byte body, so FLAC-in-Ogg — one of only two shipped FLAC container lanes — fails at configure (`min_block_size=0` → typed `flac.streaminfo.blocksize`) and cannot decode at all.

## 2. What actually works (verified)
— verified by hand-decode of the packed field (metadata.rs:172-178): rate `packed>>44` (20 bits), channels `((packed>>41)&7)+1`, bps `((packed>>36)&0x1f)+1`, total `packed & 0xF_FFFF_FFFF` (36 bits) — matches spec §5.1; test at :346-373 pins the exact arithmetic (packed construction hand-checked: 44100<<44 | 1<<41 | 15<<36 | 44100 → 44100/2/16/Some(44100) ✓).
— verified by reading :240-312: every slice access (`data.get(pos..pos+4)`, `get(body_start..body_start+length)`) is bounds-checked with typed truncation errors; the walk cannot loop forever (pos strictly advances ≥4 per block, ends at data end with `flac.metadata.header_truncated`).
— verified by baseline run #3 (`test-workspace.log`, 1681 passed): the in-file tests + the MKV end-to-end flac decode test ran green in the shipped-lane shape.
— `read_u24` (:329-332) is only ever called with `&body[4..7]`/`&body[7..10]` after the ≥34 check — no panic path.

## 3. Findings
### F-001 [S2]
Where: defect site apps/cli/src/containers/ogg.rs:192-199; contract site crates/codecs/src/audio/flac/metadata.rs:168-199 + decoder.rs:536-540
Code (containers/ogg.rs:192-199):
```
            if codec == "flac" {
                // The native STREAMINFO block (the mapping header's
                // successor) — the FLAC decoder's 34-byte truth.
                match headers.get(1) {
                    Some(h) => SharedBytes::from_vec(h.as_slice().to_vec()),
                    None => SharedBytes::from_vec(Vec::new()),
```
What is wrong: The Ogg FLAC mapping's second header packet is the STREAMINFO block **including its 4-byte native metadata-block header** (`[0x00,0x00,0x00,0x22]` + 34-byte body = 38 bytes). The repo's own demuxer proves the packet shape: `is_header_packet` matches `data.starts_with(&[0x00, 0x00, 0x00, 0x22])` (demux/src/ogg/mod.rs:171-173) and `flac_streaminfo_rate` reads the 20-bit rate at packet offset 14 = 4 (block header) + 10 (body offset) (mod.rs:304-314). The harvest passes the whole 38-byte packet verbatim as `extra_data`; `FlacDecoder::configure` hands it to `parse_streaminfo` (decoder.rs:540), which reads `body[0..2]` as `min_block_size` — that is the block header `0x00 0x00` → `min_block_size == 0` → typed `Unsupported` `flac.streaminfo.blocksize` (metadata.rs:194-199) → `decoder_for` fails → the FLAC-in-Ogg session errors out. The comment "the FLAC decoder's 34-byte truth" states the correct contract while the code ships the wrong bytes. Contrast: the MKV harvest strips and verifies the block header correctly (`containers/mkv.rs:320-336`: checks type 0 + declared 34, extracts `private[8..42]`) and has an end-to-end decode test; the Ogg lane has NO flac test (containers/ogg.rs tests cover vorbis/opus/theora only — verified by reading :232-700).
Why it matters: FLAC-in-Ogg is one of only two shipped FLAC container lanes (native `.flac` is a stub refusal, es.rs:258-271; MP4-FLAC unverified here). Matrix row `container-ogg` (:165 "carries … FLAC-in-Ogg mappings") + `codec-audio-flac` (implemented) together claim a working FLAC-in-Ogg playback; the end-to-end lane is broken and untested. Every real-world `.ogg` FLAC file fails to decode in the shipped build.
What correct looks like: `headers.get(1)` → verify `[0x00,0x00,0x00,0x22]` prefix (or type/length generally), slice `h[4..38]` as the blob — mirroring mkv.rs `flac_streaminfo`; add an ogg-flac fixture test through the real open path (the vorbis/opus tests at ogg.rs:364/496 are the template).
Confidence: HIGH — Verification: read (three independent in-repo confirmations of the packet shape; mkv contrast; no execution possible — REQUEST-ORCH-RUN in log).

### F-002 [S3]
Where: crates/codecs/src/audio/flac/metadata.rs:133-137, 213-326, 67-82 (parse_stream, is_flac, SeekIndex::from_body)
Code:
```
pub fn parse_stream(data: &[u8]) -> Result<StreamLayout, EngineError> {
    parse_stream_inner(data).map_err(FlacError::into_engine_error)
}
```
What is wrong: The metadata-chain walk and the SEEKTABLE→`SeekIndex` conversion have no shipped caller. `parse_stream` is called only by the fuzz harness (fuzz.rs:69,107); `SeekIndex` reaches only `seek.rs`'s `SeekPlanner`, whose constructor is called only from `crates/codecs/tests/flac_seek.rs` (test) — no `apps/` or engine code constructs it (repo-wide grep). Native `.flac` files — the one format where the chain walk would be needed — are refused at open by the stub `es::open_flac` (es.rs:258-271, "the raw-FLAC elementary-stream demuxer has not landed").
Why it matters: The matrix `codec-audio-flac` notes advertise "SEEKTABLE seeking + binary-search fallback" (feature-matrix.toml:708) — neither is reachable from any shipped entry point; shipped FLAC (MKV lane) cannot seek via SEEKTABLE. ~120 lines of tested parser + the whole seek-index surface are library/dead weight in the product.
What correct looks like: Either land the native-FLAC demuxer card the stub names (which would make `parse_stream` the open-path parser) or wire the SEEKTABLE through the MKV/Ogg harvest into the engine seek path; until then mark the seek rows honest about reachability.
Confidence: HIGH — Verification: read (caller grep end to end).

### F-003 [S4]
Where: crates/codecs/src/audio/flac/metadata.rs:88-98, 182-199, 267-291
Code:
```
    /// Minimum block size in inter-channel samples (spec floor 16).
    pub min_block_size: u16,
```
What is wrong: Four spec/doc-precision gaps: (a) doc says "spec floor 16" but the validation (:194-198) only enforces `min_block_size >= 1` — STREAMINFO with min 1..=15 is accepted; (b) doc says "Sample rate in Hz (spec ceiling 65 535" (:98) — the field is 20 bits and FLAC defines rates above 65535 (e.g. 96000); the code correctly enforces no ceiling but the doc is wrong; (c) the `parse_stream` doc (:217-218) lists "a missing first STREAMINFO" as a Corrupt, but the code only rejects a chain with NO STREAMINFO anywhere — a STREAMINFO in a later position (spec violation: it must be the first block) is accepted; (d) a second SEEKTABLE silently overwrites the first (:290) while a second STREAMINFO is a typed corrupt (:269-277) — inconsistent duplicate policy; spec allows at most one of each.
Why it matters: Doc-code contradictions in a "spec-driven parser" module; tolerant acceptance of spec-violating STREAMINFO position can mask real corruption; the duplicate-seektable overwrite silently discards data.
What correct looks like: Enforce min ≥ 16; fix the rate doc; enforce STREAMINFO as block #1; type the duplicate SEEKTABLE like the duplicate STREAMINFO.
Confidence: HIGH — Verification: read (spec layout knowledge; FLAC spec §5.1 block-size floor 16 / STREAMINFO-first requirement).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| STREAM_MARKER / STREAMINFO_LEN consts | 37-41 | REAL | — | Correct values. |
| SeekPoint (struct) | 48-55 | REAL | — | Plain data; offset-relative convention documented. |
| SeekIndex::from_body | 67-81 | UNREACHABLE (shipped) | S3 | Correct 18-byte chunking + placeholder filter; `try_into().unwrap_or([0u8;8])` on a fixed-size slice is defensive dead code (always Ok); half-placeholder rows (valid sample, offset=u64::MAX) are kept — see F-002. |
| StreamInfo (struct) | 86-110 | REAL | — | Field docs have the two defects in F-003a/b. |
| StreamInfo::is_fixed_blocksize | 112-118 | REAL | — | Trivial, correct. |
| StreamLayout (struct) | 120-131 | UNREACHABLE (shipped) | — | frames_start anchor correct; only fuzz/tests consume it. |
| is_flac | 133-137 | REAL (unreachable) | — | Correct marker test. |
| parse_streaminfo | 139-153 | REAL | — | Thin taxonomy wrapper; reachable via MKV lane. |
| parse_streaminfo_inner | 155-211 | REAL | S4 | Hand-verified bit layout; guards rate/bps/blocksize; floor-16 gap (F-003a). |
| parse_stream | 213-225 | UNREACHABLE (shipped) | S3 | See F-002. |
| parse_stream_inner | 227-326 | REAL (unreachable) | S4 | Bounded walk, typed truncations, type-127 refusal, duplicate-STREAMINFO corrupt; STREAMINFO-position and duplicate-SEEKTABLE laxness (F-003c/d); no infinite-loop or overflow path (pos strictly grows; 24-bit lengths). |
| read_u24 | 328-332 | REAL | — | Safe (fixed-size callers post-length-check). |
| tests ×6 | 334-471 | REAL tests | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| probe_accepts_only_the_marker | YES | Exact true/false cases incl. 3-byte prefix. |
| streaminfo_parses_the_packed_field | YES | Every field asserted exactly (packed math hand-verified). |
| streaminfo_rejects_short_bodies_and_bad_geometry | YES | Error codes + kinds asserted (short body, rate 0). |
| seek_index_drops_placeholders_and_keeps_points | YES | Placeholder filtered, order/values asserted. |
| parse_stream_walks_blocks_and_finds_frames_start | YES | frames_start asserted to the byte; seek point count asserted. |
| parse_stream_rejects_magic_and_truncation_typed | YES | magic + header_trunc + body_trunc codes asserted. |
| Coverage gap (not a test failure): `flac.metadata.duplicate_streaminfo`, `flac.metadata.block_type` (127), `flac.metadata.seektable_length`, `flac.metadata.streaminfo_missing`, and the never-last-flag exhaustion path have NO test — five typed branches unexercised (mutation: deleting any of these error arms would not fail the suite). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "SEEKTABLE seeking + binary-search fallback" | feature-matrix.toml:708 (codec-audio-flac notes) | UNREACHABLE | No shipped caller constructs SeekPlanner/SeekIndex (F-002). |
| "metadata blocks … surfaced through our own metadata.rs" | docs/formats/audio-flac.md:20-22 | SHALLOW | Parse exists+tested; only STREAMINFO (34 bytes) is consumed by the shipped lane; the chain walk is fuzz-only. |
| "zero panics" (module doc) | metadata.rs:8 | REAL | All accesses bounds-checked (verified by reading every index site). |
| "seek points for seek" as the parse side's purpose | metadata.rs:9-13 | UNREACHABLE | F-002. |
| "duplicate STREAMINFO blocks are typed Corrupts" | metadata.rs:26-28 | REAL (untested) | Code :269-277; no test covers it (§5 gap). |
| FLAC-in-Ogg carried by the ogg container (implemented) | feature-matrix.toml:165 | FALSE (end-to-end) | F-001: demux yes, decode broken at the harvest seam. |

## 7. Dead / duplicated / unreachable code
- `parse_stream`, `parse_stream_inner`, `is_flac`, `SeekIndex::from_body`, `StreamLayout` (shipped-dead, F-002).
- Cross-crate duplication: the STREAMINFO rate extraction exists three times with three different shapes — `flac_streaminfo_rate` (demux/ogg/mod.rs:307, block-header-prefixed read), `flac_streaminfo` strip (containers/mkv.rs:320, fLaC-prefixed), and this file's `parse_streaminfo_inner` (bare body) — the exact confusion that produced F-001.
- `try_into().unwrap_or([0u8;8])` defensive dead code (:74-75).

## 8. Dependencies & flags
- `codec-flac` default ON; claxon used only by decoder.rs (not this file — this file is dependency-free hand-rolled parsing, as documented).
- No unsafe, no allows. Std + media-types error taxonomy only.

## 9. Open questions for the next agent
- containers/ogg.rs auditor: F-001 lives in your file — the 38-vs-34-byte blob; confirm and rate against the ogg container row.
- Is FLAC-in-MP4 (esds/`flac` SampleEntry) wired anywhere in mp4.rs? (My grep found no flac arm in containers/mp4.rs — if absent, MKV is the ONLY shipped FLAC lane that works.)
- Engine seek auditor: does any engine-level seek for the MKV-flac lane exist at all (i.e., is FLAC seekable in any shipped lane, binary-search or otherwise)?
