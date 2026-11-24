# CLAIM AUDIT: docs/perf/raw/probe.csv
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 3 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 2 / 1 / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A 60-row timing record (3 fixtures × 20 runs). Internally consistent:
media_info_ms ≥ response_ms in every populated row; the 20 empty
media_info_ms rows are all mp4_mjpeg_pcm.mp4 with outcome=typed-refusal
— the honest pre-MP4-demuxer era (matches the 2026-10-23 diff report's
"container mp4: no Lumen demuxer in this build"). The fixtures exist
in-tree.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | timings for avi_mjpeg_pcm.avi / avi_mjpeg_2pcm.avi (media-info ok) | rows 1-40 | REAL | python3: 40 ok rows, media_info ≥ response all; fixtures exist in tests/fixtures/generated/valid/ |
| 2 | mp4_mjpeg_pcm.mp4 → outcome typed-refusal, empty media_info_ms | rows 41-60 | REAL | 20 rows exactly; consistent with mp4 demuxer absent at 2026-10-24; mp4 landed later (container-mp4.md record) |
| 3 | all exit=ok | column | REAL-UNPROVEN | run record; the CLI exits 0 on typed refusal (baseline engine behavior) |

## 3. Contradictions
None.

## 4. Fabrication / inflation indicators
None — records the refusal half of the matrix honestly.

Findings: none. Commands: python3 csv analysis (exit 0).
