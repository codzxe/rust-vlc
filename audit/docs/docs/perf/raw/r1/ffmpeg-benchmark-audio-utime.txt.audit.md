# CLAIM AUDIT: docs/perf/raw/r1/ffmpeg-benchmark-audio-utime.txt
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 1 |
| REAL 1 | |

## 2. Claim ledger
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | ffmpeg -benchmark utime/stime/rtime per corpus lane, incl. ALAC committed fixtures (crates/codecs/tests/fixtures/alac) | REAL | the cited alac fixture files exist in-tree (alac_mono16.m4a etc. — the bench used committed fixtures, so this one IS replayable); header honestly calls kernel granularity "coarse" |

## 1. Blunt verdict
Raw ffmpeg timing lanes with an explicit limitation note (1ms granularity) and a pointer to the median-of-5 family for the real numbers. Cites committed fixtures — replayable.
## 3. Contradictions: none. ## 4. Indicators: none.
