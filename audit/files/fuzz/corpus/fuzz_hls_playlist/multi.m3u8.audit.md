# AUDIT: fuzz/corpus/fuzz_hls_playlist/multi.m3u8
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (fuzz seed corpus). Consumed by its fuzz target via the cargo-fuzz corpus convention (fuzz/corpus/<target>/); nightly campaign .github/workflows/audit.yml:325-355 runs it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256/size batch; byte-compare vs committed fixtures; cat/od for text seeds |

Data audit: 572 B, 9 lines, sha256 bc9eebcc774cc400…. Content verified by cat: audio + subtitles #EXT-X-MEDIA groups (en audio DEFAULT=YES AUTOSELECT=YES; sv subtitles) + 3-tier #EXT-X-STREAM-INF ladder (2 Mbit/800 kbit/200 kbit, AVERAGE-BANDWIDTH on two, CODECS+RESOLUTION each) + child URIs hi/mid/lo.m3u8. A complete multi-variant master. Consumers: fuzz_hls_playlist target.
