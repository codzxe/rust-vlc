# AUDIT: tools/vta/dnx/evidence/ffprobe-facts.txt (data audit)
| Field | Value |
|---|---|
| Auditor | W17-A02 |
| Date | 2026-11-24 |
| Lines / blocks | 45 / 5 lane blocks (L1-L5) |
| File verdict | REAL — L1 block REPRODUCED byte-for-field by me |
| Reachable from shipped entry point? | N/A (evidence data for tools/vta/dnx, a standalone non-workspace admission spike; not shipped) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `sed -n '1,9p'` (quote verify); `ffmpeg -f lavfi -i testsrc2=size=1920x1080:rate=24 -frames:v 3 -c:v dnxhd -b:v 175M -pix_fmt yuv422p -f dnxhd <audit/runs/tmp>` + `ffprobe -show_entries ...` -> output identical to committed L1 block; tmp removed |

## 1. One-paragraph verdict
This is real recorded ffprobe output, and I reproduced it: running the exact
L1 command from tools/vta/dnx/README.md (ffmpeg 7.1.5, same version the repo
claims) produced a lane whose ffprobe facts match the committed L1 block
field-for-field, including the initially-suspicious `r_frame_rate=25/1`
despite the README generating at `rate=24` — the dnxhd encoder writes the
25/1 rate code, so the committed commands and evidence are consistent after
execution. The file's weaknesses: the bitrate labels in the lane names
(175M/36M/145M/90M/185M) are NOT evidenced anywhere (bit_rate=N/A, and my
run shows -b:v 175M actually lands on the 917504-byte/frame DNxHD row), and
the file carries zero provenance header (no command, date, or tool version —
unlike its sibling lane-md5s.txt which does).

## 2. What actually works (verified)
- Reproduction run (exit 0): ffmpeg L1 command -> ffprobe output:
```
codec_name=dnxhd
profile=DNXHD
width=1920
height=1080
pix_fmt=yuv422p
r_frame_rate=25/1
bit_rate=N/A
nb_frames=N/A
```
  Identical to committed block (tools/vta/dnx/evidence/ffprobe-facts.txt:1-9).
  Artifact was written under audit/runs/ then deleted; nothing outside audit/
  was touched.
- Internal consistency across blocks: L1-L3/L5 all 1920x1080, L4 1280x720;
  only L5 carries pix_fmt=yuv422p10le — matches README lane list.

## 3. Findings
F-001 [S4] — Evidence does not evidence the lane names
Where: tools/vta/dnx/evidence/ffprobe-facts.txt:8-9 (all blocks)
Code:
```
bit_rate=N/A
nb_frames=N/A
```
What is wrong: every lane is named for its bitrate (`175M`, `36M`, `145M`,
`90M`, `185M`) but the file records `bit_rate=N/A` for all — nothing
committed proves the numbers the lanes are named after. My reproduction
shows `-b:v 175M` quantizes to the DNxHD 917504-byte/frame row anyway
(2752512 bytes / 3 frames = 917504), i.e. the name records the requested
flag, not the achieved bitrate. Honest N/A, weak evidence. What correct
looks like: a `filesize=` fact per lane (provable) in the file.
Confidence: HIGH — Verification: ran (my own encode+probe; byte count).
F-002 [S4] — No provenance header
Where: file starts directly at `=== L1_1080p_175M.dnxhd ===` (line 1)
What is wrong: no date, no ffmpeg/ffprobe version, no command line in the
file; sibling lane-md5s.txt does carry one ("# per-lane whole-stream MD5s
(ref vs mine) — 2026-10-26, ffmpeg 7.1.5 vs filmcraft-dnx 0.9.0").
Provenance for this file must be reconstructed from README/worklog.
Confidence: HIGH — Verification: read both files.

## 4. Chunk coverage
Single 45-line file, read in full. Blocks L1 (1-9), L2 (10-18), L3 (19-27),
L4 (28-36), L5 (37-45) — all five compared against README lane list:
consistent (dims/pix_fmt match; L5 10-bit as documented).

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ffprobe facts committed as evidence" | docs/worklog/archive-2026-10.md:10277 | REAL | this file; L1 block reproduced by me |
| "per-lane codec/profile/pix_fmt/dims/rate facts" | tools/vta/dnx/README.md:51 | REAL | file contains exactly those fields |
| README generation commands reproduce the lanes (rate=24) | README:18-27 | REAL — but rate normalizes to 25/1 | my run: rate=24 input -> r_frame_rate=25/1 (encoder rate code) |

## 6. Dead / duplicated / unreachable code
N/A (data). Note: fixtures (.dnxhd) and raw outputs are NOT committed —
reproducibility rests on ffmpeg 7.1.5 + the README commands (verified for L1).

## 7. Dependencies & flags
ffmpeg/ffprobe 7.1.5 (present in this sandbox — same version). tools/vta/dnx
is standalone ([workspace] of its own — NOT in the root workspace members;
see my tools/vta/mpeg4/Cargo.toml audit next).

## 8. Open questions for the next agent
- compare-results.txt shows filmcraft-dnx is NOT bit-exact vs ffmpeg
  (L1: maxDelta=2, ~1.3% differing pixels) — admission used a tolerance,
  not bit-exactness; anyone claiming "dnxhd bit-exact" is inflating (only
  rusty_vp9 was recorded as bit-exact).
- Lane L3 uses mandelbrot source, L1/L4/L5 testsrc2 — the md5 differences
  between ref/mine in lane-md5s.txt are packing-level, resolved by
  compare.py normalization; that script belongs to another agent's lane.
