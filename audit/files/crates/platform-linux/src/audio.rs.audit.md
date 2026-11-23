# AUDIT: crates/platform-linux/src/audio.rs
| Field | Value |
|---|---|
| Auditor | W08-A06 |
| Date | 2026-11-23 |
| Lines / functions | 114 / 5 (trait AudioDevices:3 methods; UnavailableAudioDevices:2; fallback impl:3; stable_device_id) |
| File verdict | REAL (code) / UNREACHABLE (from shipped entry point) |
| Reachable from shipped entry point? | NO — orphan crate (see xdg.rs audit §7). Within the crate: `AudioDevices` is implemented by `linux::pulse::PulseAudioDevices` (pulse.rs:588) and wired by `lib.rs::audio_seam` (136-170); both reachable only from tests. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg "alsa|ALSA" crates/platform-linux/` (zero backend hits — only a PulseAudio sink-name test fixture); `rg "fn enumerate" crates/audio-output/src` (contract cross-check); prebuilt test runs (13/13 + 15/15) |

## 1. One-paragraph verdict
Hostile-focus answer — what does Linux actually get for audio? **(1)** This seam file: a 3-method trait (enumerate / subscribe / poll) + a typed refusal fallback that returns the exact reason on every call. **(2)** A real PulseAudio client (linux/pulse.rs 759 LOC + pulse_ffi.rs 402 + dl.rs 170 = 1331 LOC, runtime-dlopen'd, no link-time libpulse). **(3)** NO ALSA and NO native PipeWire client anywhere in the crate (rg: zero hits; the only "alsa" string is a PulseAudio sink-name fixture). **(4)** NO audio output/streaming: the doc (lines 6-12) states v0 is the enumeration/notification half only; `AudioBackend::open` stays behind the ADR-005 gate — so this crate cannot play audio even on a machine with a live PulseAudio server. **(5)** On this sandbox the wired instance is `UnavailableAudioDevices` returning `ServiceUnavailable { service: "PulseAudio server" }`: libpulse.so.0 loads (present, 0.24.3) but no daemon exists — the honest typed refusal. The code that exists here is correct and honestly documented; the two defects are contract-level smells, not brokenness.

## 2. What actually works (verified)
- FNV-1a-64 minting verified against the known independent vector: `stable_device_id("a").handle() == 0xaf63_dc4c_8601_ec8c` (the published FNV-1a-64 of "a") — pinned by `stable_device_id_is_deterministic_and_pinned` (seam_behaviour.rs:351-365), executed in the 15/15 prebuilt run; the constants (0xcbf29ce484222325 / 0x100000001b3, lines 108-111) are the standard FNV-1a parameters.
- Typed fallback behavior executed: `unavailable_audio_devices_returns_the_typed_reason` (seam_behaviour, 15/15 run) drives all three methods and asserts the reason round-trips.
- Cross-crate contract claim verified: `AudioBackend::enumerate` at audio-output/src/device.rs:88-91 says "Empty is honest ('no devices'), never a placeholder entry" — matching this file's line 30-31 claim verbatim in meaning.

## 3. Findings
F-001 [S4]
Where: crates/platform-linux/src/audio.rs:48-56
Code:
```rust
    /// Pump the OS event loop once, non-blocking. Returns the number
    /// of loop iterations dispatched. Push-based integrations
    /// (Windows COM callbacks, CoreAudio listeners) return 0 —
    /// events already sit in the queue; poll-style integrations
    /// (PulseAudio mainloop) dispatch here.
```
What is wrong: The `poll()` return value cannot distinguish "push-based integration, polling is a no-op" (always 0) from "poll-style integration that dispatched zero iterations this round" (also 0). A caller that needs to know whether it must keep pumping cannot tell the two cases apart; the distinction lives only in per-OS doc text. Why it matters: contract ambiguity that the future unified L5 crate will inherit; no current caller branches on it (the crate has no callers). What correct looks like: a separate `is_poll_driven()`/`Polling::Push|Poll` classification on the trait, or documenting that consumers must not interpret 0. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/platform-linux/src/audio.rs:82-94
Code:
```rust
impl AudioDevices for UnavailableAudioDevices {
    fn enumerate(&mut self) -> Result<Vec<DeviceDesc>, PlatformError> {
        Err(self.reason.clone())
    }
```
What is wrong: Every call on the fallback clones the full `PlatformError` (string payloads included). On a host where audio is unavailable, a polling consumer calling `poll()` per frame pays a string clone per frame. No current caller does this (orphan crate), hence S4. What correct looks like: `Err(self.reason.clone())` is fine for one-shot calls; document the fallback as not for hot loops, or cache an Arc'd error. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AudioDevices::enumerate | 36 | REAL | — | Contract matches AudioBackend::enumerate (empty = honest) |
| AudioDevices::subscribe | 46 | REAL | — | "Re-subscribing replaces the previous queue" — implemented in pulse.rs (not this file) |
| AudioDevices::poll | 56 | REAL | S4 | Ambiguous 0 return (F-001) |
| UnavailableAudioDevices::because / reason | 68-80 | REAL | — | Typed refusal carrier |
| AudioDevices for UnavailableAudioDevices | 82-94 | REAL | S4 | Clone per call (F-002); executed by test |
| stable_device_id | 105-114 | REAL | — | FNV-1a-64, constants correct, pinned to independent vector; collision caveat documented (99-104) |

## 5. Test honesty
No tests in this file; the seam_behaviour suite covers it: `unavailable_audio_devices_returns_the_typed_reason` (all three methods refuse with the reason — would fail if the fallback faked success), `stable_device_id_is_deterministic_and_pinned` (independent-vector pin — would fail if the hash constants drifted), `typed_fallbacks_satisfy_the_seam_traits`. All executed (15/15). The live-PulseAudio success path (`PulseAudioDevices::enumerate` returning sinks) has NO test that observes a non-empty list anywhere — sandbox has no server (test file's own [VERIFY] marker, linux_integrations.rs:399).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v0 scope is the enumeration/notification half ... the streaming half stays behind the ADR-005 admission gate" | audio.rs:6-9 | REAL | No `open`/sink method exists on the trait (29-57); audio-output's AudioBackend gate documented at device.rs:88 |
| "An empty list is honest ('no devices'), matching AudioBackend::enumerate's contract" | audio.rs:30-31 | REAL | audio-output/src/device.rs:89-91 verbatim "Empty is honest" |
| "The OS strings are long and stable ... FNV-1a-64 ... documented as not collision-free" | audio.rs:99-104 | REAL | Constants verified + pinned vector (§2) |
| (context) Linux audio = PulseAudio only | Cargo.toml:16-18 | REAL | rg: no ALSA/PipeWire client in crate; matrix row `platform-aout-pipewire` = not_started |

## 7. Dead / duplicated / unreachable code
- Orphan crate: this file's production reachability is zero (see xdg.rs audit §7).
- Near-verbatim mirror exists in platform-windows/src/audio.rs and platform-macos/src/audio.rs (documented [PROPOSAL] duplication, cross-ref platform-windows lib.rs audit F-003).

## 8. Dependencies & flags
- deps: audio-output (DeviceDesc/DeviceId), crate::error, crate::events, std::sync::Arc. No unsafe in this file (the unsafe lives in linux/{dl,pulse_ffi,pulse}.rs, out of my file list). No feature gates.

## 9. Open questions for the next agent
- pulse.rs (759 LOC, another agent): does `subscribe` on a live server actually deliver events, and does `poll` dispatch the mainloop non-blockingly as this trait documents? UNVERIFIABLE here (no server).
