# AUDIT: apps/web/src/audio/clock.rs
| Field | Value |
|---|---|
| Auditor | W12-A06 |
| Date | 2026-11-21 |
| Lines / functions | 521 / 22 (+6 tests) |
| File verdict | UNREACHABLE (real, tested arithmetic; zero production callers) |
| Reachable from shipped entry point? | NO — `rg "WorkletClock\|Resampler\|AudioLatency\|DriftPolicy\|instants_to_ns\|WallclockAnchor"` over apps/web/src → only hits are the module itself and `audio/mod.rs:47`'s `pub use`. The v1 host's clock is `media_clock::MediaClock` over the null device (host.rs:91, host.rs:1736-1743), not this model. Nothing in the wasm module calls it; the JS twin (`apps/web/audio-worklet.js`, an independent transcription) does not call Rust. Tests are the only callers. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "WorkletClock\|clock::\|Resampler\|AudioLatency\|DriftPolicy\|instants_to_ns\|WallclockAnchor" apps/web/src` (callers enumerated); `cat apps/web/src/audio/mod.rs` (exports + honest-boundary docs); arithmetic independently recomputed (gcd(44100,48000)=300 → 160/147; 100·160=16000=108·147+124; drift 2 frames/s at 48 kHz = 41 666.7 ns/s → 5 ms bound at ~121 s); baseline: `orchrun-test-workspace-nofailfast.log.txt` (all 6 clock tests `ok`); `ls apps/web/audio-worklet.js` (JS twin exists, 9012 bytes) |

## 1. One-paragraph verdict
The arithmetic in this file is correct — I recomputed every constant the tests pin (the 160/147 reduction, the 124 remainder, the ~121 s drift-refresh cycle at −41.7 ppm) and the math holds; all six tests passed in the orchestrator's no-fail-fast baseline run. But the module is a spec-as-code island: no production path in the entire workspace calls `WorkletClock`, `Resampler`, `AudioLatency`, `DriftPolicy`, or `instants_to_ns`. The shipped web clock is the null-device sample counter via `media_clock::MediaClock`; the browser worklet lane this file models is unwired future work (card 7.I1 `[VERIFY]`), and the actual JS processor is a separate hand-written file that cannot call this Rust. The module docs say all of this honestly ("the model is the spec the JS transcribes"), which keeps it out of fraud territory — it is 300+ lines of tested dead code compiled into a size-budgeted wasm artifact, plus one latent overflow edge in the wallclock mapping that will matter if the lane ever ships.

## 2. What actually works (verified)
— `audio::clock::tests::{the_counter_mapping_is_exact, the_wallclock_fallback_maps_through_the_anchor, the_latency_model_adds_and_converts, drift_simulation_refreshes_at_the_bound_never_earlier, zero_drift_never_refreshes, the_resampler_is_exact_rational_arithmetic}` → all `ok` in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (lumen_web 111 passed / 0 failed).
— Independent recomputation: `greatest_common_divisor(44_100, 48_000) = 300` → ratio (160,147) ✓; `output_frames(100) = 16000/147 = 108 rem 124` ✓; drift at 2 frames/s under 48 kHz = 41 666.67 ns/s ⇒ first 5 ms crossing at s=121, second at s=242 — the test's `refresh_seconds.len()==2`, windows [110,130] ✓.

## 3. Findings

### F-001 [S3]
Where: apps/web/src/audio/clock.rs:173-257 (whole `WorkletClock`), 267-317 (`Resampler`), 91-119 (`AudioLatency`), 56-62 (`instants_to_ns`)
Code:
```rust
pub struct WorkletClock {
    rate_hz: u32,
    anchor: WallclockAnchor,
```
What is wrong: every public item in this file is production-dead. The only references outside the file are `audio/mod.rs:47`'s re-export; `host.rs` imports its clock from `media_clock` (host.rs:91) and constructs `MediaClock::audio_master` over the null device (host.rs:1736-1743). `AudioLatency` is not even re-exported.
Why it matters: 521 lines (300+ production) of uncalled code in a crate whose wasm artifact is size-gated (`cargo xtask web-budget --check`); a reader of the module tree concludes the web target has a worklet clock — it has a model of one. The shipped truth (null-device counter) lives elsewhere. Dead code also rots: the `wallclock_ns` overflow edge (F-002) would ship unnoticed.
What correct looks like: land this file with the card that wires the worklet lane (7.I1), or gate it `#[cfg(any(test, feature = "worklet-model"))]` so the shipped module does not carry it.
Confidence: HIGH — Verification: read + rg caller enumeration (documented above); mod.rs docs confirm the honest-boundary posture.

### F-002 [S4]
Where: apps/web/src/audio/clock.rs:215-218
Code:
```rust
    pub fn wallclock_ns(&self, context_s: f64) -> i64 {
        let delta_ns = (context_s * NS_PER_SECOND as f64).round() as i64 - self.anchor.context_ns();
        self.anchor_media_ns.saturating_add(delta_ns)
```
What is wrong: the subtraction is unchecked `i64 - i64`. `context_s` is an untrusted page observation: `f64::INFINITY.round() as i64` saturates to `i64::MAX` and an anchor built from a negative-infinity observation yields `i64::MIN` — `i64::MAX - i64::MIN` overflows (panic in debug, wrap in release). NaN maps to 0 (defined), so only the ±inf pair is the hazard.
Why it matters: latent only while the module is dead (F-001); becomes an input-controlled panic the day the worklet lane wires this mapping. The adjacent code path (`position_ns`) uses saturating arithmetic — this one missed it.
What correct looks like: `self.anchor.context_ns().saturating_sub`-style, or compute the delta in f64 and clamp before the cast.
Confidence: MEDIUM — Verification: read (cast/overflow semantics reasoned; not executed — no caller exists to build a test through, and constructing one requires the unwired lane).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `instants_to_ns` | 56-62 | REAL (dead) | — | exact u128 math, rate==0 → 0, u64-saturating; twin of host.rs `frames_to_ns`. |
| `WallclockAnchor::context_ns` | 82-84 | REAL (dead) | — | one-time f64→ns round; saturating casts (NaN→0, ±inf→i64 bounds). |
| `AudioLatency::total_s` | 103-105 | REAL (dead) | — | sum. |
| `AudioLatency::latency_frames` | 110-112 | REAL (dead) | — | `.max(0.0)` before cast — negative latencies honest. |
| `AudioLatency::latency_ns` | 116-118 | REAL (dead) | — | same guard. |
| `DriftPolicy::DEFAULT` | 139-141 | REAL (dead) | — | 5 ms, labeled `[PROPOSAL — number]` honestly. |
| `DriftPolicy::should_refresh` | 146-148 | REAL (dead) | — | `unsigned_abs()` compare, strict `>`. |
| `DriftPolicy::Default` | 151-155 | REAL (dead) | — | delegates to DEFAULT. |
| `WorkletClock::new` | 185-198 | REAL (dead) | — | `rate_hz.max(1)` guards zero. |
| `WorkletClock::position_ns` | 203-210 | REAL (dead) | — | saturating; stale-report clamp documented; theoretical `u64::MAX as i64 → -1` cast edge needs ~584 years of frames — not realistic. |
| `WorkletClock::wallclock_ns` | 215-218 | REAL (dead, defect) | S4 | F-002 unchecked i64 subtraction. |
| `WorkletClock::observe` | 225-244 | REAL (dead) | — | refresh re-anchors both halves + `anchor_media_ns = position_ns` (position continuity proven by the drift test's ≤1 ns assert); `refreshes` saturating. |
| `WorkletClock::refreshes` | 248-250 | REAL (dead) | — | counter getter. |
| `WorkletClock::rate_hz` | 254-256 | REAL (dead) | — | getter. |
| `Resampler::for_rates` | 281-291 | REAL (dead) | — | refuses zero/equal → None bypass; gcd-reduced. |
| `Resampler::ratio` | 295-297 | REAL (dead) | — | getter. |
| `Resampler::remainder` | 302-304 | REAL (dead) | — | test observability. |
| `Resampler::output_frames` | 310-316 | REAL (dead) | — | saturating mul/add; `total % den < den ≤ u32::MAX` keeps the u32 cast safe; exact composition property (test-proven). |
| `greatest_common_divisor` | 320-327 | REAL (dead) | — | const Euclid, correct. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `the_counter_mapping_is_exact` | YES | exact ns values incl. 44_100→1e9 and 1/48000→20 833 — any float/truncating mapping fails. |
| `the_wallclock_fallback_maps_through_the_anchor` | YES | three exact anchor mappings (forward/backward). |
| `the_latency_model_adds_and_conversions` (`…_adds_and_converts`) | YES | 720 frames / 15 000 000 ns exact; partial-latency floor compared. |
| `drift_simulation_refreshes_at_the_bound_never_earlier` | YES | 300-step simulation asserting position-within-1-ns at EVERY step, refresh count == 2, both windows [110,130] — a broken drift policy or a jumping re-anchor fails multiple asserts; I recomputed the schedule (121 s / 242 s) independently. |
| `zero_drift_never_refreshes` | YES | 600 steps, zero divergence asserted exactly. |
| `the_resampler_is_exact_rational_arithmetic` | YES | ratio, quantum, split-chunk composition, remainder values — all exact; I verified each by hand. |
All six → `ok` in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt`.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the played-instants → media-time mapping … exact integer arithmetic" | module docs l.10-13 | REAL | instants_to_ns u128 math + exact tests (orchrun). |
| "Every number below is proven by this crate's native tests (simulation …)" | module docs l.39-44 | REAL | 6/6 tests passed in orchrun nofailfast; no browser/device involved — label honest. |
| "The `AudioContext`-side facts … are `[VERIFY]`" | module docs l.42-44 | honest gap | no baseline browser run exists (rc-smoke web stages skipped, `RC_SMOKE_SKIP_WEB=1`). |
| Implicit: the web audio lane uses this clock | module tree name "the web audio pillar (card 7.H3)" | FALSE-as-implied / honest-as-documented | mod.rs l.35-38 states the boundary explicitly ("the v1 host's own audio output remains the null device"); zero callers (F-001). |

## 7. Dead / duplicated / unreachable code
- The entire module (F-001). `AudioLatency` doubly dead (not even re-exported).
- Duplication: `instants_to_ns` ≡ `host.rs::frames_to_ns` (identical body modulo constant name) — same-file-family twin, both live in the same crate.

## 8. Dependencies & flags
- No dependencies beyond core std + testkit (dev). No feature gates. Compiles on all targets.

## 9. Open questions for the next agent
- Who audits `apps/web/audio-worklet.js` (A03): verify the JS processor's constants actually match this model (WORKLET_QUANTUM 128, silence-on-underrun, counter bump) — the "pinned to the same constants" claim (mod.rs l.21) is checkable there.
- If card 7.I1 lands, re-audit `wallclock_ns` for F-002 before wiring page observations into it.
