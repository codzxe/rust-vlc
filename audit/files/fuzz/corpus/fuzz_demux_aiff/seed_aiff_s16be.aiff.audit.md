# AUDIT: fuzz/corpus/fuzz_demux_aiff/seed_aiff_s16be.aiff
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 35,334 B / 365bdd18e7324c9ae803e4953be120d9c26109ec42ca42c40c40e535bf4b5a53 |
| File verdict | REAL (valid-seed corpus file; structure independently parsed) |
| Reachable from shipped entry point? | N/A (fuzz corpus; consumed by cargo-fuzz target below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

Assignment rows 3021-3025 (5 chunks -> one audit). Parsed with python: FORM len
35,326 type `AIFF` (container length matches file size exactly), COMM: 2 channels,
8820 frames, 16 bits, 80-bit extended rate bytes `400eac44000000000000` = 44100.0
Hz; SSND len 35,288 = 8 (offset+blockSize) + 8820*2*2 = 35,280 — internally
consistent valid AIFF. This is a *valid* seed (not hostile).

Generated-vs-handmade: repo-side generated seed corpus (fuzz/README.md "Seed
corpora (committed — doc 04 §8.5)"). Consumer: cargo-fuzz target
fuzz/fuzz_targets/fuzz_demux_aiff.rs:19 `demux::aiff::fuzz::fuzz_demux_aiff(data);`
via the standard `fuzz/corpus/<target>/` directory convention (fuzz/Cargo.toml:97
declares the target); crates/demux/src/aiff/fuzz.rs:47 implements the harness.
No test or script references the file by name (rg over tests/ tools/ crates/
scripts/ = no hits) — corpus consumption is by convention only.
Matrix-evidence: none (not cited by docs/corpus/compat-matrix-20261110.jsonl).

Caveat (not a finding against this file): whether cargo-fuzz ever *runs* in CI is
a fuzz-infra question — the committed baselines (audit/runs/baseline/) contain no
fuzz run output, and fuzz/fuzz_targets/fuzz_demux_aiff.rs:12 says "Build (once
nightly is wired)" — see the fuzz target audits in this wave.
