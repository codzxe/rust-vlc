# AUDIT: fuzz/fuzz_targets/fuzz_codec_mjpeg.rs
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Lines / functions | 37 / 1 (fuzz_target shim) |
| File verdict | REAL-UNPROVEN (shim real, callee verified; libFuzzer campaign never executed) |
| Reachable from shipped entry point? | NO — fuzz bin, `test = false` in fuzz/Cargo.toml:77-78 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | full read; rg callee; rg baseline |

## 1. Verdict
Thin libFuzzer shim; one call = `codecs::video::mjpeg::fuzz::fuzz_codec_mjpeg` (verified to exist: crates/codecs/src/video/mjpeg/fuzz.rs:59). Two passes per input: free `decode_image(data)` entry + the codec-api contract path (configure/decode/flush) — mjpeg/fuzz.rs:59-73.
Wiring status (own header): nightly cargo-fuzz wiring owed by the Lead (repo toolchain is stable-pinned; cargo-fuzz needs `-Zsanitizer` nightly); target declared in fuzz/Cargo.toml:77-78 with `test = false / doc = false / bench = false` — never built/run by any gate; no cargo-fuzz run exists in audit/runs/baseline/ (rg "cargo-fuzz|libfuzzer" → no matches).
## 2. What actually works (verified)
The callee's deterministic in-crate equivalents run and pass in the baseline: cargo-test-workspace.txt:793-796 (`video::mjpeg::fuzz::tests::empty_input_survives_both_passes`, `hostile_marker_soup_survives_both_passes`, `soi_only_...`, `truncated_valid_stream_...` all ok).
## 3. Findings
F-001 [S3] The target is compile-ready but the fuzz lane has zero committed execution evidence; its seeds (fuzz/corpus/fuzz_codec_mjpeg/) are inert data (see seed audits). Confidence: HIGH — Verification: ran (rg baseline + Cargo.toml).
## 4. Test honesty
N/A — not a test; survivability harness. Would fail (as a crash) if the callee panicked on fuzz input — but only IF the campaign ever ran.
