# AUDIT: scripts/quality/09_features.sh
| Field | Value |
|---|---|
| Auditor | W17-A08 (recovered by W17-R2) |
| Date | 2026-11-24 |
| Lines / functions | 39 / 0 (3 pipeline sections: 9a, 9b, 9c + 1 python heredoc) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — audit tooling lane: audit.yml:168 installs tools, audit.yml:171 `99_run_all.sh dynamic` -> 99_run_all.sh:54 `J 09_features.sh`. Nightly cron / dispatch only; no runner ever billed => never executed (UNVERIFIED) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `bash -n` -> 0; pipefail rc probe -> 3 (propagates); 9b filter recomputed over tree manifests -> 2 crates (input, media-probe); rg for log consumers -> script itself only |

## 1. One-paragraph verdict
Real cargo-hack feature-combination harness wired into the CI dynamic tier, but by construction it can never fail anything: it runs under the `J` reporter lane, only one of its three subsections even echoes an exit code, and zero output logs are committed anywhere. Its actual subject matter in today's tree is two crates with one feature each — the machinery is real, the blast radius is a pair of boolean toggles, and there is no evidence it has ever run.

## 2. What actually works (verified)
— verified by `bash -n scripts/quality/09_features.sh` -> exit 0 (syntax clean).
— verified by pipefail probe `(exit 3) | tee f | tail -1; echo $?` -> `3`: with `set -o pipefail` (common.sh:6, re-set at :3), `say "each-feature exit=$?"` (:14) does capture cargo-hack's failure rc.
— verified by recomputing the 9b filter over every Cargo.toml in the tree (python3/tomllib): exactly 2 crates pass `0 < len(feats) <= 6`: `input` [contract], `media-probe` [contract] — so 9b's powerset subjects exist and the filter logic is sound; no crate has >6 features (nothing silently escapes the "<= 6" cap today).
— verified wiring: `need cargo-hack` -> 00_install_tools.sh:11 pins cargo-hack@0.6.46, installed in the dynamic job (audit.yml:161-171).

## 3. Findings
F-001 [S3]
Where: scripts/quality/09_features.sh:2 vs 99_run_all.sh:54
Code: `# T09 - feature powerset. CI only ever builds --all-features; this finds the rest.` / `    J 09_features.sh`
What is wrong: the mission is delivered as logs only. `J` = REPORTER: "its exit code is recorded but does not fail the tier" (99_run_all.sh:15-16). A broken feature combination (the exact thing T09 exists to find) surfaces as a `say` line inside an uncommitted log; the dynamic tier's only gate is `JG 26_complexity.sh` (99_run_all.sh:56). Why it matters: a CI section advertised as finding feature-combination breakages cannot fail CI — reporter theatre. What correct looks like: `JG 09_features.sh` or a FAILED-ledger append per non-zero section rc.
Confidence: HIGH — Verification: read (99_run_all.sh:14-22 quoted, sed-verified).

F-002 [S3]
Where: scripts/quality/09_features.sh:7-9 vs :30 and :35
Code: `# NOTE: --no-dev-deps is deliberately NOT used: cargo-hack rewrites the real
# Cargo.toml while running, and manifests are Lead-owned in this repo. We accept
# the slower run instead of mutating manifests during an audit.` / `  cargo hack check --feature-powerset --depth 2 --no-dev-deps -p "$name" \`
What is wrong: the stated invariant ("We accept the slower run instead of mutating manifests during an audit") is violated by 2 of the script's own 3 invocations — 9b (:30) and 9c (:35) both pass `--no-dev-deps`, the exact flag the NOTE refuses. cargo-hack restores the manifest on normal exit only; a timeout/kill mid-9b/9c leaves a rewritten Lead-owned Cargo.toml on disk. Why it matters: the comment documents a protection the code does not provide, and the latent failure mode is repo mutation by an audit script. What correct looks like: drop `--no-dev-deps` at :30/:35 or rewrite the NOTE to scope it to 9a only and accept the mutation risk explicitly.
Confidence: HIGH — Verification: read (all lines sed-verified).

F-003 [S3]
Where: scripts/quality/09_features.sh:10-12
Code: `head1 "9a. --each-feature (linear, all crates)"
cargo hack check --each-feature --workspace \
  --exclude platform-linux --exclude platform-macos \`
What is wrong: the section header says "all crates" while the command silently excludes platform-linux and platform-macos; 9c (:35) does the same without any comment. 9b's python filter (:23) excludes ALL `platform-*`. platform-windows is never excluded from 9a/9c. Today the platform crates carry zero `[features]` (crates/platform-linux/Cargo.toml:39-46 — no features table), so the practical coverage loss is ~zero, but the header is factually FALSE and the scope cut is unexplained — the same uncommented platform-linux+platform-macos pair flagged S3 in the 02_rustc_lints.sh audit.
Confidence: MEDIUM — Verification: read; whether the excluded crates actually fail `cargo hack check` on the linux runner is UNVERIFIED (cargo-build-budget).

F-004 [S4]
Where: scripts/quality/09_features.sh:30-31, 35-36
Code: `  cargo hack check --feature-powerset --depth 2 --no-dev-deps -p "$name" \
    2>&1 | tee -a "$OUT/features-powerset.log" | tail -5` (and 9c, no rc capture at all)
What is wrong: 9a echoes its rc (:14); 9b's loop and 9c capture nothing — a failing powerset or no-default-features check produces zero rc visibility even inside the tier log. Inconsistent self-reporting.
Confidence: HIGH — Verification: read.

F-005 [S4]
Where: scripts/quality/09_features.sh:27
Code: `while read -r name ncombo feats; do`
What is wrong: `feats` is read and never used; the feature list is carried in the .txt purely for display via `cat` (:26). Dead variable.
Confidence: HIGH — Verification: read.

## 4. Section table (39 loc, <100 — brief)
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| guard + NOTE | 3-9 | REAL | — | need cargo-hack -> honest not_run record when absent |
| 9a each-feature | 10-14 | REAL-UNPROVEN | F-001/F-003 | rc echoed, not gated |
| 9b powerset filter+loop | 16-32 | REAL-UNPROVEN | F-002/F-004 | 2 subject crates exist today |
| 9c no-default | 34-36 | REAL-UNPROVEN | F-002/F-003/F-004 | rc dropped entirely |
| verdict | 38-39 | — | — | prints log paths only |

## 5. Test honesty
No tests exist for this script. The script itself is the test-harness. Execution evidence: docs/audit/ contains only docs-sweep-20261110.md and panic-baseline.json — no features-*.log/features-crates.txt ever committed => no run has ever been recorded in-tree (UNVERIFIED whether it ran on a billing-blocked runner).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "CI only ever builds --all-features" | 09_features.sh:2 | TRUE for test lane, FALSE for release lane | ci.yml:223/233/234 all --all-features; release lane builds default features (AGENT-SPEC §7) |
| "this finds the rest" | 09_features.sh:2 | SHALLOW | finds = logs only, J reporter, nothing gated (F-001) |
| "9a ... (linear, all crates)" | 09_features.sh:10 | FALSE | two platform crates excluded (F-003) |
| "--no-dev-deps is deliberately NOT used" | 09_features.sh:7 | FALSE | used at :30 and :35 (F-002) |

## 7. Dead / duplicated / unreachable code
`feats` loop var (F-005). No other dead code. Script unreachable from shipped lumen-cli/desktop entry points by design (audit tooling).

## 8. Dependencies & flags
cargo-hack@0.6.46 (pinned, 00_install_tools.sh:11), installed only in audit.yml tool-install steps. Writes to docs/audit/reports (never committed). No feature flags.

## 9. Open questions for the next agent
- Does `cargo hack check --workspace` fail on platform-linux/platform-macos on a linux runner (the unexplained exclusion)? Would need a runner (UNVERIFIED).
- Why is platform-windows never excluded while its two siblings are?
