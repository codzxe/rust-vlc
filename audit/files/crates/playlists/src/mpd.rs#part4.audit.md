# AUDIT: crates/playlists/src/mpd.rs#part4 (lines 1717-2273)
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-21 |
| Chunk boundaries | part4 = 1717 (`apply_period_attrs`) through 2273 (end of `ffmpeg_list_fixture_types` + its doc). Part3 ends with `warn_bool` (≤1715); part5 begins at the `hand_static_fixture_full_surface` doc comment (2274). Equal-split anchor 1714-2284 adjusted ±3 lines to function boundaries. |
| Lines / functions | 557 / 17 (14 prod + 2 tests + tests-mod header) |
| File verdict | UNREACHABLE (real, tested parser code; no shipped entry point reaches it — see reachability) |
| Reachable from shipped entry point? | NO. Chain proven absent: `parse_mpd`/`parse_mpd_bytes` exported (playlists/src/lib.rs:87) but every caller is `#[cfg(test)]` (streaming/src/dash/abr.rs:327-333, sched.rs:474-480, resolve.rs:741-758) or the cargo-fuzz harness (dash/fuzz.rs:39 — not a shipped binary). `rg "DashScheduler|DashAbr|plan_representation"` outside `src/dash/` → zero hits; `rg -in "mpd|dash"` over apps/cli → zero hits; remote_playlist.rs:34 states "PLS/MPD remote open are not in 6.E2's scope (M3U + XSPF only)". The shipped `lumen open` http lane (demux_host.rs:413-418) probes fetched bytes as a container — an MPD never reaches this parser. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `rg -n "^(pub |)fn|impl|mod" mpd.rs` (outline/part split); `rg -n "parse_mpd|mpd::"` across crates+apps (all callers test-only); `rg -n "DashScheduler|DashAbr|plan_representation"` (zero production users); `rg -in "mpd|dash"` apps/cli (nothing); `sed -n` on all quoted ranges (verbatim); read of part5 test `base_url_and_entity_decoding` (2736-2761) to confirm hex char-refs are untested. |

## 1. One-paragraph verdict
The attribute-binding and lexing utilities in this chunk are genuine, careful work — warn-vs-fatal classification, a spec-honest `xs:duration` subset that refuses years/months instead of guessing, a single-pass entity decoder with a real entity-bomb bound — and the two fixture tests assert a deep field surface that would catch regressions. But the XML hex character reference `&#x41;` (the standard W3C spelling) is NOT decoded: `strip_hex` looks for a C-style `0x` prefix, so only the non-XML spelling `&#0x41;` works, and the doc comment's "decimal/hex numeric references" claim is false for the standard form. And none of it is reachable from the shipped binary: no DASH session wiring exists in any app; the entire mpd.rs + streaming::dash stack is a tested library island.

## 2. What actually works (verified)
— verified by baseline run `audit/runs/baseline/test-workspace.log.txt` (workspace tests, 1681 passed incl. the playlists crate; `mpd` tests listed in orchrun-test-workspace-nofailfast.log.txt): the chunk's functions execute in-tree via `parse_mpd` over the committed ffmpeg fixtures.
— verified by read: `parse_xs_duration` refuses `Y/M/W`, fractional days, and non-finite results instead of guessing (1947-1948, 1973-1975); `decode_entities` is genuinely single-pass (no rescan of emitted output — `&amp;amp;` cannot expand, pinned by test at 2752-2761); `line_over_cap` handles the final unterminated line (2014-2017).

## 3. Findings

### F-001 [S2]
Where: crates/playlists/src/mpd.rs:2068-2083
Code:
```
let num = name.strip_prefix('#')?;
let value = if let Some(hex) = strip_hex(num) {
    u32::from_str_radix(hex, 16).ok()?
} else {
    num.parse::<u32>().ok()?
};
...
fn strip_hex(num: &str) -> Option<&str> {
    let lower = num.to_ascii_lowercase();
    if lower.starts_with("0x") {
```
What is wrong: XML 1.0 character references are `&#x1F600;` — the hex marker is `x` immediately after `#` (W3C XML §2.4 CharRef). `strip_hex` instead strips a C-style `0x` prefix, so the standard form `&#x41;` reaches `num.parse::<u32>()` with `num = "x41"`, fails, and is emitted verbatim with a `BadEntity` warning — while the non-XML form `&#0x41;` decodes. Decimal refs (`&#65;`) are correct.
Why it matters: any hex-encoded non-ASCII character in an MPD (BaseURLs, titles, descriptor values from international providers) stays as literal `&#xE9;` text in the model — silent data corruption in exactly the metadata this parser exists to carry; the doc comment at 2023-2024 ("plus decimal/hex numeric references") asserts the capability that the standard spelling defeats. Currently muted by unreachability (library island), but this is the 6.C1a landing surface for future wiring.
What correct looks like: after `strip_prefix('#')`, strip a leading `x`/`X` and parse radix-16; drop the `0x` form.
Confidence: HIGH — Verification: read + hand-trace of both spellings; no test covers hex refs anywhere in mpd.rs (checked `base_url_and_entity_decoding`, 2736-2761: named entities and nesting only).

### F-002 [S3]
Where: crates/playlists/src/mpd.rs:1816-1818 vs 1870
Code:
```
"timescale" => {
    template.timescale = Some(fatal_u64("SegmentTemplate@timescale", value)?)
}
...
"timescale" => base.timescale = warn_u64(line, "timescale", value, warnings),
```
What is wrong: identical damage (garbage `timescale`) is FATAL on SegmentTemplate/SegmentList but a WARNING on SegmentBase. The same attribute, same damage class, same spec element family, two different classifications.
Why it matters: inconsistent degrade philosophy — a hostile or buggy manifest fails hard or limps on depending on which sibling element carries the damage; downstream consumers cannot reason about which invariants the parsed model guarantees.
What correct looks like: one policy for `timescale` across all three segment-addressing elements (fatal, matching its role in playback math).
Confidence: HIGH — Verification: read of all three `apply_*` functions in this chunk.

### F-003 [S4]
Where: crates/playlists/src/mpd.rs:1916-1977 (function as a whole)
Code:
```
let (neg, rest) = match s.strip_prefix('-') {
    Some(r) => (true, r),
    None => (false, s),
};
```
What is wrong: unbounded magnitude — `P999999999999999999999D` parses to a finite f64 (~8.6e25 s) with no sanity ceiling; negative durations are accepted (correct per `xs:duration`, but nothing downstream is shown to handle a negative Period@start).
Why it matters: cosmetic today (model is a data structure, no allocation scales with the value); a documented bound would make the "honest numbers" doctrine complete.
What correct looks like: an upper-magnitude refusal or a documented decision that magnitude is unbounded by design.
Confidence: MEDIUM — Verification: read + hand-trace; no execution.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| apply_period_attrs | 1717-1742 | REAL | — | id/start/duration/bitstreamSwitching; durations fatal, bool warn; unknown → warning. Consistent |
| apply_adaptation_set_attrs | 1744-1776 | REAL | — | warn-only numeric fields; frameRate/par kept raw strings (resolution is resolve.rs's job) |
| apply_representation_attrs | 1778-1806 | REAL | — | MissingBandwidth warning (spec-required attr degrades to warning — deliberate, typed) |
| apply_segment_template_attrs | 1808-1836 | REAL | — | all numerics fatal; media/initialization raw |
| apply_segment_list_attrs | 1838-1860 | REAL | — | same fatal policy as template — consistent with each other |
| apply_segment_base_attrs | 1862-1885 | REAL (defect) | S3 | F-002: warn policy diverges from its two siblings on the same attributes |
| timeline_entry_from_attrs | 1887-1908 | REAL | — | `d` required (BadValue), `t`/`r` optional; `r` as i64 covers negative (unknown-repeat) per DASH; no overflow risk (u64/i64 parse) |
| parse_xs_duration | 1916-1977 | REAL | S4 | spec-honest Y/M/W refusal; fractional-seconds-only; F-003 unbounded magnitude |
| local_name | 1981-1986 | REAL | — | strips prefix at last ':' — correct for `dash:Representation` |
| is_name_start / is_name_char | 1988-1994 | REAL | — | Unicode-lenient vs XML 1.0 strict Name production — safe direction for a reader |
| line_over_cap | 1998-2019 | REAL | — | final unterminated line handled; 64 KiB cap from m3u (mpd.rs:118 import) |
| decode_entities | 2025-2057 | REAL | — | single-pass (no re-scan) — genuine entity-bomb bound; unknown refs verbatim + warning; output ≤ input length so with_capacity is sound |
| decode_one | 2059-2075 | REAL (defect) | S2 | F-001: standard `&#x…;` hex form fails; decimal + 5 named entities correct; surrogates refused via from_u32 |
| strip_hex | 2077-2084 | REAL (defect) | S2 | the `0x` branch implementing the wrong dialect (with F-001) |
| tests::ffmpeg_template_fixture_types_and_matches_ffprobe | 2116-2224 | REAL | — | deep field surface incl. 1e-9 duration tolerances; ffprobe differential recorded out-of-band in doc comment (honest: values recorded, command pinned) |
| tests::ffmpeg_list_fixture_types | 2234-2272 | REAL | — | documents ffprobe's own refusal of this shape as the oracle's gap — honest |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ffmpeg_template_fixture_types_and_matches_ffprobe (2116) | YES | ~50 field-level asserts over the parsed model (durations, ladder, template strings, timeline entries); any binding regression fails it. Would NOT catch F-001 (no hex-ref fixture) |
| ffmpeg_list_fixture_types (2234) | YES | list-model asserts incl. segment URL shapes |
Mutation check (reasoned, not run): delete the `total += value` in the H arm of parse_xs_duration → first test fails at `min_buffer - 12.0` (fixture carries PT12H minBufferTime). Make decode_one's named-entity arm return None → part5's base_url_and_entity_decoding fails on `a&b`. Both tests are load-bearing.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The five named entities plus decimal/hex numeric references" decode | mpd.rs:2023-2024 doc | FALSE for standard hex | F-001: `&#x41;` → BadEntity + verbatim; only `&#0x41;` (non-XML) decodes |
| network-dash row: "MPEG-DASH: MPD handling … buffer-based ABR" status partial, evidence = this parser + dash/** tests | feature-matrix.toml:1444-1454 | REAL code / UNREACHABLE from shipped binary; row never discloses that no app wires a DASH session | reachability chain above; row's pending list (e2e, DASH-IF vectors) omits "not wired into any playback session" |
| "6.C — landed: dash" | crates/streaming/src/lib.rs:8-9 | REAL as library, UNREACHABLE as playback | same chain |

## 7. Dead / duplicated / unreachable code
The entire chunk (and file) is unreachable from shipped entry points — see header. `line_over_cap` is an admitted copy of xspf's helper (comment at 1996-1997 says "a copy of its helper — see module docs") — tracked duplication, S4-level, already self-documented.

## 8. Dependencies & flags
No external deps in this chunk (std only). `MAX_LINE_BYTES` reused from crate::m3u (64 KiB). No feature gates. playlists crate is a hard dependency of streaming, so the code compiles into the shipped binary's dependency graph but no call chain reaches it.

## 9. Open questions for the next agent
- Part5 owner: does `fuzz_lite_mutations_never_panic` (2803) mutate entity regions? If yes, it exercises F-001's path as no-panic only — still no correctness pin.
- Wave-10 matrix-rows owner: network-dash (1444) and network-hls (1431) — both cite library tests as evidence; neither states that no shipped entry point opens HLS/DASH. Verify whether any CLI path plays an m3u8 (my rg says no hls:: in apps/cli).
- Whoever owns streaming/schemes.rs: what does `lumen open https://…/manifest.mpd` actually do today? (Probe refusal? Which typed error?) That is the user-visible truth for the DASH row.
