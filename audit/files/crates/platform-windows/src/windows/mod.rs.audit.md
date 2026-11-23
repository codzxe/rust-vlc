# AUDIT: crates/platform-windows/src/windows/mod.rs
| Field | Value |
|---|---|
| Auditor | W08-A03 |
| Date | 2026-11-23 |
| Lines / functions | 21 / 0 (module index: 4 cfg-gated `pub mod` declarations) |
| File verdict | REAL (module index, honest) — the integrations it indexes are REAL-UNPROVEN (compile-verified-only hand-rolled COM FFI) / UNREACHABLE from shipped entry points (orphan crate) |
| Reachable from shipped entry point? | NO — no crate outside platform-windows depends on it (rg over every Cargo.toml); lumen-cli/desktop dependency lists exclude it |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p platform-windows` → 16 passed (incl. `discover_on_a_foreign_host_reports_wrong_os_everywhere`), exit 0; `rustup target list --installed` → wasm32 + x86_64-unknown-linux-gnu ONLY (no windows-msvc rust-std); `rg` FFI spot-checks of the four submodules; `sed -n` quote verification |

## 1. One-paragraph verdict
A 21-line cfg-gated module index whose only content is four `pub mod` declarations (com, dialogs, dxgi, wasapi) behind `target_os = "windows"` plus an evidence-status header. The header is unusually honest: it volunteers that this sandbox has no Windows SDK, that everything under the module is declarations-only compile-verified, and that runtime Windows behavior is `[VERIFY]` — the exact opposite of the usual stub-hiding pattern. The integrations it indexes are structurally real: wasapi.rs (787 lines, `IMMDeviceEnumerator` vtable, `EnumAudioEndpoints`, a hand-rolled notification client with AddRef/Release), dxgi.rs (264 lines, `CreateDXGIFactory1` + `EnumAdapters1` + `GetDesc1`), dialogs.rs (596 lines, `IFileOpenDialog`), com.rs (415 lines, `CoInitializeEx`) — all genuine hand-declared COM ABI, no windows-s crate, no todo!/unimplemented! (rg). Two caveats: the "compile-verified via cargo check --target x86_64-pc-windows-msvc" claim is UNVERIFIABLE in this environment (the target's rust-std is not installed; no in-repo log records the run — wave-0 agents W08-A01 and W08-A05 hit the same wall for their crates), and the crate ships to nobody like its two sibling platform crates.

## 2. What actually works (verified)
— verified by `cargo test -p platform-windows` → 16 passed, exit 0 (this Linux host): the cfg gating works as documented — the windows modules compile out, the typed fallbacks and the all-`WrongOs` capability report are what actually run, and the foreign-host test pins that.
— verified by `rg` spot-checks: all four submodules contain real `unsafe extern "system"` COM vtable declarations matching the claimed integrations (no placeholders found).

## 3. Findings
F-001 [S3]
Where: crates/platform-windows/src/windows/mod.rs:1 (whole crate) and dependency reality
Code:
```
//! The Windows integrations (cfg-gated to `target_os = "windows"`):
```
What is wrong: orphan crate — zero dependents outside platform-windows; no shipped binary compiles these modules for Windows (nothing depends on the crate at all). 2 083 lines of real COM FFI integration shipping to nobody; any feature-matrix row citing WASAPI/DXGI/Common-Dialogs as a product capability is unreachable-by-wiring. Confidence: HIGH — Verification: read (rg over all Cargo.toml).
F-002 [S4]
Where: crates/platform-windows/src/windows/mod.rs:7-10
Code:
```
//! sandbox has no Windows SDK — everything under this module is
//! compile-verified via `cargo check --target x86_64-pc-windows-msvc`
//! (declarations typechecked, no linking, no execution). Runtime
```
What is wrong: the compile-verification claim cannot be checked here: `rustup target list --installed` shows only wasm32 + x86_64-unknown-linux-gnu (no windows rust-std), and no worklog/baseline file records the check being run. The claim is documentary, not reproducible — same class of unverifiable cross-compile claim already flagged by wave-0 agents W08-A01 and W08-A05. Not rated FALSE (the check may have run at authoring time); rated UNVERIFIED. What correct looks like: a committed log of the cross-check (as rc-smoke does for wasm). Confidence: HIGH that it is unverifiable — Verification: UNVERIFIED (target absent).

## 4. Per-function table
Not applicable — 21-line module index, no functions (<100 LOC threshold). Module inventory: com.rs 415 L, dialogs.rs 596 L, dxgi.rs 264 L, wasapi.rs 787 L (all `#[cfg(target_os = "windows")]`).

## 5. Test honesty
No tests in this file. Host-runnable coverage lives in tests/seam_behaviour.rs (16 tests, all pass on this host — they exercise the typed fallbacks, not the Windows code, which is exactly what the module doc promises).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cfg-gated to target_os = windows" | mod.rs:1 | TRUE | mod.rs:14-21 four cfg attrs; crate compiles+tests green on Linux with modules compiled out (my run) |
| "the WASAPI MMDevice client, the DXGI adapter scanner, the Common File Dialogs" exist | mod.rs:2-3 | REAL-UNPROVEN | wasapi.rs/dxgi.rs/dialogs.rs contain the real FFI declarations (rg spot-checks); never compiled-for-windows or executed in any verifiable lane |
| "On any other OS … typed fallbacks … capability report says WrongOs honestly" | mod.rs:3-5 | TRUE (executed) | `discover_on_a_foreign_host_reports_wrong_os_everywhere ... ok` (my run) |
| "compile-verified via cargo check --target x86_64-pc-windows-msvc" | mod.rs:9 | UNVERIFIED | F-002: target rust-std not installed; no in-repo record |
| "Runtime behavior on real Windows is [VERIFY]" | mod.rs:11 | Honest caveat | — |

## 7. Dead / duplicated / unreachable code
Whole crate unreachable from shipped entry points (F-001). The gpu.rs/audio.rs seam-vocabulary files are the same triple-duplication pattern noted in my platform-macos/src/gpu.rs audit (same PCI table, same 0x1b36 "VMware" label — UNVERIFIED against the PCI DB there).

## 8. Dependencies & flags
Std-only; all FFI behind `#[cfg(target_os = "windows")]`; `#![allow(unsafe_code)]` lives in the submodules (not this file). No feature gates.

## 9. Open questions for the next agent
- Whoever audits wasapi.rs/dxgi.rs/dialogs.rs/com.rs: the vtable slot comments (`_activate`, `_get_state`, `_create_swap_chain` prefixed with underscore = deliberately un-declared slots) deserve a slot-order audit against the real COM headers — a mis-ordered vtable is a memory-safety bug that compile-check CANNOT catch (field names don't enforce slot order).
- Is there ANY CI lane with the windows/msvc and apple-darwin rust-std components? If not, every "compile-verified" claim across the three platform crates is unverifiable-by-automation (see also W08-A01/A05 logs).
