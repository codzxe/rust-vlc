# AUDIT: crates/playlists/tests/fixtures/dash/ffmpeg_av_template.mpd (text fixture)
| Field | Value |
|---|---|
| Auditor | W10-A10 | Date | 2026-11-23 |
| Type / size | `file` -> "XML 1.0 document, ASCII text"; 2288 bytes, 44 lines |
| File verdict | REAL fixture — genuine ffmpeg DASH manifest, self-consistent, consumed by 3 mpd.rs test sites |
| Reachable from shipped entry point? | Test-only (DASH sessions unwired from shipped entry points, same as HLS) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`, full read, duration arithmetic re-derived; grep of consumers (mpd.rs:2117/2515/2804) and provenance doc (mpd.rs:101-104) |

## 1. Blunt verdict
Real ffmpeg output, not a hand-written lookalike: isoff-live profile, static type, `mediaPresentationDuration="PT6.0S"`, two video Representations (avc1.42c014 @500k 320x240, avc1.42c00d @200k 160x120 — ffmpeg's template ladder), one audio (mp4a.40.2 @64k, 44100, mono via AudioChannelConfiguration), `SegmentTemplate` with `$RepresentationID$` and the printf tag `$Number%05d$`, `SegmentTimeline` `S` entries — exactly the `-use_template 1 -use_timeline 1` shapes mpd.rs:101-104 claims ("real ffmpeg 7.1.5 `-f dash -use_template 1 -use_timeline 1` output"). Duration math self-consistent: video 61440/10240 = 6.0 s; audio (220160+44440)/44100 = 6.0 s — both match the declared presentation duration.
## 2. Consumers & citations
- crates/playlists/src/mpd.rs:2117 (parse test), :2515, :2804 (fuzz-seed mutation sweep) via `include_str!`.
- docs/feature-matrix.toml:1453 (network-dash row) cites mpd.rs cards 6.C1a/6.C1b incl. "SegmentTemplate $Identifier$ substitution incl. printf format tags, SegmentTimeline t+n×d expansion" — this fixture is the evidence for exactly those features.
## 3. Verdict
REAL. No fabrication indicators; the manifest exercises the hard parts (printf width tags, two timescales, timeline continuation `S` without `t`).
