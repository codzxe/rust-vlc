# AUDIT: crates/plugin-host-wasm/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W12-A09 |
| Date | 2026-11-21 |
| Lines / functions | 120 / 15 items (13 `pub mod` decls, headline re-export block, 1 test) |
| File verdict | UNREACHABLE (crate root of a crate no app depends on; everything below it is test-only reachable) |
| Reachable from shipped entry point? | NO. Grep `plugin_host_wasm` over all `*.rs`: only `src/bin/lumenext.rs` (which itself is unshipped) and the crate's own `tests/`. No `apps/*` Cargo.toml depends on `plugin-host-wasm`. Shipped bins: `lumen`, `desktop`, `lumen-desktop-tauri` (scripts/packaging/gate.sh:85). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | Read whole file; Grep `unsafe` in src (only comments/const-name — the no-unsafe claim holds); Grep re-exported symbols (`SupervisorEvent` etc.) → all resolve; Read Cargo.toml (wasmi pin, lints); Grep `plugin_host_wasm` consumers; sed -n verification of quotes; cross-refs to my install/instance/limits audits. |

## 1. One-paragraph verdict
The crate root is an accurate map with one lie and two cosmetic defects. The module docs describe each module honestly (verified against my own audits of install/instance/limits and the caller maps: F1's no-host-pointers model and `[1,1]` window are real; F3's "honestly unverified" signature posture is real; F4's supervisor exists with settings-schema keys `extensions.crash-*` present in schemas/settings.toml:1409+), the wasmi 2.0.0 pin matches Cargo.toml, the no-`unsafe` claim is true (grep), and every re-exported symbol resolves. The lie is the headline safety posture: "bounded memory everywhere (D-2) … never a panic" is contradicted by `instance.rs read_guest_memory`, which sizes a host allocation from a guest-controlled length with no cap (my instance.rs F-001, S1) — the claim should not survive in the crate's front door. The cosmetics: the "three cards" heading introduces four cards (F4 was appended without updating the heading), and the one in-file test is named `headline_surface_is_wired` while its body is a compile-plus-window check that wires nothing.

## 2. What actually works (verified)
— Verified by command: `grep unsafe crates/plugin-host-wasm/src` → zero `unsafe` blocks (claim TRUE); all 5 supervisor re-exports + 10 other re-exported symbols resolve in their modules; `ENGINE_PLUGIN_API_MIN == 1` and `plugin_api::ENGINE_PLUGIN_API_MAJOR == 1` are consistent with api.rs:92 and the negotiate window `[1, 1]` (api.rs:97-107).
— Compilation of the whole crate (this file included) proven by `audit/runs/baseline/build-release-workspace.log.txt:231`; the in-file test has no baseline run (crate's tests never reached in either workspace test log).

## 3. Findings

### F-001 [S2]
Where: crates/plugin-host-wasm/src/lib.rs:46-50
Code:
```rust
//! # No-unsafe, bounded, typed
//!
//! No `unsafe` (AGENTS §3.3); bounded memory everywhere (D-2); every
//! failure is a typed [`error::HostError`] with an actionable message
//! (MP §8) — never a panic, never a hang, never a swallowed `Result`.
```
What is wrong: "bounded memory everywhere" is false. `WasmInstance::read_guest_memory` (instance.rs:274-288, my S1 finding there) allocates `vec![0u8; len as usize]` from a caller-supplied — in the documented v1 protocols, guest-reported u32 — length with no cap and before the bounds check; a large length is a multi-GiB host allocation attempt, an extreme one a capacity-overflow abort. "never a panic" is likewise contradicted by the abort path.
Why it matters: this is the crate's front-door safety posture, the sentence an embedder reads first; it over-promises exactly the property the sandbox exists for. The mechanism is already counted at S1 in instance.rs; this row is for the false claim living in this file.
What correct looks like: cap the read path (instance.rs fix), or scope the claim honestly ("bounded guest memory; host-side read-outs capped at io_transfer_bytes").
Confidence: HIGH — Verification: read + the instance.rs trace (grep callers; allocation-before-check order).

### F-002 [S4]
Where: crates/plugin-host-wasm/src/lib.rs:13-14
Code:
```rust
//! # The three cards
//!
//! - **F1 — runtime** ([`instance`], [`api`], [`imports`]): wasmi
```
What is wrong: the heading says "three cards"; the list under it has four (F1, F2, F3, and F4 "card 7.F4 / issue #46" appended at 32-35).
Why it matters: heading/body count mismatch — the F4 append did not update the heading; trivial but exactly the kind of stale-doc drift this audit exists to catch.
What correct looks like: "The four cards" (or "The cards").
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/plugin-host-wasm/src/lib.rs:107-119
Code:
```rust
    #[test]
    fn headline_surface_is_wired() {
        // One-touch compile check: the embedding surface exists and the
        // version window matches plugin-api's contract major.
        assert_eq!(ENGINE_PLUGIN_API_MIN, 1);
```
What is wrong: the test name promises that the headline surface is WIRED; the body (honestly commented as "One-touch compile check") proves only that the symbols exist and the version window matches. Nothing wires the surface — no embedder exists in the repository.
Why it matters: test-name-versus-body gap (the checklist's "names promising more than the body"); a future reader grepping for wiring proof finds a name that asserts it and a body that does not deliver it.
What correct looks like: rename to `headline_surface_compiles_and_window_matches`, or add an actual wiring assertion once an embedder exists.
Confidence: HIGH — Verification: read; embedder absence proven by the header-row grep.

## 4. Per-item table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Module doc block (whole-crate description) | 1-75 | REAL (one false claim) | S2 | F1-F4 descriptions verified against my audits; wasmi pin, module map (13 mods, all present), no-unsafe TRUE; "bounded memory everywhere" FALSE (F-001); "The three cards" count bug (F-002); "nothing depends on this crate but the apps that embed it" (9-11) is vacuously true — zero apps embed it (see §6). |
| `pub mod api` … `pub mod supervisor` (13 decls) | 77-89 | REAL | — | All 13 modules exist with content (fsjail/http/imports/package/state/supervisor audited by other W12 agents; api/crc32/error/sha256 likewise assigned). |
| Re-export block | 91-101 | REAL | — | All symbols resolve (grep); `engine`, `WasmInstance`, `ExtensionStore`, `ResourceLimits`, supervisor types; omits `LoadedExtension`/`InstallPreview` (available via `install::` path — lumenext/tests use it). |
| test `headline_surface_is_wired` | 107-119 | REAL (misnamed) | S4 | F-003; mutation-sensitive only for window drift (changing ENGINE_PLUGIN_API_MIN fails it). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `headline_surface_is_wired` | PARTIAL | Fails if the API window drifts (MIN≠1 or plugin-api major≠1) or if any re-export stops compiling; proves nothing about wiring, execution, or behavior. |
| Baseline evidence | NO RUN | Crate's tests absent from both baseline workspace test logs. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "bounded memory everywhere (D-2) … never a panic" | lib.rs:48-50 | FALSE | instance.rs:274-288 (my S1 there). |
| "No `unsafe`" | lib.rs:48 | TRUE | grep `unsafe` in src → comments only. |
| "wasmi 2.0.0 per ADR-002 … pure-Rust interpreter, fuel metering, no WASI" | lib.rs:2-3 | TRUE | Cargo.toml:57 pin; instance.rs fuel usage; no WASI imports (api.rs catalog is `lumen:v1.*` only). |
| "nothing depends on this crate but the apps that embed it" | lib.rs:9-11 | TRUE-vacuous / misleading | Zero apps embed it (grep); the phrasing implies embedding apps exist. |
| "policy values come from the settings layer (`extensions.crash-*`), never re-stated in code" | lib.rs:43-44 | REAL-UNPROVEN | Keys exist (schemas/settings.toml:1409-1429); supervisor.rs is another agent's file — unverified by me. |
| feature-matrix `plugin-wasm-extensions` not_started (all platforms), evidence "" | docs/feature-matrix.toml:2321-2331 | TRUE (honest) | No embedder; product.toml:166 `wasm-extensions = false` (Phase 7). |

## 7. Dead / duplicated / unreachable code
- The entire crate is production-unreachable (header row) — this file's re-exports are the surface an embedder would use; none exists.
- No dead symbols at this level; `HostCapacity`/host-accounting deadness is recorded in my limits.rs audit (F-001).

## 8. Dependencies & flags
- Crate deps: plugin-api (path), settings-schema (path, supervisor's settings plane), toml 1, wasmi 2.0.0 (default-features=false, features std/validate/auto-dispatch). Dev-deps: playback-engine + media family (the 7.F4 in-process isolation proof — dev-only, boundary-exempt per Cargo.toml comment). `[lints] workspace = true`; two bins (`lumenext`, `lumenext-examples`). No feature gates on the lib.

## 9. Open questions for the next agent
- supervisor.rs auditor: verify the lib.rs:43-44 claim (crash policy read through settings, never re-stated) — my cross-check stopped at the schema keys.
- Wave-17 (packaging): should `lumenext`/`lumenext-examples` be shipped as dev tools, or excluded from the release workspace build to stop paying their compile time in every lane?
