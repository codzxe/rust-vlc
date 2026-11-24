# AUDIT: tests/fixtures/generated/malformed/avi_mjpeg_pcm__absidx1.avi
| Field | Value |
|---|---|
| Auditor | W16-A05 |
| Date | 2026-11-24 |
| Lines / functions | 425516 bytes / n-a (binary fixture; assignment listed 25 #part chunks of one file) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (test-only asset; consumed at tests/integration/first-vertical/tests/recovery_matrix.rs:670 and crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs:131) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 batch -> audit/runs/W16-A05.datahashes.tsv; rg consumers; file -b |

Data audit. RIFF/AVI MJPEG+PCM 320x240 30fps, 425516 bytes, sha256 7910eed2d7a9649e… (full hash in audit/runs/W16-A05.datahashes.tsv row 2) — byte-identical to tests/fixtures/generated/MANIFEST.toml:146 (`sha256 = "7910eed2d7a9649ea1e78153e28df4fdb4ec23f5a8e7b29513645f991c2e2c1c"`, size 425_516) and tests/fixtures/CORPUS.md:91. Generated, not handmade: op `absidx1` rewrites every idx1 dwOffset as an absolute file position (generator path tools/xtask/src/cmd/fixtures.rs:2734-2737); regeneration is checked by the xtask fixtures self-check at tools/xtask/src/cmd/fixtures.rs:5113 (`find("generated/malformed/avi_mjpeg_pcm__absidx1.avi")`).

Consumers (rg, verified): recovery_matrix.rs:670 (test `matrix_avi_absidx1_clean_ended_full_payout` at :668) and playback-engine scenario_09_corrupt.rs:131 (`contained("generated/malformed/avi_mjpeg_pcm__absidx1.avi", 0, 118)`). Executed evidence: audit/runs/baseline/cargo-test-workspace.txt:3670,5075 (`Running tests/fixtures_matrix.rs`); committed decode outcome docs/corpus/compat-matrix-20261110.raw-rows.jsonl:3 — status "decoded", demux_recovered 1, pcm_md5 08db7ccc163dc478709627c1db9044b5. The fixture is a real spec-violation variant that both decoders and recovery tests consume; hash provenance is consistent across three registries. No findings.
