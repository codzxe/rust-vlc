# AUDIT: tests/fixtures/generated/malformed/mp4_mjpeg_pcm__trunc4.mp4
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 424,451 B / 9ebf78e460a062979c1d3c9c0a03107605af403920df881d468643f553802e36 |
| File verdict | REAL (data fixture; derivation independently re-executed) |
| Reachable from shipped entry point? | N/A (test data; consumed by test binaries below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

Binary MP4 fixture (assignment rows 2971-2995, chunked #part1-25 -> this one audit;
orchestrator chunk line-ranges (L1-15576) do not match the file's real newline count
(2385 by `wc -l`) — chunk indices are synthetic, whole file audited instead).

What it is: MP4 (ftyp isom), class-B malformed corpus entry = byte-exact truncation
of tests/fixtures/generated/valid/mp4_mjpeg_pcm.mp4 (424,452 B) with the final byte
removed. Verified by python: `trunc4 == valid[:424451]` -> True; valid tail ends
`...f6df90ffdb2fc`, trunc4 tail ends `...f6df90ffdb2` (1 byte short). sha256 matches
both tests/fixtures/CORPUS.md:96 and tests/fixtures/expectations/malformed/
mp4_mjpeg_pcm__trunc4.mp4.toml:11 verbatim:
`sha256 = "9ebf78e460a062979c1d3c9c0a03107605af403920df881d468643f553802e36"`.

Generated-vs-handmade: generated (`generator = "cargo xtask fixtures"`, seed_root
lumen-2a3-v1, per expectation toml lines 9-10); derivation `[derivation] op =
"truncate", cut_offset = 424_451, note = "cut the final byte"` — independently
re-executed and byte-exact. Consumers (rg): crates/codecs/tests/fuzz_survival.rs:40,
tests/integration/first-vertical/tests/recovery_matrix.rs:753,
tests/fixtures/CORPUS.md:96, tests/fixtures/generated/MANIFEST.toml:182.
Matrix-evidence: docs/corpus/compat-matrix-20261110.raw-rows.jsonl cites this name.

Honesty: the mutation is real and minimal (1-byte truncation at EOF); whether the
consuming tests actually exercise it is judged in those tests' audits (recovery_
matrix.rs and fuzz_survival.rs are both in this wave's assignments).
