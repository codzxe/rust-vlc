# AUDIT: crates/playlists/tests/fixtures/hls/master/malformed.m3u8 (fixture)
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-23 |
| Type / size | M3U playlist, ASCII text, 340 bytes |
| File verdict | REAL (deliberately malformed master playlist; consumers assert the tolerance behavior) |
| Reachable from shipped entry point? | N/A (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `file`, `wc -c`, full read, consumer scan (hls.rs:989, netfuzz_sweep.rs:126) |

## 1. Verdict
A dirty-master vector: one good variant row (`BANDWIDTH=1000000,RESOLUTION=640x360` → good.m3u8) followed by four damaged ones — missing BANDWIDTH (no-bw.m3u8), non-numeric BANDWIDTH=big (bad-bw.m3u8), unterminated CODECS quote (broken-quote.m3u8), trailing comma (trailing-comma.m3u8) — plus a dangling `#EXT-X-MEDIA` with no URI row. The consumer test (`hls.rs:988-995`) asserts exactly the honest-tolerance contract: 1 surviving variant, non-empty warnings. Also in the netfuzz_sweep corpus.

## 2. What actually works (verified)
— Read + consumer test cross-check: `parse_master_playlist` on this fixture must yield `variants.len() == 1` and warnings, or hls.rs:991/993 fail (test runs green in the playlists suite — 38/38 device-discovery is unrelated; the playlists lib binary 46/46 includes hls tests).

## 3. Findings
F-001 [S4]
Where: crates/playlists/tests/fixtures/hls/master/malformed.m3u8:12
Code:
```
#EXT-X-MEDIA:GROUP-ID="g",NAME="no-type-row"
```
What is wrong: the trailing EXT-X-MEDIA row has no TYPE attribute (required by RFC 8216 §4.3.4.1) and no following URI line — it exercises the dangling-media case, but no consumer assertion names it specifically (the hls.rs test only checks the aggregate: 1 variant + some warnings). The four damaged STREAM-INF rows likewise are not individually pinned — the test cannot distinguish "no-bw skipped" from "trailing-comma skipped". Adequate as a corpus seed (netfuzz), weak as a per-defect pin. Confidence: HIGH — Verification: read (hls.rs:989-995 asserts only count + non-empty).

## 4-9. —
Per-function table N/A. Test honesty: aggregate-level (see F-001). Doc claims: none beyond fixture content. Dead code: none. Dependencies: none. Open questions: if per-defect tolerance pins are wanted, split into one-row fixtures (suggestion, not a defect).
