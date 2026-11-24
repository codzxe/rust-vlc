# AUDIT: fuzz/fuzz_targets/fuzz_codec_flac.rs
| Field | Value |
|---|---|
| Auditor | W16-A08 | Date | 2026-06-14 |
| Lines / functions | 20 / 1 fuzz_target! |
| File verdict | REAL (thin shim; harness verified) — UNREACHABLE as a running fuzz target: the campaign is SCAFFOLDED, NOT WIRED (fuzz/README.md:17) |
| Reachable from shipped entry point? | NO — cargo-fuzz binary; campaign never run (README:165-168 "No such run has happened yet"); compiles-claimed only |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg `fn fuzz_codec_flac` → crates/codecs/src/audio/flac/fuzz.rs:105 (exists); baseline full_gate.txt:56-58 registry gate PASS (35 targets) |

One-line audit per spec §12: a 7-line `fuzz_target!` shim calling the real
in-crate harness `codecs::audio::flac::fuzz::fuzz_codec_flac(data)` (rg: the
fn exists at flac/fuzz.rs:105). The comment block honestly states the wiring
status (nightly pending; proven by in-crate hostile-input sweeps until then).
The harness logic is exercised on stable by crates/codecs' fuzz_survival /
fuzz_loop_stable tests (baseline: codec fuzz_survival tests ran and passed).
Mutation honesty: n/a — the shim has no logic; the registry gate
(full_gate.txt:57 "CLEAN — 35 targets, diff empty, corpora present") keeps
the shim↔harness↔corpus triangle honest. No findings; checked the call
target, the no_main attribute, and the registry-row match (README:70).
