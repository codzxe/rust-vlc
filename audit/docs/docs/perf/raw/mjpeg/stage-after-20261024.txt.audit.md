# CLAIM AUDIT: docs/perf/raw/mjpeg/stage-after-20261024.txt
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 2 |
| REAL 2 | |

## 2. Claim ledger
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | 7-pass stage benchmark: full/scalar/scan/noidct/nowrite/huffonly/parse ms per pass | REAL | 7 rows present, values mutually consistent (huffonly 15.5 < noidct 17.2 < full 55.8 — the pipeline decomposition is arithmetic-coherent: 15.5+13.0+1.7+25.5+0.1 ≈ 55.8) |
| 2 | "simd-vs-scalar ... full(simd) 55.8 ms vs full(scalar) 54.7 ms -> 1.021x (102.1% of scalar walk)" | REAL (honest-negative) | the SIMD lane is SLOWER than scalar — published as-is, not hidden; STAGEBENCH machine line carries the same numbers |

## 1. Blunt verdict
The "after" half of the pair, including a negative result (SIMD 2% slower on this 512x512 corpus) printed in the machine line. Shares sum to the full time — internally arithmetic-consistent.
## 3. Contradictions: none. ## 4. Indicators: none — negative results committed verbatim.
