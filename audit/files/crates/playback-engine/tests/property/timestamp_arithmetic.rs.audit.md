# AUDIT: crates/playback-engine/tests/property/timestamp_arithmetic.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 623 / 4 tests + 2 oracles + 5 checkers + generators |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | N/A — property test binary over media-types' public time surfaces (Timestamp::convert, as_nanos, Scaler tiers, TimeBase/Stamp discipline). W00 baseline: 1751/2/25; individual result UNVERIFIED (no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | `cat` full file (exit 0). No cargo. |

## 1. One-paragraph verdict

The strongest test file in my assignment and the model the weaker oracles should follow: the conversion oracle is **independently authored u128 arithmetic with no shared code with the crate** (the header's claim, and the code matches it — hand-rolled quotient/remainder half-away rounding, its own overflow rules including the −2^63 corner), fed by genuinely adversarial generators (i64 edges and near-edges, near-MAX PTS clusters, power-of-two magnitudes, degenerate struct-literal time bases with zero components, demuxer-common rates). Every answer must be exactly the oracle's value or the typed `TimeError::Overflow` with its context payload verified field-by-field — "impossible error variant" arms fail the test. The Scaler tiers are differentially checked against the one-shot forms on every input; round-trips carry a derived two-rounding drift bound; monotonicity and sign preservation are checked on pairs. Two minor notes: the round-trip drift bound's derivation is stated but not itself pinned by a proof (a too-loose bound would silently weaken the law), and `check_roundtrip`/`check_monotone` skip degenerate bases entirely (correct — not a contract — but the skip means those ops are no-ops ~37% of the time given the generator mix).

## 2. What actually works (verified)

- `oracle_convert`/`oracle_nanos`/`rounded_i64` (55-113): independent u128 math; the documented subtleties (zero-divisor degenerate literals → typed error; zero multiplier → exactly 0; negative tick whose magnitude rounds to zero → Ok(0); only a non-zero rounded negative is a u64 overflow) are each encoded and each exercised by the generators.
- `check_convert`/`check_nanos` (268-349): three-way verdicts — Ok must equal the oracle exactly; Overflow must be oracle-None AND carry the full (from, to, ticks) context; any other variant fails.
- `check_scaler` (351-418): three tiers (scaler_to convert_ticks, scaler nanos, the dedicated nanos_scaler) each checked against the one-shot forms for value and error — the fast-path tiers cannot drift from the reference.
- `negative_offsets_are_legal_ticks` (598-623): the DTS-below-PTS-below-zero shape as pure arithmetic against the oracle — pins that negative ticks are data, not errors.
- `base_construction_is_typed_and_normalizing` (540-569): zero components refuse typed; 2/180000 ≡ 1/90000 ≡ 90/8100000; Rate follows the same laws.

## 3. Findings

### F-001 [S4] The round-trip drift bound is asserted but not independently pinned
Where: crates/playback-engine/tests/property/timestamp_arithmetic.rs:437-447
Code (verbatim):
```rust
    // Two half-away roundings over a rate ratio r = num/den bound the
    // homecoming drift by 0.5/r + 0.5 input ticks (the forward error
    // scales by the inverse rate, the backward error is half an input
    // tick): drift ≤ (num + den) / (2·num), integer division on both
    // sides — i128 for the difference so no magnitude is lost, u128
    // for the bound so no product wraps.
```
What is wrong: nothing demonstrably — the derivation reads correctly (forward half-output-tick error scaled by the inverse ratio plus backward half-input-tick) — but if the bound formula were too loose, the test would still pass and nothing would notice; too tight and it fails spuriously (which would be caught). The bound has no second derivation or brute-force cross-check. MEDIUM confidence in the bound itself; HIGH confidence the check runs.
Confidence: MEDIUM (bound derivation re-derived by reading, not executed).

### F-002 [S4] Degenerate-base ops are silent no-ops in two checkers
Where: 420-425 (`check_roundtrip` returns Ok on any zero component), 462-464 (`check_monotone` same)
What is wrong: nothing — skipping non-contract shapes is correct — but with `raw_base` producing a zero component in 3 of 8 draws, ~37% of generated RoundTrip/Monotone ops do nothing, and the op stream does not record the skip (the failure context would, if one ever fired). Hygiene: a counter of skipped ops in the final message would make the coverage visible.
Confidence: HIGH — Verification: read + arithmetic on the generator mix.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `oracle_convert` | 55-69 | REAL | — | independent u128; degenerate rules encoded |
| `oracle_nanos` | 76-95 | REAL | — | negative-rounds-to-zero subtlety encoded |
| `rounded_i64` | 99-113 | REAL | — | −2^63 corner included |
| `nanos_base` | 118-120 | REAL | — | NANOS_BASE spelled out |
| `raw_base` | 130-149 | REAL | — | 9 common rates + degenerate shapes + uniform |
| `adversarial_ticks` | 153-178 | REAL | — | edges, near-MAX clusters, powers of two, uniform |
| `build_base` | 183-192 | REAL | — | struct literal for degenerate shapes (public-field escape hatch exercised) |
| `TimeOp`/`generate_ops` | 200-266 | REAL | — | 5 op families, stateless (prefix shrinking exact) |
| `check_convert` | 268-309 | REAL | — | three-way verdict + context-payload check |
| `check_nanos` | 311-349 | REAL | — | same |
| `check_scaler` | 351-418 | REAL | — | three tiers vs one-shot, differential |
| `check_roundtrip` | 420-454 | REAL | S4 | derived bound (F-001); degenerate skip (F-002) |
| `check_monotone` | 456-487 | REAL | S4 | monotone + sign preservation; degenerate skip |
| `run` | 489-515 | REAL | — | stateless ops; op index + payload in failures |
| `conversions_answer_typed_errors_or_exact_values_never_panics` | 521-530 | REAL | — | 64 seeds × 512 ops |
| `base_construction_is_typed_and_normalizing` | 539-569 | REAL | — | 512 random constructs + zero refusals + normalization |
| `stamp_absence_is_typed_never_fabricated` | 574-591 | REAL | — | Missing is a type; UNKNOWN span both ends |
| `negative_offsets_are_legal_ticks` | 597-623 | REAL | — | 256 negative-tick conversions vs oracle |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| conversions_answer_typed_errors_or_exact_values_never_panics | YES (reasoned) | exact-oracle asserts on value AND error-context; a wrap, panic, wrong rounding, or lost context fails with seed+op |
| base_construction_is_typed_and_normalizing | YES (reasoned) | exact equality across scaled pairs; typed zero refusals with message check |
| stamp_absence_is_typed_never_fabricated | YES (reasoned) | exact Err(Missing) |
| negative_offsets_are_legal_ticks | YES (reasoned) | oracle-differential on the negative corner |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "an independently authored u128 oracle — no shared code with the crate" | header 6-7 | REAL | oracle is hand-rolled u128; verified it calls nothing from media-types |
| "wrap-around must be impossible: overflow is a typed error" | header 11-12 | REAL-UNPROVEN | near-MAX clusters in the generator + oracle-None agreement; run UNVERIFIED |
| Scaler tiers are one contract | header 25-26 | REAL-UNPROVEN | differential checks present; run UNVERIFIED |

## 7. Dead / duplicated / unreachable code

None.

## 8. Dependencies & flags

media-types, testkit, shared rng. No features. This suite is the correct pattern that engine_fx's `resolve_target_ns` (my engine_fx F-001) failed to follow — same repo, both styles exist.

## 9. Open questions for the next agent

- media-types' time.rs: does `div_round_i128`'s reference actually implement half-away-from-zero as the oracle assumes? The oracle-vs-engine agreement is tested, so any run failure answers it — but the run result is UNVERIFIED here (no cargo). File: crates/media-types/src/time.rs.
