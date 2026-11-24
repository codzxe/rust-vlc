# CLAIM AUDIT: docs/corpus/compat-matrix-transitions-20261110.json
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 5 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 5 / 0 / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Five status-DOWNGRADE records (implemented → partial, linux) with typed
error text, reasons, and next steps. All five are reflected in the current
matrix rows — the anti-inflation direction: a compat run that found the
claims overstated and corrected them downward.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | container-asf-wmv linux implemented→partial (probe refuses asf corpus) | row 1 | REAL | matrix:215 linux partial today |
| 2 | codec-audio-aac-lc linux →partial (142 concealed packets on VALID files) | row 2 | REAL | matrix:673 linux partial |
| 3 | codec-audio-aac-lc second transition (1 refused track vs 4 decoded) | row 3 | REAL | same row partial; two distinct reasons recorded separately |
| 4 | codec-audio-alac linux →partial (1 refused vs 1 decoded) | row 4 | REAL | matrix:720 linux partial |
| 5 | codec-video-vp9 linux →partial (1 refused vs 2 decoded) | row 5 | REAL | matrix:480 linux partial |

## 3. Contradictions
None — every "to" state matches the live matrix.

## 4. Fabrication / inflation indicators
None — this is the correction mechanism working: claims reduced to match
evidence, with reproducible error strings (sniffed bytes for asf).

Findings: none. Commands: rg matrix rows (exit 0).
