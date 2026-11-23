# AUDIT: crates/audio-engine/src/volume.rs
| Field | Value |
|---|---|
| Auditor | W07-A02 |
| Date | 2026-11-23 |
| Lines / functions | 176 / 7 production items (UNITY, MUTE, from_linear, from_db, linear, db, apply) + 5 tests |
| File verdict | REAL (correct tested primitive) — but its mutation paths are UNREACHABLE from every shipped entry point, and `from_db` violates its own finiteness invariant on finite input |
| Reachable from shipped entry point? | NO for any volume change: zero production callers of `set_volume`/`from_db`/`Gain`/`Eq3Band` (rg across crates+apps, non-test); the UI's `audio.set-volume` is refused by every real host (see F-002). The UNITY default is used implicitly by `Mixer`/`AudioPipeline` constructors (mix.rs:40), which ARE shipped (apps/cli/src/pipeline_host.rs:51,546). |
| Findings | S0: 0 S1: 0 S2: 2 S3: 0 S4: 0 |
| Commands run | `./target/debug/deps/audio_engine-78c3118f7cd3bd18 volume` → 7 passed (exit 0); `rustc -O` (pinned 1.99.1) overflow proof program → `powf(771/20) = inf, is_finite: false`; python3 threshold math (db > 770.6367 overflows f32); rg reachability traces (UI → command → hosts) |

## 1. One-paragraph verdict
The arithmetic here is right and well-tested for the sane range: linear and dB constructors, the 10^(db/20) / 20·log10 round trip, bit-preserving unity, and the pure-multiply `apply` are all verified by executed tests. Two serious problems sit at the edges. First, `from_db` checks the INPUT for finiteness but never the OUTPUT: any finite dB above 770.64 makes `10.0f32.powf(db/20.0)` overflow to `+inf`, which the constructor happily stores — breaking the type's documented invariant ("the stored value is always finite", lines 16-17) and shipping an infinite gain that turns every sample into ±inf, the exact poisoning the module says is "rejected at the control boundary". Second — and this answers the hostile-focus question directly — nothing shipped can ever change a volume: no production code calls `set_volume`/`from_db`/`Gain`/`Eq3Band`; the UI volume slider dispatches `audio.set-volume`, which the CLI host, the desktop host (it adopts the CLI host), the web host, and the control-server ALL refuse; only the UI's built-in demo transport implements it. The mixer runs permanently at UNITY.

## 2. What actually works (verified)
- `./target/debug/deps/audio_engine-78c3118f7cd3bd18 volume` → `test result: ok. 7 passed` (exit 0) — the 5 volume.rs tests plus mixer/pipeline volume tests.
- `from_db(-6.0)` → 0.501187, `from_db(6.0)` → 1.995262, round-trip −23.5 dB within 1e-2 (executed by `db_construction_matches_the_amplitude_convention`).
- Rejection paths executed: NaN/±inf linear → `audio.gain_not_finite`; negative → `audio.gain_negative`; NaN dB → `audio.gain_db_not_finite` (`linear_construction_rejects_bad_values`).
- Log-vs-linear convention is correct: amplitude dB (10^(db/20)), not power dB (10^(db/10)) — pinned by the ±6 dB test values.

## 3. Findings
F-001 [S2]
Where: crates/audio-engine/src/volume.rs:62-71 (code) vs 16-20 (invariant doc)
Code:
```
    pub fn from_db(db: f32) -> Result<Volume, EngineError> {
        if !db.is_finite() {
            return Err(error::bad_gain("audio.gain_db_not_finite"));
        }
        // 10^(db/20) via std: powf is deterministic per IEEE 754 on
        // every host for the same input (exp10 is not in std).
        Ok(Volume {
            linear: 10.0_f32.powf(db / 20.0),
        })
    }
```
What is wrong: the finiteness check is on `db`, not on the converted `linear`. A finite `db > 770.6367` (10^(db/20) > f32::MAX ≈ 3.4028e38) makes `powf` return `+inf`, which is stored unchecked. Executed proof with the repo's pinned toolchain (rustc 1.99.1, standalone program): `10.0f32.powf(771.0/20.0)` → `inf`, `is_finite: false`. The struct doc (lines 16-17: "the stored value is always finite and (linearly) non-negative, so the DSP can multiply without defensive checks") and the NaN rationale (lines 18-20: a non-finite gain "would poison every output and is rejected at the control boundary") are both false for `+inf`: `Ok(Volume { linear: inf })` is returned and every multiplied sample becomes ±inf (and inf·0 = NaN).
Why it matters: `Volume` is the sanctioned public control-boundary type (audio-engine lib.rs:128 `pub use volume::Volume`); any consumer that passes a config-file or wire dB value ≥ 771 silently poisons the entire mix instead of getting the promised typed rejection. `apply` then has no defensive check to catch it — by design, because the constructor promised the invariant.
What correct looks like: `let linear = 10.0f32.powf(db / 20.0); if !linear.is_finite() { return Err(error::bad_gain("audio.gain_db_overflow")); }` (and a test at db = 771/1000).
Confidence: HIGH — Verification: ran (rustc proof program; test suite shows no large-dB case exists).

F-002 [S2]
Where: crates/audio-engine/src/volume.rs:35-71 (the entire mutation surface) — cross-file wiring evidence below
Code:
```
    pub fn from_linear(linear: f32) -> Result<Volume, EngineError> {
```
What is wrong: not this function — nothing shipped ever calls it with anything but test values. The full trace:
- UI: `ui/src/screens/player/controls/ControlBar.tsx:477-483` renders `VolumeSlider` with `onCommit={actions.setVolume}`; `ui/src/screens/player/context.tsx:389-391` dispatches `{ cmd: "audio.set-volume" }`.
- Hosts: `apps/cli/src/dispatch.rs` (the `route` match) has NO `AudioSetVolume` arm — every unserved command falls to `other => Err(unserved(other))` ("`audio.set-volume` is not served by this build's command-line host"); `apps/desktop/src/dispatch.rs` drives `lumen_cli::host::CliHost` (same route); `apps/web/src/host.rs:1404-1430` has no audio arm; `apps/control-server/src/dispatch.rs` answers everything except the two introspection commands with not-ready.
- Engine side: the only callers of `set_volume`/`from_db`/`Gain::new`/`Eq3Band::new` in the entire tree are `#[cfg(test)]` modules (pipeline.rs:450 `mod tests`, mix.rs:128, effects.rs:271, eq.rs tests) and `crates/audio-output/tests/sine_sink_hash.rs`.
- The ONLY implementation of `audio.set-volume` anywhere is `ui/src/client/demo-transport.ts:482` — the UI's offline fake host.
Why it matters: the shipped player's volume slider is a dead control against every real backend (and "engine-reported, never optimistic" volume stays null → slider disabled), while the mixer sits permanently at UNITY. The EQ preamp (`Eq3Band`, which takes a `Volume`) is likewise constructed by zero production code. This is the "UI settings wired to nothing" pattern at S2 scale: a primary player control that cannot work in any shipped composition.
What correct looks like: one `Command::AudioSetVolume { volume } =>` arm in the CLI route mapping 0..1 → `Volume::from_linear` → `pipeline.set_volume(0, v)` (plus the audio-volume event emission the UI already listens for, context.tsx:236).
Confidence: HIGH — Verification: read+ran (rg traces across ui/, apps/, crates/; volume tests executed; no executor found repo-wide).
Severity note: the volume.rs code itself is correct for its tested domain; the S2 lands on the file because its entire reason to exist — runtime gain control — is unreachable, and the doc's framing ("the control boundary") suggests a wiring that does not exist.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| UNITY / MUTE consts | 29-33 | REAL | — | bit-exact identities; executed |
| from_linear | 44-52 | REAL | — | rejects non-finite/negative; executed |
| from_db | 62-71 | REAL (broken edge) | S2 | F-001: output-overflow unchecked |
| linear | 74-76 | REAL | — | accessor |
| db | 81-83 | REAL | — | MUTE → −inf (documented); inf-linear (via F-001) → +inf |
| apply | 86-88 | REAL | — | pure multiply; executed incl. NaN passthrough |
| tests (5) | 96-175 | REAL | — | honest; missing only the extreme-dB case that would catch F-001 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| unity_is_bit_preserving | YES | `to_bits` equality across ±0, extremes |
| mute_zeroes_everything_finite | YES | exact zero + −inf dB assert |
| db_construction_matches_the_amplitude_convention | YES | ±6 dB numeric pins + round trip |
| linear_construction_rejects_bad_values | YES | three typed rejection codes |
| apply_is_a_pure_multiply | YES | exact products + NaN payload survival |
Gap: no test constructs a dB magnitude above 770.6 — F-001 is invisible to the suite.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the stored value is always finite and (linearly) non-negative" | volume.rs:16-17 | FALSE | from_db(771.0) stores +inf (rustc-executed proof) — F-001 |
| "a NaN *gain* would poison every output and is rejected at the control boundary" | volume.rs:18-20 | HALF-FALSE | NaN is rejected; +inf is not, and poisons identically |
| "dB gain is finite and converts as `10^(db/20)`" | volume.rs:7-8 | REAL (for |db| ≤ 770) | executed tests |
| "MUTE renders as NEG_INFINITY — the honest zero on a log scale (documented, not clamped)" | volume.rs:79-80 | REAL | executed test asserts −inf |
| "clipping is the stage-D limiter's business" | volume.rs:36-37 | UNVERIFIED | no limiter reachable from shipped code found in my traces (the stage-D limiter card is future work per doc 03 §4.8 e) |

## 7. Dead / duplicated / unreachable code
- The whole mutation surface (from_linear/from_db/set_volume callers, Gain, EQ preamp) is production-dead today: exercised only by tests and the UI demo transport (F-002).

## 8. Dependencies & flags
- media_types (EngineError) + crate::error::bad_gain (verified present, error.rs:19). No features, no unsafe.

## 9. Open questions for the next agent
- W07-A06 (effects/eq.rs): the EQ preamp Volume is equally unwired — coordinate the F-002 reachability story.
- Whoever owns apps/cli/src/dispatch.rs: the `AudioSetVolume` arm is the single missing link for the whole volume feature; the UI, catalog (commands/src/catalog), and engine are all waiting on it.
