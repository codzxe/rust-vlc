# AUDIT: fuzz/corpus/fuzz_demux_mp4/seed_mp4_frag.mp4
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 14045 bytes |
| File verdict | REAL (fragmented MP4; byte-identical to the committed valid fixture) |
| Reachable from shipped entry point? | NO (fuzz corpus; campaign unwired) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 provenance byte-compare (exit 0); fuzz_registry_check.sh RAN -> CLEAN |

## 1. Verdict
Genuine fragmented MP4 (head `ftyp` size 28), 14045 bytes, sha256 530bad7f9faa3fe0...
(recomputed). VERIFIED byte-identical to tests/fixtures/generated/valid/mp4_frag.mp4 — the
README's "copied from the committed CC0 fixtures" provenance holds.

## 2. Consumers
fuzz/fuzz_targets/fuzz_demux_mp4.rs (registry-gated — gate RAN by me, CLEAN). The identical
bytes as a fixture pin fragmented-container truth in crates/demux/tests/mp4_fixtures.rs
(`mp4_frag_sidx`/`mp4_frag` rows: fragmented = true) and the corpus jsonl records outcomes.

## 3. Findings
None. Note: the source fixture is outside the MANIFEST.toml drift gate (see systemic gap),
but the seed's existence itself is fuzz-registry-gated.

## 4. Open questions
None.
