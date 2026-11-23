# AUDIT: crates/demux/src/asf/guid.rs
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-23 |
| Lines / functions | 271 / 27 (6 Guid methods, 3 reader fns, 15 consts, 3 tests) |
| File verdict | UNREACHABLE (real, byte-verified code; no shipped entry point reaches the asf crate) |
| Reachable from shipped entry point? | NO — apps/cli/src/containers/mod.rs:50-75 has no `"asf"` arm (15 ids: adts aiff avi flac h264 hevc ivf matroska mp2 mp3 mp4 ogg mpegps mpegts wav); media-probe registers no ASF signature (rg `asf|wmv|wma` in crates/media-probe/src: zero hits); `AsfDemuxer` referenced only inside crates/demux/src/asf/* (rg across crates/ apps/). Independently re-derived; consistent with prior asf/mod.rs + asf/header.rs audits. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `cargo test -p demux --lib guid` (exit 0, asf::guid 3/3 ok); python3 fixture byte-scan over all 9 asf fixtures (exit 0, output below); rg caller sweeps (exit 0); sed quote checks (exit 0) |

## 1. One-paragraph verdict
The GUID table is mostly genuine and I byte-verified it: HEADER, DATA,
SIMPLE_INDEX, STREAM_TYPE_AUDIO/VIDEO, EC_AUDIO_SPREAD all match real
fixture bytes, and the little-endian readers are exercised by passing
tests. One doc claim is FALSE and I disproved it by execution: `EC_NONE`
is documented as "the video SPs' observed type field is the all-zero GUID
in ffmpeg output", but every video Stream Properties object in all 9
fixtures carries `20FB5700-5B55-11CF-A8FD-00805F5C442B` (the actual ASF
no-error-correction GUID) — the all-zero GUID matches nothing in the
corpus. Because `Guid::read` returns all-zeros on a short buffer, the only
value that can ever equal `EC_NONE` is a truncated read. The whole asf
family is unreachable from shipped entry points (probe never names asf;
CLI registry has no arm), so none of this is user-visible today.

## 2. What actually works (verified)
— `cargo test -p demux --lib guid` → `header_guid_round_trips_from_wire_bytes`, `known_table_covers_the_verified_set`, `readers_decode_little_endian_and_tolerate_short_buffers` all ok.
— python3 byte-scan (executed): HEADER GUID at offset 0 in 8/9 fixtures; DATA object present in 9/9; SIMPLE_INDEX present exactly in the 4 video fixtures (doc: "video fixtures only" TRUE); all 5 audio SPs carry EC_AUDIO_SPREAD bytes `50 CD C3 BF 8F 61 CF 11 8B B2 00 AA B4 E2 20 C0` (doc: "observed on every audio SP" TRUE); all 5 video SPs carry `00 57 FB 20 55 5B CF 11 A8 FD 00 80 5F 5C 44 2B` ≠ all-zeros (doc FALSE, F-001).

## 3. Findings
F-001 [S3]
Where: crates/demux/src/asf/guid.rs:168-170
Code: (verbatim)
```
/// No error correction type (the video SPs' observed type field is
/// the all-zero GUID in ffmpeg output).
pub const EC_NONE: Guid = Guid([0u8; 16]);
```
What is wrong: the "observed" claim is false. Executed byte-scan of every video SP in crates/demux/src/asf/fixtures/*.wm{v,a}: error-correction type field = `20FB5700-5B55-11CF-A8FD-00805F5C442B` in all 5 (asf_pkt8000, asf_stream_broadcast, wmv_wmv1_64x48, wmv_wmv2_64x48, wmv_wmv2_wma_av), never all-zeros. This contradicts the module's own honesty banner (guid.rs:11-14: "the GUIDs marked *observed* were byte-verified against ffmpeg 7.1.5 output this session"). Why it matters: (a) a fabricated observation in the one table whose stated virtue is observation-vs-recall honesty; (b) `Guid::read` on a buffer shorter than 32 bytes yields all-zeros (guid.rs:36-48), so EC_NONE is reachable ONLY through truncation — a future consumer matching it would misclassify short reads as "no error correction"; (c) `known_name` on a real video SP's EC GUID returns None (the real no-EC GUID is not in the table). What correct looks like: `EC_NONE = Guid([0x00,0x57,0xFB,0x20,0x55,0x5B,0xCF,0x11,0xA8,0xFD,0x00,0x80,0x5F,0x5C,0x44,0x2B])` and zeros treated as unknown. Confidence: HIGH — Verification: ran (python3 fixture scan).

F-002 [S3]
Where: crates/demux/src/asf/guid.rs:95-96 (against header.rs:244,730,768 and 629)
Code: (verbatim)
```
            EC_AUDIO_SPREAD => Some("error-correction-audio-spread"),
            EC_NONE => Some("error-correction-none"),
```
What is wrong: both EC constants and `CONTENT_DESCRIPTION` (guid.rs:88) are dead vocabulary in production: the only production `known_name()` call is header.rs:629 on top-level header children, and no code path ever passes an SP-inner GUID (stream type / EC type) to `known_name`. `StreamRecord.error_correction_type` is parsed (header.rs:730) and stored (header.rs:768) but never read by any consumer (rg `error_correction_type`: 3 hits, all in header.rs, none a use). The audio-spread packet layout is never consulted. Why it matters: the names exist only in tests; a reader of the table believes error-correction types are recognized, but the recognition is unreachable. What correct looks like: either consume error_correction_type where packets are framed or delete the field + the two constants. Confidence: HIGH — Verification: ran (rg sweeps).

F-003 [S4]
Where: crates/demux/src/asf/guid.rs:184-189 (vs 36-38)
Code: (verbatim)
```
pub fn u16le(data: &[u8], offset: usize) -> u16 {
    if offset + 2 > data.len() {
        return 0;
    }
    u16::from_le_bytes([data[offset], data[offset + 1]])
}
```
What is wrong: inconsistent overflow hardening — `Guid::read` uses `offset.saturating_add(16)` (guid.rs:38) while u16le/u32le/u64le use plain `offset + N`, which wraps/panics on 32-bit targets (wasm32 is a shipped target per AGENT-SPEC baseline) for offsets near usize::MAX. Current callers keep offsets bounded (verified the codec-list walk header.rs:862-899 clamps `off` and `byte_len` against `payload.len()`), so this is latent, not live. Confidence: MEDIUM — Verification: read (caller discipline verified; no 32-bit build run — cargo-build-budget).

F-004 [S4]
Where: crates/demux/src/asf/guid.rs:41-45
Code: (verbatim)
```
            let got = &data[offset..end];
            let mut i = 0;
            while i < got.len() {
                bytes[i] = got[i];
                i += 1;
            }
```
What is wrong: hand-rolled byte-copy loop where `bytes[..got.len()].copy_from_slice(got)` is the whole body; `render()` likewise builds 6 intermediate allocations per call (guid.rs:64-74). Cold paths only (skip reasons), so hygiene. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Guid::from_bytes | 29-31 | REAL | — | Tests-only callers (guid.rs:227,253); pub API otherwise unused |
| Guid::read | 36-48 | REAL | — | 7 production call sites (mod.rs:97, header.rs:478,480,553,584,729,730); truncation sentinel collides with EC_NONE (F-001) |
| Guid::as_bytes | 52-54 | REAL | — | Callers: tests.rs:84,88,147,318 + fuzz.rs:65,73 only |
| Guid::render | 59-75 | REAL | — | Verified: wire `30 26 B2 75...` → `75B22630-668E-11CF-A6D9-00AA0062CE6C` (test + fixture scan); 6 allocs/call (F-004) |
| Guid::known_name | 80-99 | REAL | — | One production call (header.rs:629); EC/CONTENT_DESCRIPTION rows unreachable (F-002) |
| Display | 102-106 | REAL | — | Delegates to render |
| u16le | 184-189 | REAL | — | Plain-add guard (F-003); ~15 header.rs call sites |
| u32le | 193-203 | REAL | — | Same; ~15 call sites |
| u64le | 207-214 | REAL | — | Same; ~10 call sites |
| 15 consts | 110-178 | REAL | — | 12 byte-verified against fixtures; EC_NONE wrong (F-001); EC_AUDIO_SPREAD/CONTENT_DESCRIPTION dead-in-practice (F-002) |
| tests: header_guid_round_trips | 221-231 | REAL | — | Fails on any HEADER byte/render/name change |
| tests: known_table_covers_the_verified_set | 234-256 | REAL | — | Honest split: asserts only the 12 observed rows — and thereby never tests EC_NONE, which is exactly how F-001 survived |
| tests: readers_decode_little_endian | 259-269 | REAL | — | Covers LE order + all short-buffer zero paths |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| header_guid_round_trips_from_wire_bytes | YES | Byte-exact equality + exact render string |
| known_table_covers_the_verified_set | YES (for the 12 rows) | Exact name match per GUID + unknown-GUID None/EE-render case |
| readers_decode_little_endian_and_tolerate_short_buffers | YES | 0x5678/0x12345678/0x90ABCDEF12345678 order pins + 4 short-buffer zeros |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "GUIDs marked *observed* were byte-verified against ffmpeg 7.1.5 output" | guid.rs:11-14 | FALSE (for EC_NONE) | Executed fixture scan: video SP EC field is 20FB5700-5B55-11CF-A8FD-00805F5C442B, never zeros (F-001) |
| Header GUID on-disk bytes + rendering | guid.rs:7-9 | REAL | Fixture scan: bytes at offset 0 in 8/9 (broadcast fixture is a test-prefixed variant); test asserts render |
| "Simple Index ... video fixtures only, after Data" | guid.rs:117-118 | REAL | Scan: SIMPLE_INDEX bytes present in exactly the 4 video fixtures, 0 audio-only ones |
| "Audio-spread ... observed on every audio SP" | guid.rs:164-165 | REAL | Scan: all 5 audio SPs match EC_AUDIO_SPREAD bytes |
| "[FORMAT KNOWLEDGE] ... never gate parsing (every unknown GUID is skipped by size)" | guid.rs:14-17 | REAL | header.rs:614-633: default arm skips by size; known_name only picks the reason string |
| "recognized so such streams skip as a known family" (command) | guid.rs:159-160 | REAL | header.rs:591 `if stream_type == STREAM_TYPE_COMMAND` picks the skip reason |

## 7. Dead / duplicated / unreachable code
- Dead-in-practice: EC_AUDIO_SPREAD, EC_NONE, CONTENT_DESCRIPTION constants (F-002); StreamRecord.error_correction_type field (header.rs-owned, noted here because guid.rs mints its values).
- Whole file unreachable from shipped entry points (probe/registry gaps, see header table).
- No duplication found within the crate.

## 8. Dependencies & flags
- std only (std::fmt). No unsafe. No feature gates. No dependencies.

## 9. Open questions for the next agent
- If asf ever gets a probe signature + CLI arm, F-001's EC_NONE must be fixed first (it inverts truncation into "no error correction").
- Does the SP-declared audio-spread layout change data-packet framing in real (non-ffmpeg) files? data.rs:130-133 parses only the packet-level EC flags byte; the SP-level declaration is ignored — a conformance question for whoever owns data.rs.
