# AUDIT: fuzz/corpus/fuzz_hls_playlist/single.m3u8
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (fuzz seed corpus). Consumed by its fuzz target via the cargo-fuzz corpus convention (fuzz/corpus/<target>/); nightly campaign .github/workflows/audit.yml:325-355 runs it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256/size batch; byte-compare vs committed fixtures; cat/od for text seeds |

Data audit: 184 B, 4 lines, sha256 7737c7754d9528e9…. Content verified by od: #EXTM3U + #EXT-X-MEDIA closed-captions group (GROUP-ID="cc", INSTREAM-ID="CC1") + #EXT-X-STREAM-INF with BANDWIDTH/CODECS(=avc1.64001f,mp4a.40.2)/RESOLUTION + child playlist URI. A well-formed single-variant HLS master with the captions track — the happy-path grammar seed. Consumers: fuzz_hls_playlist target (issue #14 M2).
