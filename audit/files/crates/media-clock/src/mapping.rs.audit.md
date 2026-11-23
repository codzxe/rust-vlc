# AUDIT: crates/media-clock/src/mapping.rs
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-23 |
| Lines / functions | 425 / 12 (7 pub fns + 3 private helpers + 15 unit tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: `lumen open` → `apps/cli/src/pipeline_host.rs:1605 MediaClock::audio_master` (the null-sink counter clock) → `crates/media-clock/src/clock.rs:77,436 timestamp_to_nanos` (and `backends.rs:91 NANO_BASE`); the full mapping surface is also re-exported at `crates/media-clock/src/lib.rs:122-124`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/media_clock-7012bdf535a3032b mapping` → 15 passed, 0 failed, finished in 0.00s, exit 0. Manual div_round tie arithmetic verified by hand (±x.5 cases). |

## 1. One-paragraph verdict
Pure, checked arithmetic with no hidden state and no panics: every function is `i128`-checked end to end, degenerate inputs (zero sample rate, zero-component rate, negative inverse deltas, unknown extents) surface as typed `TimeError`s, rounding is ties-away-from-zero and deterministic, and the forward/inverse sample↔timestamp mappings round-trip exactly when values divide evenly. I executed all 15 unit tests (pass) and hand-verified the tie-breaking of `div_round` in both signs. The rate-semantics deviation from doc 03 §4.7(e)'s sketch formula is explicitly documented in the module header with the dimensional argument and pinned by tests — the correct way to deviate. The only blemish is taxonomy cosmetics: a zero-component rate is reported as an `Overflow` of a NANO_BASE→NANO_BASE conversion of 0 ticks, which misdescribes the failure.

## 2. What actually works (verified)
- `./target/debug/deps/media_clock-7012bdf535a3032b mapping` → `test result: ok. 15 passed; 0 failed` — bases/signs, round-trips, rate scaling (5/4, 1/2, 2/1), rounding determinism, degenerate rates (including pub-field-constructed `Rate { num: 0 }` bypassing `Rate::new`), extreme counts (u64::MAX at 1 Hz and 48 kHz), inverse rejections, extent taxonomy (Known/Unknown/overflow).
- Hand-check of `div_round` (60-73): (+5,2)→3, (−3,2)→−2 — ties away from zero in both signs; `denom <= 0` refuses (never divides by zero or flips sign).
- Inverse-mapping arithmetic re-derived by hand: origin 0.5 s + 144 000 samples @48 kHz @5/4 → 4.25 s; back-mapping 3.75e9 ns × 4 × 48000 / (5 × 1e9) = 144 000 — matches the code path (207-216) and the test (373-382).

## 3. Findings
F-001 [S4]
Where: crates/media-clock/src/mapping.rs:123-128
Code:
```
fn checked_rate(rate: Rate) -> Result<Rate, TimeError> {
    if rate.num == 0 || rate.den == 0 {
        return Err(overflow(NANO_BASE, NANO_BASE, 0));
    }
    Ok(rate)
}
```
What is wrong: a zero-component rate is reported as `TimeError::Overflow { from: NANO_BASE, to: NANO_BASE, ticks: 0 }` — an error claiming a zero-tick nanosecond→nanosecond conversion overflowed. The failure is an invalid rate, not a magnitude problem; a triage consumer reading the error fields is misled (the real culprit, the rate, appears nowhere in the error). The module docs (32-40) own this as a taxonomy compromise ("the taxonomy's 'cannot be represented' variant"), and clock-side storage is validated at `set_rate`, so the reachable blast radius is callers passing hand-constructed `Rate` pub-field values. What correct looks like: a `TimeError` variant (or `Invalid`-class kind) naming the degenerate rate, or at minimum a nonzero magnitude marker. Confidence: HIGH — Verification: read + ran (degenerate_rates_overflow_typed covers the behavior, not the error's honesty).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `div_round` | 60-73 | REAL | — | Checked ties-away-from-zero; `denom <= 0` → None. Hand-verified. |
| `overflow` | 79-85 | REAL | — | Saturating magnitude for context field only. |
| `timestamp_to_nanos` | 98-107 | REAL | — | ticks×num×1e9/den, all checked; signed (negative positions legal). |
| `nanos_to_timestamp` | 115-118 | REAL | — | i64 tick-range gate (±292 y); tested at both i64 extremes. |
| `checked_rate` | 123-128 | REAL | S4 | F-001 taxonomy cosmetics. |
| `sample_delta_to_media_nanos` | 139-155 | REAL | — | One rational, one rounding: delta×1e9×num/(rate_hz×den); rate_hz=0 → denom 0 → typed error. |
| `map_samples_to_timestamp` | 165-177 | REAL | — | origin + delta, checked add, nano-base result. |
| `map_timestamp_to_samples` | 189-217 | REAL | — | Inverse; negative delta → typed error (u64 unrepresentable); single rational. Exact-inverse test verified. |
| `samples_until_end` | 228-241 | REAL | — | `Duration::Unknown` → `TimeError::Missing` (absence is data, MP §8) — verified by test 411-417. |
| 15 unit tests | 243-425 | REAL | — | All exact-value asserts incl. adversarial extremes; all executed. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| All 15 | YES | Exact-value assertions (22_676/45_351 rounding, 4_250_000_000 forward, 144_000 inverse, 240_000 extent, i64 boundary accepts, >i64 rejects). A wrapping `as` cast, a wrong rounding mode, a swapped num/den, or a silent 0 for Unknown each fail a specific test. No tautologies; the degenerate-rate test constructs invalid rates through pub fields specifically to bypass the validating constructor. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "All arithmetic is checked i128 with ties-away-from-zero rounding" | mapping.rs:17-19 | REAL | Every op is `checked_*`; div_round tie logic verified by hand + tests. |
| "Doc 03 §4.7(e) sketches now() = anchor.media + (device_samples − anchor.at_sample) / rate. That formula is only dimensionally consistent if 'rate' means seconds of output per second of media. Lumen's Rate is the opposite convention … scales as media_elapsed = counter_elapsed × rate.num / rate.den" | 21-30 | REAL (documented deviation) | `sample_delta_to_media_nanos` implements num/den exactly; `rate_scales_media_time` pins 5/4 → 4 counter-s = 5 media-s. The doc-vs-code conflict is surfaced, argued, and recorded — the honest posture. |
| "A zero sample rate or a zero-component Rate … are reported as TimeError::Overflow" | 32-40 | REAL (with F-001's honesty caveat) | Behavior verified; error-shape cosmetics flagged S4. |
| "absence is data, never 0, never −1 (MP §8)" | 13-15 | REAL | `samples_until_end` propagates Missing; test 411-417. |

## 7. Dead / duplicated / unreachable code
- None. `div_round` is a documented re-implementation of media-types' crate-private `div_round_i128` (the comment at 57-59 says why) — acceptable duplication with justification.

## 8. Dependencies & flags
- `media_types::time` only (Duration, Rate, TimeBase, TimeError, Timestamp). No std-heavy deps, no features, no unsafe. Pure functions — trivially testable, and tested.

## 9. Open questions for the next agent
- clock.rs:436 wraps `timestamp_to_nanos` in `i64::try_from` with its own Overflow — confirm no double-reporting confusion when mapping already typed-errors (cosmetic overlap only).
- `Rate` having pub fields that bypass `Rate::new` validation is a media-types design fact this module compensates for — the compensation is tested here; the root cause (field visibility) is media-types' business.
