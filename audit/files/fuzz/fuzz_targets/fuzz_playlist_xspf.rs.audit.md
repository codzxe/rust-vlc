# AUDIT: fuzz/fuzz_targets/fuzz_playlist_xspf.rs
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Lines / functions | 20 / 1 fuzz_target! |
| File verdict | REAL-UNPROVEN (thin shim to a REAL, baseline-tested in-crate harness; campaign never executes) |
| Reachable from shipped entry point? | NO (fuzz/ scaffold workspace, unwired) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. One-paragraph verdict
Third identical shape: `fuzz_target!` -> `playlists::fuzz::fuzz_playlist_xspf` —
the function exists (crates/playlists/src/fuzz.rs:45, verified) and the
playlists crate's hostile-input tests passed in the baseline. Campaign-never-runs
finding identical to the h264/mkv shims'.

## 2. What actually works (verified)
- `rg 'pub fn fuzz_playlist_xspf' crates/` -> crates/playlists/src/fuzz.rs:45.
- Baseline (cargo-test-workspace.txt:4787): `test
  fuzz_lite_mutations_of_valid_xspf_no_panic ... ok` (the xspf_suite) — the
  XSPF hostile path is exercised by committed tests.
- Corpus twin seed_dirty.xspf audited this wave (dirty-shape verified by read).

## 3. Findings
F-001 [S3] Where: fuzz/Cargo.toml:5-10 + this file (same as the h264 shim's
F-001): the cargo-fuzz campaign never executes in any recorded lane; the
corpus dir fuzz/corpus/fuzz_playlist_xspf/ (3 seeds) is convention-only and
dead weight. Confidence: HIGH — read + baseline greps.

## 4. Per-function table
| Function | Lines | Verdict | Notes |
|---|---|---|---|
| fuzz_target! body | 17-19 | REAL-UNPROVEN | calls the in-crate harness (baseline-tested via xspf_suite) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| (in-crate twins) fuzz_lite_mutations_of_valid_xspf_no_panic | YES | ran green in baseline |
## 6. Doc claims: "panic-free by construction" — UNVERIFIED (no fuzz execution); "compiles on stable" — UNVERIFIED.
## 7. Dead code: shim + corpus executable-dead until nightly wired.
## 8. Dependencies: libfuzzer-sys, playlists (path).
## 9. Open questions: none.
