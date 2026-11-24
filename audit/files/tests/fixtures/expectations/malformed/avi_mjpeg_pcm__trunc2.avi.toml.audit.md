# AUDIT: tests/fixtures/expectations/malformed/avi_mjpeg_pcm__trunc2.avi.toml
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture/evidence data; consumed by the tests named below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256/size/prefix/byte-diff batch; cat/od for content |

Data audit: 27-line TOML, sha256 0a028564b1ec4609…, schema expectation/1. Verified: names fixture sha 9ec527a6… size 324 — matches the committed 324-byte trunc2 fixture (byte-exact prefix cut at the first movi chunk header per [derivation] cut_offset 324 = structure-map movi_data_offset; CORPUS.md row for trunc2 agrees). [error] union contract + never-terms identical shape. Consumers: recovery_matrix.rs trunc2 row (open accepted then FATAL demux.avi.truncated with >=1 non-fatal RecoveredDamage), demux fixtures_class_b, xtask --check.
