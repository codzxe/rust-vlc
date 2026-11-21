# AUDIT: crates/media-clock/src/sync.rs
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-21 |
| Lines / functions | 697 / 26 prod items + 14 tests |
| File verdict | REAL (DriftEstimator/RateWindow) with an UNREACHABLE dead half (SyncBands/SyncBand) |
| Reachable from shipped entry point? | PARTIAL. DriftEstimator+RateWindow: YES (apps/cli/Cargo.toml:48 audio-engine → crates/audio-engine/src/governor.rs:57 `use media_clock::sync::{DriftEstimator, RateWindow};` → governor.rs:75/130 `DriftGovernor` → pipeline.rs:34/91/156 → audio-engine lib.rs:124 export). SyncBands/SyncBand: NO — zero production importers (rg over all crates/apps/tests: only sync.rs itself + lib.rs:127-128 re-export). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 3 |
| Commands run | Grep `DriftEstimator\|SyncBands\|SyncBand\b\|RateWindow\|observe_windowed\|observe_sample\|slope_mppm` repo-wide (only audio-engine/src/governor.rs:57 consumes; SyncBands has none) · Grep `SyncPolicy` (video-engine/src/policy.rs:355 own ladder, no SyncBands import) · Grep `S5` in crates/video-engine/tests/sync_scenarios.rs:166 (S5 pins the scheduler, not SyncBands) · sed -n quotes (all verified) · baseline orchrun-test-workspace-nofailfast.log.txt:3204-3301 (`media_clock` lib: `running 92 tests` → `test result: ok. 92 passed; 0 failed`; all 14 `sync::tests::*` listed ok, incl. `exit_requires_half_amplitude_sustained_for_dwell`, `ema_is_bounded_by_the_input_range_for_any_sequence`) · rust-toolchain.toml pin 1.99.1 (per W01-A03 audit) |

## 1. One-paragraph verdict
The arithmetic core is genuine and hostile-input-proof: a dyadic integer EMA whose
convexity/boundedness is property-tested over 4096 random samples including i64
extremes, an exact i128 slope in milli-ppm, a correct hand-rolled ceil-division,
and a Schmitt-trigger band machine whose enter/exit hysteresis tests are honest
and pass in the workspace baseline (92/92). But the module ships two publics it
never wired: `SyncBands`/`SyncBand` — presented by the module docs as the video
drop/repeat ladder's classifier — has **zero production consumers**; video-engine
implements its own ladder (`policy.rs::SyncPolicy`) and the "S5 no-flapping
assertion is pinned against this type" claim is false. The one live defect is a
shift-overflow hole in `for_shift_and_window` (the code clamps the weight at
shift ≤ 126 but then shifts by the unclamped `self.shift`).

## 2. What actually works (verified)
— Dyadic EMA formula: hand-checked k=2 ladder in test `ema_step_weight_matches_the_dyadic_formula` (sync.rs:531-541): 0,800→200; +800→350; +0→262 — matches `ema'=(x+3·ema)>>2`. Verified by reading + baseline pass (orchrun log: `sync::tests::ema_step_weight_matches_the_dyadic_formula ... ok`).
— Boundedness property incl. i64 extremes: sync.rs:481-528 asserts `min ≤ ema ≤ max` over 4096 xorshift samples and exact convergence at `i64::MIN`; the documented 7-unit stall below `i64::MAX` (sync.rs:505-511) is arithmetically correct for k=3.
— Slope math: `windowed_slope_measures_an_exact_ramp_in_milli_ppm` (sync.rs:544-556) — −1000 ns/s ⇒ exactly −1000 milli-ppm / −1 ppm; verified against the code by hand.
— Frozen-source and backwards-jump determinism: sync.rs:559-568, 688-696 — both properties hold in code (saturating_sub) and pass in baseline.
— Reachability of the live half: governor.rs:57 imports + DriftGovernor::new builds `RateWindow::new(CORRECTION_WINDOW_NS)` and `DriftEstimator::new()` (governor.rs:133-135), wired through audio-engine pipeline (pipeline.rs:156).
— All 92 media-clock lib tests passed in the true workspace baseline (audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3300 `test result: ok. 92 passed; 0 failed`), none of the 7 workspace failures is in media-clock.

## 3. Findings

### F-001 [S2] — unbounded `shift` panics (debug) / silently masks (release) in `ema_step`
Where: crates/media-clock/src/sync.rs:112-124, 226-229
Code:
```rust
    pub fn for_shift_and_window(shift: u32, window_ns: u64) -> DriftEstimator {
```
```rust
        let weight = (1u128 << self.shift.min(126)).saturating_sub(1) as i128;
        let combined =
            i128::from(error_ns).saturating_add(i128::from(self.ema).saturating_mul(weight));
        self.ema = saturate_i64(combined >> self.shift);
```
What is wrong: `for_shift_and_window` accepts any `u32` shift with no bound and
no documented limit (the doc only blesses zero, sync.rs:108-109). The weight
computation defends against big shifts (`.min(126)`), but the final
`combined >> self.shift` (line 229) uses the raw, unclamped shift. For
`shift >= 128` the shift overflows: debug builds panic ("attempt to shift right
with overflow"); release builds mask the shift amount (`>> 130` behaves as
`>> 2`), silently producing a wrong EMA — against the "saturate, never
misfire" discipline this module claims (sync.rs:43-46 "Bounded for any input").
Why it matters: this is a public, re-exported constructor (lib.rs:127-128) on
the A/V-sync control path; the inconsistency between the clamped weight and the
unclamped shift proves the hazard was known and half-fixed. Any future caller
tuning `k` at runtime (the W1-18 `[PROPOSAL]` numbers invite experimentation)
gets a panic in test builds or corrupted filtering in release.
What correct looks like: clamp once at construction — reject
`shift > 126` (or `debug_assert!`/`assert!` in `for_shift_and_window`) and store
the clamped value, so weight and shift can never disagree.
Confidence: HIGH — Verification: read (Rust semantics: shift overflow panics
under overflow checks, masks otherwise); no in-tree caller passes shift > 3
(rg: only sync.rs tests use 2/3, governor uses `DriftEstimator::new()`), so
latent, not live.

### F-002 [S3] — `SyncBands`/`SyncBand` are dead production code and the docs claim a consumer that does not exist
Where: crates/media-clock/src/sync.rs:17-20, 276-277, 281-283
Code:
```rust
//! - [`SyncBands`]: the Schmitt-trigger hold/correct band classifier
//!   of the video drop/repeat ladder (doc 03 §4.10 e + the W1-18
//!   hysteresis design): enter a correction band at `|ê| > H`
//!   sustained `D` ticks, exit at `|ê| ≤ H/2` sustained `D` ticks.
```
```rust
/// hovering near the threshold cannot flap (W1-18 §3.1; S5's
/// no-flapping assertion is pinned against this type).
```
What is wrong: No production code imports `SyncBands`/`SyncBand`. Repo-wide rg
finds only: sync.rs itself, the lib.rs:127-128 re-export, and docs/worklog
prose. video-engine — the claimed consumer — builds its own ladder
(`crates/video-engine/src/policy.rs:355 pub struct SyncPolicy`,
`scheduler.rs:519/526 note_late_drop/rotate_windows`) and its S5 no-flapping
scenario pins the *scheduler*
(crates/video-engine/tests/sync_scenarios.rs:166 "S5 — ±½ frame PTS jitter: no
amplification, no threshold flapping", importing `SyncPolicy, SyncSample,
TickOutcome` — not SyncBands). The claim "S5's no-flapping assertion is pinned
against this type" is FALSE, and "the actuators … live in video-engine's
`SyncPolicy`" describes a wiring that does not exist: nothing carries a
`SyncBand` from media-clock into video-engine.
Why it matters: the module presents itself as "the W1-18 design's pure L2 core"
of the sync loop, but half of it is a parallel unused implementation of the
ladder video-engine already has — duplication the module doc explicitly denies
("deliberate, not duplication-by-oversight", sync.rs:24-25 — said about
RateWindow, but the SyncBands half is duplication-by-oversight in exactly that
sense). Future maintainers will "fix" sync in the wrong copy.
What correct looks like: either video-engine's scheduler consumes
`SyncBands::classify` (delete its private ladder), or SyncBands is deleted
along with its re-export and the header rewritten to describe only what ships.
Confidence: HIGH — Verification: read + rg (callers enumerated above).

### F-003 [S3] — doc claim about `window_ns = 0` is contradicted by the code
Where: crates/media-clock/src/sync.rs:108-110 vs 145
Code:
```rust
    /// The explicit constructor. A zero shift is legal (EMA = the last
    /// sample — no filtering); a zero slope window disables slope
    /// measurement (the window never elapses).
```
```rust
                if now_ns.saturating_sub(opened) >= self.window_ns {
```
What is wrong: With `window_ns == 0`, `now_ns.saturating_sub(opened) >= 0` is
true for every u64 value — including a frozen source (saturating_sub = 0).
So a zero window does not "disable slope measurement": every
`observe_windowed` call after the first elapses the window instantly, measures
a slope with `dt` clamped to 1 ns (sync.rs:150 `.max(NS)`), i.e. slope ≈
Δe × 1e9 saturating, and re-opens. The frozen-source property documented at
sync.rs:133-135 ("A frozen `now_ns` never elapses the window") also breaks for
window 0. The in-tree tests pass window 0 only to `observe_sample` paths
(sync.rs:468, 533), so the contradiction is never exercised.
Why it matters: the constructor documents an edge-case contract that is the
exact opposite of the code; the next caller relying on "0 disables the slope"
gets a saturating garbage slope instead.
What correct looks like: either treat 0 as "disabled" explicitly
(`if self.window_ns == 0 { return; }` after the EMA step) or fix the doc to
"a zero window measures a slope on every windowed sample".
Confidence: HIGH — Verification: read (u64 `>= 0` is vacuously true); no test
covers window-0 + observe_windowed.

### F-004 [S4] — `unwrap_or(i64::MAX as i128)` would flip the sign of a negative overflow
Where: crates/media-clock/src/sync.rs:152-155
Code:
```rust
                    let slope = de
                        .checked_mul(1_000_000_000)
                        .map(|scaled| scaled / dt)
                        .unwrap_or(i64::MAX as i128);
```
What is wrong: if `de * 1_000_000_000` overflows in either direction,
`unwrap_or` substitutes **positive** i64::MAX — for a large negative ramp that
is a sign flip. Unreachable today because `de` is the difference of two i64s
(|de| ≤ 2^64 ≈ 1.8e19), so `de·1e9` ≤ ~1.8e28 < i128::MAX ≈ 1.7e38 — the
checked_mul can never fail; the branch is dead. But it is dead *by accident of
the input types*, not by construction, and it silently encodes wrong behavior
if ever reached.
Why it matters: defensive code that is both unreachable and wrong-if-reached is
a trap, not a guard.
What correct looks like: `unwrap_or(i64::MIN as i128)` for the negative side —
or a comment proving the bound (|de| ≤ 2^64) and no fallback at all.
Confidence: HIGH — Verification: read (i64 input bounds) — branch proven dead
for all i64 inputs.

### F-005 [S4] — stale claim that `i64::div_ceil` is unstable
Where: crates/media-clock/src/sync.rs:180-183
Code:
```rust
    /// Ceiling division on signed milli-ppm, spelled out by hand: the
    /// toolchain's `i64::div_ceil` is still `int_roundings`-unstable,
    /// and the one-line identity below carries the same semantics
    /// (round toward +∞) with stable primitives.
```
What is wrong: `i64::div_ceil` was stabilized with `int_roundings` in Rust
1.73.0; the repo pins 1.99.1 (rust-toolchain.toml, per W01-A03). The
justification for the hand-rolled identity is false on this toolchain. The
hand-rolled ceil itself is correct (quotient + (remainder > 0) matches
round-toward-+∞ for the truncating remainder).
Why it matters: comments that misstate the toolchain misdirect the next
maintainer into preserving needless hand-rolled arithmetic.
What correct looks like: `mppm.div_ceil(1000)` (or delete the stale
justification line).
Confidence: HIGH — Verification: read (div_ceil stable since 1.73.0; toolchain
pin 1.99.1).

### F-006 [S4] — `classify` Hold arm does not reset `calm_streak` on the calm path
Where: crates/media-clock/src/sync.rs:355-358
Code:
```rust
                } else {
                    self.late_streak = 0;
                    self.early_streak = 0;
                }
```
What is wrong: when in `Hold` with `|e| ≤ H`, only `late_streak`/`early_streak`
are reset; `calm_streak` is left untouched (it is reset only on the > H path,
line 354, on `None`, lines 336-338, and via `clear_streaks`). Currently
harmless — `calm_streak` is provably always 0 while in `Hold` (entry to Hold
goes through `clear_streaks`, line 375-376) and it is only read in the
correction-band arm — but the invariant is implicit, not enforced.
Why it matters: a future edit that ever leaves `calm_streak > 0` entering Hold
would let one calm tick exit a band it never entered. Fragile-by-omission.
What correct looks like: reset all three streaks on every path in `classify`,
or add a debug assertion that `calm_streak == 0` in the Hold arm.
Confidence: HIGH — Verification: read (all calm_streak write/read sites enumerated).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| const DEFAULT_EMA_SHIFT | 48-52 | REAL | — | k=3; consumed by `new()` and audio-engine governor (governor.rs:135) |
| const DEFAULT_SLOPE_WINDOW_NS | 54-56 | REAL | — | 2 s; consumed by `new()` |
| const NS | 58-59 | REAL | — | 1 ns i128 factor; used only as `.max(NS)` floor |
| DriftEstimator (struct) | 74-98 | REAL | — | 8 integer fields, Copy, zero alloc — matches "bounded for any input" claim |
| DriftEstimator::new | 103-106 | REAL | — | delegates to for_shift_and_window(3, 2s) |
| DriftEstimator::for_shift_and_window | 111-124 | REAL | S2 | F-001: unbounded `shift` accepted; F-003: window-0 doc lie |
| DriftEstimator::observe_sample | 126-130 | REAL | — | pure EMA step; video cadence (no caller yet — video-engine does not import it; reachable half is observe_windowed via governor) |
| DriftEstimator::observe_windowed | 132-163 | REAL | — | window elision verified by hand; dt floor 1 ns; saturating throughout; governor.rs calls it per quantum |
| DriftEstimator::error_ema | 165-169 | REAL | — | Option-gated by `primed` |
| DriftEstimator::slope_mppm | 171-176 | REAL | — | Option-gated by `slope_primed` |
| DriftEstimator::slope_ppm | 178-195 | REAL | S4 | F-005 stale unstable-claim; math correct |
| DriftEstimator::samples | 197-202 | REAL | — | monotonic, not reset by reset() — matches doc |
| DriftEstimator::reset | 204-214 | REAL | — | resets EMA+slope, keeps `samples` (documented) |
| DriftEstimator::ema_step | 216-230 | REAL | S2 | F-001 shift-overflow site; convexity claim verified by property test |
| Default for DriftEstimator | 233-237 | REAL | — | = new() |
| saturate_i64 | 239-243 | REAL | — | clamp-then-cast; correct |
| enum SyncBand | 249-270 | UNREACHABLE | S3 | F-002: no production consumer |
| SyncBands (struct) | 284-300 | UNREACHABLE | S3 | F-002 |
| SyncBands::new | 302-315 | UNREACHABLE | — | half-band saturates u64→i64; dwell max(1) |
| SyncBands::band | 317-321 | UNREACHABLE | — | getter |
| SyncBands::half_band_ns | 323-327 | UNREACHABLE | — | getter |
| SyncBands::classify | 329-381 | UNREACHABLE | S4 | F-006 calm_streak omission; state machine otherwise verified by tests |
| SyncBands::reset | 383-388 | UNREACHABLE | — | to Hold + clear |
| SyncBands::clear_streaks | 390-394 | UNREACHABLE | — | zeroes all three |
| RateWindow::new | 415-423 | REAL | — | used by governor.rs:133 |
| RateWindow::permits | 425-445 | REAL | — | frozen-source/backwards-jump semantics verified by tests 669-696 |
| RateWindow::reset | 447-450 | REAL | — | reopen |
| RateWindow::window_ns | 452-456 | REAL | — | getter; no consumer outside tests |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ema_is_primed_by_the_first_sample_and_converges (467) | YES | None→Some(1e6) then exact fixed point; mutation of priming or formula breaks it |
| ema_is_bounded_by_the_input_range_for_any_sequence (481) | YES | 4096 xorshift samples assert min≤ema≤max; any wrap/overflow escapes the range; i64-extreme stall documented and asserted (≥ MAX−7) |
| ema_step_weight_matches_the_dyadic_formula (531) | YES | hand-checked ladder (200/350/262) — verified independently by me |
| windowed_slope_measures_an_exact_ramp_in_milli_ppm (544) | YES | exact −1000 mppm / −1 ppm / samples=11; any unit or window bug shifts it |
| frozen_source_never_elapses_the_slope_window (559) | YES | 100 frozen calls must leave slope None |
| reset_restarts_ema_and_slope_regime (571) | YES | None after reset, then Some(-7) |
| hold_band_absorbs_errors_within_the_half_band (589) | YES | 100 ticks at exactly ±H stay Hold (strict `>` entry verified) |
| entry_requires_dwell_sustained_beyond_the_half_band (598) | YES | 7-vs-8 dwell boundary + interruption resets streak — pins the hysteresis |
| exit_requires_half_amplitude_sustained_for_dwell (620) | YES | gap (H/2,H] persists; 8th calm tick exits — pins exit threshold incl. i128 floor of H/2 |
| none_samples_hold_the_band_and_decay_streaks (643) | YES | None decays entry streaks |
| reset_returns_to_hold (657) | YES | trivial but real |
| rate_window_first_call_opens_and_permits_once (669) | YES | boundary at exactly window_ns |
| rate_window_frozen_source_permits_exactly_one (679) | YES | 1000 frozen calls |
| rate_window_backwards_jump_holds_and_reset_reopens (688) | YES | saturating_sub hold + reopen |
All 14 ran and passed in the true workspace baseline (orchrun log, media_clock
lib 92/92). NOTE: none of them covers the two false-doc edge cases (F-001
shift>127, F-003 window=0 with observe_windowed) — the tests are honest about
what they test but leave the documented edge contracts untested.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "SyncBands: … the video drop/repeat ladder" classifier | sync.rs:17-20 (module doc, cites doc 03 §4.10 e) | FALSE (as description of shipped wiring) | rg: zero importers; video-engine/src/policy.rs:355 has its own SyncPolicy ladder |
| "S5's no-flapping assertion is pinned against this type" | sync.rs:276-277 | FALSE | S5 lives in crates/video-engine/tests/sync_scenarios.rs:166 and imports video_engine's SyncPolicy, not SyncBands |
| "a zero slope window disables slope measurement (the window never elapses)" | sync.rs:109-110 | FALSE | sync.rs:145 `>= self.window_ns` is vacuously true for 0 |
| "A frozen `now_ns` never elapses the window" | sync.rs:133-135 | REAL | holds for window > 0 (sync.rs:145, test 559); breaks only for the window-0 case the constructor blesses (F-003) |
| "Bounded for any input … an i64 extreme sample can never overflow the EMA" | sync.rs:43-46 | REAL | property test 481-528 passes in baseline; except the shift-amount hole (F-001), which is a shift overflow, not a value overflow |
| "the toolchain's `i64::div_ceil` is still `int_roundings`-unstable" | sync.rs:181 | FALSE | stable since 1.73.0; repo pins 1.99.1 |
| worklog: "media-clock gains DriftEstimator + SyncBands" | docs/worklog/archive-2026-10.md:3519 | REAL (historical) / SyncBands half UNREACHABLE today | see F-002 |

## 7. Dead / duplicated / unreachable code
- `SyncBands`, `SyncBand`, `classify`, `reset`, `clear_streaks`, `band`,
  `half_band_ns` (sync.rs:249-394): dead in production — exported
  (lib.rs:127-128), unit-tested, never imported. Duplicate of video-engine's
  own ladder logic (policy.rs SyncPolicy + scheduler.rs windows).
- `DriftEstimator::observe_sample` (sync.rs:128-130): the documented "video
  cadence" entry point — no video-side caller exists (video-engine never
  imports DriftEstimator); only reachable half is `observe_windowed` via
  audio-engine. Half-dead: implemented + tested, consumer absent.
- `unwrap_or` fallback (sync.rs:155): dead for all i64 inputs (F-004).

## 8. Dependencies & flags
No external deps (media-clock is L2, media-types only — verified in Wave 1
Cargo.toml audit). No feature gates in this file; all code compiles
unconditionally in the shipped CLI build. testkit used in `#[cfg(test)]`
(`Must` trait) — dev-scope only.

## 9. Open questions for the next agent
- audio-engine src auditor (governor.rs): does the governor ever feed
  `observe_sample` (video cadence), or only `observe_windowed`? If only the
  latter, the estimator's `observe_sample` is video-side dead surface.
- video-engine scheduler auditor: confirm the scheduler's ladder (SyncPolicy)
  genuinely implements enter>|H|/exit≤H/2 dwell semantics — if it diverges from
  the spec SyncBands encodes, the *spec* is being violated by the only live
  implementation, and the tested SyncBands copy is the wrong one to trust.
- Whoever owns W1-18: decide SyncBands' fate (wire it or delete it) — this is
  a half-implemented seam in this domain.
