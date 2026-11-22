# AUDIT: crates/audio-engine/src/effects.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 416 / 16 production items (1 trait, 1 chain, 3 stages) + 11 `#[test]` fns |
| File verdict | UNREACHABLE (clean, tested library code that no shipped binary constructs) |
| Reachable from shipped entry point? | **NO** — zero consumers outside `crates/audio-engine` itself: `EffectChain`/`Gain`/`Mute`/`Balance` are matched only by `lib.rs` (re-export), `effects/eq.rs` (sibling, another agent's file), and this file's tests. The pipeline's "insertion point B" — the place this chain was built to plug into — is documented **empty**: crates/audio-engine/src/pipeline.rs:4 ("point B empty"), lib.rs:17-19 ("insertion point B is empty in v0"). |
| Findings | S0:1 S1:0 S2:1 S3:0 S4:1 |
| Commands run | rg for `EffectChain|Gain::new|Balance|Mute|Eq3Band` consumers across crates+apps (only in-crate hits); rg for `set_volume` callers (pipeline.rs:237 — test-only callers); rg for `AudioSetVolume` handlers (catalog definitions + one control-protocol test only); sed quote verification |

## 1. One-paragraph verdict

The v0 effect chain is small and correct: a bounded (8-stage) `EffectChain`, a `Gain` multiply, a `Mute` zero-multiply, and a constant-power `Balance` with typed validation (non-finite/out-of-range positions refused; non-stereo plane sets refused) — all allocation-free in `process`, fail-fast composition, pan-law constants pinned by tests. As library code: fine. As a product feature: **fiction**. Nothing constructs an `EffectChain` in any shipped binary; the pipeline's insertion point B is empty by its own docs; the real (also unreachable) volume knob `AudioPipeline::set_volume` has test-only callers; and the `audio.set-volume` command the UI dispatches exists in the commands catalog with **zero handlers anywhere in the engine or apps**. The `afeat-volume` matrix row cites this file as evidence that "engine Gain/Mute stages + live settings controls shipped" — the stages ship as dead code and the "live" control is a UI button wired to a command nobody answers.

## 2. What actually works (verified)

- The pan law is real: `gains() = (cos((p+1)·π/4), sin((p+1)·π/4))` (239-241), constant power pinned by test (`balance_center_is_minus_3db_constant_power`, 320-328: `L²+R²=1` at 0.37 too), hard stops pinned at the honest f32 tolerance (331-340).
- `Balance::process` uses `split_at_mut` + `zip` — no panic on ragged planes (252-261), unlike the resampler's kernel (my resample.rs audit F-002).
- 11 in-file tests are behavioral (known sines → measured peaks, bit-exact identity, order-of-stages observability, bound-is-a-wall).
- W00 baseline cargo-test.log shows the audio-engine suite green (86 tests incl. these) — UNVERIFIED by my own execution (ADAPTATION).

## 3. Findings

### F-001 [S0] — The volume feature's evidence chain is false: this file is dead code, the mixer API is uncalled, and `audio.set-volume` has no handler in any shipped binary

Where: crates/audio-engine/src/effects.rs:81-138 (the chain nobody builds); docs/feature-matrix.toml:1667 (the evidence string); the missing handler for `crates/commands/src/catalog/command.rs:182`.

Code (docs/feature-matrix.toml:1667, verbatim):
```toml
evidence = "crates/audio-engine/src/effects.rs (EffectChain v0 + Gain/Mute AudioEffect stages, commit 8bd13ed; ui settings area volume slider + mute switch live via audio.set-volume/audio.set-mute + audio.get-volume hydrate, commit 0a8e70d; effects tests 86/0 in crate)"
```

Code (crates/audio-engine/src/lib.rs:17-19, verbatim):
```rust
//!    active tracks with per-track gain. No reverb, no EQ, no
//!    crossfade — those are later filter-chain cards (doc 03 §4.8 b,
//!    insertion point B is empty in v0).
```

What is wrong (three independent dead-ends, all verified by grep):
1. `EffectChain`, `Gain`, `Mute`, `Balance` have zero constructors outside this crate's own tests and the sibling `eq.rs`.
2. The alternate volume path — `AudioPipeline::set_volume` (pipeline.rs:237-238, which forwards to `Mixer::set_volume`) — is called only by in-file tests (pipeline.rs:533, 686).
3. `Command::AudioSetVolume` (`audio.set-volume`, commands/src/catalog/command.rs:180-182, ids.rs:118) has **no executor**: rg over `crates/playback-engine`, `crates/audio-engine`, `apps/cli`, `apps/desktop`, `apps/desktop-tauri` finds only the catalog definitions and one `control-protocol` policy test. The UI dispatches it (ui/src/screens/player/controls/ControlBar.tsx, ui/src/screens/player/context.tsx) into a command nobody answers.

Why it matters: the user cannot change the volume in any shipped binary, while the matrix row claims the feature "partial … shipped" with this file as evidence. This is the W00-identified pattern (evidence = path existence) in its purest form: the cited classes exist, compile, pass tests — and are unreachable. §4 S0: the claim is false.

What correct looks like: the tauri/CLI dispatch handles `audio.set-volume` by calling `AudioPipeline::set_volume` (or installing a `Gain` stage), and the evidence string names the reachable path.

Confidence: HIGH — Verification: read + exhaustive greps (`EffectChain|Gain::new|Balance|Mute`, `set_volume`, `AudioSetVolume`, `set-volume`).

### F-002 [S2] — The mixer's real volume seam and the effect chain are two parallel volume implementations, one dead and one unreachable

Where: crates/audio-engine/src/effects.rs:162-171 (`Gain::process`) vs crates/audio-engine/src/mix.rs:112 (`let gain = self.volumes[track].linear();`).

What is wrong: the codebase has two volume mechanisms — the per-track mixer gain (mix.rs, actually in the audio path, but with no production caller of its setter) and the `Gain` effect stage (this file, designed as "the effect-chain's front door for `audio.set-volume`" per the doc at 141-143, never installed). Neither is reachable from a user action. Duplicated responsibility (§3.5) plus double deadness.

Why it matters: any future volume wiring must pick one; the other rots. The doc comment on `Gain` (140-143) already misdescribes reality by claiming it is the front door for a command that has no handler.

Confidence: HIGH — Verification: read.

### F-003 [S4] — `Gain`'s doc comment promises a command that does not exist

Where: crates/audio-engine/src/effects.rs:140-143.

Code (140-143, verbatim):
```rust
/// A gain stage: one [`Volume`] scalar multiply on every sample of
/// every channel (the effect-chain's front door for `audio.set-volume`
/// — engine-side it is the same multiply the mixer applies, exposed as
```

What is wrong: `audio.set-volume` has no engine-side existence beyond the catalog row (F-001). Naming a phantom command in a doc comment is §3.4 evidence-debt. Hygiene given the comment describes intent, but it has been false for at least one public release.

Confidence: HIGH — Verification: read + grep.

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (pan law, bounds, never-list) | 1-44 | REAL-UNPROVEN | — | accurate about the math; silent about being unreachable |
| `MAX_EFFECTS` const | 52 | REAL-UNPROVEN | — | 8 |
| `AudioEffect` trait | 60-73 | REAL-UNPROVEN | — | in-place, `&'static str` name |
| `EffectChain::new`/`Default` | 85-97 | UNREACHABLE | S0 | F-001; capacity pre-reserved |
| `EffectChain::push` | 104-110 | UNREACHABLE | — | bound is a wall, typed error |
| `EffectChain::len`/`is_empty`/`clear` | 113-125 | UNREACHABLE | — | — |
| `EffectChain::process` | 132-137 | UNREACHABLE | — | fail-fast, empty = no-op |
| `Gain::new`/`volume` | 152-159 | UNREACHABLE | — | — |
| `Gain::process`/`name` | 163-175 | UNREACHABLE | S2 | F-002 duplicate volume path; simple multiply |
| `Mute` + impl | 182-198 | UNREACHABLE | — | `*sample *= 0.0` preserves NaN payload sign (documented) |
| `Balance::UNITY_GAIN`/`LEFT_GAIN`/`RIGHT_GAIN` consts | 208-215 | REAL-UNPROVEN | — | pinned by tests |
| `Balance::new` | 224-229 | UNREACHABLE | — | typed rejection of non-finite/OOR |
| `Balance::position`/`gains` | 232-241 | UNREACHABLE | — | constant-power law, correct math |
| `Balance::process`/`name` | 245-267 | UNREACHABLE | — | stereo check typed; zip = no OOB on ragged planes |
| tests (11) | 287-414 | REAL-UNPROVEN | — | §5 |

## 5. Test honesty (11 `#[test]`)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `gain_scales_sine_amplitude` (288-301) | YES | known sine → measured peak vs computed expectation |
| `gain_identity_is_bit_exact` (304-310) | YES | bit equality |
| `mute_zeroes_everything` (313-317) | YES | all samples == 0.0 |
| `balance_center_is_minus_3db_constant_power` (320-328) | YES | L²+R²=1 at two positions |
| `balance_hard_stops_are_pinned` (331-340) | YES | 1e-7 tolerance, honest f32 trig note |
| `balance_scales_each_channel_by_its_law_gain` (343-349) | YES | per-channel peaks |
| `balance_rejects_out_of_range_and_non_stereo` (352-363) | YES | typed refusals |
| `chain_runs_stages_in_push_order` (366-381) | YES | composition value pinned (0.4·0.5 on R) |
| `empty_chain_is_identity_noop` (384-390) | YES | bit equality |
| `chain_bound_is_a_wall` (393-402) | YES | 9th push errs |
| `gain_mute_compose_to_silence` (405-414) | YES | composition to zero |

All tests exercise dead-in-production code — honest about the math, silent about the reachability (the module never claims engine wiring, so no self-contradiction; the contradiction lives in the matrix row, F-001).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "afeat-volume … partial … engine Gain/Mute stages + live settings controls shipped" | docs/feature-matrix.toml:1665-1668 | FALSE | F-001: stages dead, `audio.set-volume` unhandled, no volume change possible in any shipped binary |
| "ui settings area volume slider + mute switch live via audio.set-volume/audio.set-mute" | same row | FALSE (implies end-to-end liveness) | UI dispatches the command (ControlBar.tsx); no handler exists engine-side |
| "insertion point B is empty in v0" | crates/audio-engine/src/lib.rs:19 | REAL | honest — and it is why this file is unreachable |
| Constant-power pan law with pinned constants | module docs 19-34 | REAL-UNPROVEN | implemented + tested; not run by me |

## 7. Dead / duplicated / unreachable code

The entire module is unreachable from shipped entry points (F-001): `EffectChain`, `Gain`, `Mute`, `Balance`. The re-exports at 48 (`BiquadCoeffs`, `BiquadFilter`, `Eq3Band` from the sibling `effects/eq.rs`) share the same status — `eq.rs` is another agent's file, but nothing outside audio-engine constructs an EQ either (grep for `Eq3Band` consumers: eq.rs + effects.rs re-export only).

## 8. Dependencies & flags

Depends on `media_types::EngineError`, `crate::error`, `crate::volume::Volume`. No feature gates; always compiled into the CLI/tauri binaries as dead code.

## 9. Open questions for the next agent

- `effects/eq.rs` (Eq3Band, `afeat-equalizer` row claims "partial: engine core is 3-band fixed-corner") — same reachability question; likely also dead-in-production. Assigned to the audio-engine agent auditing eq.rs.
- The UI volume button (`ui/src/screens/player/controls/ControlBar.tsx`) dispatching a dead command — W15's dead-buttons audit should cross-reference F-001.
- `Volume` (crates/audio-engine/src/volume.rs) — construction discipline (NaN rejection) is relied on here; other agent's file.
