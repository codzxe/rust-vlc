# DATA AUDIT: docs/perf/raw/parity-v4-smoke-audio-pkts-20261024.log (credit: W19-A04, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Lines / entries | 35 / 26 `nb_read_packets` rows across 9 files |
| Cited by any doc? | NO — `rg parity-v4-smoke-audio-pkts docs/` -> no matches. Orphaned raw evidence. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |

## Verdict
Raw per-file audio-packet probe dump. Zero provenance: no tool name, no version, no command
line, no date header inside the file (rg for `ffprobe|ffmpeg` -> NO-TOOLNAME-IN-FILE); the
ffprobe-style key=value format (`index=1|codec_name=pcm_s16le|nb_read_packets=6`) is the only
hint of origin. UNVERIFIED: which binary produced it. Cross-checks vs
docs/perf/raw/parity-v4-lumen-digests-20261024.json (independent lumen-side digests):
- `0029-riff-size-lie.avi` log `nb_read_packets=6` == lumen json `"packets": 6` — MATCH.
- `0076-maxtracks-avi.avi` log `index=9 ... nb_read_packets=8` == lumen json `"track": 9, "packets": 8` — MATCH.
- `0039-avi-av.avi` log `nb_read_packets=41377` vs lumen json `"packets": 41` — 1000x-style
  granularity mismatch (PCM block vs aggregate counting). docs/perf/parity-v4-20261024.md:120
  claims byte-exact md5 for this file, so bytes agree while packet counts do not; the log never
  explains its own counting unit (S3).
S3-2: `media/0074-hevc-in-avi.avi` yields only `index=1 pcm_u8 nb_read_packets=7`; the HEVC
video stream is absent from this "audio-pkts" log without a stated filter — a reader cannot
tell filtering from failure. S4: no header/footer totals; 26 rows are unverifiable by re-run
(corpus media not in repo; corpus-v4-smoke-manifest-20261023.json:6770 lists the file, media
itself lives off-repo at /home/z/my-project/worktrees/w5-16/corpus-v4 — UNVERIFIED re-run).
Quotes verified: sed -n '1,2p;7,8p;17,18p' (above output is byte-verbatim).
