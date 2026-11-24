# CLAIM AUDIT: docs/web/size-record.toml
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 4 |
| REAL 3 / REAL-UNPROVEN 1 | |

## 1. Blunt verdict
A clean machine-readable measurement record: parses as TOML, 5 configs, budgets in config (not literals), floor under budget. Its self-description ("the gate ... rewrites this file byte-stably") is a mechanism claim I could not execute here (wasm32 target absent — the same environment limit the baseline web-budget gate failure records). No timestamps, per its own determinism rule — verified by reading.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "the measured sizes of the lumen-web wasm module ... across codec-feature configurations" | header | REAL | parses; 5 config rows each with raw/gzip/sha256; floor raw_bytes 1,294,311 / gzip 300,250 |
| 2 | "BUDGETS live in config ... product.toml [web.size_budgets]" | :25-28 | REAL | product.toml:212 [web.size_budgets], phase2_vertical_max_mib = 3 with the gate-wiring comment |
| 3 | "`cargo xtask web-budget --check` re-measures and compares (budget breach or >5% growth ... fails the gate)" | :9-13 | REAL-UNPROVEN | the gate step exists (baseline full_gate step 10 "web-budget: FAILED" — but for the missing-target environment reason E0463, which itself proves the gate RUNS a real cargo build); the compare logic not executed by me |
| 4 | "DETERMINISM: no timestamps, no paths-to-nowhere" | :30-31 | REAL | file content verified: no timestamp/path fields; sha256 digests pinned per row |

## 3. Contradictions
None within the file. Cross-doc note: worklog w28 mentions "wasm 37.9MB" in rc-smoke — that is a different feature configuration than any recorded row (external-all = 2.44 MB raw); the record covers the analysis configs, rc-smoke its own build. No conflict in-file.

## 4. Fabrication / inflation indicators
None — the record labels its non-rebuilt rows "manual size analysis 2026-11-11" and pins sha256s; the report it feeds (docs/web/size-report.md) exists.
