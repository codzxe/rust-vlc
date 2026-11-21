# AUDIT: crates/platform-windows/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-21 |
| Lines / functions | 216 / 8 items (WindowsPlatform + discover/report/3 accessors, 3 seam builders) + module docs |
| File verdict | UNREACHABLE (real, cross-compile-verified, seam-tested — but no shipped entry point links the crate) |
| Reachable from shipped entry point? | NO. rg over every Cargo.toml: `platform-windows` appears only in its own manifest (and a comment in platform-linux's) — no crate or app depends on it; rg `platform_windows\|WindowsPlatform` over apps/: zero hits (cli, desktop, desktop-tauri, web all clean). It compiles as a bare workspace member (baseline #1 exit 0) and is tested (orchrun log 3712+: `platform_windows` seam_behaviour `test result: ok. 16 passed; 0 failed`), but neither `lumen` nor `lumen-tauri` can reach it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg `platform-windows` in **/Cargo.toml (dependents: none) · rg `platform_windows\|WindowsPlatform` in apps/ (none) · Read audit/runs/baseline/orchrun-check-platform-windows.log.txt (`cargo check -p platform-windows --target x86_64-pc-windows-msvc` → EXIT=0, W01-A03-requested) · orchrun test log 3712-3740 (16 seam tests ok, lib unittests 0) · sed -n quotes verified · read tests/seam_behaviour.rs:400+ (foreign-host discover test) |

## 1. One-paragraph verdict
The module does what an honest platform adapter should: one `discover()`
entry point, three cfg-gated seams with typed fallbacks, a capability report
that says `WrongOs` verbatim on foreign hosts (pinned by a passing test), and
a header whose evidence-status paragraph is exactly true — the cross-target
check exists in the baseline (EXIT=0) and runtime Windows behavior is marked
`[VERIFY]`, not claimed. The problem is everything above this file: nothing
in the repository depends on `platform-windows`, so the whole crate —
including the hand-declared COM vtables for WASAPI/DXGI/dialogs — is dead
weight from every shipped binary's perspective, and its "real" integration
paths have zero executed coverage anywhere (compile-only). The three seam
builders are also a verbatim triplication.

## 2. What actually works (verified)
— Cross-target compile claim: header says "compile-verified via `cargo check --target x86_64-pc-windows-msvc`" — REAL: audit/runs/baseline/orchrun-check-platform-windows.log.txt, `Checking platform-windows v0.1.0 … Finished dev profile in 0.61s, EXIT=0`.
— Foreign-host honesty: `discover_on_a_foreign_host_reports_wrong_os_everywhere` (tests/seam_behaviour.rs:400) asserts every feature reports `Unavailable(WrongOs { crate_os: "windows" })` and the seams refuse with the same typed reason — passed in baseline (orchrun log, 16/16 ok).
— Capability plumbing: `discover()` (98-116) sets four feature rows from the three probes; AudioNotifications shares the WASAPI verdict with a documented rationale ("the notification client registers against the same enumerator") — accurate to the described design.
— Accessor design: `audio()/gpu()/dialogs()` return `&mut dyn Seam` so callers cannot branch on which implementation they got — matches the "typed refusal, never faked success" doctrine; `unavailable_*_returns_the_typed_reason` tests passed.

## 3. Findings

### F-001 [S3] — the crate is unreachable from every shipped entry point; ~2,300 lines of adapter (incl. all FFI) ship as dead weight
Where: crates/platform-windows/src/lib.rs:74-116 (the public surface nobody calls)
Code:
```rust
pub struct WindowsPlatform {
    audio: Box<dyn AudioDevices>,
    gpu: Box<dyn GpuAdapters>,
    dialogs: Box<dyn FileDialogs>,
    report: PlatformReport,
}
```
What is wrong: No manifest in the repository declares a dependency on
`platform-windows` (rg over **/Cargo.toml: only its own manifest and a
comment in platform-linux/Cargo.toml:5), and no app source references
`platform_windows` or `WindowsPlatform` (rg over apps/: zero). The CLI builds
its audio lane through audio-output's own device layer; the desktop shell
doesn't reference it either. The crate is a workspace *member* (crates/* glob)
so it compiles and tests in every gate run — which is exactly what makes the
deadness invisible: green CI on code no binary links.
Why it matters: every maintenance cost (the mirrored seam vocabulary the
header itself flags, the unsafe FFI surface, the clippy/lint burden) is paid
for zero shipped capability; and the parity-report's Windows column
(xtask-parity-report: windows=66/252) can be silently inflated by
platform-adapter code that no Windows user can actually reach through a
shipped binary.
What correct looks like: either a host wires `WindowsPlatform::discover()`
behind a cfg(windows) block in the CLI/desktop app, or the crate is marked
explicitly as a not-yet-wired library (KNOWN-LIMITS.md entry) so the parity
accounting cannot count it as delivered platform capability.
Confidence: HIGH — Verification: read + rg (dependent enumeration is
exhaustive over the repo); baseline build/test cite above.

### F-002 [S4] — the three seam builders' foreign-OS arms are a verbatim triplication
Where: crates/platform-windows/src/lib.rs:155-166, 182-193, 208-216
Code:
```rust
    #[cfg(not(target_os = "windows"))]
    {
        let reason = PlatformError::NotCompiledForThisOs {
            crate_os: "windows",
        };
        let capability = Capability::from_probe_error(&reason);
        (
            Box::new(UnavailableAudioDevices::because(reason)),
            capability,
        )
    }
}
```
What is wrong: Three byte-identical 12-line fallback blocks differing only in
the fallback type. The header acknowledges the mirror-by-design doctrine
("deliberately mirrored across the three platform-* crates … `[PROPOSAL]`"),
but this triplication is *within* one crate — a single generic
`fn foreign_os_seam<T: FnOnce(PlatformError) -> Box<dyn Seam>>` or a macro
would collapse it without any cross-crate unification.
Why it matters: three copies of a probe-fallback pattern drift independently;
a fix to one (e.g. a capability-reason change) can miss the others.
What correct looks like: one parameterized builder for the non-Windows arm.
Confidence: HIGH — Verification: read (all three blocks quoted/compared).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-50 | REAL | — | evidence-status paragraph verified true (cross-check log exists); runtime [VERIFY] honestly marked |
| mod declarations | 52-62 | REAL | — | windows module cfg-gated out on Linux (its FFI never compiled in baseline #1 — only in the cross-check) |
| struct WindowsPlatform | 82-87 | UNREACHABLE | S3 | F-001: no dependents |
| WindowsPlatform::discover | 98-116 | REAL (UNREACHABLE) | — | 4 feature rows, WASAPI verdict reuse documented; foreign-host behavior test-pinned |
| WindowsPlatform::report | 120-122 | REAL | — | pure accessor, #[must_use] |
| WindowsPlatform::audio/gpu/dialogs | 125-137 | REAL | — | &mut dyn accessors; no branch-on-impl possible |
| audio_seam | 143-166 | REAL | S4 | F-002 triplication; Windows arm delegates to wasapi (compile-verified only) |
| gpu_seam | 170-193 | REAL | S4 | F-002; DXGI arm compile-verified only |
| dialog_seam | 197-216 | REAL | S4 | F-002; Common-Item-Dialog arm compile-verified only |

## 5. Test honesty
In-file tests: none (lib unittests = 0, orchrun log 3712-3713). Integration
suite tests/seam_behaviour.rs: 16 passed in baseline. Key ones:
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| discover_on_a_foreign_host_reports_wrong_os_everywhere (400) | YES | all four features + all three seams asserted to carry the exact typed WrongOs reason |
| unavailable_{audio,gpu,dialogs}_returns_the_typed_reason | YES | fallback refusal payloads pinned |
| typed_fallbacks_satisfy_the_seam_traits | YES | fallbacks implement the real seam traits |
| capability_classifies_and_converts_probe_errors / platform_report_is_a_table_with_honest_defaults | YES | report vocabulary pinned |
| event_queue_* (5 tests) | YES | bounded FIFO ring, capacity clamp, thread safety |
CRITICAL CAVEAT: every one of these tests exercises the **fallback** arms —
on this sandbox (and in the baseline) the real WASAPI/DXGI/dialogs code has
zero executed coverage; its only verification is the cross-target `cargo
check` (typecheck, no link, no run). The suite is honest about this (the
header says so), but "16 passed" must not be read as integration evidence.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "compile-verified via `cargo check --target x86_64-pc-windows-msvc`" | lib.rs:40-42 | REAL | orchrun-check-platform-windows.log.txt EXIT=0 |
| "runtime behavior on real Windows hardware is `[VERIFY]`" | lib.rs:43 | REAL (honest UNVERIFIED marker) | no Windows execution anywhere in baseline |
| "The host-runnable tests cover the typed seams, capability negotiation, fallbacks and event plumbing with simulated inputs" | lib.rs:43-45 | REAL | 16/16 ok (seam_behaviour.rs) — "simulated inputs" is accurate |
| "it never leaks OS types upward (ARCH-6)" | lib.rs:11 | REAL-UNPROVEN | seam signatures use audio-output vocabulary (audio.rs/gpu.rs/dialog.rs — separate files, other auditors); no OS type in this file's public surface |
| "capability negotiation is a first-class outcome … never a guessed success" | lib.rs:20-22 | REAL | WrongOs test + typed fallbacks |

## 7. Dead / duplicated / unreachable code
- The entire crate is unreachable from shipped binaries (F-001) — the file
  itself is the assembly point of that dead surface.
- F-002's triplicated fallback arms.
- (Cross-file note for other auditors: src/windows/{wasapi,dxgi,dialogs,com}.rs
  — the hand-declared COM vtables — are compile-verified only.)

## 8. Dependencies & flags
Cargo.toml (audited W01-A03): deps on media-types/codec-api/audio-output
(seam vocabulary). `#[cfg(target_os = "windows")]` gates all FFI; on Linux
the crate builds fallback-only. No features. Cross-check used the msvc
target's std — no windows-s crate (per header; consistent with Cargo.toml
having no external deps beyond the seam crates).

## 9. Open questions for the next agent
- platform-linux/platform-macos auditors: same F-001 question — are those
  crates wired into any app? If all three platform adapters are orphaned,
  that is a wave-level S3 pattern worth one consolidated finding.
- Whoever owns parity accounting: do platform-* crates contribute to the
  windows/macos parity columns (xtask-parity-report) without being linked by
  a shipped binary? If yes, those columns overstate delivered capability.
- desktop-tauri owner: is platform-windows intended to be wired there (its
  Cargo.toml doesn't reference it today)?
