# AUDIT: crates/media-clock/src/drift.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 312 / MAX_CORRECTION_PPM, PPM_SCALE, DriftCorrection::{ZERO, new, ppm, for_divergence, apply}, Display, extreme, div_round, gcd128, rate_overflow + 7 tests (181-312, 42%) |
| File verdict | **REAL code, dormant in the shipped product** — only `DriftCorrection::ZERO` is ever constructed at runtime; `for_divergence`'s sole production caller is the never-armed governor; `apply` has no production caller at all |
| Reachable from shipped entry point? | PARTIAL. `DriftCorrection::ZERO` flows through the live audio pipeline (`pipeline.rs:129` initial `drift`, `set_drift`/`reset_drift` on the production pump path via `observe_drift`'s early-return). `for_divergence`: only caller governor.rs:203 (unreachable in product — see my governor.rs audit F-001 and W03-A02's sync.rs F-001). `apply`: rg across crates/apps finds zero production callers (the resampler consumes `ppm` directly as its ratio; the video side uses the sync ladder, not rate composition). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg (`DriftCorrection|for_divergence|set_correction|.apply(` across crates; ZERO-construction sites in pipeline/resample); hand-verification of the rational arithmetic. No cargo (static). |

## 1. One-paragraph verdict

The arithmetic is correct and the limiter invariant is airtight: `new` clamps to ±1000 ppm (i64::MAX/MIN inputs included, tested), `for_divergence` computes `-(divergence × 1e6 / window)` in fully checked i128 with direction-consistent saturation on every overflow path (verified by hand against the i128-extreme tests), zero windows answer ZERO rather than dividing, and `apply` composes onto a `Rate` in exact reduced rational arithmetic (5/4 + 1000 ppm = 5005/4000 — verified) with typed errors for degenerate or unrepresentable rates. But in the shipped product this module is a loaded weapon with nobody holding it: the only producer of non-zero corrections is the audio governor, which no shipped composition arms (my governor.rs F-001; W03-A02's sync.rs F-001 — three-agent consensus), and `DriftCorrection::apply` — the doc's stated mechanism by which "consumers apply the correction" — has zero production callers, because the micro-resampler consumes `ppm` directly as its ratio and the video side never composes rates. The ±1000 ppm invariant is therefore never exercised in production by anything but ZERO.

## 2. What actually works (verified)

- `new` (59-63): clamp; `DriftCorrection::new(i64::MIN).ppm() == -MAX` (test 198).
- `for_divergence` (78-102): checked mul → rounded divide → checked neg → i64 fit, with `extreme(follower_minus_master_ns)` saturating in the correct direction on every failure arm (divergence < 0 = follower behind ⇒ speed up ⇒ +MAX; tests 221-224 pin both i128 extremes).
- `apply` (113-132): `rate.num × (1e6+ppm) / (rate.den × 1e6)`, gcd-reduced, u32-fit-checked, `Rate::new` revalidated; degenerate zero-component rates rejected typed (114-116). Hand-verified: 1 + 100 ppm = 10001/10000; 5/4 + 1000 ppm = 5005/4000.
- `gcd128`'s `debug_assert!(a > 0 && b > 0)` (164): precondition proven — `factor_num ≥ 999000 > 0` and `Rate` fields are u32 with `num == 0` rejected at 114, so both gcd inputs are strictly positive on every reachable path.
- The bound-invariant test (254-283) checks the composed rate against `1 ± 1000 ppm` over an 8×3 divergence×window matrix including both i128 extremes — the strongest test in the file.
- `ZERO` is live in the product: pipeline.rs:129 constructs it; `reset_drift`/`observe_drift` re-assert it on the production pump path.

## 3. Findings

### F-001 [S3] — `DriftCorrection::apply` has no production caller; the doc's "consumers apply the correction" describes a mechanism nothing uses
Where: crates/media-clock/src/drift.rs:16-18 (claim), 113-132 (the dead function)
Code (drift.rs:16-18):
```rust
//! is the clock itself: nothing here resamples audio or schedules video; the
//! consumers (audio-engine, video-engine) apply the correction, and
//! they can rely on it never exceeding the bound.
```
What is wrong: rg across crates/ and apps/ for callers of `DriftCorrection::apply` (a `Rate → Rate` composition): none outside drift.rs's own tests. The audio consumer (MicroResampler, resample.rs:124 `set_correction`) consumes the `ppm` field directly as its resample ratio (`out = in × (1e6 + ppm)/1e6` per the module doc's own description) — it never calls `apply`. The video consumer (PresentationScheduler) uses the sync-band ladder over the estimator, not rate composition. So the one function that turns a correction into a playback rate is dead code.
Why it matters: dormant-by-design would be fine if documented as such; instead the module doc asserts a consumption mechanism ("composes onto a playback Rate") that no consumer performs. Anyone wiring rate-based drift absorption would reach for `apply` and find it untested against any real consumer path.
What correct looks like: either a consumer composes rates through `apply` (e.g. the clock's master-rate math) or the doc says "the resampler consumes ppm directly; apply is the composition utility for future rate-based absorbers".
Confidence: HIGH — Verification: read (full caller trace).
Severity: S3 (dead export + misleading doc), not S2 — the limiter invariant itself is not weakened; only the composition path is unused.

### F-002 [S4] — `div_round` is now the fourth copy of the same function in the workspace
Where: crates/media-clock/src/drift.rs:146-161
Code:
```rust
/// Divide with rounding, ties away from zero; `None` on a degenerate
/// denominator (the `media-types` discipline).
fn div_round(numer: i128, denom: i128) -> Option<i128> {
```
What is wrong: identical body to mapping.rs:60-73 (this agent's earlier finding, F-002 there) and clock.rs:415-428 (`checked_div_round`), both of which cite the crate-private media-types original. Four copies of a rounding-division helper whose correctness everything downstream leans on.
Why it matters: one rounding-mode fix would have to land four times.
Confidence: HIGH — Verification: read (bodies compared across the three media-clock copies).

### F-003 [S4] — the ±1000 ppm value's `[PROPOSAL]` promise ("final number from Phase 3 measurements") was never resolved
Where: crates/media-clock/src/drift.rs:8-10, 31-35
Code:
```rust
//! **limiter**: "correction rate limited to ±1000 ppm
//! `[PROPOSAL — final number from Phase 3 measurements; the limiter
//! itself is the invariant, not the value]`".
```
What is wrong: Phase 3 exited (docs/gates/phase3-exit.md) with no recorded measurement revisiting the constant; the tag survives into a shipped invariant. The comment's own escape hatch ("Changing it is a one-line, one-test change") remains the only verification the value ever got.
Why it matters: unresolved [VERIFY]/[PROPOSAL] tags in invariant constants are how guessed numbers become contracts.
Confidence: HIGH — Verification: read + gate-log grep (no measurement recorded).

## 4. Per-function table

| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MAX_CORRECTION_PPM | 35 | REAL-UNPROVEN | S4 | the invariant; value still [PROPOSAL] (F-003) |
| PPM_SCALE | 38 | REAL | — | const |
| DriftCorrection (type) | 45-48 | REAL | — | i64 ppm, private field |
| DriftCorrection::ZERO | 52 | REAL | — | the only value the product ever constructs |
| DriftCorrection::new | 59-63 | REAL | — | clamping constructor; governor + tests only for non-zero |
| DriftCorrection::ppm | 66-68 | REAL | — | accessor |
| DriftCorrection::for_divergence | 78-102 | UNREACHABLE | — | correct checked math; sole production caller is the unarmed governor (governor.rs:203) |
| DriftCorrection::apply | 113-132 | UNREACHABLE | S3 | exact rational composition; zero production callers (F-001) |
| Display | 135-139 | REAL | — | "+100 ppm" format, tested |
| extreme | 142-144 | UNREACHABLE | — | direction-consistent saturation; only reachable via for_divergence's overflow arms |
| div_round | 148-161 | REAL (4th copy) | S4 | F-002 |
| gcd128 | 163-169 | UNREACHABLE | — | debug_assert precondition proven (see §2); only used by apply |
| rate_overflow | 173-179 | UNREACHABLE | — | typed error builder |

## 5. Test honesty (7 tests, lines 181-312)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| new_applies_the_limiter (190) | YES | in-bounds pass-through + four saturation cases incl. i64 extremes |
| divergence_signs_speed_the_right_way (202) | YES | both signs + zero divergence + zero window |
| divergence_saturates_at_the_bound (216) | YES | -100000 ppm desire clamps; i128::MIN+1 and i128::MAX divergences |
| apply_composes_exactly (228) | YES | four exact rational results incl. 5/4+1000ppm=5005/4000 |
| apply_stays_within_the_bound (254) | YES | 24-cell matrix, bound checked in exact integer arithmetic |
| apply_overflows_typed_on_unrepresentable_rates (286) | YES | u32::MAX numerator; both zero-component rates (pub-field bypass) |
| display_is_signed_and_bounded (307) | YES | exact strings |

Excellent test quality; all UNVERIFIED by run here (historical green in the gate logs).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the consumers (audio-engine, video-engine) apply the correction" | drift.rs:16-18 | FALSE as mechanism | no consumer calls `apply`; resampler reads ppm directly (F-001) |
| "card 3.B1 — audit M12 closed" consumers wired | drift.rs:20-27 | HALF-FALSE | the types are consumed, but the derivation path (governor) is never armed in the shipped build (governor.rs F-001) — the correction is permanently ZERO in product |
| "always within ±MAX_CORRECTION_PPM by construction" | drift.rs:40-41 | REAL | clamp + the 24-cell invariant test |
| "rational arithmetic, no float drift" | drift.rs:14-16 | REAL | apply verified by hand (§2) |
| "final number from Phase 3 measurements" | drift.rs:8-10 | UNRESOLVED | no measurement recorded in any gate log (F-003) |

## 7. Dead / duplicated / unreachable code

- `apply`, `gcd128`, `rate_overflow`, `extreme`, `for_divergence`: unreachable in the shipped product (F-001 + governor un-armed).
- `div_round`: fourth workspace copy (F-002).

## 8. Dependencies & flags

- Depends only on `media_types::time::{Rate, TimeBase, TimeError}`. No features, no unsafe, no allocation. `MAX_CORRECTION_PPM` is consumed by audio-engine's governor and resample tests.

## 9. Open questions for the next agent

- resample.rs owner (W03-A0x): the resampler's ratio math duplicates `apply`'s factor arithmetic inline — same dedup opportunity as mapping/clock.
- Wave 20 (reality trace): the audio-drift story is "genuine machinery, zero runtime effect" — REAL% scoring should count card 3.B1's audio half as UNREACHABLE, consistent with the governor and sync.rs findings.
