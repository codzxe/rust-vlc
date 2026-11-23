# AUDIT: crates/playlists/tests/fixtures/sparse.pls (text fixture)
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-23 |
| Size / type | 118 bytes; `file` → "ASCII text" |
| File verdict | REAL (hand-crafted dirty-world PLS truth-set member, doing real semantic work) |
| Reachable from shipped entry point? | N/A (test asset). Consumer: crates/playlists/tests/pls_suite.rs:18 `include_str!("fixtures/sparse.pls")` → `golden_sparse_indexes_case_and_comments` (:59-81) + roundtrip corpus (:257). Runs in the workspace gate (baseline: 4744 passed). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. Verdict
A deliberately hostile-but-legal PLS: sparse entry indexes (file1 then File5 — 2..4 absent), mixed-case keys (`file1` vs `File5` vs `TITLE5`), both comment styles (`;` and `#`), and a `NumberOfEntries=2` that matches the sparse count. The consuming test pins the interesting semantics exactly: index ORDER not line order (entry 1 then entry 5), TITLE5 attaching to index 5, zero warnings. This is the "handcrafted basic/sparse/dirty" truth-set the plfmt-pls matrix row cites as evidence (docs/feature-matrix.toml:1179, status "implemented"), and it would fail if the PLS parser sorted by line, dropped sparse indexes, or mishandled case.
## 2. What actually works (verified)
— `cat` inspection (contents byte-read this audit); consumer assertions read at pls_suite.rs:59-81; suite passes in the orchestrator baseline.
## 3-9. Findings / notes
None. No secrets, no absolute paths, no encoded payloads; 118 bytes of pure test truth.
