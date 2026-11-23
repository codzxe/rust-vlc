# AUDIT: crates/_template/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W01-A03 |
| Date | 2026-11-23 |
| Lines / functions | 24 / 1 (`template_marker`) + 1 cfg-gated module decl (`lint_canary`) |
| File verdict | REAL (a template, honestly labeled as such; every doc claim verified, canary proof executed) |
| Reachable from shipped entry point? | NO (no crate depends on `template` — rg over apps/ crates/ tools/ manifests+sources finds zero dependents; it is built only because the root `members = ["crates/*"]` glob sweeps it in) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p template` → 1 passed (template_marker_is_stable ok); `RUSTFLAGS="--cfg lint_canary" cargo check -p template --all-targets` → exit 101, `error: unused variable: 'unused_var'` (canary proof, rust-side half, executed); `rg` lint-table cross-check; `rg` dependent scan (none). cargo clippy half of the canary proof NOT run (clippy banned by spec) — UNVERIFIED. |

## 1. One-paragraph verdict
This is a 24-line crate template that does exactly what it says: a placeholder `template_marker()` returning a constant (explicitly labeled "Template placeholder — replace with the crate's real public API"), a cfg-gated lint canary module, and instructions. Nothing here pretends to be functionality. The doc-comment's lint-policy enumeration matches the root `[workspace.lints]` table verbatim (unsafe_code, unwrap/expect/panic/todo/unimplemented/dbg, must_use, await-holding-lock — all `deny`), and the lint-inheritance canary is not fiction: I executed `RUSTFLAGS="--cfg lint_canary" cargo check -p template --all-targets` and it failed exactly as documented (exit 101, `error: unused variable: 'unused_var'` — an error sourced exclusively from the inherited workspace lint, since the crate's own manifest carries only `[lints] workspace = true`), while the normal `cargo test -p template` passes with the canary invisible (1 passed). The clippy-side of the proof (`clippy::unwrap_used`) is UNVERIFIED because clippy is banned for this audit.

## 2. What actually works (verified)
- `cargo test -p template` → `test template_marker_is_stable ... ok; test result: ok. 1 passed` — the template builds and its smoke test runs in the workspace harness.
- `RUSTFLAGS="--cfg lint_canary" ~/.cargo/bin/cargo check -p template --all-targets` → exit 101, `error: unused variable: \`unused_var\`` at src/lint_canary.rs:29 — the inherited-lint denial fires, proving `[lints] workspace = true` genuinely propagates (no `--deny` flag was passed).
- Doc-claim cross-check: lib.rs:6-9's lint list ≡ root Cargo.toml:53-66 `[workspace.lints.rust]`/`[workspace.lints.clippy]` — exact match, every named lint is denied there.
- Zero dependents: no manifest or source outside `crates/_template` references the crate.

## 3. Findings
F-001 [S4]
Where: crates/_template/src/lib.rs:22-24
Code:
```
/// Template placeholder — replace with the crate's real public API.
pub fn template_marker() -> &'static str {
    "lumen-crate-template" // lumen-allow-literal: crate-template placeholder value — replaced when the template is instantiated
```
What is wrong: nothing functionally — the constant return is the documented purpose and the `lumen-allow-literal` marker keeps it honest against the C1 lint-literals gate. The one quality nit: `crates/_template` is swept into every `--workspace` build/test by the `members = ["crates/*"]` glob (it is in the root Cargo.lock as `template 0.1.0`), so a placeholder crate is compiled and its constant-vs-constant smoke test executed on every workspace lane forever. Harmless (24 lines, std-only) but permanent scaffolding in the build graph.
Confidence: HIGH — Verification: read + cargo test run + Cargo.lock grep.

F-002 [S4]
Where: crates/_template/tests/smoke.rs:6-8 (context for lib.rs's claim that tests prove the surface)
Code:
```
#[test]
fn template_marker_is_stable() {
    assert_eq!(template::template_marker(), "lumen-crate-template");
```
What is wrong: tautological by construction (asserts a constant equals itself); it cannot catch any regression because there is nothing to regress. Its honest purpose (per its own header: "proves the template builds and its public surface works inside the workspace") is build-harness proof, which it does achieve — but a reader could mistake it for a behavioral test. Acceptable for a template; would be a finding in a real crate.
Confidence: HIGH — Verification: read + executed (passed).

## 4. Per-function table (file ≤100 LOC — table included anyway for completeness)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `template_marker()` | 22-24 | REAL (placeholder by design) | S4 | constant return, honestly labeled; reachable only from smoke test |
| `#[cfg(lint_canary)] pub mod lint_canary` (decl) | 18-19 | REAL | — | gate closed in normal builds (verified: normal test run compiles nothing from it); canary fires under the documented RUSTFLAGS (executed) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| tests/smoke.rs::template_marker_is_stable | NO (behavior) / YES (build) | constant==constant is unfalsifiable behaviorally; its real assertion is "the template crate compiles and links in the workspace," which it does test |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "keep `[lints] workspace = true` — mandatory. The workspace lint policy (doc 10 §3) is inherited through it: no `unsafe`, no `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!`/`dbg!`, no dropped `#[must_use]`, no `.await` across a held lock" | lib.rs:5-9 | REAL | root Cargo.toml:53-66 matches item-for-item, all `deny`; inheritance PROVEN by canary execution (exit 101) |
| "The canary proves the lint-inheritance mechanism itself — see `src/lint_canary.rs` for the exact reproducible proof commands" | lib.rs:15-16 | REAL (half executed) | check-command executed → failed as documented (rust-side); clippy-command (unwrap_used half) UNVERIFIED — clippy banned for this audit |
| "Delete the template-only canary proof … when copying" | lib.rs:10-11 | REAL (instruction) | the three artifacts exist and are self-identifying (build.rs header: "DELETE THIS FILE when copying") |
| lint_canary.rs: "the ONLY sanctioned lint violation in this repository" | src/lint_canary.rs:1-2 | REAL | cfg-gated off in every normal invocation (verified: plain `cargo test -p template` passes); no other cfg-gated lint violations found in template |

## 7. Dead / duplicated / unreachable code
`lint_canary` module: unreachable by design in normal builds (cfg never set by cargo build/check/test — verified by the passing normal test run); reachable only via the documented manual proof command. Not dead: it is the proof instrument, and its proof I executed.

## 8. Dependencies & flags
No dependencies (std-only). Member of the workspace via the `crates/*` glob (in Cargo.lock as `template 0.1.0`, publish = false). The `#[cfg(lint_canary)]` gate is registered by build.rs via `cargo::rustc-check-cfg=cfg(lint_canary)` — correct modern syntax, and build.rs self-documents as template-only ("a plain Lumen crate must not carry a build script").

## 9. Open questions for the next agent
- The canary's clippy half (`cargo clippy -p template --all-targets -- --cfg lint_canary` → `clippy::unwrap_used` error) remains unexecuted under this audit's load discipline; any agent with a clippy budget should run it once to close the loop.
- Should `crates/_template` be moved out of the `crates/*` glob (e.g. `exclude` + template instantiation script) so placeholder code stops compiling in every workspace lane? Cosmetic, but it also muddies crate-count metrics (the 39 workspace members include 1 template).
