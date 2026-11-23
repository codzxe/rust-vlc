# AUDIT: crates/audio-engine/src/effects.rs
| Field | Value |
|---|---|
| Auditor | W07-A08 |
| Date | 2026-11-23 |
| Lines / functions | 415 / 14 incl. trait + impls (+12 tests, 2 helpers) |
| File verdict | REAL (correct, tested DSP) / UNREACHABLE — nothing outside this crate's tests ever constructs an effect or a chain |
| Reachable from shipped entry point? | NO. Exhaustive repo-wide search (`rg -l "EffectChain|Eq3Band|BiquadFilter|AudioEffect"` minus audio-engine/audit) returns only effects.rs, effects/eq.rs, lib.rs (re-export), and UI/docs files that merely *name* the types. The shipped audio path is `AudioPipeline` (pipeline.rs) which applies per-track `Volume` in the mixer (mix.rs:55-61) and never touches `EffectChain`. No host handles `audio.set-volume`: `rg "set-volume" apps/` → zero matches (cli, web, desktop, control-server all absent; only crates/commands catalog spec + UI's demo-transport mock). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `./target/debug/deps/audio_engine-78c3118f7cd3bd18 --test-threads=2 effects` → 20 passed, exit 0; `./target/debug/deps/audio_engine-9c8a3ce142468d30 --test-threads=2` → 86 passed, exit 0 |

## 1. One-paragraph verdict
Hostile-focus answer: the effects that exist are **Gain, Mute, Balance (constant-power pan)** in this file, plus the `eq` submodule's **Eq3Band** (preamp → low shelf → mid peak → high shelf, RBJ-cookbook biquads) re-exported at line 48; `EffectChain` composes up to 8 stages. The DSP itself is real and clean — validated constructors (NaN/out-of-range rejected typed), a documented and tested pan law, bit-exact unity gain, bounded chain, zero allocation in the hot loop — and I executed the suites (20 + 86 pass). But **not one effect is wired**: no production code anywhere constructs `Gain`, `Mute`, `Balance`, `Eq3Band`, or an `EffectChain`; the pipeline's volume path is the mixer's own per-track multiply; `audio.set-volume` exists in the command catalog and the UI's mock transport but has no handler in any shipped host. The module doc's claim that `Gain` is "the effect-chain's front door for `audio.set-volume`" is false — that command has no door at all.

## 2. What actually works (verified)
— `./target/debug/deps/audio_engine-78c3118f7cd3bd18 --test-threads=2 effects` → `test result: ok. 20 passed` (gain amplitude + bit-exact identity, mute-zero, constant-power L²+R²=1 at 0.0/0.37, hard stops pinned to 1e-7, out-of-range/NaN/non-stereo rejections, chain order, bound wall, gain∘mute).
— `Volume` discipline verified by reading volume.rs:44-71 (finite/non-negative linear; `10^(db/20)`); `Gain::process` is one multiply per sample (163-171); `Balance::process` validates exactly-2 planes typed (246-248) and uses `split_at_mut` + `zip` (252-261) — no panic possible on unequal plane lengths.

## 3. Findings
F-001 [S3]
Where: effects.rs:81-138 (EffectChain) and the whole file's product reachability
Code:
```
pub struct EffectChain {
    effects: Vec<Box<dyn AudioEffect>>,
}
```
What is wrong: dead abstraction. No code outside `audio-engine`'s own tests constructs an `EffectChain` or any `AudioEffect`; inside the crate, pipeline.rs/mix.rs/governor.rs never reference it (verified by caller search). The module doc itself concedes the insertion point is still empty ("doc 03 §4.8 b's insertion point B, empty in card 2.C3, starts here", line 8-10) — and it is still empty. The shipped volume path is `mix.rs` per-track `Volume` (pipeline.rs:237-238 `set_volume` → `mixer.set_volume`), itself called only from tests. Why it matters: 415 lines (+392 in eq.rs) of maintained, tested code with zero product callers; the "effect chain" exists only in unit tests. What correct looks like: either pipeline stage B constructs the chain from session config, or the surface is marked experimental/unwired in the docs. Confidence: HIGH — Verification: read (exhaustive `rg` caller search across crates/, apps/, tools/).

F-002 [S3]
Where: effects.rs:140-143 (doc comment on Gain)
Code:
```
/// A gain stage: one [`Volume`] scalar multiply on every sample of
/// every channel (the effect-chain's front door for `audio.set-volume`
```
What is wrong: the doc asserts a wiring that does not exist. `audio.set-volume` (commands catalog id AudioSetVolume, crates/commands/src/catalog/ids.rs:118) has **no handler in any shipped host** — `rg "set-volume" apps/` returns nothing for cli/web/desktop/control-server; the only implementation is the UI's `demo-transport.ts` mock (ui/src/client/demo-transport.ts:482). Even the mixer's `set_volume` that would be the real target is never called by production. The feature-matrix `afeat-volume` row (docs/feature-matrix.toml:1666-1667) builds on this: "ui settings area volume slider + mute switch live via audio.set-volume/audio.set-mute" — live in the demo transport, unserved by every real host. Why it matters: a reader of this doc believes volume commands flow through this stage. Confidence: HIGH — Verification: read + repo-wide search.

F-003 [S4]
Where: effects.rs:60-63 (trait contract)
Code:
```
pub trait AudioEffect {
    /// Process one quantum's planes, in place. Idempotent per quantum;
    /// stateless stages must be exactly re-runnable.
```
What is wrong: the contract is self-contradictory for the module's own stateful effect: `Eq3Band`/`BiquadFilter` keep per-channel IIR state (eq.rs:154-192), so processing a quantum twice is NOT idempotent — the sentence "Idempotent per quantum" can only describe the stateless stages, but it is written as the trait-level rule. Confidence: HIGH — Verification: read (eq.rs state fields).

F-004 [S4]
Where: effects.rs:46-48
Code:
```
pub mod eq;
pub use eq::{BiquadCoeffs, BiquadFilter, Eq3Band};
```
What is wrong: hygiene only — the EQ surface is exported through `effects::` while lib.rs:123 re-exports only `{AudioEffect, Balance, EffectChain, Gain, MAX_EFFECTS, Mute}` at the crate root, so the EQ is reachable only via the longer path `audio_engine::effects::Eq3Band`; nothing consumes either path (F-001). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AudioEffect::process/name (trait) | 60-73 | REAL | S4 | contract wording (F-003) |
| EffectChain::{new,push,len,is_empty,clear,process} | 91-138 | REAL (unwired) | S3 | bound enforced at 104-110; fail-fast order |
| Default for EffectChain | 85-89 | REAL | — | |
| Gain::{new,volume}, process, name | 149-176 | REAL (unwired) | S3 | one multiply; doc lie F-002 |
| Mute process/name | 182-198 | REAL (unwired) | — | ×0.0, NaN-sign semantics documented |
| Balance::{new,position,gains}, process | 206-268 | REAL (unwired) | — | typed range + stereo gates; constant-power law tested |
| (re-export eq) | 46-48 | REAL (unwired) | S4 | F-004 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| gain_scales_sine_amplitude | Yes | known sine in → predicted peak out at −6.02 dB |
| gain_identity_is_bit_exact | Yes | `assert_eq!` on the plane, not tolerance |
| mute_zeroes_everything | Yes | exact 0.0 checks |
| balance_center_is_minus_3db_constant_power | Yes | UNITY_GAIN const + L²+R²=1 at an arbitrary position |
| balance_hard_stops_are_pinned | Yes | 1e-7 trig tolerance, honest about f32 cos(π/2) |
| balance_scales_each_channel_by_its_law_gain | Yes | hard-left keeps L, kills R |
| balance_rejects_out_of_range_and_non_stereo | Yes | 1.01, NaN, mono → typed errors |
| chain_runs_stages_in_push_order | Yes (weakly) | comment itself admits gain∘balance vs balance∘gain is not distinguishable at the extreme; pins the composed value |
| empty_chain_is_identity_noop / chain_bound_is_a_wall / gain_mute_compose_to_silence | Yes | identity, 8-cap error, composition |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the effect-chain's front door for `audio.set-volume`" | effects.rs:141-143 | FALSE | no host handles audio.set-volume; no caller constructs Gain (F-002) |
| "insertion point B, empty in card 2.C3, starts here" | effects.rs:8-10 | REAL-as-admission | pipeline.rs never inserts it — still empty |
| "Balance uses the constant-power law … pinned by tests, not prose" | effects.rs:19-34 | REAL | tests executed, pass |
| "Allocation in process, panics, unwraps, NaN gains — never" | effects.rs:36-39 | REAL | read all process bodies; constructors validate |
| matrix afeat-volume: "ui settings area volume slider + mute switch live via audio.set-volume/audio.set-mute" | docs/feature-matrix.toml:1667 | MISLEADING | UI demo-transport mock only; no shipped host serves the command |
| matrix afeat-equalizer: "UI sliders render inert-with-reason until that command lands" | docs/feature-matrix.toml:1680 | REAL (honest) | ui/src/screens/settings/areas/effects.tsx stages the EQ rows as inert |

## 7. Dead / duplicated / unreachable code
The entire public surface (EffectChain, Gain, Mute, Balance, eq::Eq3Band/BiquadFilter/BiquadCoeffs) is unreachable from shipped entry points. `Gain` duplicates the mixer's per-track Volume multiply (mix.rs:55-61) — two implementations of "apply a Volume to planes", one shipped (mixer), one not (this file).

## 8. Dependencies & flags
No external deps (media_types + crate::error/volume only). No feature gates — the unreachability is not cfg-gating, it is simply never called. audio-engine is compiled into the release lane (rc-smoke.txt:104 `Compiling audio-engine`).

## 9. Open questions for the next agent
- pipeline.rs / mix.rs owners: is stage B insertion planned with a config seam (session → EffectChain), or should this surface be flagged unwired in doc 05 §2? The EQ UI already renders "inert-with-reason" awaiting `audio.set-eq` — a catalog command that does not exist yet.
- Who owns the commands-catalog↔host gap (`audio.set-volume` defined, unhandled)? That is a repo-wide pattern worth its own finding (afeat-volume matrix row rides it).
