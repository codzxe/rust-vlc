# AUDIT: fuzz/corpus/fuzz_playlist_pls/seed_dirty.pls
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 301 B / ef5c90d3dcf91b45d0eb8bed0a84d6adb4c30a67782402058df19200d7c92d9d |
| File verdict | REAL (as data); the corpus copy itself is never read |
| Reachable from shipped entry point? | NO — fuzz corpus seed; byte-twin executed via include_str! |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. Verdict (corpus-seed audit)
A genuinely hostile PLS seed, exactly what a playlist fuzzer should eat: zero-index `File0`, bad-suffix `Filex`, non-numeric `Length1`, orphan `Title1`, duplicate `File2`, empty `File3=`, a garbage line without `=`, entries under a foreign `[other-section]` section, a second `[playlist]` header, and `NumberOfEntries=99` (a count lie) — full content verified in batchmeta.txt. It is sha256-identical to `crates/playlists/tests/fixtures/dirty.pls`, loaded by `const DIRTY: &str = include_str!("fixtures/dirty.pls");` (crates/playlists/tests/pls_suite.rs:19) and round-trip-tested.

## 2. What was checked and how
- batch.py: 301 B, 18 lines, full content in batchmeta.txt.
- sha256 equality with the playlists twin (ef5c90d3… both, verified).
- Twin consumers: pls_suite.rs:19 (DIRTY include_str) — pls suite passes in baseline (audit/runs/baseline/cargo-test-workspace.txt, playlists suites ok).
- Registry gate (existence) run by me: exit 0.

## 3. Findings
F-001 [S3] Corpus-lane gap (shared): no stable gate reads fuzz/corpus/fuzz_playlist_pls/*; the campaign that would is unwired. The twin's include_str! coverage makes the fuzz material real. Verification: ran (sha256 + rg).
