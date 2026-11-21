# AUDIT: crates/media-clock/src/mapping.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-21 |
| Lines / functions | 425 / 8 production items + 15 tests |
| File verdict | UNREACHABLE for 5 of 8 items (only `timestamp_to_nanos`/`NANO_BASE` have a production caller); the arithmetic itself is REAL and correct |
| Reachable from shipped entry point? | PARTIAL: `timestamp_to_nanos` ← clock.rs:436 `position_nanos` ← MediaClock ← session ← `lumen` (baseline avi/mp4 open sessions exercised the clock). `sample_delta_to_media_nanos`, `map_samples_to_timestamp`, `map_timestamp_to_samples`, `samples_until_end`, `nanos_to_timestamp` — zero non-test callers (rg over tree, excluding mapping.rs: only clock.rs:77 imports `{NANO_BASE, timestamp_to_nanos}`; lib.rs:123-124 re-exports are unconsumed). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 1 |
| Commands run | `rg "map_samples_to_timestamp\|sample_delta_to_media_nanos\|samples_until_end\|map_timestamp_to_samples\|timestamp_to_nanos" -g '!target' -g '!*mapping.rs'` → clock.rs:77 + lib.rs re-exports only; `sed -n '378,432p' + '676,695p' clock.rs` → the live clock reimplements the arithmetic inline; `sed -n '545,565p' docs/03-engine-plan.md` → §4.7(e) formula verbatim; div_round hand-checked against ties-away-from-zero cases |

## 1. One-paragraph verdict
The pure checked arithmetic in this file is correct — I hand-verified the ties-away-from-zero division, the rate-semantics deviation (media = counter × num/den), the forward/inverse duality, and every overflow path is typed, never a panic or wrap; its 15 unit tests are exact-valued and mutation-sensitive, and they ran green in the no-fail-fast baseline. The problem is that almost none of it is the code that runs: the live `MediaClock` (clock.rs) reimplements the same arithmetic inline — with a duplicated rounding helper and a DIFFERENT rounding structure (two roundings where this module deliberately does one) — so the exhaustively tested pure functions are an unused façade, and the spec (doc 03 §4.7(e)) still teaches a formula this module itself proves wrong. One function (`samples_until_end`) has genuinely ambiguous extent semantics that its only test cannot distinguish.

## 2. What actually works (verified)
- `div_round` (60-73) hand-checked: (5,2)→3, (-5,2)→-3, exact ties away from zero; degenerate `denom <= 0` → `None`; `r.checked_mul(2)` overflow propagates as `None` → typed Overflow by every caller. Correct.
- Rate semantics: `sample_delta_to_media_nanos(4×48000, 48000, 5/4) = 5e9` — verified by formula and by the test at 312-315; the module-doc deviation analysis (21-30) is dimensionally sound (Lumen `Rate` is a speed multiplier; the doc's `/ rate` sketch would halve media progress at 2×).
- Forward/inverse duality: forward `delta × 1e9 × num / (rate_hz × den)` vs inverse `delta_ns × den × rate_hz / (num × 1e9)` — algebraically exact inverses; roundtrip test 373-382 pins it (144000 → 4.25 s → 144000).
- All overflow paths typed (`TimeError::Overflow` with saturating context ticks, 79-85); no unwrap/panic/index anywhere in production lines; degenerate zero rates/`rate_hz` rejected typed (tests 335-343, including `Rate { num: 0 }` constructed via pub fields).
- Execution evidence: media-clock suite green in `orchrun-test-workspace-nofailfast.log.txt` (media-clock not among the 7 failing binaries; these are its in-file unit tests).

## 3. Findings

### F-001 [S3]
Where: crates/media-clock/src/mapping.rs:4-6 (module doc) vs the whole-tree caller graph
Code:
```rust
//! The audio-master clock's arithmetic, exposed as pure checked
//! functions so the engine crates (and tests) can map sample-counter
//! positions without going through a clock instance:
```
What is wrong: the stated purpose — engine crates using these pure functions — is false. Only `timestamp_to_nanos` is consumed (clock.rs:436). The four headline functions (`sample_delta_to_media_nanos`, `map_samples_to_timestamp`, `map_timestamp_to_samples`, `samples_until_end`) plus `nanos_to_timestamp` have zero production callers; the lib.rs re-exports (123-124) are unconsumed public API.
Why it matters: a "pure checked core + thin clock" architecture is claimed but not realized; the pure core is a test-only twin of the real arithmetic (F-002). Maintenance drift between the two is guaranteed.
What correct looks like: clock.rs's extrapolation path calls `sample_delta_to_media_nanos`/`map_samples_to_timestamp` instead of its inline copy, or the unused functions are deleted.
Confidence: HIGH — Verification: rg over the whole tree (excluding mapping.rs itself).

### F-002 [S3]
Where: crates/media-clock/src/mapping.rs:60-73 vs crates/media-clock/src/clock.rs:414-428, 390-393, 684-685
Code (mapping.rs, the tested copy):
```rust
fn div_round(numer: i128, denom: i128) -> Option<i128> {
    if denom <= 0 {
        return None;
    }
```
clock.rs (the running copy, 684-685):
```rust
            let numer = i128::from(sample).checked_mul(1_000_000_000);
            match numer.and_then(|n| checked_div_round(n, i128::from(rate_hz))) {
```
What is wrong: `checked_div_round` (clock.rs:414) is a verbatim duplicate of `div_round` (mapping.rs:60). Worse, the live clock path rounds TWICE — samples→ns at clock.rs:684-685, then ns × num/den at 390-393 — while mapping.rs deliberately computes one rational with ONE rounding (its comment at 145-146: "done as one rational to round once"). Double rounding and single rounding differ by up to 1 ns at half-way values, so the two implementations are not semantically identical.
Why it matters: the exhaustive unit tests pin the semantics of the copy that never runs; the clock's actual rounding discipline is untested against the documented one. This is exactly the drift the "pure core" architecture was supposed to prevent.
What correct looks like: one implementation, called by both; the clock's `Extrapolation` path delegates to `sample_delta_to_media_nanos` (which folds rate_hz and rate into a single rational).
Confidence: HIGH — Verification: both sites read; divergence class (double vs single rounding) is arithmetic fact.

### F-003 [S3]
Where: docs/03-engine-plan.md:552-553 (spec) vs crates/media-clock/src/mapping.rs:21-30 (code)
Code (doc):
```text
- Audio-master: `now() = anchor.media + (device_samples - anchor.at_sample) /
  rate` when audio is running; the device sample counter is the only trusted
```
What is wrong: the spec still teaches `/ rate`. The module's own analysis (mapping.rs:21-30) proves that formula dimensionally wrong for Lumen's `Rate` convention (a speed multiplier: 5/4 = five media-seconds per four counter-seconds) and records the deliberate deviation — but doc 03 §4.7(e) was never corrected.
Why it matters: the spec is the reference every wave cites; a reader implementing from doc 03 builds a clock that halves media progress at 2× speed. The deviation is recorded in code comments and a worklog, not where the spec reader will look.
What correct looks like: doc 03 §4.7(e) updated to `media = anchor + (counter − anchor_sample) × rate.num / rate.den` with a pointer to the recorded deviation.
Confidence: HIGH — Verification: doc read verbatim at 545-565; code deviation note read at 21-30; clock.rs live path (× num, / den) confirms the code side.

### F-004 [S3]
Where: crates/media-clock/src/mapping.rs:228-241
Code:
```rust
pub fn samples_until_end(
    origin: Timestamp,
    extent: Duration,
    rate_hz: u32,
    rate: Rate,
) -> Result<u64, TimeError> {
    let end_ns = extent.nanos()?; // Unknown -> TimeError::Missing (taxonomy)
    let origin_ns = timestamp_to_nanos(origin)?;
    let end = origin_ns
        .checked_add(i128::from(end_ns.0))
```
What is wrong: the function computes `end = origin + extent` — treating `extent` as remaining-from-origin. The doc line says "until the end of the media extent — the end-of-media arithmetic the playback engine needs for `Ended` detection", which reads as extent = the media's TOTAL duration (the thing `TimingDurationKnown` carries); under that reading the end should be `extent` itself and the code overshoots by `origin`. The only test (400-408) uses `origin = 0`, the one value at which both interpretations agree — it cannot distinguish them, and no production caller exists to pin intent (F-001).
Why it matters: whichever caller eventually wires this has a 50/50 chance of passing the wrong `Duration` and silently getting an end-of-media that is `origin` too late (missed `Ended`, over-drained pump).
What correct looks like: either rename the parameter `remaining` and fix the doc, or take the total duration and compute `end = extent` (clamping `origin > end` to 0).
Confidence: HIGH on the ambiguity / the resolution is UNVERIFIABLE (no caller, no disambiguating test).
Note: severity held at S3 (not S2) only because the function is currently unreachable; the moment it gains a caller this is live drift.

### F-005 [S4]
Where: crates/media-clock/src/mapping.rs:261,266,270-271,277
Code:
```rust
        assert_eq!(
            timestamp_to_nanos(ts(90_000, 1, 90_000)).unwrap_or(0),
            1_000_000_000
        );
```
What is wrong: the `unwrap_or(0)` test idiom swallows the error variant — a regression that turns these into `Err` still fails the assert, but reports `left: 0` instead of the actual error, degrading diagnosis.
Why it matters: test hygiene; the neighboring tests use `unwrap_or_else(|e| unreachable!("{e}"))` — the file mixes both idioms inconsistently.
What correct looks like: one idiom (`expect`/`unwrap_or_else(unreachable)`) throughout.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `NANO_BASE` (const) | 50-53 | REAL | — | Used by clock.rs:77 — the live position base |
| `div_round` | 60-73 | REAL | S3 | Correct ties-away-from-zero; duplicated verbatim as clock.rs `checked_div_round` (F-002) |
| `overflow` | 79-85 | REAL | — | Saturating context ticks — correct |
| `timestamp_to_nanos` | 98-107 | REAL | — | The one production-used function (clock.rs:436); signed complement of `as_nanos`; hand-checked |
| `nanos_to_timestamp` | 115-118 | REAL | — | i64-range gate; no production caller (F-001) |
| `checked_rate` | 123-128 | REAL | — | Zero components → typed Overflow |
| `sample_delta_to_media_nanos` | 139-155 | REAL | S3 | Correct one-rational scaling; unused in production (F-001); the live twin rounds twice (F-002) |
| `map_samples_to_timestamp` | 165-177 | REAL | S3 | Correct checked composition; unused (F-001) |
| `map_timestamp_to_samples` | 189-217 | REAL | S3 | Exact algebraic inverse, verified; negative-target → typed Overflow (honest per u64 contract); unused (F-001) |
| `samples_until_end` | 228-241 | REAL | S3 | F-004 (ambiguous extent semantics; origin=0-only test) |
| tests (15) | 243-425 | REAL | — | Exact-valued, mutation-sensitive; see §5 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `timestamp_to_nanos_handles_bases_and_signs` (258-272) | YES | Exact values for 90 kHz, negative ticks, and both sub-ns rounding directions |
| `rate_scales_media_time` (312-322) | YES | Swapping num/den (the doc-03 formula) flips every assert |
| `rounding_is_deterministic_and_monotone_in_delta` (325-332) | YES | 22676/45351 pin ties-away exactly |
| `degenerate_rates_overflow_typed` (335-343) | YES | Zero rate_hz, zero num, zero den each asserted typed-error — removing `checked_rate`/denominator guard fails it |
| `extreme_sample_counts_overflow_typed` (346-359) | YES | u64::MAX at 1 Hz and 48 kHz both asserted err; 4-hour session asserted ok |
| `inverse_mapping_roundtrips_exactly` (373-382) | YES | 144000 ↔ 4.25 s exact roundtrip under 5/4 |
| `known_extent_maps_to_samples` (400-408) | WEAK | origin=0 — cannot distinguish extent-as-total from extent-as-remaining (F-004) |
| `unknown_extent_is_missing_per_taxonomy` (411-417) | YES | Missing variant pinned by match |
| Baseline: all green — media-clock absent from `orchrun-test-workspace-nofailfast.log.txt`'s 7 failures | — | cited |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| `now() = anchor.media + (device_samples - anchor.at_sample) / rate` | doc 03 §4.7(e):552-553 | FALSE | mapping.rs:21-30's own deviation analysis + clock.rs 390-393 (× num, / den) — F-003 |
| "exposed as pure checked functions so the engine crates … can map sample-counter positions" | module doc 4-6 | FALSE | No engine crate calls them (F-001) |
| "checked `i128` with ties-away-from-zero rounding — the same discipline as media-types' Timestamp::convert" | module doc 17-19 | REAL | div_round hand-checked; consistent with the media-types precedent it cites |
| "`samples_until_end` … the end-of-media arithmetic the playback engine needs for `Ended` detection" | doc lines 219-221 | REAL-UNPROVEN | Correct-looking arithmetic, ambiguous extent semantics (F-004), no caller |

## 7. Dead / duplicated / unreachable code
- Duplicated: `div_round` ↔ clock.rs `checked_div_round` (F-002).
- Unreachable-in-production: 5 of 8 items (F-001).
- No other dead code; the module is small and self-consistent.

## 8. Dependencies & flags
- Depends only on `media-types::time` types. No feature gates, no unsafe, no allocation.
- The module's correctness is hostage to `media-types`' `TimeBase`/`Rate`/`Duration` definitions (audited by the media-types wave — W02 found time.rs REAL).

## 9. Open questions for the next agent
- Q-1 (clock.rs owner, likely W03 peer): does the double-rounded live path ever produce a `now()` that differs by 1 ns from `sample_delta_to_media_nanos` on real 44.1/48 kHz streams? If yes, fold the clock onto the pure function (F-002's fix) — behavior change, needs a pinned test first.
- Q-2 (session.rs owner): when `Ended` detection needs "samples until end", which Duration will be passed — total or remaining? F-004's answer determines a rename or a fix.
- Q-3 (docs owner): doc 03 §4.7(e) correction (F-003) — the worklog entry the code cites should be linked from the spec.
