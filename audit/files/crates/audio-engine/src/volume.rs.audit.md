# AUDIT: crates/audio-engine/src/volume.rs
| Field | Value |
|---|---|
| Auditor | W07-A06 |
| Date | 2026-11-21 |
| Lines / functions | 176 / 11 (6 prod items + 5 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open/play` → apps/cli/src/pipeline_host.rs:51 `audio_engine::{AudioPipeline, PipelineSpec}` → pipeline mix stage → `Mixer::new` seeds `vec![Volume::UNITY; tracks]` (mix.rs:40) → `linear()` read per track per mix (mix.rs:112) and multiplied per sample (mix.rs:119) — UNITY runs on every shipped audio frame. The non-unity constructors (`from_linear`/`from_db`) have NO production caller today: `set_volume` has no production caller (audit/files/crates/audio-engine/src/pipeline.rs#part1.audit.md), and no app/playback-engine code constructs a `Volume` (rg: only audio-engine internal + tests). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | rg `Volume::from_db\|from_db\(` repo-wide (callers: volume.rs tests, eq.rs test, effects.rs tests only — zero production); rg `Volume::from_linear\|set_volume` crates+apps (mix.rs/pipeline.rs internals + tests; playback-engine: zero Volume hits; apps/cli: zero); sed -n mix.rs:35-124 (UNITY default + per-sample multiply — reachability proof); grep baseline log (5/5 volume tests ok, :124-128); float-overflow arithmetic traced by hand (F-001) |

## 1. One-paragraph verdict
The core is exactly what it claims: a validated scalar gain whose UNITY and MUTE constants are bit-behaved (pinned by `to_bits()` assertions including −0.0, f32::MIN, f32::MAX — all executed in the baseline run), running per-sample on the shipped mixer path. But the file's central invariant — "Constructors validate; the stored value is always finite" — is false: `Volume::from_db(800.0)` computes `10.0_f32.powf(40.0)` = +inf in f32 and returns `Ok`, handing the mixer an infinite gain that turns every finite sample into ±inf and every zero sample into NaN (0·inf). The input validation checks the dB value, never the converted result. Today the hole is unreachable (nothing in any shipped build constructs a non-unity Volume — `set_volume` itself has no production caller), which keeps this at S3, but it is a public-constructor contract break one wiring away from poisoned audio, and the W06 effects.rs audit independently flagged its downstream symptom. Three smaller nits: `from_linear(-0.0)` is accepted (a sign-flipping zero gain), the MUTE doc over-claims NaN-sign preservation through `* 0.0`, and the `powf` determinism claim cites IEEE 754 for a guarantee the standard does not give.

## 2. What actually works (verified)
— UNITY bit-preservation: `unity_is_bit_preserving` pins `apply(v).to_bits() == v.to_bits()` for 8 edge values — baseline ok (test-workspace.log.txt:128).
— MUTE value-zeroing + `db()` = −∞: `mute_zeroes_everything_finite` ok (:127); `20·log10(0.0)` = −∞ documented honestly.
— dB convention: `from_db(−6)` → 0.501187, `+6` → 1.995262, round-trip −23.5 dB within 1e-2 — analytic expectations, ok in baseline (:125).
— Typed refusals: `from_linear(NAN/INF)` → `audio.gain_not_finite`, `from_linear(−1)` → `audio.gain_negative`, `from_db(NAN)` → `audio.gain_db_not_finite` — codes pinned (:126).
— Shipped-path execution: the mixer multiplies every mixed sample by `volumes[track].linear()` (mix.rs:112,119) with UNITY defaults — the 1.0 multiply is the bit-preserving identity the mix AC relies on.

## 3. Findings
### F-001 [S3] `from_db` breaks the documented always-finite invariant — finite input, infinite gain out
Where: crates/audio-engine/src/volume.rs:62-71 vs 16-20
Code:
```rust
    pub fn from_db(db: f32) -> Result<Volume, EngineError> {
        if !db.is_finite() {
            return Err(error::bad_gain("audio.gain_db_not_finite"));
        }
        // 10^(db/20) via std: powf is deterministic per IEEE 754 on
        // every host for the same input (exp10 is not in std).
        Ok(Volume {
            linear: 10.0_f32.powf(db / 20.0),
        })
```
and the invariant it violates:
```rust
/// Constructors validate; the stored value is always finite and
/// (linearly) non-negative, so the DSP can multiply without defensive
```
What is wrong: `db = 800.0` is finite and passes the gate; `10.0_f32.powf(40.0)` overflows f32 to `+inf` (f32::MAX ≈ 3.4e38 < 10^40), so `from_db(800.0)` returns `Ok(Volume { linear: inf })`. `apply` then produces ±inf for every finite sample and **NaN for every zero sample** (`0.0 * inf = NaN` by IEEE 754) — the exact "NaN gain would poison every output" outcome the struct doc (lines 18-20) promises is rejected. `db()` on that Volume returns +inf. The threshold is |db| ≳ 771 (10^(db/20) > f32::MAX).
Why it matters: A public constructor's documented invariant is false; any first wiring of `audio.set-volume`/per-track gain through the dB path (doc 03 §4.8 `MixPoint.gain_db` vocabulary) trips it with an unvalidated settings value. Currently latent: rg shows `from_db` has zero production callers and `Mixer::set_volume` has none (prior pipeline.rs#part1 audit) — hence S3, not S2. Independently corroborated as downstream symptom by audit/files/crates/audio-engine/src/effects.rs.audit.md F-004.
What correct looks like: Validate the result, not just the input: `let linear = 10.0_f32.powf(db / 20.0); if !linear.is_finite() { return Err(error::bad_gain("audio.gain_db_overflow")); }` — or clamp the dB domain (VLC's volume range is 0..512 % / ±? — a ±60 dB domain covers every real slider).
Confidence: HIGH — Verification: read (IEEE overflow arithmetic traced by hand; no execution — the path has no production caller, and the test suite does not cover it).

### F-002 [S4] `from_linear(-0.0)` accepted — a sign-flipping "zero" gain
Where: crates/audio-engine/src/volume.rs:44-52
Code:
```rust
        if linear < 0.0 {
            return Err(error::bad_gain("audio.gain_negative"));
        }
        Ok(Volume { linear })
```
What is wrong: In IEEE 754, `-0.0 < 0.0` is false, so `from_linear(-0.0)` passes both gates and stores −0.0. `apply(x)` = `x * -0.0` flips the sign of every zero result (+0.0→−0.0 and vice versa), and `db()` renders it as −∞ — identical to MUTE's rendering though it is a different constant. Value-wise inaudible; bit-wise it violates the "non-negative" spirit the error message defends ("phase inversion is a filter, not a volume" — yet −0.0 is a sign flip).
Why it matters: Hair-split edge today (no production caller), but the same class as F-001: the validation checks the wrong predicate (`linear.is_sign_negative()` is the correct gate for the documented intent).
What correct looks like: `if linear.is_sign_negative() && linear != 0.0` … or explicitly canonicalize `-0.0 → 0.0`.
Confidence: HIGH — Verification: read (IEEE comparison semantics; no execution needed).

### F-003 [S4] MUTE doc over-claims NaN-sign preservation through `* 0.0`
Where: crates/audio-engine/src/volume.rs:31-32
Code:
```rust
    /// Silence — the exact zero (multiplying by 0.0 preserves the
    /// sign of a NaN payload but zeroes everything finite).
```
What is wrong: IEEE 754 requires `NaN * 0.0` to be *some* NaN; the sign (and payload) of the result is implementation-defined, not guaranteed to be preserved. The claim is platform folklore (true on common x86 hardware, unspecified in Rust). The MUTE test pins only finite values, not NaN signs.
Why it matters: A documented guarantee the language does not give; anyone relying on NaN-payload stability across platforms (the wasm target included) is misled.
What correct looks like: "NaN samples stay NaN (sign unspecified); finite samples become ±0 with the sample's sign."
Confidence: HIGH — Verification: read (IEEE 754 NaN propagation rules).

### F-004 [S4] `powf` determinism claim cites IEEE 754 for a guarantee it does not provide
Where: crates/audio-engine/src/volume.rs:66-67
Code:
```rust
        // 10^(db/20) via std: powf is deterministic per IEEE 754 on
        // every host for the same input (exp10 is not in std).
```
What is wrong: IEEE 754 requires correct rounding for +,−,×,÷, sqrt, fma — **not** for `pow`. Rust's `f32::powf` lowers to the platform's libm; last-ulp differences between libm implementations are permitted. "Deterministic per IEEE 754 on every host" is a false citation.
Why it matters: mix.rs:6-9 stakes byte-stable-across-hosts behavior on this crate's determinism; a reader auditing that claim will trace it here and find a standards citation that does not hold. Zero current impact (the dB path is unreachable, and the shipped unity/mute gains never call powf).
What correct looks like: Either drop the claim, or implement 10^x via `exp2(x * log2(10))`… which has the same problem — the honest fix is "deterministic per host libm; cross-host byte-stability of dB-sourced gains is not claimed."
Confidence: HIGH — Verification: read (IEEE 754-2019 required-operations list; Rust powf → libm lowering).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct Volume` | 21-24 | REAL | — | private f32, invariant documented (broken by F-001) |
| `Volume::UNITY` | 29 | REAL | — | 1.0; bit-preserving pinned by to_bits test; runs on shipped mix path (mix.rs:40) |
| `Volume::MUTE` | 33 | REAL | — | 0.0; −∞ rendering pinned; F-003 doc nit |
| `Volume::from_linear` | 44-52 | REAL | — | finite + non-negative gates, typed codes pinned; −0.0 hole (F-002); no production caller today |
| `Volume::from_db` | 62-71 | REAL | — | input-finite gate only; result overflow → +inf Ok (F-001); underflow → +0.0 (benign); powf claim (F-004); no production caller today |
| `Volume::linear` | 74-76 | REAL | — | trivial getter; hot path (mix.rs:112) |
| `Volume::db` | 81-83 | REAL | — | 20·log10; MUTE → −∞ documented + pinned; inf Volume → +inf (GIGO, F-001) |
| `Volume::apply` | 86-88 | REAL | — | one multiply; NaN sample survives (pinned) |
| test `unity_is_bit_preserving` | 96-119 | REAL | — | to_bits identity over 8 edge values — mutation-proof |
| test `mute_zeroes_everything_finite` | 121-127 | REAL | — | value equality (−0.0 passes ==); −∞ db pinned |
| test `db_construction_matches_the_amplitude_convention` | 129-148 | REAL | — | analytic constants |
| test `linear_construction_rejects_bad_values` | 150-166 | REAL | — | 3 typed codes pinned; NOTE: no from_db(800) overflow case — F-001 untested |
| test `apply_is_a_pure_multiply` | 168-175 | REAL | — | incl. NaN survival |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| unity_is_bit_preserving | YES | bitwise identity assertions |
| mute_zeroes_everything_finite | YES | value + −∞ rendering |
| db_construction_matches_the_amplitude_convention | YES | analytic 10^(±6/20) values, 1e-4/1e-3/1e-2 windows |
| linear_construction_rejects_bad_values | YES | exact error codes |
| apply_is_a_pure_multiply | YES | exact products + NaN |

All 5 executed ok in baseline (test-workspace.log.txt:124-128). **Coverage gap:** no test constructs a `from_db` value whose conversion overflows — F-001 would survive the whole suite.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the stored value is always finite and (linearly) non-negative" | volume.rs:16-17 | FALSE | from_db(800.0) → Ok(inf) (F-001) |
| "a NaN *gain* would poison every output and is rejected at the control boundary" | volume.rs:18-20 | FALSE (as stated) | only non-finite *inputs* are rejected; a finite input yielding inf/NaN-inducing gain is accepted (F-001) |
| "multiplying by 1.0 is bit-preserving in IEEE 754" | volume.rs:27-29 | REAL | to_bits test, baseline ok |
| "MUTE renders as NEG_INFINITY — documented, not clamped" | volume.rs:79-80 | REAL | test pinned |
| "powf is deterministic per IEEE 754 on every host" | volume.rs:66-67 | FALSE (citation) | F-004 |
| mix.rs "byte-stable across runs and hosts" (dependent claim) | mix.rs:6-9 | REAL for the shipped path (unity/mute/from_linear only), UNPROVEN for dB-sourced gains | F-004 + from_db has no production caller |

## 7. Dead / duplicated / unreachable code
- Nothing dead in this file; `from_db`/`from_linear`/`MUTE` are exercised by tests and await the volume-command wiring (doc 03 §4.8); the mixer's defaults keep UNITY on the live path.

## 8. Dependencies & flags
- `media_types::EngineError` + `crate::error` (bad_gain); std-only math, no `unsafe`, no feature gate; part of the unconditional audio-engine crate compiled into shipped builds.

## 9. Open questions for the next agent
- Whoever wires the volume command (audio.set-volume / MixPoint.gain_db): fix F-001 first — validate the converted linear value; also decide the −0.0 policy (F-002).
- crates/audio-engine/src/mix.rs owner (if unaudited): the `*dst += *sample * gain` accumulation is f32; with F-001's inf gain the NaN poison enters here — the mix doc's "data, not damage" posture assumes validated gains.
