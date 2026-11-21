# AUDIT: crates/streaming/src/hls/timestamp_map.rs
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-11-21 |
| Lines / functions | 421 / 8 production items + 8 tests |
| File verdict | REAL (correct arithmetic, strong tests) — UNREACHABLE from shipped entry points; one real overflow defect in the date parser |
| Reachable from shipped entry point? | NO. Consumers: crates/streaming/src/hls/media.rs:734 (`TimestampMap::from_playlist_text`) and crates/streaming/src/hls/live.rs (HlsLiveStream fields/surface) — the HLS session layer, which no app references (rg `hls::` apps/ → zero; CLI uses only streaming::{fetch,ranged,schemes,mockhttp}). Compiled into `lumen` via the streaming dep; never invoked. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 0 |
| Commands run | read 1-421 in full · hand-verified signed_wrap_delta and pts_for_wallclock_ms modulo arithmetic (incl. negative-delta wrapping_add path) · hand-computed the days→ms overflow threshold for F-001 (i64::MAX / 86 400 000 ≈ 1.0675e11 days ≈ 2.92e8 years) · rg TAG_TIMESTAMP_MAP (media.rs:628 — includes the colon, so strip_prefix yields the tag value) · rg TimestampMap callers (media.rs, live.rs — both hls-internal) · baseline #3 (all 8 tests pass) |

## 1. One-paragraph verdict
The PTS↔wallclock arithmetic is genuinely correct and unusually well
defended: the 33-bit wrap distance is exact in both directions across
the wrap (hand-verified modulo math + the rollover test), the inverse
saturates instead of overflowing (a previously recorded netfuzz finding,
now pinned by exact-value tests), and the Hinnant calendar arithmetic
has an independent truth table covering the 1900/2000 leap-century
rules. One real defect survives: the ISO-8601 parser validates month,
day, hour, minute, second — but not the YEAR, so a LOCAL year beyond
~292 million (or below ~−292 million) overflows the i64
days→milliseconds conversion: a panic under overflow-checks, a silent
wrap in the shipped release profile, in both cases violating the
function's own "out-of-range fields … are refused, not clamped"
contract. The malformed-tags test covers every other field's range but
not the year's. And the whole module is dead code in the shipped binary
(the HLS session layer has no app caller).

## 2. What actually works (verified)
— `signed_wrap_delta` (142-149): `(b-a) mod 2^33` rebased into
[-2^32, 2^32) — the standard signed-circle representative; correct for
both rollover directions (test 275-291 pins +1 s across the wrap from
either side).
— `wallclock_ms_for_pts` (115-117): `local_ms + delta * 1000 / 90_000` —
delta bounds ±2^32 keep the product at ≤ 4.3e12, no i64 overflow for any
in-contract anchor; exact within the documented ±13.25 h window.
— `pts_for_wallclock_ms` (133-137): saturating sub/mul (the netfuzz fix),
and the `wrapping_add(delta_ticks as u64) % PTS_MODULUS` tail is correct
for NEGATIVE deltas too (negative i64 as u64 wraps mod 2^64; since
2^33 divides 2^64, the final % 2^33 equals the true modular result) —
pinned by `pts_for_wallclock_ms(-500) == PTS_MODULUS - 45_000` (250).
— `parse_tag_value` (73-96): either attribute order, both required
(`mpegts?`/`local?` at 93-94), unknown well-formed attributes tolerated,
MPEGTS ≥ 2^33 refused, negative MPEGTS refused by the u64 parse.
— `from_playlist_text` (103-108): last PARSEABLE tag wins (malformed
occurrences skipped — dirty playlist test 339-361); TAG_TIMESTAMP_MAP
includes the colon (media.rs:628) so the stripped remainder is the tag
value.
— Calendar: `days_from_civil` is Hinnant's algorithm with correct
floor-division emulation for negative years (`(y-399)/400` under
truncating division); truth table 411-420 independently pins 1970/1969,
leap day 2000 (400-year rule), non-leap 1900, 2024-12-31, 2100-01-01.
— Fractional seconds: 1-9 digits enforced, first three digits weighed
100/10/1 ms, finer truncated — documented and tested (.789 → 789).
— All 8 tests passed in baseline #3.

## 3. Findings

### F-001 [S2]
Where: crates/streaming/src/hls/timestamp_map.rs:159 (unbounded year parse) and 194-195 (the overflowing conversion)
Code:
```rust
    let year: i64 = date_parts.next()?.parse().ok()?;
    let month: u32 = date_parts.next()?.parse().ok()?;
```
and:
```rust
    let days = days_from_civil(year, month, day);
    Some(((days * 24 + hour) * 60 + minute) * 60 * 1000 + second * 1000 + frac_ms)
```
What is wrong: Every field is range-checked except the year. A LOCAL
value of `3000000000-01-01T00:00:00Z` parses (year fits i64), yields
days ≈ 1.0957e12 from days_from_civil, and the conversion
`days * 24 * 60 * 60 * 1000` ≈ 9.47e19 exceeds i64::MAX (9.22e18): panic
under overflow-checks (debug/test profiles), silent wrap in the shipped
release profile. Threshold: |year| ≳ 292,000,000 (i64::MAX /
86 400 000 / 365.2425). Negative years of similar magnitude underflow
the same way. The function's contract (151-154) says "Out-of-range
fields (month 13, Feb 30, hour 24, …) are refused, not clamped" — the
year field breaks it, and the malformed-tags test (366-405) covers every
other field's range but has no absurd-year case.
Why it matters: A hostile or generator-broken playlist tag that should
be a typed `None` (no mapping, playback continues) instead corrupts the
anchor: release builds map every PTS through a garbage `local_ms`
(wrong live-edge wallclock, wrong WebVTT alignment — the module's two
documented consumers), and any overflow-checked build panics inside a
parser whose doctrine is never-panic. Practical severity is capped by
unreachability (F-002); wired into a live session this is S1-class
(panic on hostile network input).
What correct looks like: Range-check the year (e.g. 0..=9999 per the
HLS ISO profile, or at minimum |days| bounded so the ms conversion
cannot overflow) and return None outside it, exactly like month/day/hour.
Confidence: HIGH — Verification: read + hand-computed arithmetic
(threshold math above; the intermediate `days * 24` at 2.63e13 is safe,
the `* 60 * 60 * 1000` chain overflows at ~9.47e19 for year 3e9). Not
executed (no cargo runs allowed) — REQUEST-ORCH-RUN candidate logged.

### F-002 [S3]
Where: crates/streaming/src/hls/timestamp_map.rs:55-138 (the whole public surface)
Code:
```rust
pub struct TimestampMap {
    /// The `MPEGTS` anchor: a PTS tick count, always `< 2^33`
```
What is wrong: The module is unreachable from shipped entry points: its
consumers are hls/media.rs:734 and hls/live.rs, and nothing outside the
streaming crate references the hls session layer (apps/cli uses only
fetch/ranged/schemes/mockhttp — rg-verified).
Why it matters: Correct, well-tested live-streaming arithmetic (the
live-edge derivation, the WebVTT alignment record's closure) sits dead
in the shipped binary; the docs/formats/sub-vtt.md "X-TIMESTAMP-MAP
unapplied" record can only be closed for library consumers, not for any
user of `lumen`.
What correct looks like: Wire the HLS session into the streaming client
/ an entry point, or record the layer as pending its Phase-6 host.
Confidence: HIGH — Verification: read (call chains grepped).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-44 | REAL | — | Tag grammar, wrap math, and the `[VERIFY]` anchor-position caveat all accurate; honest about the spec not guaranteeing the anchor. |
| `PTS_MODULUS` / `PTS_HZ` | 48-51 | REAL | — | 2^33 / 90 000. |
| `TimestampMap` | 54-63 | REAL / UNREACHABLE | S3 | F-002. |
| `parse_tag_value` | 72-96 | REAL | — | Either order, both required, ≥2^33 refused, unknown attrs tolerated; note: an unknown token WITHOUT '=' refuses the whole tag (77) — defensible (malformed), but stricter than the "tolerated" doc line; duplicate MPEGTS/LOCAL: last wins (no warning). |
| `from_playlist_text` | 102-108 | REAL | — | Last parseable wins; malformed skipped, never fatal. |
| `wallclock_ms_for_pts` | 114-117 | REAL | — | Exact within ±13.25 h; no overflow for in-contract anchors. |
| `pts_for_wallclock_ms` | 132-137 | REAL | — | Saturating (netfuzz fix); negative-delta wrap path hand-verified correct. |
| `signed_wrap_delta` | 140-149 | REAL | — | Standard signed-circle representative; correct. |
| `parse_iso8601_utc` | 151-196 | REAL | S2 | F-001 year-range hole; everything else (Z-only, 3-part date, 1-9 frac digits, month/day/hh/mm/ss ranges, truncating ms) verified correct. |
| `is_leap_year` / `days_in_month` | 198-210 | REAL | — | Correct; `_ => 0` unreachable behind the month range check (short-circuit at 188). |
| `days_from_civil` | 212-222 | REAL | — | Hinnant's algorithm; negative-year floor-division emulation correct; truth-table tested. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| parses_the_tag_in_either_attribute_order (231) | YES | Order-independence + exact anchor values. |
| bom_mapping_converts_pts_to_epoch_milliseconds (243) | YES | Exact forward/inverse values incl. negative wallclock → wrap-side PTS. |
| non_zero_anchor_maps_affinely (255) | YES | Affine (not BOM-relative) mapping, leap-day LOCAL, fractional ms. |
| pts_rollover_across_the_33_bit_wrap_is_exact (275) | YES | ±1 s across the wrap both directions — a naive subtraction reports ~26.5 h. |
| far_from_anchor_wallclock_saturates_instead_of_overflowing (301) | YES | Exact saturated images pinned for i64::MAX/MIN probes and opposite-extreme anchors — the netfuzz regression proof. |
| last_parseable_tag_in_the_playlist_wins (339) | YES | Supersession, garbage-occurrence tolerance, no-tag None. |
| malformed_tags_refuse (367) | YES | 33-bit overflow, missing attrs, Feb 30/month 13/hour 24/min 60/sec 60, non-UTC zone, fractional garbage. GAP: no absurd-year case (F-001 untested). |
| civil_date_arithmetic_truth_table (411) | YES | Independent truth values incl. 1900 vs 2000 leap rules. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "exact for any PTS within ±13.25 h of the anchor" | timestamp_map.rs:30-32 | REAL | signed_wrap_delta bounds + rollover test. |
| "Out-of-range fields … are refused, not clamped" | timestamp_map.rs:151-154 | FALSE (year field) | F-001. |
| "the delta saturates at the signed bounds instead of panicking" (pts_for_wallclock_ms) | timestamp_map.rs:126-131 | REAL | Saturating ops + exact-value saturation test. |
| "a tag we cannot understand must not refuse playback" (parse → None) | timestamp_map.rs:69-71 | REAL | All malformed shapes → None; no panics except F-001's overflow-checks case. |
| live-edge derivation rests on an unguaranteed anchor assumption | timestamp_map.rs:39-44 | REAL (honest `[VERIFY]`) | Caveat recorded rather than claimed — good hygiene. |

## 7. Dead / duplicated / unreachable code
Whole module unreachable from shipped entry points (F-002). No dead
items internally; all eight functions are exercised by the suite.

## 8. Dependencies & flags
crate::hls::media::TAG_TIMESTAMP_MAP only (plus std). No unsafe, no
feature gates, no I/O — pure text→numbers. testkit::Must used in tests.

## 9. Open questions for the next agent
- Whoever holds hls/media.rs + hls/live.rs: `live_edge_wallclock_ms`
  consumes this mapping — when F-001's year check is added, keep the
  refusal total (None) so the session degrades to no-mapping, per the
  module's own doctrine.
- REQUEST-ORCH-RUN (logged): `cargo test -p streaming timestamp_map`
  plus a one-off debug-profile assertion of
  `parse_tag_value("MPEGTS=0,LOCAL=3000000000-01-01T00:00:00Z")` —
  expected: panic under overflow-checks, demonstrating F-001.
