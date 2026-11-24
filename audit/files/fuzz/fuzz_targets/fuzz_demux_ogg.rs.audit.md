# AUDIT: fuzz/fuzz_targets/fuzz_demux_ogg.rs
| Field | Value |
|---|---|
| Auditor | W16-A03 |
| Date | 2026-11-24 |
| Lines / functions | 21 / 1 macro invocation |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — only via the UNWIRED nightly campaign (fuzz/README.md:17); compiles under cargo check per README:38-41 (not re-runnable here: no cargo) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read; rg "fn fuzz_demux_ogg" → crates/demux/src/ogg/fuzz.rs:50; rg 'name = "fuzz_demux_ogg"' fuzz/Cargo.toml:118; ls fuzz/corpus/fuzz_demux_ogg/ (3 seeds) |

## 1. One-paragraph verdict
Identical pattern to fuzz_codec_mp3.rs: a one-call libFuzzer shim over the
real in-crate harness `demux::ogg::fuzz::fuzz_demux_ogg` (exists at
crates/demux/src/ogg/fuzz.rs:50), registered (fuzz/Cargo.toml:118),
gate-checked, 3-seed corpus present. Campaign unwired — stated honestly in
this file's own header. No defects.

## 2. What actually works (verified)
— rg: harness fn exists (crates/demux/src/ogg/fuzz.rs:50
`pub fn fuzz_demux_ogg(data: &[u8]) {`).
— In-crate hostile-input sweeps for ogg run on stable (baseline
cargo-test-workspace.txt — demux suite green).
— Seed corpus: seed_ogg_chain.ogg, seed_ogg_trunc0.bin,
seed_ogg_vorbis16k.ogg (ls).

## 3. Findings
None. Compiles-on-this-tree: UNVERIFIED (no cargo; README records
verification 2026-11-09).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_target!(data → fuzz_demux_ogg) | 18-20 | REAL-UNPROVEN | — | one full untrusted pass per call |

## 5. Test honesty
No tests in this file (test infrastructure itself).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one call = one full untrusted pass over the bytes" | file :4-5 | REAL | shim passes `data` straight through |
| wiring status honest note | file :7-10 | REAL | README LEAD-DECISION |

## 7. Dead / duplicated / unreachable code
None. Gate-enforced presence.

## 8. Dependencies & flags
libfuzzer-sys 0.4.13; fuzz/ is its own workspace root.

## 9. Open questions for the next agent
None.
