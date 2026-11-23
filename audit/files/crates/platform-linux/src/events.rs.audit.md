# AUDIT: crates/platform-linux/src/events.rs
| Field | Value |
|---|---|
| Auditor | W08-A04 |
| Date | 2026-11-23 |
| Lines / functions | 153 / 9 methods + 1 enum (5 variants) |
| File verdict | REAL code, UNREACHABLE in product — the queue works and is tested, but no shipped binary constructs, fills, or drains it |
| Reachable from shipped entry point? | NO. Producer side: `linux/pulse.rs:246-261` pushes from the dlopen'd PulseAudio subscription callback (real FFI, real-server behavior `[VERIFY]` — no PA daemon in sandbox). Consumer side: NOTHING. No app depends on platform-linux (rg over apps/*/Cargo.toml: zero hits); the engine-side hook that would consume such events (`Engine::poll_hotplug`) has zero callers outside playback-engine's own tests. product.toml:163 honestly keeps `audio-device-enumeration = false` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cargo test -p platform-linux` → 13 linux_integrations + 15 seam_behaviour, ALL PASS (exit 0); rg dependency/caller traces; sed quote verification |

## 1. One-paragraph verdict
The queue itself is correct, bounded, and honestly counter-accounted:
drop-oldest under flood, poison-safe push, monotonic pushed/dropped
counters, and real tests that assert FIFO order, retention of the
newest three under overflow, and clamp-to-one capacity (I ran them,
28/28 in the crate pass). The real producer exists too — the
PulseAudio subscription callback in pulse.rs translates sink/card
events through it. But the entire notification lane is dead in the
product: no shipped binary links platform-linux, and the engine hook
that would drain such a queue (`poll_hotplug`) is called by nobody
outside tests. This is a well-built organ with no body attached —
honest only because product.toml keeps the feature flag false.

## 2. What actually works (verified)
— verified by `cargo test -p platform-linux` (exit 0):
`event_queue_drains_fifo_and_empties`,
`event_queue_is_bounded_and_counts_drops`,
`event_queue_capacity_is_clamped_to_one` (seam_behaviour.rs:241-300)
plus the absent-server typed-fallback probes in linux_integrations.rs
(13 pass). The PulseAudio FFI producer compiles and its absent-server
path is exercised; live-server push behavior is UNVERIFIED (no
PulseAudio daemon in the sandbox — same limitation lib.rs:31-33
records).

## 3. Findings
F-001 [S3]
Where: crates/platform-linux/src/events.rs:1-11 (module purpose) vs
the shipped lanes.
Code:
```
//! The bounded device-event queue: where OS change notifications
//! land (card 7.A2; the event-plumbing half of the notification lane).
```
What is wrong: nothing lands anywhere a user can observe. The queue
is constructed only by platform-linux's own tests; apps/cli,
apps/desktop, apps/web do not depend on platform-linux (verified:
rg "platform" over apps/*/Cargo.toml → no match), and
`Engine::poll_hotplug` / `observe_audio_reopen` / `observe_audio_loss`
have zero callers in apps/ (rg). The whole hotplug lane — platform
queue → engine recovery hooks — is unwired end to end in every
shipped binary. Not a doc fraud (product.toml:163
`audio-device-enumeration = false # Phase 3/4, host-dependent` is
truthful), but two crates of real, tested code with no consumer.
Confidence: HIGH — Verification: rg (three independent traces).
F-002 [S4]
Where: crates/platform-linux/src/events.rs:39-47.
Code:
```
    /// A device transitioned active/inactive (Windows
    /// `DEVICE_STATE_NOTPRESENT`↔`ACTIVE`; disabled/unplugged states
    /// surface here rather than as spurious add/remove pairs).
    AudioDeviceStateChanged {
```
What is wrong: on Linux this variant is never constructed —
pulse.rs's translation emits only `AudioDeviceAdded`,
`AudioDeviceRemoved`, and `AudioSetChanged` (pulse.rs:246-261, change
events map to "re-enumerate"). Dead vocabulary in this crate whose
doc describes another OS. Confidence: HIGH — Verification: rg
(`StateChanged` absent from platform-linux/src/linux/*).
F-003 [S4]
Where: crates/platform-linux/src/events.rs:96-109.
Code:
```
        } else {
            // A poisoned queue means a consumer panicked mid-drain;
            // dropping the event is the honest, non-crashing choice.
            self.dropped.fetch_add(1, Ordering::Relaxed);
            false
        }
```
What is wrong: the `false` return and the `dropped` counter conflate
"evicted by capacity" with "discarded because a consumer panicked" —
an operator reading `dropped()` cannot distinguish a notification
flood from a crashing consumer, the one diagnosis this counter exists
for. A separate `poisoned_drops` (or an error return) would keep the
ledger honest. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PlatformEvent (enum) | 27-57 | REAL | F-002 | 5 variants; StateChanged never minted on Linux |
| DeviceEventQueue::with_capacity | 73-80 | REAL | — | clamp to ≥1, Arc-wrapped |
| DeviceEventQueue::new | 85-87 | REAL | — | 128 default |
| DeviceEventQueue::push | 92-110 | REAL | F-003 | bounded, drop-oldest, poison-safe |
| DeviceEventQueue::drain | 113-119 | REAL | — | whole batch, oldest first |
| DeviceEventQueue::len | 123-128 | REAL | — | 0 on poison |
| DeviceEventQueue::is_empty | 132-134 | REAL | — | |
| DeviceEventQueue::capacity | 138-140 | REAL | — | |
| DeviceEventQueue::dropped | 144-146 | REAL | F-003 | conflates overflow + poison drops |
| DeviceEventQueue::pushed | 150-152 | REAL | — | counts attempts incl. poison drops (doc says "pushed" — accurate) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| event_queue_drains_fifo_and_empties (seam_behaviour.rs:241) | YES | exact ordered vector equality |
| event_queue_is_bounded_and_counts_drops (276) | YES | 6 pushes / cap 3 → len 3, dropped 3, newest survive |
| event_queue_capacity_is_clamped_to_one (297) | YES | cap(0)→1, push still retained |
No tests in events.rs itself — all queue tests live in
tests/seam_behaviour.rs (same crate, acceptable).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "OS notification callbacks arrive on foreign threads … the PulseAudio event loop here" | events.rs:4-5 | REAL-UNPROVEN | FFI callback exists (pulse.rs:231-265); live-server arrival UNVERIFIED (no daemon) |
| "a notification flood can never grow the process" | events.rs:8-10 | REAL | bounded-drop test passes |
| "tests drive the same push path … the production path is identical" | events.rs:15-17 | REAL | tests call the same `push`; but the *consumption* path has no production existence (F-001) |
| "audio-device-enumeration = false" | product.toml:163 | REAL | matches the unwired lane |

## 7. Dead / duplicated / unreachable code
The entire module is product-dead (F-001). Additionally mirrored
copies of this exact design exist in platform-windows/src/events.rs
and platform-macos/src/events.rs (unified-L5 crate is a recorded
[PROPOSAL] in platform-linux lib.rs:44-47) and a fourth variant in
crates/device-discovery/src/events.rs — four parallel implementations
of one 150-line queue.

## 8. Dependencies & flags
`audio_output::DeviceId` (the only cross-crate type — the ADR-005
vocabulary); std only; no unsafe in this file; cfg(target_os) gates
only the producer module, not the queue. Compiles on all targets.

## 9. Open questions for the next agent
- Who audits crates/device-discovery (the fourth queue) and
playback-engine/src/hotplug.rs? The three platform queues + engine
hooks + device-discovery form one unwired lane; a lane-level verdict
should exist somewhere central.
- pulse.rs's `stable_device_id` — hash stability across sessions
claims "stable id (same minting as enumeration)": verify the minting
actually is stable (name-hash vs index) when auditing pulse.rs.
