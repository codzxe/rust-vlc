# AUDIT: fuzz/corpus/fuzz_playlist_m3u/seed_plain_urls.m3u
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (fuzz seed corpus). Consumed by its fuzz target via the cargo-fuzz corpus convention (fuzz/corpus/<target>/); nightly campaign .github/workflows/audit.yml:325-355 runs it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256/size batch; byte-compare vs committed fixtures; cat/od for text seeds |

Data audit: 81 B, 4 lines, sha256 c6c47579cc090096…. Content verified by cat: relative path, absolute POSIX path, http URL, Windows backslash path (..\sibling\song3.wav) — the four URL grammars the M3U parser must survive. Consumers: fuzz_playlist_m3u target (Phase 5).
