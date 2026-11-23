# AUDIT: crates/playlists/tests/fixtures/hls/master/single.m3u8 (fixture)
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-23 |
| Type / size | M3U playlist, ASCII text, 184 bytes |
| File verdict | REAL (minimal valid master playlist with a CLOSED-CAPTIONS media group) |
| Reachable from shipped entry point? | N/A (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`, `wc -c`, full read, consumer scan (hls.rs:982, netfuzz_sweep.rs:125) |

## 1. Verdict
A minimal well-formed master: one `#EXT-X-MEDIA` row (TYPE=CLOSED-CAPTIONS, GROUP-ID="cc", NAME="CEA-608", INSTREAM-ID="CC1" — RFC 8216 §4.3.4.1 closed-captions shape) and one `#EXT-X-STREAM-INF` variant (BANDWIDTH=550000, CODECS="avc1.4d401f,mp4a.40.2", RESOLUTION=854x480 → only.m3u8). Consumed by `hls.rs:982` (single-variant + media-group parse test, asserts `variants.len() == 1`) and the netfuzz_sweep corpus. No defects: attributes are legal, quoted, and complete.

## 2. What actually works (verified)
— Read + consumer test cross-check (`parse_master_playlist` must yield exactly 1 variant or hls.rs:986 fails; suite runs green).

## 3. Findings
None. Checked: tag spelling, attribute legality (TYPE/GROUP-ID/NAME/INSTREAM-ID for CLOSED-CAPTIONS per RFC 8216; INSTREAM-ID is mandatory for that TYPE and present), CODECS quoting, no trailing damage.

## 4-9. —
Per-function table N/A. Test honesty: consumer asserts count + (per hls.rs context) the group row is recorded. Doc claims: none beyond content. Dead code: none. Dependencies: none. Open questions: none.
