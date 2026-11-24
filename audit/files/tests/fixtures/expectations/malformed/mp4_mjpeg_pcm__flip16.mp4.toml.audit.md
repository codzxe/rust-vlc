# AUDIT: tests/fixtures/expectations/malformed/mp4_mjpeg_pcm__flip16.mp4.toml
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Lines / functions | 29 / 1 doc + 4 tables |
| File verdict | REAL (machine-generated expectation, verified against bytes) |
| Reachable? | NO (fixture metadata); consumers: demux/testkit expectation loaders |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |
| Commands run | Read; python3 sha256+size of the referenced fixture |

## 1. Verdict
Expectation record for the MP4 flip16 derivative. Verified against bytes:
sha256 63a746dead5ccbe0… (full digest match), size_bytes 424,452 match,
op bit-flip with flip_bits = 16 within flip_extent = 512 (header extent) —
in-extent mutation, the stronger class. The derivation note documents the
shuffle-prefix construction ("flip-k positions are a k-prefix of one
shuffle"), which makes flip1/4/16 a nested family from one RNG stream.
[error] contract identical union shape as the garbage0 record.
## 2. Verified
— python3: fixture sha256 + size match the toml exactly.
— Consumers: crates/demux/tests/fixtures_class_b.rs (flip family at :478),
crates/codecs/tests/fuzz_survival.rs:33 (flip16 in the survival list).
— Generator: tools/xtask/src/cmd/fixtures.rs flip_derivative
(`flip-k positions are a k-prefix of one shuffle` at :3234 region).
## 3. Findings
None.
