# CLAIM AUDIT: docs/soak/raw/scenarios-sandbox__reopen-stress.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 2 |
| REAL 2 | |

## 2. Claim ledger
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | "reopen-stress 150000 cycles, wall 200.1s, verdict PASS; live bytes 292783 -> 292783 (budget 524288 B); fd 4 -> 4" | REAL | internally consistent (zero live-byte drift across 150k open/play/close cycles); fixtures committed (tests/fixtures/generated/valid/avi_mjpeg*.avi) |
| 2 | "Decoding: 1650000 frames, 103887000000 payload bytes at the sinks (the D-6 liveness axis)" | REAL-UNPROVEN | the liveness proof that ops really decoded (11 frames/cycle x 150k = 1.65M — arithmetic consistent) |

## 1. Blunt verdict
The full 150k-cycle PASS run with live-bytes EXACTLY flat (292783 -> 292783) — a genuine no-leak soak record with committed fixtures and instrumented measurement.
## 3. Contradictions: none. ## 4. Indicators: none.
