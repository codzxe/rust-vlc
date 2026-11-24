# CLAIM AUDIT: docs/soak/raw/scenarios-forced-threshold.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 3 |
| REAL 3 | |

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "open-close-churn ... verdict FAIL (2 violation(s))" | Summary | REAL | the report commits its own FAIL: 82 live bytes over a 0-B budget and 0.25 MB RSS over 0.01 MB — byte-level honesty |
| 2 | "fixtures ... against the committed `tests/fixtures/` media" | Method | REAL | both cited fixtures exist: tests/fixtures/generated/valid/avi_mjpeg_pcm.avi + malformed/avi_mjpeg_pcm__trunc3.avi (ls) — replayable |
| 3 | "Every MB/h figure below is an EXTRAPOLATION from that window, not a 4 h measurement" | Method | REAL | the extrapolation caveat is stated up-front — the exact anti-inflation disclosure the FREEZE decision needs |

## 1. Blunt verdict
A soak report that publishes FAIL for 82 bytes of drift over 849,815 cycles — the strictest memory-budget honesty in the repo. Instruments (counting allocator, /proc RSS, fd count) and warmup exclusion are documented.
## 3. Contradictions: none. ## 4. Indicators: none. Manifest path /home/z/my-project/wt/w24b (worktree) is outside the repo — S4 replay limitation.
