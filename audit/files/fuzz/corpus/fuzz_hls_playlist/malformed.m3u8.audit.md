# AUDIT: fuzz/corpus/fuzz_hls_playlist/malformed.m3u8
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (fuzz seed corpus). Consumed by its fuzz target via the cargo-fuzz corpus convention (fuzz/corpus/<target>/); nightly campaign .github/workflows/audit.yml:325-355 runs it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256/size batch; byte-compare vs committed fixtures; cat/od for text seeds |

Data audit: 340 B, 12 lines, sha256 356a8bc4b79c85db…. Content verified by cat: five distinct malformed shapes — #EXT-X-STREAM-INF missing BANDWIDTH, non-numeric BANDWIDTH=big, unterminated CODECS quote, trailing-comma attrs, #EXT-X-MEDIA with GROUP-ID+NAME but no TYPE. Each targets a distinct parse-refusal path (typed errors, not panics). Genuine hostile seed. Consumers: fuzz_hls_playlist target.
