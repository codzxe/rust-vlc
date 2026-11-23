# AUDIT: crates/playlists/tests/fixtures/dash/ffmpeg_av_list.mpd
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2025-06-14 |
| Type / size | `file`: XML 1.0 document, ASCII text — 2125 bytes, 41 lines |
| Verdict | REAL fixture (provenance worklog-recorded; structural pins verified by me) |
| Cited as evidence? | YES — feature-matrix.toml:1453 (`streaming-dash` row evidence → `crates/playlists/src/mpd.rs`, which `include_str!`s this fixture at :2235); docs/worklog/archive-2026-10.md:10869 records the exact generation command |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

Content verified: MPEG-DASH MPD, isoff-live profile, static, `mediaPresentationDuration="PT6.0S"`, one Period, 2 AdaptationSets — video (2 Representations: avc1.42c014 @ 500000/320x240, avc1.42c00d @ 200000/160x120) + audio (mp4a.40.2 @ 64000, 44100 Hz, 1 channel). SegmentList shape with `init-stream{0,1,2}.m4s` Initialization + `chunk-stream*-%05d.m4s` SegmentURLs — exactly ffmpeg's dash muxer naming. The audio representation carries 2 SegmentURLs (:36), matching the worklog's "audio carries 2 SegmentURLs" note verbatim.
Consumers (grep-verified): `mpd.rs:2235` (`ffmpeg_list_fixture_types` — pins Static/1 period/2 sets/timescale 1_000_000/duration 5_000_000/startNumber 1/init+chunk URLs), `streaming/src/dash/fuzz.rs:136,160` (mutation-sweep seed), `streaming/src/dash/resolve.rs:1052` (resolver test).
Provenance: worklog says "same generation with -use_template 0 -window_size 0" from the ffmpeg 7.1.5 run that produced ffmpeg_av_template.mpd. I cannot re-run ffmpeg to prove byte provenance (UNVERIFIED: ffmpeg not run), but structure, naming convention, and the 2-SegmentURL detail all match the recorded command. No fabrication indicators.
Commands run: `file` (XML), `stat` (2125 B), full read, grep for consumers.
