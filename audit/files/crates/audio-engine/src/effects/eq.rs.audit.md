# AUDIT: crates/audio-engine/src/effects/eq.rs
| Field | Value |
|---|---|
| Auditor | W07-A06 |
| Date | 2026-11-21 |
| Lines / functions | 392 / 16 (12 prod items + 9 tests) |
| File verdict | UNREACHABLE (real, correct, tested DSP; no shipped entry point constructs it) |
| Reachable from shipped entry point? | NO — `lumen` CLI imports only `audio_engine::{AudioPipeline, PipelineSpec}` (apps/cli/src/pipeline_host.rs:51), and `AudioPipeline` never references `EffectChain`/`Eq3Band` (rg over crates/audio-engine/src: Eq3Band appears only in eq.rs itself, the effects.rs re-export, and tests — confirmed independently and by audit/files/crates/audio-engine/src/effects.rs.audit.md's caller census). The UI's EQ controls are rendered inert-by-design "awaiting the `audio.set-eq` catalog command" (ui/src/screens/settings/areas/effects.tsx:7-8,44-45), and that command does not exist (feature-matrix notes: "D-012 pending"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | rg `Eq3Band` repo-wide (no construction outside eq.rs/tests/effects.rs re-export); rg `audio_engine::` in apps/cli/src (pipeline_host.rs:51,619,671,1068 — pipeline/sink/drift only); sed -n ui/src/screens/settings/areas/effects.tsx:1-60 (staged-inert disclosure); sed -n docs/feature-matrix.toml:1660-1685 (afeat-equalizer row); grep baseline log (9/9 eq tests ok, :63-71); RBJ cookbook formula verification by hand (all three filters, entry-by-entry) |

## 1. One-paragraph verdict
The DSP is correct: all three RBJ audio-EQ-cookbook transfer functions (low shelf, high shelf, peaking) match the published formulas coefficient-for-coefficient (verified by hand below), the 0 dB collapse to identity is exact algebra, and the 9 tests — including analytic |H| gain assertions computed from first principles (10^(6/20), cookbook |H(w0)|=A, RMS-based 3-samples-per-period amplitude probing) — all executed in the baseline run. But this is a museum piece: nothing in any shipped build constructs an `Eq3Band` or even an `EffectChain`; the UI equalizer renders disabled with an honest reason; the `audio.set-eq` command it awaits is unimplemented. The file also carries four smaller defects: the topology is mislabeled "direct-form-1" when the code is transposed direct form II, `Eq3Band::flat` cannot be constructed at sample rates ≤ 8 kHz (the hard-coded 4 kHz shelf corner hits the Nyquist check), unbounded finite gains pass the parameter gate and produce inf/NaN coefficients, and the "bit-identical" identity claim has a signed-zero exception.

## 2. What actually works (verified)
— RBJ formula fidelity (hand-checked against the cookbook, independent of the code's tests): low_shelf b0 = A((A+1)−(A−1)cos w0+2√A α) etc. (eq.rs:67-74 ✓), high_shelf (eq.rs:90-97 ✓), peaking b0=1+αA, a0=1+α/A (eq.rs:113-120 ✓); A=10^(G/40), w0=2πf/Fs, α=sin w0/(2Q) (eq.rs:143-150 ✓).
— 0 dB identity algebra: at gain 0, A=1 ⇒ b1 and a1 expressions both reduce to exactly −4·cos w0 (bit-equal after exact power-of-two scalings), b2 and a2 to 2−2α ⇒ normalized coefficients with b1==a1, b2==a2 ⇒ y[n]=x[n] sample-for-sample. Pinned by `flat_eq_is_bit_exact_identity ... ok` (baseline test-workspace.log.txt:65).
— All 9 tests ok in baseline (:63-71), including gain-magnitude tests with analytic expected values (not golden-from-subject).
— Hot-path claim: `process` allocates only at state warm-up (eq.rs:174-178); per-sample work is 5 mul + 4 add/sub per band (matches the doc's D-1 claim).

## 3. Findings
### F-001 [S3] The entire EQ is dead code in shipped builds — "partial" matrix status inflates zero user reachability
Where: crates/audio-engine/src/effects/eq.rs:1-2 vs apps + UI
Code:
```rust
//! Three-band biquad equalizer (issue #33 M2 — the `afeat-equalizer`
//! row's engine core: low/mid/high shelf + preamp, pure Rust).
```
What is wrong: No shipped entry point constructs `Eq3Band` (or any `AudioEffect`): the CLI uses `AudioPipeline`/mixer only (pipeline_host.rs:51), the web host likewise (apps/web/src/host.rs imports `AudioPipeline, PipelineSpec`), tauri has no audio dependency. The UI's EQ toggles/sliders render disabled "awaiting the `audio.set-eq` catalog command" (effects.tsx:7-8) — a command that does not exist. A user cannot reach this DSP by any action in any shipped binary.
Why it matters: The matrix row `afeat-equalizer` (docs/feature-matrix.toml:1673-1684) declares `status = partial` on windows/linux/macos. The notes honestly disclose the 3-band/10-band gap and the pending command, but "partial" still credits a feature with no reachable path — the parity-report's 0/252 posture in miniature. Anyone wiring EQ later must also fix F-003 before 8 kHz streams work.
What correct looks like: status `not_started` (or `blocked` with the D-012 dependency) until a command → `EffectChain` wiring exists; keep the engine core as the implementation seam.
Confidence: HIGH — Verification: read (caller census + UI staging text + prior effects.rs audit's independent census).

### F-002 [S4] Filter topology mislabeled: code is transposed direct form II, docs say "direct-form-1"
Where: crates/audio-engine/src/effects/eq.rs:6 and 152
Code:
```rust
/// One biquad with per-channel direct-form-1 state.
```
What is wrong: Direct form 1 needs four state values per channel (x1, x2, y1, y2). The state here is `Vec<[f32; 2]>` (eq.rs:157) and the update equations (eq.rs:183-185: `y = b0*x + z[0]; z[0] = b1*x − a1*y + z[1]; z[1] = b2*x − a2*y`) are textbook **transposed direct form II** (two state variables). The math is correct; the label is wrong in both the module doc (line 6) and the struct doc (line 152).
Why it matters: Naming that lies. TDF2 and DF1 have different numerical-noise properties; a maintainer reasoning about "DF1" from the docs will mis-analyze denormal/overflow behavior.
What correct looks like: Change both doc comments to "transposed direct form II".
Confidence: HIGH — Verification: read (structure equations compared against both canonical forms).

### F-003 [S4] `Eq3Band::flat(rate)` is unconstructible at sample rates ≤ 8 kHz
Where: crates/audio-engine/src/effects/eq.rs:218-220 and 137
Code:
```rust
            high: BiquadFilter::new(BiquadCoeffs::high_shelf(rate, 4000.0, high_db, 0.707)?),
```
What is wrong: `check_params` requires `freq < rate/2` (eq.rs:137). At rate = 8000 the high-shelf corner 4000 Hz fails `4000 < 4000`, so `Eq3Band::new` — and therefore `flat`, the *identity* — returns `Err(audio.effects_eq_params)`. Every 8 kHz stream (telephony PCM/µ-law rates this project's demuxers accept) cannot even get a flat EQ.
Why it matters: Latent today (F-001: nothing constructs it), but the moment the EQ is wired, low-rate audio either errors at construction or needs a special case.
What correct looks like: Clamp corners to < rate/2 (e.g., `freq.min(rate as f32 / 2.0 * 0.95)`) or skip a band whose corner is not realizable at the rate, instead of failing the whole EQ.
Confidence: HIGH — Verification: read (guard arithmetic traced; `4000 >= 8000/2` is arithmetic).

### F-004 [S4] Finite-but-extreme gains pass the parameter gate and yield inf/NaN coefficients (NaN audio)
Where: crates/audio-engine/src/effects/eq.rs:133-141 and 145
Code:
```rust
    if !freq.is_finite() || !gain_db.is_finite() || !q.is_finite() || q <= 0.0 {
        return Err(error::unsupported_format("audio.effects_eq_params"));
    }
```
What is wrong: `gain_db = 1e30` is finite, so it passes; `A = 10.0_f32.powf(gain_db / 40.0)` (eq.rs:145) overflows to `inf`, and the normalized coefficients become inf/inf = NaN. The filter then emits NaN samples forever (no panic — the process loop is arithmetic-only). A coefficient-finiteness check after `normalize` would catch it.
Why it matters: Typed-refusal discipline exists for the params but not for the *derived* values; a settings file or command carrying an absurd dB value turns audio into NaN instead of a typed error. Unreachable today (F-001), so S4 not S2.
What correct looks like: `if !(b0.is_finite() && a0 != 0.0 && …)` after normalization → typed error; or bound |gain_db| (VLC's own EQ gates at ±20 dB per band).
Confidence: HIGH — Verification: read (float overflow path traced; f32::powf overflow to inf is standard IEEE).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `BiquadCoeffs` | 32-39 | REAL | — | 5 normalized coeffs (a0 divided out) |
| `BiquadCoeffs::normalize` | 42-50 | REAL | — | a0 > 0 provable for finite inputs (min at cos=−1 is 2+2√A·α); NaN case = F-004 |
| `BiquadCoeffs::low_shelf` | 58-75 | REAL | — | matches RBJ cookbook exactly (verified) |
| `BiquadCoeffs::high_shelf` | 81-98 | REAL | — | matches RBJ cookbook exactly (verified) |
| `BiquadCoeffs::peaking` | 105-121 | REAL | — | matches RBJ cookbook exactly; \|H(w0)\|=A property pinned by test |
| `BiquadCoeffs::IDENTITY` | 124-130 | REAL | — | const (1,0,0,0,0) |
| `check_params` | 133-141 | REAL | — | finite + q>0 + freq ∈ (0, rate/2); rate=0 naturally refuses; no gain magnitude bound (F-004) |
| `precompute` | 143-150 | REAL | — | A=10^(G/40), w0, α=sin/(2Q) ✓ |
| `BiquadFilter::new` | 162-167 | REAL | — | empty state |
| `BiquadFilter::coeffs` | 170-172 | REAL | — | getter |
| `BiquadFilter::process` | 174-189 | REAL | — | TDF2 (mislabeled DF1 — F-002); per-channel state; warm-up reset on channel-count change (documented); alloc-free after warm-up; no panic paths |
| `Eq3Band::new` | 209-222 | REAL | — | hard-coded corners 250/1k/4k documented; F-003 rate ≤ 8k failure |
| `Eq3Band::flat` | 225-227 | REAL | — | identity composition; fails ≤ 8 kHz (F-003) |
| `Eq3Band::preamp` | 230-232 | REAL | — | getter |
| `AudioEffect::process` (Eq3Band) | 236-249 | REAL | — | preamp skip at exactly g==1.0 (identity preserved); band order preamp→low→mid→high as documented |
| `AudioEffect::name` | 251-253 | REAL | — | "eq3" |
| tests ×9 | 263-391 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| flat_eq_is_bit_exact_identity | YES | exact `assert_eq!` on f32 slices; any coefficient deviation breaks it |
| identity_coeffs_are_the_trivial_filter | YES (weakly) | pins const values |
| low_shelf_boosts_bass_by_the_asked_db | YES | expected 0.5·10^(6/20) computed analytically, 5% window |
| high_shelf_cuts_treble_by_the_asked_db | YES | analytic −6 dB with RMS probe (correctly handles 3-samples/period), 2% window |
| mid_peak_gains_at_center_frequency | YES | cookbook \|H(w0)\|=A, analytic |
| preamp_scales_before_the_bands | YES | ×0.5 settled amplitude |
| per_channel_state_is_independent | YES | shared-state mutation would put energy into the silent channel |
| bad_parameters_are_typed_errors | YES | 4 refusal shapes |
| eq_plugs_into_the_effect_chain | YES | chain-level bit-exact identity |

All 9 executed ok in baseline (test-workspace.log.txt:63-71).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "At 0 dB … the output is bit-identical to the input" | eq.rs:11-16 | REAL (with a signed-zero footnote) | identity algebra verified; test pins f32 equality. Exception: a −0.0 input sample flips to +0.0 on the first quantum (y = 1.0·x + 0.0); inaudible and invisible to `==`, but not bit-identical under `to_bits()` |
| "direct-form-1 biquad" | eq.rs:6,152 | FALSE (label) | code is TDF2 (F-002) |
| "process allocates only on the first quantum" | eq.rs:20-22 | REAL | state warm-up only (eq.rs:174-178) |
| afeat-equalizer row: status `partial`, "engine core is 3-band fixed-corner, NOT VLC's 10-band … audio.set-eq … pending; UI sliders render inert" | docs/feature-matrix.toml:1673-1684 | SHALLOW | engine core real+tested, but zero reachability in shipped builds (F-001); notes disclose the gaps honestly, the status cell over-credits |

## 7. Dead / duplicated / unreachable code
- The whole module is unreachable from shipped entry points (F-001) — alive only in `cargo test`.
- No duplication found; `Volume` reuse (preamp) is the sanctioned crate-local type.

## 8. Dependencies & flags
- `media_types::EngineError`, `crate::error`, `crate::volume::Volume` — std-only DSP, no external deps, no `unsafe`, no feature gate. Compiled into every default build as dead weight (with the rest of the effects family, per the prior effects.rs audit).

## 9. Open questions for the next agent
- audio-engine/src/effects.rs (audited W06) flagged a cross-file NaN concern into `Volume::from_db` — I own volume.rs this wave; see my volume.rs audit for the verdict on that concern.
- Whoever wires `audio.set-eq` (D-012): fix F-003 (corner clamping) and F-004 (coefficient finiteness gate) before the EQ becomes reachable; 8 kHz streams and absurd settings values are the two immediate failure shapes.
