# CLAIM AUDIT: tools/vta/dnx/README.md
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Claims extracted | 6 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 4 REAL / 2 REAL-UNPROVEN / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Every committed-artifact claim in this README verifies, and verifies HARD: the `evidence/` directory exists with all three named files PLUS two more the README under-advertises (lane-md5s.txt, compare.py itself). The hostile battery's "Result this run: 0 panics (3 lanes)" is byte-verified against hostile-results.txt (three lane sections, each ending "TOTAL PANICS: 0"). NOTE: this README audit is what exposed my own earlier false claim about a missing Cargo.lock (see the CORRECTION INCIDENT in the run log) — the directory is fully disciplined: lock + evidence + record all present.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "evidence/ (committed) — ffprobe-facts.txt, compare-results.txt, hostile-results.txt" | L49-53 | REAL | `ls tools/vta/dnx/evidence/` -> all 3 + compare.py + lane-md5s.txt (ran) |
| 2 | "Result this run: 0 panics (3 lanes)" | L47 | REAL | hostile-results.txt: sections L1/L4/L5 each "TOTAL PANICS: 0"; sweeps "0 panics, 208/201/206 Ok-frames" (ran, read) |
| 3 | "ours: split CBR ES into N frames ... python3 evidence/compare.py" | L33-36 | REAL | compare.py exists (ls, ran); main.rs implements the split (cross-file audit) |
| 4 | "corpus has NO dnxhd lanes" + 5 ffmpeg lane commands | L12-27 | REAL-UNPROVEN | commands not re-run (ffmpeg budget); ffprobe-facts.txt documents exactly these lanes (cids 1238/1251/1235 consistent with hostile-results) |
| 5 | "N ... cross-checked by byte-count parity" | L38-39 | REAL-UNPROVEN | operator-side check; no in-script assert |
| 6 | "Evidence record: docs/dependencies/records/filmcraft-dnx-0.9.0.md" | L4-5 | REAL | file exists (ls, ran) |

## 3. Contradictions
None found.

## 4. Fabrication / inflation indicators
NONE — the opposite: the README UNDER-lists its own evidence (2 extra committed files); the committed compare-results.txt honestly records non-bit-exact tolerance-class differentials (L1: "md5 differs", Y maxΔ=2, ~1.3% differing) — recorded, not smoothed.

## 5. Matrix rows
N/A.

## 6. Cross-checks run
- `ls -la tools/vta/dnx` -> Cargo.lock + evidence/ + src (ran; triggered the correction incident in the run log).
- `head/tail evidence/hostile-results.txt` -> 3 lanes, TOTAL PANICS: 0 each (ran).
- `head evidence/{compare-results,lane-md5s,ffprobe-facts}.txt` -> genuine per-plane delta tables, MD5 pairs, codec facts (ran).
- Cargo.lock filmcraft-dnx 0.9.0 ck `a3a59d55...` == root Cargo.lock (ran, byte-compared).
