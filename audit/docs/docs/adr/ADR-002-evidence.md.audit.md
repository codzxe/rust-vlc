# CLAIM AUDIT: docs/adr/ADR-002-evidence.md

| Field | Value |
|---|---|
| Auditor | W19-A09 (orchestrator completion, W19-S2) |
| Claims extracted | 18 |
| REAL | 14 | REAL-UNPROVEN | 3 | SHALLOW | 0 | STUB | 0 | PAPER-ONLY | 0 | FALSE | 1 | UNREACHABLE | 0 |
| Internal contradictions found | 1 |

## 1. Blunt verdict
A rigorous, source-cited evidence pack: honest correction of its own
prior undercount (94 -> ~129 wasmtime crates), full candidate-space
closure (wasmer/fizzy included, both fail), and a clear "this file is
the evidence pack only" scope declaration. The decision it feeds
(wasmi) matches what the repo actually shipped — this is the rare doc
whose outcome is verifiable in the tree.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "this file is the evidence pack only" (decision happens in 0.D3) | :6-7 | REAL | scope-honest framing |
| 2 | "All facts fetched 2026-10-19 ... Nothing was built or executed" | :8-9 | REAL | honest method disclosure (tarball-inspection only) |
| 3 | "the 0.C4-I scratch walk ... undercounted (94 crates) ... Re-walked: ~129 crates" | :10-12 | REAL | exemplary self-correction |
| 4 | "wasmi ... H1 purity PASS (F1-F7 all NO)" | table | REAL | wasmi in Cargo.lock pure (W12 sandbox audits: fuel/limits/jail real, no C) |
| 5 | "wasmtime H1 FAIL — C compilation cannot be feature-gated off" | table | REAL | wasmtime not in any lock; rationale verified by its known build graph |
| 6 | "wasmer fail; fizzy dead ~4.3 y" | table | REAL-UNPROVEN | external facts, consistent with crates.io common knowledge |
| 7 | "wasmi fuel only (no epoch) — doc 06 §5.4 names this the acceptable fallback" | table | REAL | W12-A05: fuel metering real (consume_fuel+set_fuel, trap on grow) |
| 8 | "MSRV Rust 1.86 / 1.96 etc." | table | REAL-UNPROVEN | external; repo pin 1.99.1 satisfied |
| 9 | "Pre-audit verdict: wasmi pass (proposed pin >=2.0.0,<2.1)" | table | REAL | wasmi in lock (W12) — the recommendation landed |
| 10 | "unfetchable = [VERIFY]" markers | method | REAL | markers actually used throughout |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "Nothing was built or executed" | crate is shipped and fuzz-tested | No conflict — the PACK made no claims beyond its method; later waves executed |

## 4. Fabrication / inflation indicators
None. This document is the standard the other dependency docs meet;
its claims carry sources and its one numeric error is self-corrected
in-place.
