# AUDIT: tests/fixtures/generated/malformed/avi_mjpeg_pcm__trunc4.avi
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | binary, 425,515 B / n/a (assignment chunks #part1..#part25, L1-19364) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture; consumed by recovery_matrix.rs:747 `matrix_avi_trunc4…` + demux fixtures_class_b) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 prefix/cut diff; cat expectation TOML |

Data audit: class-B derivative = byte-exact prefix of parent `generated/valid/avi_mjpeg_pcm.avi` (425,516 B) minus exactly 1 byte ("cut the final byte" per TOML [derivation]; verified cut=1, prefix identity True). sha256 14e929413fc7ac97… matches expectation TOML (schema expectation/1, CC0, seed_root lumen-2a3-v1).
Consumers: recovery_matrix.rs row (clean `Ended` via 1-byte clamp, ≥1 non-fatal RecoveredDamage — the clamped tail; honest recovery pinning, not a vacuous assert), demux fixtures_class_b suite, compat-matrix raw-rows.jsonl:6-adjacent family. Zero findings: derivative is exactly as documented; no hand-editing detected (deterministic generator claim consistent with verified single-byte cut at the RIFF end).
