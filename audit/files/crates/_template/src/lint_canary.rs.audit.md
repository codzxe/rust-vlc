# AUDIT: crates/_template/src/lint_canary.rs
| Field | Value |
|---|---|
| Auditor | W01-A01 |
| Date | 2026-11-21 |
| Lines / functions | 31 / 1 (plus module docs) |
| File verdict | UNREACHABLE (intentionally cfg-gated tripwire; mechanism traced REAL, one-time proof pasted in archived worklog only) |
| Reachable from shipped entry point? | NO (by design: `#[cfg(lint_canary)]` is never set by any shipped build, script, or workflow — rg over repo shows zero setters) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n lint_canary` (whole repo: only template files, root Cargo.toml comment, worklog archive — **no** script/.github reference); `rg '"template"|^template =' --glob '**/Cargo.toml'` (no crate depends on package `template`); `sed -n` on all quoted ranges; per-manifest `[lints]` inheritance loop (39/39 member manifests carry `[lints] workspace = true`); baseline INDEX #5/#6 (clippy clean) |

## 1. One-paragraph verdict
This is a deliberate lint tripwire, not product code: a 4-line function that violates two inherited deny-lints (`unused_variables`, `clippy::unwrap_used`) and is compiled only under a cfg that no build path ever sets. The mechanism is genuine — I traced the chain: root `Cargo.toml:53-66` denies both lints, `crates/_template/Cargo.toml:21-22` inherits via `[lints] workspace = true`, `lib.rs:18-19` gates the module, `build.rs:8` only *registers* the cfg (does not set it). The one-time proof (worklog archive-2026-10.md:670-673, exit 101 with both errors) is a pasted manual run from 2026-10, never re-run by any gate. The file is honest about being a tripwire, but the repo's claim that lint inheritance "is proven by the canary" overstates a dead, unautomated, one-crate historical artifact into an ongoing guarantee.

## 2. What actually works (verified)
— Mechanism traced: cfg never set in default builds → module absent from baseline clippy runs (INDEX #5 `cargo clippy --workspace --all-targets --locked -- -D warnings` exit 0 — impossible if the module compiled, since `unwrap_used` is denied at root Cargo.toml:60). Verified by reading build.rs (check-cfg registration only) + lib.rs cfg gate + rg proving no `--cfg lint_canary` anywhere in scripts/.github.
— All 39 workspace-member manifests carry `[lints] workspace = true` (per-manifest grep loop, this audit) — matches baseline INDEX claim "39/40 crates inherit".

## 3. Findings
### F-001 [S3]
Where: crates/_template/src/lint_canary.rs:1-16 (claim) vs. absence of any runner
Code:
```
//! This module is compiled ONLY when `--cfg lint_canary` is passed to rustc
...
//! It exists to prove that `[workspace.lints]` (root `Cargo.toml`,
//! doc 10 §3.1) is genuinely inherited by a crate whose manifest carries
//! nothing but `[lints] workspace = true`.
```
What is wrong: The "proof" is not wired into any automated gate. `rg -n lint_canary` across the entire repo finds zero hits in `scripts/`, `.github/workflows/`, `tools/xtask/`, or `scripts/full_gate.sh`. The only execution evidence is a manual pasted run from 2026-10 in `docs/worklog/archive-2026-10.md:670-673`.
Why it matters: The canary exists precisely to catch regressions in lint inheritance; since it never runs, deleting `unwrap_used = "deny"` from the root manifest (or a crate dropping `[lints] workspace = true` and adding its own lax table) would pass every CI gate silently while the canary file still asserts it "proves" inheritance. The per-member `[lints] workspace = true` key is likewise enforced by nothing automated (baseline hygiene count 39/40 is a one-off wave-0 tally, not a gate).
What correct looks like: A gate step (xtask or workflow) running `cargo clippy -p template --all-targets -- --cfg lint_canary` and asserting exit≠0 with both expected error strings, plus a manifest lint enforcing the `[lints] workspace = true` key on every member.
Confidence: HIGH — Verification: read (rg exhaustive; baseline INDEX #5 cited).

### F-002 [S4]
Where: crates/_template/src/lint_canary.rs:21-22
Code:
```
//! The verbatim failure output of the original proof run is recorded in the
//! worklog under task 1.A1.
```
What is wrong: Stale pointer. The live worklog is repo-root `worklog.md` (docs/worklog/README.md:3); task 1.A1's entry has been archived to `docs/worklog/archive-2026-10.md:670-673`. A reader following the pointer to the "worklog" literally finds nothing for 1.A1.
Why it matters: The file's whole value is its reproducible proof; a dead pointer forces every auditor to re-derive where the evidence lives.
What correct looks like: Point to `docs/worklog/archive-2026-10.md` (task 1.A1, "CANARY PROOF") explicitly.
Confidence: HIGH — Verification: read (worklog README + archive sed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `canary(maybe: Option<u8>) -> u8` | 28-31 | UNREACHABLE | S3 (via F-001) | Body is two sanctioned violations (`let unused_var = 7_u8;` unused; `maybe.unwrap()`). No panic beyond the denied unwrap itself; no callers anywhere (rg `canary(` → only this file); compiled only under `--cfg lint_canary` which nothing sets. Parameter `maybe` IS used (`.unwrap()`), so no ignored-parameter issue. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `crates/_template/tests/smoke.rs: template_marker_is_stable` | No (not about this file) | Asserts `template_marker()` string only; the canary function has zero tests — correct, since testing it would require enabling the cfg, and its "output" is a compile error. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the ONLY sanctioned lint violation in this repository" | lint_canary.rs:1-2 | REAL-UNPROVEN | Baseline hygiene-counts.txt shows the only tracked `.unwrap()` in the template crate is this file (1 hit); repo-wide .unwrap()=15 are in other crates (whether those are `#[allow]`'d is other agents' scope). Not re-verified exhaustively here. |
| "Inheritance is proven by the canary" | root Cargo.toml:31-33 | SHALLOW | Mechanism real (traced), executed once manually (worklog archive:670-673, exit 101); no gate re-runs it (F-001); proves inheritance for the `template` crate only, not the other 38 members. |
| "cfg ... is never set by cargo build/check/test/clippy" | lint_canary.rs:4-6 | REAL | Traced: build.rs:8 emits only `cargo::rustc-check-cfg=cfg(lint_canary)` (registration, not activation); rg finds no other setter. |

## 7. Dead / duplicated / unreachable code
The entire module is dead code in every shipped build (cfg-gated out) — intentional and documented. No duplication. Crate `template` has zero dependents (rg over all Cargo.tomls) and `publish = false`.

## 8. Dependencies & flags
Crate has zero runtime/dev dependencies (crates/_template/Cargo.toml:15-22 — only `[package]` + `[lints]`). Not feature-gated. Workspace member via `crates/*` glob (root Cargo.toml:41).

## 9. Open questions for the next agent
- Does `cargo xtask check-architecture` (tools/xtask) enforce the `[lints] workspace = true` key per member? If yes, F-001's blast radius shrinks to the root-table content. W01-A01 is tracing xtask for the domain questions and will record the answer in the log/xtask audit.
- The 15 repo-wide `.unwrap()` hits outside this file (hygiene-counts.txt) — which agent owns those files; are they `#[allow]`-sanctioned like this canary?
