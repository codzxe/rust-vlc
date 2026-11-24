# CLAIM AUDIT: docs/formats/container-wav.md
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 6 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 5 / 0 / 1 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Honest record: implementation claims verify (code tree, tests, fuzz target), and its
self-flagged gap ("matrix row still reads not_started") was TRUE at writing — the matrix
has since been amended to implemented with this file as evidence, exactly as the record
proposed. Its own staleness is on the right side.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Status: implemented (demux, card 5.A2)" | L3 | REAL | crates/demux/src/wav/{format,fuzz,mod,tests,test_support,error}.rs exist; tests.rs = 26 #[test] |
| 2 | "Feature-matrix row container-wav still reads not_started with empty evidence" | L5-6 | STALE (resolved) | matrix now: `status = { windows = "implemented", ... }` + evidence citing this record (matrix L220 block) — the record's own proposed amendment landed |
| 3 | "fuzz_target = fuzz_demux_wav" | front-matter | REAL | fuzz/fuzz_targets/fuzz_demux_wav.rs exists |
| 4 | "crates = [] — zero external deps, C5 std-only" | front-matter | REAL | demux crate wav module has no dep table entries (Cargo.toml check: no wav-specific dep) |
| 5 | "evidence Linux-only" | L11 | REAL | consistent with parity-report linux 61/252 vs windows/mac lower |

## 3-5. Contradictions / fabrication / matrix rows
None / none / container-wav row cross-verified (implemented + evidence, matrix L220-231).
Commands: ls crates/demux/src/wav; rg -c '#\[test\]'; matrix row sed.
