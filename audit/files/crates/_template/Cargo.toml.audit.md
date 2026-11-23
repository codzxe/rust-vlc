# AUDIT: crates/_template/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A04 |
| Date | 2025-11-23 |
| Lines / functions | 22 / 3 config keys (name, version, edition, publish, lints) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (scaffolding crate; nothing depends on it — rg over all member manifests: zero dependents) — but IS a workspace member via `crates/*` glob, so it is built and tested on every lane |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg Cargo.lock (`name = "template"` :1109); rg member manifests for dependents (0); baseline cargo-test-workspace.txt:6173-6197 (PACKAGE template exit 0, smoke 1 passed); rg cyclonedx.json (template@0.1.0 :2647, scope "required"); my lint-literals real-tree run (template lib.rs:23 marker-allowed); read of all 5 template files; worklog archive canary record check |

## 1. One-paragraph verdict
A correct, minimal, live crate template. Every comment instruction in the manifest matches reality: `build.rs`, `src/lint_canary.rs`, the `#[cfg(lint_canary)]` declaration in `src/lib.rs`, `template_marker()` and `tests/smoke.rs` all exist exactly as the copy-checklist describes; the crate is a real workspace member (Cargo.lock:1109; baseline builds and runs its smoke test, exit 0); `[lints] workspace = true` matches the mandatory policy (baseline arch gate: 39/39 crates inherit); the cfg-gated canary never compiles in normal lanes (0 unit tests in baseline) and its proof — both documented commands failing with the two inherited-lint errors — is recorded verbatim in docs/worklog/archive-2026-10.md:672-674 (UNVERIFIED by re-run: clippy execution is outside my load budget). One real defect: because the template is a permanent workspace member, the generated product SBOM lists the scaffolding crate `template@0.1.0` with `"scope": "required"` — composition data that is false (nothing depends on it, it ships in no binary).

## 2. What actually works (verified)
- Baseline cargo-test-workspace.txt:6173-6197: PACKAGE template compiled, `tests/smoke.rs ... template_marker_is_stable ... ok` (1 passed), exit 0 — the manifest is a valid, building member.
- All five template files exist and match the manifest's copy-checklist (build.rs registers `cfg(lint_canary)` via `cargo::rustc-check-cfg`; lib.rs declares `#[cfg(lint_canary)] pub mod lint_canary`; canary carries the sanctioned `unwrap()` + unused-var violations only under that cfg).
- The canary proof record exists as claimed: docs/worklog/archive-2026-10.md:672-674 (both commands, exit 101, both lint tables named) + negative control green.
- The template's placeholder literal is properly covered by the lint-literals regime: my real-tree run reports `allowed: crates/_template/src/lib.rs:23: identity — crate-template placeholder value — replaced when the template is instantiated`.

## 3. Findings
F-001 [S3]
Where: crates/_template/Cargo.toml:15-16 (manifest that makes the crate a member) -> docs/security/sbom/cyclonedx.json:2645-2649
Code:
```
[package]
name = "template"
version = "0.1.0"
```
and (SBOM, verbatim):
```
      "bom-ref": "template@0.1.0",
      "name": "template",
      "version": "0.1.0",
      "scope": "required",
```
What is wrong: the copy-me scaffolding crate is swept into the workspace by the `crates/*` member glob and from there into the generated product SBOM, where it is declared a REQUIRED component of "Lumen 0.1.3". Nothing depends on it (rg over every member manifest: zero dependents) and it is compiled into no shipped binary; `scope: required` is false composition data in a security artifact that compliance/vulnerability tooling consumes. The membership itself is deliberate and defensible (canary + smoke test prove lint inheritance — archive-2026-10.md:656, 662-674); the SBOM leakage is the unhandled consequence. What correct looks like: the sbom generator excludes scaffolding members (or the template moves out of `crates/`).
Confidence: HIGH — Verification: ran (rg cyclonedx.json + rg dependents) + read.

## 4. Per-function table
Not applicable (22-line manifest, no functions). Config keys checked: `name` (generic but collision-safe, publish=false), `version` 0.1.0, `edition` 2024 (matches workspace), `publish = false` (policy), `[lints] workspace = true` (mandatory, inherited — baseline 39/39).

## 5. Test honesty
tests/smoke.rs:1 real assertion (`template::template_marker() == "lumen-crate-template"`) — passes in baseline; would fail if the placeholder changed. The canary module intentionally has no automated test (it must FAIL clippy/check under `--cfg lint_canary`; its proof is a recorded manual run — honest about being manual).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every crate inherits the workspace lint policy; no overrides, no lowering" (L6-8) | manifest comment | REAL | baseline full_gate unsafe-lints PASS (39/39); canary record archive:672-674 |
| "Delete the template-only canary proof artifacts: build.rs, src/lint_canary.rs, and the #[cfg(lint_canary)] module declaration in src/lib.rs" (L9-11) | manifest comment | REAL | all three artifacts exist as described (read) |
| "Replace template_marker() and tests/smoke.rs with the crate's real responsibility" (L12-13) | manifest comment | REAL | both exist (read) |
| canary is "compiled ONLY when --cfg lint_canary is passed" (lint_canary.rs:6-9) | canary doc | REAL-UNPROVEN | baseline shows 0 unit tests for template lib (cfg off in all lanes); clippy re-run outside my budget (cargo-clippy ban) — record verified in archive:672-674 |

## 7. Dead / duplicated / unreachable code
The whole crate is scaffolding by design; its only live payload is the canary proof (cfg-off in shipped lanes = UNREACHABLE by design, documented). No duplication.

## 8. Dependencies & flags
Zero dependencies. No features. Member of the workspace via `crates/*` glob; `publish = false`.

## 9. Open questions for the next agent
- xtask sbom owner: template@0.1.0 sits in the committed SBOM with scope "required" (F-001) — is any filtering of non-product members planned? Also check whether tools/mem-audit, tools/sweep (also members) leak the same way.
- crates/_template/src/lib.rs owner (not my file): doc typo at L8 — "no dropped `#ust_use]` results" should read `[must_use]`.
