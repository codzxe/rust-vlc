# CLAIM AUDIT: docs/perf/raw/parity-v4-smoke-ffprobe-20261024.log
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 2 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 0 / 2 / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
An ffprobe ground-truth dump over 21 fixtures (one `== <rel>` header
per fixture, stream/duration lines in genuine ffprobe flat format).
Format is exactly ffprobe's `-show_streams -show_format` flat output
grammar (index/codec_name/codec_type/nb_read_frames, duration lines).
Not re-runnable (corpus absent).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | 21 fixtures covered, AVI/MJPEG/PCM lanes incl. a multivideo mpeg4+mjpeg+pcm file | whole log | REAL-UNPROVEN | rg -c '^== ' → 21; the 0021 multivideo 3-stream record (mpeg4+mjpeg+pcm_s16le) matches the diff-report era corpus naming |
| 2 | ffprobe outputs are the ground truth for the v4 smoke comparison | implicit | REAL-UNPROVEN | ffprobe binary absent; format grammar authentic |

## 3. Contradictions
None.

## 4. Fabrication / inflation indicators
None.

Findings: F-001 [S4] corpus absent — log not re-runnable.
Commands: rg -c headers (exit 0).
