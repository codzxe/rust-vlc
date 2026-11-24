# DATA AUDIT: docs/perf/raw/parity-v4-ffmpeg-md5-20261024.tsv (credit: W19-A04, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Rows | 9 (dirty/0029, dirty/dup-range, media/0021, 0039, 0056, 0063, 0066, 0074, 0076) |
| Method (documented upstream) | parity-v4-20261024.md:107-108: `ffmpeg -i FILE -map 0:a:0 -c:a copy -f md5 -` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## Verdict
The ffmpeg-side digest half of the parity pair (with my file-25 audit of the lumen JSON):
- 8 of 9 hashes are byte-identical to the lumen digests (0029/0021/0039/0056/0063/0066/
  0074/0076 — verified row-by-row against parity-v4-lumen-digests-20261024.json).
- The 1 divergence (row 2: dup-range `8f3359d321deec5e75a51777aa476644` vs lumen
  `26b5fc050114c2a86c2276da02b778b8`) is DISCLOSED and attributed in
  parity-v4-20261024.md:127 ("MISMATCH — the −2 packets of §4 are real payload loss:
  2 077 056 − 2 072 532 = 4 524 B ... Damage-residue defect signal, not hidden").
- The `MD5=` prefix matches ffmpeg's md5 muxer output grammar; the doc's method line
  (§5, :107-108) names the exact command, and the comparison logic (copied packets vs
  decoded PCM being identical bytes for PCM) is sound.
S4-1: no provenance in the TSV itself — no command, no date, no tool version (the method
lives only in the derived doc). S4-2: the file is uncited by filename anywhere (rg no
matches) — an orphan whose values were fully absorbed into the parity doc's §5 table;
also ragged space-padding, not true TSV alignment. Not re-runnable (corpus off-repo).
Quotes byte-verbatim from Read output.
