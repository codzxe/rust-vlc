# WAVE 18 SUMMARY — claim ledger A: matrix rows 1-126 + inventory

| Field | Value |
|---|---|
| Agents | A01-A10 all complete |
| Files audited | 32 (126 matrix rows + 9 inventory docs + raw tooling) |
| Findings | S0:6+ S2:10+ S3:~25 S4:~30 |
| Inventory provenance | HOLDS: parse scripts re-run -> byte-identical JSON regeneration (both), all claimed counts reproduced exactly |
| Tree purity | clean |

## Matrix verdict landscape (rows 1-126, headline facts for A07 scoring)
REAL (executed or execution-backed): rows 001 mkv, 002 mp4-open+decode
(seek FALSE), 003 ogg, 005 ps, 006 avi, 008 wav, 010 aiff, 013 mp3,
015/016 annexb parsers, 017 ivf, 023 h264-sw, 026 av1, 032 mjpeg, 038 pcm,
039 mp3-family, 040-046 audio codecs (mp2/aac/flac/alac/vorbis/opus/adpcm),
102/104 play+rate, 114 format-detection.
FALSE: 002-mp4-seek-notes, 004 ts (panics on valid 188B), 007 asf
(impl claimed, unreachable), 105 frame-step (partial claimed, unserved),
108 chapter-title, 109 bookmarks (mock only), 110 snapshot, 115 volume,
116 equalizer, 084 file (UNDERSTATED: shipped+executed but row says
not_started), 095 http-client (UNDERSTATED: shipped, row says
not_started), 099/100/101 reconnect/auth/remote-playlists (UNDERSTATED:
~4900 lines of landed tested code marked not_started), 103 seek.
UNREACHABLE (real code, no shipped consumer): 057/059/064/065 srt/vtt/
pgs/dvb subtitles, 058 ass, 067/068 cc/vobsub, 072 encoding-detection,
074 rasterizer, 077/078 m3u/pls playlists, 022 h264-hw, 129 colors-adjust.
SHALLOW: 024 hevc, 097 hls, 098 dash, 106 track-switch, 107 av-sync.
HONEST-absent/blocked: the rest (speex/musepack/midi, realmedia, zip,
teletext, kate, truehd/dts/ac3/wma/wavpack, ftp, mmsh, census tails).

## Key findings
1. [S0 x6] The matrix UNDERSTATES as well as overstates: rows 099/100/
   101 mark ~4,900 lines of landed, tested code (reconnect/auth/
   remote-playlists) as not_started; row 095 contradicts row 096 in the
   same file (http-client "not_started" vs the shipped ureq lane).
2. [S2] The matrix's own validator would catch none of this: R4 checks
   path existence only (Wave 1), statuses drift in both directions, and
   the refresh tooling is admitted unassigned debt.
3. [S2] Notes' machine-written triage misdiagnoses (rows 041/044) cite
   the canned phrase "no decoder in this build" (report.rs:1199) which
   collapses codec-gate refusals (aac.channel_count, alac.config.missing).
4. [S3] Evidence anchors drifted (rows 092/093: line-cites 173/191 vs
   current 212); probe-work raw logs (~120 refs) lost to .gitignore
   *.log; "61 logs + status.txt" cited by security-phase0.md absent.
5. [KEY] Inventory data itself is bulletproof: every number in
   options/modules/formats inventories re-derived exactly by agents;
   the rot is in the STATUS layer, not the census layer.
