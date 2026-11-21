# AUDIT: crates/playlists/src/xspf.rs#part1 (lines 1-518)
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-11-21 |
| Lines / functions | 518 (of 1037; boundary: `impl Scanner` begins 515, `Scanner::new` body 516-523 straddles the line-518 cut; all other Scanner methods + helpers + Frame/FrameKind/FieldKind/TrackBuilder/apply_field/close_frame are part2/A07) / 12 items |
| File verdict | REAL (parser, in-crate-tested) — UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO. Proof: the only production consumer is `crates/streaming/src/remote_playlist.rs:194` (`playlists::parse_xspf_bytes(&response.body)`), and remote_playlist itself has zero callers outside streaming (rg `remote_playlist\|RemotePlaylist` apps/ crates/ → streaming-internal only); apps/cli uses only streaming::{fetch,ranged,schemes,mockhttp} (demux_host.rs rg). The playlists crate ships inside `lumen` only as streaming's compile-time dep. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "^enum \|^struct \|^fn ..." xspf.rs` (item map; chunk split at Scanner) · read parse_detailed (272-475) line-by-line with hand-traced interleavings (nested track skip, unclosed fields, dup fields, name-collision end tags) · cross-read part2 helpers apply_field/close_frame/track_entry (838-925) to verify forced-close salvage semantics · rg xspf callers (remote_playlist.rs:194 only) · `rg "MAX_.*BYTES\|TooLarge" crates/playlists/src/*.rs` (only mpd has a total-size cap) · tests/xspf_suite.rs read 189-267 + test list (12 tests + random-mutation test) · fixtures basic.xspf/namespaced.xspf exist · baseline #3 (suite passes) |

## 1. One-paragraph verdict
A genuinely careful hand-rolled XSPF pull-parser state machine — salvage
semantics that actually hold up under hand-traced hostile interleavings
(forced-closed fields still apply their text; nested same-name elements
resolve by topmost-name matching; the inner-track skip works), with an
honest warning taxonomy that never embeds content, and a strong 12-test
suite including a random-mutation pass. It is also dead code in the
shipped binary: its only consumer is the streaming crate's remote-
playlist loader, which no app reaches. Within the dead code: the
inner-track skip counter is structural rather than name-aware, so one
stray end tag inside a nested track prematurely ends the skip and
corrupts the outer track's field attribution (no panic, warnings
emitted, doubly-dirty input required); a self-closing `<location/>`
silently yields an empty location and suppresses the
TrackWithoutLocation note (inconsistent with `<duration/>`'s explicit
absent handling); and unlike the MPD parser there is no total-input
bound, only the 64 KiB line cap.

## 2. What actually works (verified)
— Salvage contract verified by hand-tracing: `<track><title>abc</track>`
(unclosed field, parent's end tag) pops the Field with forced=true, and
close_frame (part2:848-852) still applies the accumulated text — the
D-2 promise "a <track> left open … still yields its entry" holds for
fields too, with an UnclosedElement warning.
— Duplicate-field first-wins verified: is_set flips at apply time
(part2:878/882), dup frames don't accumulate text (443-447) and aren't
applied on close — `<title>a</title><title>b</title>` keeps "a" with one
DuplicateField warning.
— Nested same-name element resolution: `<title>a<title>b</title></title>`
— the inner <title> becomes an Other frame named "title" (328-334);
`</title>` matches the TOPMOST name (rposition at 421), popping only the
Other; the outer Field survives to its own close. Traced, correct.
— Inner-track skip for the CLEAN case traced correct: `<track>…<track>
…</track>…</track>` — skip_depth counts 1 at the inner track, +1/−1 per
nested start/end, the inner `</track>` returns the counter to 0, the
outer `</track>` then closes the outer track normally (pinned by
tests/xspf_suite.rs:209-222, which asserts exactly one NestedTrack
warning and the outer entry intact).
— EOF salvage (463-472) pops all frames forced — traced through the
unclosed-inner-track-at-EOF case; outer entry survives.
— Warning discipline: no XspfWarning variant carries element text,
locations, or titles (lines 105-108 doc claim verified against all 17
variants, 110-255).
— Duration conversion: XSPF milliseconds → shared-vocabulary seconds
(ms/1000.0 at part2:899), negative/NaN/inf refused with BadDuration
(part2:897-901) — unit correct.
— Suite: crates/playlists/tests/xspf_suite.rs — 12 named tests + a
SplitMix64 random-input no-panic test; fixtures exist; passed in
baseline #3.

## 3. Findings

### F-001 [S3]
Where: crates/playlists/src/xspf.rs:267-482 (the entire public parse surface)
Code:
```rust
pub fn parse(input: &str) -> Result<Playlist, XspfError> {
    Ok(parse_detailed(input)?.playlist)
}
```
What is wrong: Nothing in the code — the parser is unreachable. The only
production caller is streaming's remote_playlist.rs:194, itself
referenced by nothing outside the streaming crate; the `lumen` CLI's
streaming usage (demux_host.rs) touches only fetch/ranged/schemes/
mockhttp. Same posture as the MPD parser (my mpd.rs#part5 F-002):
compiled into the shipped binary via the streaming→playlists dep edge,
never invoked.
Why it matters: 518 lines of state machine (part1) plus the scanner
(part2) and a 480-line test suite protect a path no user can trigger;
any doc claiming XSPF playlist open works in the shipped binary would
be false on reachability alone.
What correct looks like: wire remote_playlist into an entry point (the
streaming client session), or accept it as library surface with the
deadness recorded.
Confidence: HIGH — Verification: read (call chains grepped end-to-end).

### F-002 [S4]
Where: crates/playlists/src/xspf.rs:413-417 (End arm), with the skip entry at 338-343
Code:
```rust
            Token::End { line, name } => {
                if skip_depth > 0 {
                    skip_depth -= 1;
                    continue;
                }
```
What is wrong: The skip counter decrements on EVERY end token,
regardless of name. A stray end tag inside a skipped inner track
consumes the skip budget: `<track><location>outer</location><track>
<title>t</title></stray></track><title>outer2</title></track>` —
`</stray>` (matching nothing) decrements skip_depth 1→0; the inner
`</track>` then finds the OUTER Track frame and closes it prematurely;
`outer2` becomes a playlist-level title (ignored) and the final
`</track>` is UnexpectedEndTag. The outer entry loses its `outer2`
title and the parse is silently restructured (though every step emits a
warning: NestedTrack, UnexpectedEndTag).
Why it matters: Doubly-dirty input (nested track + stray end tag) —
never a panic, entries still emitted — but the skip mechanism is
name-blind where the rest of the end-tag handling (rposition matching)
is name-aware. No test covers a stray end tag inside a skipped track
(xspf_suite.rs:209-222 covers only the clean nested case).
What correct looks like: Track the skipped region's open element names
(a small Vec of names, or reuse the Frame stack with a skip flag) so an
end token only exits the skip when it actually matches.
Confidence: HIGH — Verification: read (interleaving hand-traced through
the state machine at 338-343, 413-417, 421-436).
### F-003 [S4]
Where: crates/playlists/src/xspf.rs:352-356 (self-closing field), semantics from apply_field (part2:876-882, 892-896)
Code:
```rust
                        if self_closing {
                            if !dup {
                                apply_field(kind, "", &mut current_track, &mut warnings, line);
                            }
                            continue;
                        }
```
What is wrong: `<location/>` inside a track applies an empty string:
apply_field (part2:876-878) sets `tb.location = Some("")` and
`location_set = true`, so track_entry (part2:908-910) suppresses the
TrackWithoutLocation warning — the entry ships with a silently empty,
unusable location. Contrast the sibling fields: `<duration/>` is an
explicit no-op ("the field is simply absent", part2:894-895) and
`<title/>` yields None via non_empty. Only location keeps the empty
value AND suppresses the note that exists precisely to flag unusable
locations (its doc: "track with no location element (kept, empty
location)").
Why it matters: Hand-edited or generator-broken playlists with
`<location/>` produce entries that look parsed but can never open;
inconsistent with both the duration precedent and the warning's purpose.
What correct looks like: Treat an empty location element like an absent
one (warn TrackWithoutLocation, location stays ""), matching duration's
handling.
Confidence: HIGH — Verification: read (both code paths compared).
### F-004 [S4]
Where: crates/playlists/src/xspf.rs:41-45 (the D-1 bounds doc), 272-282 (preflight)
Code:
```rust
//! * Bounded input (D-1): nesting deeper than [`MAX_XML_DEPTH`] elements
//!   is a fatal [`XspfError::TooDeep`], and any line over the crate's
//!   64 KiB line cap (counted on raw bytes between newlines) is a fatal
```
What is wrong: The bounds are line-cap + depth-cap only — there is no
total-input ceiling, unlike the sibling MPD parser's MAX_MPD_BYTES
(8 MiB, justified as "matching the fetch port's default response
budget", mpd.rs:57-58). A multi-gigabyte XSPF of short lines passes the
preflight and is parsed (and retained) in full.
Why it matters: D-1 "bounded input" is applied inconsistently across the
crate's four parsers; remote loads are presumably bounded upstream by
the fetch budget, but parse_bytes accepts any local/caller buffer. No
doc claim is broken (the file never promises a total cap) — this is a
doctrine gap, and the unbounded-entry Vec is file-controlled allocation.
What correct looks like: A MAX_XSPF_BYTES aligned with the fetch budget
(typed TooLarge), or a documented statement that the caller owns the
total bound.
Confidence: HIGH — Verification: read (grep of all *BYTES caps in the
crate: only mpd has one).

## 4. Per-function table (items in lines 1-518)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-58 | REAL | — | Format model, dirty-world and mapping claims all verified against code/traces; M3U bridge asymmetry claim pinned by test xspf_bridges_through_m3u_with_documented_asymmetry. |
| imports | 60-61 | REAL | — | MAX_LINE_BYTES, Entry/HlsSegment/Playlist — all used. |
| `MAX_XML_DEPTH` | 63-66 | REAL | — | 64; enforced at 302; suite pins fatal at 65 (tests:280). |
| `XspfError` (3 variants) | 68-103 | REAL | — | LineTooLong/InvalidUtf8/TooDeep — only D-1 bounds fatal, matches doc. |
| `XspfWarning` (17 variants) | 105-255 | REAL | — | All line-number-only payloads; no content embedded (MP §12 claim holds). |
| `XspfParsed` | 257-265 | REAL | — | Data. |
| `parse` | 267-270 | REAL / UNREACHABLE | — | Warning-discarding wrapper. |
| `parse_detailed` | 272-475 | REAL / UNREACHABLE | S4 | The state machine: BOM strip, line preflight, Start/End/Text arms, EOF salvage. Hand-traced ~10 hostile interleavings; F-002 (skip blindness) and F-003 (self-closing location call site) are the only defects found; depth guard counts stack only (skipped regions push nothing — no stack growth, no guard bypass). |
| `parse_bytes` | 268→479-482 | REAL / UNREACHABLE | — | UTF-8 gate typed; F-004 (no total cap). |
| `Token` enum | 486-503 | REAL | — | Start/End/Text; local names, entity-decoded text per doc (scanner in part2). |
| `Scanner` struct | 505-513 | REAL | — | Data (src/pos/line/warnings). |
| `Scanner::new` (boundary) | 515-523 | REAL | — | Trivial constructor; straddles the 518 cut — rest of Scanner is A07's part2. |

## 5. Test honesty (suite: crates/playlists/tests/xspf_suite.rs — for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| golden_basic_entities_and_fields (31) | YES | Fixture-driven equality on all five fields incl. entity decodes (fixture read). |
| golden_namespaced_prefixes_cdata_and_subsets (71) | YES | Prefix-stripped local names, CDATA, tolerated subsets. |
| bom_and_xml_declaration_tolerance (114) | YES | BOM strip (275) and PI skip. |
| empty_and_markup_only_inputs (125) | YES | Empty/comment-only → Ok empty playlist (no panic). |
| dirty_fixture_salvages_entries (146) | YES | Salvage semantics pinned on entries. |
| prefix_mismatched_end_tags_and_stray_text (189) | YES | Exact warning VECTOR asserted (order + kinds) + entry fields. |
| nested_track_is_skipped_with_note (209) | YES | Single NestedTrack warning + outer entry intact — pins the clean skip path (not F-002's stray-tag case). |
| malformed_markup_degrades_not_dies (223) | YES | Five damage kinds asserted ≥5 + entry fields incl. unterminated-CDATA swallow. |
| garbage_bytes_are_typed_invalid_utf8 (268) | YES | parse_bytes typed error. |
| depth_guard_is_fatal_at_65 (279) | YES | 65-deep input → TooDeep. |
| over_cap_line_is_a_typed_error (308) | YES | LineTooLong with exact length. |
| xspf_bridges_through_m3u_with_documented_asymmetry (335) | YES | Round-trip bridge pins the creator/annotation directive mapping (54-58 doc claim). |
| random no-panic test (401+) | NO (beyond panics) | Stated no-panic harness; honest. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Never panics on any input (fuzz-pinned)" | xspf.rs:36 | REAL (in-crate) | Random-mutation test + fuzz.rs target; baseline #3 pass. Not verifiable against shipped reachability (F-001). |
| "unclosed elements are salvaged — … still yields its entry" | xspf.rs:37-39 | REAL | close_frame forced-path applies field text (848-852) — traced. |
| "Extra occurrences of a field are damage (DuplicateField, first value wins)" | xspf.rs:14-15 | REAL | is_set/!dup logic traced (346-367, 443-447). |
| "duration (milliseconds in XSPF)→Entry::duration in seconds" | xspf.rs:49-50 | REAL | ms/1000.0 (part2:899). |
| "no … locations or titles are ever embedded" in warnings | xspf.rs:105-108 | REAL | All 17 variants carry only line numbers. |
| "Bounded input (D-1)" | xspf.rs:41-45 | SHALLOW | Line+depth caps real; no total-input bound (F-004) — the claim never says total, but the crate's own MPD parser shows the intended doctrine. |

## 7. Dead / duplicated / unreachable code
The whole parser is unreachable from shipped entry points (F-001).
Within the file: no dead items found; every warning variant is
constructible and most are asserted by the suite.

## 8. Dependencies & flags
Crate-internal only (m3u::MAX_LINE_BYTES, types::{Entry, HlsSegment,
Playlist}); thiserror used for the two error enums (matches the crate's
style — note mpd.rs is thiserror-free by contrast; cosmetic divergence,
not a defect). No unsafe, no I/O, no feature gates.

## 9. Open questions for the next agent
- A07 (xspf.rs#part2): scan_start_tag/scan_end_tag own local-name
  stripping and the `</>` degenerate handling — verify the End name
  really is prefix-stripped before rposition matching (the suite's
  `</xspf:location>` case says yes, but the code is in your range).
- Whoever owns streaming/remote_playlist.rs: it discards warnings
  (`.map_err(|_| PARSE_REFUSED)?` keeps only the count) — confirm the
  warning count is actually surfaced anywhere user-visible.
- The skip_depth fix (F-002) would add a small Vec of skipped names —
  check part2's Scanner for an existing suitable structure before
  designing one.
