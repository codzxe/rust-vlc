# AUDIT: crates/platform-macos/src/macos/coreaudio.rs
| Field | Value |
|---|---|
| Auditor | W08-A02 |
| Date | 2026-11-15 |
| Lines / functions | 766 / 16 |
| File verdict | UNREACHABLE (real hand-rolled FFI, zero shipped consumers, runtime never exercised) |
| Reachable from shipped entry point? | NO — no crate in the workspace depends on `platform-macos`; only its own `tests/seam_behaviour.rs` imports it. Release lane (`-p lumen-cli -p desktop`) never compiles this crate. |
| Findings | S0: 0 S1: 1 S2: 1 S3: 1 S4: 1 |
| Commands run | wc -l (766); sed -n ranges (quote verification, 5 ranges); rg for `platform-macos`/`platform_macos` across crates/apps/tools/tests (only self-references); python3 four-char-code arithmetic (8/8 constants verified) |

## 1. One-paragraph verdict
This is genuine, careful CoreAudio/CoreFoundation FFI — real `extern "C"` declarations (`AudioObjectGetPropertyDataSize/Data`, `Add/RemovePropertyListener`, `CFStringGetCString`, `CFRelease`), real bounded-size property reads, real listener registration with matched-removal teardown, and honest `[VERIFY]` labeling that runtime was never executed. It is NOT a refusal and NOT a stub. But it is dead code from the product's perspective: nothing depends on `platform-macos`, so no shipped binary contains it, and its one outright defect — `ELEMENT_MAIN = 1` where Apple's public `AudioObjectBase.h` defines `kAudioObjectPropertyElementMaster = 0` — sits in every property address the file builds, meaning the entire lane would likely fail its first HAL call on real hardware. The header's compile-only evidence claim cannot mask a constant that is checkable against public headers without any hardware.

## 2. What actually works (verified)
— All 8 four-char-code constants verified by arithmetic (`python3`: 'glob' 0x676c6f62, 'outp' 0x6f757470, 'dev#' 0x64657623, 'dOut' 0x644f7574, 'name' 0x6e616d65, 'uid ' 0x75696420, 'str#' 0x73747223, 'nsr ' 0x6e737220 — 8/8 MATCH the file) plus `kCFStringEncodingUTF8` 0x08000100 and `kAudioObjectSystemObject` 1 (public header values). FFI signatures match the documented ABI by inspection. Nothing was executed (no macOS; cargo cross-check not re-run — UNVERIFIED: no rust macOS target in sandbox).

## 3. Findings
F-001 [S1]
Where: crates/platform-macos/src/macos/coreaudio.rs:133-137
Code:
```
/// `kAudioObjectPropertyElementMaster` (renamed
/// `kAudioObjectPropertyElementMain`) — the whole-object element.
/// `[VERIFY]` the numeric value on real hardware (the declaration is
/// compile-only evidence in this sandbox).
const ELEMENT_MAIN: u32 = 1;
```
What is wrong: `kAudioObjectPropertyElementMaster`/`Main` is `0` in Apple's public `AudioObjectBase.h` (`enum { kAudioObjectPropertyElementMaster = 0 }`), not `1`. Every `AudioObjectPropertyAddress` in this file — devices list, default output, name, UID, stream configuration, nominal rate, both listener registrations and both removals — is built with `element: ELEMENT_MAIN`. Why it matters: if the HAL enforces the documented master-element contract for global-scope properties, every `AudioObjectGetPropertyDataSize` call returns an error and the whole macOS audio seam degrades to the typed fallback at first probe — the lane does nothing on the only OS it targets. `[VERIFY]`-labeling a constant does not repair it: the value is checkable against public headers from any OS. What correct looks like: `const ELEMENT_MAIN: u32 = 0;` (and an integration test against the real header when a macOS SDK exists). Confidence: HIGH (constant vs public header) / consequence on hardware UNVERIFIED (no macOS in sandbox).
Verification: read + public-header comparison; runtime UNVERIFIED.

F-002 [S2]
Where: crates/platform-macos/src/macos/coreaudio.rs:562-567
Code:
```
        if selector == SELECTOR_DEFAULT_OUTPUT {
            // Read the new default inline. Reading a *different*
            // property than the one being notified (devices-set vs
            // default-device) is the listener-reentrancy pattern
            // Apple's own samples use; `[VERIFY]` on hardware.
            let new_default = default_output_device_id().ok().flatten();
```
What is wrong: the safety comment is false. This branch fires when the notified selector IS `SELECTOR_DEFAULT_OUTPUT`, and `default_output_device_id()` reads exactly `kAudioHardwarePropertyDefaultOutputDevice` — the SAME property that triggered the listener, not "a *different* property". Re-entrant `AudioObjectGetPropertyData` on the property being notified from inside its own listener is the classic HAL deadlock hazard. The module header (lines 39-47) admits "the claim that reading `kAudioHardwarePropertyDefaultOutputDevice` inside its own listener is deadlock-free ... needs hardware confirmation" — so the inline comment asserts safety the header itself does not claim. Why it matters: a deadlock on a HAL thread hangs the process on real hardware, and the false comment would mislead any future reviewer. What correct looks like: defer the read off the listener thread (queue a task), or at minimum fix the comment to state it reads the same property and the risk. Confidence: MEDIUM (comment/code mismatch: HIGH; actual deadlock: hardware-dependent, UNVERIFIED).
Verification: read.

F-003 [S3]
Where: crates/platform-macos/src/macos/coreaudio.rs (whole file) + repo-wide dependency graph
Code:
```
#[cfg(target_os = "macos")]
pub mod coreaudio;
```
( crates/platform-macos/src/macos/mod.rs:15-16 — the only gate; `rg 'platform-macos|platform_macos'` over crates/*/Cargo.toml matches only the crate's own manifest; apps/ tools/ tests/ have zero hits. )
What is wrong: real code, zero consumers. Even on a macOS build, no lumen-cli/desktop path reaches `CoreAudioDevices` — the crate is an orphan in `members = ["crates/*", ...]`. Why it matters: 766 lines of unsafe FFI that no shipped entry point can execute; the capability it advertises (`Feature::AudioEnumeration`) is reported by a crate nobody asks. What correct looks like: wire it via a platform facade in the engine, or mark the crate as not-yet-wired in the registry. Confidence: HIGH.
Verification: rg over the whole repo.

F-004 [S4]
Where: crates/platform-macos/src/macos/coreaudio.rs:426-462 and 660-665
Code:
```
fn list_devices() -> Result<Vec<(AudioObjectId, DeviceDesc)>, PlatformError> {
    system_device_ids()?
        .into_iter()
        .filter_map(|device| describe_device(device).transpose())
        .collect()
}
```
What is wrong: `describe_device` can never return `Err` (every fallible read inside is `.ok()`-ed or matched to `Ok(None)`), so the `transpose()` + `collect()` error propagation is dead generality — the only real error source is `system_device_ids()?`. Misleading shape: suggests per-device failures propagate when they are silently swallowed into `Ok(None)` skips. Confidence: HIGH.
Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| get_property_bytes | 235-293 | REAL-UNPROVEN | — | bounded (64 KiB cap), size-recheck, zero-init; sound by inspection; never executed |
| get_property_string | 301-344 | REAL-UNPROVEN | — | ptr-width-portable, single CFRelease, 4096-byte cap, NUL scan |
| get_property_f64 | 350-366 | REAL-UNPROVEN | — | strict 8-byte size check |
| system_device_ids | 369-395 | REAL-UNPROVEN | — | u32-chunk parse; `bytes.len() % 4` guard |
| default_output_device_id | 398-421 | REAL-UNPROVEN | — | id 0 → None |
| describe_device | 426-462 | REAL-UNPROVEN | S4 | never errs; UID-fail → Ok(None) skip |
| preferred_format | 467-492 | REAL-UNPROVEN | — | rate filter + 48 kHz fallback; channels clamp |
| output_channels | 496-530 | REAL-UNPROVEN | — | per-buffer bounds checks, saturating add; correct stride math |
| on_property_change | 548-590 | REAL-UNPROVEN | S2 | F-002 same-property reentrancy; panic-free otherwise |
| CoreAudioDevices::new / Default | 604-614 | REAL-UNPROVEN | — | trivial |
| Watch::drop | 623-657 | REAL-UNPROVEN | — | removes both registrations without holding state mutex (invariant honored) |
| list_devices | 660-665 | REAL-UNPROVEN | S4 | F-004 dead error path |
| AudioDevices::enumerate | 668-682 | REAL-UNPROVEN | — | refreshes listener id-map without mutex-across-HAL-call |
| AudioDevices::subscribe | 684-759 | REAL-UNPROVEN | — | rollback of first listener on second failure; correct |
| AudioDevices::poll | 761-765 | REAL (contract) | — | constant `Ok(0)` is the documented push-based contract, not a fake |

## 5. Test honesty
No tests in this file. `crates/platform-macos/tests/seam_behaviour.rs` exercises only the typed fallback path on Linux (`platform.audio().enumerate()` refuses, line ~409) and capability-report machinery; the CoreAudio implementation has zero runtime coverage anywhere. Any future test claiming CoreAudio behavior from this crate on Linux tests nothing of this file.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "compile-verified via `cargo check --target aarch64-apple-darwin`" | coreaudio.rs:39-40, Cargo.toml, macos/mod.rs:8-9 | REAL-UNPROVEN | no log of that run was found in audit/runs/baseline/ or docs/ (rg `aarch64-apple-darwin` docs/ → only unrelated packaging/h264 hits); plausible (check does not link) but UNVERIFIED |
| Runtime NOT exercised, `[VERIFY]` on real hardware | coreaudio.rs:41-47 | TRUE (honest) | this sandbox cannot run it either |
| `platform-aout-coreaudio` matrix row: macos = "not_started", evidence = "" | docs/feature-matrix.toml:2379-2386 | TRUE (honest — this file is enumeration only, no AudioUnit playback) | file has no AudioUnit/AudioDeviceIOProc/OutputUnit symbols (read) |
| afeat-audio-device-switch: all platforms "not_started" | docs/feature-matrix.toml:1785-1794 | TRUE | nothing wires device switching into the player (no consumers of this crate) |

## 7. Dead / duplicated / unreachable code
The entire crate is unreachable from shipped entry points (F-003). Within the file: no dead functions; the `Result` shape of `describe_device` is dead generality (F-004).

## 8. Dependencies & flags
`#[link(name = "CoreAudio"/"CoreFoundation", kind = "framework")]` — std-only crate, no coreaudio-sys/core-foundation deps (matches Cargo.toml allowlist). `#![allow(unsafe_code)]` module inner attribute (line 52) — needed because the workspace presumably denies unsafe outside audited modules. `cfg(target_os = "macos")` gate on the parent module (mod.rs:15).

## 9. Open questions for the next agent
- Verify `ELEMENT_MAIN` (F-001) against a real SDK header when macOS hardware exists; if 0 is confirmed, the entire seam needs re-probing.
- Confirm whether the HAL really allows element≠master for global-scope reads (some HAL versions are lenient) — determines whether F-001 is total-lane failure or latent.
- Whoever audits `capability.rs`/`lib.rs` of this crate: check whether `Feature::AudioEnumeration` capability reporting implies to any consumer that enumeration is wired into the player (it is not).
