# CLAIM AUDIT: docs/adr/ADR-002-evidence.md (L1-196)
| Field | Value |
|---|---|
| Auditor | W19-A09 |
| Date | 2026-11-24 |
| Claims extracted | 6 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 2 / 0 / 0 / 0 / 0 / 0 / 4 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A clean evidence pack: it explicitly declares itself "the evidence pack only -- NOT a decision", separates the four candidates honestly (wasmi PASS, wasmtime/wasmer/fizzy FAIL on H1 purity), discloses its own methodology limits ("Nothing was built or executed"), and publishes a self-correction (the 94 to ~129 crate undercount caused by manifest-name vs crates.io-name drift). Every crates.io/tarball fact is a dated network fetch not re-verifiable here; the downstream decision (wasmi accepted) and the implemented host (crates/plugin-host-wasm, wasmi 2.0.0 in lockfile) are in-tree facts.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "this file is the evidence pack only" (not a decision) | header | REAL | Self-scoped; the decision lives in ADR-002-wasm-runtime-wasmi.md (exists). |
| 2 | wasmi 2.0.0: "100% Rust... 0 native files, 0 blobs, 0 extern C, no links=" | S3 | UNVERIFIED | Dated tarball scan (2026-10-19); downstream: wasmi 2.0.0 in lockfile, plugin-host-wasm crate real (rc-smoke: crash isolation tested). |
| 3 | wasmtime "FAIL -- C compilation cannot be feature-gated off" | S3 | UNVERIFIED | Dated fetch; consistent with the chosen outcome. |
| 4 | fizzy "dead ~4.3 y" + Apache-2.0 C++ bindings | S3 | UNVERIFIED | Dated fetch. |
| 5 | Correction note: undercount 94 to ~129 crates, cause named | header | REAL (governance) | The self-correction is itself evidence of method (M8 approximation rules) being applied honestly. |
| 6 | Candidate space closure (wasmer/fizzy scanned; wasm3/WAMR/wazero fail by language) | S2 | UNVERIFIED | Reasoned closure from language policy -- sound logic, facts dated. |

## 3. Contradictions
(none)
## 4. Fabrication / inflation indicators
None -- the doc's whole value is [VERIFY]-labeled negative results. Commands run: ls docs/adr/, plugin-host-wasm existence cross-check. Exit 0.
Findings: S0-S4: 0.
