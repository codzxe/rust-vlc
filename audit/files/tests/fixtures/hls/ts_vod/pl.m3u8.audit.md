# AUDIT: tests/fixtures/hls/ts_vod/pl.m3u8
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 223 bytes / a48a7b5892fc7af3429d0d38c583a0db305442993982c166479c349daa8af640 |
| Type | M3U playlist, ASCII |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | sha256 batch; cat; rg consumers; baseline check |

## 1. Verdict
VOD HLS playlist: VERSION 3, MEDIA-SEQUENCE 0, TARGETDURATION 3, four #EXTINF segments (seg000-seg003.ts, durations 2.508/2.508/2.485/0.523 ≈ 8s total, all present on disk), ENDLIST (VOD marker). Consumers: crates/streaming/tests/hls_vod_e2e.rs:168,193,225 (real HTTP server serves the lane; session open + on-disk byte comparison per segment); hls_conformance_e2e.rs also references the lane. Baseline: hls_vod_e2e.rs ran ok (audit/runs/baseline/cargo-test-workspace.txt:5670; the streaming package block reports all-ok).
## 2. What actually works (verified)
cat shows well-formed VOD playlist whose 4 referenced segments all exist in the same dir (ls: seg000-003.ts); consumers cite exact paths; baseline run cited.
