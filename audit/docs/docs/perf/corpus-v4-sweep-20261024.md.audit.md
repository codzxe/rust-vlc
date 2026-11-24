# CLAIM AUDIT: docs/perf/corpus-v4-sweep-20261024.md

| Field | Value |
|---|---|
| Auditor | W19-A02 (orchestrator completion, W19-S2) |
| Claims extracted | 9 |
| REAL | 7 | REAL-UNPROVEN | 2 | SHALLOW | 0 | STUB | 0 | PAPER-ONLY | 0 | FALSE | 0 | UNREACHABLE | 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A real census run over a genuinely-regenerated corpus with committed
raw evidence — the strongest kind of perf doc in the repo. The 21/126
"ok" vs 103 "unsupported" tally is honest (matches the shipped-surface
reality the engine audits found).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "126 files regenerated from the committed manifest ... via gen_corpus.py --replay (own seed)" | :13-15 | REAL | manifest committed; regen tooling committed (W18 corpus-plan audit) |
| 2 | "lumen-sweep --timeout-secs 30 ... single pass" | :16-17 | REAL | tools/sweep exists (W16-A07 audited); SWEEPLINE format documented |
| 3 | tally: ok 21 / unsupported 103 / dirty-reject 1 / typed-error 1 / panic 0 / timeout 0 | table | REAL | sums to 126 exactly; consistent with shipped surface (15 containers + codecs subset) |
| 4 | "0 panics and 0 timeouts across all 126 files, including 14 dirty / 21 garbage" | :~30 | REAL-UNPROVEN | raw log committed (force-added per header) — not re-run in this audit |
| 5 | "per-file wall times are all <= 0.63 s" | :~20 | REAL-UNPROVEN | plausible (process spawn dominated); raw log cited |

## 3. Contradictions
None.

## 4. Fabrication / inflation indicators
None — header discloses that raw *.log is gitignored but this one was
force-added (honest evidence-chain note). Contrast with the probe-work
logs LOST to .gitignore (W18-A03/A04 S2 findings).
