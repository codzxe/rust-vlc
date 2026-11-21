# AUDIT: crates/platform-macos/src/audio.rs
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-21 |
| Lines / functions | 115 / 7 items (trait + fallback struct + 2 constructors + 3-method impls + hash fn) |
| File verdict | UNREACHABLE (clean seam vocabulary; orphan crate — cross-ref W03-A02 coreaudio.rs audit / W01-A09 manifest F-001, not re-filed) |
| Reachable from shipped entry point? | NO. Consumers: `lib.rs:64/125/143-163` (`audio_seam` inside `MacOsPlatform::discover`) + `tests/seam_behaviour.rs` — no app depends on platform-macos. Compiles on every OS (no cfg gate) and is executed by the 16/16 seam tests (orchrun :3691-3712). The shipped audio path runs entirely elsewhere (audio-output null device, per INDEX.md standing defects). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `sed -n` full read (twice — also read during the coreaudio.rs audit) · `rg -n 'fn enumerate' crates/audio-output/src/*.rs` (device.rs:88-91 — the cited `AudioBackend::enumerate` contract verified: "Empty is honest") · FNV-1a-64 vector recomputed from the algorithm (`"a"` → 0xaf63dc4c8601ec8c, the published test vector) · `rg -n 'stable_device_id' tests/seam_behaviour.rs` (:290-300 — pinned vectors) · orchrun :3691-3712 (16/16 pass) |

## 1. One-paragraph verdict
The audio seam is a small, honest vocabulary: three methods (enumerate/subscribe/poll), a typed fallback that answers the same reason from every method (never a fake success), and an FNV-1a-64 device-id minter whose vectors I recomputed and whose collision caveat is documented rather than hidden. The doc's cross-crate contract claim ("empty list is honest, matching AudioBackend::enumerate's contract") is verified against the real audio-output trait. The one defect is a documentation gap with teeth: the trait's `subscribe` contract specifies re-subscribe replacement but is silent on failure semantics — and the only real implementation (coreaudio.rs :684-691) exploits exactly that gap by destroying the working subscription before the new probe succeeds (my coreaudio F-001). Like the rest of this orphan crate, nothing shipped can reach any of it.

## 2. What actually works (verified)
— `stable_device_id` (:106-114): FNV-1a-64 over UTF-8, offset basis 0xcbf29ce484222325, prime 0x100000001b3 — the pinned vector `stable_device_id("a").handle() == 0xaf63_dc4c_8601_ec8c` (seam_behaviour.rs:291) is the published FNV-1a-64 test vector; recomputed by this auditor. Collision non-freedom documented with a birthday-bound argument (:99-104).
— Typed fallback (:82-94): all three methods return `Err(reason.clone())` — pinned by `unavailable_audio_devices_returns_the_typed_reason` (seam_behaviour.rs:35-46, passed in baseline) and exercised through `dyn AudioDevices` by `typed_fallbacks_satisfy_the_seam_traits` (:80-97).
— Contract cross-reference (:30-32): `audio_output::AudioBackend::enumerate` really does specify "Empty is honest ('no devices'), never a placeholder entry" (device.rs:88-91) — the claim is true, not decorative.
— `poll` contract (:48-52) names push-based integrations explicitly — the CoreAudio implementation's `Ok(0)` is contract-conformant, not a stub (verified from the coreaudio side).
— All 16 platform-macos seam tests pass in the no-fail-fast baseline (orchrun :3691-3712).

## 3. Findings

### F-001 [S4]
Where: crates/platform-macos/src/audio.rs:38-46
Code:
```
    /// Subscribe to device changes; the integration pushes
    /// [`PlatformEvent`]s into `queue` (bounded — the queue owns its
    /// memory bound, D-3). Re-subscribing replaces the previous
    /// queue.
    ///
    /// # Errors
    /// Typed [`PlatformError`] when notifications are not possible
    /// (integration absent, server unreachable).
    fn subscribe(&mut self, queue: Arc<DeviceEventQueue>) -> Result<(), PlatformError>;
```
What is wrong: The contract specifies the success case ("re-subscribing replaces the previous queue") and the error case ("when notifications are not possible"), but is silent on the interaction: what happens to the PREVIOUS subscription when a re-subscribe FAILS? The only real implementation answers by demolition — coreaudio.rs :684-691 drops the live `Watch` (unregistering both HAL listeners) before the new probe can fail, leaving the adapter with no subscription and an `Err` (my coreaudio.rs F-001, filed this wave).
Why it matters: The trait doc is the seam's law; an implementer reading it cannot know whether failure must preserve the old subscription. The shipped-behavior bug in the one implementation is a direct consequence of the unspecified case. S4 because the crate is unreachable (nothing shipped can trigger it), but the doc gap is the root cause of what would be S2 behavior if wired.
What correct looks like: One sentence in the contract: "A failed re-subscribe leaves the previous subscription in force" (then coreaudio's ordering becomes a contract violation, fixable) — or — "A failed re-subscribe may have destroyed the previous subscription; callers must re-subscribe on error" (then coreaudio is conformant and callers are warned).
Confidence: HIGH — Verification: read (trait + the implementation's ordering, cross-file).

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-19 | REAL | — | v0 scope (enumeration/notification only; streaming behind ADR-005 gate) honestly disclosed. |
| `trait AudioDevices` | 29-57 | REAL | S4 | 3 methods, `Send` bound; F-001 subscribe-contract gap. |
| `UnavailableAudioDevices` | 63-80 | REAL | — | `because`/`reason`; `#[must_use]` both. |
| `impl AudioDevices for UnavailableAudioDevices` | 82-94 | REAL | — | Uniform typed refusal; `_queue` param ignored by design (the reason IS the answer). |
| `stable_device_id` | 106-114 | REAL | — | FNV-1a-64, vector-pinned, collision caveat documented. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `unavailable_audio_devices_returns_the_typed_reason` (seam_behaviour.rs:35) | YES | Exact typed error asserted for all three methods. |
| `typed_fallbacks_satisfy_the_seam_traits` (:80) | YES | Object-safety + error vocabulary through `dyn`. |
| `stable_device_id_is_deterministic_and_pinned` (:290) | YES | Published FNV vectors + determinism + distinctness + empty-string distinctness. |
All passed in baseline (16/16, orchrun :3691-3712).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "An empty list is honest ('no devices'), matching AudioBackend::enumerate's contract" | audio.rs:30-32 | REAL | device.rs:88-91 verified verbatim. |
| "v0 scope is the enumeration/notification half… the streaming half stays behind the ADR-005 admission gate" | audio.rs:6-9 | REAL | Trait has no open/stream method; consistent with the product reality that audio plays only into the null device (INDEX.md standing defects). |
| "The fallback is a real implementation of graceful degradation… never a fake success" | audio.rs:25-28 | REAL | Code + tests. |
| "Re-subscribing replaces the previous queue" | audio.rs:40-41 | REAL-UNPROVEN | True in the implementation's success path; failure semantics unspecified (F-001). |

## 7. Dead / duplicated / unreachable code
Unreachable in product (orphan crate — see coreaudio.rs audit §7; not re-filed). Mirrored byte-for-byte (modulo OS names) in platform-linux/src/audio.rs and platform-windows/src/audio.rs — documented `[PROPOSAL]` unification, same drift hazard as capability.rs.

## 8. Dependencies & flags
`audio_output::{DeviceDesc, DeviceId}`, `crate::error::PlatformError`, `crate::events::DeviceEventQueue`, `std::sync::Arc` — all used. No cfg gates (compiles on every OS), no unsafe, no external deps.

## 9. Open questions for the next agent
- platform-linux/src/audio.rs owner (wave-3 colleague): same subscribe-contract gap (F-001) applies to the PulseAudio implementation — check whether its re-subscribe failure path preserves or destroys the old subscription.
- If the Lead ever unifies the three seam crates (the recorded [PROPOSAL]), this trait should gain the failure-semantics sentence first, or the three implementations will codify three different answers.
