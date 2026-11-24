# AUDIT: fuzz/fuzz_targets/fuzz_codec_h264.rs
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 20 / 1 fuzz_target! |
| File verdict | REAL-UNPROVEN (thin shim to a REAL, baseline-tested in-crate harness; the cargo-fuzz campaign it serves never executes anywhere) |
| Reachable from shipped entry point? | NO (fuzz/ is its own workspace scaffold, not wired into any gate) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. One-paragraph verdict
A 20-line honest shim: `fuzz_target!` -> `codecs::video::h264::fuzz::fuzz_codec_h264`
— that function exists (crates/codecs/src/video/h264/fuzz.rs:49, verified) and its
in-crate deterministic tests passed in the committed baseline. The shim itself
can only execute under cargo-fuzz + nightly, which this repo does not have:
fuzz/Cargo.toml's own header says "This manifest is a *scaffold* ... It is NOT
wired into any gate ... this repository's toolchain is stable-pinned 1.99.1".

## 2. What actually works (verified)
- `rg 'pub fn fuzz_codec_h264' crates/` -> crates/codecs/src/video/h264/fuzz.rs:49
  — the harness body is real, not invented.
- Baseline (cargo-test-workspace.txt:645-646): `test video::h264::fuzz::tests::
  empty_and_marker_inputs_survive ... ok / hostile_wide_inputs_survive ... ok`
  — the harness logic is exercised by committed tests.
- Shim wiring correct: `#![no_main]` + `libfuzzer_sys::fuzz_target!` calling the
  documented path verbatim (read).

## 3. Findings
F-001 [S3] Where: fuzz/Cargo.toml:5-10 (scaffold) + this file.
Code (fuzz/Cargo.toml:5-8): "# This manifest is a *scaffold*: it makes the three
previously-dead target files under fuzz_targets/ buildable by cargo-fuzz. It is
NOT wired into any gate".
What is wrong: the fuzz campaign — 40+ corpus seeds across ~30 targets, three
shims, a Cargo scaffold — has never executed anywhere: no nightly toolchain in
repo, no CI lane, no run in audit/runs/baseline/ (rg "fuzz" over rc-smoke/
full_gate: zero campaign evidence). The "fuzzing" line of the quality story is
served entirely by in-crate deterministic sweeps of the same harness bodies.
Honest (the comments say exactly this), but the corpus seeds I audited this
wave are then dead assets in every recorded lane. What correct looks like: a
nightly cargo-fuzz smoke run in CI or dropping the corpus.
Confidence: HIGH — read + baseline greps. Verification: read (UNVERIFIED:
nothing ever compiled fuzz/ on stable either).

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| fuzz_target! body | 17-19 | REAL-UNPROVEN | one full untrusted pass per call via in-crate harness (tested in baseline) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| (in-crate twins) video::h264::fuzz::tests::* | YES | ran green in baseline on the same harness body this shim calls |
## 6. Doc claims: "panic-free by construction, bounds documented there" (:4-5) — the in-crate module documents bounds; UNVERIFIED panic-freedom (no fuzz execution ever); "compiles on stable (cargo check in fuzz/)" (:9-10) — UNVERIFIED, no committed cargo check of fuzz/.
## 7. Dead code: the shim + its corpus are executable-dead until nightly is wired.
## 8. Dependencies: libfuzzer-sys (fuzz-local), codecs (path).
## 9. Open questions: none.
