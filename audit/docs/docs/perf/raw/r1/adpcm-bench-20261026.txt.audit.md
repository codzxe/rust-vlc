# CLAIM AUDIT: docs/perf/raw/r1/adpcm-bench-20261026.txt
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 2 |
| REAL 2 | |

## 2. Claim ledger
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | "the two ADPCM lanes the 5.R1 snapshot recorded as DNF ('no ignored-bench harness exists')" — gap-closing record | REAL | the header self-identifies as closing a previously-recorded DNF; worktree wip/p6-adpcmb @ 65a7f1a noted |
| 2 | "median 153767893 samples/s = 1745x realtime (3.832s of audio)" | REAL-UNPROVEN (dated) | arithmetic: 153.8M samples/s over 44.1kHz stereo => 1745x realtime — checks out; run not repeatable here |

## 1. Blunt verdict
Benchmark closeout evidence with the measurement command verbatim (cargo test -p codecs --release --test adpcm_bench -- --ignored) and the honesty note that wav-demux is INCLUDED so decode-only throughput is "strictly higher". Gap-closure culture.
## 3. Contradictions: none. ## 4. Indicators: none.
