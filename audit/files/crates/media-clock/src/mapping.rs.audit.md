# AUDIT: crates/media-clock/src/mapping.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 425 / NANO_BASE, div_round, overflow, timestamp_to_nanos, nanos_to_timestamp, checked_rate, sample_delta_to_media_nanos, map_samples_to_timestamp, map_timestamp_to_samples, samples_until_end + 15 tests (243-425, 43%) |
| File verdict | **REAL math, mostly UNREACHABLE exports** — `NANO_BASE` and `timestamp_to_nanos` are production-live; the four sample-mapping functions and `nanos_to_timestamp` have zero callers outside the module (the clock re-implements equivalent math inline instead of calling them) |
| Reachable from shipped entry point? | YES for `NANO_BASE` (dozens of production sites: apps/cli containers, demux_host.rs:178/558, apps/web host.rs:392/1739, apps/desktop bridge.rs:315) and `timestamp_to_nanos` (clock.rs:77→436 `position_nanos`, on the MediaClock read path every host drives). NO for `sample_delta_to_media_nanos` / `map_samples_to_timestamp` / `map_timestamp_to_samples` / `samples_until_end` / `nanos_to_timestamp`: rg tree-wide finds callers only inside mapping.rs (internal + tests) and the lib.rs:123-124 re-export. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg (per-function callers across the workspace; NANO_BASE sites); sed reads of clock.rs:388-428/670-700. No cargo (static). |

## 1. One-paragraph verdict

The arithmetic is correct, checked, and adversarially tested: i128 intermediates throughout, ties-away-from-zero rounding (`div_round`, verified against -2.5→-3 and 0.5 ns→1), typed `TimeError::Overflow`/`Missing` for every degenerate or unrepresentable case (zero rate components, u64::MAX sample counts, ±292-year position caps, target-before-origin), and 15 tests with exact-value assertions that would fail under any mutation of the scaling or rounding. The hostile question — "real clock math or dead" — answers as: real math, half-dead surface. `NANO_BASE` is the workspace's universal time base and `timestamp_to_nanos` feeds the live `MediaClock` read path; but the four functions the module doc says exist "so the engine crates (and tests) can map sample-counter positions without going through a clock instance" are called by no engine crate — `clock.rs` re-implements the identical scaling inline (`read_source` at 683-691, rate scaling at 388-396) rather than calling them, and `checked_div_round` (clock.rs:415-428) is a third verbatim copy of this file's `div_round` (mapping.rs:60-73), whose own comment already apologizes for the second copy.

## 2. What actually works (verified)

- `timestamp_to_nanos` (98-107): `ticks × num × 1e9 / den` with checked i128 muls and rounded division — verified by hand for the 90 kHz → 1 s case; production-live via clock.rs:436 (`position_nanos`).
- `sample_delta_to_media_nanos` (139-155): `delta × 1e9 × num / (rate_hz × den)` as one rational rounded once — verified by hand against the tests' values (48 000 samples @48 kHz 5/4 → 1.25 s; 4×48 000 → 5 s).
- `map_timestamp_to_samples` (189-217): the exact inverse rational; round-trip test (373-382) recovers 144 000 exactly.
- `samples_until_end` (228-241): `Unknown` extent → `TimeError::Missing` (absence is data — MP §8), no guessed 0/-1.
- `NANO_BASE` (50-53): used by every host's container glue and clock conversions (rg: apps/cli/src/containers/*.rs, demux_host.rs, pipeline_host.rs:1607/1632, apps/web, apps/desktop).
- W00 baseline: media-clock tests pass in the workspace run (1751 pass; crate section of the trimmed log does not show them individually — UNVERIFIED individually, consistent with pass).

## 3. Findings

### F-001 [S3] — four of six public functions are dead exports; the clock re-implements their math inline
Where: crates/media-clock/src/mapping.rs:139-241 (the functions), module-doc premise at 4-6
Code (mapping.rs:4-6):
```rust
//! The audio-master clock's arithmetic, exposed as pure checked
//! functions so the engine crates (and tests) can map sample-counter
//! positions without going through a clock instance:
```
Code (clock.rs:683-691 — the inline twin):
```rust
            // ns = sample x 1e9 / rate_hz, checked; a zero rate is
            // handled above so the division is well-defined.
            let numer = i128::from(sample).checked_mul(1_000_000_000);
            match numer.and_then(|n| checked_div_round(n, i128::from(rate_hz))) {
```
What is wrong: rg tree-wide for `sample_delta_to_media_nanos|map_samples_to_timestamp|map_timestamp_to_samples|samples_until_end|nanos_to_timestamp` outside media-clock/src/mapping.rs: only the lib.rs re-export (123-124) — zero engine/host callers. `clock.rs` implements the absolute-count variant of the same scaling in `read_source` (683-691) and the rate scaling in its `now` math (388-396) with its own `checked_div_round`, so the "pure functions" are a parallel implementation of live math, not the live math.
Why it matters: drift risk — a rounding or overflow fix in one copy will not reach the other (three copies of the ties-away division now exist: media-types time.rs, mapping.rs `div_round`, clock.rs `checked_div_round`). The module doc's stated consumer ("the engine crates") does not exist.
What correct looks like: clock.rs calls the mapping functions (or mapping.rs is demoted to test-support and the doc says so).
Confidence: HIGH — Verification: read (full caller trace).

### F-002 [S4] — `div_round` triplication inside one crate
Where: crates/media-clock/src/mapping.rs:57-73 vs crates/media-clock/src/clock.rs:415-428
Code (mapping.rs:60-73):
```rust
fn div_round(numer: i128, denom: i128) -> Option<i128> {
    if denom <= 0 {
        return None;
    }
    let (q, r) = (numer / denom, numer % denom);
    let twice_r = r.checked_mul(2)?;
    if twice_r >= denom {
        q.checked_add(1)
    } else if twice_r <= -denom {
        q.checked_sub(1)
    } else {
        Some(q)
    }
}
```
What is wrong: clock.rs:415-428 `checked_div_round` is character-for-character the same function (same branches, same comment style, same "media-types discipline" citation); mapping.rs:58-59 already notes it is a re-implementation "because the original is crate-private" — within the same crate the copy is unnecessary.
Why it matters: the file's own hygiene commentary condemns the pattern it participates in.
What correct looks like: one `pub(crate)` helper in mapping.rs, used by clock.rs.
Confidence: HIGH — Verification: read (both bodies compared).

### F-003 [S4] — degenerate rate reported as `Overflow` with magnitude 0
Where: crates/media-clock/src/mapping.rs:123-128
Code:
```rust
fn checked_rate(rate: Rate) -> Result<Rate, TimeError> {
    if rate.num == 0 || rate.den == 0 {
        return Err(overflow(NANO_BASE, NANO_BASE, 0));
    }
    Ok(rate)
}
```
What is wrong: an invalid argument (zero-component rate) is reported through the `Overflow` taxonomy variant with `ticks: 0` — "cannot be represented" is a stretch for "input was nonsense"; the module doc (36-40) defends the choice, but the error carries no signal distinguishing it from a genuine magnitude overflow.
Why it matters: diagnostics only; behavior is safe either way (typed error, no panic — the important property holds).
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| NANO_BASE | 50-53 | REAL | — | universal base; dozens of production sites |
| div_round | 60-73 | REAL (dead-copied) | S4 | correct ties-away; triplicated (F-002) |
| overflow | 79-85 | REAL | — | saturating magnitude for context |
| timestamp_to_nanos | 98-107 | REAL | — | checked i128; live via clock.rs position_nanos |
| nanos_to_timestamp | 115-118 | UNREACHABLE | — | no external caller; i64 tick-range cap correct |
| checked_rate | 123-128 | UNREACHABLE | S4 | F-003 |
| sample_delta_to_media_nanos | 139-155 | UNREACHABLE | S3 | math correct; zero callers (F-001) |
| map_samples_to_timestamp | 165-177 | UNREACHABLE | S3 | composes the two; zero callers |
| map_timestamp_to_samples | 189-217 | UNREACHABLE | S3 | exact inverse; negative-delta → typed error; zero callers |
| samples_until_end | 228-241 | UNREACHABLE | S3 | Unknown → Missing (taxonomy-faithful); zero callers |

## 5. Test honesty (15 tests, lines 243-425)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| timestamp_to_nanos_handles_bases_and_signs (258) | YES | 90 kHz base, negative ticks, sub-ns ties (0.5→1, 1/3→0) |
| nanos_roundtrip_through_nano_base (275) | YES | lossless round-trip incl. base recovery |
| extreme_timestamps_overflow_typed (290) | YES | i64::MAX×1000 ticks overflow; both i64 extremes ok |
| one_second_of_samples_is_one_second_at_rate_one (304) | YES | 48k/44.1k identities |
| rate_scales_media_time (312) | YES | 5/4, 1/2, 2/1 exact values |
| rounding_is_deterministic_and_monotone_in_delta (325) | YES | 1 and 2 samples at 44.1 kHz exact (22676, 45351) — catches rounding-mode flips |
| degenerate_rates_overflow_typed (335) | YES | rate_hz=0, num=0, den=0 (pub-field bypass — adversarial) |
| extreme_sample_counts_overflow_typed (346) | YES | u64::MAX at 1 Hz and 48 kHz both typed-err; 4 h ok |
| forward_mapping_composes_origin_and_delta (364) | YES | origin+delta exact |
| inverse_mapping_roundtrips_exactly (373) | YES | forward∘inverse = identity at 5/4 |
| inverse_mapping_rejects_target_before_origin (385) | YES | negative-delta → error |
| inverse_mapping_overflows_typed_for_huge_targets (392) | YES | ~9.2e21 s target |
| known_extent_maps_to_samples (401) | YES | 2× rate halves the counter extent (240 000) |
| unknown_extent_is_missing_per_taxonomy (411) | YES | variant-specific match |
| extent_overflow_is_typed (420) | YES | origin near i64::MAX + u64::MAX ns |

All UNVERIFIED by execution (no cargo); docs/gates/logs/* record mapping tests passing in historical runs. The assertions are exact-value, not range — mutation-resistant by construction.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "exposed as pure checked functions so the engine crates (and tests) can map sample-counter positions" | mapping.rs:4-6 | FALSE (engine-crates half) | rg: no engine-crate caller; only tests (F-001) |
| "All arithmetic is checked i128 with ties-away-from-zero rounding" | mapping.rs:17-18 | REAL | verified by reading every function |
| rate-convention deviation from doc 03 §4.7(e) recorded | mapping.rs:21-30 | REAL | the deviation is implemented and tested (rate_scales_media_time) |
| "conversion failure is data, never a panic, never a wrap" | mapping.rs:18-19 | REAL | no unwrap/panic in the module; all paths typed |

## 7. Dead / duplicated / unreachable code

- Dead exports: `nanos_to_timestamp`, `sample_delta_to_media_nanos`, `map_samples_to_timestamp`, `map_timestamp_to_samples`, `samples_until_end` (F-001).
- Duplicated: `div_round` ↔ clock.rs `checked_div_round` (F-002); both mirror media-types' crate-private original.

## 8. Dependencies & flags

- Depends only on `media_types::time` (Duration, Rate, TimeBase, TimeError, Timestamp). No features, no unsafe, no allocation anywhere (pure integer code — D-1 satisfied by construction).

## 9. Open questions for the next agent

- clock.rs owner: `read_source`'s absolute-count scaling vs `sample_delta_to_media_nanos`' delta scaling — a shared helper would kill the triplication; check whether clock.rs's inline math has the same rounding (it does — `checked_div_round`) so no behavioral divergence exists today.
- Wave 16 (test audit): mapping.rs's tests are the model the rest of the repo should follow — worth citing as the positive control.
