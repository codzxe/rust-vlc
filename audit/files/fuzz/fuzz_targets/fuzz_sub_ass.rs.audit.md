# AUDIT: fuzz/fuzz_targets/fuzz_sub_ass.rs
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Lines / functions | 20 / 1 (fuzz_target shim) |
| File verdict | REAL-UNPROVEN (shim real, callee verified; libFuzzer campaign never executed) |
| Reachable from shipped entry point? | NO — fuzz bin, `test = false` in fuzz/Cargo.toml:251-252 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | full read; rg callee; rg baseline |

## 1. Verdict
Thin libFuzzer shim; one call = `subtitles::ass::fuzz::fuzz_sub_ass` (verified to exist: crates/subtitles/src/ass/fuzz.rs:28). UTF-8-lossy parse of the whole input + bounded surface probes (title(), style("Default")) — ass/fuzz.rs:28-34.
Wiring status (own header): nightly cargo-fuzz wiring owed by the Lead (repo toolchain is stable-pinned; cargo-fuzz needs `-Zsanitizer` nightly); target declared in fuzz/Cargo.toml:251-252 with `test = false / doc = false / bench = false` — never built/run by any gate; no cargo-fuzz run exists in audit/runs/baseline/ (rg "cargo-fuzz|libfuzzer" → no matches).
## 2. What actually works (verified)
The callee's deterministic in-crate equivalents run and pass in the baseline: cargo-test-workspace.txt:5742,5753 (`ass::fuzz::tests::empty_and_marker_inputs_survive`, `hostile_wide_inputs_survive` ok).
## 3. Findings
F-001 [S3] The target is compile-ready but the fuzz lane has zero committed execution evidence; its seeds (fuzz/corpus/fuzz_sub_ass/) are inert data (see seed audits). Confidence: HIGH — Verification: ran (rg baseline + Cargo.toml).
## 4. Test honesty
N/A — not a test; survivability harness. Would fail (as a crash) if the callee panicked on fuzz input — but only IF the campaign ever ran.
