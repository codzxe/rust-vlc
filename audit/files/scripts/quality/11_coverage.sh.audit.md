# AUDIT: scripts/quality/11_coverage.sh
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Date | 2026-10-30 |
| Lines / functions | 53 / 4 blocks (env+excludes, suite run, per-crate loop, zero-coverage report) |
| File verdict | SHALLOW — real llvm-cov run, exit plumbing broken on every path |
| Reachable from shipped entry point? | YES (CI): audit.yml quality tier (nightly) → 99_run_all.sh:46 `J 11_coverage.sh` |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `bash -n` (exit 0); `python3 -c 'raise SystemExit'` → rc 0 (proof for F-001); `rg` over input test imports (exit 0). Coverage run NOT executed: needs cargo-llvm-cov + full rebuild (cargo-build-budget). |

## 1. One-paragraph verdict
The measurement core is genuine (`cargo llvm-cov nextest --workspace --all-features` with JSON+LCOV outputs feeding T14/CRAP), and it is the one script in this set that honestly records its exclusions in not-run.txt with reasons — but its exit-code plumbing is broken three ways: the tool-missing path exits 0, the llvm-cov pipeline's status is printed and dropped, and — the subtle one — the cov.json parse-failure branch ends in `raise SystemExit` with no argument, which I proved exits 0, so even the script's own error signal is a no-op. The script cannot fail; it is a reporter that can only ever report success to the tier.

## 2. What actually works (verified)
- `bash -n scripts/quality/11_coverage.sh` → exit 0.
- Exclusion honesty (read): `not_run "coverage:input" "crates/input test target does not compile (F-002) - excluded from coverage, not scored as 0%"` (:13) — the exact comment pattern 18_ub_checks.sh lacks for the same exclusion.
- `raise SystemExit` rc proof (executed): `python3 -c 'raise SystemExit'; echo rc=$?` → `sys-exit-none-rc=0` — Python semantics: SystemExit with None status exits 0.

## 3. Findings
F-001 [S3]
Where: scripts/quality/11_coverage.sh:39-40
Code:
```
    print("cannot parse cov.json:", e); raise SystemExit
```
What is wrong: `raise SystemExit` with no argument exits with status 0 — proven by execution above. The only intended failure signal in the script is therefore a success exit. Why it matters: a corrupt/unparseable cov.json (e.g. llvm-cov failed upstream and wrote an empty/partial file) prints an error and the script still ends rc 0; the J wrapper records nothing. What correct looks like: `raise SystemExit(1)`. Confidence: HIGH — Verification: ran.

F-002 [S2]
Where: scripts/quality/11_coverage.sh:18-22
Code:
```
cargo llvm-cov nextest --workspace --all-features "${EXCLUDES[@]}" \
  --json --output-path "$OUT/cov.json" \
  --lcov --output-path "$OUT/lcov.info" \
  2>&1 | tee "$OUT/coverage.log" | tail -30
say "exit=$?"
```
What is wrong: same swallow pattern as T23/T18 — pipefail gives the pipeline cargo's rc, `say "exit=$?"` prints it, nothing enforces it; the script continues to blocks 11b/11c whose failures are also discarded (`2>/dev/null` at :30, empty `out` printed as a bare crate name). What correct looks like: capture rc, `exit $rc` after 11a (and per-crate rc collection in 11b). This is a J reporter by design, so the tier does not fail on it — but the tier's FAILED[] recording also never fires because the script is always green. Confidence: HIGH — Verification: read.

F-003 [S3]
Where: scripts/quality/11_coverage.sh:6-7
Code:
```
need cargo-llvm-cov || exit 0
rustup component add llvm-tools-preview >/dev/null 2>&1
```
What is wrong: missing tool → recorded not_run, exit 0 (a coverage tier that silently vanishes from the report when the tool is absent); and the llvm-tools-preview install has ALL output and errors discarded — a failed component add is invisible until the later llvm-cov invocation fails (which itself cannot propagate, per F-002). Confidence: HIGH — Verification: read.

F-004 [S4]
Where: scripts/quality/11_coverage.sh:10-12 and crates/input/tests/file_stream.rs:7-15
Code:
```
# F-002: crates/input/tests/file_stream.rs has a denied unused import.
EXCLUDES=(--exclude platform-linux --exclude platform-macos --exclude input)
```
What is wrong: the F-002 rationale is stale or unverifiable today: the phase-3 committed gate log shows input's file_stream tests compiling and passing 9/9 (phase3-exit-workspace-test.log:2025-2035, "Running tests/file_stream.rs … running 9 tests … ok") — though that run predates --all-features differences; a compile check under the workspace's deny lints was not affordable in this audit (cargo-build-budget). The exclusion therefore possibly excludes a healthy crate from coverage on a fixed defect. Confidence: LOW (staleness) — Verification: UNVERIFIED (needs `cargo check -p input --tests --all-features`).

F-005 [S4]
Where: scripts/quality/11_coverage.sh:30-31
Code:
```
  out=$(cargo llvm-cov report --package "$name" --summary-only 2>/dev/null | tail -5)
```
What is wrong: stderr discarded per crate; a failing per-crate report degrades to an empty string with no marker, so the per-crate table can silently omit coverage data for any crate. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| env + exclusions | 1-16 | REAL | — | honest not_run records (the model 18 lacks) |
| 11a suite run | 17-22 | REAL instrument / SHALLOW enforcement | S2 | rc printed only |
| 11b per-crate loop | 24-32 | REAL | S4 | per-crate stderr discarded |
| 11c zero-coverage | 34-52 | REAL | S3 | useful list; parse-failure exits 0 (F-001) |

## 5. Test honesty
No self-tests. Mutation reasoning: gutting the JSON parsing to `rows=[]` would print "files with ~0% line coverage: 0" and exit 0 — nothing detects it.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "T11 - coverage per crate (llvm-cov). Produces LCOV for T14 (CRAP)." | :2 | **REAL-UNPROVEN** | the llvm-cov/LCOV plumbing is real code; whether T14 consumes it verified by the T14 auditor (14_crap.sh not in my assignment) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
cargo-llvm-cov (need-gated), llvm-tools-preview (silently installed), cargo nextest integration via `llvm-cov nextest`, python3 JSON parsing. Excludes: platform-linux, platform-macos, input, _template (in the loop).

## 9. Open questions for next agent
- Does 14_crap.sh (CRAP) actually read `$OUT/lcov.info`? If not, the "Produces LCOV for T14" claim is dead-letter.
- Re-run `cargo check -p input --tests --all-features` when budget allows to settle F-004 staleness.
