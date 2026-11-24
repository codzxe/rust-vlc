# CLAIM AUDIT: docs/perf/v1-input-readpath-20261024.md
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 5 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 4 / 1 / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The doc's central claim — every number is a test-pinned assertion — is
true: all five named pin tests exist verbatim in
crates/demux/tests/zero_alloc_read_chain.rs and riff.rs, and the seam-call
decomposition (14 open + 6 payload + 6 header = 26; 6×1.5 MiB) is exactly
what the test constants encode (CHUNKS 6, PAYLOAD 1536*1024, OPEN_READS
14). The arena pool implementation exists as described. Part-2 claims
(input crate window bypass) also resolve.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "PayloadSlots rebuilt as a bounded arena pool … SharedBytes::from_pool_range" | :55-60 | REAL | crates/demux/src/avi/riff.rs:457-465 arena pool doc + impl; SharedBytes import :22 |
| 2 | "zero allocation per video packet … pinned by varying_size_payloads_pack_into_the_arena_zero_alloc" | :86 | REAL | zero_alloc_read_chain.rs:391 exact test name |
| 3 | seam calls "164 → 26 (6 payload-class, full ask)" | :90 | REAL | :1072 test: CHUNKS=6, OPEN_READS=14, PAYLOAD=1.5 MiB pinned constants |
| 4 | "arena growth … pinned by riff::tests::arena_growth_stays_under_the_declared_bound" | :94-95 | REAL | riff.rs:1148 exact name |
| 5 | "Full demux suite on this branch: 140 passed, 0 failed" | :97-99 | REAL-UNPROVEN | branch count; baseline cargo-test-workspace shows demux suites green (workspace 4744/2), exact 140/0 not re-run (cargo-build-budget) |

## 3. Contradictions
None. Not-measured list (§5) is explicit: syscall counts, EOF clamp,
latency benches all declared unmeasured — no inflation.

## 4. Fabrication / inflation indicators
None — §5 "Not measured / known limits" + §6 D-10 protocol disclose scope
honestly; the amended pin (12→26) is explained with a non-weakening
argument verifiable in the test comments.

Findings: F-001 [S4] perf numbers (2.000 allocs/packet before) rely on
branch-state runs; the after-numbers are pinned in committed tests and
therefore durable.
Commands: rg test names, sed test constants (exit 0 each).
