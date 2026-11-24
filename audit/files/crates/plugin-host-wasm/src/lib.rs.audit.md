# AUDIT: crates/plugin-host-wasm/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W12-A08 |
| Date | 2025-06-27 |
| Lines / functions | 120 / 1 test (facade: 13 module decls + 30 re-exports) |
| File verdict | REAL (facade; crate is an island — not shipped) |
| Reachable from shipped entry point? | NO — zero workspace dependents (rg over every Cargo.toml: no crate depends on plugin-host-wasm); release lane builds only `-p lumen-cli -p desktop`; product.toml:166 `wasm-extensions = false` (Phase 7). Reachable from the test lane only (44 unit + 4 integration suites, all green in baseline) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `ls src/` (13 modules present); `rg "unsafe \{|unsafe fn" src/` -> exit 1 (zero unsafe blocks); `rg unreachable|unwrap` -> all hits inside `#[cfg(test)]` mods (state.rs:336 verified); `rg fuel src/instance.rs` -> set_fuel at :108/:186; baseline citations below |

## 1. One-paragraph verdict
A documentation-heavy facade that honestly maps its 13 modules and re-exports the embedding surface; the one test is a real (if thin) wiring check that pins the API window `[1,1]` to plugin-api's contract major. The crate's headline claims check out where I could execute or count: zero `unsafe` blocks in the entire src tree, all panic-capable constructs (`unreachable!`/`unwrap`) confined to `#[cfg(test)]` helper blocks, fuel metering real (`set_fuel` at instance.rs:108/:186), trap containment proven by the 6/6 crash-isolation suite, signature honestly Unverified. The one dishonest note: "nothing depends on this crate but the apps that embed it" (L10-11) — no app embeds it; the crate is an island reachable only through its own binaries and tests, and the shipped product keeps the whole feature parked.

## 2. What actually works (verified)
— `rg "unsafe \{|unsafe fn" crates/plugin-host-wasm/src/` -> exit 1: ZERO unsafe blocks in production code (the two textual "unsafe" hits are lib.rs's doc header and a test const name `MSG: &str = "unsafe path component"`, install.rs:585). The L48 claim "No `unsafe`" is TRUE.
— Panic audit: every `.unwrap()`/`unreachable!()` hit (state.rs:430/438, supervisor.rs:493-534, package.rs:415/508, http.rs:201/265, fsjail.rs:203-350 …) sits below each file's `#[cfg(test)] mod tests` line (state.rs:336-337 verified) — house test helpers, not production panics. The L50 "never a panic" claim holds for shipped code paths.
— Baseline: `test tests::headline_surface_is_wired ... ok` (cargo-test-workspace.txt:4942) inside the 44-pass unit run (:4945); crash isolation 6/6 (:4961-4968), conformance 5/5 (:4971-4981), lifecycle 4/4 (:4990-4996) — the facade's re-exported surface is exercised end-to-end by the integration suites.

## 3. Findings
F-001 [S4]
Where: crates/plugin-host-wasm/src/lib.rs:9-11
Code:
```
//! (a `.lumenext` WASM package) touches, from install to dispatch.
//! The registry/selection half (doc 06 §3) is not here. Layer L6
//! (doc 01 §2.8): nothing depends on this crate but the apps that
//! embed it.
```
What is wrong: The sentence implies embedding apps exist. In this tree, ZERO apps embed the crate: `rg "plugin-host-wasm" --glob Cargo.toml` over the workspace returns only the crate's own manifest (plus a comment in plugin-api's). apps/cli depends on plugin-api only (apps/cli/Cargo.toml:59); apps/desktop on neither. The crate's only entry points are its own `lumenext` / `lumenext-examples` binaries — neither in the release lane — and its test suites. The doc should say "no app embeds it yet (Phase 7; wasm-extensions=false)". Why it matters: the module map reads as a shipping system; the reality is a fully-tested, fully-parked island. What correct looks like: state the parked status in the header the way product.toml does. Confidence: HIGH — Verification: ran (rg dependency survey).

F-002 [S4]
Where: crates/plugin-host-wasm/src/lib.rs:103-119
Code:
```
    #[test]
    fn headline_surface_is_wired() {
        // One-touch compile check: the embedding surface exists and the
        // version window matches plugin-api's contract major.
        assert_eq!(ENGINE_PLUGIN_API_MIN, 1);
```
What is wrong: The test asserts only the MIN of the claimed "window `[1, 1]`" (L20); the max leg is pinned nowhere — `ENGINE_PLUGIN_API_MIN` is the only window constant re-exported (L92). If the window's upper bound drifted (api.rs growing a MAX constant that diverges from plugin-api's major), this test stays green. Honest about being a compile check, but the doc's window claim is half-pinned. What correct looks like: assert the max too (or assert the window type's min==max==plugin major). Confidence: HIGH — Verification: read + rg; the upper bound IS behaviorally pinned one layer down — api.rs:509-517 `version_window_is_v1_only_today` rejects `api_version` 2/3/999 with `ApiVersion` errors (baseline: `api::tests::version_window_is_v1_only_today ... ok`), so drift is caught by the suite, just not by THIS file's test.

## 4. Per-function table (file >100 LOC — facade, so the surface is the unit)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module decls (13) | 77-89 | REAL | — | all 13 files exist (ls verified); 57-917 LOC each |
| re-exports (30 symbols) | 91-101 | REAL | — | compile-verified by the crate build + the test |
| `tests::headline_surface_is_wired` | 107-119 | REAL | S4 | F-002: window max unpinned; exercises ResourceLimits::for_kind + HostServices::empty construction |

## 5. Test honesty (if tests exist)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| headline_surface_is_wired | PARTIAL | fails on any re-export removal (compile) or MIN/major drift; cannot catch window-max drift (F-002) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "runtime: wasmi 2.0.0 per ADR-002" | L2-3 | REAL | Cargo.toml `wasmi = { version = "2.0.0", … }` |
| "fuel metering" | L24-25 | REAL | instance.rs:108 `set_fuel(limits.startup_fuel)`, :186 `set_fuel(limits.call_fuel)` |
| "No `unsafe`" | L48 | REAL | rg -> zero unsafe blocks in src |
| "every failure is a typed HostError … never a panic" | L48-50 | REAL (production code) | all panic sites are #[cfg(test)] helpers (state.rs:336+) |
| "signature suite (recorded, honestly unverified…)" | L28-30 | REAL | lifecycle test asserts `SignatureStatus::Unverified` receipt+loaded (plugin_install_lifecycle.rs:454-463) |
| "a trap destroys only that instance's Store" | L39-41 | REAL | baseline crash-isolation 6/6 (:4961-4968) |
| "nothing depends on this crate but the apps that embed it" | L10-11 | MISLEADING | zero embedding apps (F-001) |
| "window `[1, 1]` today" | L20 | REAL-UNPROVEN | MIN pinned by test; max leg unpinned (F-002) |

## 7. Dead / duplicated / unreachable code
The entire crate is runtime-unreachable from the shipped product (parked Phase 7) — by design, tested, not dead. No duplication at this layer.

## 8. Dependencies & flags
plugin-api, settings-schema, toml, wasmi 2.0.0 (default-features=false, std/validate/auto-dispatch). No WASI. No feature gates on the lib. No unsafe.

## 9. Open questions for the next agent
- api.rs auditor: does a window-max constant exist, and does any test pin it to 1? (F-002.)
- When wasm-extensions flips true (Phase 7), which app embeds this crate first — CLI, desktop, or web — and does the consent UX (publisher/permissions/signature) get constructed from what was actually rendered? Cross-ref plugin_install_lifecycle.rs F-001 (permissions-leg negative test missing).
