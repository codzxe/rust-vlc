# AUDIT: tests/fixtures/generated/malformed/mp4_mjpeg_pcm__garbage0.mp4
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | binary, 428,548 B / n/a (assignment chunks #part1..#part25, L1-19498) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture; consumed by tests/integration/first-vertical/tests/recovery_matrix.rs:762 via testkit::fixture_bytes) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256/prefix-diff (parent vs derivative) -> matches; cat expectation TOML |

Data audit: class-B derivative = parent `generated/valid/mp4_mjpeg_pcm.mp4` (424,452 B) + 4,096 B seeded garbage append. Verified byte-exact prefix identity (`gv[:len(pv)]==pv -> True`), diff exactly 4096, tail sha256 5e9c059bd553c3cc…, all 256 byte values present. sha256 d7636fc61dad… matches expectation TOML `tests/fixtures/expectations/malformed/mp4_mjpeg_pcm__garbage0.mp4.toml` (schema expectation/1, license CC0, seed_root lumen-2a3-v1, size 428_548).
Consumers: recovery_matrix.rs `matrix_mp4_garbage0_clean_ended_zero_damage` (asserts CleanEnded + ZERO damage — honest: garbage past declared structure must be unparsed); compat-matrix raw row docs/corpus/compat-matrix-20261110.raw-rows.jsonl:40 (decoded, 60+60 packets, pcm_md5 08db7ccc…).
F-001 [S3]: expectation `[error] expected = "typed-error-or-bounded-recovery"` is a union contract, not the specific pinned flavor; the per-flavor pin lives in recovery_matrix.rs only (doc-comment admits "17 of 22 rows were unpinned" pre-G2). Fixture itself is sound. Confidence: HIGH — Verification: read.
