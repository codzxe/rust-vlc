# AUDIT: tests/fixtures/expectations/malformed/avi_mjpeg_pcm__negheight0.avi.toml
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture/evidence data; consumed by the tests named below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256/size/prefix/byte-diff batch; cat/od for content |

Data audit: 28-line TOML, sha256 e5e5941d8200f350…, schema expectation/1. Verified against the fixture bytes: names sha256 edae3802… / size 425,516 — matches the fixture file (byte-audit family: negheight0 same-size sibling of the parent). [derivation] records op negate-biheight, patched_offset 180, patched_value 4,294,967,056 (=-240 two's complement, i.e. biHeight -240 bottom-up DIB) — the recorded patch is byte-derivable from the parent's +240 at the strf offset (verified offset/soundness by family: parent 425,516 B same size). [error] union contract typed-error-or-bounded-recovery + never-terms (panic/hang/unbounded) + rss_cap 64 MB + wall cap 5 s. Consumers: recovery_matrix.rs (negheight0 row: clean Ended, V1 height-abs acceptance), demux fixtures_class_b, xtask fixtures --check (ledger byte-verification).
