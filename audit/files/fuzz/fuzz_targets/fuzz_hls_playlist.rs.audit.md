# AUDIT: fuzz/fuzz_targets/fuzz_hls_playlist.rs
| Field | Value |
|---|---|
| Auditor | W16-A10 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / functions | 41 / 1 fuzz_target! + 1 #[path] mod |
| File verdict | REAL-UNPROVEN (dual-pass shim; both harness fns exist; netfuzz sweep baseline-proven) |
| Reachable from shipped entry point? | NO — fuzz campaign asset; nightly audit.yml fuzz tier |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg both harness fns; rg baseline netfuzz_sweep; rg audit.yml |

## 1. Verdict
The one non-thin shim: each input takes TWO passes — the generic M3U harness
`playlists::fuzz::fuzz_playlist_m3u` (exists, crates/playlists/src/fuzz.rs:35) and the
HLS-specific stack `netfuzz::fuzz_hls_playlist` (exists, crates/streaming/tests/netfuzz/
fuzz.rs:82, pulled in via #[path] so no src/ edit was needed). The HLS consumer half is
baseline-proven by `netfuzz_sweep_hls_playlist_survives ... ok`
(cargo-test-workspace.txt:5714; the 60s variants are #[ignore]d long-run forms, :5711-5712).

## 2. What actually works (verified)
- Both harness fns exist at the named paths (rg).
- Baseline: netfuzz_sweep binary ran with 2 survivors green + 2 documented long-run
  ignores (cites above).
- The #[path] include of a tests-dir module is a real ownership-boundary device (:14-17),
  not a dead include — the module's fn is called on line 40.

## 3. Findings
F-001 [S3]
Where: fuzz/README.md:17-42 vs .github/workflows/audit.yml:325-351; docs/audit/reports/fuzz/ absent
What is wrong: same stale "SCAFFOLDED, NOT WIRED" status while the nightly fuzz tier
exists, and no committed campaign evidence — identical to the adts/av1 targets' finding.
Confidence: HIGH — Verification: read + ls.
F-002 (observation, S4-level, recorded not counted): the committed corpus
fuzz/corpus/fuzz_hls_playlist/ holds 5 .m3u8 seeds (listed this run) — real HLS playlists
per the header (:25-27), but per the corpus audits the seeds are never executed by any
committed run.

## 4. Test honesty
No in-file tests; equivalence evidence = netfuzz_sweep survivors (green) + registry gate
(full_gate.txt:57).

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One call = TWO passes (both panic-free by construction)" | :5-6 | REAL (wiring) | both calls present :38-41 |
| "harness logic is proven by … crates/streaming/tests/netfuzz_sweep.rs" | :19-23 | REAL | baseline :5714 ok |

## 6. Dead / duplicated / unreachable code
None — both passes are live calls.

## 7. Dependencies & flags
libfuzzer_sys + #[path] cross-crate module include (tests-dir, ownership-honest).

## 8. Open questions for the next agent
None beyond F-001.
