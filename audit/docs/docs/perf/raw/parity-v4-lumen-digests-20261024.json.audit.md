# DATA AUDIT: docs/perf/raw/parity-v4-lumen-digests-20261024.json (credit: W19-A06, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Content | 9 lumen-side PCM digest rows (md5, pcm_bytes, frames/packets, codec facts, recovery counters) |
| Cross-verified against | parity-v4-ffmpeg-md5-20261024.tsv, parity-v4-20261024.md, audio-pkts log, tools/bench-vs-vlc |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## Verdict
The strongest evidence file of the batch: 8 of 9 MD5 rows are byte-exact against the
independent ffmpeg-side TSV (0021/0039/0056/0063/0066/0074/0076/0029 all match), and the
ONE mismatch (dup-range: lumen `26b5fc…02b778b8` vs ffmpeg `8f3359…766644`) is explicitly
disclosed downstream in parity-v4-20261024.md:127 with correct arithmetic: "2 077 056
(undamaged 0063 sibling) − 2 072 532 = 4 524 B dropped across 2 pcm_s24le packets" — my
recomputation: 2077056 − 2072532 = 4524 ✓, frames 338 vs 339 = −2 ✓, a_concealed=0
("today it skips, it does not conceal") ✓. Internal arithmetic holds per row (e.g. 0039:
1324064 B / (8ch x 4B f32) = 41252 samples = 3.74 s at 11025 Hz; 41 packets).
Provenance: schema `bench-vs-vlc/lumen-pcm-digests/1` — the emitter exists in-repo
(tools/bench-vs-vlc/src/lib.rs + tests/lumen_pcm.rs, rg verified). Cross-file consistency
with the audio-pkts ffprobe log: packets match exactly for 7 rows (0029=6, 0021=32,
0056=353, 0063=339, 0066=14, 0074=7, 0076 track9=8); dup-range 338-vs-340 and 0039
41-vs-41377 are both attributable (dirty salvage; PCM-block vs aggregate granularity —
the latter disclosed as "the §4 granularity dispute settles byte-exact here").
S4 only: no capture timestamp or tool version inside the JSON (filename + schema carry
it). Quotes byte-verbatim from Read output; md5 comparisons run row-by-row.
