# AUDIT: crates/playlists/src/hls.rs
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-23 |
| Lines / functions | 997 / 24 (7 production fns, 3 error impls, 2 type families, 14 tests) |
| File verdict | UNREACHABLE (real, tested parser; its only production consumers are the streaming HLS sessions, themselves unwired) |
| Reachable from shipped entry point? | NO — `rg parse_media_playlist|parse_master_playlist` production callers: crates/streaming/src/hls/media.rs, master.rs (live.rs via `MediaPlaylist`), which have no shipped entry point (see live.rs audit header). The reachable-from-CLI playlist surface (`streaming::remote_playlist`) uses `parse_bytes`/`parse_xspf_bytes` (M3U/XSPF/PLS), NOT this module. Test-only callers: crates/playlists/tests/{hls_attrs,m3u_suite}.rs. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 5 |

## 1. One-paragraph verdict
A competent, well-tested recording parser: media playlists typed with first-occurrence-wins promoted tags, RFC 3986 URI resolution, dirty-world warnings instead of guesses, and a master-playlist parser with a strict attribute splitter (duplicate names, unterminated quotes, trailing commas all reject the row). The 14 unit tests are honest and pass (I ran them). The defects are at the edges: the `#EXTM3U` sniff runs on the raw text and a leading UTF-8 BOM — which the underlying M3U parser explicitly tolerates — turns a real CDN playlist into a typed `not-hls` refusal; `#EXT-X-DISCONTINUITY-SEQUENCE` (and PROGRAM-DATE-TIME, GAP, I-FRAME-STREAM-INF, INSTREAM-ID) are dropped entirely rather than recorded, despite the module's "records, does not refuse" motto; and one warning enum variant is dead code whose doc promises behavior that does not exist. Like the live session, none of this is reachable from the shipped binary.

## 2. What actually works (verified)
— `cargo test -p playlists --lib hls` → `15 passed; 0 failed; 0 ignored; 31 filtered out` (14 of these are this module's tests). Promoted-tag typing, URI resolution shapes (`../a.ts`, `/b.ts`, absolute), NotHls/master/media mirror refusals, BadTagValue for `three`/`2.5s`/`-1`, recorded KEY IV hex bytes, BYTERANGE/MAP/DISCONTINUITY recording, malformed-master row skipping (4 exact warnings) — all asserted exactly; they would fail if the typing logic broke.

## 3. Findings
F-001 [S3]
Where: crates/playlists/src/hls.rs:222-223 (media) and :458 (master)
Code:
```
    let parsed = parse_detailed(text).map_err(|source| HlsPlaylistError::Parse { source })?;
    let saw_extm3u = text.lines().any(|line| line.trim() == "#EXTM3U");
```
What is wrong: the `#EXTM3U` magic scan runs on the RAW `text`. `parse_detailed` tolerates and skips exactly one leading U+FEFF (crates/playlists/src/m3u.rs:120 "BOM tolerance: exactly one leading U+FEFF"), but `str::trim()` does not strip U+FEFF (not `is_whitespace`), so a BOM-prefixed playlist — which the crate's own m3u.rs:7 doc says is "tolerated and skipped" — fails the magic scan and refuses `playlists.hls.not-hls` in BOTH `parse_media_playlist` and `parse_master_playlist`. Why it matters: real CDNs/encoders occasionally emit BOM'd playlists; VLC plays them; this crate's documented tolerance is defeated at the layer that decides HLS-ness. What correct looks like: strip the leading BOM from `text` before the scan (or reuse a parse_detailed-supplied flag).
Confidence: HIGH — Verification: read (string-compare logic; U+FEFF `is_whitespace()==false`). Not executed (no BOM test exists in the crate — confirmed by rg).

F-002 [S3]
Where: crates/playlists/src/hls.rs:1-42 (module contract) vs the model (54-108)
Code:
```
//! Everything else stays *recorded, not resolved*: `#EXT-X-KEY`
//! presence, `BYTERANGE`, `MAP`, `DISCONTINUITY` ride on the
```
What is wrong: the module's stated contract is "recording: it keeps `#EXT-X-*` tags … and stamps the segment-level subset on entries" — but only four segment-level tags survive (KEY/MAP/BYTERANGE/DISCONTINUITY). `#EXT-X-DISCONTINUITY-SEQUENCE` (RFC 8216 §4.3.3.3 — the live client's discontinuity-count continuity anchor), `#EXT-X-PROGRAM-DATE-TIME`, `#EXT-X-GAP`, and `#EXT-X-BITRATE` are dropped with no field, no warning, no directive echo (rg `discontinuity_sequence|PROGRAM-DATE|EXT-X-GAP` in this crate → 0 model hits). Why it matters: the live session (live.rs) cannot implement DISCONTINUITY-SEQUENCE tracking because the data never leaves this parser; the matrix note "EXT-X-GAP tolerated (RFC 8216 4.1 ignore)" is accurate only in the sense that the tag vanishes. What correct looks like: at minimum a `discontinuity_sequence: Option<u64>` field and a `gap: bool` per segment.
Confidence: HIGH — Verification: read + rg.

F-003 [S4]
Where: crates/playlists/src/hls.rs:437-443
Code:
```
    /// A well-formed attribute value this model does not type (e.g.
    /// a non-`WxH` `RESOLUTION`): the attribute is dropped, the row
    /// survives.
    BadAttributeValue,
```
What is wrong: `MasterWarningKind::BadAttributeValue` is never constructed anywhere (rg `BadAttributeValue` → single hit: this definition). A non-`WxH` RESOLUTION in `variant_from_attrs` (:547-553) silently yields `resolution: None` with no warning — the doc claims "the attribute is dropped, the row survives" with a warning kind that the code never emits. Why it matters: dead enum variant + a documented warning that does not exist. What correct looks like: emit the warning in `variant_from_attrs` (which needs the line number threaded in) or delete the variant.
Confidence: HIGH — Verification: read + rg (1 hit total).

F-004 [S4]
Where: crates/playlists/src/hls.rs:512-523
Code:
```
        if let Some((line_no, attrs)) = pending_stream_inf.take() {
            match variant_from_attrs(&attrs, line, base) {
                Some(v) => master.variants.push(v),
                None => warnings.push(MasterWarning {
```
What is wrong: a `#EXT-X-STREAM-INF` row whose URI line never arrives (truncated playlist, trailing tag) leaves `pending_stream_inf` set at loop end and is dropped WITHOUT a `MasterWarning`, though the function doc (:450-453) says "Damaged rows are warnings, not errors". Why it matters: silent data loss contradicting the stated posture; a caller cannot distinguish "1-variant master" from "2-variant master truncated mid-transfer". What correct looks like: after the loop, if `pending_stream_inf.is_some()` push `VariantWithoutBandwidth` (or a dedicated kind) with the stored line number.
Confidence: HIGH — Verification: read + control-flow trace.

F-005 [S4]
Where: crates/playlists/src/hls.rs:570-593 (rendition_from_attrs)
Code:
```
    Some(Rendition {
        kind,
        group_id: find("GROUP-ID").cloned(),
        name: find("NAME").cloned(),
        language: find("LANGUAGE").cloned(),
```
What is wrong: `Rendition` drops `INSTREAM-ID` (REQUIRED for CLOSED-CAPTIONS per RFC 8216 §4.3.4.1 — the test at :908 even feeds `INSTREAM-ID="CC1"` and the row parses, but the value is discarded), plus `FORCED`, `CHANNELS`, and `CHARACTERISTICS`. A closed-captions rendition without its INSTREAM-ID is unidentifiable downstream. Why it matters: the "records" model loses REQUIRED data the consumer needs. What correct looks like: an `instream_id: Option<String>` (and ideally CHANNELS for audio) field.
Confidence: HIGH — Verification: read + test at :906-921 which proves parse-and-discard.

F-006 [S4]
Where: crates/playlists/src/hls.rs:526-528
Code:
```
    if master.variants.is_empty() && master.renditions.is_empty() && saw_media_shape {
        return Err(HlsPlaylistError::MediaPlaylist);
    }
```
What is wrong: the media-shape detector recognizes only `#EXTINF`/`#EXT-X-TARGETDURATION` prefixes (:506). A bare playlist (`#EXTM3U\ns.ts\n` — a degenerate but legal media playlist with no EXTINF, or an EXTINF-less live window from a sloppy packager) parses as an Ok MASTER with zero variants and zero renditions; the "is this even a master?" question is pushed to the caller. `streaming::hls::master::open_master` does refuse the empty shape (its own lane), so the blast radius is a confusing cross-crate refusal code. What correct looks like: an explicit empty-master refusal here, or any-URI-line → media shape.
Confidence: MEDIUM — Verification: read; downstream refusal not re-verified in this file's audit.

F-007 [S3→ downgraded S4 boundary note, recorded as S3 per unsure-higher rule]
Where: whole file (reachability)
Code:
```
//! * [`parse_media_playlist`] — parse + validate + resolve in one
```
What is wrong: unreachable from the shipped binary (see header). The parser is real and tested, but a user cannot get an HLS playlist into it through `lumen open`: the CLI's http lane probes bytes, never parses playlists. Same class as live.rs F-001: real code, no shipped entry.
Confidence: HIGH — Verification: rg call-chain.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HlsPlaylistError::code | 150-158 | REAL | — | stable dotted codes |
| Display for HlsPlaylistError | 161-192 | REAL | — | no URIs/paths embedded (MP §12) |
| Error::source for HlsPlaylistError | 195-202 | REAL | — | parse error chained |
| parse_media_playlist | 218-224 | REAL | S3 | F-001 BOM sniff on raw text |
| media_playlist_from_parsed | 231-323 | REAL | — | first-occurrence-wins promotion; empty window allowed (segments=[]) — live reload relies on this |
| parse_tag_uint | 327-333 | REAL | — | strict decimal-integer, u64::MAX accepted (see live.rs F-008) |
| parse_master_playlist | 454-530 | REAL | S4 | F-004 dangling row silent; F-006 weak media-shape sniff; strict attr splitter |
| variant_from_attrs | 534-566 | REAL | S4 | F-003 silent attribute drop |
| rendition_from_attrs | 570-593 | REAL | S4 | F-005 INSTREAM-ID lost |
| split_attr_list | 599-628 | REAL | — | duplicates/trailing comma/unterminated quote/trailing junk all None; RFC 8216 §4.2 conformant |
| 14 test fns | 648-996 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| vod_playlist_types_and_resolves | YES | exact typed fields + resolved URIs + 1e-9 duration sum |
| no_base_keeps_locations_verbatim | YES | verbatim vs resolved |
| uri_resolution_shapes | YES | three RFC 3986 shapes, exact URLs |
| plain_m3u_refuses_not_hls | YES | error variant + code |
| master_playlist_refuses_typed | YES | STREAM-INF and EXT-X-MEDIA both decisive; EXT-X-MEDIA-SEQUENCE not (negative case) |
| bad_numeric_values_refuse_typed | YES | 3 tags × junk values |
| model_records_not_refuses_b1b_shapes | YES | live/encrypted/byterange/map/discontinuity recorded incl. exact IV bytes |
| dirty_world_warnings_flow_through | YES | junk EXTINF → duration None + ParseWarning |
| ffmpeg_fixture_ts_vod_types_fully | YES | committed fixture, 5 segments, exact first/last URIs |
| master_types_variants_and_renditions | YES | exact bandwidth/avg/program-id/codecs/resolution/rendition fields |
| master_single_variant_and_uri_less_rendition | YES | URI-less CC rendition → None |
| master_malformed_rows_skip_with_warnings | YES | 4 exact warnings, 1 surviving variant |
| master_refuses_media_shape_and_non_hls | YES | mirror refusals |
| master_fixture_files_type | YES | 3 committed fixtures, counts + warnings |
No tautologies. Note: fixtures used here (ts_vod/pl.m3u8, master/*.m3u8) are themselves audited in this wave (pl1.ts/pl2.m4s under my assignment).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A leading UTF-8 BOM is tolerated and skipped" (M3U layer) | crates/playlists/src/m3u.rs:7 | FALSE at the HLS layer | F-001: hls.rs:222-223/458 raw-text scan |
| "Damaged rows are warnings, not errors" (master) | hls.rs:450-453 | SHALLOW | F-004: dangling STREAM-INF row silently dropped |
| "records, does not refuse" (B1b+ shapes) | hls.rs:23-28 | SHALLOW | F-002: DISCONTINUITY-SEQUENCE/GAP/PROGRAM-DATE-TIME not recorded at all |
| "each attribute appears at most once" enforced | hls.rs:596-598 | REAL | split_attr_list:608-610 + malformed fixture test |

## 7. Dead / duplicated / unreachable code
- `MasterWarningKind::BadAttributeValue` — never constructed (F-003).
- Whole module unreachable from shipped binary (F-007).
- `HlsPlaylistError::MediaPlaylist` is only reachable for EXTINF/TARGETDURATION-bearing inputs (F-006 shapes fall through to Ok-empty).

## 8. Dependencies & flags
Depends only on crate-local `m3u` (parse_detailed, resolve_uri), `error::ParseWarning`, `types::{ByteRange, HlsKey, HlsMap}`. No features, no I/O. Clean layering.

## 9. Open questions for the next agent
- Does `streaming::hls::master::open_master` refuse the empty-master fallthrough (F-006)? (Its own audit lane.)
- The m3u_suite.rs (my file 4) — do its BOM tests cover the HLS typing path or only parse_detailed?
