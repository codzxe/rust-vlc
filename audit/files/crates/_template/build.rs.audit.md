# AUDIT: crates/_template/build.rs
| Field | Value |
|---|---|
| Auditor | W01-A09 |
| Date | 2026-11-23 |
| Lines / functions | 9 / 1 (`main`) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — build script of the `template` crate (a workspace member kept only as lint canary + scaffolding template, publish = false, in no internal_deps list per boundaries.toml:695-699). It runs during `cargo build/test --workspace`; it is not shipped. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `RUSTFLAGS="--cfg lint_canary" cargo check -p template --all-targets` -> FAILS as documented (`error: unused variable: 'unused_var'` ... `could not compile`); `cargo clippy -p template --all-targets -- --cfg lint_canary` -> FAILS with both documented errors (`unused variable` + `used unwrap() on an Option value`); `cargo --version` -> 1.99.1 (toolchain pin rust-toolchain.toml `channel = "1.99.1"` — supports the `cargo::` directive syntax); glob `**/build.rs` -> exactly 2 in repo (this file + apps/desktop-tauri/build.rs, outside the workspace gate) |

## 1. One-paragraph verdict
This is a real, working, minimal build script that does exactly one thing — registering `lint_canary` as a known cfg — and I verified the whole mechanism end-to-end: the canary module compiles silently in normal builds (baseline clippy -D warnings: CLEAN, so no `unexpected_cfgs` leak), and with the cfg enabled both documented lint failures fire through pure `[lints] workspace = true` inheritance (I ran both proof commands; both fail exactly as the canary's doc predicts). The instruction to delete the file when copying the template is honored tree-wide: this is the only build script among the 39 workspace members (the second `build.rs` in the repo, apps/desktop-tauri, is ADR-004-excluded and outside every gate — the same coverage hole recorded in the boundaries.toml audit).

## 2. What actually works (verified)
- `RUSTFLAGS="--cfg lint_canary" cargo check -p template --all-targets` -> `error: unused variable: 'unused_var'` ... `error: could not compile` — canary activates and the inherited rust lint denies it.
- `cargo clippy -p template --all-targets -- --cfg lint_canary` -> 2 errors: `unused variable: 'unused_var'` + `used unwrap() on an Option value` — both inherited-lint denials, exactly the two violations in `src/lint_canary.rs:29-30`.
- Normal builds see no warnings from the cfg-gated module (baseline cargo-clippy.txt CLEAN; cargo 1.99.1 supports `cargo::rustc-check-cfg`).

## 3. Findings
None. What was checked and how:
- Directive syntax/semantics: `println!("cargo::rustc-check-cfg=cfg(lint_canary)")` is the correct modern form; verified by the clean baseline clippy run (no `unexpected_cfgs`) and by the activating runs above.
- Fakeness/stubs: none — one directive, matches the comment "This is the ONLY reason a build script exists here" (file contains nothing else).
- Side effects: none (no file writes, no env, no native code — consistent with "build scripts are native-code audit suspects" doc 10 §2.3).
- Missing `rerun-if-changed` directives: absent, so cargo uses default rerun semantics (rerun when package files change); zero impact for a deterministic no-output script — not a defect.
- Template-copy discipline: glob `**/build.rs` — no workspace crate copied the template's build script; only apps/desktop-tauri/build.rs exists, in the ADR-004-excluded tree (see boundaries.toml audit F-003 for the gate-coverage hole that implies).
- Manifest wiring: build.rs is auto-detected by cargo convention (crates/_template/Cargo.toml declares no `build` key; cargo uses ./build.rs when present) — works, verified by the compiled canary behavior.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `main` | 7-9 | REAL | — | Emits the single check-cfg directive; verified live |

## 5. Test honesty
No tests in this file. The mechanism it enables is proven by the canary proof commands (which I executed — see §2) and by the clean gate runs in the baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "registers the `lint_canary` cfg name so rustc's `unexpected_cfgs` lint does not warn" | build.rs:1-3 | REAL | Baseline clippy CLEAN with the cfg-gated module present; activation runs fail with the inherited lints |
| "This is the ONLY reason a build script exists here" | build.rs:3-4 | REAL | File is 9 lines, one println |
| "a plain Lumen crate must not carry a build script" | build.rs:4-6 | REAL (workspace) | glob: zero build.rs in any of the other 38 members |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
None — std-only println in a build script; no features, no external crates.

## 9. Open questions for the next agent
- apps/desktop-tauri/build.rs (tauri-build) is never audited by any in-repo gate (excluded workspace): who reviews it under the "build scripts are native-code audit suspects" policy?
