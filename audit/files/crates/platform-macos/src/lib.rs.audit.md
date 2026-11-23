# AUDIT: crates/platform-macos/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W08-A05 |
| Date | 2026-11-23 |
| Lines / functions | 210 / 8 (discover, report, audio, gpu, dialogs, audio_seam, gpu_seam, dialog_seam) |
| File verdict | REAL-UNPROVEN (fallback arms REAL and test-executed on Linux; the macOS arms are real wiring never executed) |
| Reachable from shipped entry point? | NO — zero workspace crates depend on platform-macos (rg over all Cargo.tomls: no matches). Sole caller of `MacOsPlatform::discover` is the crate's own tests/seam_behaviour.rs. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | diff vs platform-windows/src/lib.rs (exit 0 diff shown, 190 diff lines); rg dependents (exit 1 = none); sed quote-verification (exit 0); baseline cite cargo-test-workspace.txt:4004-4042 |

## 1. One-paragraph verdict
This is platform-windows/src/lib.rs with `s/Windows/macOS/` applied: same struct shape, same three seam builders, same one-verdict-covers-two-rows report, byte-identical control flow (diff shows only identifier/doc renames across 190 lines). The wiring logic itself is honest and the non-macOS arms are actually executed (16/16 baseline tests, including `discover_on_a_foreign_host_reports_wrong_os_everywhere` asserting every feature reports WrongOs and every seam refuses with the typed reason), so the Linux-side claims are REAL. The macOS arms (`#[cfg(target_os = "macos")]`) call `CoreAudioDevices::enumerate`, `MetalDevices::enumerate` and `AppKitPanels::probe` and have never run on any machine in this repo's evidence. The crate as a whole is an island: nothing in the workspace depends on it, so `MacOsPlatform::discover` is unreachable from lumen-cli/desktop. The module doc's integration claims check out against the implementation files I read (main-thread guard verified in macos/dialogs.rs:106-119).

## 2. What actually works (verified)
- Fallback-arm execution: baseline cargo-test-workspace.txt:4016-4034 (16/16 ok) — `discover_on_a_foreign_host_reports_wrong_os_everywhere` exercises `discover()`'s non-macOS path end-to-end on this host.
- Doc-claim cross-checks: "enforced main-thread guard" (lib.rs:36) — REAL, verified in macos/dialogs.rs run_panel (is_main_thread() check, pthread_main_np); "one CoreAudio enumeration round; one Metal device round; one non-intrusive AppKit panel-class lookup" (lib.rs:92-96) — matches audio_seam/gpu_seam/dialog_seam bodies; probe in macos/dialogs.rs is class-lookup-only (no UI, no thread requirement), so running it inside discover() from any thread is correct.
- diff of the two lib.rs files quantifies the duplication (see F-002).

## 3. Findings
F-001 [S3]
Where: whole file; evidence: rg over all Cargo.tomls
Code:
```
    pub fn discover() -> MacOsPlatform {
        let mut report = PlatformReport::new();
        let (audio, audio_capability) = audio_seam();
        let (gpu, gpu_capability) = gpu_seam();
        let (dialogs, dialogs_capability) = dialog_seam();
```
What is wrong: the crate's only entry point (`MacOsPlatform::discover`, lib.rs:98) has zero callers outside its own test file; no workspace manifest lists platform-macos as a dependency. 210 lines of seam-assembly code — and the entire CoreAudio/Metal/AppKit stack beneath it — cannot be reached from any shipped entry point. Same island status as platform-windows (my W08-A05 dialogs.rs audit F-002) and (unverified here, same pattern) platform-linux.
Why it matters: the "macOS platform adapter" adapts nothing in the product; capability reports it computes are consumed by nobody.
Confidence: HIGH — Verification: ran rg (exit 1, no dependents).
F-002 [S4]
Where: whole file vs crates/platform-windows/src/lib.rs (and platform-linux/src/lib.rs per its header)
Code:
```
//! The seam vocabulary (error/capability/events) is deliberately
//! mirrored across the three `platform-*` crates for the Lead to
//! unify into a shared L5 crate later — `[PROPOSAL]` (creating a
//! fourth crate was outside this card's file ownership).
```
What is wrong: the mirroring is a declared, acknowledged triplication — diff vs platform-windows/src/lib.rs shows only rename-level differences (190 diff lines: Windows/macOS, WASAPI/CoreAudio, DXGI/Metal, IFileDialog/NSOpenPanel, windows/macos module names). Every future seam change (e.g. adding a Feature row or fixing the Windows probe bug I filed) must be hand-copied three times; the "Lead to unify later" proposal has not landed.
Confidence: HIGH — Verification: ran diff (output quoted in commands).
F-003 [S4]
Where: lib.rs:38-45 (module doc)
Code:
```
//! **Evidence status (sandbox honesty):** this Linux sandbox has no
//! macOS SDK — the FFI surface is compile-verified via
//! `cargo check --target aarch64-apple-darwin` and
//! `x86_64-apple-darwin` (typechecked declarations, no linking, no
//! execution); runtime behavior on real Apple hardware is
```
What is wrong: cross-target compile claim not re-runnable in this sandbox (no rustup, no darwin targets) — UNVERIFIED; recorded at worklog.md:556. Typecheck-only means even when it ran it did not link against CoreAudio/CoreFoundation/libobjc.
Confidence: MEDIUM — Verification: UNVERIFIED (toolchain absent).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MacOsPlatform::discover | 98-116 | REAL (non-macOS arm, test-executed) / REAL-UNPROVEN (macOS arm) | S3 | F-001: no shipped caller; one capability verdict covers enumeration+notifications by design (comment lines 103-105) |
| report | 120-122 | REAL | — | pure accessor, exercised via tests |
| audio | 125-127 | REAL | — | &mut dyn accessor; exercised on fallback |
| gpu | 130-132 | REAL | — | same |
| dialogs | 135-137 | REAL | — | same |
| audio_seam | 143-164 | REAL (fallback arm) / REAL-UNPROVEN (macos arm) | — | calls CoreAudioDevices::enumerate (never executed) |
| gpu_seam | 168-189 | REAL (fallback arm) / REAL-UNPROVEN (macos arm) | — | calls MetalDevices::enumerate (never executed) |
| dialog_seam | 193-210 | REAL (fallback arm) / REAL-UNPROVEN (macos arm) | — | calls AppKitPanels::probe (never executed); probe is class-lookup-only — correct non-intrusive shape, no Windows-probe-style init bug (unlike platform-windows dialogs probe, my F-001 there) |

## 5. Test honesty
No tests in this file; tests/seam_behaviour.rs (16/16 baseline pass) covers discover()'s fallback arm, capability conversion, and seam refusals thoroughly — these WOULD fail if the fallback wiring were broken (assertions on exact WrongOs/NotCompiledForThisOs values). But the macOS arms have zero coverage: deleting macos/coreaudio.rs, macos/metal.rs, macos/dialogs.rs and objc.rs entirely would not fail any test in the repo. Honest for what it tests; silent about everything this crate exists for.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "an enforced main-thread guard" for dialogs | lib.rs:34-36 | REAL | macos/dialogs.rs:106-119 (pthread_main_np check + typed refusal), read-verified |
| probes: "one CoreAudio enumeration round; one Metal device round; one non-intrusive AppKit panel-class lookup" | lib.rs:92-96 | REAL | matches audio_seam/gpu_seam/dialog_seam bodies + AppKitPanels::probe (class lookups only) |
| "compile-verified via cargo check --target aarch64-apple-darwin and x86_64-apple-darwin" | lib.rs:39-41 | UNVERIFIED | worklog.md:556 records it; not re-runnable here |
| seam vocabulary "deliberately mirrored across the three platform-* crates" | lib.rs:47-50 | REAL | diff vs platform-windows/src/lib.rs = renames only |
| "it never leaks OS types upward (ARCH-6)" | lib.rs:11 | REAL (read) | seams return DeviceDesc/PathBuf etc., no raw OS types in trait signatures (dialog.rs/audio.rs read) |

## 7. Dead / duplicated / unreachable code
- Unreachable: the entire `#[cfg(target_os = "macos")]` surface from any shipped entry point (F-001).
- Duplicated: the file itself is a two-way (three-way with platform-linux) near-copy (F-002).
- No internal dead code: all functions used; `error::PlatformError` import correctly cfg-gated to the arms that need it (lib.rs:70-71).

## 8. Dependencies & flags
No cargo features (see platform-macos Cargo.toml audit — separate file). Module graph: audio/capability/dialog/error/events/gpu always compiled; macos only on target_os=macos. No unsafe in this file.

## 9. Open questions for the next agent
- platform-linux/src/lib.rs is claimed to mirror this — its auditor should confirm and file the same triplication finding.
- When the Lead unifies the L5 seam crate (the [PROPOSAL]), the Windows probe bug (my dialogs.rs F-001) is the kind of per-platform defect that triplicated silently — check the unified crate fixes it.
