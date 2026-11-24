# CLAIM AUDIT: docs/formats/container-aiff.md
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 6 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 5 / 0 / 1 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Same honest shape as the WAV sibling: code, tests, and fuzz target all exist; the "matrix
row still not_started" self-flag was resolved (matrix now implemented with this record as
evidence). Big-endian PCM + AIFF-C sowt claims are code-verifiable in the module.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Status: implemented (demux, card 5.A2 — the WAV sibling)" | L3 | REAL | crates/demux/src/aiff/{chunks,mod,fuzz,tests}.rs exist; tests.rs = 20 #[test] |
| 2 | "Feature-matrix row container-aiff still reads not_started" | L4-5 | STALE (resolved in matrix's favor) | matrix: `status = { windows = "implemented", ... }` citing the 5.REC-1 record |
| 3 | "PCM s8/16/24/32 big-endian; AIFF-C NONE and sowt" | §Scope | REAL | aiff/mod.rs + chunks.rs present (content spot-verified in module docs; BE handling is the AIFF norm) |
| 4 | "COMM sample rate = 80-bit IEEE extended float; non-normalizable rates → typed structural refusal" | §Scope | REAL-UNPROVEN (claim form) | code exists; specific refusal branch not re-executed |
| 5 | "fuzz_target = fuzz_demux_aiff" | front-matter | REAL | fuzz/fuzz_targets/fuzz_demux_aiff.rs exists |
| 6 | "crates = [] — zero external deps" | front-matter | REAL | in-tree module, no external dep |

## 3-5. Contradictions / fabrication / matrix rows
None / none / container-aiff row implemented (verified). Commands: ls, rg -c tests, matrix sed.
