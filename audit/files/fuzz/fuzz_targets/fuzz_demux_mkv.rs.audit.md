# AUDIT: fuzz/fuzz_targets/fuzz_demux_mkv.rs
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 20 / 1 fuzz_target! |
| File verdict | REAL-UNPROVEN (thin shim to a REAL, baseline-tested in-crate harness; campaign never executes) |
| Reachable from shipped entry point? | NO (fuzz/ scaffold workspace, unwired) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. One-paragraph verdict
Identical shape to fuzz_codec_h264.rs: `fuzz_target!` ->
`demux::mkv::fuzz::fuzz_demux_mkv` — the function exists
(crates/demux/src/mkv/fuzz.rs:38, verified) and its in-crate tests passed in
the baseline. The campaign-never-runs finding (F-001) is identical to the
h264 shim's: scaffold workspace, stable-pinned toolchain, no CI lane, no
committed cargo-fuzz output.

## 2. What actually works (verified)
- `rg 'pub fn fuzz_demux_mkv' crates/` -> crates/demux/src/mkv/fuzz.rs:38.
- Baseline (cargo-test-workspace.txt:1965-1966): `test mkv::fuzz::tests::
  empty_and_marker_inputs_survive ... ok / hostile_wide_inputs_survive ... ok`.
- Corpus twin seed_crossformat_mkv.mkv audited this wave (EBML walk EOF-exact).

## 3. Findings
F-001 [S3] Where: fuzz/Cargo.toml:5-10 + this file (same as the h264 shim's
F-001). The cargo-fuzz campaign has never executed in any recorded lane; the
shim's only consumers-by-convention corpus (fuzz/corpus/fuzz_demux_mkv/…)
is dead weight. Confidence: HIGH — read + baseline greps.

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| fuzz_target! body | 17-19 | REAL-UNPROVEN | calls the in-crate harness (baseline-tested) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| (in-crate twins) mkv::fuzz::tests::* | YES | ran green in baseline on the same harness body |
## 6. Doc claims: "panic-free by construction" — UNVERIFIED (no fuzz execution); "compiles on stable (cargo check in fuzz/)" — UNVERIFIED.
## 7. Dead code: shim + corpus executable-dead until nightly wired.
## 8. Dependencies: libfuzzer-sys, demux (path).
## 9. Open questions: none.
