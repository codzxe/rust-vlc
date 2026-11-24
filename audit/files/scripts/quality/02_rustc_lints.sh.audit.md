# AUDIT: scripts/quality/02_rustc_lints.sh
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Date | 2026-11-24 |
| Lines / functions | 65 / 2 phases (lint-escalation cargo check; jsonl histogram in python) |
| File verdict | REAL (reporter-grade; one silent coverage hole) |
| Reachable from shipped entry point? | YES — scripts/quality/99_run_all.sh:43 `J 02_rustc_lints.sh` (quality tier, nightly) → .github/workflows/audit.yml:127. Not part of the release binary lane. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | bash -n (read-level review; NOT executed — runs cargo check with cache-busting RUSTFLAGS and writes to docs/audit/reports) |

## 1. One-paragraph verdict
A real rustc lint-escalation reporter: 11 lints escalated through RUSTFLAGS over `cargo check --workspace --all-targets --all-features`, JSON output histogrammed by a careful python parser that tolerates malformed lines and prints sample locations. As a REPORTER its exit semantics are correct (cargo's rc is echoed, not propagated — the tier design). The defect that matters: the escalation silently excludes `platform-linux` and `platform-macos` with no comment and no doc — two of the three platform crates escape all eleven lints (while platform-windows is not excluded), so the "workspace" lint claim is narrower than advertised.

## 2. What actually works (verified)
— verified by read: LINTS array (L7-19) -> FLAGS (L20-21) -> `RUSTFLAGS="$FLAGS" cargo check` (L26-30); rc captured post-redirect (L31) and echoed (L32) — honest rc reporting.
— verified by read: histogram python (L35-64) — skips non-JSON lines, requires `reason == "compiler-message"`, counts by lint code at warning/error level, primary-span locations, `most_common()` ordering — a genuine diagnostic reducer, not a pretty-printer.
— verified by wiring: quality tier invokes it (99_run_all.sh:43) and audit.yml:127 runs the quality tier nightly.
— bash -n clean (trivial file).

## 3. Findings
F-001 [S3]
Where: scripts/quality/02_rustc_lints.sh:26-29
Code:
```
RUSTFLAGS="$FLAGS" cargo check \
  --workspace --all-targets --all-features \
  --exclude platform-linux --exclude platform-macos \
```
What is wrong: `platform-linux` and `platform-macos` are excluded from the lint escalation with no comment, no doc, and no symmetry (`platform-windows` is not excluded). The header claims "T02 - rustc lint escalation: dead code, unreachable pub, unused deps, lifetime elision" over the workspace; two platform crates escape all 11 lints. Why it matters: dead code / unused deps / `let _` drops in the excluded crates are never escalated by this reporter — a coverage hole in the quality program's stated scope. What correct looks like: either a comment stating why (OS-specific unsafe modules that cannot compile under the escalated set on Linux?) or linting them via `--target` checks per crate. Confidence: HIGH — Verification: read (exclusion literal in the command).
F-002 [S4]
Where: scripts/quality/02_rustc_lints.sh:29
Code:
```
  --message-format=json 2>/dev/null \
```
What is wrong: stderr is discarded. Compile ERRORS survive in the jsonl stream (compiler-message reason), but cargo-level failures (registry/network, lockfile errors) land only on stderr and vanish from the saved evidence — a failed run leaves a truncated jsonl plus an `exit=$rc` line with no cause. What correct looks like: `2> "$OUT/rustc-lints.stderr"` next to the jsonl. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: scripts/quality/02_rustc_lints.sh:31-32
Code:
```
rc=$?
say "exit=$rc  raw=$OUT/rustc-lints.jsonl"
```
What is wrong: nothing for a reporter (exit code intentionally not propagated; the tier's J contract). But the script's FINAL exit status is the histogram python's (L35), so a cargo-check failure is invisible to `99_run_all.sh`'s FAILED ledger for this script — it would only appear in the human-readable "exit=$rc" line. The tier design accepts this (J = record-not-fail), but the recording is into stdout, not the FAILED array. Confidence: HIGH — Verification: read. Severity: S4 because quality tier is nightly-reporter by design.

## 4. Per-function table (file <100 LOC; the two phases)
| Phase | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| lint flag assembly | 7-21 | REAL | — | 11 lints; deterministic |
| cargo check run | 26-32 | REAL | S3 | platform exclusions (F-001); stderr discard (F-002); rc echoed |
| histogram | 35-64 | REAL | — | robust jsonl reducer with sample locations |

## 5. Test honesty
No tests for the script. Its own evidence artifact (rustc-lints.jsonl + histogram stdout) is the output; honesty depends on the run. The lint list is real rustc lint names (all 11 are valid `rustc -W` lints).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "T02 - rustc lint escalation: dead code, unreachable pub, unused deps, lifetime elision" | 02_rustc_lints.sh:2 | SHALLOW | implemented, but excludes 2 platform crates silently (F-001) |
| "WARNING: changing RUSTFLAGS invalidates the whole build cache. Run once per session." | :3 | REAL (honest cost note) | RUSTFLAGS is set at L26 |

## 7. Dead / duplicated / unreachable code
None. Every line is live.

## 8. Dependencies & flags
cargo check (heavy — not executable under the audit load discipline); writes `$OUT/rustc-lints.jsonl` (docs/audit/reports). `--all-features` pulls the full feature set including codec-h264-sw — full-coverage intent, good.

## 9. Open questions for the next agent
- WHY are platform-linux/macos excluded? If they fail under the escalated lints, that's a finding about those crates; if laziness, it's a gate hole. UNVERIFIED (cannot run cargo check under the load discipline).
- Last committed rustc-lints.jsonl/histogram output: not present in docs/audit/reports in this tree (only docs-sweep + panic-baseline exist) => the tier's evidence has never been committed. UNVERIFIED whether it ever ran.
