# AUDIT: fuzz/fuzz_targets/fuzz_sub_vtt.rs
| Field | Value |
|---|---|
| Auditor | W16-A03 |
| Date | 2026-11-24 |
| Lines / functions | 21 / 1 macro invocation |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — only via the UNWIRED nightly campaign (fuzz/README.md:17); compiles under cargo check per README:38-41 (not re-runnable here: no cargo) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read; rg "fn fuzz_sub_vtt" → crates/subtitles/src/vtt/fuzz.rs:28; rg 'name = "fuzz_sub_vtt"' fuzz/Cargo.toml:258; ls fuzz/corpus/fuzz_sub_vtt/ (2 seeds) |

## 1. One-paragraph verdict
Same shim pattern: one call to the real in-crate harness
`subtitles::vtt::fuzz::fuzz_sub_vtt` (exists at
crates/subtitles/src/vtt/fuzz.rs:28), registered (fuzz/Cargo.toml:258),
gate-checked, 2-seed corpus (seed_vtt_hostile.vtt, seed_vtt_plain.vtt).
Campaign unwired, honestly disclosed in the file header. No defects.
Note the strength of the VTT lane's stable-side evidence: the feature matrix
cites "7003 fuzz inputs zero panics" for the vtt row
(docs/feature-matrix.toml:927) — that evidence belongs to the in-crate
sweep, NOT to this shim; nothing in the tree claims this target ran.

## 2. What actually works (verified)
— rg: harness fn exists (crates/subtitles/src/vtt/fuzz.rs:28
`pub fn fuzz_sub_vtt(data: &[u8]) {`).
— In-crate VTT hostile-input sweeps ran green in the baseline
(cargo-test-workspace.txt — subtitles suite).

## 3. Findings
None. Compiles-on-this-tree: UNVERIFIED (no cargo; README records
verification 2026-11-09).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fuzz_target!(data → fuzz_sub_vtt) | 18-20 | REAL-UNPROVEN | — | one full untrusted pass per call |

## 5. Test honesty
No tests in this file (test infrastructure itself).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "7003 fuzz inputs zero panics" (vtt row) | docs/feature-matrix.toml:927 | REAL-UNPROVEN for THIS file | evidence names crates/subtitles/src/vtt/** sweeps, not this target — attribution is correct (no false claim that this shim ran) |

## 7. Dead / duplicated / unreachable code
None. Gate-enforced presence.

## 8. Dependencies & flags
libfuzzer-sys 0.4.13; fuzz/ own workspace root.

## 9. Open questions for the next agent
None.
