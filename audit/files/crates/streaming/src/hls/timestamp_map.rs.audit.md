# AUDIT: crates/streaming/src/hls/timestamp_map.rs
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-23 |
| Lines / functions | 421 / 17 (9 production fns, 8 tests) |
| File verdict | REAL (with one executed integer-overflow defect on crafted input) |
| Reachable from shipped entry point? | NO from the CLI (consumed only by `fetch_playlist_model` → `open_live`/`open_vod`, unwired — see live.rs audit). YES from the in-tree fuzz target: crates/streaming/tests/netfuzz/fuzz.rs:109 calls `TimestampMap::from_playlist_text` directly on fuzzed text. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 4 |

## 1. One-paragraph verdict
The wraparound math is genuinely good: signed wrap-distance on the 33-bit PTS circle (exact ±13.25 h across the wrap, tested both directions with round-trips), a netfuzz-recorded i64 overflow in the inverse mapping fixed with saturating arithmetic and pinned with exact saturated images, and Hinnant's `days_from_civil` behind a truth-table test including the 1900/2000 leap rules. The 8 tests pass (I ran them). But the ISO-8601 parser has NO range check on the YEAR: a crafted `LOCAL=999999999-01-01T00:00:00Z` overflows the days→milliseconds multiplication at line 195 — I EXECUTED the verbatim-copied parse path: with overflow checks on it panics `attempt to multiply with overflow`; in the release profile it silently wraps to a garbage anchor (`local_ms=-5336598346174303232`). That violates the function's own contract ("a malformed or out-of-range value is `None`… must not refuse playback") and is fuzzer-reachable via the in-tree netfuzz target.

## 2. What actually works (verified)
— `cargo test -p streaming --lib timestamp_map` (run as part of `hls::` filter earlier; targeted re-run below) → all pass. Rollover vector (anchor 0.5 s before 2^33 wrap → +1 s exact), inverse saturation at `i64::MAX`/`i64::MIN` anchors with exact tick images, last-tag-wins with garbage-skip, malformed-shape refusal table, civil-date truth table.
— Execution probe (this audit): verbatim copy of L48-222 compiled standalone:
  `./verbatim_probe 1970-01-01T00:00:00Z` → `PARSED local_ms=0`; `292277025-01-01…` → `PARSED local_ms=9223309881446400000` (last in-range year); `999999999-…` → panic (debug profile) / `local_ms=-5336598346174303232` (release profile). Probe source retained at audit/runs/w10a06-verify/verbatim_probe.rs.

## 3. Findings
F-001 [S2]
Where: crates/streaming/src/hls/timestamp_map.rs:188-195 (year parsed unbounded at :159)
Code:
```
    let days = days_from_civil(year, month, day);
    Some(((days * 24 + hour) * 60 + minute) * 60 * 1000 + second * 1000 + frac_ms)
```
What is wrong: `year` is parsed as bare `i64` (:159 `let year: i64 = date_parts.next()?.parse().ok()?;`) with month/day/hour/minute/second all range-checked but the year not. A year ≥ ~292,277,226 makes `days * 86_400_000` exceed `i64::MAX`. EXECUTED: with `-C overflow-checks=on` the parse path panics `attempt to multiply with overflow` at this line; with `-O` (shipped profile) it silently wraps — `LOCAL=999999999-01-01T00:00:00Z` yields `local_ms = -5336598346174303232`, a poisoned anchor that then flows into `live_edge_wallclock_ms` and `wallclock_ms_for_pts`. Why it matters: (a) the documented contract at :69-71 says out-of-range values are `None`, never fatal — FALSE for this shape; (b) the in-tree netfuzz target calls `from_playlist_text` on fuzzed input (fuzz.rs:109), so this is a fuzzer-findable panic lane; (c) if the HLS lane is ever wired to the CLI this becomes a remotely triggerable panic (S1 at that point). What correct looks like: bound the year (e.g. reject `year > 9999` or compute `days` then `checked_mul` chain → `None`).
Confidence: HIGH — Verification: EXECUTED (verbatim-copy probe, both profiles; boundary year 292277025 in-range / 300000000 panics). Cross-checked against the real file by sed quote above.
F-002 [S4]
Where: crates/streaming/src/hls/timestamp_map.rs:115-117
Code:
```
    pub fn wallclock_ms_for_pts(&self, pts: u64) -> i64 {
        self.local_ms + signed_wrap_delta(self.mpegts, pts) * 1000 / PTS_HZ as i64
    }
```
What is wrong: the forward mapping has no saturation while its inverse (`pts_for_wallclock_ms`, :133-137) saturates by design — asymmetric posture. With a legal-year anchor near the i64 ceiling (year ≈ 292 million, reachable without F-001) plus a wrap delta up to ±4.3e12 ms, the addition overflows: same panic/wrap split by profile. Root cause shared with F-001: unbounded year. What correct looks like: `checked_add` → the caller already handles `None`-shaped absence… this returns i64, so saturating_add matching the inverse's posture.
Confidence: HIGH — Verification: read + arithmetic (i64::MAX − 9.2233e18 < 4.3e12 margin is the failure window; requires absurd-but-parseable years).
F-003 [S4]
Where: crates/streaming/src/hls/timestamp_map.rs:155-157
Code:
```
fn parse_iso8601_utc(text: &str) -> Option<i64> {
    let text = text.trim().strip_suffix('Z')?;
```
What is wrong: UTC-`Z`-only profile — `LOCAL=…+01:00` (a legal ISO-8601 spelling some regional encoders emit) refuses to `None`, silently disabling the timestamp-map surface for those streams rather than converting. Documented as the admitted profile (:152-153), so an honest limitation, not a lie.
Confidence: HIGH — Verification: read + malformed_tags_refuse test pins the refusal.
F-004 [S4]
Where: crates/streaming/src/hls/timestamp_map.rs:76-77
Code:
```
        for attr in value.split(',') {
            let (name, val) = attr.split_once('=')?;
```
What is wrong: one attribute without `=` aborts the WHOLE tag (`?` on Option inside the loop), discarding already-parsed good attributes — e.g. `MPEGTS=900000,LOCAL=…,FOO` → None rather than ignoring the damaged trailing attribute. Strictness choice, consistent with "cannot understand → no mapping", but stricter than the sibling parser in playlists (which tolerates unknown attributes WITH `=`). Note also: ISO-8601 values never contain commas, so the comma split is safe today.
Confidence: HIGH — Verification: read.
F-005 [S4]
Where: crates/streaming/src/hls/timestamp_map.rs:104-107
Code:
```
        text.lines()
            .filter_map(|line| line.strip_prefix(TAG_TIMESTAMP_MAP))
            .filter_map(TimestampMap::parse_tag_value)
            .next_back()
```
What is wrong: lines are not trimmed before the prefix strip — a tag indented by whitespace (never legal in HLS, but produced by sloppy republishers) is skipped. Sub-HTTP-shape tolerance gap only.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| TimestampMap::parse_tag_value | 73-96 | REAL | S4 | F-004 whole-tag abort; 33-bit MPEGTS bound enforced |
| TimestampMap::from_playlist_text | 103-108 | REAL | S4 | F-005 no trim; last-parseable-wins correct |
| TimestampMap::wallclock_ms_for_pts | 115-117 | REAL | S4 | F-002 unsaturated add |
| TimestampMap::pts_for_wallclock_ms | 133-137 | REAL | — | netfuzz fix: saturating sub/mul, wrapping add + % modulus — exact in range, tested at extremes |
| signed_wrap_delta | 142-149 | REAL | — | correct signed representative; no overflow (max ±2^32) |
| parse_iso8601_utc | 155-196 | REAL | S2 | F-001 EXECUTED overflow on unbounded year |
| is_leap_year | 198-200 | REAL | — | |
| days_in_month | 202-210 | REAL | — | `_ => 0` unreachable (month pre-validated) |
| days_from_civil | 214-222 | REAL | — | Hinnant algorithm; truth-table tested incl. 1900/2000 |
| 8 test fns | 230-420 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| parses_the_tag_in_either_attribute_order | YES | both orders equal, exact fields |
| bom_mapping_converts_pts_to_epoch_milliseconds | YES | exact ms values + inverse round-trips incl. negative |
| non_zero_anchor_maps_affinely | YES | leap-day anchor, exact affine offsets |
| pts_rollover_across_the_33_bit_wrap_is_exact | YES | wrap-crossing both directions, exact ±1000 ms |
| far_from_anchor_wallclock_saturates_instead_of_overflowing | YES | exact saturated images at i64 extremes |
| last_parseable_tag_in_the_playlist_wins | YES | last-good selection, garbage skip, None case |
| malformed_tags_refuse | YES | 10 refusal shapes |
| civil_date_arithmetic_truth_table | YES | 7 independently-computed dates |
Gap: no test for extreme years — F-001 lives precisely in the untested gap between "impossible dates" (year-agnostic) and the saturation test (which constructs TimestampMap directly, bypassing the parser).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A malformed or out-of-range value is `None` … must not refuse playback" | timestamp_map.rs:69-71 | FALSE for 9-digit years | F-001 EXECUTED: panic (checks on) / garbage wrap (release) |
| "exact for any PTS within ±13.25 h of the anchor" | timestamp_map.rs:30-32 | REAL | rollover test both directions |
| "the tick delta saturates … instead of panicking" (inverse) | timestamp_map.rs:127-131 | REAL | saturation test — but the claim is not extended to the parser (F-001) or forward map (F-002) |
| "[VERIFY] against a real-world corpus" (anchor assumption) | timestamp_map.rs:43-44 | honest flag | no corpus in tree; cross-ref live.rs F-004 |

## 7. Dead / duplicated / unreachable code
`days_in_month`'s `_ => 0` arm unreachable (month validated 1..=12 before call). No dead fns. Module unreachable from CLI (see header).

## 8. Dependencies & flags
`crate::hls::media::TAG_TIMESTAMP_MAP` (string const) only; no external crates; no features. Probe build used rustc 1.100.0 standalone (crate rlib was built by a different rustc — noted, does not affect integer arithmetic of the verbatim copy).

## 9. Open questions for the next agent
- netfuzz lane owner: has any campaign actually reached `LOCAL=<9-digit-year>`? (fuzz.rs:109 is the call site; a seed corpus with structured tags would find F-001 fast.)
- media.rs `fetch_playlist_model`: does it pass the RAW playlist text to `from_playlist_text` (yes per live.rs usage) — the [VERIFY] anchor assumption propagates from there.
