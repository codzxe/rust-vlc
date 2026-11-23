# AUDIT: crates/audio-engine/src/effects/eq.rs
| Field | Value |
|---|---|
| Auditor | W07-A06 |
| Date | 2026-11-23 |
| Lines / functions | 392 / 15 (incl. 4 test helpers + 9 tests) |
| File verdict | UNREACHABLE (the DSP itself is REAL and independently verified) |
| Reachable from shipped entry point? | NO — no pipeline construction, no command, no product/settings key, UI staged disabled. Full chain below |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | prebuilt `./target/debug/deps/audio_engine-9c8a3ce142468d30 effects::eq` -> 9 passed; python3 independent transfer-function evaluation (RBJ formulas transcribed from the code) -> shelf/peak gains match to <0.005 dB; rg reachability sweep over crates/ apps/ ui/ product.toml schemas/ |

## 1. One-paragraph verdict
This is REAL biquad math, not a stub: textbook RBJ audio-EQ-cookbook low-shelf/peaking/high-shelf coefficient formulas, a correct direct-form-1 recurrence with per-channel state, typed parameter validation (non-finite, q≤0, freq outside (0, rate/2)), and a genuine zero-dB identity. I verified the math three ways: the 9 unit tests executed from the prebuilt binary (9 passed, including analytic |H| shelf/peak-gain assertions with 2–5% tolerance), an independent Python evaluation of the transcribed formulas (low shelf +6 dB@250 Hz → |H(40 Hz)| = 5.9957 dB; high shelf −6 dB@4 kHz → |H(16 kHz)| = −5.9962 dB; peaking +6 dB@1 kHz → |H(w0)| = 6.0000 dB; DC gain exactly 6.0000), and line-by-line comparison against the published cookbook. BUT none of it is reachable: the engine's render pipeline (pipeline.rs / mix.rs / governor.rs) never constructs an EffectChain or any effect, the command catalog has no `audio.set-eq` (only audio.set-volume/set-mute, crates/commands/src/catalog/ids.rs:118-120), product.toml carries no `afeat-equalizer` availability key, schemas/settings.toml has no effects entries, and the UI Effects area renders the EQ toggle DISABLED with the panel in local-state-only mode. The staging is honest (matrix row afeat-equalizer says "partial ... UI sliders render inert-with-reason until that command lands"), so this is not fraud — it is finished DSP parked behind a command that was never built.

## 2. What actually works (verified)
- `./target/debug/deps/audio_engine-9c8a3ce142468d30 effects::eq` -> `test result: ok. 9 passed` — flat-EQ bit-exact identity, shelf/peak analytic gains, preamp, per-channel independence, typed bad params, EffectChain plug-in.
- python3 independent |H| evaluation (command above): all three bands within 0.005 dB of asked gain; 0 dB collapses to exact identity coefficients; low-shelf DC gain exact.
- Formula audit vs RBJ cookbook: low_shelf (L67-74), high_shelf (L90-97), peaking (L113-120) all match the published b0..a2 exactly (A = 10^(G/40), alpha = sin w0/(2Q)).

## 3. Findings
F-001 [S3]
Where: crates/audio-engine/src/effects/eq.rs:1-9 (module) vs the whole repo
Code:
```
//! Three-band biquad equalizer (issue #33 M2 — the `afeat-equalizer`
//! row's engine core: low/mid/high shelf + preamp, pure Rust).
```
What is wrong: the "engine core" is wired to nothing. Reachability chain (all verified by rg): (1) no production constructor — `Eq3Band` is built only in eq.rs tests and re-exported (crates/audio-engine/src/lib.rs:123, effects.rs:48); pipeline.rs/mix.rs/governor.rs contain zero effect references; (2) no command — `rg "set-eq" crates/commands` finds nothing; catalog has audio.set-volume/audio.set-mute only; (3) no product key — `rg "afeat" product.toml` finds nothing (no afeat-equalizer availability gate); (4) no settings — `rg -i "eq|equalizer" schemas/settings.toml crates/settings-schema/src` finds nothing (the UI file itself says "No schemas/settings.toml entries exist for effects yet"); (5) the UI toggle is `disabled` with `checked={false}` and a no-op `onChange` (ui/src/screens/settings/areas/effects.tsx:143-149), and the panel comment says "Never a dead control, never an engine write" (effects.tsx:320-321). Why it matters: the P1 audio feature "Equalizer" ships as inert engine code + inert UI; a user cannot hear this filter in any shipped build. What correct looks like: the D-012 `audio.set-eq` command + an EffectChain hook in the pipeline stage (the UI markers already name it). Confidence: HIGH — Verification: rg call-graph + executed tests (the code works; nothing calls it).
F-002 [S4]
Where: crates/audio-engine/src/effects/eq.rs:11-16
Code:
```
//! At 0 dB every band's normalized coefficients collapse to the exact
//! identity filter — the output is **bit-identical** to the input
//! (pinned by test; it is what makes a flat EQ free, not merely
//! transparent).
```
What is wrong: "bit-identical" is false for an input of −0.0: the recurrence computes `y = 1.0 * x + 0.0`, and `(-0.0) + 0.0 = +0.0` in IEEE-754, so the sign bit flips. The pinning test uses a sine generator that never produces −0.0 (sin(0)=+0; 997 Hz at 48 kHz never lands on an exact multiple of π). Nitpick-level, but the doc claims bit-exactness and the test does not cover the one value that breaks it. What correct looks like: "bit-identical except −0.0 → +0.0" or special-case `*x = y` when coefficients are identity. Confidence: HIGH — Verification: read + IEEE semantics; not executed (no Rust scratch build available; marked UNVERIFIED-executed).
F-003 [S4]
Where: crates/audio-engine/src/effects/eq.rs:174-178
Code:
```
    fn process(&mut self, planes: &mut [Vec<f32>]) {
        if self.state.len() != planes.len() {
            // First quantum (or channel-count change): warm the state.
            self.state = vec![[0.0; 2]; planes.len()];
        }
```
What is wrong: the module doc (L20-21) says "process allocates only on the first quantum" — the code actually reallocates on EVERY channel-count change, and alternating channel counts (2→6→2) allocate per call. Bounded (proportional to channel count), no leak, but the doc overstates. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/audio-engine/src/effects/eq.rs:133-139
Code:
```
fn check_params(rate: u32, freq: f32, gain_db: f32, q: f32) -> Result<(), EngineError> {
    if !freq.is_finite() || !gain_db.is_finite() || !q.is_finite() || q <= 0.0 {
        return Err(error::unsupported_format("audio.effects_eq_params"));
```
What is wrong: `gain_db` magnitude is unbounded — any finite value passes, and at |G| ≳ 770 dB the shelf precompute `a * ((a + 1.0) ...)` overflows f32 (A = 10^(G/40) → A² ~ 10^39 > f32::MAX), yielding inf/NaN coefficients and a filter that emits NaNs. The UI design range is ±20 dB; the API should clamp. Currently unreachable (F-001), so no user blast radius. What correct looks like: clamp/reject |gain_db| beyond e.g. ±40 dB. Confidence: MEDIUM — Verification: read + arithmetic reasoning; overflow threshold computed, not executed.
F-005 [S4]
Where: crates/audio-engine/src/effects/eq.rs:216-221
Code:
```
        Ok(Eq3Band {
            preamp,
            low: BiquadFilter::new(BiquadCoeffs::low_shelf(rate, 250.0, low_db, 0.707)?),
            mid: BiquadFilter::new(BiquadCoeffs::peaking(rate, 1000.0, mid_db, 1.0)?),
            high: BiquadFilter::new(BiquadCoeffs::high_shelf(rate, 4000.0, high_db, 0.707)?),
```
What is wrong: fixed 4 kHz high-shelf corner means `Eq3Band::new` fails typed for any stream with rate ≤ 8000 Hz (`freq >= rate/2` rejection) — telephone-rate audio cannot have an EQ at all. Typed and honest, but a silent capability cliff nobody documented. Confidence: HIGH — Verification: read (check_params L137).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| BiquadCoeffs::normalize | 42-50 | REAL | — | Divides all five by a0; a0 cannot be 0 for valid params (verified numerically) |
| BiquadCoeffs::low_shelf | 58-75 | REAL | — | RBJ cookbook exact; typed param gate |
| BiquadCoeffs::high_shelf | 81-98 | REAL | — | RBJ cookbook exact |
| BiquadCoeffs::peaking | 105-121 | REAL | — | RBJ cookbook exact; \|H(w0)\| = full gain verified numerically |
| BiquadCoeffs::IDENTITY | 124-130 | REAL | — | Const identity; tested |
| check_params | 133-141 | REAL | S4 | Non-finite/q/freq gates good; gain unbounded (F-004); rate=0 → all freq rejected typed |
| precompute | 143-150 | REAL | — | A = 10^(G/40), w0 = 2πf/rate, alpha = sin/(2Q) — cookbook |
| BiquadFilter::new/coeffs | 162-172 | REAL | — | Trivial |
| BiquadFilter::process | 174-189 | REAL | S4 | Correct DF1; reallocates on channel-count change (F-003); NaN sample poisons state permanently (no scrub — folded here) |
| Eq3Band::new | 209-222 | REAL | S4 | Fixed corners 250/1k/4k (F-005) |
| Eq3Band::flat | 225-227 | REAL | — | 0 dB/UNITY |
| Eq3Band::preamp | 230-233 | REAL | — | Trivial |
| AudioEffect::process (Eq3Band) | 236-249 | REAL | — | Preamp scale (skipped at unity) then three biquads in place; 5 mul + 4 add per sample per band as documented |
| AudioEffect::name | 251-253 | REAL | — | "eq3" |
| tests ×9 + 4 helpers | 256-392 | REAL | — | See §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| flat_eq_is_bit_exact_identity | YES | Vec equality on 4800 samples through the real chain |
| identity_coeffs_are_the_trivial_filter | WEAK | Asserts two constants (b0=1, b1=0) — near-tautology |
| low_shelf_boosts_bass_by_the_asked_db | YES | 40 Hz settles at ×1.995 ±5%; my independent |H| computation matches (5.9957 dB) |
| high_shelf_cuts_treble_by_the_asked_db | YES | 16 kHz RMS ratio = 10^(−6/20) ±2%; independent |H| = −5.9962 dB |
| mid_peak_gains_at_center_frequency | YES | 1 kHz settles ×1.995 ±5%; independent |H(w0)| = 6.0000 dB |
| preamp_scales_before_the_bands | YES | −6 dB preamp → ×0.5 ±5% |
| per_channel_state_is_independent | YES | Silence stays exactly 0.0 while the other channel boosts |
| bad_parameters_are_typed_errors | YES | Four distinct rejections asserted |
| eq_plugs_into_the_effect_chain | YES | Round-trip through the real EffectChain type |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "RBJ audio-EQ-cookbook biquads" | eq.rs:4-5 | REAL | Formulas match cookbook term-for-term; independent numeric check (commands above) |
| "the output is **bit-identical** to the input (pinned by test)" | eq.rs:13-16 | SHALLOW | True for the tested input; false for −0.0 (F-002) |
| "process allocates only on the first quantum" | eq.rs:20-21 | PARTLY FALSE | Reallocates on every channel-count change (F-003) |
| "five multiplies and four adds per sample per band" | eq.rs:22 | REAL | DF1 recurrence count verified by reading L183-185 |
| Matrix afeat-equalizer: status partial; "UI sliders render inert-with-reason until that command lands" | docs/feature-matrix.toml:1671-1682 | REAL (honest) | Confirmed: no command, no product key, disabled UI (F-001) |
| Matrix evidence "ui pending sliders awaiting audio.set-eq" | docs/feature-matrix.toml:1681 | REAL | effects.tsx:340 `pendingCommand="audio.set-eq"` on a disabled Switch |
| UI: "the EQ enable awaits the `audio.set-eq` catalog command; engine `Eq3Band` exists, doc 05 §2" | ui/src/screens/settings/areas/effects.tsx:6-8 | REAL (honest staging) | Catalog grep: no set-eq |

## 7. Dead / duplicated / unreachable code
The entire module is production-dead (F-001): compiled into every shipped build (audio-engine is a default dep of lumen-cli), zero production callers. The sibling effects (Gain/Mute/Balance in effects.rs) share the same unwired status via EffectChain — out of my file list, flagged for the effects.rs owner.

## 8. Dependencies & flags
No feature gates — always compiled. No external deps (pure Rust f32 math; media_types for errors, crate::volume for preamp). No unsafe.

## 9. Open questions for the next agent
- effects.rs (EffectChain/Gain/Mute/Balance): same unreachability question — is ANY effect wired into pipeline.rs's render stage? (My rg says no.)
- volume.rs: Volume::from_db/linear semantics feeding the preamp (not my file).
- When D-012 lands, remember F-004 (gain clamp) and F-005 (low-rate streams) before exposing Eq3Band to user input.
