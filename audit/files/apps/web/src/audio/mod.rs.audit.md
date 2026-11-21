# AUDIT: apps/web/src/audio/mod.rs
| Field | Value |
|---|---|
| Auditor | W12-A05 |
| Date | 2026-11-21 |
| Lines / functions | 48 / 4 config items (3 `pub mod` + 3 `pub use` groups; zero functions — module manifest) |
| File verdict | REAL (accurate module manifest; every doc claim I could mechanically check holds) |
| Reachable from shipped entry point? | NO from shipped binaries (crate UNREACHABLE, W01-A10 F-001). In-crate: `pub mod autoplay/sink/clock` compiled into the lib; autoplay consumed by `WebHost` (host.rs:102,813); sink/clock items consumed by `lib.rs:71`'s re-export (`AutoplayGate, WORKLET_QUANTUM, WorkletQueue`) and the module's own tests. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read audio/mod.rs 1-48 (full); `rg -n "WorkletQueue|ProcessReport|WORKLET_QUANTUM|WorkletClock|DriftPolicy|Resampler|WallclockAnchor" apps/web/src --glob '!audio/*'` (consumers: lib.rs:71 re-exports 3 of 8 names; clock items consumed nowhere outside audio/); `rg -n "the_shared_constants" -A 12 apps/web/src/audio/sink.rs` (WORKLET_QUANTUM==128 asserted, sink.rs:380-385) + `rg -n "WORKLET_QUANTUM" apps/web/src/audio/sink.rs` (defined :52, PREBUFFER 2× :121); `rg -n "128|QUANTUM" apps/web/audio-worklet.js` (JS twin: `const QUANTUM = 128` :56, `PREBUFFER_TARGET = 2 * QUANTUM` :58); baseline `orchrun-test-workspace-nofailfast.log.txt:3095-3109` (autoplay 4/4, sink 5/5, clock 6/6 ok); crate externals check via apps/web/Cargo.toml audit (W01-A02) + boundaries.toml (externals = serde/serde_json only ⇒ no web-sys/js-sys ⇒ the "no DOM and no imports" claim holds at the dependency level). Quotes verified with the Read tool. |

## 1. One-paragraph verdict
A 48-line module manifest whose value is its honesty: it states precisely what the audio pillar is (three pure-Rust MODELS — the autoplay gate, the worklet sink arithmetic, the sample-counter clock), what it is NOT (no AudioContext, no DOM, no imports — the page owns the browser objects), and what remains unwired (the worklet lane's end-to-end browser wiring is card 7.I1 `[VERIFY]`). Every mechanically checkable claim verified: the constants twin claim holds today (Rust `WORKLET_QUANTUM = 128` + 2×-quantum prebuffer vs JS `QUANTUM = 128` + `PREBUFFER_TARGET = 2 * QUANTUM`), the no-DOM claim holds at the dependency level (no web-sys/js-sys in the allowlist), and all 15 tests across the three submodules ran green in the repair run. The two findings are overstatement-shaped: "pinned to the same constants" suggests mechanical cross-language pinning that does not exist (the JS side is hand-maintained), and the clock model's re-exported surface has no consumer beyond its own tests without any marker saying so.

## 2. What actually works (verified)
— Verified by baseline `orchrun-test-workspace-nofailfast.log.txt:3095-3109` → `audio::autoplay::tests` 4/4 (3095-3098), `audio::clock::tests` 6/6 (3099-3104, incl. `drift_simulation_refreshes_at_the_bound_never_earlier`), `audio::sink::tests` 5/5 (3105-3109, incl. `the_shared_constants`, `the_prebuffer_target_is_two_quanta`) all `ok`.
— Verified by cross-file read: the "arithmetic twin" constants agree across languages TODAY — sink.rs:52 `pub const WORKLET_QUANTUM: usize = 128` and :121 `PREBUFFER_TARGET: usize = 2 * WORKLET_QUANTUM` vs audio-worklet.js:56 `const QUANTUM = 128` and :58 `const PREBUFFER_TARGET = 2 * QUANTUM`.
— Verified by dependency graph: the "wasm module has no DOM and no imports" claim (mod.rs:29-31) is enforced structurally — apps/web's external allowlist is exactly [serde, serde_json] (boundaries.toml, W01-A02 audit), so no web-sys/js-sys/wasm-bindgen DOM surface can exist.

## 3. Findings

### F-001 [S4]
Where: apps/web/src/audio/mod.rs:14-19 (with sink.rs:380-385, audio-worklet.js:56)
Code:
```
//! - [`sink`] — the AudioWorklet sink **model** (doc 09 §5.1): the
//!   arithmetic twin of the hand-written JS processor
//!   (`apps/web/audio-worklet.js`) — the bounded queue, the fixed
//!   128-frame render quantum, silence-on-underrun, and the played
//!   sample counter (the clock's truth source). The model is the
//!   spec the JS transcribes; both are pinned to the same constants.
```
What is wrong: "both are pinned to the same constants" reads as mechanical pinning, but only the Rust side is pinned (`the_shared_constants`, sink.rs:380-385, asserts 128/8192/clamps); the JS twin (`const QUANTUM = 128`, audio-worklet.js:56) is hand-maintained with no test, CI check, or codegen tying it to the Rust constant. They agree today (verified); nothing fails if they diverge.
Why it matters: the whole "model is the spec the JS transcribes" discipline rests on the two sides staying numerically identical; an unpinned side is exactly how a silent A/V-timing drift enters (e.g., someone "fixes" 128→256 render-quantum on one side only).
What correct looks like: a test that reads `apps/web/audio-worklet.js` and asserts the literal `= 128` / `= 2 * QUANTUM` lines (a one-file text check — the repo already does committed-artifact drift checks elsewhere, e.g. `lumenext-examples --check`), or codegen of the JS constants from the Rust source.
Confidence: HIGH — Verification: read (both files, both constants, and the absence of any cross-check test — sink.rs's tests never open the JS file).

### F-002 [S4]
Where: apps/web/src/audio/mod.rs:46-48
Code:
```
pub use autoplay::{AutoplayGate, GateChange};
pub use clock::{DriftPolicy, Resampler, WallclockAnchor, WorkletClock};
pub use sink::{ProcessReport, WORKLET_QUANTUM, WorkletQueue};
```
What is wrong: the clock re-export line (`DriftPolicy, Resampler, WallclockAnchor, WorkletClock`) exposes a public surface with zero consumers outside the audio module's own tests — lib.rs:71 re-exports only `{AutoplayGate, WORKLET_QUANTUM, WorkletQueue}`, host.rs consumes only autoplay, and no other crate can reach lumen-web. The models are deliberately contract-only (the module doc says so: "What this card lands is the typed contract"), but the re-export presents them as live API with no marker distinguishing "consumed" from "landed for card 7.I1".
Why it matters: dead-ish public surface in an already-unreachable crate compounds the audit burden: every future reader must re-derive that `Resampler`/`WallclockAnchor` are model-only. (It also means the resample policy — media rate ≠ context rate ⇒ ONE conversion — has NO production caller anywhere, which the docs imply but the re-export obscures.)
What correct looks like: a doc-attribute or comment on the clock/sink re-exports ("consumed by tests only until card 7.I1 wires the worklet lane"), or narrowing the re-export to what lib.rs actually forwards.
Confidence: HIGH — Verification: read + rg (consumer scan above; lib.rs:71 is the only non-audio consumer, and it forwards 3 of the 8 names).

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `pub mod autoplay` | 42 | REAL | — | consumed by host.rs (F-00x none); 4 tests green |
| `pub mod clock` | 43 | REAL | — | model-only (F-002); 5 tests green |
| `pub mod sink` | 44 | REAL | — | model + lib.rs re-export; 5 tests green |
| `pub use autoplay::{…}` | 46 | REAL | — | consumed (host.rs:102) |
| `pub use clock::{…}` | 47 | REAL | S4 | no consumer beyond tests = F-002 |
| `pub use sink::{…}` | 48 | REAL | — | WORKLET_QUANTUM/WorkletQueue forwarded by lib.rs:71; ProcessReport test-only |

## 5. Test honesty
No tests in this file (module manifest). The submodules' tests are covered by their own audits (sink.rs = W12-A02, clock.rs = W12-A06, autoplay.rs = mine, this wave); all green in the baseline repair run (`orchrun-test-workspace-nofailfast.log.txt:3095-3109`).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the arithmetic twin of the hand-written JS processor … the bounded queue, the fixed 128-frame render quantum, silence-on-underrun, and the played sample counter" | mod.rs:14-18 | REAL | sink.rs:52/:121 + worklet.js:56/:58 agree; sink tests (silence-on-underrun, counter) green |
| "both are pinned to the same constants" | mod.rs:18-19 | SHALLOW | agreement verified today; only the Rust side is pinned (F-001) |
| "No `AudioContext` is constructed here: the wasm module has no DOM and no imports — the PAGE owns the context" | mod.rs:29-31 | REAL | dependency allowlist [serde, serde_json] only (boundaries.toml; W01-A02) — no DOM crate can be linked |
| "The v1 host's own audio output remains the null device (the headless time source the pump drives)" | mod.rs:32-33 | REAL | open_null_lane (host.rs:1720-1752): NullBackend always; engine_info features tag `audio.null-device` |
| "the worklet lane's end-to-end browser wiring … is card 7.I1's `[VERIFY]` matrix" | mod.rs:33-35 | REAL-UNPROVEN | honest gap label; no wiring exists in-tree (rg: no consumer of WorkletQueue outside tests) |
| "the models' arithmetic is proven native-cfg by tests (labeled **simulation** …)" | mod.rs:36-38 | REAL | 14/14 submodule tests green in the repair run |

## 7. Dead / duplicated / unreachable code
`DriftPolicy/Resampler/WallclockAnchor/WorkletClock/ProcessReport` re-exports: test-only consumers (F-002). No duplication. The crate itself is unreachable from shipped entry points (header).

## 8. Dependencies & flags
None (pure manifest). Submodules are std-only per the crate's no-external rule; no feature gates at this level.

## 9. Open questions for the next agent
- W12-A03 (audio-worklet.js): F-001's cross-language pinning — if you find any constant in the JS that DIVERGES from sink.rs (queue bound 8192, underrun accounting), that upgrades F-001 from S4 to S2 immediately.
- W12-A02 (sink.rs) / W12-A06 (clock.rs): confirm no in-file consumer exists for the model items beyond tests (my scan says none; your files are the authority).
