# AUDIT: crates/media-clock/src/drift.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-23 |
| Lines / functions | 312 / 13 (7 production items + 4 private helpers + 7 tests) |
| File verdict | REAL (with one dead-in-production public method) |
| Reachable from shipped entry point? | PARTIAL. `DriftCorrection::new/ZERO/ppm/for_divergence` — YES: audio-engine governor.rs:203 (`for_divergence`) and resample.rs:124 (`set_correction` → `.ppm()`), wired from the CLI's `AudioLimb::observe_drift` (apps/cli/src/pipeline_host.rs:613-623) called per rendered quantum (host.rs:735). `DriftCorrection::apply(rate)` — NO: zero production callers (rg repo-wide: only this file's own tests). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `./target/debug/deps/media_clock-7012bdf535a3032b drift` (prebuilt) -> 7 passed, 0 failed (exit 0); rg consumer enumerations listed per finding |

## 1. One-paragraph verdict
drift.rs is a pure, correct, overflow-proof bounded-ppm limiter with exact rational rate composition — nothing in it measures anything, so nothing in it can be self-referential. The hostile question is answered by the consumers: the divergence fed to `for_divergence` is a real plant signal (audio ring fill vs the device sample counter — two different clocks), the module explicitly refuses the self-referential design (the media clock itself is never drift-corrected, and no code anywhere does so), and the shipped consumers use only the ppm surface (resampler step ratio, governor step limiter). The catch: `DriftCorrection::apply` — the module's headline "composes onto a playback Rate" capability — has zero production callers, and the governor loop that derives corrections is not armed in any shipped build (armed only by a test harness), so the shipped product runs with drift correction at ZERO everywhere: the shipped A/V sync is the video-scheduler ladder alone, open-loop on the audio side. The module is honest about this in its own docs' absorber assignment; the dead `apply` is the one misleading surface.

## 2. What actually works (verified)
— verified by `./target/debug/deps/media_clock-7012bdf535a3032b drift` -> `test result: ok. 7 passed; 0 failed` (exit 0): limiter saturation (incl. i64::MAX/MIN), divergence signs (±100 ppm for ∓1 ms/10 s), zero-window refusal, i128-extreme saturation, exact composition (10001/10000, 999/1000, 5005/4000), the ±1000-ppm bound invariant over an 8×3 divergence/window grid, typed overflow for degenerate/unrepresentable rates.
— verified consumers exist as documented: resample.rs:124-135 `set_correction` computes `step_den = PPM_SCALE + ppm` (the `out = in × (1e6+ppm)/1e6` ratio, with phase-carry renormalization); governor.rs:203 `DriftCorrection::for_divergence(i128::from(filtered), INTEGRAL_TIME_NS)` with a ≤250-ppm step limiter at governor.rs:204-211.

## 3. Findings
F-001 [S3]
Where: crates/media-clock/src/drift.rs:104-112 and 12-18
Code:
```
    /// Compose this correction onto a playback rate:
    /// `rate x (1 + ppm / 1e6)`, exact rational arithmetic (reduced;
    /// no float drift), checked.
```
What is wrong: `apply` has zero production callers. rg over the whole repo finds `.apply(` on a DriftCorrection only inside this file's `#[cfg(test)]` block (7 hits, all lines 231-301). The shipped consumers (`MicroResampler::set_correction`, `DriftGovernor`) consume `.ppm()` and `for_divergence` — never the Rate composition. Worse, the module's own doc (lines 25-27) pins the design rule that makes the method's only plausible production use forbidden: "The clock itself is never drift-corrected (doc 03 §4.7 e … never `set_rate`)." So the method is simultaneously (a) advertised in the module summary ("composes onto a playback [`Rate`] exactly") and (b) unusable in any shipped path without violating the cited design rule.
Why it matters: Dead public API whose doc reads as the module's purpose; a future contributor calling it inside media-clock would commit the exact §4.7-e violation the module exists to prevent. Also inflates the audited surface ("the consumers … apply the correction" — true of the ppm, false of this method).
What correct looks like: Delete `apply` (and its `rate_overflow`/`gcd128` helpers) until a real consumer exists, or move it behind the audio-engine-only actuator docs with a "no clock-rate use" warning.
Confidence: HIGH — Verification: rg (all `.apply(` call sites enumerated repo-wide) + read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MAX_CORRECTION_PPM / PPM_SCALE consts | 31-38 | REAL | — | ±1000 ppm marked [PROPOSAL]; invariant-over-value stated. |
| DriftCorrection::ZERO / new / ppm | 50-68 | REAL | — | Clamp-by-construction; ZERO for no-info. |
| for_divergence | 78-102 | REAL | — | `ppm = -(div/window)×1e6`, ties away from zero, every step checked; zero window → ZERO (no div-by-zero, no fabricated max); i128 extremes saturate via `extreme` with correct sign direction (signed<0 → i64::MAX → +bound). |
| apply | 113-132 | UNREACHABLE | S3 | F-001: zero production callers; correct and tested in isolation. |
| Display | 135-139 | REAL | — | `+100 ppm` shape, pinned. |
| extreme | 142-144 | REAL | — | Sign-correct saturation; the signed==0 arm is unreachable (0 never overflows). |
| div_round | 148-161 | REAL | — | Ties away from zero; checked ×2 and ±1; None on degenerate denom (defensive). |
| gcd128 | 163-169 | REAL | — | Inputs provably positive in `apply` (num ≥ 1 × ≥999000, den ≥ 1 × 1e6); debug_assert documents the invariant. |
| rate_overflow | 173-179 | REAL | — | Taxonomy-consistent TimeError::Overflow. |
| 7 tests | 189-312 | REAL | — | See §5; all pass live. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| new_applies_the_limiter | YES | Saturation at the bound incl. i64 extremes. |
| divergence_signs_speed_the_right_way | YES | Sign convention pinned numerically (±1 ms/10 s → ∓100 ppm). |
| divergence_saturates_at_the_bound | YES | i128::MIN+1 / i128::MAX over window 1 — the checked-math ladder's worst cases. |
| apply_composes_exactly | YES | Reduced-fraction equality (10001/10000 etc.) — exact rational arithmetic proven, not approximated. |
| apply_stays_within_the_bound | YES | The invariant over a 24-cell grid, cross-multiplied in i128. |
| apply_overflows_typed_on_unrepresentable_rates | YES | u32::MAX base + degenerate 0-component rates → typed Err, no panic. |
| display_is_signed_and_bounded | YES | Format pinned. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Nothing here resamples audio or schedules video" | lines 16-17 | REAL | Pure functions over injected numbers; no I/O, no clocks. |
| "audio-engine's MicroResampler consumes a stored DriftCorrection as its resample ratio (out = in × (1e6 + ppm)/1e6, resample.rs)" | lines 20-23 | REAL | resample.rs:124-135 verbatim mechanics (step_den = PPM_SCALE + ppm). |
| "its DriftGovernor derives one per window via for_divergence over the measured ring-fill error" | lines 23-25 | REAL | governor.rs:200-211. |
| "The clock itself is never drift-corrected … never set_rate" | lines 25-27 | REAL | rg: no set_rate call takes a drift-derived value; `apply` (the rate composer) is caller-free (F-001) — the claim holds because the capability is dead. |
| "composes onto a playback Rate exactly" (module purpose) | lines 14-15 | MISLEADING | The composition method is production-dead (F-001). |
| (context, cross-file) The governor loop is armed in shipped builds | — | FALSE | `arm_drift_governor` callers: pipeline.rs:145/155 + pipeline_host.rs:639-640 (definitions/passthrough) and ONE test caller (apps/cli/tests/sync_scenarios.rs:93). The production open path leaves the limb unarmed by design (pipeline_host.rs:600-609 regime note); also independently traced by the sync.rs audit (audit/files/crates/media-clock/src/sync.rs.audit.md). |

## 7. Dead / duplicated / unreachable code
- `DriftCorrection::apply` + `gcd128` + `rate_overflow` — production-dead (F-001); tested but unreachable from any shipped entry point.
- Nothing else: every other item has a live consumer chain (governor → resampler → CLI observe_drift per quantum).

## 8. Dependencies & flags
- `media_types::time::{Rate, TimeBase, TimeError}` only — no std time, no floats (the exact-rational claim holds: integer arithmetic throughout). No feature gates; ships in the release lane via media-clock.

## 9. Open questions for the next agent
- The hostile question's operational half: with null audio in every shipped lane, the plant (ring fill vs device counter) degenerates to pump-vs-pump — the governor's regime note (pipeline_host.rs:600-609) says as much and disarms. When 3.B3 pacing lands, arming must be re-audited: does the fill error then measure device drift (real) or the decode park's threshold (still self-referential)? The current code cannot answer that; it has never run armed outside tests.
- `MAX_CORRECTION_PPM = 1000` is [PROPOSAL] pending Phase-3 measurement — with the loop disarmed, no measurement can ever arrive. The proposal is unfalsifiable in the shipped build.
