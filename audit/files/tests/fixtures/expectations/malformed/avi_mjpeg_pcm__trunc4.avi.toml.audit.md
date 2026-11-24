# AUDIT: tests/fixtures/expectations/malformed/avi_mjpeg_pcm__trunc4.avi.toml
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 | 923 B / 8b51962e571b6660… (audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (ledger row; derivation + hash independently verified) |
| Reachable from shipped entry point? | N/A (test data) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

`expectation/1` ledger row for the AVI final-byte truncation derivative.
Independent verification: fixture sha256 recomputed = 14e929413fc7ac971278398e
159bb15810076ac8caabf941cd388e4f2dbe2664 == toml `sha256` (verbatim);
python `trunc4 == valid[:425515]` -> True (425,516 -> 425,515, one byte cut) —
the toml `[derivation] op = "truncate", cut_offset = 425_515, note = "cut the
final byte"` is exact. Consumers: class-B suite + recovery_matrix.rs pinned
outcome for avi trunc4; drift-gated via CORPUS.md/MANIFEST `--check`.
Matrix-evidence: compat-matrix raw-rows entry for the AVI truncation family.
Zero defects; same union [error] contract (rss 64 MB / wall 5 s caps).
