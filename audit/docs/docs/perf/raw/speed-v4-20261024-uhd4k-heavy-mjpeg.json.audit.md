# CLAIM AUDIT: docs/perf/raw/speed-v4-20261024-uhd4k-heavy-mjpeg.json
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 3 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 0 / 3 / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
ffmpeg-vs-lumen speed record. Every derived number recomputes exactly
(fps = frames/wall, realtime_x = dur/wall — 8/8 checks). It records
Lumen LOSING to ffmpeg (0.43× realtime on the heavy lane) — the honest
direction. Not re-runnable (corpus at /tmp absent).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | per-file fps/realtime_x for ffmpeg + lumen, n=2 | per_file[] | REAL-UNPROVEN | python3 recompute: 133/1.2199=109.03 ✓, 1338/12.8747=103.92 ✓, 5.575/3.7643=1.481 ✓, 5.575/12.8747=0.433 ✓ (all 8 derived values exact) |
| 2 | lumen ok:true on both lanes | rows | REAL-UNPROVEN | consistent with the AVI+PCM decode path (baseline rc-smoke headless-decode PASS) |
| 3 | repeat=2, runners ffmpeg+lumen | header | REAL-UNPROVEN | record; n:2 fields consistent |

## 3. Contradictions
None.

## 4. Fabrication / inflation indicators
None — the headline is a LOSS (0.43× realtime), retained verbatim.

Findings: F-001 [S4] corpus absent, not re-runnable.
Commands: python3 arithmetic recompute (exit 0).
