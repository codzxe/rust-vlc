# AUDIT: tests/fixtures/generated/malformed/mp4_mjpeg_pcm__flip1.mp4
| Field | Value |
|---|---|
| Auditor | W16-A05 |
| Date | 2026-11-24 |
| Lines / functions | 424452 bytes / n-a (binary fixture; assignment listed 25 #part chunks of one file) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (test-only asset; consumed at crates/media-probe/tests/fixtures_matrix.rs:445 and tests/integration/first-vertical/tests/recovery_matrix.rs:780) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 batch -> audit/runs/W16-A05.datahashes.tsv; rg consumers; file -b |

Data audit. MP4 (ISO 14496-12) 424452 bytes, sha256 be1715889efedce8… (full hash in audit/runs/W16-A05.datahashes.tsv row 1) — byte-identical to the hash pinned in three committed registries: tests/fixtures/generated/MANIFEST.toml:198 (op = "flip1"), tests/fixtures/expectations/malformed/mp4_mjpeg_pcm__flip1.mp4.toml:11 (sha256 = "be1715889efedce83fe58ace62da502e5724552cd083f1d8241a19b123c71c45", size 424_452) and tests/fixtures/CORPUS.md:98. Generated, not handmade: deterministic seeded 1-bit header flip (flip_extent = 512) of valid parent generated/valid/mp4_mjpeg_pcm.mp4 via `cargo xtask fixtures` per expectation toml `generator` field; license CC0.

Consumers (rg, all verified as files that exist): crates/media-probe/tests/fixtures_matrix.rs, crates/codecs/tests/fuzz_survival.rs, tests/integration/first-vertical/tests/recovery_matrix.rs. Executed evidence: audit/runs/baseline/cargo-test-workspace.txt:3670,5075 (`Running tests/fixtures_matrix.rs`) and :2373-2374 (`malformed_mp4_fixtures_only_ever_refuse_or_demux_typed ... ok`); committed probe outcome docs/corpus/compat-matrix-20261110.raw-rows.jsonl:37 — `{"rel":"tests/fixtures/generated/malformed/mp4_mjpeg_pcm__flip1.mp4",...,"outcome":{"status":"refused_demux","message":"corrupt structure at byte 1291 (demux.mp4.box-overruns-parent)"}}`. The fixture genuinely exercises a malformed-input path and has committed typed-error evidence. No findings. Nothing suspicious.
