# AUDIT: crates/platform-linux/src/linux/mod.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-21 |
| Lines / functions | 16 / 6 units (module docs + 5 cfg-gated `pub mod` declarations at lines 7-8, 9-10, 11-12, 13-14, 15-16) |
| File verdict | UNREACHABLE (the crate has no dependents — see the pulse.rs audit's manifest scan; the module itself IS compiled and its integrations executed by the crate's tests on the Linux baseline) |
| Reachable from shipped entry point? | NO from shipped binaries (no `Cargo.toml` outside the crate lists `platform-linux`; `LinuxPlatform::discover` caller-free). Compiled + tested on the Linux host: baseline `build-release-workspace.log.txt:244` (`Compiling platform-linux v0.1.0`) and `orchrun-test-workspace-nofailfast.log.txt:3644-3656` (13 `linux_integrations` tests ok — pulse, drm, xdg all executed on this host). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read of full file via Read tool (16 lines, line numbers verified: docs 1-5, declarations 7-16); `ls crates/platform-linux/src/linux/` (all five declared modules exist: dl.rs 170, drm.rs 135, pulse.rs 759, pulse_ffi.rs 402, xdg.rs 350 LOC); `rg` seam fallback arms in `src/lib.rs` (the `#[cfg(not(target_os = "linux"))]` blocks returning `NotCompiledForThisOs`); baseline cites above. |

## 1. One-paragraph verdict
A 16-line module hub: one doc paragraph and five `#[cfg(target_os = "linux")] pub mod` declarations, each pointing at a real file (verified by ls), each gate matching the identical pattern in lib.rs's seam builders. There is nothing to hide in a file this size, and nothing hidden: the doc's claim that non-Linux hosts get typed fallbacks is implemented verbatim in `lib.rs`'s three `#[cfg(not(target_os = "linux"))]` seam arms, and the claim that the integrations work is backed by the 13-test `linux_integrations` suite that ran green in the no-fail-fast baseline on this very host (pulse probe/subscribe against real libpulse, DRM sysfs scans, xdg chains). The only unverifiable claim is the doc's parenthetical that "the crate still compiles" on other OSes — no non-Linux build of platform-linux exists in recorded evidence, though the mirrored windows/macos crates compiling on this Linux host is indirect evidence the pattern holds. Zero findings; what was checked is listed in §3.

## 2. What actually works (verified)
— All five declared modules exist and are non-empty (ls + wc: dl 170, drm 135, pulse 759, pulse_ffi 402, xdg 350 lines).
— The cfg pattern is consistent: every declaration gated `target_os = "linux"`; lib.rs gates the whole `pub mod linux` the same way (lib.rs:61-ish `#[cfg(target_os = "linux")] pub mod linux;`), and each seam builder has the matching `#[cfg(not(...))]` fallback arm returning `PlatformError::NotCompiledForThisOs { crate_os: "linux" }` (lib.rs audio_seam/gpu_seam/dialog_seam — read during the pulse.rs audit).
— The integrations executed: baseline `orchrun-test-workspace-nofailfast.log.txt:3644-3656` — `tests/linux_integrations.rs` 13 tests ok, covering pulse (probe + absent-server subscribe), drm (real sysfs + fixture trees), xdg (zenity/kdialog/portal probes) on this host.

## 3. Findings
None. Zero-finding justification (what was checked, how):
1. **Fakeness scan:** no code beyond the declarations — nothing to stub, no dead bodies possible; every `pub mod` target exists (ls).
2. **Doc-claim audit:** "cfg-gated to target_os = linux" — verified (5/5 declarations + the parent gate in lib.rs); "replaced by the typed fallbacks in the seam modules" — verified (lib.rs's three not-linux arms); "the PulseAudio client, the DRM sysfs scanner, the xdg dialog chain" — all three integration families present and test-executed (baseline :3644-3656); the two support modules (dl, pulse_ffi) are unnamed in the doc but are implementation details of the named pulse integration — not a defect.
3. **Gate consistency:** no declaration is ungated (an ungated `pub mod` here would break non-Linux builds by pulling dlopen symbols); no module is gated with a different predicate.
4. **The one open claim** ("the crate still compiles [on any other OS]") is UNVERIFIED — no non-Linux platform-linux build exists in `audit/runs/baseline/` (INDEX.md reviewed); recorded in §6 rather than filed, since the sibling crates' Linux compilation is the design's own proof-by-mirror.

## 4. Per-function table
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-5 | REAL | — | Claims verified per §3.3. |
| `pub mod dl` | 7-8 | REAL | — | dlopen helper (170 LOC, unaudited by me); consumed by pulse.rs's `DynamicLibrary::open`. |
| `pub mod drm` | 9-10 | REAL | — | DRM sysfs scanner (135 LOC, unaudited); executed by baseline drm tests. |
| `pub mod pulse` | 11-12 | REAL | — | Audited this wave (see `pulse.rs.audit.md` — S2×3 findings live there). |
| `pub mod pulse_ffi` | 13-14 | REAL | — | FFI table (402 LOC, unaudited); the constant/ABI source pulse.rs consumes. |
| `pub mod xdg` | 15-16 | REAL | — | Dialog chain (350 LOC, unaudited); executed by baseline xdg tests. |

## 5. Test honesty
No in-file tests. The module's integrations are covered by `tests/linux_integrations.rs` (13 tests, all ok in baseline :3644-3656) — those tests' honesty is the individual modules' auditors' concern; pulse's two tests were assessed in `pulse.rs.audit.md` §5.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cfg-gated to `target_os = "linux"`" | mod.rs docs 1-2 | REAL | 5/5 declarations gated; parent gate in lib.rs. |
| "On any other OS every one of these is replaced by the typed fallbacks in the seam modules" | mod.rs docs 2-4 | REAL | lib.rs `#[cfg(not(target_os = "linux"))]` arms returning `NotCompiledForThisOs` (read during pulse.rs audit). |
| "the crate still compiles [on any other OS] and its capability report says `WrongOs` honestly" | mod.rs docs 4-5 | UNVERIFIED | No non-Linux build of platform-linux in recorded evidence; indirect evidence: platform-windows/macos compile on this Linux host with the mirrored pattern. |

## 7. Dead / duplicated / unreachable code
The crate is unreachable from shipped entry points (no dependents — the finding is filed against the crate in `pulse.rs.audit.md` F-005; this hub file adds nothing beyond it). No duplication.

## 8. Dependencies & flags
- Single predicate (`target_os = "linux"`) applied uniformly; no features; the module-level `#![allow(unsafe_code)]` exceptions live in the FFI submodules (dl/pulse/pulse_ffi), not here.
- Std-only crate (C5) — the hub links nothing.

## 9. Open questions for the next agent
- `dl.rs` (170 LOC — the dlopen wrapper whose `DynamicLibrary::open` underpins pulse) and `pulse_ffi.rs` (402 LOC — the symbol table/struct layouts `on_sink_info` reads through) are the two unaudited modules whose correctness pulse.rs's safety comments assume; both are small enough for one audit pass.
- `drm.rs` and `xdg.rs` (485 LOC combined) are executed-but-unaudited; the xdg chain shells out to zenity/kdialog (SpawnFailed surface) — worth a look for argument-injection hygiene.
