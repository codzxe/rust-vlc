# AUDIT: crates/platform-macos/src/audio.rs
| Field | Value |
|---|---|
| Auditor | W08-A08 |
| Date | 2026-11-23 |
| Lines / functions | 114 / 8 (trait with 3 methods, fallback ×2 + 3 impl methods, stable_device_id) |
| File verdict | REAL (an honest typed-refusal seam + shared id minting; the macOS real lane never compiles in this sandbox or the shipped lane) |
| Reachable from shipped entry point? | NO — platform-macos has no dependents (rg over crates/, apps/); within the crate, lib.rs audio_seam() wires this trait behind cfg(target_os) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | sed -n (quotes); diff vs platform-windows/src/audio.rs (2 lines differ) and platform-linux/src/audio.rs (2 lines differ); baseline citation cargo-test-workspace.txt:4014-4034 |

## 1. One-paragraph verdict
This file is the macOS audio seam's OS-neutral half: the AudioDevices trait
(enumerate / subscribe / poll), the UnavailableAudioDevices fallback whose
every method answers a typed error, and the FNV-1a-64 stable_device_id shared
across lanes. It is honest about its scope — v0 covers enumeration and
notifications ONLY; the streaming half (AudioBackend::open, the render
callback) is explicitly deferred behind the ADR-005 admission gate (header
lines 6-12), so no code here pretends audio output exists on macOS. The
fallback's typed refusals are the documented graceful-degradation contract,
not fake success, and the crate's seam suite proves each method refuses with
the exact reason (baseline green). The defects are structural: the file is a
3-way copy-paste (only two doc-comment lines differ from the windows and
linux siblings — verified by diff), and the real CoreAudio implementation
(src/macos/coreaudio.rs) has zero runtime coverage anywhere (compile-check
only, per the crate manifest's sandbox-honesty note). Nothing shipped can
reach any of it.

## 2. What actually works (verified)
- Baseline: audit/runs/baseline/cargo-test-workspace.txt:4014-4034 — the
  crate's seam suite (16/16 ok) exercises UnavailableAudioDevices via the
  trait object: unavailable_audio_devices_returns_the_typed_reason asserts
  enumerate/subscribe/poll each return the exact typed reason.
- stable_device_id is the same FNV-1a-64 body as platform-windows
  (audio.rs:106-114); the pinned vector constants were independently
  recomputed in my file-1 audit (python3 FNV-1a-64 matches the test's
  constants) — the macos seam test pins the same values.
- Cross-lane id stability claim verified: diff shows the function bodies are
  byte-identical across the three lanes.

## 3. Findings
F-001 [S3]
Where: crates/platform-macos/src/audio.rs (whole file)
Code:
```
//! The audio seam (card 7.A3): device enumeration + change
//! notifications against the `audio-output` vocabulary (doc 01 §2.7
```
What is wrong: 3-way copy-paste. diff against platform-windows/src/audio.rs
and platform-linux/src/audio.rs: each differs in exactly two doc-comment
lines (card id; the order of the OS-identifier examples at line 97). The
trait, the fallback, and the hash function are byte-identical ×3. The
CoreAudioDevices real implementation (src/macos/coreaudio.rs) is the
macOS-specific part and has zero runtime tests (compile-checked via
`cargo check --target aarch64-apple-darwin` per the manifest — a manual,
one-time act recorded in worklog.md; no workflow automates it). Same
coverage hole as platform-windows (my file-1 audit F-001).
Confidence: HIGH — Verification: ran diff.

F-002 [S4]
Where: crates/platform-macos/src/audio.rs:48-56
Code:
```
    /// Pump the OS event loop once, non-blocking. Returns the number
    /// of loop iterations dispatched. Push-based integrations
    /// (Windows COM callbacks, CoreAudio listeners) return 0 —
    /// events already sit in the queue; poll-style integrations
    /// (PulseAudio mainloop) dispatch here.
```
What is wrong: the contract permits an implementation whose poll() is a
permanent Ok(0) no-op (push-based integrations) — a method that can
legitimately do nothing on two of the three lanes. The fallback refuses; the
CoreAudio lane would return 0 forever. Not a defect in this file (the seam
documents it), but a caller cannot distinguish "no events" from "events are
pushed elsewhere" — recorded for whoever wires the session loop.
Confidence: HIGH — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AudioDevices::enumerate (trait) | 36 | REAL | — | contract: empty list = honest "no devices" |
| AudioDevices::subscribe (trait) | 46 | REAL | — | re-subscribe replaces queue; bounded by D-3 |
| AudioDevices::poll (trait) | 56 | REAL | S4 | permanent-Ok(0) shape permitted (F-002) |
| UnavailableAudioDevices::because | 71-73 | REAL | — | wraps reason |
| UnavailableAudioDevices::reason | 77-79 | REAL | — | accessor |
| impl enumerate/subscribe/poll | 82-94 | REAL | — | every method Err(reason.clone()) — honest refusal, proven by tests |
| stable_device_id | 106-114 | REAL | — | FNV-1a-64, non-collision-free caveat documented; identical ×3 lanes |

## 5. Test honesty
No tests in-file. The crate's tests/seam_behaviour.rs covers the fallback
(baseline 16/16): would fail if a fallback method returned Ok, or if the
hash constants drifted. Nothing tests CoreAudioDevices at runtime (cannot —
cfg'd out on Linux; macOS runner lanes unverified in this sandbox,
UNVERIFIED whether ci-full.yml's macos lane runs them).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v0 scope is the enumeration/notification half... streaming half stays behind the ADR-005 admission gate" | header:6-9 | REAL | no open/streaming code in crate; matrix row platform-aout-coreaudio = not_started |
| "every method answers a typed error naming the reason, never a fake success" | :26-28 | REAL | fallback impl 82-94 + seam tests |
| "deterministic, std-only, and documented as not collision-free" | :100-104 | REAL | FNV-1a verified; caveat in doc |
| "the real backend consumes this seam's DeviceDescs directly" (future) | :10-12 | PAPER-ONLY (future wiring) | no consumer exists today |

## 7. Dead / duplicated / unreachable code
Duplicated: entire file ×3 lanes (F-001). Unreachable: crate has no
dependents. Dead: none within the file.

## 8. Dependencies & flags
Deps: audio-output (DeviceDesc/DeviceId), crate::error, crate::events,
std::sync::Arc. No features. The CoreAudio lane is cfg(target_os="macos").

## 9. Open questions for the next agent
- The seam test for this crate (tests/seam_behaviour.rs) was audited by
  another agent (audit/files/crates/platform-macos/tests/seam_behaviour.rs.audit.md
  exists) — cross-read for the poll()/subscribe() contract coverage.
- If ADR-005 opens, does CoreAudioDevices::enumerate get a runtime test lane
  (macOS runner) before the seam is trusted? Nothing today guarantees it.
