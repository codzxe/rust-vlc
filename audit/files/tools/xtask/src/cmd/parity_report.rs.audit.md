# AUDIT: tools/xtask/src/cmd/parity_report.rs

| Field | Value |
|---|---|
| Auditor | W01-A01 |
| Date | 2026-11-16 |
| Lines / functions | 325 / 7 (run, compute_table, cell, find_root + 3 test fns) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — dev tooling, entry `cargo xtask parity-report` |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | `cargo xtask parity-report` → exit 0; full output in `audit/runs/baseline/xtask-parity-report.txt` |

## 1. One-paragraph verdict
A small, genuinely derived statistics tool: it parses the matrix with toml_lite and computes implemented∪verified and verified-only shares per category and platform at run time. Nothing is hardcoded (verified: numbers in my run differ from any doc constant, e.g. windows 66/252 = 26.2%, linux 61/252). Its output on the committed matrix is the repo's real parity state: implemented∪verified = 66/252 windows / 61/252 linux / 66/252 macos / 0/252 web; verified-only = 0/252 on every platform. The "0/252" is the honest number; the 26.2% is the aspirational one, and the tool itself labels the distinction correctly in prose.

## 2. What actually works (verified)
- Full derivation — verified by running it and by the exact-string unit test `table_derives_from_a_small_matrix` (lines 277-324) which constructs a matrix inline and asserts every table row; it ran in the workspace suite (1001 passed).
- The strict verified-only table exists and prints 0/252 everywhere on the real matrix — the premise's "0/252 verified" confirmed by execution.

## 3. Findings

### F-001 [S4] Two "TOTAL" rows with different denominators can be conflated
- **Where:** `tools/xtask/src/cmd/parity_report.rs:175-196`
- **Code:**
  ```rust
    let mut line = format!("| **TOTAL** | {} ", total.rows);
    ...
    out.push_str("\nverified-only (MP §1 strict definition — automated evidence):\n\n");
  ```
- **What is wrong:** The output contains a bold `**TOTAL**` (implemented∪verified) and, ~25 lines later, a plain `TOTAL` (verified-only) with no per-category Rows column. A reader tailing the output sees `| TOTAL | 0/252 (0.0%) | … | 0/252 (0.0%) |` directly above `parity: rows=252 windows=66/252 …` — two adjacent, contradictory-looking numbers describing different sets. The orchestrator itself misread this on first pass (recorded honestly in WAVE-00-SUMMARY.md).
- **Why it matters:** This tool's output is pasted into docs as evidence; ambiguity here becomes doc contradiction there.
- **What correct looks like:** Label the strict row `**TOTAL (verified-only)**` or move the machine line to echo both.
- **Confidence:** HIGH — **Verification:** ran + misread it personally.

### F-002 [S4] Machine summary line carries only the loose metric
- **Where:** `tools/xtask/src/cmd/parity_report.rs:199-203`
- **Code:**
  ```rust
    let mut machine = String::from("parity: rows=");
    ...
        machine.push_str(&format!(" {platform}={}/{}", total.imp_ver[i], total.rows));
  ```
- **What is wrong:** The machine-readable line reports implemented∪verified only; any script consuming it will record 26.2% as "parity" while the strict verified figure (0%) is absent from the machine line entirely.
- **Confidence:** HIGH — read + ran.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `run` | 56-111 | REAL | — | arg parse; parse failure → exit 1 with message |
| `compute_table` | 115-208 | REAL | — | pure derivation, no I/O, tested exact-string |
| `cell` | 211-217 | REAL | — | n/a for zero denominator; 1 decimal |
| `find_root` | 219-236 | REAL | — | same walker as feature_audit.rs (duplicated, S4) |
| tests | 268-324 | REAL | — | honest, would fail on regression; ran |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `cell_formats_share` | YES | exact strings incl. n/a |
| `table_derives_from_a_small_matrix` | YES | asserts 5 table lines + machine line exactly |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Parity percentages are computed by the tool, never stored" | matrix header (quoted in file doc lines 4-5) | REAL | compute_table derives at runtime; no stored numbers in this file |
| "0/252 verified" | premise §0 / PARITY-REPORT | REAL | verified-only TOTAL row: 0/252 on all platforms (run output) |
| "64/252" claims in docs | to audit W19 | — | current derived value is 66/252 windows implemented∪verified |

## 7. Dead / duplicated / unreachable code
`find_root` and the `Must` test trait are copy-pasted between feature_audit.rs and parity_report.rs (S4).

## 8. Dependencies & flags
None gating; std + toml_lite only.

## 9. Open questions for the next agent
- W19: do PARITY-REPORT.md / ROADMAP quote the loose 26.2% figure or the strict 0%? With which label?
