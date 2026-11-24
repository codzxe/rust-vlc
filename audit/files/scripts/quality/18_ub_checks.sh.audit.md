# AUDIT: scripts/quality/18_ub_checks.sh
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Date | 2026-10-30 |
| Lines / functions | 64 / 6 lanes (overflow, careful, miri, asan/ubsan/tsan, usage, epilogue) |
| File verdict | SHALLOW — real instruments, zero enforcement path |
| Reachable from shipped entry point? | YES (CI): audit.yml:276 (miri, `|| true`), :417/:419 (asan/ubsan, `|| true`); 99_run_all.sh:53 `J 18_ub_checks.sh overflow` (dynamic tier, reporter) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 3 |
| Commands run | `bash -n` (exit 0); `rustup toolchain list` (no output in this shell — lanes would not_run here); `rg`/`sed` over audit.yml + 99_run_all.sh (exit 0). Lanes NOT executed: cargo test rebuild under new RUSTFLAGS = cargo-build-budget. |

## 1. One-paragraph verdict
The overflow lane is a real mechanism (whole-workspace `cargo test` under `-C overflow-checks=on -C debug-assertions=on`), and the miri/asan/ubsan lanes invoke the real tools — but the script has no failure propagation at all: every lane ends in a `| tee | tail` pipeline whose status is at best printed (`say "exit=$?"`, overflow only, never acted on), and the script's last command is a heredoc `cat`, so the script always exits 0. Every CI consumer then bolts on `|| true` (audit.yml:276, :417, :419) or `continue-on-error` (sanitizers job, audit.yml:412). Net effect across the repo: a UB finding — the exact class of bug this lane exists to catch — can never fail any gate; it can only be read by a human in a `tail -30` or an uploaded artifact. Plus the `--exclude input` oddity: the file-I/O crate is excluded from every UB lane without explanation.

## 2. What actually works (verified)
- `bash -n scripts/quality/18_ub_checks.sh` → exit 0 (syntax clean).
- Tool gating is honest: careful → `need cargo-careful` else not_run recorded; miri/asan/ubsan → nightly check via `rustup toolchain list | grep -q nightly` else not_run recorded (common.sh:30-34 appends to not-run.txt). In this shell `rustup toolchain list` produced no output → all three lanes would honestly not_run here.
- The RUSTFLAGS overflow lane is the correct stable-toolchain approach: `RUSTFLAGS="-C overflow-checks=on -C debug-assertions=on" cargo test "${EXCL[@]}" --all-features --no-fail-fast` (:16-18) — this genuinely surfaces arithmetic overflow panics as test failures in the log.

## 3. Findings
F-001 [S2]
Where: scripts/quality/18_ub_checks.sh:16-20, 33, 48-49, 56-63
Code:
```
      cargo test "${EXCL[@]}" --all-features --no-fail-fast \
      2>&1 | tee "$OUT/ub/overflow.log" | tail -30
    say "exit=$?"
```
What is wrong: with `set -o pipefail` the pipeline status equals cargo's exit code, but it is only printed for the overflow lane and never acted on; the miri (`cargo +nightly miri test ... | tee ... | tail -30`, :33) and sanitizer (:48-49) lanes don't even print it; and the script's final statement is the `cat <<'TXT'` heredoc (:57-63) — so the script's exit code is `cat`'s, i.e. 0, unconditionally. Why it matters: a detected UB/overflow does not fail anything, ever — not the tier (99_run_all's J records only non-zero rcs, which never occur), not the CI job (see F-002). What correct looks like: capture `$rc` per lane, `exit $rc` at the end (or at least after each lane). Confidence: HIGH — Verification: read (pipeline/exit-code semantics; script not executed — build budget).

F-002 [S2]
Where: .github/workflows/audit.yml:276, 417, 419 (consumers of this script)
Code:
```
            bash scripts/quality/18_ub_checks.sh miri "$c" || true
```
(and :417 `... asan codecs || true`, :419 `... ubsan codecs || true`; job-level `continue-on-error: true` at :412)
What is wrong: even the zero-enforcement script above is further neutralized with `|| true` and job-level continue-on-error. Why it matters: the sanitizer job ("AddressSanitizer on crates/codecs (rav1d C ABI)") can report a detected memory error and the workflow still shows green. This is the spec's "steps that cannot fail" anti-pattern. Mitigation present: artifacts are uploaded `if: always()` (audit.yml:423-427, path docs/audit/reports/ub/) so raw logs survive — evidence yes, enforcement no. Confidence: HIGH — Verification: read (sed quoted above).

F-003 [S3]
Where: scripts/quality/18_ub_checks.sh:11
Code:
```
EXCL=(--workspace --exclude platform-linux --exclude platform-macos --exclude input)
```
What is wrong: `input` — the crate whose job is bounded reads, seek math, and truncation handling (integer-boundary territory the overflow lane exists for) — is excluded from the overflow, careful, and sanitizer lanes with no comment explaining why. Why it matters: the UB program never covers the file-I/O attack surface; if the exclusion exists because input's tests intentionally panic under debug assertions, that exception is undocumented and unbounded (it silently drops ALL input-crate coverage). What correct looks like: an inline comment + a targeted re-inclusion plan (or a lane that runs input with the known-failing tests filtered). Confidence: HIGH — Verification: read.

F-004 [S4]
Where: scripts/quality/18_ub_checks.sh:48
Code:
```
RUSTFLAGS="-Zsanitizer=$san" cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu \
```
What is wrong: hardcoded x86_64 target — the repo ships arm64 CI lanes (ubuntu-24.04-arm) yet the sanitizer lanes cannot run there. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: scripts/quality/18_ub_checks.sh:33, 20
Code:
```
cargo +nightly miri test "${PKG_FLAG[@]}" 2>&1 | tee "$OUT/ub/miri.log" | tail -30
```
What is wrong: the miri lane ignores `${EXCL[@]}` and `--all-features` (inconsistent with every other lane — different coverage envelope); with empty `crate` arg it would miri the entire workspace (hours); and `say "any 'attempt to add with overflow' / 'panic' line above is a finding"` (:20) defines "finding" as a line a human notices in a 30-line tail. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Block | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| env/source | 1-11 | REAL | — | `set -uo pipefail`; EXCL with the F-003 oddity |
| overflow lane | 14-21 | REAL instrument / SHALLOW enforcement | S2 | real RUSTFLAGS mechanism; rc printed only |
| careful lane | 22-29 | REAL (gated) | — | `need` + not_run honest |
| miri lane | 30-37 | REAL (gated) | S4 | coverage-envelope inconsistency |
| asan/ubsan/tsan | 38-50 | REAL (gated) | S4 | Linux-only + x86_64 hardcode |
| usage + epilogue | 51-63 | REAL | — | heredoc makes final exit 0 (F-001) |

## 5. Test honesty
No self-tests. Mutation reasoning: if RUSTFLAGS were dropped entirely, no run anywhere would notice — the lane's logs would look identical for a non-overflowing suite.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "T18 - UB, overflow, memory and race detection lanes" (title) | 18_ub_checks.sh:2 | **SHALLOW** | instruments real, enforcement zero (F-001/F-002) |
| "The overflow lane above is the cheap stable-only version of that check." | :61-62 | REAL | `--all-features --no-fail-fast` under overflow-checks — the correct cheap variant (read) |

## 7. Dead / duplicated / unreachable code
None dead. The `tsan` (race) lane is wired to nothing in CI (audit.yml only runs miri/asan/ubsan) — reachable only by manual invocation. UNVERIFIED whether any committed doc promises tsan coverage.

## 8. Dependencies & flags
cargo (stable for overflow), cargo-careful 0.4.10 (optional), nightly toolchain + miri/rust-src components for miri/asan/ubsan, Linux + x86_64 for sanitizers. Excludes: platform-linux, platform-macos, input.

## 9. Open questions for next agent
- Why is `input` excluded (F-003)? If an input test panics under debug-assertions by design, that's a finding on its own — the exclusion hides it repo-wide.
- audit.yml's `|| true` list (miri, asan, ubsan, 20_formal) — whoever audits audit.yml should count them all; every one is a gate that cannot fail.
