# CLAIM AUDIT: docs/release-checklist.md
| Field | Value |
|---|---|
| Auditor | W17-A10 (recovered by W17-R1) |
| Date | 2026-11-24 |
| Claims extracted | 25 |
| Verdict counts | TRUE: 13 TRUE-AT-TIME/STALE-NOW: 4 FALSE-STALE: 3 UNVERIFIED: 3 CORROBORATED (in-tree, not re-executable): 2 |
| Internal contradictions found | 3 |

Note: W17-A10 also wrote this ledger at audit/docs/release-checklist.md.audit.md
(missing the `docs/` path segment); this recovery re-audit is the canonical
audit/docs/docs/<path>.audit.md copy, independently re-verified (syntax checks,
product-env run, YAML parse, SBOM JSON count, worklog greps re-run by W17-R1).

## 1. Blunt verdict
An unusually honest RC runbook: blockers carry owners, red rows stay red, §9
preserves struck history, and the cross-referenced evidence file
(docs/audit/docs-sweep-20261110.md §7, L193+) really contains every cited
output. Its two real defects are both temporal rot, not invention: (a) the
top line still asserts the release pipeline "has **never executed**" — its
own §9 plus worklog.md:1625/1937/1997 and release.yml's inline live-run
citations prove the opposite, and the doc's own §8 rule ("edit the row and
strike the blocker") was never applied to that line; (b) the **[verified
locally]** green rows for lint-literals/check-identity/sbom are RED on the
current tree (baseline full_gate.txt:60-75) — a checklist that says green
where the gate says red is an RC-day landmine.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "[verified locally] were run ... with recorded output (docs-sweep-20261110.md §7 has the outputs)" | :5-6 | TRUE | file exists (19,241 bytes); §7 header at :193 "Commands re-run this sweep (recorded outputs)" contains the bash -n loop, product-env, yaml-parse, xtask-gates outputs |
| 2 | "the CI half ... has **never executed** — CI billing is blocked on this private repository" | :12-14 | FALSE-STALE (unstruck) | §9:245 "EXECUTED: dry-runs + two tag runs; final green = 43466814633"; worklog.md:1625 "CI run 43378741271/43378741286/43378757248 ... the first full real execution"; release.yml:278 cites run 43378757248 in a post-mortem |
| 3 | "product-env.py --print → PRODUCT_VERSION=0.1.0-dev ... BIN_CLI=lumen, BIN_DESKTOP=lumen, exit 0" | :27 | TRUE-mechanism / values STALE / BIN_* needs flags | my run `python3 scripts/release/product-env.py --print` -> 13 vars: PRODUCT_VERSION=0.1.3, PRODUCT_VERSION_CODE=4, MSI_VERSION=0.1.3, exit 0; BIN_CLI/BIN_DESKTOP emitted only with --cli-package/--desktop-package (product-env.py:209-211, needs cargo — absent in sandbox) — sweep §7 records the flags; the checklist's compressed citation drops them |
| 4 | "Pre-release suffixes (-dev) are stripped ... MSI_VERSION=0.1.0" | :28 | TRUE (mechanism, code-verified) | product-env.py:96-100 `def msi_version` splits "-" prefix; current display_version 0.1.3 has no suffix (no strip observable today) |
| 5 | "sbom --check → no drift (157 components, subject Lumen 0.1.0-dev)" | :54 | FALSE-STALE | baseline full_gate.txt:70-75 `sbom: FAILED` ... `differs from the regenerated output`; my count: `python3 json cyclonedx.json` -> components: 158, subject Lumen 0.1.3 |
| 6 | "lint-literals (0 violations, 452 allowlisted) and check-identity (0 violations, 453) ... both PASS" | :53 | FALSE-STALE | baseline full_gate.txt:60-66 both `FAILED — 1 violation(s): apps/cli/src/host.rs:1799: identity` |
| 7 | "config-validator RED today — 4 violations, all the missing assets/brand/* files" | :55 | TRUE | baseline xtask-config-validator.txt:11-15 exactly 4 violations: icon.ico, icon.icns, icon-512.png, logo.svg |
| 8 | "full_gate.sh does not run this validator (its 9 steps are fmt/.../sbom)" | :55 | TRUE (validator absent) / FALSE-STALE (step count) | scripts/full_gate.sh has ELEVEN steps — :70 "=== 10. web-budget", :77 "=== 11. packaging gate"; no config-validator anywhere in the script (rg: no hit) |
| 9 | "8 sh (rc-smoke.sh + the 7 scripts/release/*.sh) ... node --check ... py_compile" | :51 | TRUE | my re-run: `ls scripts/release/*.sh | wc -l` -> 7; loop `bash -n` + `sh -n` over 8 files -> 8/8 OK; `node --check scripts/rc-smoke-web.mjs` -> OK; `python3 -m py_compile product-env.py gen-icon.py` -> OK |
| 10 | "Workflow YAML parses ... jobs: windows, macos-arm64, macos-x64, linux-x64, linux-arm64, release" | :52 | TRUE | my `python3 -c "import yaml; ..."` -> same 6-job list |
| 11 | "linux-arm64 ubuntu-24.04-arm (continue-on-error)" | :105 | TRUE | same parse -> `continue-on-error: True` |
| 12 | "release.yml — actionlint 1.7.12 clean, bash -n/sh -n on every script" | :94 | PARTIAL / UNVERIFIED (actionlint) | actionlint not in sandbox (no binary); bash -n/sh -n re-verified for the 8 release-side scripts by me; "every script" (packaging set) not re-run here |
| 13 | "checks 7-8 [web] red on main — F-1/F-2, lane A's fixes unmerged" | :76-77 | TRUE-AT-TIME / RESOLVED | baseline rc-smoke.txt:204-210 `web-ui-build: PASS`, `web-ui-browser-load: PASS` — wave-24 merge landed as §6.1 required |
| 14 | "wasm-web-build ... green" | :75 | ENV-DEPENDENT | baseline rc-smoke.txt:146 `wasm-web-build: FAILED (cargo build)` (missing wasm32 rust-std); addendum retry compiles to `Finished dev profile in 11.45s` with the component installed |
| 15 | "A skipped-web run (RC_SMOKE_SKIP_WEB=1) is an engine-only smoke ... not a passing RC smoke" | :81-83 | TRUE | rc-smoke.sh:143-144 `=== 7/8. web-ui: SKIPPED (RC_SMOKE_SKIP_WEB=1 — engine-only smoke, NOT a full RC smoke) ===` |
| 16 | "rc-smoke never downloads a browser; takes RC_SMOKE_CHROMIUM or newest PLAYWRIGHT_BROWSERS_PATH chromium" | :85-88 | TRUE | rc-smoke.sh:169-176 `CHROMIUM=${RC_SMOKE_CHROMIUM:-}`; find under `${PLAYWRIGHT_BROWSERS_PATH:-$HOME/.cache/ms-playwright}` |
| 17 | "Every archive/installer contains both binaries — lumen ... and lumen-desktop" | :108-110 | FALSE-STALE | release.yml:8-18: lumen-desktop is "a CI-only diagnostic as of w30.8 ... dropped from every user payload this workflow's scripts own"; AppRun/CFBundleExecutable exec the real shell (cross-ref W17-A08 build-rpm.sh F-001) |
| 18 | "evidence in docs/packaging/evidence/linux-lane-20261110.log — real .deb + real executable AppImage" | :119-120 | TRUE-existence / content UNVERIFIED-here | file exists (6,338 bytes); content verified by W17-A09's w26m-t2 audit (full verify run PASS for the older 2-binary era) — not re-verified in this recovery |
| 19 | "No update mechanism exists in v0.1.0 and none is claimed" | :147 | TRUE | rg updater/updateUrl/appcast apps/desktop-tauri/tauri.conf.json -> no hits; MSI UpgradeCode in product.toml:43 is replace-not-coinstall only |
| 20 | "dispatch-mode dry run ... wrote the notes (20-commit range, exit 0)" | :174-176 | CORROBORATED (recorded) | docs-sweep §7 records the exact command + result; not re-runnable here (script shells git; git prohibited) |
| 21 | "RC 1 EXECUTED ... final green = 43466814633 ... dry-run 43461062527" | :245,247 | CORROBORATED (worklog) | worklog.md:1937 "round-16 dispatched release dry-run (43461062527) is GREEN", :1997 "re-cut tag run (43466814633) is GREEN end to end"; direct GH verification impossible (no network) |
| 22 | "v0.1.0 published 17:00:58Z, 40 assets, body bounded at 120k chars (422 fix, round 18)" | :248 | CORROBORATED (worklog) | worklog.md:1999 "tag/v0.1.0 (17:00:58Z, 40 assets, body 118,906 chars ...)", :2006 "the 422 + the 120k bound" |
| 23 | "ADR-010 ... STILL PROPOSED — shipped on the user's explicit instruction" | :251 | TRUE (status) / instruction UNVERIFIED | docs/adr/README.md:20 register row "**proposed**"; issue #66 instruction not checkable (no network) |
| 24 | "the release ships honestly labeled-unsigned (*.UNSIGNED.txt sidecars)" | :252 | TRUE | sign-macos.sh:203-207 `write_sidecar "$target" ".UNSIGNED"`; worklog.md:2002 "UNSIGNED sidecars (honest labeled-unsigned)" |
| 25 | "lumen --version reads apps/cli/Cargo.toml (env!(CARGO_PKG_VERSION)) ... NOT wired to product.toml ... [PROPOSAL]" + "today they do (0.1.0 vs 0.1.0-dev)" | :30 | TRUE (mechanism) / drift FALSE-NOW | apps/cli/src/help.rs:99-101 [PROPOSAL: product.toml identity ...] + `format!("lumen {}\n", env!("CARGO_PKG_VERSION"))`; today ALIGNED: product.toml:27 display_version="0.1.3" == apps/cli/Cargo.toml:34 == apps/desktop/Cargo.toml:20 (0.1.3) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "has **never executed** — CI billing is blocked" (:12-14, unstruck) | §9:245 "EXECUTED: dry-runs + two tag runs"; worklog.md:1625/1937/1997; release.yml:278/1138/1145 live-run citations | §9 + worklog are true — the top line is stale-false and violates the doc's own §8 strike rule (§3.2's w24/c row WAS properly struck with ~~...~~; the top line was not) |
| "full_gate.sh (its 9 steps ... sbom)" (:55) | scripts/full_gate.sh:70-82 has steps 10 (web-budget) and 11 (packaging) | The script is truth; the enumeration is a 9-step-era fossil. §4.1's "full_gate.sh step 9" for sbom still matches, so the fossil is only the count |
| "today they do (0.1.0 vs 0.1.0-dev)" (:30) | product.toml:27 = 0.1.3, apps/cli:34 = 0.1.3, apps/desktop:20 = 0.1.3 | Aligned now — the drift was cured by the §0 procedure in later waves |

## 4. Fabrication / inflation indicators
NONE. The opposite: red rows kept red (1.6 config-validator), blockers carry
owners + next steps, §9 strikes in place and preserves history, and the
backing evidence file exists with the outputs. The single honesty gap is
maintenance: green [verified locally] rows were never re-validated as the
tree moved (claims 5, 6, 8, 17, 25).

## 5. Matrix rows
N/A — not a matrix chunk.
