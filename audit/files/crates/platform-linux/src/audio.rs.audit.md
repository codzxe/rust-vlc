# AUDIT: crates/platform-linux/src/audio.rs
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-21 |
| Lines / functions | 114 / 6 items (AudioDevices trait, UnavailableAudioDevices + because/reason + trait impl, stable_device_id) |
| File verdict | REAL (fallback + hash real and test-pinned; the PulseAudio impl lives in linux/pulse.rs, other files) |
| Reachable from shipped entry point? | NO — crate-level orphan (no manifest depends on platform-linux; see the dl.rs audit F-001 in this wave for the full evidence). Within the crate: pulse.rs implements this trait; tests exercise both the real impl (libpulse.so.0 present) and the fallback. Tests executed in baseline: seam_behaviour `stable_device_id_is_deterministic_and_pinned`, `unavailable_audio_devices_returns_the_typed_reason` — ok (orchrun log 3638+); linux_integrations 13/13 ok. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg `pub trait AudioBackend\|pub struct DeviceId\|pub struct DeviceDesc` in audio-output/src (device.rs:28/46/88 — the doc-linked vocabulary exists) · read seam_behaviour.rs stable_device_id test (exact FNV pins) · sed -n quotes verified · baseline orchrun log (seam_behaviour + linux_integrations results) |

## 1. One-paragraph verdict
A clean seam definition: three methods (enumerate/subscribe/poll) against the
audio-output vocabulary, a typed fallback that refuses every method with the
exact reason it was constructed with, and an FNV-1a-64 device-id mint whose
constants are correct, whose collision caveat is documented honestly, and
whose exact outputs are pinned by a passing test (including the canonical
FNV-1a value for "a"). Zero defects found; the only shadow is the crate's
orphan status, filed once at crate level (dl.rs audit, this wave) and
cross-referenced here rather than double-counted.

## 2. What was checked, and how (zero-findings file — full list)
— FNV-1a constants (108-112): offset basis 0xcbf29ce484222325 and prime
0x00000100000001b3 are the standard 64-bit FNV-1a values; the pinned test
value for "a" (0xaf63dc4c8601ec8c) matches the canonical FNV-1a-64("a") —
hand-verified. Determinism, inequality for distinct inputs, and the empty
string are all asserted (seam_behaviour.rs, baseline ok).
— Fallback totality (82-94): all three trait methods return the constructed
reason; no fake success, no ignored input semantics beyond the deliberately
unused queue parameter (named `_queue` — the refusal is the contract).
— Vocabulary claims: `DeviceDesc`/`DeviceId` (audio-output/src/device.rs:46/28)
and `AudioBackend::enumerate` (device.rs:88-91) all exist — the intra-doc
links resolve; ARCH-6 honored (no OS types in any signature here).
— Doc claims: "Re-subscribing replaces the previous queue" is an
implementation contract for pulse.rs (that auditor's scope); "push-based
integrations … return 0" documents the poll() return contract symmetrically.
— `Send` supertrait: matches the cross-thread queue push design
(DeviceEventQueue is shared via Arc).
— Error-path cloning (reason.clone() per refusal): PlatformError is small and
this is the cold path — no concern.
— No panics, unwrap, unsafe, features, or dead code (read line-by-line).

## 3. Findings
None in this file. (The crate-level orphan S3 — platform-linux has zero
dependents — is filed once in this wave at
audit/files/crates/platform-linux/src/linux/dl.rs.audit.md F-001; this file
inherits that status. Re-filing per file would double-count one defect.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-12 | REAL | — | v0 scope (enumeration/notification half) honest; ADR-005 gate cited for the streaming half |
| trait AudioDevices | 29-57 | REAL | — | 3 methods, typed errors documented per method; Send supertrait |
| struct UnavailableAudioDevices | 63-66 | REAL | — | carries the reason |
| UnavailableAudioDevices::because / reason | 68-79 | REAL | — | #[must_use] both |
| impl AudioDevices for UnavailableAudioDevices | 82-94 | REAL | — | total typed refusal; test-pinned |
| stable_device_id | 105-114 | REAL | — | correct FNV-1a-64; collision caveat documented; exact values pinned by test |

## 5. Test honesty
No in-file tests. Baseline-executed coverage:
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| seam_behaviour `stable_device_id_is_deterministic_and_pinned` | YES | exact hash pins for 3 shapes (short, ALSA-style, WASAPI-GUID-style) + determinism + inequality + empty-string |
| seam_behaviour `unavailable_audio_devices_returns_the_typed_reason` | YES | every method's refusal payload pinned |
| linux_integrations `pulse_probe_reports_this_hosts_truth` / `pulse_subscribe_without_a_server_is_the_typed_error` | YES | the REAL implementation of this trait runs against libpulse.so.0 (open success, connect-failure typed) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every method answers a typed error naming the reason, never a fake success" (fallback) | audio.rs:26-28 | REAL | impl 82-94 + seam test |
| "An empty list is honest ('no devices'), matching AudioBackend::enumerate's contract" | audio.rs:30-31 | REAL | audio-output device.rs:91 returns bare Vec (empty = none); consistent |
| "the mapping is FNV-1a-64 — deterministic … documented as not collision-free" | audio.rs:100-104 | REAL | constants verified; pins in test; caveat explicit |
| "ids are only meaningful within one backend instance" | audio.rs:103-104 | REAL-UNPROVEN (scope claim) | consistent with per-instance minting; no cross-instance consumer exists yet (crate orphaned) |
| "the streaming half stays behind the ADR-005 admission gate" | audio.rs:7-9 | REAL | no open/render callback here; records/cpal-0.18.2.md referenced |

## 7. Dead / duplicated / unreachable code
None in-file. Crate-level orphan: see dl.rs audit F-001 (this wave).

## 8. Dependencies & flags
audio-output (vocabulary) + crate-internal error/events; no external deps, no
features, no unsafe. Compiles in the default Linux workspace lanes.

## 9. Open questions for the next agent
- pulse.rs auditor: confirm `subscribe`'s "re-subscribing replaces the
  previous queue" is actually implemented (the trait contract lives here, the
  obligation there).
- If the ADR-005 gate ever opens, the real audio backend should reuse
  `stable_device_id` — its pinned values are a compatibility contract.
