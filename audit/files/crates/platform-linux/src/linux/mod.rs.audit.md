# AUDIT: crates/platform-linux/src/linux/mod.rs
| Field | Value |
|---|---|
| Auditor | W08-A06 |
| Date | 2026-11-23 |
| Lines / functions | 16 / 0 (5 cfg-gated module declarations) |
| File verdict | REAL (code) / UNREACHABLE (from shipped entry point) |
| Reachable from shipped entry point? | NO — orphan crate (F-001 below). Within the crate: all five modules are consumed (verified: pulse.rs:47-48 uses dl+pulse_ffi; lib.rs:144 pulse, 178 drm, 206 xdg). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `~/.cargo/bin/cargo check -p platform-linux --target wasm32-unknown-unknown` -> exit 0, "Finished `dev` profile ... in 0.97s" (compiles the `not(target_os = "linux")` arms); `rg "linux::(dl|drm|pulse|pulse_ffi|xdg)"` (usage); `rg platform-linux` over all Cargo.tomls (no dependents) |

## 1. One-paragraph verdict
This is a pure module hub: five `#[cfg(target_os = "linux")]` declarations, each module real and consumed within the crate. Nothing is wrong inside the 16 lines. The file is the right place to record the crate-level verdict for the whole `platform-linux` integration surface: the crate is an orphan — no workspace Cargo.toml depends on it, so none of the 2719 LOC of Linux integrations it gates (dl 170 + drm + pulse 759 + pulse_ffi 402 + xdg 350 + vocabulary) is reachable from the shipped lumen-cli/desktop release lane; its only callers are its own tests. The doc's cross-OS claim compiles true (wasm32 check passes, exercising the not-linux arms) and the WrongOs pattern is runtime-verified in the mirrored platform-windows crate on this host; runtime of platform-linux itself on a genuinely foreign OS is UNVERIFIED (needs a non-Linux host).

## 2. What actually works (verified)
- `cargo check -p platform-linux --target wasm32-unknown-unknown` -> exit 0 (0.97s): the crate compiles with every module here cfg'd out — the "crate still compiles [on any other OS]" claim holds at the type level.
- All five modules have real consumers (rg evidence in §Reachability) — no dead module declarations.

## 3. Findings
F-001 [S3]
Where: crates/platform-linux/src/linux/mod.rs:7-16 (crate-level finding; filed here as the integration hub)
Code:
```rust
#[cfg(target_os = "linux")]
pub mod dl;
#[cfg(target_os = "linux")]
pub mod drm;
#[cfg(target_os = "linux")]
pub mod pulse;
#[cfg(target_os = "linux")]
pub mod pulse_ffi;
#[cfg(target_os = "linux")]
pub mod xdg;
```
What is wrong: The entire integration surface this file gates is dead weight in the release lane. `rg platform-linux` over every Cargo.toml in the workspace (crates/*, apps/*, apps/desktop-tauri) matches only the crate's own manifest — lumen-cli's dependency list (apps/cli/Cargo.toml:47-65) and desktop's (apps/desktop/Cargo.toml:32-43) do not include it. The only callers of `LinuxPlatform::discover` are the crate's own tests. Why it matters: ~2.7k LOC of real, tested platform integration (PulseAudio client, DRM scanner, xdg dialog chain) ship nowhere; the honest capability report they produce is read by no product code. What correct looks like: phase-7 wiring per the docs (docs/feature-matrix.toml platform rows are phase_target 7; lib.rs:44-47 records the Lead's unification [PROPOSAL]) — until then this is UNREACHABLE by design, not by accident, which keeps it S3 rather than S2. Confidence: HIGH — Verification: ran rg over all manifests; lib.rs + feature-matrix phase_target cited.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mod dl (gate) | 7-8 | REAL | — | Used by pulse.rs:47, pulse_ffi.rs:30 |
| mod drm (gate) | 9-10 | REAL | — | Used by gpu seam (lib.rs:178); tested with fixture trees |
| mod pulse (gate) | 11-12 | REAL | — | Used by audio seam (lib.rs:144); tested on this host (typed absent-server path) |
| mod pulse_ffi (gate) | 13-14 | REAL | — | Used by pulse.rs:48,356 |
| mod xdg (gate) | 15-16 | REAL | — | Used by dialog seam (lib.rs:206); tested with stub binaries |

## 5. Test honesty
No tests in this file. The modules' tests are covered in the linux_integrations.rs audit (13/13 executed) and the drm/pulse/dl/ffi files belong to other agents.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "On any other OS every one of these is replaced by the typed fallbacks ... (the crate still compiles ...)" | mod.rs:3-5 | REAL | Executed `cargo check -p platform-linux --target wasm32-unknown-unknown` -> exit 0 (not-linux arms typecheck) |
| "... and its capability report says `WrongOs` honestly" | mod.rs:4-5 | REAL-UNPROVEN | The WrongOs runtime path is executed for the mirrored platform-windows crate on this host (`discover_on_a_foreign_host_reports_wrong_os_everywhere` ok); platform-linux itself on a foreign OS not executed — UNVERIFIED (needs non-Linux host; wasm test runner not configured) |

## 7. Dead / duplicated / unreachable code
- F-001: the whole crate is unreachable from shipped entry points.
- Mirrors: platform-windows/src/windows/mod.rs and platform-macos analogues (documented [PROPOSAL] unification).

## 8. Dependencies & flags
- No code, no deps. The cfg gates are correct and complete: every module is linux-only and every consumer is inside `#[cfg(target_os = "linux")]` blocks in lib.rs (137-160, 176-185, 203-231).

## 9. Open questions for the next agent
- drm.rs / dl.rs / pulse.rs / pulse_ffi.rs deep audit (unsafe blocks, FFI invariants) — assigned elsewhere; nothing in this hub file can substitute.
