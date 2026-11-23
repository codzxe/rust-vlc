# AUDIT: crates/platform-windows/src/audio.rs
| Field | Value |
|---|---|
| Auditor | W08-A05 |
| Date | 2026-11-23 |
| Lines / functions | 114 / 8 (trait AudioDevices: enumerate/subscribe/poll; UnavailableAudioDevices: because/reason + 3 impl methods; stable_device_id) |
| File verdict | REAL (seam + fallback + hashing compiled on every OS, fallback and hashing host-tested with verified goldens; the OS implementation behind the seam is REAL-UNPROVEN and unreachable) |
| Reachable from shipped entry point? | NO — platform-windows is an island (zero dependents). In-crate wiring: lib.rs `audio_seam()` → `WasapiAudioDevices` (cfg windows) or `UnavailableAudioDevices`; tests exercise the fallback + hashing. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg AudioBackend/DeviceId in audio-output (exit 0); sed wasapi.rs impl block (exit 0); diff vs platform-linux audio.rs (exit 0, 2 lines differ); python3 FNV-1a recomputation of both goldens (exit 0, exact match) |

## 1. One-paragraph verdict
The layering question ("audio.rs vs wasapi — who wraps whom") has a clean answer: audio.rs is the OS-neutral seam — it defines `trait AudioDevices` (enumerate/subscribe/poll), the typed `UnavailableAudioDevices` fallback, and the FNV-1a-64 `stable_device_id` mint; it imports nothing from WASAPI (imports are audio_output, crate::error, crate::events only). wasapi.rs is the wrapper: it implements `AudioDevices for WasapiAudioDevices` (verified: enumerate spawns a dedicated STA thread running `list_endpoints`; subscribe spawns a watch thread with ready/shutdown channels and a deadline; poll returns `Ok(0)` push-based exactly per the seam contract, wasapi.rs:600-671). lib.rs::audio_seam() wires one or the other. No circular wrapping, no OS types leaking (ARCH-6 holds). The hashing is provably correct: I recomputed both pinned goldens with an independent FNV-1a implementation and they match exactly. The defects are the crate's: nothing shipped can obtain an `AudioDevices` instance, and the file is a byte-identical triplicate.

## 2. What actually works (verified)
- Golden-hash verification (executed): `python3` FNV-1a-64 recomputation → `fnv("a") = 0xaf63dc4c8601ec8c` and `fnv("{0.0.0.00000000}.{e32f9c65-1234-4a55-bc86-8ed0d9d1c021}") = 0x5b40ed9bb4ce1081` — both match the pinned assertions in tests/seam_behaviour.rs:301-306. The test is a true golden, not self-generated.
- Baseline execution: `stable_device_id_is_deterministic_and_pinned`, `unavailable_audio_devices_returns_the_typed_reason`, `typed_fallbacks_satisfy_the_seam_traits` pass (cargo-test-workspace.txt:4054-4073).
- Seam-contract conformance of the Windows impl (read): poll()→Ok(0) (wasapi.rs:667-671), subscribe replaces previous watch (`self.watch = None;` wasapi.rs:624), enumerate on dedicated STA thread (wasapi.rs:600-622).
- Vocabulary: `audio_output::DeviceId::new` is a real const constructor (audio-output/src/device.rs:30-36); `AudioBackend::enumerate` exists (device.rs:88-91), so the doc cross-reference is true.
- PulseAudio counterpart honors the poll half of the contract (platform-linux/src/linux/pulse.rs:734-739 `pump_once`) — the trait's push/poll split is real, not decorative.

## 3. Findings
F-001 [S3]
Where: whole file (seam) + lib.rs:143-166 (wiring)
Code:
```
pub trait AudioDevices: Send {
```
What is wrong: the seam exists to be implemented and consumed, but the only consumer of `AudioDevices` values is `WindowsPlatform::audio()` — and `WindowsPlatform` itself has zero shipped callers (no crate depends on platform-windows). The one real implementation (`WasapiAudioDevices`) is compiled only on Windows and never executed anywhere; the fallback is the only implementation any test touches.
Why it matters: enumeration/notification capability this seam models (matrix rows `afeat-audio-device-switch`, Feature::AudioEnumeration/AudioNotifications) has no production path: `afeat-audio-device-switch` is `not_started` with empty evidence (docs/feature-matrix.toml:1785-1792) — consistent, but it means this file is scaffolding for a feature that ships nothing today.
Confidence: HIGH — Verification: ran rg (no dependents).
F-002 [S4]
Where: whole file vs platform-linux/src/audio.rs and platform-macos/src/audio.rs
Code (the only differences vs platform-linux, diff output):
```
1c1
< //! The audio seam (card 7.A1): device enumeration + change
---
> //! The audio seam (card 7.A2): device enumeration + change
97c97
< /// (WASAPI endpoint id, PulseAudio sink name, CoreAudio UID).
---
> /// (PulseAudio sink name, WASAPI endpoint id, CoreAudio UID).
```
What is wrong: 114-line file byte-identical across three crates except the card id and one reordered doc line — third instance (with events.rs, lib.rs) of the declared-but-unlanded triplication. The trait will need one definition when the Lead unifies ([PROPOSAL], lib.rs:47-50).
Confidence: HIGH — Verification: ran diff.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AudioDevices::enumerate (trait method) | 36 | REAL (contract) | — | implemented by Wasapi (cfg win, unproven) + fallback (tested) |
| AudioDevices::subscribe | 46 | REAL (contract) | — | re-subscribe-replaces semantics honored by wasapi.rs:624 |
| AudioDevices::poll | 56 | REAL (contract) | — | push (Windows/CoreAudio→0) vs poll (Pulse→pump_once) split verified in both impls |
| UnavailableAudioDevices::because | 71-73 | REAL | — | tested |
| UnavailableAudioDevices::reason | 77-79 | REAL | — | tested |
| Unavailable impl: enumerate/subscribe/poll | 82-94 | REAL | — | all three return the typed reason clone; host-tested (`unavailable_audio_devices_returns_the_typed_reason`) |
| stable_device_id | 106-114 | REAL | — | FNV-1a-64 constants correct (0xcbf29ce484222325 / 0x100000001b3), goldens independently recomputed and matched; collision risk honestly documented |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| stable_device_id_is_deterministic_and_pinned (seam_behaviour.rs:300-313) | YES | pins two externally-computable goldens (I recomputed both: exact match), plus determinism/distinctness/empty-vs-"a" assertions |
| unavailable_audio_devices_returns_the_typed_reason | YES | asserts exact typed error from all three methods |
| typed_fallbacks_satisfy_the_seam_traits | YES | trait-object dispatch through the fallback |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the streaming half (AudioBackend::open ...) stays behind the ADR-005 admission gate" | audio.rs:6-9 | REAL | audio-output cpal not admitted (records/cpal-0.18.2.md, matrix row platform-aout-cpal notes); no open() on this seam |
| "poll-style integrations (PulseAudio mainloop) dispatch here" | audio.rs:51-52 | REAL | pulse.rs:734-739 pump_once |
| "Push-based integrations ... return 0" | audio.rs:49-51 | REAL | wasapi.rs:667-671 Ok(0) |
| "the mapping is FNV-1a-64 — deterministic ... not collision-free" | audio.rs:100-104 | REAL | constants verified; risk documented; recomputation matched |
| "Re-subscribing replaces the previous queue" | audio.rs:40-41 | REAL | wasapi.rs:624 drops old watch first |

## 7. Dead / duplicated / unreachable code
Duplicated: byte-identical in platform-linux + platform-macos (F-002). Unreachable: the `WasapiAudioDevices` implementation path from shipped entry points (F-001). No dead items in-file: every function tested or wired.

## 8. Dependencies & flags
Depends on audio-output (DeviceDesc/DeviceId — the declared L4 edge) and crate-internal events/error. No unsafe, no cfg gates in this file, no cargo features.

## 9. Open questions for the next agent
- If the ADR-005 gate opens and a real backend consumes these DeviceDescs, the id-minting collision doc (audio.rs:101-104) should be revisited: ids are minted per enumeration from OS strings; two enumerations of the same device must yield the same id (they do — FNV is deterministic) but two devices colliding would silently alias in `subscribe` event routing. Fine at device counts; worth a comment in the unified crate.
