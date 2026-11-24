# AUDIT: fuzz/corpus/fuzz_demux_es/seed_raw_aac.aac
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (fuzz seed corpus). Consumed by its fuzz target via the cargo-fuzz corpus convention (fuzz/corpus/<target>/); nightly campaign .github/workflows/audit.yml:325-355 runs it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256/size batch; byte-compare vs committed fixtures; cat/od for text seeds |

Data audit: 2,500 B, sha256 1027bc9b5bd43503…, ADTS AAC frame head (fff1 sync, profile/flags consistent with ADTS LC stereo). Raw elementary-stream seed for the ES demux target; siblings seed_raw_mp3.mp3 + seed_crossformat_mkv.mkv (ES demuxer must classify the codec, not the container). Consumers: fuzz_demux_es target via corpus convention.
