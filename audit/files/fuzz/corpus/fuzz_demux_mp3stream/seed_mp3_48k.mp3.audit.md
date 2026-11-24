# AUDIT: fuzz/corpus/fuzz_demux_mp3stream/seed_mp3_48k.mp3
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | data file / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (fuzz seed corpus). Consumed by its fuzz target via the cargo-fuzz corpus convention (fuzz/corpus/<target>/); nightly campaign .github/workflows/audit.yml:325-355 runs it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256/size batch; byte-compare vs committed fixtures; cat/od for text seeds |

Data audit: 12,564 B, sha256 45cadbb515ac40a9…, MP3 frame head (49443303 = 0xFFF3 sync + MPEG-1 Layer III no CRC). Standalone generated audio (no byte-identity with any committed fixture — the mp3 families are not mirrored under tests/fixtures/generated). Sibling seeds: seed_mp3_mpeg2_24k.mp3, seed_mp3_vbr.mp3 (layer/rate/VBR variety). Consumers: fuzz_demux_mp3stream target via corpus convention.
