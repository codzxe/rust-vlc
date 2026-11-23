# AUDIT: crates/codecs/src/audio/mp3/id3.rs
| Field | Value |
|---|---|
| Auditor | W07-A06 |
| Date | 2026-11-23 |
| Lines / functions | 621 / 12 (incl. 2 test helpers + 8 tests) |
| File verdict | UNREACHABLE (the parser itself is REAL quality; no shipped entry point calls it) |
| Reachable from shipped entry point? | NO — full call graph below |
| Findings | S0: 0 S1: 0 S2: 0 S3: 4 S4: 2 |
| Commands run | prebuilt `./target/debug/deps/codecs-a0d7a93b7072ff58 mp3::id3` -> 8 passed; rg call-graph traces over crates/ apps/ |

## 1. One-paragraph verdict
This is a genuine, hand-rolled ID3v2.2/2.3/2.4 parser (no symphonia here): syncsafe sizes, v2.2 6-byte frames, v2.3 plain sizes + tag-global unsynchronisation, v2.4 syncsafe frame sizes + per-frame unsync, footer accounting, typed errors on every out-of-bounds claim, and no panic path I could find (every index goes through `get`/bounds-checked slices). The 8 unit tests pass (I ran them). BUT: it is dead weight in the shipped product. The shipped MP3 lane (lumen CLI `open` → containers/es.rs → demux::es::mp3::RawMp3Demuxer) uses the demuxer's OWN separate ID3 skip (`skip_id3`, crates/demux/src/es/mp3.rs:352, with its own MAX_ID3_BYTES ceiling) and explicitly drops APIC cover art; the contract Mp3Decoder::decode is packet-level and never parses ID3. The only callers of `parse_id3v2`/`id3v2_len` are the free `decode_stream` surfaces (mp3/decoder.rs:193, mp2/decoder.rs:131, mp2/header.rs:307 `is_mp2_stream` — itself called by nothing outside the codecs crate), fuzz harnesses, and test suites. "The player wants the text frames (title/artist/…) and the cover-art blobs" (L5-6) describes a consumer that does not exist.

## 2. What actually works (verified)
- `./target/debug/deps/codecs-a0d7a93b7072ff58 mp3::id3` -> `test result: ok. 8 passed` — v2.3 parse/skip, oversized-tag typed error, v2.4 footer+syncsafe, v2.3 global unsync reversal, v2.2 3-byte ids + APIC parse, bad encoding byte typed, no-magic None, frame-past-end typed.
- Bounds discipline read-verified end to end: `id3v2_len` rejects `total > data.len()` (L228); frame walk rejects `end > body.len()` (L325); extended-header skip clamps `pos = (pos + skipped).min(body.len())` (L297); `parse_picture` uses only `get`/clamped arithmetic — no `[i]` indexing on unchecked values.
- Baseline corroborates: cargo-test-workspace.txt (mp3 suite) — bitexact/corpus/fuzz all ok.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/mp3/id3.rs:344-347
Code:
```
        let compressed_23 = version.0 == 3 && frame_flags & 0x0080 != 0;
        let compressed_24 = version.0 >= 4 && frame_flags & 0x08 != 0;
        let encrypted = version.0 >= 3 && frame_flags & 0x04 != 0;
        if compressed_23 || compressed_24 || encrypted {
```
What is wrong: the encryption bit is checked as `0x04` for BOTH v2.3 and v2.4. Per ID3v2.3.0 the frame format-flags byte is `%abc00000` — compression bit 7 (0x80, correctly checked on the line above), encryption bit 6 (0x40), grouping bit 5; bit 2 is undefined in v2.3. So a v2.3 encrypted frame (0x40 set) is NOT refused — it falls through to `decode_text`/`Other` and its ciphertext payload is kept as tag data — while a v2.3 frame that merely has undefined bit 2 set is wrongly refused as "encrypted". 0x04 is the correct bit only for v2.4. Why it matters: the module's stated contract ("compressed or encrypted are refused, not guessed", L343) is half-false for v2.3. What correct looks like: `let encrypted = (version.0 == 3 && frame_flags & 0x40 != 0) || (version.0 >= 4 && frame_flags & 0x04 != 0);`. Confidence: HIGH — Verification: read (bit assignment is stable spec text); no test covers either branch.
F-002 [S3]
Where: crates/codecs/src/audio/mp3/id3.rs:270-275
Code:
```
    // De-unsynchronise the body (≤2.3: tag-global; 2.4: per frame).
    let body: Vec<u8> = if unsynchronised && version.0 < 4 {
        deunsynchronise(&data[10..body_end])
    } else {
        data[10..body_end].to_vec()
    };
```
What is wrong: for a v2.4 tag whose HEADER unsynchronisation flag is set, no de-unsynchronisation is applied unless each frame separately sets its own unsync flag (L339-341). A v2.4 tag written with tag-level unsynchronisation (the header flag is still defined in v2.4; taggers that write it globally exist) parses with `FF 00` pairs intact: text frames keep spurious NULs (mojibake) and the frame walk can desync after the first frame because stored payloads are longer than their logical sizes. Why it matters: wrong metadata on a legal flag combination; the in-file comment asserts "2.4: per frame" as if the tag-level flag had no v2.4 meaning. What correct looks like: for v2.4 with the tag flag set, either de-unsynchronise each frame payload unconditionally or refuse typed (documented interop choice). Confidence: MEDIUM — Verification: read; spec text not available in sandbox (UNVERIFIED against the official ID3v2.4.0 formal.txt — flag semantics recalled from the spec's §5).
F-003 [S3]
Where: crates/codecs/src/audio/mp3/id3.rs:336-341 (context 309-322)
Code:
```
        let mut payload = &body[start..end];
        // v2.4 per-frame unsynchronisation.
        let deunsynced;
        if version.0 >= 4 && frame_flags & 0x02 != 0 {
            deunsynced = deunsynchronise(payload);
            payload = &deunsynced;
```
What is wrong: the v2.4 data-length-indicator flag (format-flags bit 0, 0x01) is never inspected. When set, the frame payload begins with a 4-byte syncsafe length that this parser leaves inside the payload — every text value from such a frame is prefixed with 4 garbage bytes (`TIT2` becomes e.g. "\0\0\0\x0cTitle"). Compressed/encrypted frames are refused, but DLI frames (legal, e.g. written by some encoders alongside unsync) are silently mis-decoded. What correct looks like: if `frame_flags & 0x01 != 0`, strip the leading 4 bytes after de-unsynchronisation. Confidence: MEDIUM — Verification: read; no fixture in the tree exercises a DLI frame.
F-004 [S3]
Where: crates/codecs/src/audio/mp3/id3.rs:122-181 and 278-298 (no test references)
Code:
```
        1 => {
            // UTF-16 with BOM.
            let (le, body) = match rest.get(..2) {
                Some([0xFF, 0xFE]) => (true, &rest[2..]),
```
What is wrong: the UTF-16 encodings (enc byte 1 with/without BOM and enc byte 2 UTF-16BE) and the extended-header skip (both the v2.3 and v2.4 arms) have ZERO test coverage. All three flag-handling bugs above (F-001..F-003) live precisely in these untested branches. The 8 tests exercise only latin-1/UTF-8 text, no extended header, and unsync only at v2.3 tag level. Why it matters: test honesty for the hard paths is nil — a regression in `body_utf16be` or the BOM sniff would pass the suite. Confidence: HIGH — Verification: read + ran the suite (filtered list confirms the 8 names).
F-005 [S3]
Where: crates/codecs/src/audio/mp3/id3.rs:4-6 (module doc)
Code:
```
//! ID3v2 tag. The demuxer primarily needs its **total length** to
//! skip to the first audio frame; the player wants the text frames
//! (title/artist/…) and the cover-art blobs. This module parses the
```
What is wrong: no "player" ever receives these frames. Shipped call graph: lumen CLI → apps/cli/src/containers/es.rs:73 `use demux::es::mp3::RawMp3Demuxer` → RawMp3Demuxer::skip_id3 (crates/demux/src/es/mp3.rs:352) — a SECOND, independent ID3 length-walk that does not call this module and skips APIC entirely ("An ID3v2 `APIC` (cover art) frame is metadata this demuxer skips", demux/es/mp3.rs:39-41). The contract Mp3Decoder::decode (mp3/decoder.rs:502) is packet-level. parse_id3v2's only callers: mp3/decoder.rs:193 `decode_stream` (free/test surface), mp2/decoder.rs:131 `decode_stream`, mp2/header.rs:307 `is_mp2_stream` (no callers outside the codecs crate), mp3/fuzz.rs:96, tests. Two ID3 parsers in one tree + a doc that describes a consumer that isn't wired = dead-code duplication with a misleading rationale. Confidence: HIGH — Verification: rg call-graph (commands listed above).
F-006 [S4]
Where: crates/codecs/src/audio/mp3/id3.rs:215-225
Code:
```
    let Some(body) = syncsafe32(&head[6..10]) else {
        return Err(corrupt(
            "mp3.id3.size_field",
```
What is wrong: `head` is exactly 10 bytes (checked at L193), so `syncsafe32(&head[6..10])` always receives 4 bytes and the `mp3.id3.size_field` error branch is unreachable dead code (same for `syncsafe32(&raw).unwrap_or(0)` at L317 — `raw` is `[u8; 4]`). What correct looks like: delete the branch or debug_assert. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Id3Tag::text | 80-85 | REAL | — | First-match lookup; fine |
| syncsafe32 | 90-101 | REAL | — | Masks high bits (spec-violation tolerant, documented) |
| deunsynchronise | 105-117 | REAL | — | Correct FF 00→FF; tail-FF handled; O(n) |
| decode_text | 122-172 | REAL | S3 | enc 0/1/2/3 handled; UTF-16 arms untested (F-004); empty payload → "" (lenient, fine) |
| body_utf16be | 174-181 | REAL | S3 | Untested (F-004) |
| id3v2_len | 189-244 | REAL | S4 | Magic→None, truncated header typed, version gate 2..=4, `total > data.len()` typed (tested); dead else-branch F-006 |
| parse_id3v2 | 254-374 | REAL | S3 | Frame walk bounds-checked; v2.3 unsync global + v2.4 per-frame (F-002); encryption bit bug (F-001); DLI ignored (F-003); ext-header skip clamped |
| parse_picture | 377-422 | REAL | — | All accesses via `get`/min-clamps; encoding-aware description terminator; no panic path found |
| tests ×8 + 2 helpers | 424-621 | REAL | S3 | Good typed-error coverage; untested arms per F-004 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| skips_and_parses_a_v23_tag | YES | Asserts len/total_len/version/text value |
| a_tag_claiming_more_than_the_file_is_typed_truncation | YES | Asserts exact error code |
| v24_footer_and_syncsafe_frame_sizes | YES | Footer counted in len; TALB value asserted |
| unsynchronisation_is_reversed | YES | FF 00→FF logical value asserted through the real v2.3 path |
| v22_three_char_ids_and_picture_frames | YES | TT2 value + APIC mime/type/bytes asserted |
| bad_text_encoding_is_typed | YES | enc 9 → exact code |
| no_magic_is_none_not_error | YES | Ok(None) + header_truncated code |
| a_frame_running_past_the_tag_is_typed | YES | Corrupted size → exact code |
| (missing) UTF-16 / ext-header / encryption / DLI / v2.4-tag-unsync | — | No test would catch F-001..F-004 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the player wants the text frames (title/artist/…) and the cover-art blobs" | id3.rs:5-6 | FALSE (no such consumer wired) | Call graph in F-005; demuxer skips APIC (demux/es/mp3.rs:39) |
| "Error paths (each typed...): a tag claiming more bytes than the buffer holds, a frame running past the tag body, a bad text encoding byte, a truncated syncsafe size field" | id3.rs:20-23 | PARTLY FALSE | First three typed+tested; "truncated syncsafe size field" is unreachable dead code (F-006) |
| "applied tag-globally in ≤2.3 and per-frame in 2.4" | id3.rs:16-17 | SHALLOW | Implemented as stated, but the v2.4 tag-level flag is thereby ignored (F-002) |
| Matrix row "Demux raw MPEG audio elementary streams + ID3v2/VBRI/Xing metadata" | docs/feature-matrix.toml:288 | REAL (demux-side, not this file) | Evidence is the demuxer's own skip; this module is not the shipped lane |

## 7. Dead / duplicated / unreachable code
The entire module is unreachable from shipped binaries (F-005): duplicated responsibility with demux::es::mp3::skip_id3 (two independent syncsafe/length walks to keep in sync). `Id3Frame::Other` payloads are collected but no shipped consumer reads them. `is_mp2_stream` (the only non-test route to `id3v2_len`) has no callers outside the codecs crate.

## 8. Dependencies & flags
No external deps. Part of the `mp3` module tree (feature `codec-mp3`, default-on) and compiled into every shipped build — it ships as dead weight, not as a feature-gated leftover.

## 9. Open questions for the next agent
- Audit demux::es::mp3::skip_id3 (not my file) for the same class of bugs — it is the parser that actually runs; does IT handle v2.4 tag-unsync, footers, inter-frame tags correctly?
- If MP3 metadata (title/artist/cover) is a product feature, the wiring is missing end-to-end (demuxer drops it); this module should either be wired into the demuxer or deleted.
- Confirm ID3v2.4.0 formal.txt §5 flag semantics for F-002 (no network in sandbox).
