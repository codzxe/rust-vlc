# AUDIT: crates/_template/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A03 |
| Date | 2026-11-21 |
| Lines / functions | 23 / 6 config keys (package×5 + [lints]) + 13-line copy-checklist header |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — by explicit design. Package `template` is a workspace member (Cargo.lock:1109; root Cargo.toml:6-12 documents why membership is required) with `publish = false` and the rule "no crate may ever depend on it", encoded in boundaries.toml:691-701 by absence from every internal_deps list. Verified: grep `^\s*template\s*=` across all Cargo.toml → zero dependents |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | grep `^\s*template\s*=` in **/Cargo.toml (zero dependents) · sed -n '688,703p' boundaries.toml ([crates.template] matches empty deps exactly) · sed -n '1107,1113p' Cargo.lock (template 0.1.0, zero deps) · grep `build.rs` in tools/xtask/src/cmd/check_architecture.rs + scripts/*.sh + ci.yml (zero build-script guards) · `ls crates/*/build.rs apps/*/build.rs` (exactly 2 in repo: template + desktop-tauri) |

## 1. One-paragraph verdict
A 23-line scaffolding manifest whose every claim is externally verifiable and
verified: name/version/edition/publish match the lockfile, the empty
dependency list matches boundaries.toml's `[crates.template]` empty lists
exactly, `[lints] workspace = true` resolves because the crate is a member
(the root manifest documents the cargo constraint that forces membership),
and nothing depends on it (grep-proven, and by declaration). The one gap is
enforcement, not accuracy: the copy checklist's step 3 — "Delete the
template-only canary proof artifacts: build.rs …" — is backed by no gate; a
rushed `cp -r` that keeps build.rs would sail through every committed check.

## 2. What actually works (verified)
— Membership + lint inheritance: package `template` in Cargo.lock:1109;
  workspace member via `crates/*` glob (root Cargo.toml:41); the canary
  mechanism this manifest's checklist references was audited by me in
  audit/files/crates/_template/build.rs.audit.md (registration verified via
  baseline clippy #5 exit 0).
— Deps honesty: manifest declares no dependencies; boundaries.toml
  [crates.template] internal_deps = [] / dev_deps = [] — exact match;
  Cargo.lock shows the package with zero dependencies.
— Isolation: no manifest in the repo declares a dependency on `template`
  (grep: zero hits); publish = false.
— The crate builds and its smoke test runs in every workspace lane
  (baseline #1 exit 0; #3 test run includes template::template_marker_is_stable).

## 3. Findings
### F-001 [S4] — the delete-on-copy rule for build.rs is enforced by nothing
Where: crates/_template/Cargo.toml:9-11 (the rule), no enforcing gate
Code:
```toml
#   3. Delete the template-only canary proof artifacts: `build.rs`,
#      `src/lint_canary.rs`, and the `#[cfg(lint_canary)]` module
#      declaration in `src/lib.rs`.
```
What is wrong: Grep `build.rs|build-script|build_script` in
tools/xtask/src/cmd/check_architecture.rs, scripts/*.sh, ci.yml → zero
guards. The repo currently has exactly two build scripts
(crates/_template/build.rs, apps/desktop-tauri/build.rs), but a crate copied
from the template without executing step 3 would carry a build script — the
manifest's own header (via build.rs:5-6) calls build scripts "native-code
audit suspects per doc 10 §2.3" — and no committed check would notice, ever.
Why it matters: The template is the replication vector for every future
crate; an unenforced deletion step silently multiplies the audit-suspect
surface the repo's own docs flag.
What correct looks like: An xtask check (or check-architecture rule) that
builds the set of `build.rs` files and fails on any outside an explicit
allowlist {`crates/_template`, `apps/desktop-tauri`}.
Confidence: HIGH — Verification: read + grep (no guard exists).

## 4. Per-key table
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `package.name = "template"` | 16 | REAL | — | Matches boundaries.toml [crates.template]; snake_case (worklog F3 precedent) |
| `package.version = "0.1.0"` | 17 | REAL | — | Matches Cargo.lock:1109-1110 |
| `package.edition = "2024"` | 18 | REAL | — | Workspace-consistent |
| `package.publish = false` | 19 | REAL | — | Never published; root manifest line 12 records the rule |
| `[lints] workspace = true` | 21-22 | REAL | — | Resolves (member); inheritance mechanism canary-verified (see my build.rs audit; enforcement gap S3 filed there) |
| absent `[dependencies]`/`[dev-dependencies]`/`[features]` | — | REAL | — | Matches boundaries empty lists; zero-dep Cargo.lock entry |

## 5. Test honesty
No tests in the manifest. The referenced tests/smoke.rs
(`template_marker_is_stable`) runs in baseline #3 — it proves the crate
compiles and its one function returns the marker literal; nothing more is
claimed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "KEEP `[lints] workspace = true` — it is mandatory" | Cargo.toml:6-8 | REAL | Member resolves it; check-architecture's missing-lints check (cmd docs line 25) guards the rule repo-wide |
| "Delete the template-only canary proof artifacts" (step 3) | Cargo.toml:9-11 | REAL as instruction / UNENFORCED as rule | No gate checks build.rs presence — F-001 |
| "no crate may ever depend on it" | root Cargo.toml:12 (mirrored here by context) | REAL | grep zero dependents; boundaries.toml:693 records the encoding |
| "publish = false until the Lead decides otherwise" | Cargo.toml:5-6 | REAL | publish = false at line 19 |

## 7. Dead / duplicated / unreachable code
The crate is deliberately unreachable (scaffolding). Consistent with sibling
audits: lib.rs (W01-A02) and lint_canary.rs (W01-A01) both verdicted
UNREACHABLE; build.rs (W01-A03) REAL as build-time infrastructure. No
duplication.

## 8. Dependencies & flags
None declared — none to be unused. Matches boundaries.toml [crates.template]
(empty all four lists). No features. The crate inherits the full workspace
deny tables via [lints].

## 9. Open questions for the next agent
- Whoever audits tools/xtask/src/cmd/check_architecture.rs: confirm the
  edges check would catch a future crate whose build.rs was not deleted
  (F-001 says it would not — build scripts are invisible to it today).
- If the Lead ever allows publishing (comment line 5), the empty-deps +
  canary design needs rework before `publish = true`.
