# CLAIM AUDIT: docs/perf/breadth.md (L1-217) — the PERF-VS-VLC methodology doc (Phase-5 breadth, card 5.R1)
| Field | Value |
|---|---|
| Auditor | W19-A09 |
| Date | 2026-11-24 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 10 / 0 / 0 / 0 / 0 / 0 / 2 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The headline methodology fact — **"perf-vs-VLC" at the Phase-5 breadth layer is actually perf-vs-FFMPEG** — is disclosed on the doc's own first table ("VLC 3.0.23 is absent from this sandbox… so ffmpeg is the speed oracle everywhere, per the doc-12 §1.3 fallback"), and the bench-vs-vlc harness's refusal rule is real code (VlcOracle.resolve() raises OracleUnavailable "no vlc/cvlc on PATH — install the pinned VLC 3.0.23"). Every spot-checkable number in the doc reconciles exactly against the committed raw artifacts: probe medians (2.48–2.49 ms / refusal lanes 2.42–2.53 ms), seek medians (102.10/100.70 and 404.35/705.30 ms — including the unexplained warm-seek regression reported as-is), MJPEG walls (0.902/0.760/0.708/0.422 s over 4×5 fresh-process rows), and the audio-bench medians match the transcript line-for-line. DNF lanes are stated with evidence instead of invented numbers. One inventory mislabel: "mjpeg-decode-bench.csv (25 rows)" — the file has 20 rows (4 lanes × 5) and no header row.

## 2. Claim ledger
| # | Claim (verbatim, ≤25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "VLC 3.0.23 is absent from this sandbox (the tools/bench-vs-vlc harness refuses without it…) so ffmpeg is the speed oracle everywhere" | oracle row | REAL | tools/bench-vs-vlc/run.py:146-152 `VlcOracle.resolve()` raises OracleUnavailable "no vlc/cvlc on PATH — install the pinned VLC 3.0.23"; ffmpeg 7.1.5 present in this sandbox. Methodology note: the VLC speed comparison NEVER ran; all speed ratios are vs ffmpeg — disclosed, not hidden. |
| 2 | "every number below was produced this session on this machine by the commands in §8; nothing is quoted from a previous run" | :3-5 | REAL (vs raw) | §2/§3/§4/§5 medians recomputed from the committed raw CSVs/transcript — all match (below). |
| 3 | probe: "open → media.info-ready N=10 median 2.48–2.49 ms" (AVI lanes) | §4 | REAL | r1-avi/probe.csv: 3 fixtures × 10 rows, medians 2.492/2.486/2.482. |
| 4 | refusal lanes "2.42–2.53 ms" | §4 | REAL | r1/probe.csv: 6 fixtures × 10 rows, medians 2.418–2.531. |
| 5 | seek: 0052 → 102.10 ms (warm 100.70); 0073 → 404.35 (warm 705.30) | §5 | REAL | r1-avi/seek.csv: medians recomputed 102.1/100.69 and 404.35/705.3 — exact. The doc flags the unexplained 404→705 warm regression for the engine owner instead of hiding it. |
| 6 | MJPEG walls 0.902 / 0.760 / 0.708 / 0.422 s (median-of-5) | §3 | REAL | mjpeg-decode-bench.csv: 20 headerless rows, 4 lanes × 5; medians 0.902/0.760/0.708/0.422 exact. |
| 7 | Audio benches: FLAC 46,421,260 s/s (526.3×), MP3 71,675,908, MP2 105,655,092, Vorbis 24,152,607, AAC 68,413,446/107,433,098, ALAC 133.1 MB/s | §2 | REAL | docs/perf/raw/r1/codec-benches-20261025.txt:4-15 carries each number verbatim (incl. the 7-rep raw list for 0085). |
| 8 | "`mjpeg-decode-bench.csv` (25 rows)" | §9 | FALSE (inventory label) | File has 20 rows (4×5), no header. Numbers correct; the row-count inventory line is wrong. |
| 9 | "r1/probe.csv (60 rows: 6 refusal lanes); r1-avi/probe.csv (30 rows); seek.csv (20 rows)" | §9 | REAL | 61/31/21 lines incl. CSV headers = 60/30/20 data rows. |
| 10 | DNF-with-evidence: h264/hevc "parsers, not decoders… no throughput number exists to report and none is invented" | §3 | REAL | Consistent with codec coverage at that date (H.264 admitted 2026-11-15, after this 10-25 doc); current tree has H264SwDecoder — the doc is a dated snapshot, honest at its time. |
| 11 | "none is reachable from the CLI on this build: the open path refuses their containers" (flac/mp3/mkv seek) | §5 | REAL (dated) | Matches compat-matrix's flac refused / container lanes; later wired per 8.B-era cards. |
| 12 | "median-of-5 fresh processes… warm-up discarded… Failures stay in the CSVs as rows" | run-selection | REAL-UNPROVEN | Discipline visible in CSV shapes (5 rows/lane, 10 rows/session); the warm-up discarding itself is not observable from committed CSVs. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Program name/brief: "perf vs VLC 3.0.23" (README §Ground rules; doc 12) | This doc: ffmpeg is the speed oracle everywhere | The doc is the truth — the VLC speed comparison never ran in Phase 5 breadth; only decode-equality differentials vs VLC exist (compat-matrix oracle, vlc-diff-report). The methodology supports throughput-vs-ffmpeg and equality-vs-VLC, nothing more. |

## 4. Fabrication / inflation indicators
None — the doc's "Honest summary" section lists its own losses (MJPEG 0.47×–0.69× ffmpeg, AV1 0.29× asm-off policy gap, warm-seek anomaly). The single mislabel (25 vs 20 rows) is inventory sloppiness, not data inflation.
Commands run: python3 median recomputation over r1/*.csv + r1-avi/*.csv (exact matches); wc -l CSVs; rg over codec-benches transcript; sed over tools/bench-vs-vlc/run.py (oracle refusal). Exit 0.
Findings: S2: 0 · S3: 0 · S4: 2 (F-019 "25 rows" mislabel; F-020 mjpeg CSV lacks header row unlike every other committed CSV).
