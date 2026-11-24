# AUDIT: fuzz/corpus/fuzz_demux_mpd/ffmpeg_av_list.mpd
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 2,125 B / d2fa59db7f5ba7c78ef672471f315a1701f5cc396dd1f143de69332e6b97aac7 |
| File verdict | REAL (real ffmpeg-generated DASH manifest; byte-identical to a consumed test fixture) |
| Reachable from shipped entry point? | N/A (fuzz corpus + test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

What it is: DASH MPD (dash:profile:isoff-live:2011, static, PT6.0S), 2 video
representations (avc1.42c014 320x240 500k / avc1.42c00d 160x120 200k) + 1 audio
(mp4a.40.2, 44.1 kHz mono, 2 SegmentURLs), SegmentList/SegmentURL shape.
Provenance (docs/worklog/archive-2026-10.md:10869): REAL ffmpeg 7.1.5 output
(`-use_template 0 -window_size 0`, adaptation_sets "id=0,streams=0,1
id=1,streams=2"). Content verified by full read (above).

Consumers: (1) corpus convention for fuzz target fuzz_demux_mpd; (2) **byte-
identical copy (sha256 d2fa59db… matches, verified) at
crates/playlists/tests/fixtures/dash/ffmpeg_av_list.mpd** which is
include_str!'d by crates/playlists/src/mpd.rs:2235 (unit tests) and used by
crates/streaming/src/dash/fuzz.rs:136/160 and resolve.rs:1052 (fixture
"ffmpeg_av_list.mpd"). So unlike most corpus seeds, this one's shape is
actively test-consumed via its identical twin. S4-free: duplication is
byte-identical and both are exercised (or at least the twin is).
