# CLAIM AUDIT: docs/dependencies/audit-method.md
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Claims extracted | 10 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 5 / 0 / 0 / 0 / 1 / 1 / 3 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
This is a real, executed method doc, and its worked example (wasmi 2.0.0) is
corroborated by the committed Cargo.lock — the tarball sha256 it recorded is
byte-identical to the lockfile checksum, and the dependency table matches the
lockfile's `wasmi` 2.0.0 dependency list. One load-bearing claim is FALSE: the
"Full audit (Phase 1)" is said to run `tools/dependency-audit --deny-native` —
that tool does not exist anywhere in the tree (no `tools/dependency-audit`, no
xtask subcommand, zero code references). The method doc's Phase-1 half is
therefore aspirational while written in the indicative. Network-dependent facts
(license, crate_size, OSV advisory IDs, fetch dates) are UNVERIFIABLE from this
sandbox (no network).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Companion files: ADR-002-evidence.md, preaudit-foundation.md, records/wasmi.md, allowlist-format.md" | :8 | REAL | `ls docs/adr/ docs/dependencies/` — all four exist |
| 2 | "Full audit (Phase 1 ...): ... cargo-deny, cargo-vet, and the tools/dependency-audit --deny-native scanner" | :20-22 | FALSE (tool absent) | `ls tools/` -> bench-vs-vlc corpus mem-audit perf sweep vta xtask; `rg 'deny-native'` -> docs only (06-plugin:800, 11:306, allowlist-format:162); xtask/src/cmd/ has no dependency_audit.rs |
| 3 | wasmi 2.0.0 tarball "sha256 78693fcdd618e0fc34af59c6b8efa9ac5d58c68df940beff4bedddb6acfe7c27", cksum MATCH | :266-271 | REAL | `sed -n '1263,1267p' Cargo.lock` -> `checksum = "78693fcdd618e0fc34af59c6b8efa9ac5d58c68df940beff4bedddb6acfe7c27"` — the committed lockfile carries the identical checksum |
| 4 | M3 dep table: spin ^0.9, wasmi_collections/core/ir ^2.0.0, wasmparser ^0.228.0, wat optional | :245-254 | REAL | `sed -n '1263,1272p' Cargo.lock` -> deps = spin, wasmi_collections, wasmi_core, wasmi_ir, wasmparser; wasmparser 0.228.0, spin 0.9.9 in lock |
| 5 | M8 graph "bitflags 2.13.2 ... spin 0.9.9 ... wasmparser 0.228.0" | :320-323 | REAL | `rg -n 'name = "(bitflags|spin|wasmparser)"' -A1 Cargo.lock` -> bitflags 2.13.2 (63-64), spin 0.9.9 (937-938), wasmparser 0.228.0 (1303-1304) |
| 6 | "Pre-audit verdicts do not create allowlist rows" | :204 | REAL | docs/dependencies/allowlist.md exists separately; records feed it (not re-verified row-by-row here) |
| 7 | "`tmp/` is in `.gitignore` — verified git check-ignore" | :31-32 | REAL | `rg -n 'tmp' .gitignore` -> line 40 `tmp/` |
| 8 | wasmi license "MIT/Apache-2.0", crate_size 300979, yanked false, advisories GHSA-75jp-vq8x-h4cq / GHSA-g4v2-cjqp-rfmq | :231-239, 308-310 | UNVERIFIED | no network from sandbox; internal cross-ref records/wasmi.md:114 carries the same checksum (self-consistent) but license/OSV facts not re-fetchable |
| 9 | "All URLs cited were fetched on that date [2026-10-19]" | :7 | UNVERIFIED | no network; no committed raw outputs (§3 M8 explicitly says walk outputs "not committed") |
| 10 | "Phase 1 moves it into tools/dependency-audit" (the §5 scan script) | :350-351 | PAPER-ONLY | `ls tools/` — no dependency-audit directory; script exists only inside this doc |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Phase 1 full audit uses `tools/dependency-audit --deny-native` (:22) | `ls tools/` — tool absent; docs/inventory/security-phase0.md:276 says "land tools/dependency-audit before Phase 2" (a TODO) | The tool is missing; security-phase0's TODO phrasing is the honest one. audit-method.md presents it as existing method |
| README.md:99 lists tools/ "dependency-audit license-audit" as if present | `ls tools/` shows neither | Same gap propagated to README (outside this file's scope, noted) |

## 4. Fabrication / inflation indicators
- LOW. The worked example's checksum matches the committed Cargo.lock exactly —
  hard to fabricate accidentally. Honest `[VERIFY-P0]` markers throughout, and
  §6 "What the pre-audit does NOT prove" is an explicit limitations section —
  the opposite of inflation.
- The one inflation: presenting the not-yet-landed scanner as part of the
  method ("identical method plus ... the scanner") instead of marking it
  `[VERIFY]`/planned like other future items.

## 5. Method soundness (reviewed, not just claim-checked)
M1–M9 are coherent and reproducible on paper: metadata -> version record ->
deps -> tarball+cksum vs sparse index -> native-source scan -> build.rs/FFI
scan -> maintenance/OSV -> graph -> verdict. The M6 token table with human
classification is a sane design (avoids false-positive auto-verdicts). The
`find` in M5 and the scan script (§5) use proper escaping. Two hygiene notes:
(a) §5 script `echo "sha256: $(sha256sum $f | cut -d' ' -f1)"` uses unquoted
`$f`/`$root` (paths with spaces would break; low impact in tmp/preaudit); (b)
M4's fallback through the crates.io API download endpoint is a legitimate
documented workaround for the static.crates.io 403.
