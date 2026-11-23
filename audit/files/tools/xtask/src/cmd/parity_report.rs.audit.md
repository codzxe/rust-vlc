# AUDIT: tools/xtask/src/cmd/parity_report.rs
| Field | Value |
|---|---|
| Auditor | W01-A09 |
| Date | 2026-11-23 |
| Lines / functions | 325 / 5 prod (`run`, `compute_table`, `cell`, `find_root` + `CategoryStats`) + 2 tests + `Must` trait |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — dev tool dispatched as `parity-report` (tools/xtask/src/cmd/mod.rs:151-155); NOT wired into CI or full_gate (rg over .github/scripts: no invocation); consumed manually by agents and quoted in docs/PARITY-REPORT.md. Not in any shipped binary. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `./target/debug/xtask parity-report` -> exit 0, full table (TOTAL 252 rows: windows 66/252, linux 61/252, macos 66/252, web 0/252; verified-only 0 everywhere); python3 tomllib independent recount of all 252 rows' status strings (matches tool exactly); `./target/debug/deps/xtask-e9d837ce498c05b0 parity` -> 2 passed; `rg evidence parity_report.rs` -> only doc comments/test fixture |

## 1. One-paragraph verdict
The tool does exactly what it says: it counts status strings in docs/feature-matrix.toml and derives every number at run time — nothing is hardcoded, and its arithmetic is correct (I reproduced the exact 66/61/66/0-of-252 and 0-verified numbers with an independent tomllib recount). The answer to the assigned question: **"verified" qualifies by exact string-match on `status.<platform> == "verified"` and NOTHING else — the `evidence` field is never read by this tool** (rg proof). The 0/252 is HONEST, not understated: zero rows in the matrix claim `verified` (my recount: 252 rows, statuses are implemented/partial/not_started/blocked/unsupported only), and per the matrix's own frozen vocabulary "verified" requires tested-on-every-claimed-platform + audited evidence transitions, while the R4 evidence-resolution machinery is admittedly "OWED and unassigned debt" (matrix header) — so 0 is a structurally inevitable, truthfully reported number. The caveats: the tool would faithfully launder a fabricated "verified" string as parity (it is a reporter, not a verifier — the enforcement is feature-audit's job and that gate is currently failing on 2 missing evidence paths per baseline), and its parse-failure message breaks its own documented `parse:` line contract.

## 2. What actually works (verified)
- `./target/debug/xtask parity-report` -> exit 0; output table matches baseline xtask-parity-report.txt and my independent python3 recount of all 252 rows (windows: 66 implemented/99 not_started/46 partial/34 blocked/7 unsupported; linux 61 implemented; macos 66; web 0 implemented — 0 `verified` strings anywhere).
- `./target/debug/deps/xtask-e9d837ce498c05b0 parity` -> 2 passed / 0 failed (unit tests).
- Deterministic: BTreeMap category ordering, fixed column order, f64 formatting only for display.

## 3. Findings
F-001 [S3]
Where: tools/xtask/src/cmd/parity_report.rs:143-148 (and absence of any `evidence` read — whole file)
Code:
```
            if value == "implemented" || value == "verified" {
                stats.imp_ver[idx] += 1;
            }
            if value == "verified" {
                stats.verified[idx] += 1;
            }
```
What is wrong: "Verified by automated evidence" (the strict line's own label, quoting MP §1) is computed from the status string alone; the row's `evidence` field is never parsed, resolved, or checked by this tool. What qualifies as "verified" is therefore: a matrix author typing the string "verified". Why it matters: this is the headline metric of the whole program; the tool cannot distinguish an audited verification from a self-declaration. Today the number is honest only because the matrix happens to contain zero "verified" strings — the safety is the data's current restraint, not the tool. Mitigation (verified): the matrix header assigns anti-fabrication (R4 evidence resolution, rejected not_started→verified jumps) to feature-audit, and transitions are claimed to be "audited in git history"; feature-audit does check evidence paths (baseline: FAILURE, 2 missing). What correct looks like: parity-report should at minimum refuse to count a `verified` cell whose row lacks a non-empty `evidence` (cheap, std-only, closes the self-declaration hole inside its own file).
Confidence: HIGH — Verification: ran + rg (no `evidence` reads outside doc comments/fixtures).

F-002 [S3]
Where: tools/xtask/src/cmd/parity_report.rs:56-111
Code:
```
pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
```
What is wrong: `run()` has zero test coverage anywhere — no test in this file calls it, and tools/xtask/tests/cli.rs contains no parity case (rg: no match). The exit-code contract (0 / 1 parse / 2 error), `--root`/`--matrix` overrides, and the find_root walk are unexercised. Why it matters: docs and agents consume this tool's exit codes and machine line; a regression (e.g. exit 0 on a matrix that fails to parse) would pass every test in the tree. What correct looks like: a cli.rs case running `run(&["--matrix", "nonexistent.toml"])` asserting exit 1, and one happy-path asserting the machine line.
Confidence: HIGH — Verification: read + rg across unit and integration tests.

F-003 [S4]
Where: tools/xtask/src/cmd/parity_report.rs:16-17 vs 102-105
Code:
```
//! is exit 1 with a `parse:` line (same contract as feature-audit).
```
(and L103: `println!("parity: cannot parse {}: {err}", matrix_path.display());`)
What is wrong: The doc promises the failure line starts with `parse:` "same contract as feature-audit" (feature_audit.rs:139 prints `parse: {err}`); the code prints `parity: cannot parse ...`. The exit code (1) matches; the message contract does not. Why it matters: anything scraping for `parse:` lines (the documented contract) misses parity-report failures. What correct looks like: print `parse:` or fix the doc.
Confidence: HIGH — Verification: read both files.

F-004 [S4]
Where: tools/xtask/src/cmd/parity_report.rs:199-206
Code:
```
    let mut machine = String::from("parity: rows=");
    machine.push_str(&total.rows.to_string());
    for (i, platform) in PLATFORMS.iter().enumerate() {
        machine.push_str(&format!(" {platform}={}/{}", total.imp_ver[i], total.rows));
    }
```
What is wrong: The machine-readable line carries only the lenient implemented+verified share; the strict verified-only totals exist solely in the human markdown table. Why it matters: the single line most likely to be scraped quotes the generous metric by default; MP §1's binding definition is the strict one. What correct looks like: add `verified-{platform}=n/total` fields to the machine line.
Confidence: HIGH — Verification: read + live output.

F-005 [S4]
Where: tools/xtask/src/cmd/parity_report.rs:44 and 127-142
Code:
```
const PLATFORMS: [&str; 4] = ["windows", "linux", "macos", "web"];
```
What is wrong: The platform set is hard-coded; a status table naming any other platform (or a future 5th column) is silently ignored (L140-142 `continue`), and a row missing `status` silently counts only in the denominator. Both failure directions are silent — no warning, no error. Currently conservative (denominator-only inflation), so honest, but mute. What correct looks like: warn on unknown platform keys in a status table.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `run` | 56-111 | REAL | — | Executed live (exit 0, full table); arg parsing incl. `--root`/`--matrix` overrides; untested (F-002) |
| `compute_table` | 115-208 | REAL | — | Arithmetic independently reproduced on the real 252-row matrix; verified-only and machine lines correct |
| `cell` | 211-217 | REAL | — | `n/a` on zero denominator; unit-tested |
| `find_root` | 219-236 | REAL | — | Walk-up requiring Cargo.toml + docs/; different heuristic than sbom's product.toml walk (inconsistent but works) |
| `CategoryStats` | 47-54 | REAL | — | Plain accumulator |
| `Must` (test trait) | 242-266 | REAL | — | `unreachable!` ×2 (tree total) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| cell_formats_share | YES | Exact-string asserts incl. n/a |
| table_derives_from_a_small_matrix | YES | Asserts category rows, TOTAL row, verified-only row, and machine line against a hand-computed 3-row fixture with mixed statuses (implemented/verified/partial/not_started) |
Both executed: 2 passed. Gap: `run()`/exit codes (F-002).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every number is derived from the matrix file at run time ... nothing hardcoded" | parity_report.rs:12-13, docs/PARITY-REPORT.md §2 | REAL | Read: no constants but PLATFORMS (column names); live run matches independent recount |
| "A strict `verified`-only line is also emitted ... so nobody can mistake shipped-but-unverified work for done" | parity_report.rs:8-10 | REAL | Live output: "verified-only (MP §1 strict definition — automated evidence)" table, all 0/252 |
| "verified by automated evidence" (what the strict line measures) | parity_report.rs:9 quoting MP §1 | OVERSTATED | Tool never reads evidence (F-001); string-match only |
| "a matrix that does not parse is exit 1 with a `parse:` line (same contract as feature-audit)" | parity_report.rs:16-17 | FALSE (message shape) | Code prints "parity: cannot parse" (L103); feature_audit.rs:139 prints "parse:" (F-003) |
| Matrix: "not_started → verified jumps are REJECTED (anti-fabrication; the tool decides from evidence)" | docs/feature-matrix.toml header | UNVERIFIED / NOT THIS TOOL | No code in parity_report.rs rejects anything; matrix header admits the full feature-audit R4 is "OWED and unassigned debt" |

## 7. Dead / duplicated / unreachable code
None. `compute_table` is `pub` "for the fixture test" — used by the unit test; fine.

## 8. Dependencies & flags
std + `toml_lite` + `XtaskError` only (verified imports L19-24). No feature gating.

## 9. Open questions for the next agent
- docs/PARITY-REPORT.md claims run outputs "quoted verbatim" from 2026-11-10 — do its numbers still match today's matrix (my live run says the imp_ver totals do)? A drift check of that doc against a fresh run belongs to its auditor.
- The matrix rows for the first vertical (container-avi, mjpeg, pcm_s16le on linux) carry evidence strings citing code paths, and the first-vertical suite produces committed transcripts (automated evidence) — yet the rows say `implemented`, not `verified`. If that evidence is real, 0/252 is process-limited (promotion tooling missing), not evidence-limited; matrix row-chunk auditors should decide whether any implemented row actually meets the verified bar.
- Who is meant to run parity-report at all (no CI wiring)? If nobody, its numbers can silently rot relative to the matrix.
