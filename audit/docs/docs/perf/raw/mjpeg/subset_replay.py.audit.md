# DATA AUDIT: docs/perf/raw/mjpeg/subset_replay.py
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A04) |
| Type | Python 3 script (63 lines), data-regeneration tool |
| Executed? | YES — `python3 subset_replay.py docs/corpus/corpus-big-manifest-20261022.json /tmp/corpus-r6` -> "11 byte-exact, 6 sha-diff, 0 failed, 17 total", exit 0 (540s timeout, ffmpeg 7.1.5-0+deb13u1) |

## 1. Blunt verdict
The script does exactly what its docstring promises and I ran it end-to-end:
17 files selected (8 avi-video + 8 avi-av + the 0127 stage representative),
deterministic ffmpeg argv replay from the committed manifest, sha256 checked
per file, every mismatch printed (never hidden), non-zero exit only on
generation failure. Zero fabrication exposure — its noise claim (muxer
timestamp sha-diffs) reproduced live at 6/17.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "regenerate ONLY the corpus-big files the 7-b speed comparison needs" | :2-4 | REAL | executed: exactly 8+8+1 = 17 files, 364,041,956 bytes |
| 2 | "the speed protocol samples the first 8 clean files of classes avi-video + avi-av plus ... (0127)" | :8-9 | REAL | selection loop :30-32 reproduced identically offline; manifest yields 8/8/1 |
| 3 | "every diff is printed, never hidden" | :14-15 | REAL | run output: 6 `[OK sha-diff]` lines printed; summary line counts them |
| 4 | "W5-16 measured 37/70 files with sha diffs on full replays" | :13 | REAL | docs/worklog/archive-2026-10.md (byte-checked): "sha256 diffs 37/70 = EXACTLY the v3 tool's own run-to-run noise baseline (v3-vs-v3 also 37/70 — muxer timestamps + encoder threading" |
| 5 | default manifest arg `/home/z/my-project/worktrees/w7-b/...` | :23-24 | STALE-PATH | worktree absent in this sandbox; in-repo copy `docs/corpus/corpus-big-manifest-20261022.json` has identical schema (fields `argvs/sha256/class/rel/ffprobe_ok` all present) — script needs argv[1] override here, S4 nit |
| 6 | exit discipline (exit 1 only on generation failure) | :63 | REAL | run exited 0 with 6 sha-diffs (diffs are OK by design, failures are not) |

## 3. Contradictions
None. Cited by docs/perf/mjpeg-simd-20261024.md as the fixture regenerator
("sha256-checked; same corpus the 2026-10-22 speed baseline used").

## 4. Fabrication / inflation indicators
None — self-limiting docstring (subset exists because full replay is ~17 min >
10-min tool cap). Findings: S4 x1 (hardcoded absolute default path into a
vanished worktree).
