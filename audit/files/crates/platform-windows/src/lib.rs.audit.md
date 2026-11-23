# AUDIT: crates/platform-windows/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W08-A06 |
| Date | 2026-11-23 |
| Lines / functions | 216 / 8 (discover, report, 3 accessors, 3 seam builders) |
| File verdict | REAL-UNPROVEN (Windows arms never executed anywhere) / UNREACHABLE (from shipped entry point) |
| Reachable from shipped entry point? | NO — orphan crate: no Cargo.toml in the workspace lists platform-windows as a dependency (verified by rg over crates/*/Cargo.toml, apps/*/Cargo.toml, apps/desktop-tauri/Cargo.toml). Only caller of `WindowsPlatform::discover` is its own test suite. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/seam_behaviour-3bda819001459956` -> 16 passed / 0 failed exit 0; `rg platform-windows` over all Cargo.tomls (no dependents); `ls`/`stat` of target/x86_64-pc-windows-msvc artifacts; `od -c` line 97 |

## 1. One-paragraph verdict
This is the composition root of the Windows platform adapter, and on this Linux host every Windows-specific arm of it is cfg'd out: what compiles and runs here is three seam builders returning typed `NotCompiledForThisOs` fallbacks — which the crate's own tests verify honestly (16/16 pass, including `discover_on_a_foreign_host_reports_wrong_os_everywhere`). The real Windows code (2083 LOC of hand-declared COM vtables in src/windows/{com,dialogs,dxgi,wasapi}.rs) exists but has never executed: CI-Full's windows lane has never run on a real runner (ci-full.yml header: "the windows/macos lanes have NEVER executed on real runners"), and the doc's claimed sandbox cross-check left corroborating artifacts (`/tmp/my-project/target/x86_64-pc-windows-msvc/debug/deps/libplatform_windows-7104d417b50f5e9a.rmeta`, 2026-11-21 07:07) that I cannot re-confirm against current source because the msvc rust-std is not installed in the current toolchain. The crate is an orphan: nothing in the release lane (lumen-cli, desktop) depends on it. Runtime Windows behavior = [VERIFY], exactly as the doc admits — this file's honesty is its strongest property.

## 2. What actually works (verified)
- Ran prebuilt test binary `./target/debug/deps/seam_behaviour-3bda819001459956` -> `test result: ok. 16 passed; 0 failed` exit 0 — covers the non-Windows fallback arms, capability conversion, typed refusals, event queue (matches baseline cargo-test-workspace.txt:4054-4074).
- Foreign-OS honesty verified by execution: on this host `discover()` reports `WrongOs` for all four features (test above).

## 3. Findings
F-001 [S3]
Where: crates/platform-windows/src/lib.rs:143-153 (and identically 170-180, 197-206)
Code:
```rust
fn audio_seam() -> (Box<dyn AudioDevices>, Capability) {
    #[cfg(target_os = "windows")]
    {
        let mut devices = windows::wasapi::WasapiAudioDevices::new();
        match devices.enumerate() {
            Ok(_) => (Box::new(devices), Capability::Available),
```
What is wrong: The probe enumerates the full device/adapter list and discards it (`Ok(_)`), then every later `enumerate()` call on the seam re-runs the whole round — on Windows, `discover()` performs one throwaway WASAPI enumeration plus one throwaway DXGI factory round, and the first real use pays the cost again. Why it matters: doubled COM/STA work at startup on the one platform where these threads exist; not a correctness bug. What correct looks like: cache the probe's device list in the client and serve it from the first `enumerate()`. Confidence: HIGH — Verification: read (Windows arm execution UNVERIFIED — no Windows host).
F-002 [S3]
Where: crates/platform-windows/Cargo.toml:27 (name) — finding filed against lib.rs as the crate root
Code:
```rust
pub mod audio;
pub mod capability;
```
What is wrong: Orphan crate. No workspace crate depends on platform-windows (rg over every Cargo.toml: only its own name matches), so this composition root and the 2083 LOC of FFI below it are dead weight in the release lane; `WindowsPlatform::discover` is called only from `tests/seam_behaviour.rs`. The lib.rs doc (lines 47-50) admits the three platform crates await Lead unification — planned phase-7 wiring, but today the code is UNREACHABLE from lumen-cli/desktop. Confidence: HIGH — Verification: rg command above. (Counted once per audit; cross-file duplication finding below.)
F-003 [S4]
Where: crates/platform-windows/src/lib.rs:47-50
Code:
```rust
//! The seam vocabulary (error/capability/events) is deliberately
//! mirrored across the three `platform-*` crates for the Lead to
//! unify into a shared L5 crate later — `[PROPOSAL]` (creating a
//! fourth crate was outside this card's file ownership).
```
What is wrong: Documented duplication: audio.rs/capability.rs/dialog.rs/error.rs/events.rs/gpu.rs/lib.rs are near-copies across platform-{linux,windows,macos} (structure of this lib mirrors platform-linux/src/lib.rs function-for-function). The doc is honest about it; it is still 6 files x 3 crates of copy-paste vocabulary to keep in sync until unification. Confidence: HIGH — Verification: read + structural comparison of the two lib.rs files.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| WindowsPlatform (struct) | 82-87 | REAL | — | Three Box<dyn> seams + report |
| discover | 89-116 | REAL-UNPROVEN (Win) / REAL (non-Win refusal) | — | Non-Windows path executed by test; Windows probes never executed anywhere |
| report | 118-122 | REAL | — | Pure accessor; tested |
| audio / gpu / dialogs | 124-137 | REAL | — | dyn accessors; tested via seam traits |
| audio_seam | 143-166 | REAL-UNPROVEN | S3 | Windows arm: probe result discarded (F-001); foreign-OS arm executed |
| gpu_seam | 168-193 | REAL-UNPROVEN | S3 | Same pattern as audio_seam |
| dialog_seam | 195-216 | REAL-UNPROVEN | — | `CommonItemDialogs::probe()` never executed on Windows; foreign-OS arm executed |

## 5. Test honesty
Covered by the executed run (16/16): `discover_on_a_foreign_host_reports_wrong_os_everywhere` genuinely pins the refusal path; `typed_fallbacks_satisfy_the_seam_traits`, `capability_classifies_and_converts_probe_errors` etc. exercise the compiled-on-this-host surface. NO test anywhere exercises the `#[cfg(target_os = "windows")]` arms — they cannot, on Linux. Mutation reasoning: breaking the Windows arms would not fail any test in the repo.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the FFI surface is compile-verified via `cargo check --target x86_64-pc-windows-msvc`" | lib.rs:40-42 | REAL-UNPROVEN | Artifact `/tmp/my-project/target/x86_64-pc-windows-msvc/debug/deps/libplatform_windows-7104d417b50f5e9a.rmeta` (stat: 2026-11-21 07:07) proves such a check ran and typechecked the lib (rmeta emitted). Cannot re-run: msvc rust-std absent from current toolchain manifests (only wasm32 + x86_64-unknown-linux-gnu present). Cannot prove the checked source == current source (checkout stamped 2026-11-23). |
| "runtime behavior on real Windows hardware is `[VERIFY]`" | lib.rs:42-43 | REAL (honest gap) | No Windows execution exists anywhere: CI-Full windows lane never ran (.github/workflows/ci-full.yml:28-29 "the windows/macos lanes have NEVER executed on real runners") |
| "Enumeration runs on a dedicated short-lived STA thread; notifications ... long-lived thread" | lib.rs:26-30 | REAL-UNPROVEN | src/windows/wasapi.rs (787 LOC) exists; not compiled on this host, not executed anywhere — thread structure unverifiable here |
| "on any other OS every seam is the typed fallback and the report says `WrongOs` honestly" | lib.rs:59-60 | REAL | Executed: 16/16 test run incl. `discover_on_a_foreign_host_reports_wrong_os_everywhere` |

## 7. Dead / duplicated / unreachable code
- Whole crate unreachable from shipped entry points (F-002).
- All `#[cfg(target_os = "windows")]` arms: never compiled in the Linux lane, never executed in any lane (cross-check rmeta is the only compile evidence).
- Duplication with platform-linux/platform-macos vocab modules (F-003).

## 8. Dependencies & flags
- Cargo.toml: deps = audio-output, media-types; dev-dep testkit (mirrors platform-linux). No windows-sys/windows crate — FFI is hand-declared in src/windows/com.rs. `unsafe_code = deny` workspace-wide with module-level exceptions in the FFI modules (per Cargo.toml header comment; not re-verified file-by-file — out of my file list).

## 9. Open questions for the next agent
- Audit src/windows/{wasapi,dxgi,dialogs,com}.rs for unsafe-block invariants — they are in another agent's list; nothing here can substitute for that review.
- Was the 2026-11-21 cross-check run against the same commit as this snapshot? UNVERIFIED (no git access; cannot diff).
