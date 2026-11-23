# AUDIT: crates/playlists/tests/fixtures/hls/ts_vod/pl3.ts (binary fixture)
| Field | Value |
|---|---|
| Auditor | W10-A10 | Date | 2026-11-23 |
| Type / size | `file` -> "MPEG transport stream data"; 7708 bytes = exactly 41 × 188-byte TS packets, 0x47 sync verified |
| File verdict | REAL fixture — real ffmpeg-captured TS segment (the successor of pl2.ts: identical 7708-byte size, same PAT/PMT structure, continuity counters advanced 0x12→0x13, audio PES packets `47 41 00 3x` visible) |
| Reachable from shipped entry point? | Test-only lane (as pl2.ts) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`, `od -A d -t x1`, `md5sum` (a2e68ccc…), `cmp` vs pl2.ts (differ at byte 4 — the continuity counter, as real consecutive segments should) |

## 1. Blunt verdict
Real TS media, not a copy of its sibling: same capture, advanced counters, distinct md5. Everything said in the pl2.ts audit applies — real capture, aligned packets, byte-exact e2e consumer (hls_vod_e2e.rs:165-193), accurate matrix provenance citation (docs/feature-matrix.toml:1167).
## 2. Verdict
REAL. No fabrication indicators.
