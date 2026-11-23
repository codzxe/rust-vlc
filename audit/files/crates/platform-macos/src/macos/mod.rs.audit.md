# AUDIT: crates/platform-macos/src/macos/mod.rs
| Field | Value |
|---|---|
| Auditor | W08-A02 |
| Date | 2026-11-15 |
| Lines / functions | 22 / 0 functions (module hub: 4 cfg-gated declarations) |
| File verdict | UNREACHABLE (real cfg gate, honest docs; the crate it gates into has zero consumers) |
| Reachable from shipped entry point? | NO — crate-level: nothing depends on `platform-macos` (see coreaudio.rs audit F-003); the gated modules are additionally compiled ONLY on macOS, which no CI lane in this repo builds |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `cargo test -p platform-macos` on this Linux host → 16 passed / 0 failed (fresh; proves the non-macOS compile path the doc claims); sed quote verification; rg `WrongOs` (capability wiring asserted at tests/seam_behaviour.rs:174,403) |

## 1. One-paragraph verdict
Twenty-two lines doing exactly one thing: cfg-gate four macOS FFI modules (coreaudio, dialogs, metal, objc) behind `target_os = "macos"`, with a candid evidence-status header. Every doc claim in the file is verified except the cross-compile one: I ran the crate's Linux test suite fresh (16 passed) proving "the crate still compiles" with the module compiled out, and the WrongOs capability honesty is asserted by the crate's own tests on this host. What remains is the structural verdict: this hub gates into a crate no shipped binary contains, and the "compile-verified via cargo check --target aarch64-apple-darwin" claim (repeated here from the other files) still has no surviving artifact in the repository.

## 2. What actually works (verified)
— `cargo test -p platform-macos` (Linux host) → `16 passed; 0 failed` — the cfg-out path compiles and the typed-fallback/negotiation tests pass. This executes the doc claim "the crate still compiles and its capability report says `WrongOs` honestly" (asserted at tests/seam_behaviour.rs:403: `Capability::Unavailable(UnavailableReason::WrongOs { crate_os: "macos" })`).
— All four gated submodules exist on disk (src/macos/{coreaudio,dialogs,metal,objc}.rs — verified by ls; coreaudio.rs and dialogs.rs spot-audited in my other files).

## 3. Findings
F-001 [S3]
Where: crates/platform-macos/src/macos/mod.rs:15-22
Code:
```
#[cfg(target_os = "macos")]
pub mod coreaudio;
#[cfg(target_os = "macos")]
pub mod dialogs;
#[cfg(target_os = "macos")]
pub mod metal;
#[cfg(target_os = "macos")]
pub mod objc;
```
What is wrong: double unreachability. (1) The parent crate has zero consumers repo-wide (rg evidence in coreaudio.rs audit F-003), so even on macOS nothing shipped pulls these modules in; (2) the modules compile only on macOS, and no CI lane in this repository builds a macOS target (sandbox has no macOS SDK; baseline logs contain no darwin build). The real FFI behind this gate has therefore never been compiled by any recorded machine-verifiable run in the repo's evidence trail. What correct looks like: a CI lane (or checked-in cross-check log) that at minimum `cargo check`s the darwin target, plus a consumer for the crate. Confidence: HIGH — Verification: rg + baseline inspection + fresh Linux test run.
(The "compile-verified via cargo check --target ..." claim is unprovable from the repo — same S4 as dialog.rs F-002; counted there, not double-counted here.)

## 4. Per-function table
Not applicable — no functions; the file is 4 module declarations + docs (22 lines total, under the 100-LOC table threshold).

## 5. Test honesty
No tests in this file. The crate's host-runnable suite (16 tests, executed fresh by me) covers the cfg-out path: typed fallbacks, capability WrongOs reporting, event queue plumbing. The cfg-IN path (the four modules) has zero runtime and zero recorded compile coverage (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "On any other OS every one of these is replaced by the typed fallbacks ... (the crate still compiles and its capability report says `WrongOs` honestly)" | mod.rs:3-5 | TRUE (EXECUTED) | `cargo test -p platform-macos` → 16 passed on Linux; WrongOs asserted in seam_behaviour.rs:403 |
| "everything under this module is compile-verified via `cargo check --target aarch64-apple-darwin` and `x86_64-apple-darwin`" | mod.rs:8-10 | REAL-UNPROVEN | no artifact/log of that run exists in audit/runs/baseline/ or docs/ (rg); cannot re-run (no macOS target installed in sandbox) |
| "Runtime behavior on real Apple hardware is `[VERIFY]`" | mod.rs:12-13 | TRUE (honest) | no counter-evidence; consistent with sandbox limits |

## 7. Dead / duplicated / unreachable code
The whole gated subtree (coreaudio.rs 766 loc, dialogs.rs, metal.rs, objc.rs) is unreachable from shipped entry points — crate orphan + target gate. Mirrored hub structure across platform-windows/platform-linux is the documented [PROPOSAL] triplication.

## 8. Dependencies & flags
No deps, no unsafe in this file. `cfg(target_os = "macos")` ×4. The crate-level `#![allow(unsafe_code)]` discipline lives in each FFI module, not here.

## 9. Open questions for the next agent
- metal.rs / objc.rs auditors: same double-unreachability applies; if either carries a wrong ABI constant like coreaudio's `ELEMENT_MAIN`, no recorded compile or run would ever have caught it.
- If a darwin CI lane is ever added, re-run the coreaudio F-001 check (element constant) first — it is the highest-probability real defect in this crate.
