# AUDIT: crates/platform-macos/src/macos/mod.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-21 |
| Lines / functions | 22 / 4 cfg-gated module declarations |
| File verdict | UNREACHABLE (and its evidence claim is unbacked) |
| Reachable from shipped entry point? | NO — doubly: (a) all four modules are `#[cfg(target_os = "macos")]` so Linux builds (every baseline run) do not compile them; (b) no workspace member depends on platform-macos at all (my Cargo.toml sweep in the dialogs.rs/audio.rs audits; independently established by W01-A09's platform-macos Cargo.toml audit: "the crate is an orphan"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | `ls crates/platform-macos/src/macos/` → coreaudio.rs, dialogs.rs, metal.rs, objc.rs all exist (1,887 LOC total, mod.rs included); `ls audit/runs/baseline \| rg -i "macos\|darwin\|apple"` → EMPTY; `rg -l "aarch64-apple-darwin" audit/` → only prior audit files, no run log; W01-A09 log lines 72/266 read for corroboration |

## 1. One-paragraph verdict
Twenty-two lines: an honesty header and four cfg-gated module declarations that all point at real files. The structure is correct — the cfg gates are present on every module, the declared names match the files, and the "typed fallbacks on other OSes" story is the same verified pattern as platform-windows. The two defects are inherited but real: the header's compile-verification claim ("cargo check --target aarch64-apple-darwin and x86_64-apple-darwin") has NO committed evidence anywhere in the baseline runs — unlike its Windows sibling, whose check was actually executed on request — and the entire crate is an orphan no shipped binary can reach, so 1,887 LOC of macOS FFI rests on an unverifiable sentence with no product consumer.

## 2. What actually works (verified)
- The cfg gates are real and complete: all four declarations (15-22) carry `#[cfg(target_os = "macos")]` — on Linux the module body is empty, matching the "crate still compiles" claim (platform-macos tests ran green in `orchrun-test-workspace-nofailfast.log.txt`, compiling only the seam files).
- All four declared modules exist as files: coreaudio.rs, dialogs.rs, metal.rs, objc.rs (`ls` verified; 1,887 LOC with mod.rs).
- The header's honesty pattern matches the verified platform-windows shape (windows/mod.rs:1-18, my dialogs.rs audit): wrong-OS → typed fallbacks, capability report says `WrongOs`.
- The contrast case proves the pattern can be backed: platform-windows's identical claim IS backed by `orchrun-check-platform-windows.log.txt` (EXIT=0) — the absence for macos is a missing run, not an impossible one.

## 3. Findings

### F-001 [S3]
Where: crates/platform-macos/src/macos/mod.rs:7-13
Code:
```rust
//! **Evidence status (sandbox honesty, card 7.A3):** this Linux
//! sandbox has no macOS SDK — everything under this module is
//! compile-verified via `cargo check --target aarch64-apple-darwin`
//! and `x86_64-apple-darwin` (declarations typechecked, no linking,
//! no execution).
```
What is wrong: the claimed cross-checks have no committed evidence. The baseline directory contains a run log for the Windows equivalent (`orchrun-check-platform-windows.log.txt`) but nothing matching "macos|darwin|apple" (ls + rg). W01-A09 independently reached the same conclusion for the same sentence repeated in the crate manifest (W01-A09.log.md:72: `'"compile-verified via cargo check --target aarch64-apple-darwin": NO'`) and for the h264 hw matrix row.
Why it matters: this header is the ONLY assurance that 1,887 LOC of hand-rolled macOS FFI (CoreAudio property listeners, Metal glue, objc_msgSend panels — per W01-A09's audit) even typechecks on its target. An assurance that cites runs nobody can read is exactly the evidence-inflation this audit exists to catch; the Windows sibling shows the project knows how to commit such evidence.
What correct looks like: run the two checks, commit the logs beside `orchrun-check-platform-windows.log.txt`, and reference them by name — or downgrade the sentence to "typecheck not yet evidenced".
Confidence: HIGH — Verification: exhaustive filename scan of audit/runs/baseline/ + content rg for both target triples; prior-wave corroboration read.
REQUEST-ORCH-RUN: `cargo check -p platform-macos --target aarch64-apple-darwin` and `cargo check -p platform-macos --target x86_64-apple-darwin` (requires the darwin std components; would resolve F-001 for the whole crate).

### F-002 [S3]
Where: crates/platform-macos/src/macos/mod.rs:15-22 (declarations) — crate-wide fact
Code:
```rust
#[cfg(target_os = "macos")]
pub mod coreaudio;
#[cfg(target_os = "macos")]
pub mod dialogs;
#[cfg(target_os = "macos")]
pub mod metal;
#[cfg(target_os = "macos")]
pub mod objc;
```
What is wrong: nothing in the declarations themselves — but the crate they organize has zero dependents. No app or crate depends on platform-macos (my Cargo.toml sweep across the workspace; W01-A09's audit of the same crate states "nothing in the repository depends on it"). W01-A09 additionally documented that the promised platform-macos edges were never taken (VideoToolbox FFI hand-rolled a second time inside `codecs`).
Why it matters: identical orphaned-scaffolding class as platform-windows (my dialogs.rs F-001, audio.rs F-001) — real code, real FFI surface, tested only at its seams, embedded in no product. The "platform layer" of the product contains a macOS library nobody links.
What correct looks like: a desktop-shell dependency (cfg-gated) or an explicit not-yet-integrated marker in the capability report.
Confidence: HIGH — Verification: Cargo.toml sweep (dialogs.rs audit, Commands run) + W01-A09 corroboration.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc (evidence-status header) | 1-13 | REAL-UNPROVEN | S3 | F-001: the compile-verified claim is unbacked by any committed run |
| `pub mod coreaudio` (cfg macos) | 15-16 | REAL (file exists, 1,887-LOC crate) | S3 | Orphaned (F-002); audited in depth by W01-A09's crate-level audit |
| `pub mod dialogs` (cfg macos) | 17-18 | REAL (file exists, 371 LOC) | S3 | Same |
| `pub mod metal` (cfg macos) | 19-20 | REAL (file exists, 384 LOC) | S3 | Same |
| `pub mod objc` (cfg macos) | 21-22 | REAL (file exists, 344 LOC) | S3 | Same |

(The per-module contents — coreaudio.rs, dialogs.rs, metal.rs, objc.rs — are separate assignment files for other agents; this audit covers only the mod.rs organizer.)

## 5. Test honesty
No tests in this file. The crate's host-runnable tests (seam_behaviour.rs) run on Linux against the typed fallbacks — they cannot execute any of the four gated modules. Nothing anywhere executes or even compiles this code in CI-observable form (the cross-check of F-001 being uncommitted). Gap honestly self-declared by the header — but honesty about missing evidence does not create the evidence.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cfg-gated to `target_os = "macos"`" | doc 1-2 | REAL | cfg attributes at 15-22 verified |
| "On any other OS every one of these is replaced by the typed fallbacks in the seam modules" | doc 3-5 | REAL | The seam/fallback pattern verified in platform-windows's identical shape (my dialogs.rs/audio.rs audits); platform-macos tests compile+pass on Linux |
| "compile-verified via `cargo check --target aarch64-apple-darwin` and `x86_64-apple-darwin`" | doc 9-10 | REAL-UNPROVEN | F-001: no committed run exists; W01-A09 found the same for the manifest and matrix copies of the claim |
| "the host-runnable tests cover the typed seams and negotiation instead" | doc 12-13 | REAL | seam_behaviour.rs exists and passed in baseline |

## 7. Dead / duplicated / unreachable code
- Unreachable: the four modules (by cfg on Linux, by zero dependents on macOS).
- No duplication in this file.

## 8. Dependencies & flags
- Pure module organizer; no imports, no unsafe, no deps of its own.
- The child modules' FFI (unsafe) is other agents' scope; W01-A09's crate audit already flagged the VideoToolbox duplication with codecs.

## 9. Open questions for the next agent
- Q-1 (orchestrator): the REQUEST-ORCH-RUN above would settle whether 1,887 LOC of macOS FFI typechecks — cheap, decisive, and it converts three unbacked claims (this header, the manifest, the h264 hw matrix row) into evidence.
- Q-2 (coreaudio.rs/dialogs.rs/metal.rs/objc.rs auditors): the per-file deep audits should cross-link F-001/F-002 here rather than re-proving the orphan/evidence facts.
- Q-3 (desktop owner): is macOS a shipped target at all (tauri build config)? If not, the whole crate is ahead-of-need scaffolding and the capability report should say so.
