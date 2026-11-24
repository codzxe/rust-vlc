# CLAIM AUDIT: docs/release-checklist.md
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Claims extracted | 24 |
| Verdict counts | TRUE: 8 TRUE-AT-TIME/STALE-NOW: 4 FALSE-STALE: 2 UNVERIFIED: 6 TRUE-PARTIAL: 4 |
| Internal contradictions found | 2 |

## 1. Blunt verdict
This is an unusually honest RC runbook: blockers carry owners and next steps,
failures are recorded in red, and the cross-referenced evidence file
(docs/audit/docs-sweep-20261110.md §7) really contains the claimed outputs.
The doc's central hazard is STALENESS: its **[verified locally]** rows for
lint-literals (0 violations), check-identity (0 violations), and SBOM (no
drift, 157 components) are all RED on the current tree (orchestrator baseline:
1+1 violations at apps/cli/src/host.rs:1799, SBOM DRIFT with 158 components) —
the tree rotted after 2026-11-10 and nobody struck those rows. A checklist
that says green where the gate says red is an RC-day landmine. Second defect:
the §1.6 "full_gate.sh ... its 9 steps" enumeration is false — the script now
has 11 steps (web-budget #10, packaging #11). Same disease as ci.yml's stale
"six steps" parity comment.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "[verified locally] were run on this branch with recorded output (docs-sweep-20261110.md §7)" | :5-6 | TRUE | file exists; §7 (line 204) contains the cited outputs verbatim |
| 2 | "the CI half ... has never executed — CI billing is blocked" | :12-14 | TRUE-AT-TIME/STRIKEN | §9:244-248 records billing unblocked 2026-11-12 + runs 43466814633/43461062527 — no git to verify runs; doc itself strikes the row |
| 3 | "product-env.py --print → PRODUCT_VERSION=0.1.0-dev ... BIN_CLI=lumen" | :27 | TRUE (mechanism) / values STALE | `python3 scripts/release/product-env.py --print` -> 13 vars incl. PRODUCT_VERSION=0.1.3, VERSION_CODE=4, MSI_VERSION=0.1.3, exit 0; BIN_CLI/BIN_DESKTOP only with `--cli-package/--desktop-package` flags (product-env.py:209-211) — that invocation died "cargo not found" (no cargo binary in this sandbox) -> UNVERIFIED for BIN_* values |
| 4 | "MSI ProductVersion ... `-dev` stripped ... MSI_VERSION=0.1.0" | :28 | TRUE-mechanism | my run: display 0.1.3 -> MSI_VERSION=0.1.3 (strip verified) |
| 5 | "sbom --check → no drift (157 components, subject Lumen 0.1.0-dev)" | :29 | FALSE-STALE | baseline full_gate.txt:70 `sbom: FAILED` + `drift: ...differs from the regenerated output`; components now 158 |
| 6 | "The two versions can drift; today they do (0.1.0 vs 0.1.0-dev)" | :30 | TRUE-AT-TIME/FALSE-NOW | now aligned: product.toml display_version 0.1.3 == apps/cli 0.1.3 == apps/desktop 0.1.3 (all rg-verified) |
| 7 | "8 sh" (rc-smoke + 7 scripts/release/*.sh) | :51 | TRUE | `ls scripts/release/*.sh` -> 7 .sh files |
| 8 | "jobs: windows, macos-arm64, macos-x64, linux-x64, linux-arm64, release" | :52 | TRUE | `python3 yaml.safe_load(release.yml)` -> identical 6-job list |
| 9 | "lint-literals (0 violations, 452 allowlisted) and check-identity (0 violations, 453)" | :53 | FALSE-STALE | baseline full_gate.txt:60-66: both FAILED, 1 violation each `apps/cli/src/host.rs:1799` |
| 10 | "config-validator RED today — 4 violations, all the missing assets/brand/*" | :55 | TRUE | baseline xtask-config-validator.txt: `result: FAILURE (4 violation(s))`, icon.ico/icns/icon-512.png/logo.svg |
| 11 | "full_gate.sh does not run this validator (its 9 steps are fmt/.../sbom)" | :55 | FALSE-STALE (9-step count) / TRUE (validator absent) | scripts/full_gate.sh:10-82 has ELEVEN steps (web-budget #10, packaging #11); config-validator still not among them (rg: no hit) |
| 12 | "checks 7-8 [web build] red on main — F-1/F-2" | :76-77 | TRUE-AT-TIME/RESOLVED | baseline rc-smoke.txt: `web-ui-build: PASS`, `web-ui-browser-load: PASS` on current tree — wave-24 merge landed as §6.1 required |
| 13 | "wasm-web-build ... green" | :75 | ENV-DEPENDENT | baseline rc-smoke.txt: `wasm-web-build: FAILED (cargo build)` — missing wasm32 rust-std in sandbox; spec §13 addendum: passes once component installed |
| 14 | "linux-arm64 ubuntu-24.04-arm (continue-on-error)" | :105 | TRUE | yaml parse -> `continue-on-error: True` |
| 15 | "release.yml — actionlint 1.7.12 clean, bash -n/sh -n on every script, YAML parse" | :94 | TRUE-PARTIAL | YAML parse: I verified; actionlint/bash -n runs UNVERIFIED (no actionlint; sh -n on release scripts not re-run by me) |
| 16 | "Every archive/installer contains both binaries — lumen and lumen-desktop" | :108-110 | UNVERIFIED | would need artifact inspection; current version set exports BIN_CLI/BIN_DESKTOP only with flags (no cargo here) |
| 17 | "evidence in docs/packaging/evidence/linux-lane-20261110.log — real .deb + real executable AppImage" | :119-120 | UNVERIFIED | evidence file not re-verified this audit (other agent's domain) |
| 18 | "No update mechanism exists in v0.1.0 and none is claimed" | :147 | TRUE | rg updater/updateUrl/appcast in tauri.conf.json + product.toml -> only a comment "installers/updaters that compare integers" |
| 19 | "dispatch-mode dry run ... wrote the notes (20-commit range, exit 0)" | :174-176 | UNVERIFIED | not re-run (needs env setup); script exists |
| 20 | "RC 1: EXECUTED ... final green = 43466814633 ... windows upgrade test green twice" | :245 | UNVERIFIED | no git/network access; run IDs uncheckable from tree |
| 21 | "v0.1.0 published 17:00:58Z, 40 assets, body bounded at 120k chars" | :248 | UNVERIFIED | same |
| 22 | "the release ships honestly labeled-unsigned (*.UNSIGNED.txt sidecars)" | :252 | TRUE-STRUCTURALLY | sign-macos.sh:203-207 writes the sidecar; gate.sh:191-196 proves the contract locally (baseline packaging: PASS) |
| 23 | "ADR-010 ... STILL PROPOSED — shipped on the user's explicit instruction" | :251 | UNVERIFIED | governance claim; ADR register exists; cannot verify instruction (issue #66) |
| 24 | "lumen --version reads apps/cli/Cargo.toml (env!(CARGO_PKG_VERSION))" | :30 | TRUE | apps/cli/Cargo.toml:34 version="0.1.3" == baseline rc-smoke.txt `engine-version: PASS (lumen 0.1.3)` |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| [verified locally] lint-literals 0 violations; sbom no drift (:53,:54) | Orchestrator baseline: lint-literals FAILED 1 violation; check-identity FAILED; sbom DRIFT (full_gate.txt:60-70) | The baseline is current truth — the checklist rows are stale (doc dated 2026-11-10; baseline 2026-11-23) |
| "full_gate.sh (its 9 steps ... sbom)" (:55) | scripts/full_gate.sh has 11 steps (:10-82) | The script is truth; doc's count is stale. Note the doc CONTRADICTS its own §4.1 row, which already cites "full_gate.sh step 9" for sbom — consistent with 9-step era, pre-web-budget/packaging |

## 4. Fabrication / inflation indicators
NONE found — the opposite: red rows are kept red, blockers carry owners,
§9 preserves struck history. The one honesty GAP is temporal: green rows are
not re-validated as the tree moves (rows 5, 9, 11 above), and the doc itself
prescribes the fix ("edit the row and strike the blocker — never delete
history":232-234) but it was not applied to the drifted gate rows.

## 5. Matrix rows
N/A — not a matrix chunk.

## 6. Hostile-focus note (CI answer cross-link)
This doc corroborates the W17-A10 ci.yml finding from the process side: §1.6
explicitly warns "do not rely on the gate to catch [config-validator]" and the
baseline proves fmt/test/arch/lint-literals/identity/sbom are red on this tree
— i.e., the release checklist's own pre-flight would fail today at rows
1.4/1.5 (plus 1.7 which is blocked-disk). RC2 per this runbook cannot go green
without first fixing: fmt drift (testkit_wiring.rs), 2 first-vertical evidence
failures, arch edge violation, host.rs:1799 identity literal, and SBOM drift.
