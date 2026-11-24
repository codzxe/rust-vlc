# AUDIT: tests/fixtures/expectations/malformed/mp4_mjpeg_pcm__flip1.mp4.toml
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 | 1,037 B / 30fb719f55969930… (audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (ledger row; derivation + hash independently verified) |
| Reachable from shipped entry point? | N/A (test data) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

`expectation/1` ledger row for the k=1 bit-flip derivative. Independent
verification: fixture sha256 recomputed = be1715889efedce83fe58ace62da502e5724
552cd083f1d8241a19b123c71c45 == toml `sha256` (verbatim); python diff vs parent
= exactly ONE flipped bit: byte 153, 0x00 -> 0x40 (bit 6), inside the declared
`flip_extent = 512` — `flip_bits = 1` is honest. Consumers: the class-B suite
(crates/demux/tests/fixtures_class_b.rs, recovery_matrix.rs pinned outcomes);
ledger drift-gated by `xtask fixtures --check` (CORPUS.md). Matrix-evidence:
compat-matrix rows for the flip family. The `[error]` contract
(`typed-error-or-bounded-recovery`, never panic/hang/unbounded-alloc, rss 64 MB,
wall 5 s) matches what recovery_matrix.rs:26 pins for mp4 flip1/flip4
("typed open refusal: `demux.corrupt` (`demux.mp4.box-overruns-parent`)").
