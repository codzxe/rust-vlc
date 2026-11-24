# AUDIT: tests/fixtures/expectations/malformed/mp4_mjpeg_pcm__garbage0.mp4.toml
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Lines / functions | 28 / 1 doc + 4 tables |
| File verdict | REAL (machine-generated expectation, verified against bytes) |
| Reachable? | NO (fixture metadata); consumers: demux/testkit expectation loaders |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |
| Commands run | Read; python3 sha256+size of the referenced fixture |

## 1. Verdict
Expectation record for the MP4 garbage0 derivative. Every verifiable field
is TRUE against the bytes on disk: sha256 d7636fc61dad3847… (full digest
matches), size_bytes 428,548 matches, parent
generated/valid/mp4_mjpeg_pcm.mp4, op garbage-append with appended_bytes
4,096. The [error] contract is a union ("typed-error-or-bounded-recovery",
never panic/hang/unbounded-alloc, rss 64 MB / wall 5 s caps) — honest about
being v0-union-level rather than a specific outcome.
## 2. Verified
— python3: fixture sha256 and size match the toml exactly.
— Generator tag "cargo xtask fixtures" matches the committed generator
(tools/xtask/src/cmd/fixtures.rs garbage_derivative, :2572-2586).
— Consumers: crates/demux/tests/fixtures_class_b.rs:319
`derivative_expectation("garbage0")` + crates/codecs/tests/fuzz_survival.rs:34.
## 3. Findings
None. (The garbage0-mutation weakness finding lives in the fixture audits.)
