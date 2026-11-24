# CLAIM AUDIT: docs/baselines/raw/linux/06-vlc-list-verbose.txt (2 parts, L1-1288)
| Field | Value |
|---|---|
| Auditor | W19-A08 |
| Claims extracted | 4 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 4 / 0 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Genuine `vlc --list-verbose` capture of the pinned 3.0.23 binary. Header's module-count
claim reproduces exactly under an independent count. Chunk ranges L1-800, L801-1288 are
inside the real 1288-line file.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Module count: 502" | L5 | REAL | `rg -c '^  [a-z0-9_]+ '` -> 502 (exact) |
| 2 | "Same env as dump 05" pinned binary | L3-4 | REAL | header verbatim (sed -n '1,14p') |
| 3 | "Format per module: ... 'c' lines = capability (score)" | L5-6 | REAL | L3-7 show `c sout access (0)` lines exactly as described |
| 4 | "# exit=0" | L1288 | REAL | tail -5 verbatim |

## 3-4. Contradictions / fabrication indicators
None.
## 5. Matrix rows
N/A — module inventory evidence baseline (modules-linux.json consumers, other agents).
Commands: wc -l (1288); rg -c module-name lines (502); sed/tail spot checks.
