# AUDIT: apps/web/src/audio/clock.rs
| Field | Value |
|---|---|
| Auditor | W12-A03 |
| Date | 2025-06-27 |
| Lines / functions | 521 / 20 (14 fns/methods + 6 tests) |
| File verdict | UNREACHABLE (real, correct arithmetic; no shipped entry point uses it) |
| Reachable from shipped entry point? | NO — `WorkletClock`/`WallclockAnchor`/`AudioLatency`/`DriftPolicy`/`Resampler` are referenced only by `apps/web/src/audio/mod.rs:47` (re-export) and this file's tests; the shipped web host's clock is `NullBackend` → `MediaClock::audio_master(sink.clock_source())` (`apps/web/src/host.rs:1730-1741`), i.e. the null device's counter, NOT AudioContext |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | rg (usage tracing, exit 0); sed -n (quote verification, exit 0); baseline test citation |

## 1. One-paragraph verdict
Correct, well-tested time arithmetic for an AudioWorklet lane that does not exist yet. The hostile-focus question — who drives time in the browser — has a two-part answer. As DESIGNED here: the worklet's played-sample counter is the master (`instants_to_ns` exact integer mapping), and `AudioContext.currentTime` is explicitly the fallback only (`wallclock_ns`), with drift re-anchoring that never moves the media position. As SHIPPED: neither — `host.rs:38-44` states the session clock is ALWAYS the null device's sample counter advanced by pump renders, and the `audioworklet-output` capability row (capabilities.rs:291-299) honestly says "v1 runs the null device as the headless time source". So this file is 300 lines of dead-but-real simulation-proven arithmetic. The code quality is high (exact u128 math, saturating casts, carried-remainder resampler); the defects are a silent zero-rate clamp that contradicts the file's own typed-refusal discipline, and the dead-lane status itself.

## 2. What actually works (verified)
— All 6 tests pass in the orchestrator baseline: `audit/runs/baseline/cargo-test-workspace.txt` lines 3340-3345 (`audio::clock::tests::*` ... `ok` ×6), including a 300-second drift simulation asserting exactly 2 refreshes at ~120 s spacing with no media-position jump.
— Verified `rg` shows zero non-doc uses of any type in this file outside it (sink.rs/autoplay.rs/host.rs: only doc-comment mentions).

## 3. Findings
F-001 [S3]
Where: apps/web/src/audio/clock.rs:1-11 (module docs), vs apps/web/src/host.rs:38-44, apps/web/src/capabilities.rs:291-299
Code:
```
//! - **Audio-master**: the media clock's anchor comes from the
//!   worklet's played-sample counter — "audio-master on web, exactly
//!   the native truth source (doc 03 §4.7)". The mapping is exact
//!   integer arithmetic ([`instants_to_ns`]; doc 01 §3.1's
```
What is wrong: Not a false claim — the module is honestly labeled simulation and the capability row honestly names the null device as v1's time source — but the entire module (WorkletClock, anchors, latency, drift, resampler) is unwired: no shipped path constructs it, and the "worklet" that would feed `played` does not exist in the repo (no AudioWorklet processor is shipped; `apps/web/worker.js` has none). Why it matters: 300 lines of dead code in the wasm artifact; a reader can mistake the model for the running system. What correct looks like: keep the module but a one-line "unwired until card 7.I1" banner, as staged_source's neighbor lanes have in capabilities. Confidence: HIGH — Verification: read (rg usage tracing).
F-002 [S3]
Where: apps/web/src/audio/clock.rs:188-196 (WorkletClock::new)
Code:
```
    ) -> WorkletClock {
        WorkletClock {
            rate_hz: rate_hz.max(1),
```
What is wrong: A zero `rate_hz` is silently clamped to 1 Hz. Every other rate consumer in this same file refuses zero typed or maps it to a harmless 0 (`instants_to_ns` returns 0 at rate 0; `Resampler::for_rates` returns `None`). A zero-rate clock clamped to 1 turns 48 000 played instants into 48 000 seconds of media time — a 10^9× time error with no error surfaced. Defense: a real `AudioContext.getSampleRate()` is never 0 — but this file's own discipline (doc-commented typed refusals everywhere else) says clamp-by-default is the bug shape. What correct looks like: refuse `rate_hz == 0` (typed error or `Option`), matching `for_rates`. Confidence: HIGH — Verification: read (no test covers `WorkletClock::new(0, ..)`; mutation would pass silently).
F-003 [S4]
Where: apps/web/src/audio/clock.rs:229-231 (observe, expected_s)
Code:
```
        let expected_s =
            self.anchor.context_s + (elapsed_instants as f64) / f64::from(self.rate_hz);
```
What is wrong: The divergence is computed in f64 seconds while everything else is integer ns — acceptable for an observation-side comparison (documented: "f64 seconds is the observation side's unit"), but at large `context_s` (hours of playback) the anchor-relative f64 addition loses ns precision, so `divergence_ns` near the 5 ms bound can mis-classify by µs. Blast radius: a refresh may fire one observation early/late — the refresh itself is the invariant, and the media position never jumps. Hygiene-grade. Confidence: MEDIUM — Verification: read (no execution; magnitude estimate for ~1e4 s anchors: f64 mantissa 2^53 ns ≈ 104 days, so error stays sub-µs — noted as theoretical).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| NS_PER_SECOND | 34-35 | REAL | — | restated constant, documented |
| instants_to_ns | 39-49 | REAL | — | u128 exact, saturating; rate 0 → 0 |
| WallclockAnchor::context_ns | 65-68 | REAL | — | one-time f64→ns boundary conversion, `.round()`, saturating cast |
| AudioLatency::total_s/latency_frames/latency_ns | 82-106 | REAL | — | NaN/negative clamped to 0; frames rounded |
| DriftPolicy::DEFAULT/should_refresh | 126-144 | REAL | — | 5 ms bound, `unsigned_abs` compare; bound honestly `[PROPOSAL]` |
| WorkletClock::new | 183-200 | REAL (defect) | S3 | zero-rate silent clamp F-002 |
| WorkletClock::position_ns | 207-218 | REAL | — | stale report clamps to anchor (monotonic within epoch); saturating |
| WorkletClock::wallclock_ns | 222-226 | REAL | — | fallback lane only |
| WorkletClock::observe | 231-255 | REAL | S4 | drift + re-anchor; media position continuous by construction (anchor_media_ns = position_ns) |
| WorkletClock::refreshes/rate_hz | 258-268 | REAL | — | getters |
| Resampler::for_rates | 283-296 | REAL | — | gcd-reduced; bypass None for equal/zero rates |
| Resampler::ratio/remainder | 300-310 | REAL | — | observability |
| Resampler::output_frames | 315-324 | REAL | — | u64 saturating; exact composition (remainder < den ≤ u32::MAX) |
| greatest_common_divisor | 328-336 | REAL | — | Euclid, const fn, correct |
| 6 tests | 341-520 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the_counter_mapping_is_exact | YES | exact ns values incl. 44.1 kHz whole-second, truncation case, stale-report clamp |
| the_wallclock_fallback_maps_through_the_anchor | YES | exact anchor±delta ns values |
| the_latency_model_adds_and_converts | YES | 720 frames / 15 ms exact; absent-outputLatency floor |
| drift_simulation_refreshes_at_the_bound_never_earlier | YES | asserts refresh COUNT (2), first-refresh window (110-130 s), spacing, and both bound directions — kills off-by-one and threshold bugs |
| zero_drift_never_refreshes | YES | 600 s, zero divergence asserted every second |
| the_resampler_is_exact_rational_arithmetic | YES | 160/147 reduction, chunk-split composition 100+47 = 147 total, remainder arithmetic |
All pass in baseline (lines 3340-3345). No #[ignore]. Mutation-sensitive throughout.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "audio-master on web" from the worklet counter | clock.rs:7-11 | REAL-UNPROVEN (model; unwired) | no worklet exists; host.rs:38-44 null device is the shipped truth |
| currentTime is "only the fallback when no audio track exists" | clock.rs:12-15 | REAL-UNPROVEN (design) | `wallclock_ns` implements it; no caller |
| "Every number below is proven by this crate's native tests (simulation … no browser)" | clock.rs:26-29 | REAL | baseline tests 3340-3345 all ok |
| AudioContext latency facts `[VERIFY per-browser]` | clock.rs:16-18 | PAPER-ONLY (honestly labeled) | no browser in sandbox; no code reads baseLatency/outputLatency |

## 7. Dead / duplicated / unreachable code
The whole module is unreachable from the shipped wasm entry (see header). `NS_PER_SECOND` duplicates `media_clock::NANO_BASE` (documented mirror). No intra-file dead code found (all items used by tests or the API surface).

## 8. Dependencies & flags
No deps beyond core/testkit; no feature gates; compiled unconditionally into lumen-web via `pub mod audio` chain (lib.rs:62 → audio/mod.rs:47 re-export).

## 9. Open questions for the next agent
- When card 7.I1 wires the AudioWorklet, does the counter feedback path (JS processor → Rust) exist anywhere? I found no ABI message for it — check `apps/web/src/abi.rs` + `protocol.rs` for a counter-report message; if absent, that is the next S0-sized gap for the audio lane.
