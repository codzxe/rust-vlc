# AUDIT: crates/playlists/src/mpd.rs
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-15 |
| Lines / functions | 2854 / 51 non-test + 16 tests |
| File verdict | REAL (parser core) — but UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO — only `streaming::dash::*` consumes it; no app reaches `streaming::dash` |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `cargo test -p playlists --lib mpd::` → 16 passed/0 failed (exit 0); rg callers; sed quote verification |

## 1. One-paragraph verdict
This is a genuine, hand-rolled, std-only MPD (XML) pull parser with a typed model,
real bounds (8 MiB total, 64 KiB/line, depth 64), single-pass entity decoding
(structurally immune to billion-laughs), fatal-vs-warn tolerance policy, and 16
honest tests that all pass (I ran them). It does NO SegmentTemplate math by design
(templates stay verbatim; resolution is `streaming/src/dash/resolve.rs`). The fatal
problem is product-level: nothing shipped can reach it. The CLI's `open` lane for
`http(s)` goes through `RangedResource` (progressive byte stream) → `media_probe`
→ container registry; `media-probe` has no MPD/XML signature at all, so an MPD
URL refuses at the probe stage. `streaming::dash` (the only consumer of
`parse_mpd`) is referenced by no app: only `apps/cli` depends on `streaming`, and
it touches only `fetch`, `ranged`, `schemes`, `mockhttp`.

## 2. What actually works (verified)
— verified by `cargo test -p playlists --lib mpd::` → 16 passed; 0 failed
(includes `fuzz_lite_mutations_never_panic`: 600 deterministic
truncate/flip/splice/window mutations of the real ffmpeg fixture, no panic).
Bounds verified by reading L652-661 (size + line cap) and L1023-1031 (depth).
Baseline corroborates: audit/runs/baseline/cargo-test-workspace.txt:4644-4673.

## 3. Findings
F-001 [S2]
Where: crates/playlists/src/mpd.rs:652 (whole module) + apps/cli/src/demux_host.rs:391-458
Code:
```
pub fn parse_mpd(text: &str) -> Result<MpdParsed, MpdError> {
    if text.len() > MAX_MPD_BYTES {
        return Err(MpdError::TooLarge {
```
What is wrong: The parser is real but dead in the shipped product. Call-chain
trace: `parse_mpd_bytes` is called only from `crates/streaming/src/dash/{resolve,sched,abr,fuzz}.rs`;
`streaming::dash` is not referenced by `apps/cli`, `apps/desktop`,
`apps/desktop-tauri` (rg over apps/: zero hits), `playback-engine`, or `commands`.
The shipped open path is `apps/cli/src/demux_host.rs::open_source` →
`open_http_source` → `streaming::schemes::SchemeRegistry::open_http` →
`RangedResource` → `media_probe::probe_stream` → `crate::containers::open`;
`media-probe` has no MPD/XML signature (rg -i "mpd|xml|dash" over
crates/media-probe/src: zero hits), so an MPD URL dies at the probe stage as
unknown signature. docs/feature-matrix.toml network-dash row still claims
status windows/linux/macos = "partial" with this file as evidence — the parse
core exists but no user-visible DASH playback path exists. Why it matters: matrix
readers believe DASH playback is partially working; the reachable truth is
"refuses at probe". What correct looks like: wire `streaming::dash` session into
the open path (MPD content-type/signature → session), or downgrade the row.
Confidence: HIGH — Verification: read (rg call-chain, sed quote of open path).

F-002 [S3]
Where: crates/playlists/src/mpd.rs:1354-1404 vs 1654-1661
Code:
```
/// First-match attribute lookup (duplicate attribute names are not
/// legal XML; the first occurrence wins, documented).
fn attr<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
```
What is wrong: The documented rule is "first occurrence wins", but the
`apply_*_attrs` functions iterate ALL attributes in a for-loop and assign, so for
duplicate (illegal-XML) attribute names the LAST occurrence wins there
(`<MPD type="static" type="dynamic">` yields `Dynamic`), while `attr()`-based
lookups (`SegmentURL@media`, `Initialization@sourceURL`, `S@t/d/r`) are
first-wins. Two different duplicate policies in one parser, and the comment at
1654 is false for the apply_* path. Blast radius: illegal XML only. What correct
looks like: one policy (spec-tolerant first-wins), documented once, applied
everywhere. Confidence: HIGH — Verification: read (both code paths);
mutation-reasoned, not executed (cargo-build-budget for a scratch test crate).

F-003 [S3]
Where: crates/playlists/src/mpd.rs:41-53 (module docs) and 1996-1998
Code:
```
/// The crate's D-1 line cap, applied to raw bytes between newlines
/// (the `xspf` rule; a copy of its helper — see module docs).
fn line_over_cap(input: &str) -> Option<(usize, usize)> {
```
What is wrong: Third copy of the same discipline: `line_over_cap` is duplicated
verbatim from `xspf.rs:951`, and the whole Scanner is a "sibling copy" of the
XSPF scanner (self-recorded follow-up, module docs L43-53). Three XML-ish parsers
now carry three scanner lineages. Why it matters: a scanner bug fix (e.g. the
resync rules) must be applied N times; divergence is already visible in posture
(XSPF salvages truncation, MPD refuses it — intentional, but the copies will
drift). Confidence: HIGH — Verification: read; rg confirmed `line_over_cap` in
both m3u-family and xspf.rs.

F-004 [S4]
Where: crates/playlists/src/mpd.rs:1916-1977
Code:
```
fn parse_xs_duration(raw: &str) -> Option<f64> {
    let s = raw.trim();
    let (neg, rest) = match s.strip_prefix('-') {
```
What is wrong: Accepts negative durations (`-PT30S` → -30.0, pinned in the
duration table as a *passing* case at L2422) for attributes where MPD (ISO/IEC
23009-1, xs:duration in a non-negative context) never allows them; the model
then carries `media_presentation_duration: Some(-30.0)`. Harmless today because
nothing consumes the value in-product, but a future C1b consumer inherits
sign-checking debt. What correct looks like: refuse negatives at the fatal
boundary (or document why they are tolerated). Confidence: MEDIUM —
Verification: read + test table at L2422 pins the behavior intentionally.

## 4. Per-function table (>100 LOC file — all functions)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parse_mpd | 652-679 | REAL | — | bounds then scan+drive; truncation checked before finish |
| parse_mpd_bytes | 685-688 | REAL | — | UTF-8 gate, delegates |
| Scanner::new/take/peek_char/advance_char/skip_ws/rest_starts_with | 725-762 | REAL | — | byte-accurate line tracking, char-boundary safe |
| Scanner::truncated_at_eof/skip_after | 766-780 | REAL | — | marks truncation, consumes rest |
| Scanner::next_token | 782-850 | REAL | — | comments/PI/CDATA/DOCTYPE-skip/text; `<`+non-name → literal text w/ warning |
| Scanner::scan_end_tag | 852-878 | REAL | — | damaged end tag resyncs to `>`, warns |
| Scanner::scan_start_tag | 880-952 | REAL | — | both quote styles, attr damage resync; unterminated value → truncation |
| Scanner::resync_attr | 957-964 | REAL | — | progress guaranteed (stops only at ws/`>`; ws consumed by loop top) |
| is_namespace_decl | 969-971 | REAL | — | xmlns dropped silently (documented) |
| Driver::new/push/feed | 999-1044 | REAL | — | depth guard on every push (L1024) |
| Driver::on_start | 1046-1320 | REAL | — | root gate (NotMpd), stray-after-root, all 12 modeled elements |
| Driver::skip_start/push_or_close | 1323-1352 | REAL | — | `let _ = attrs` intentional (already warned via note_unknown_attrs callers) |
| Driver::apply_mpd_attrs | 1354-1406 | REAL | F-002 | fatal type/durations; last-wins dup bug |
| Driver::note_unknown_attrs | 1410-1419 | REAL | — | known-list warn |
| Driver::attach_protection/attach_descriptor/attach_base_url | 1421-1473 | REAL | — | position-checked attach |
| Driver::attach | 1476-1570 | REAL | — | first-wins duplicate segment descriptions |
| Driver::on_end | 1572-1595 | REAL | — | innermost name match + salvage-close above it |
| Driver::on_text | 1597-1614 | REAL | — | BaseURL accumulation; ws-only runs dropped |
| Driver::finish | 1618-1631 | REAL | — | no-root → NotMpd; open stack → Truncated; 0 periods → warning |
| attach_segment_list | 1635-1646 | REAL | — | first-wins |
| attr | 1656-1661 | REAL | F-002 | first-wins lookup |
| fatal_u64/fatal_duration | 1664-1674 | REAL | — | typed BadValue |
| warn_u64/warn_bool | 1678-1715 | REAL | — | degrade-to-None + warning |
| apply_period_attrs | 1717-1742 | REAL | — | fatal start/duration |
| apply_adaptation_set_attrs | 1744-1776 | REAL | — | warn-level facts |
| apply_representation_attrs | 1778-1806 | REAL | — | MissingBandwidth warning |
| apply_segment_template_attrs | 1808-1836 | REAL | — | fatal timescale/duration/startNumber/pto; verbatim init/media |
| apply_segment_list_attrs | 1838-1860 | REAL | — | same fatal set |
| apply_segment_base_attrs | 1862-1885 | REAL | — | warn-level (indexRange verbatim) |
| timeline_entry_from_attrs | 1887-1908 | REAL | — | S@d required fatal; t/r typed |
| parse_xs_duration | 1916-1977 | REAL | F-004 | refuses Y/M/W, non-finite; accepts negatives |
| local_name/is_name_start/is_name_char | 1981-1994 | REAL | — | prefix-tolerant |
| line_over_cap | 1998-2019 | REAL | F-003 | O(n), covers unterminated last line |
| decode_entities | 2025-2057 | REAL | — | single pass, no re-scan (bomb-proof), verbatim unknowns |
| decode_one/strip_hex | 2059-2084 | REAL | — | 5 named + dec/hex numeric; char::from_u32 range-checked |
| 16 tests | 2115-2854 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ffmpeg_template_fixture_types_and_matches_ffprobe | YES | pins ~40 exact model values incl. timescale 10240, timeline d=61440, template strings verbatim |
| ffmpeg_list_fixture_types | YES | pins SegmentList/SegmentURL/Initialization exact values |
| hand_static_fixture_full_surface | YES | pins CENC record, per-level BaseURLs, SegmentBase ranges, inheritance-not-copy (r0.segment_template None) |
| hand_dynamic_fixture_types | YES | pins dynamic type, PTO 4.5M, timeline t/d/r incl. negative r=-1 |
| duration_table | YES | 10 accept + 12 refuse cases, exact values |
| not_mpd_refuses / bad_value_refuses | YES | exact error variants + codes per input |
| truncated_refuses | YES | 5 real cut offsets must all give Truncated |
| depth_exceeded_refuses | YES | 70-deep input must give DepthExceeded(>64) |
| bounded_input_battery | YES | bomb parses with BadEntity warnings; oversize/long-line/UTF-8 refusals |
| dirty_world_degrades_to_warnings | YES | junk facts→None+warn, dup first-wins, salvage, stray end tag |
| empty_root_records_no_periods | YES | NoPeriods warning pinned |
| namespace_tolerant_local_names | YES | prefixed names must parse |
| base_url_and_entity_decoding | YES | `&amp;amp;` decodes exactly once to `&amp;` |
| error_code_table | YES | all 7 codes pinned incl. Display containing code |
| fuzz_lite_mutations_never_panic | WEAK-ISH | only no-panic (any Err/Ok accepted) — honest as a panic harness, not a correctness oracle |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Entity-expansion bombs are structurally impossible" | mpd.rs:63-68 | REAL | decode-once at L2025-2057; bomb test passes (ran) |
| "first occurrence wins, documented" (dup attrs) | mpd.rs:1654-1655 | FALSE for apply_* path | F-002 for-loop overwrite |
| network-dash "partial" on win/linux/macos with this file as evidence | docs/feature-matrix.toml:1450-1453 | SHALLOW | parse/resolution/ABR exist as libs; no shipped path reaches them (F-001) |
| "8 MiB, matching the fetch port's default response budget" | mpd.rs:128-132 | UNVERIFIED | streaming fetch default response_limit not re-checked in this audit (other agent's file) |

## 7. Dead / duplicated / unreachable code
- Whole module is dead in the shipped product (F-001) — reachable only from
  `streaming::dash` library/tests.
- Scanner + `line_over_cap` duplicated from xspf.rs (F-003).

## 8. Dependencies & flags
Std + thiserror only (as documented, lib.rs:53). No features gate this module;
`playlists` is a plain dependency of `streaming` and (transitively) nothing else
that ships.

## 9. Open questions for the next agent
- Does `streaming::fetch::HttpRequest::response_limit` default actually equal
  8 MiB (mpd.rs:129 claim)? Check crates/streaming/src/fetch.rs.
- Who will own the DASH session wiring: is there a card/issue for
  `open_source` → MPD detection, or is DASH permanently library-only?
- xspf.rs scanner copy: same duplicate-attribute policy bug there?
