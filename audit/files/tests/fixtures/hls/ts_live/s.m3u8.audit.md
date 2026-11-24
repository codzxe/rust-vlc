# AUDIT: tests/fixtures/hls/ts_live/s.m3u8
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 211 B / 0ec986d5426e284e… (full digest in audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (ffmpeg-authored HLS media playlist) — but UNREACHABLE as test data (no consumer) |
| Reachable from shipped entry point? | N/A (test data) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

What it is: HLS VOD media playlist, EXTM3U v3, MEDIA-SEQUENCE 0, 4 segments
(s00..s03.ts, durations 2.507756/2.507756/2.484533/0.023178) + ENDLIST.
Provenance: ffmpeg segmenter output (crates/streaming/tests/hls_live_e2e.rs:23
"The committed `s.m3u8`/`t.m3u8` are ffmpeg's own output").

F-001 [S4] Where: tests/fixtures/hls/ts_live/ (directory) vs this playlist.
What is wrong: the playlist references `s03.ts` (the 0.023 s flush-tail segment),
but `s03.ts` is NOT committed — the tree has only s00.ts, s01.ts, s02.ts
(`ls tests/fixtures/hls/ts_live/` verified). hls_live_e2e.rs:27-29 says the
flush-tail segments "are dropped: the corpus is the three/two full segments" —
so the playlist's own fourth entry dangles. No test ever fetches it, so the
inconsistency is latent. What correct looks like: commit the 4-segment playlist
with s03.ts present, or trim s03 from the committed playlist (the test's live
windows already hand-author their playlists).

F-002 [S4] Where: whole repo scoped rg for `s.m3u8` consumers.
What is wrong: zero code references — only comments (hls_live_e2e.rs:17-23).
The file is a provenance artifact, not a consumed fixture. Blast radius: none;
the tests script `/hls/ts_live/pl.m3u8` themselves (hls_live_e2e.rs:126) and
serve only the .ts segments (line 120-122). Dead-by-design data with a dangling
entry — S4 hygiene.
