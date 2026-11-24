# AUDIT: fuzz/fuzz_targets/fuzz_demux_mp3stream.rs
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Lines / functions | 20 / 1 (fuzz_target shim) |
| File verdict | REAL-UNPROVEN (shim real, callee verified; libFuzzer campaign never executed) |
| Reachable from shipped entry point? | NO — fuzz bin, `test = false` in fuzz/Cargo.toml:160-161 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | full read; rg callee; rg baseline |

## 1. Verdict
Thin libFuzzer shim; one call = `demux::es::fuzz::fuzz_demux_mp3stream` (verified to exist: crates/demux/src/es/fuzz.rs:90). Header-parse probes at 8 input-derived offsets + full drain of RawMp3Demuxer over a mem stream — es/fuzz.rs:90-101.
Wiring status (own header): nightly cargo-fuzz wiring owed by the Lead (repo toolchain is stable-pinned; cargo-fuzz needs `-Zsanitizer` nightly); target declared in fuzz/Cargo.toml:160-161 with `test = false / doc = false / bench = false` — never built/run by any gate; no cargo-fuzz run exists in audit/runs/baseline/ (rg "cargo-fuzz|libfuzzer" → no matches).
## 2. What actually works (verified)
The callee's deterministic in-crate equivalents run and pass in the baseline: cargo-test-workspace.txt:1891-1892 (`es::fuzz::tests::empty_and_marker_inputs_survive`, `hostile_wide_inputs_survive` ok).
## 3. Findings
F-001 [S3] The target is compile-ready but the fuzz lane has zero committed execution evidence; its seeds (fuzz/corpus/fuzz_demux_mp3stream/) are inert data (see seed audits). Confidence: HIGH — Verification: ran (rg baseline + Cargo.toml).
## 4. Test honesty
N/A — not a test; survivability harness. Would fail (as a crash) if the callee panicked on fuzz input — but only IF the campaign ever ran.
