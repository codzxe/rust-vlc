# AUDIT: crates/codecs/src/audio/vorbis/comments.rs
| Field | Value |
|---|---|
| Auditor | W07-A04 |
| Date | 2026-11-23 |
| Lines / functions | 379 / 13 (+4 tests, 1 helper) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — CLI "vorbis" arm (apps/cli/src/pipeline_host.rs:270-273) → `VorbisDecoder::configure` (crates/codecs/src/audio/vorbis/decoder.rs:693-714) → `VorbisHeaders::from_extra_data` (headers.rs:220-224) → `header_len` + `parse` on every Vorbis open |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p codecs --lib audio::vorbis::comments` → 4 passed; `cargo test -p codecs --test vorbis_bitexact` → 5 passed (0.64s) |

## 1. One-paragraph verdict
A correct, panic-free Vorbis comment parser: little-endian u32 lengths, 16 MiB per-field ceiling, count clamped against the remaining buffer so a lying count cannot loop or over-allocate, strict UTF-8, framing-bit check, case-insensitive ASCII field lookup with case-preserved keys and preserved tag multiplicity — all pinned by unit tests plus a fixture test that decodes an ffmpeg-authored tagged .ogg and asserts vendor/TITLE/ARTIST/ALBUM/DATE exactly (executed: PASS). The one real defect is economic, not mathematical: the parsed comments are consumed by NOTHING in production — the only reader of `.comments` is a test (crates/codecs/tests/vorbis_bitexact.rs:183-195); no app or metadata surface ever sees TITLE/ARTIST, so every Vorbis open pays the parse and allocation for metadata that is parsed and dropped.

## 2. What actually works (verified)
— `cargo test -p codecs --lib audio::vorbis::comments` → 4 passed; `cargo test -p codecs --test vorbis_bitexact` → 5 passed (`fixtures_are_pinned_against_the_ffmpeg_reference` asserts the tagged fixture's five fields exactly as ffmpeg wrote them).

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/audio/vorbis/comments.rs:1-2 vs crates/codecs/tests/vorbis_bitexact.rs:183-195
Code:
```
//! Vorbis comment header (card 5.B2's metadata exposure): the
//! `vendor` string + the `key=value` user-comment list (spec §5.2,
```
What is wrong: the module claims to be the "metadata exposure", but the parsed `VorbisComments` has zero production consumers — `rg "\.comments" crates apps` yields only the bitexact test; `title()/artists()/album()/date()` are called by no shipped code. Every Vorbis open (configure lane) parses the comment header, allocates a `String` per key and value, and the result is stored on `VorbisHeaders` and never read again. What correct looks like: either surface the comments through the demux/track-facts seam (the claimed 5.R3 registry) or defer parsing until a consumer exists. Confidence: HIGH — Verification: read + rg.
F-002 [S4]
Where: crates/codecs/src/audio/vorbis/comments.rs:221-229
Code:
```
    if len > MAX_ONE_COMMENT {
        return Err(corrupt(
            "vorbis.comment.truncated",
            media_types::error::CorruptDetail::Truncated {
                expected: MAX_ONE_COMMENT as u64,
                found: len as u64,
            },
```
What is wrong: an over-ceiling length claim is reported as `Truncated { expected: 16 MiB, found: len }` — the kind and the expected/found pair are semantically wrong (nothing was truncated; the claim `len` exceeds the ceiling `MAX_ONE_COMMENT`, so expected/found are inverted and the code should be an absurd-claim kind, not `truncated`). Cosmetic because it is typed and logged, but the detail misleads triage. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| VorbisComments (struct) | 40-47 | REAL | — | data |
| get | 52-58 | REAL | — | case-folded first match |
| get_all | 62-69 | REAL | — | multiplicity preserved (test-pinned) |
| title / artists / album / date | 71-93 | REAL | S3 | production-dead accessors (F-001) |
| eq_ignore_ascii_case | 98-103 | REAL | — | len check then byte-fold; correct |
| header_len | 111-140 | REAL | — | measures chain split; clamps count same as body; requires framing byte to exist (value not checked here — parse checks it) |
| parse | 149-159 | REAL | — | magic + delegate |
| parse_comment_body | 167-205 | REAL | — | clamp defeats lying counts; framing bit enforced |
| read_len | 208-211 | REAL | — | LE u32; fits usize on 32/64-bit |
| read_slice | 215-245 | REAL | S4 | ceiling error kind inverted (F-002) |
| decode_utf8 | 248-257 | REAL | — | strict UTF-8, typed |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| comments_roundtrip_with_case_insensitive_lookup | YES | exact vendor/title/artist, case-preserved key, folded lookup |
| repeatable_tags_keep_multiplicity | YES | two ARTIST tags kept in order |
| damage_is_typed_never_panicking | YES | truncated / framing-clear / wrong magic / no-'=' / lying-count all typed (the clamp's adversarial case) |
| body_length_measurement_feeds_the_chain_split | YES | header_len == full length |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Nothing panics; every length is bounds-checked before use" (L23-25) | comments.rs | REAL | damage test + fuzz pass (vorbis/fuzz.rs runs the comment parser over fuzzed bytes, L9) |
| "metadata exposure" (L1) | comments.rs | FALSE (exposure does not exist) | F-001: no production consumer |
| "a future FLAC reuse is a two-line composition" (L10-11) | comments.rs | REAL-UNPROVEN | `parse_comment_body` is field-generic as claimed; no FLAC caller exists today (rg parse_comment_body → only this module) |

## 7. Dead / duplicated / unreachable code
`title/artists/album/date` accessors and the stored comments: test-only (F-001). No duplication found.

## 8. Dependencies & flags
super::error (VorbisError/corrupt), media-types. No feature gates; reached via the shipped "vorbis" decoder arm unconditionally.

## 9. Open questions for the next agent
- 5.R3 metadata registry: does any card actually surface track tags to the UI/CLI today (e.g. mkv title does — demux/src/mkv), and is Vorbis comment exposure on a roadmap card or dead weight?
- headers.rs auditor: does `from_extra_data` bound the total chain (IDENT + comment + setup) so the 16 MiB-per-field ceiling cannot be stacked (4 × vendor/comments/setup) beyond the container's own header cap?
