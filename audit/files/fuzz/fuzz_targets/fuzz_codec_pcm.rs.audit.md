# AUDIT: fuzz/fuzz_targets/fuzz_codec_pcm.rs
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Lines / functions | 38 / 1 fuzz_target shim |
| File verdict | REAL — thin shim over a real, stable-tested harness; campaign lane UNWIRED (honest) |
| Reachable from shipped entry point? | NO — fuzz/ is its own workspace needing nightly cargo-fuzz; release lane never builds it. Harness logic itself is exercised on stable by crates/codecs/tests/pcm_fuzz_survival.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; rg harness fn + consumers; baseline full_gate fuzz-registry citation |

## 1. One-paragraph verdict
An honest shim: `fuzz_target!(|data: &[u8]| codecs::audio::pcm::fuzz::fuzz_codec_pcm(data));`
— one line of dispatch to the in-crate harness at
crates/codecs/src/audio/pcm/fuzz.rs:99 (`pub fn fuzz_codec_pcm` — rg
verified). The 28-line header documents the exact wiring status ("not yet
compiled in CI", Lead decision pending) and where the real proof lives
today. The header's claims check out: the harness fn exists and is
exercised deterministically on stable by pcm_fuzz_survival.rs (imports it
at :73, calls at :557, :930 over mutants and full parents).

## 2. What actually works (verified)
- Shim body (36-38, verbatim):
```
fuzz_target!(|data: &[u8]| {
    codecs::audio::pcm::fuzz::fuzz_codec_pcm(data);
});
```
- Harness exists: `crates/codecs/src/audio/pcm/fuzz.rs:99: pub fn fuzz_codec_pcm(data: &[u8])`.
- Stable-lane proof: crates/codecs/tests/pcm_fuzz_survival.rs:73 `use codecs::audio::pcm::fuzz::fuzz_codec_pcm;` — called over seeded mutations; suite green in baseline (cargo-test-workspace.txt).
- Registry gate PASS: audit/runs/baseline/full_gate.txt:56-58 ("fuzz-registry: CLEAN — 35 targets, diff empty, corpora present") — bin name `fuzz_codec_pcm` declared at fuzz/Cargo.toml:63 (verified).

## 3. Findings
None. Zero-findings justification: the only risk for a shim is pointing at
a nonexistent/renamed harness or being dead-registered — all three checked
(fn exists, bin declared, registry gate green, corpus present
[fuzz/corpus/fuzz_codec_pcm/] with 3 seeds including my audited
seed_pcm_u8.bin).

## 4. Per-function table (mandatory only >100 LOC; N/A — 38 lines)
| Unit | Lines | Verdict | Notes |
|---|---|---|---|
| header WIRING STATUS doc | 1-31 | REAL (accurate) | cross-checked against fuzz/README.md + baseline |
| fuzz_target! shim | 36-38 | REAL | dispatches to verified harness |

## 5. Test honesty
N/A (fuzz target). The in-crate survival suite would fail if the harness
panicked on hostile input — that is the executable contract today.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "harness LOGIC lives in codecs::audio::pcm::fuzz ... panic-free by construction" | :7-11 | REAL-UNPROVEN | fn exists; panic-freedom proven by the stable survival suite (baseline green), construction argument read in module docs by the codecs wave |
| "not yet compiled in CI (same status as fuzz_demux_avi/fuzz_codec_mjpeg)" | :23-25 | REAL | fuzz/README.md:17 "SCAFFOLDED, NOT WIRED" |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
libfuzzer-sys 0.4.13 (Lead-gated), codecs crate path dep. Nightly-only to BUILD in campaign mode; `cargo check` on stable per README.

## 9. Open questions
None.
