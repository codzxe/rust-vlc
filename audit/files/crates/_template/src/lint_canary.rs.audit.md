# AUDIT: crates/_template/src/lint_canary.rs
| Field | Value |
|---|---|
| Auditor | W01-A06 |
| Date | 2026-11-23 |
| Lines / functions | 31 / 1 (canary) |
| File verdict | REAL |
| Reachable from shipped entry point? | UNREACHABLE BY DESIGN: the module compiles only under `--cfg lint_canary` (registered by crates/_template/build.rs; never set by build/check/test/clippy lanes — proven by the CLEAN clippy baseline). The template crate is scaffolding, not a dependency of lumen-cli/desktop. Reachable only via the two documented developer proof commands — which I executed. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `RUSTFLAGS="--cfg lint_canary" cargo check -p template --all-targets` -> FAIL (exit 101) with `error: unused variable: 'unused_var'` at lint_canary.rs:29, `-D unused-variables` sourced from inherited workspace lints; `cargo clippy -p template --all-targets -- --cfg lint_canary` -> FAIL (exit 101) with BOTH `used 'unwrap()' on an 'Option' value` (line 30, `-D clippy::unwrap-used`) and the unused-variable error ("due to 2 previous errors"); `rg` workspace-lints confirmation (root Cargo.toml:57,60); `rg -n "1.A1"` worklog record found |

## 1. One-paragraph verdict
This file does exactly what it claims and I proved it by executing both documented proof commands: with the canary cfg enabled, compilation fails with precisely the two errors the doc promises — `unused_variables` (rust table) and `clippy::unwrap_used` (clippy table) — both reported as "requested on the command line with -D ..." even though the proof commands pass NO deny flags of their own, which is the actual claim: the denies come exclusively from inherited `[workspace.lints]` (verified present at root Cargo.toml:57 `unused_variables = "deny"` and :60 `unwrap_used = "deny"`). Both commands exit 101 exactly as the archived original proof run records (docs/worklog/archive-2026-10.md:672-673 — the record the doc's "recorded in the worklog under task 1.A1" refers to; root worklog.md:5-6 documents the archive convention). In normal lanes the module is invisible (cfg never set — consistent with the baseline clippy -D warnings CLEAN and the 4744-pass test run). Zero findings.

## 2. What actually works (verified)
— verified by executing proof command 2: `RUSTFLAGS="--cfg lint_canary" cargo check -p template --all-targets` -> `error: unused variable: 'unused_var'` (lint_canary.rs:29:9), `= note: requested on the command line with -D unused-variables`, exit 101.
— verified by executing proof command 1: `cargo clippy -p template --all-targets -- --cfg lint_canary` -> `error: used 'unwrap()' on an 'Option' value` (lint_canary.rs:30:5) plus the unused-variable error, `error: could not compile 'template' (lib) due to 2 previous errors`, exit 101.
— verified by read: build.rs registers `cargo::rustc-check-cfg=cfg(lint_canary)`; lib.rs gates `#[cfg(lint_canary)] pub mod lint_canary;`; root Cargo.toml denies both lints workspace-wide.

## 3. Findings
None (S0-S4: 0). What was checked and how:
- Fakeness: no stubs/constants-returning bodies — the body IS the violation pair the module exists to carry (lines 28-31); verified by reading and by the compiler tripping on both lines.
- Correctness: cfg-gating verified end-to-end (normal baseline lanes clean; explicit cfg trips exactly 2 errors, nothing else); build.rs is the declared sole purpose cfg registration (read).
- Reachability: unreachable in shipped lanes by design (see header) — verified via baseline clippy CLEAN + no lane passes the cfg (rg over .cargo/config.toml, CI workflows: no `lint_canary` in any workflow — only the 5 template files + archive reference it).
- Doc claims: all four concrete claims (only-when-cfg, both commands must fail, errors sourced from inheritance, worklog record) — verified as above; the "ONLY sanctioned lint violation in this repository" claim is consistent with the CLEAN clippy baseline but not exhaustively provable — noted, not counted.
- Test honesty: no tests in this file (the crate's tests/smoke.rs tests template_marker, not the canary — out of scope, file exists).

## 4. Per-function table (31 loc — under the 100-LOC bar, included anyway)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| canary | 28-31 | REAL | — | intentional violations; compiler-verified to trip both inherited denies |

## 5. Test honesty
No tests in this file. The module's "test" is the documented proof procedure, which I executed successfully (both commands fail as specified).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "compiled ONLY when `--cfg lint_canary` is passed to rustc ... never set by cargo build/check/test/clippy" | lint_canary.rs:4-7 | REAL | baseline clippy CLEAN + 4744-pass test run with the module absent; explicit cfg trips it (executed) |
| "Proof — run from the repository root; BOTH commands must FAIL" | lint_canary.rs:11-16 | REAL | both executed, both exit 101 with the promised errors |
| "every `error:` in the output is sourced exclusively from the inherited `[workspace.lints]`" | lint_canary.rs:18-20 | REAL | commands pass no deny flags; notes say `-D unused-variables` / `-D clippy::unwrap-used` (inherited); root Cargo.toml:57,60 |
| "The verbatim failure output of the original proof run is recorded in the worklog under task 1.A1" | lint_canary.rs:21-22 | REAL | docs/worklog/archive-2026-10.md:672-673 contains both verbatim outputs matching my reproduction; root worklog.md:5-6 explains the archive convention |
| "the ONLY sanctioned lint violation in this repository" | lint_canary.rs:1-2 | REAL-UNPROVEN (consistent) | clippy -D warnings baseline CLEAN; no other cfg-gated violation found by rg; exhaustiveness not provable |

## 7. Dead / duplicated / unreachable code
The module is unreachable-by-design in shipped lanes (its purpose). The DELETE-THIS instructions appear in four places (Cargo.toml comment, lib.rs doc, this file's doc, build.rs comment) — redundant on purpose, benign if one is missed.

## 8. Dependencies & flags
No dependencies; no features; std-only. Depends on build.rs cfg registration and root `[workspace.lints]` (both verified present).

## 9. Open questions for the next agent
- None for this file. (Adjacent, out of scope: template's tests/smoke.rs and root Cargo.toml's full [workspace.lints] table belong to another assignment.)
