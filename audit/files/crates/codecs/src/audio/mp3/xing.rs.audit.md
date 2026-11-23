# AUDIT: crates/codecs/src/audio/mp3/xing.rs
| Field | Value |
|---|---|
| Auditor | W07-A04 |
| Date | 2026-11-23 |
| Lines / functions | 578 / 9 (+5 tests, 2 helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — `parse_xing`/`parse_vbri` are called only by `decode_stream` (crates/codecs/src/audio/mp3/decoder.rs:213-215), a pub conformance/bench surface no app calls (`rg decode_stream apps/` → 0 hits); the shipped `.mp3` open lane uses `Mp3Decoder::decode` → `decode_one` (decoder.rs:502) which never parses Xing, and the demuxer does its OWN header-frame skip (crates/demux/src/es/mp3.rs:595-632) |
| Findings | S0: 0 S1: 1 S2: 0 S3: 2 S4: 2 |
| Commands run | `cargo test -p codecs --lib audio::mp3::xing` → 5 passed (0.01s); `cargo test -p codecs --test mp3_bitexact` → 2 passed (1.10s) |

## 1. One-paragraph verdict
Genuine, spec-correct Xing/Info/LAME/VBRI parsing with honest typed truncation errors — and one real panic bug: `parse_vbri` validates only 14 bytes of tail then indexes bytes 14..21, so a crafted or truncated frame with a `VBRI` magic and 14–21 trailing bytes panics with index-out-of-bounds. The math that matters (LAME 12+12-bit delay/padding at tag offsets 21..24, `trim_window` = [delay, total−padding), frames/bytes/TOC/quality flag-gated fields, side-info-derived ancillary offset) is correct and pinned by fixture tests that decode recorded ffmpeg 7.1 references exactly (mp3_bitexact, executed: PASS). The file is library-surface code the shipped CLI never calls: the production packet decoder skips Xing handling entirely (the container lane's decoded count for mono48.mp3 is 73,199 vs ffmpeg's 72,000 — the end-padding trim is documented as not unified through that lane), and the demuxer reimplements header-frame detection instead of calling this module.

## 2. What actually works (verified)
— `cargo test -p codecs --lib audio::mp3::xing` → 5 passed; `cargo test -p codecs --test mp3_bitexact` → 2 passed (fixtures decode the recorded ffmpeg window exactly; live-ffmpeg ±1 LSB check skipped, no ffmpeg in sandbox).

## 3. Findings
F-001 [S1]
Where: crates/codecs/src/audio/mp3/xing.rs:339-367
Code:
```
    let rest = frame.get(pos + 4..).unwrap_or(&[]);
    if rest.len() < 14 {
        return Err(corrupt(
            "mp3.vbri.truncated",
```
```
    let toc_entries = u16::from_be_bytes([rest[14], rest[15]]);
    let toc_scale = u16::from_be_bytes([rest[16], rest[17]]);
    let toc_entry_size = u16::from_be_bytes([rest[18], rest[19]]);
    let toc_frames_per_entry = u16::from_be_bytes([rest[20], rest[21]]);
```
What is wrong: the guard admits `rest.len()` 14..21, but lines 364-367 index `rest[14]` through `rest[21]` directly — any complete first frame whose `VBRI` magic sits such that 14–21 bytes follow it panics (slice index out of bounds). E.g. a 96-byte MPEG-1 L3 32 kbps/48 kHz stereo frame (frame_size = 144·32000/48000 = 96, side info 32, scan window 36..96) with `VBRI` at offset 78 leaves exactly 14 tail bytes → `rest[14]` panics. The module's own contract ("Damage is never a panic", decoder.rs:185-186) is violated. Why it matters: `parse_vbri` is a pub fn of the codecs crate; `decode_stream` is exported (mod.rs:67) and is the advertised stream-level pipeline — any embedder (or a fuzz target; note crates/codecs/src/audio/mp3/fuzz.rs:110 fuzzes `parse_xing` only, never `parse_vbri`) hits a reachable panic on malformed input. The shipped CLI open lane does not call it (verified: no app references `decode_stream`), which is the only reason this is not a shipped-lane DoS. What correct looks like: `if rest.len() < 22` (the fixed VBRI fields are 22 bytes past the magic), or `rest.get(14..22)?`. The error's `expected: pos + 18` (line 344) is also wrong — the fixed fields end at `pos + 4 + 22`. Confidence: HIGH — Verification: read (panic window arithmetic; not executed — no test can be added under read-only rules). Confidence on non-reachability of the shipped lane: HIGH (`rg -n "decode_stream" apps/` → no matches).

F-002 [S3]
Where: crates/codecs/src/audio/mp3/xing.rs:140-153 + crates/demux/src/es/mp3.rs:595-632
Code:
```
    let Some(pos) = find_magic(frame, base, b"Xing").or_else(|| find_magic(frame, base, b"Info"))
    else {
        return Ok(None);
    };
```
What is wrong: duplication of the Xing/Info/VBRI detection — the demuxer re-implements magic detection (`&bytes[xing_at..xing_at + 4] == b"Xing" ... &bytes[36..40] == b"VBRI"`, demux/src/es/mp3.rs:629-632) with its own offset logic instead of calling this module, so the two implementations can drift (one tolerates a 64-byte scan window, the other only the exact offsets). What correct looks like: one shared parser used by both. Confidence: HIGH — Verification: read.
F-003 [S3]
Where: crates/codecs/src/audio/mp3/xing.rs:1-46 (module doc) vs decoder.rs:213-215
Code:
```
//! Total frame count, when the flag bit was set.
```
(claim-bearing docs; see What is wrong)
What is wrong: the module doc presents this as the VBR seek index support ("the VBR seek index ([`seek`](super::seek))"), but the shipped decode lane never parses any of it — `Mp3Decoder::decode` (decoder.rs:502-530, the path the CLI provider uses, apps/cli/src/pipeline_host.rs:261-265) calls `decode_one` only; gapless/Xing facts flow exclusively through the unconsumed `decode_stream`. Users opening a VBR .mp3 in `lumen` get no Xing-derived duration/seek and no end-padding trim (container-lane decoded count 73,199 vs ffmpeg 72,000 for mono48.mp3 — apps/cli/src/containers/es.rs:388-402, documented honestly there). What correct looks like: wire `decode_stream`'s Xing facts (or at least the LAME trim) into the packet decoder. Confidence: HIGH — Verification: read + executed container test count claim from es.rs docs (not re-run: cli crate build over budget; cite prior audit of es.rs).
F-004 [S4]
Where: crates/codecs/src/audio/mp3/xing.rs:368
Code:
```
    let toc_len = usize::from(toc_entries) * usize::from(toc_entry_size.max(1));
```
What is wrong: a header claiming `toc_entry_size = 0` is silently reinterpreted as 1 — fabricated geometry instead of a typed refusal or honest skip. Confidence: HIGH — Verification: read.
F-005 [S4]
Where: crates/codecs/src/audio/mp3/xing.rs:293-296
Code:
```
    let window = frame.get(base..(base + 64 + 4).min(frame.len()))?;
    window.windows(4).position(|w| w == magic).map(|p| base + p)
```
What is wrong: tolerant 64-byte scan accepts a Xing/Info magic embedded in side-info/audio data as a header (false-positive surface); standard practice is tolerant, but the accepted window is generous and unmatched by the demuxer's strict-offset check (F-002 drift). Confidence: MEDIUM — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| XingInfo | 54-74 | REAL | — | data; frame_offset/frame_end recorded |
| LameTag | 77-84 | REAL | — | data |
| LameTag::trim_window | 86-95 | REAL | — | [delay, total−padding), saturating clamp; verified by test + ffmpeg-reference fixture test |
| VbriInfo | 98-118 | REAL | — | data |
| ancillary_offset | 123-126 | REAL | — | 4+CRC+side_info; matches spec (36/21/21/13) vs header.rs:458-470 |
| parse_xing | 140-287 | REAL | — | flag-gated fields, typed truncations, zero-frames typed; executed |
| parse_head | 289-291 | REAL | — | thin wrapper |
| find_magic | 293-296 | REAL | S4 | 64-byte tolerant scan (F-005) |
| parse_lame | 305-315 | REAL | — | 12+12 bits at 21..24; verified vs ffmpeg fixtures |
| parse_vbri | 323-401 | REAL | **S1** | panic window rest.len() ∈ [14,21] (F-001) |
| find_magic_vbri | 403-409 | REAL | — | 40-byte window; covers the spec's fixed offset 36 in all channel-mode cases |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| parses_frames_bytes_toc_quality_and_lame | YES | exact field values, toc[50]==128, delay/padding 1105/340, trim_window exact |
| info_magic_is_cbr_flagged | YES | is_vbr false, frames Some(50), toc None, frame_offset 4096 |
| no_xing_is_none_and_zero_frames_is_typed | YES | Ok(None) + mp3.xing.zero_frames + flags_truncated codes |
| truncated_field_claims_are_typed | YES | toc_truncated on 50-byte cut |
| hand_crafted_vbri_header_parses | YES (partial) | fields + zero_totals + toc_truncated asserted — but truncation only from the TOC tail; the 14..21-byte rest window (F-001) is untested and would panic |
Missing: no test for rest.len() ∈ [14,21] (the panic window); no LAME `Lavc` magic test despite the code accepting it.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "verified empirically on ffmpeg 7.1-authored fixtures … equals ffmpeg's decoded sample count exactly" (L298-304) | xing.rs | REAL | mp3_bitexact.rs `fixtures_decode_the_recorded_window_exactly` PASS (executed) |
| "No known offline encoder in the sandbox produces [VBRI] … unit-tested against hand-crafted headers" (L43-46) | xing.rs | REAL | only hand-crafted VBRI tests exist; fixture dir has no VBRI file |
| "Damage is never a panic" (decoder.rs:185) | decoder.rs | FALSE for parse_vbri | F-001 panic window |

## 7. Dead / duplicated / unreachable code
`parse_vbri`'s output (`VbriInfo`) is consumed only by `decode_stream` → `DecodedMp3.vbri` — which no shipped app reads. Xing/LAME duplication vs demux/src/es/mp3.rs (F-002).

## 8. Dependencies & flags
Uses super::error (corrupt/unsupported), super::header (FrameHeader, side_info_size), media-types errors. No feature gates. MP3 decoder wired in shipped CLI provider ("mp3" arm, apps/cli/src/pipeline_host.rs:261) — but this module's entry (`decode_stream`) is not on that lane.

## 9. Open questions for the next agent
- demux/src/es/mp3.rs auditor: does the demuxer's own Xing/VBRI skip (L595-632) bounds-check before slicing `bytes[xing_at..]`/`bytes[36..40]` on short frames?
- Who consumes `DecodedMp3.xing/.vbri` (duration/seek facts) in any real composition? If nobody, VBR seek for shipped .mp3 open is effectively CBR-walk-based only — check seek.rs audit (this wave) for the shipped-lane seek story.
