# AUDIT: scripts/quality/07_supply_chain.sh
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 39 / 5 sections (7a duplicates, 7b cargo-deny, 7c SBOM drift, 7d unsafe census, 7e dep graph) |
| File verdict | REAL (reporter-tier: J in the static tier of 99_run_all.sh — exit codes recorded, tier not failed by design) |
| Reachable from shipped entry point? | YES as tooling — static tier of scripts/quality/99_run_all.sh |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## 1. One-paragraph verdict
A real reporter over the real supply-chain tools: cargo-tree duplicates, cargo-deny (four checks), the xtask SBOM drift check (same generator ci.yml uses), a cargo-geiger unsafe census with an honest grep-based first-party fallback (recorded as not_run when geiger is absent), and a DOT dep graph. Its pin claim is true — ci.yml:482-490 pins cargo-deny 0.20.2 with a SHA-256-verified musl tarball. NOT EXECUTED here deliberately: every section writes into docs/audit/reports/ (repo files) and sections 7b/7c invoke cargo — prohibited by load discipline. Two S4 notes (below); no fabrication vectors: missing tools produce not_run entries, never invented output.

## 2. What actually works (verified)
— Read in full; common.sh contract read (need/not_run/sg_run; "NEVER invent a result" at common.sh L21).
— Pin claim verified (ran, Grep tool): ci.yml L95 + L482-490 — "cargo-deny 0.20.2 ... tarball cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz" — the script's L15 claim is TRUE.
— Fallback census (7d) is a real grep over first-party code excluding /tests/ (L29-31) — a genuine measurement, not a stub.
— Exit-code capture: `say "deny exit=$?"` directly after the pipeline (pipefail on) records the pipeline status — correct usage.

## 3. Findings

F-001 [S4]
Where: scripts/quality/07_supply_chain.sh:13 and 21
Code:
```
  say "deny exit=$?"
...
  say "sbom exit=$?"
```
What is wrong: the exit codes are PRINTED, never acted on — as a J-tier reporter that is the design, but a FAILING cargo-deny or SBOM drift produces a red "exit=1" line buried in log output while the script itself exits 0. The operator must read; nothing aggregates these two (unlike the JG gates). Confidence: HIGH — read; severity S4 because the J/JG split is documented and deliberate.

F-002 [S4]
Where: scripts/quality/07_supply_chain.sh:37-38
Code:
```
  cargo modules dependencies --workspace > "$OUT/depgraph.dot" 2>/dev/null \
    && say "dot: ..."
```
What is wrong: stderr swallowed and no else-branch — a cargo-modules RUNTIME failure (after `need` passes) silently produces no dot file and no not_run entry. Confidence: MEDIUM (read; failure path not exercised).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| 7a duplicates | 6-8 | REAL | - | tee full output, head display-only; count with `grep -c \|\| echo 0` |
| 7b cargo-deny | 10-17 | REAL | S4 | F-001 (reporter by design); missing tool -> install hint with the TRUE pin |
| 7c SBOM drift | 19-21 | REAL | S4 | F-001; same xtask generator ci.yml uses |
| 7d unsafe census | 23-32 | REAL | - | geiger or honest grep fallback; not_run recorded |
| 7e dep graph | 34-39 | REAL | S4 | F-002 silent-failure corner |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| (reporter; no tests) | n/a | honest-absence discipline (not_run) verified in common.sh; no invented output paths |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the repo already pins 0.20.2 in ci.yml" | L15-16 | REAL | ci.yml:482-490 (Grep, ran): 0.20.2 musl tarball, SHA-256-verified |
| "SBOM drift (xtask, same generator as ci.yml)" | L19 | REAL | `cargo run -p xtask --locked -- sbom --check` — the same xtask sbom subcommand packaging.toml documents as the SBOM content generator |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
cargo tree/deny/geiger/modules + xtask sbom (cargo — the reason this audit did not execute it); writes under docs/audit/reports/ by design.

## 9. Open questions for the next agent
1. docs/audit/reports/deny.txt (if committed) — does the recorded deny exit differ from 0 anywhere?
