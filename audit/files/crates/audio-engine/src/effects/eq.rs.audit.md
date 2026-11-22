# AUDIT: crates/audio-engine/src/effects/eq.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 392 / 12 production fns + 9 tests |
| File verdict | REAL as DSP, UNREACHABLE as a feature — no shipped entry point constructs `Eq3Band` or any `EffectChain`; the `audio.set-eq` command the UI awaits does not exist |
| Reachable from shipped entry point? | NO — library export only (audio-engine/src/lib.rs:123); the audio `AudioPipeline`'s effects insertion point is empty (audio-engine/src/pipeline.rs:2-4 "with insertion point B empty"); `EffectChain`/`Eq3Band` appear nowhere in apps/ except the UI's disabled "staged" rows |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | Read/Grep/sed only; RBJ formulas re-derived by hand (see F-002 proof) |

## 1. One-paragraph verdict

The DSP is real and correct: RBJ cookbook biquads (I re-derived all
three coefficient sets against the published formulas — they match),
a correct direct-form-1 per-channel recursion, typed parameter
validation, and a bit-exact 0 dB identity that holds analytically
(numerator and denominator polynomials are computed by expressions
that produce identical IEEE-754 values at A=1, so `H(z)=1` exactly)
and is pinned by test. The problem is that none of it can execute in a
shipped binary: the equalizer is not wired into the audio pipeline
(insertion point B is empty), the `audio.set-eq` catalog command does
not exist anywhere in `crates/commands` or `apps/`, and the UI's
Settings→Audio→Effects area honestly renders its EQ rows disabled
"staged" awaiting that command. The module doc's own framing — "the
`afeat-equalizer` row's engine core" — makes this the engine half of a
feature whose user-facing half is admitted missing.

## 2. What actually works (verified)

— Formula verification (hand re-derivation vs the RBJ cookbook):
`low_shelf` (:67-74), `high_shelf` (:90-97), `peaking` (:113-120) all
match Robert Bristow-Johnson's published coefficient sets term for
term (A=10^(G/40), α=sin(w0)/(2Q)).
— DF1 recursion (:183-186):
```
                let y = c.b0 * *x + z[0];
                z[0] = c.b1 * *x - c.a1 * y + z[1];
                z[1] = c.b2 * *x - c.a2 * y;
```
is the standard two-state direct-form-1 (z1/z2 partial sums) — correct.
— Bit-exact identity: at gain 0 dB, A=1 exactly (`10f32.powf(0.0)==1.0`),
(A−1)=0, (A+1)=2; b and a polynomials coincide term-wise in IEEE-754
(e.g. b1 = 2·1·(0−2cw) = −4cw, a1 = −2·(0+2cw) = −4cw, both exact
power-of-2 scalings), so `flat_eq_is_bit_exact_identity` (:284-290)
passes for the right reason.
— Parameter validation (:133-141): non-finite freq/gain/q, q ≤ 0,
freq ≤ 0 or ≥ rate/2 are typed `audio.effects_eq_params`; rate=0 is
implicitly rejected (any freq then fails the Nyquist check).

## 3. Findings

### F-001 [S3] The equalizer is unreachable: no command, no pipeline insertion, shipped disabled
Where: eq.rs:1-2 (the framing), audio-engine/src/pipeline.rs:2-4 (empty
insertion point), ui/src/screens/settings/areas/effects.tsx:7-9 (the
admission), repo-wide Grep for `set-eq`/`Eq3Band` (zero production hits)

Code (eq.rs:1-2):
```
//! Three-band biquad equalizer (issue #33 M2 — the `afeat-equalizer`
//! row's engine core: low/mid/high shelf + preamp, pure Rust).
```
Code (ui/src/screens/settings/areas/effects.tsx:7-9):
```
//     `audio.set-eq` catalog command; engine `Eq3Band` exists, doc 05 §2),
```
Code (audio-engine/src/pipeline.rs:2-4):
```
//! straight mix → device format (card 2.C3; doc 03 §4.8 a/c/d — the
//! v0 instantiation of the `AudioPipeline` sketch with insertion
//! point B empty).
```
What is wrong: `Eq3Band` is exported (audio-engine/src/lib.rs:123) and
correct, but (a) `AudioPipeline` never runs an `EffectChain` — the
stage-B insertion point is empty; (b) no `audio.set-eq` command exists
in `crates/commands` (Grep: zero matches); (c) the CLI and web hosts
never construct an `EffectChain` (Grep: only eq.rs's own test
:383-391 does). The UI renders the EQ controls disabled with an honest
staged marker. Verdict per §5: UNREACHABLE.
Why it matters: a user reading the feature matrix row `afeat-equalizer`
pointing at this file would believe the shipped player has an EQ. The
engine core exists; the feature does not. (The row's final verdict
belongs to the Wave 18/19 claim ledger; from the code side the feature
is not reachable.)
What correct looks like: wire an `audio.set-eq` command to an
`EffectChain` at the pipeline's stage-B seam, or mark the matrix row
engine-only/partial.
Confidence: HIGH — Verification: read + repo-wide Grep
(`Eq3Band|EffectChain|AudioEffect|set-eq`).

### F-002 [S4] `BiquadFilter::process` resets all state on a channel-count change, silently
Where: eq.rs:174-178

Code:
```
        if self.state.len() != planes.len() {
            // First quantum (or channel-count change): warm the state.
            self.state = vec![[0.0; 2]; planes.len()];
        }
```
What is wrong: a mid-stream channel-count change zeroes every band's
filter state — an audible click at the seam, with no signal in the API
(no error, no event, no counter). The comment documents it, which is
why this is hygiene and not a defect: today no caller can change
channel counts mid-stream (the pipeline's shape is negotiated once),
so the path is theoretical.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `BiquadCoeffs::normalize` | 42-50 | REAL | — | divides by a0; a0>0 by construction (validated params, α>0, A>0) |
| `BiquadCoeffs::low_shelf` | 58-75 | REAL | — | matches RBJ cookbook exactly (hand-verified) |
| `BiquadCoeffs::high_shelf` | 81-98 | REAL | — | matches cookbook |
| `BiquadCoeffs::peaking` | 105-121 | REAL | — | matches cookbook |
| `BiquadCoeffs::IDENTITY` | 124-130 | REAL | — | 1,0,0,0,0 — the true identity |
| `check_params` | 133-141 | REAL | — | finite/positive/Nyquist checks; rate=0 implicitly rejected |
| `precompute` | 143-150 | REAL | — | A=10^(G/40), w0, cos, sin/(2Q) |
| `BiquadFilter::new`/`coeffs` | 162-172 | REAL | — | |
| `BiquadFilter::process` | 174-189 | REAL | S4 | correct DF1; channel-change resets state (F-002); 5 mul + 4 add per sample per band as documented |
| `Eq3Band::new` | 209-222 | REAL | — | fixed corners 250 Hz / 1 kHz / 4 kHz, Q 0.707/1.0 |
| `Eq3Band::flat` | 225-227 | REAL | — | the bit-exact identity |
| `Eq3Band::preamp` | 230-232 | REAL | — | |
| `AudioEffect::process for Eq3Band` | 236-249 | REAL | S3 | correct composition; **no production caller** (F-001) |
| `AudioEffect::name` | 251-253 | REAL | — | "eq3" |

## 5. Test honesty (:256-392)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `flat_eq_is_bit_exact_identity` | YES | exact `assert_eq!` on 4800 samples — any coefficient drift fails |
| `identity_coeffs_are_the_trivial_filter` | NO | asserts two constants of `IDENTITY` — a tautology pin, harmless |
| `low_shelf_boosts_bass_by_the_asked_db` | YES | 40 Hz settles at ×10^(6/20), 5% tolerance — catches formula/sign errors |
| `high_shelf_cuts_treble_by_the_asked_db` | YES | settled-RMS method (honest for 3-samples/period), 2% tolerance, −6 dB |
| `mid_peak_gains_at_center_frequency` | YES | |H(w0)|=A property, 5% |
| `preamp_scales_before_the_bands` | YES | −6 dB preamp → ×0.5 |
| `per_channel_state_is_independent` | YES | silence stays exactly 0.0 — catches cross-channel state bleed |
| `bad_parameters_are_typed_errors` | YES | freq 0, above Nyquist, NaN gain, q 0 |
| `eq_plugs_into_the_effect_chain` | YES | runs through the real `EffectChain` (the only chain use in the repo) |

Good suite — behavioral amplitude checks, not "does not panic". Not
executed by me (no cargo); UNVERIFIED per-test (W00 workspace run
green).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "RBJ audio-EQ-cookbook biquads" | :4-5 | REAL | hand-verified §2 |
| "At 0 dB … bit-identical to the input" | :13-16 | REAL | analytic proof §2 + test |
| "`process` allocates only on the first quantum" | :20-21 | REAL | state warm-up only on len mismatch |
| "`afeat-equalizer` row's engine core" | :1-2 | UNREACHABLE (engine half only) | F-001 — no command, no insertion point |

## 7. Dead / duplicated / unreachable code

The entire public surface (`BiquadCoeffs`, `BiquadFilter`, `Eq3Band`)
is engine library code with zero production callers (F-001). The
`EffectChain` it plugs into is likewise production-dead (only eq.rs's
own test constructs one).

## 8. Dependencies & flags

`media_types::EngineError`, `crate::{error, volume}`, `super::AudioEffect`.
No feature gates; audio-engine ships in the CLI, but this module's
code is never constructed outside tests.

## 9. Open questions for the next agent

- Wave 18/19 (claim ledger): which matrix row(s) cite `effects/eq.rs`
  as evidence (`afeat-equalizer`, issue #33 M2)? Those rows must be
  verdicted UNREACHABLE, not REAL, unless another wave finds a caller.
- Wave 15 (UI): confirm the EQ faders' local state never claims to
  affect playback (the effects.tsx header says staged/disabled —
  verify the rendered DOM matches).
