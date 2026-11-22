# AUDIT: crates/audio-engine/src/volume.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 176 / Volume::{UNITY, MUTE, from_linear, from_db, linear, db, apply} + 5 tests (91-176, 45%) |
| File verdict | **REAL** (on the live path via `UNITY`) — but `from_db` breaks the type's own always-finite invariant on finite inputs, and every non-unity constructor is production-dead |
| Reachable from shipped entry point? | PARTIAL. `Volume::UNITY` and `linear()` are live: Mixer::new seeds `vec![Volume::UNITY; tracks]` (mix.rs:40) and the per-quantum mix reads `self.volumes[track].linear()` (mix.rs:112). `from_linear` / `from_db` / `MUTE` / `apply` / `db()`: zero production callers (rg: only tests in audio-engine, audio-output's sine_sink_hash, effects/eq tests; the `audio.set-volume` command exists in the catalog — commands/src/catalog/command.rs:182 — with no handler anywhere, per W03-A05's effects.rs audit). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg (`from_db|from_linear|Volume::|Volume {` across the workspace, separating audio-engine's gain Volume from device-discovery's disk Volume); hand-verification of the overflow math. No cargo (static). |

## 1. One-paragraph verdict

Seventy lines of gain scalar with the right instincts: linear gains validated finite-and-non-negative with typed errors (NaN/∞/negative refused), dB conversion by the amplitude convention (`10^(db/20)`), unity bit-preserving for finite samples (proven by a to_bits test), MUTE rendering as −∞ dB (documented honesty). Two defects. First, `from_db` accepts any *finite* dB value but `10.0_f32.powf(db/20.0)` overflows to `+inf` for `db > ~767` — so `from_db(800.0)` returns `Ok(Volume { linear: inf })`, breaking the type's headline invariant ("the stored value is always finite … so the DSP can multiply without defensive checks", lines 16-20) and re-introducing exactly the gain-poisoning the doc says is impossible ("a NaN *gain* would poison every output and is rejected at the control boundary" — an `inf` gain turns every zero sample into `0 × inf = NaN`, which stage-D's clamp does not clean). Second, everything except `UNITY`/`linear()` is production-dead: no shipped code ever constructs a non-unity volume or calls `apply`, consistent with the mixer and effects audits (hosts pin one track at unity; the volume command has no handler).

## 2. What actually works (verified)

- `from_linear` (44-52): finite + non-negative validation, typed codes `audio.gain_not_finite` / `audio.gain_negative`; test 151-166 pins all three refusals + the dB NaN refusal.
- `UNITY` bit-preservation for finite values (97-119): to_bits equality across ±0.0, ±1.0, subnormal, MIN, MAX — the property the mixer's passthrough relies on.
- `MUTE` (122-127): finite samples → 0.0; `db()` → −∞, asserted.
- dB convention (130-148): −6 dB → 0.501187, +6 dB → 1.995262, round-trip −23.5 dB within 1e-2 — matches the documented 20·log10 amplitude convention.
- `apply` (169-175): pure multiply; NaN *sample* survives (data), documented.
- Live path: `UNITY` + `linear()` through Mixer (mix.rs:40, 112) — the shipped product's only Volume usage.

## 3. Findings

### F-001 [S3] — `from_db` produces non-finite gain from finite input, violating the type's core invariant (latent: zero production callers today)
Where: crates/audio-engine/src/volume.rs:62-71 (the constructor), 16-20 (the invariant it breaks)
Code (volume.rs:66-70):
```rust
        // 10^(db/20) via std: powf is deterministic per IEEE 754 on
        // every host for the same input (exp10 is not in std).
        Ok(Volume {
            linear: 10.0_f32.powf(db / 20.0),
        })
```
Code (volume.rs:16-20, the contract):
```rust
/// Constructors validate; the stored value is always finite and
/// (linearly) non-negative, so the DSP can multiply without defensive
/// checks (NaN never enters the mix — a NaN sample stays a NaN sample,
/// which is *data*, but a NaN *gain* would poison every output and is
```
What is wrong: only the *input* is checked finite; the *output* is not. For `db = 800.0` (finite, accepted), `10^(40)` exceeds `f32::MAX` (~3.4e38), so `powf` returns `+inf` and the constructor returns `Ok(Volume { linear: inf })`. Downstream, `mix` computes `sample * inf`: nonzero samples → ±inf (stage D's saturating clamp would bound those to ±1), but zero samples → `0.0 × inf = NaN`, and `NaN.clamp(…) = NaN` — the poison the doc declares impossible. The mirror case `from_db(-800.0)` underflows to `0.0` (mute-by-underflow) — benign.
Why it matters: the type's entire reason to exist is the always-finite guarantee ("the DSP can multiply without defensive checks"); this constructor breaks it on inputs it accepts. Today it is dormant — rg shows `from_db` has no production caller (tests only), and no shipped volume control exists (`audio.set-volume` has a catalog entry and no handler). The moment any host wires a dB control, this becomes a live S2.
What correct looks like: validate the converted value (`linear.is_finite()` else typed `audio.gain_db_out_of_range`), or cap the dB domain.
Confidence: HIGH — Verification: read (IEEE powf overflow semantics; hand-computed threshold db ≈ 767).
Severity: S3 (latent, zero reachability today), explicitly upgradable to S2 on first wiring — flagged for wave 20 adjudication.

### F-002 [S4] — the UNITY doc overclaims bit-preservation for NaN payloads
Where: crates/audio-engine/src/volume.rs:27-29
Code:
```rust
    /// Unity gain — the exact identity on samples (multiplying by 1.0
    /// is bit-preserving in IEEE 754).
```
What is wrong: `x * 1.0` is bit-preserving for finite values and zeroes, but for NaN inputs the multiply is permitted to quiet the payload / alter the sign bit per IEEE 754 implementation license. The test (100-109) correctly omits NaN from the bit-preservation list — the doc says more than the code proves.
Why it matters: an overbroad doc claim on the one value the production path uses.
Confidence: HIGH — Verification: read (test's value list vs doc wording).

## 4. Per-function table

| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Volume::UNITY | 29 | REAL | — | live via Mixer::new; bit-preserving for finite values |
| Volume::MUTE | 33 | REAL (unreachable) | — | test-only |
| Volume::from_linear | 44-52 | REAL (unreachable) | — | correct validation |
| Volume::from_db | 62-71 | REAL (unreachable) | S3 | F-001: inf on finite db > ~767 |
| Volume::linear | 74-76 | REAL | — | live (mix gain read) |
| Volume::db | 81-83 | REAL (unreachable) | — | log10; MUTE → −inf documented |
| Volume::apply | 86-88 | REAL (unreachable) | — | no production caller (mix multiplies by linear() directly) |

## 5. Test honesty (5 tests, lines 91-176)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| unity_is_bit_preserving (97) | YES | to_bits equality over 8 finite values incl. ±0.0, MIN, MAX |
| mute_zeroes_everything_finite (122) | YES | finite → 0.0; db → −inf asserted |
| db_construction_matches_the_amplitude_convention (130) | YES | exact ±6 dB values + round-trip |
| linear_construction_rejects_bad_values (151) | YES | NaN/∞/negative with exact context codes; NaN dB |
| apply_is_a_pure_multiply (169) | YES | ±0.375 exact; NaN sample survives |

Honest tests — notably they do NOT test from_db at the overflow threshold (F-001's blind spot). UNVERIFIED by run here.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the stored value is always finite and (linearly) non-negative" | volume.rs:16-17 | FALSE | from_db(800.0) → linear = +inf (F-001) |
| "a NaN *gain* would poison every output and is rejected at the control boundary" | volume.rs:18-20 | FALSE | an inf gain (same poisoning class for zero samples → NaN) is *accepted* (F-001) |
| "multiplying by 1.0 is bit-preserving in IEEE 754" | volume.rs:27-29 | REAL for finite / overbroad for NaN | F-002 |
| "−6 dB ≈ 0.50119, +6 dB ≈ 1.99526" | volume.rs:54-55 | REAL | test 130-148 |
| "powf is deterministic per IEEE 754 on every host" | volume.rs:66-67 | REAL-UNPROVEN | determinism across hosts not tested here (the sink-hash suite is the cross-host check) |

## 7. Dead / duplicated / unreachable code

- `from_linear`, `from_db`, `MUTE`, `apply`, `db`: production-unreachable (no shipped volume control). `UNITY` + `linear` are live.
- Name collision to note (not a defect): `audio_engine::Volume` (gain) vs `device-discovery`'s `Volume` (a disk volume) — same name, unrelated crates; both re-exported. Confusing in imports; harmless.

## 8. Dependencies & flags

- Depends on `media_types::EngineError` and `crate::error`. No features, no unsafe, no allocation. PartialEq derive on f32 wrapper (partial — NaN volumes can't exist per invariant, except F-001's inf case, where `==` still works for inf).

## 9. Open questions for the next agent

- Whoever wires `audio.set-volume` (catalog command.rs:182, currently handler-less): F-001 is the first bug you will hit if the UI sends dB values; validate the converted linear gain.
- Wave 16: the from_db overflow threshold (db ≈ 767) is a ready-made mutation/property test gap.
