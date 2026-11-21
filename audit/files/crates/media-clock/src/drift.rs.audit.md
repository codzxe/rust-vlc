# AUDIT: crates/media-clock/src/drift.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-21 |
| Lines / functions | 312 / 9 production items + 7 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `DriftCorrection::for_divergence`/`apply` ← audio-engine `AudioPipeline::observe_drift`/`set_drift` (pipeline.rs:185-192, 162) ← apps/cli `AudioLimb::observe_drift` (pipeline_host.rs:613-623) ← the CLI host's audio limb — apps/cli depends on audio-engine, shipped in `lumen`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg "DriftCorrection\|for_divergence\|MAX_CORRECTION_PPM" crates/audio-engine` → consumers confirmed (pipeline.rs:24,83-87,162,169,185); `rg "audio_engine" apps/cli/src` → pipeline_host.rs:51; `sed -n '185,200p' audio-engine/pipeline.rs` + `'613,624p' apps/cli/src/pipeline_host.rs` → live chain; hand-checked div_round/extreme/apply arithmetic including i128 edges |

## 1. One-paragraph verdict
The drift limiter is small, exact, and honest: the ±1000 ppm bound is enforced by construction (clamped constructor), the divergence→ppm sign convention is correct and saturated in the right direction on every overflow edge I could construct (including the i128::MIN negation corner, which routes to the correct extreme), rate composition is exact reduced rational arithmetic with typed overflow, and the module's consumer claims are TRUE — audio-engine's governor/resampler consume it and the shipped CLI host drives the whole chain. Its 7 tests are exact-valued, cover the saturation extremes, and ran green in the no-fail-fast baseline. The single defect is hygiene: this file carries the third verbatim copy of the crate's `div_round` helper.

## 2. What actually works (verified)
- The limiter invariant: `new` clamps to ±1000 (59-63); test 190-199 pins saturation at i64::MAX/MIN inputs; test 254-283 proves the composed rate stays within the bound for i128-extreme divergences across three window sizes — a real invariant test, not a tautology.
- Sign convention: follower-behind (negative divergence) → positive ppm — verified by hand and by test 202-213 (`-1ms/10s → +100 ppm`).
- Overflow corners hand-checked: `checked_mul(1e6)` failure → `extreme(divergence)` preserves direction; `checked_neg` on an `i128::MIN` quotient → `extreme` receives the (negative) divergence and answers `i64::MAX` (speed-up) — the correct saturated direction; `div_round`'s internal `checked_mul(2)` cannot overflow here (`|r| < window ≤ u64::MAX << i128::MAX/2`) but is handled anyway.
- `apply` exactness: `(1e6+ppm)/1e6` composed via i128, gcd-reduced, u32-checked, `Rate::new`-validated — test 228-251 pins exact rationals (10001/10000, 999/1000, 5005/4000).
- Consumer claims (module doc 20-27) verified TRUE: audio-engine/pipeline.rs `set_drift` (162) feeds the micro-resampler, `observe_drift` (185-192) derives via the governor; apps/cli pipeline_host.rs:613-623 drives it with the sink's sample count and ring latency — a live shipped chain.
- Unit tests green in `orchrun-test-workspace-nofailfast.log.txt` (media-clock not among the 7 failures).

## 3. Findings

### F-001 [S3]
Where: crates/media-clock/src/drift.rs:146-161 (identical copies at mapping.rs:60-73 and clock.rs:414-428)
Code:
```rust
/// Divide with rounding, ties away from zero; `None` on a degenerate
/// denominator (the `media-types` discipline).
fn div_round(numer: i128, denom: i128) -> Option<i128> {
    if denom <= 0 {
        return None;
    }
```
What is wrong: this is the third verbatim copy of the same 12-line ties-away-from-zero division helper inside one crate (mapping.rs:60 `div_round`, clock.rs:414 `checked_div_round`, drift.rs:148 `div_round`). Each copy carries its own "the media-types discipline" comment; the original in media-types is crate-private (mapping.rs:57-59 says so).
Why it matters: three copies of a subtle rounding function is a divergence hazard — a rounding fix (e.g., a ties-to-even change for resampler symmetry) must land three times in this crate alone, and the copies already have two different names. My mapping.rs audit (W03-A10) caught the semantic twin problem (clock.rs double-rounds where mapping.rs single-rounds); this is the structural root.
What correct looks like: one `pub(crate) fn div_round` in a shared module (or make media-types' public), used by all three sites.
Confidence: HIGH — Verification: read all three sites (mapping.rs:60-73, clock.rs:414-428, drift.rs:148-161 — byte-identical logic).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MAX_CORRECTION_PPM` (const) | 35 | REAL | — | 1000, honestly marked [PROPOSAL] — final number deferred to Phase 3 |
| `PPM_SCALE` (const) | 38 | REAL | — | 1e6 |
| `DriftCorrection` (struct) + `ZERO` | 45-52 | REAL | — | Private ppm field — bound holds by construction |
| `DriftCorrection::new` | 59-63 | REAL | — | Clamp to ±bound — the limiter |
| `DriftCorrection::ppm` | 66-68 | REAL | — | Accessor |
| `DriftCorrection::for_divergence` | 78-102 | REAL | — | Zero-window → ZERO (no div-by-zero, no fabricated max); all i128 edges checked and direction-correct (hand-verified) |
| `DriftCorrection::apply` | 113-132 | REAL | — | Exact rational; degenerate rate rejected typed; u32-fit checked; no unwrap |
| `Display` | 135-139 | REAL | — | `+100 ppm` — tested |
| `extreme` | 142-144 | REAL | — | Direction-correct saturation helper |
| `div_round` | 148-161 | REAL | S3 | F-001 (third copy) |
| `gcd128` | 163-169 | REAL | — | debug_assert-only positivity; unreachable-by-construction (rejections at 114) — acceptable |
| `rate_overflow` | 173-179 | REAL | — | Diagnostic TimeBase{1,1} fields — documented as diagnostic; cosmetic |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `new_applies_the_limiter` (190-199) | YES | Saturation at ±1000 pinned incl. i64 extremes — removing the clamp fails it |
| `divergence_signs_speed_the_right_way` (202-213) | YES | Sign flip (dropping the negation at 91) swaps every assert |
| `divergence_saturates_at_the_bound` (216-225) | YES | i128 extremes → ±bound — removing any checked step panics or mis-saturates |
| `apply_composes_exactly` (228-251) | YES | Exact reduced rationals — a float path or unreduced result fails |
| `apply_stays_within_the_bound` (254-283) | YES | The invariant itself, cross-product form, over extreme × window grid |
| `apply_overflows_typed_on_unrepresentable_rates` (286-304) | YES | u32::MAX rate and pub-field degenerate rates each asserted typed-err |
| `display_is_signed_and_bounded` (307-311) | YES | Format pinned |
| Baseline: green — media-clock absent from the 7 failures in `orchrun-test-workspace-nofailfast.log.txt` | — | cited |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "correction rate limited to ±1000 ppm [PROPOSAL]" | doc 03 §4.7(e); module doc 8-10 | REAL | Clamp at 61; invariant test 254-283; proposal status honestly carried at 32-34 |
| "audio-engine's MicroResampler consumes a stored DriftCorrection … DriftGovernor derives one per window via for_divergence" | module doc 20-25 | REAL | audio-engine/pipeline.rs:24, 83-87, 162, 185-192 read |
| "composes onto a playback Rate exactly (rational arithmetic, no float drift)" | module doc 14-16 | REAL | apply (113-132) + exact-rational test 228-251 |
| "The clock itself is never drift-corrected … never set_rate" | module doc 26-27 | REAL | DriftCorrection::apply answers a NEW Rate to the resampler; no clock call site exists in this module (traced) |

## 7. Dead / duplicated / unreachable code
- Duplicated: `div_round` (F-001) — third copy in-crate.
- No dead functions; every item has callers (audio-engine chain) or is a tested constructor/accessor.

## 8. Dependencies & flags
- Depends only on `media_types::time` types. No unsafe, no allocation (Copy struct throughout), no feature gates.
- The `[PROPOSAL]` ±1000 constant is the module's single tunable; changing it is a one-line, one-test change (doc 34) — verified: only `new`'s clamp and the invariant test reference it.

## 9. Open questions for the next agent
- Q-1 (audio-engine owner): does `DriftGovernor::observe` (governor.rs) pass a window large enough that ±1000 ppm saturation is rare rather than the steady state? A saturating limiter on a persistently drifting sink means uncorrected drift — the governor's windowing policy is the untested half of the invariant.
- Q-2 (media-types owner): consider exporting the original `div_round_i128` publicly to kill the three in-crate copies (F-001's root fix).
