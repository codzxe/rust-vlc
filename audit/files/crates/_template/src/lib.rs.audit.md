# AUDIT: crates/_template/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W01-A02 |
| Date | 2026-11-21 |
| Lines / functions | 25 / 1 (`template_marker`) + 1 cfg-gated module decl |
| File verdict | UNREACHABLE (honest template scaffolding; nothing in a shipped binary reaches it) |
| Reachable from shipped entry point? | NO — package `template` has zero dependents (proof below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Grep `^template\s*=\|^\s*template\s*=` over `**/Cargo.toml` → 0 matches (no crate depends on `template`); `sed -n '1,24p' crates/_template/src/lib.rs` (quote verify); `od -c` on line 8 (no hidden bytes; earlier terminal mangling of `#[must_use]` was rendering only); `rg -n '^\[crates\.template\]' boundaries.toml` → 695; `rg -n 'CANARY PROOF' docs/worklog/archive-2026-10.md` → 671; Grep `lints\|inherit` in tools/xtask/src/cmd/check_architecture.rs → check 3 `unsafe-lints`; baseline INDEX #3/#5/#11/#13/#14 |

## 1. One-paragraph verdict
This file is exactly what it claims to be: a 25-line crate template with one documented-placeholder function and a cfg-gated lint canary that no normal build compiles. Nothing here is fraudulent — but nothing here is product code either. No crate in the workspace depends on package `template` (boundaries.toml:695 encodes "no crate may ever depend on it" as empty dep lists), so `template_marker()` is unreachable from both shipped entry points (`lumen` CLI, `lumen-tauri`). The two findings are doc-strength inflation ("no `unsafe`, no `unwrap`…" describes lint config, not the enforceable reality of deny-plus-`#[allow]`-escapes) and the placeholder function's dead weight in every `--workspace` build.

## 2. What actually works (verified)
— The crate compiles and its smoke test passes as part of the workspace suite: baseline #3 `cargo test --workspace --locked` → 1681 passed / 1 failed (the failure is `apps/desktop/tests/zero_alloc_transport.rs:98`, unrelated), and the template's own test `template_marker_is_stable` (tests/smoke.rs:5-8) is in that suite.
— Lint inheritance for this crate is continuously checked, not just by the manual canary: baseline #11 `cargo xtask check-architecture` → "unsafe-lints 39/39 inherit" (xtask check_architecture.rs:130 `"unsafe-lints"`, :1609 `fn check_lints`).
— No hidden bytes in the file: `od -c` on line 8 shows plain ASCII `#[must_use]`.

## 3. Findings
### F-001 [S4]
Where: crates/_template/src/lib.rs:5-9
Code:
```
//! 1. Set the `[package] name` in `Cargo.toml` and keep `[lints] workspace =
//!    true` — mandatory. The workspace lint policy (doc 10 §3) is inherited
//!    through it: no `unsafe`, no `unwrap`/`expect`/`panic!`/`todo!`/
//!    `unimplemented!`/`dbg!`, no dropped `#[must_use]` results, no `.await`
//!    across a held lock. Typed error propagation everywhere.
```
What is wrong: The sentence sells the lints as an absolute ("no `unsafe`, no `unwrap`…"). Every entry in the root `[workspace.lints]` table (Cargo.toml:53-66) is `deny`, and rustc `deny` is overridable by an inner `#[allow]` — which this repository exercises heavily: baseline hygiene-counts.txt records `.unwrap()` 15, `.expect(` 33, `todo!` 8, `unimplemented!` 6, `unsafe {` 316, and `#[allow(` 121 across tracked `*.rs`. The template never mentions that escape hatch exists.
Why it matters: This doc is the first thing every new-crate author reads; it teaches a false model of the gate ("the workspace lint makes `unsafe` uncompilable" is how xtask's own doc puts it, check_architecture.rs:14 — also false in the presence of `#[allow(unsafe_code)]`). Authors will assume violations cannot compile instead of auditing for `#[allow]`-masked ones.
What correct looks like: State the truth: "`deny`-level lints inherited via `[lints] workspace = true`; note `deny` ≠ `forbid` — item-level `#[allow]` can override, so review any `#[allow]` you add against the 121-repo allow inventory."
Confidence: HIGH — Verification: read (root Cargo.toml:53-66) + baseline run (INDEX.md #5 clippy CLEAN, hygiene-counts.txt verbatim counts).

### F-002 [S4]
Where: crates/_template/src/lib.rs:21-24
Code:
```
/// Template placeholder — replace with the crate's real public API.
pub fn template_marker() -> &'static str {
    "lumen-crate-template" // lumen-allow-literal: crate-template placeholder value — replaced when the template is instantiated
}
```
What is wrong: `template_marker()` is a constant-returning placeholder that no product code can ever call: Grep for `template` as a dependency key across every `Cargo.toml` in the repo → 0 matches; boundaries.toml:691-700 deliberately gives `[crates.template]` empty `internal_deps`/`dev_deps`. Yet the crate is a workspace member (root Cargo.toml:41 `members = ["crates/*", …]`), so this dead function is compiled and its literal is allowlisted in every `cargo build --workspace` lane.
Why it matters: Dead code in the build graph and one more `lumen-allow-literal` site to carry; the cost is tiny and the design rationale (keep the template lint-clean as the workspace grows, root Cargo.toml:6-12) is documented — hence S4, not S3 — but an auditor must record that nothing here ships.
What correct looks like: Accepted as scaffolding; alternatively `publish = false` + `cargo check -p template` in CI only, removing it from the default workspace lane.
Confidence: HIGH — Verification: read + Grep dependency scan (0 dependents) + boundaries.toml:695-700.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `template_marker()` | 22-24 | UNREACHABLE | S4 | Constant return; no panic paths; only caller is its own `tests/smoke.rs:7`; zero product dependents; literal carries sanctioned `lumen-allow-literal` marker (syntax defined at tools/xtask/src/cmd/lint_literals.rs:495 `const MARKER`). |
| `#[cfg(lint_canary)] pub mod lint_canary;` | 18-19 | UNREACHABLE-by-design | — | Never compiled by any gate (cfg is only *registered* by build.rs:8, never set); compiled only under manual `--cfg lint_canary`. The one-shot proof lives at docs/worklog/archive-2026-10.md:671-675. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `template_marker_is_stable` (crates/_template/tests/smoke.rs:5-8) | Only if the literal changes | Asserts a constant equals itself; it pins the placeholder string (that is its stated job), but it proves nothing about any product behavior — honest placeholder, zero product value. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The canary proves the lint-inheritance mechanism itself" | lib.rs:15-16 | REAL-UNPROVEN (in this audit's hands) | The recorded proof run (docs/worklog/archive-2026-10.md:671-675, exit 101 with verbatim `error: used \`unwrap()\` …` / `error: unused variable …`) is committed evidence, but no baseline run re-executed it (INDEX.md has no canary entry); corroborated indirectly by baseline #11 "unsafe-lints 39/39 inherit" and #5 clippy CLEAN. |
| "no crate may ever depend on it" | root Cargo.toml:12 | REAL | Grep `**/Cargo.toml` for a `template` dependency → 0 matches; boundaries.toml:695-700 encodes empty deps. |
| "no `unsafe`, no `unwrap`… inherited" | lib.rs:7-9 | REAL (as lint config) / misleading (as code-state claim) | Root Cargo.toml:53-66 carries exactly those denies; but 316 `unsafe {` + 121 `#[allow(` survive in tracked sources (baseline hygiene-counts.txt) — see F-001. |

## 7. Dead / duplicated / unreachable code
- `template_marker()` — unreachable from shipped entry points (see F-002).
- `lint_canary` module — unreachable in every gate by design; delete-on-copy instruction (lib.rs:10-11) is followed by no other crate needing it (it exists only here).

## 8. Dependencies & flags
- Manifest (crates/_template/Cargo.toml): no dependencies at all; `edition = "2024"`, `publish = false`, `[lints] workspace = true` only. No features. No unsafe. The crate's `build.rs` exists solely to emit `cargo::rustc-check-cfg=cfg(lint_canary)` (build.rs:7-9) — documented as delete-on-copy (build.rs:1-6).

## 9. Open questions for the next agent
- Root Cargo.toml auditor (W01-A01): the systemic question — deny-vs-forbid and the 121 `#[allow(` escape hatches — belongs to the root manifest's audit; this file only inherits the wording.
- If anyone audits tools/xtask/src/cmd/check_architecture.rs: the doc line 14 claim "the workspace lint makes `unsafe` uncompilable" is contradicted by 316 `unsafe {` blocks compiling today (they carry `#[allow]`s).
