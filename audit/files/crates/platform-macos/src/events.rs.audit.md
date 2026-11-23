# AUDIT: crates/platform-macos/src/events.rs
| Field | Value |
|---|---|
| Auditor | W08-A01 |
| Date | 2026-11-23 |
| Lines / functions | 153 / 9 methods |
| File verdict | REAL (logic, tested) + UNREACHABLE (shipped builds) |
| Reachable from shipped entry point? | NO — `platform-macos` is an orphan crate (no Cargo.toml depends on it); in-crate callers: audio.rs:19,46,87 + macos/coreaudio.rs:64,539,684 + tests |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | `cargo test -p platform-macos` -> 16 passed / 0 failed (exit 0); `diff` of the three platform events.rs files; rg for `dropped()/pushed()` readers; sed quote verification |

## 1. One-paragraph verdict
A small, correct, honestly-bounded `Mutex<VecDeque>` event queue with drop-oldest policy and monotonic counters; its behavior is genuinely tested (I ran the crate suite: 16/16 green, including FIFO, bounds+drop counting, capacity clamp, thread safety). Two problems: (1) it is dead in shipped builds — `platform-macos` has zero dependents, so no shipped entry point can ever construct this queue; (2) the file is a byte-identical triple copy of the windows/linux variants (only the card number on line 2 differs), and its module doc still narrates **PulseAudio/DRM — Linux concepts — inside the macOS crate**, pure copy-paste residue. The logic earns REAL; the shipping reality is UNREACHABLE.

## 2. What actually works (verified)
— verified by `cargo test -p platform-macos` -> "test result: ok. 16 passed; 0 failed" (event_queue_drains_fifo_and_empties, event_queue_is_bounded_and_counts_drops, event_queue_capacity_is_clamped_to_one, event_queue_is_thread_safe_for_listener_threads all pass).
— verified by `diff platform-macos/src/events.rs platform-linux/src/events.rs` -> only line 2 differs ("card 7.A3" vs "card 7.A2"); same vs platform-windows.

## 3. Findings
F-001 [S2]
Where: crates/platform-macos/src/events.rs:62-67 (the whole `DeviceEventQueue`)
Code:
```
pub struct DeviceEventQueue {
    inner: Mutex<VecDeque<PlatformEvent>>,
    capacity: usize,
    dropped: AtomicU64,
    pushed: AtomicU64,
}
```
What is wrong: `platform-macos` is an orphan crate — `rg -n "platform-" --glob **/Cargo.toml` shows no app or core crate depends on any `platform-*` crate. `MacOsPlatform::discover` is called only from its own tests. This queue, and the CoreAudio integration that pushes into it (macos/coreaudio.rs:539), can never run in a shipped lumen-cli/desktop build on any OS.
Why it matters: The macOS notification lane (and its 3 siblings) is architecture-dead; any doc/matrix row claiming macOS device-change notifications is FALSE-by-unreachability.
What correct looks like: apps declare `[target.'cfg(target_os = "macos")'.dependencies] platform-macos` and the host wires `MacOsPlatform::discover`.
Confidence: HIGH — Verification: read (manifests + rg call-chain).

F-002 [S3]
Where: crates/platform-macos/src/events.rs:104-109
Code:
```
        } else {
            // A poisoned queue means a consumer panicked mid-drain;
            // dropping the event is the honest, non-crashing choice.
            self.dropped.fetch_add(1, Ordering::Relaxed);
            false
        }
```
What is wrong: A poisoned mutex never un-poisons. After one consumer panic while holding the lock, **every future push is silently dropped forever** — a permanent notification blackout with no recovery path and no surfaced signal. The `dropped`/`pushed` counters that would reveal this have zero production readers (rg over crates/apps: only test assertions in other crates' own queues read such counters).
Why it matters: A single UI-tick panic permanently kills device-change events and nothing in the product can notice.
What correct looks like: On poison, replace the guard (`into_inner` or re-init) or surface a typed degradation event.
Confidence: MEDIUM — Verification: read; poison never exercised by tests.

F-003 [S4]
Where: crates/platform-macos/src/events.rs:4-6
Code:
```
//! OS notification callbacks arrive on foreign threads (the OS
//! callback thread on Windows, the PulseAudio event loop here), so
//! the queue is `Mutex` + `VecDeque` with a **hard capacity**: when
```
What is wrong: This is the macOS crate; the doc narrates "the PulseAudio event loop here" (Linux's audio server) and line 13-14 "it knows nothing about PulseAudio or DRM" (DRM is the Linux graphics stack). On macOS the producers are CoreAudio listeners (per this crate's own lib.rs:26-28). Copy-paste residue that misstates the OS the file ships for.
Confidence: HIGH — Verification: read (diff vs platform-linux proves the copy).

F-004 [S4]
Where: crates/platform-macos/src/events.rs (whole file) vs platform-windows/src/events.rs and platform-linux/src/events.rs
Code: (diff output, only line 2 differs)
```
2c2
< //! land (card 7.A3; the event-plumbing half of the notification lane).
---
> //! land (card 7.A2; the event-plumbing half of the notification lane).
```
What is wrong: The file is triplicated byte-for-byte across three crates (and a fourth structural sibling exists: device-discovery's `VolumeEventQueue`). The mirroring is admitted in lib.rs:47-50 with a `[PROPOSAL]` to unify — but until then any fix (e.g., F-002's poison handling) must be applied four times.
Confidence: HIGH — Verification: ran diff.

F-005 [S4]
Where: crates/platform-macos/src/events.rs:94-95
Code:
```
        // Lock discipline: short critical section, no callbacks, no
        // allocation — push from an OS callback thread is safe.
```
What is wrong: "no allocation" is steady-state only: the first `capacity` pushes grow the VecDeque's ring buffer **inside the mutex**; a fresh queue reached from an OS callback thread allocates under the lock. After warm-up (len bounded at capacity) pop_front/push_back reuses slots and the claim holds, but the unqualified comment overstates it.
Confidence: MEDIUM — Verification: read (VecDeque growth semantics); not measured.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PlatformEvent (enum) | 27-57 | REAL | — | 5 variants, engine vocabulary, no OS types |
| DeviceEventQueue::with_capacity | 73-80 | REAL | — | Clamps capacity to min 1 |
| DeviceEventQueue::new | 85-87 | REAL | — | 128 default |
| DeviceEventQueue::push | 92-110 | REAL | S3 | Poison = permanent silent drop (F-002); returns false on drop |
| DeviceEventQueue::drain | 113-119 | REAL | — | Allocates the returned Vec (consumer side; fine) |
| DeviceEventQueue::len | 123-128 | REAL | — | 0 on poison |
| DeviceEventQueue::is_empty | 132-134 | REAL | — | Delegates |
| DeviceEventQueue::capacity | 138-140 | REAL | — | Accessor |
| DeviceEventQueue::dropped | 144-146 | REAL | — | No production reader (F-002) |
| DeviceEventQueue::pushed | 150-152 | REAL | — | No production reader |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| event_queue_drains_fifo_and_empties | YES | Order + emptiness asserted |
| event_queue_is_bounded_and_counts_drops | YES | Capacity 3, over-push, dropped() asserted |
| event_queue_capacity_is_clamped_to_one | YES | with_capacity(0) must hold 1 event |
| event_queue_is_thread_safe_for_listener_threads | YES (weakly) | Concurrent push/drain with 64-capacity; catches gross races, not the poison blackout (untested path) |
All four pass in my run (16/16). The tests are the sanctioned simulated-event double the doc names (events.rs:15-17) — they exercise the real production push path, no mock of the queue itself.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "when full, the oldest event is dropped and a monotonic `dropped` counter increments" | events.rs:6-8 | REAL | push:97-101; bounded test asserts it |
| "a notification flood can never grow the process" | events.rs:8-9 | REAL | len capped at `capacity` (pop_front before push_back) |
| "the PulseAudio event loop here" (macOS crate) | events.rs:4-5 | FALSE (doc) | macOS has no PulseAudio; producers are CoreAudio listeners (lib.rs:26-28, coreaudio.rs:539) — F-003 |
| "no allocation" in push critical section | events.rs:94-95 | SHALLOW | True only after first-fill warm-up (F-005) |

## 7. Dead / duplicated / unreachable code
- Unreachable from shipped builds (F-001).
- Duplicated: byte-identical in platform-windows and platform-linux (F-004); structural 4th sibling `VolumeEventQueue` in device-discovery/src/events.rs:71.

## 8. Dependencies & flags
Std-only (Mutex/VecDeque/atomics) + `audio_output::DeviceId`. No feature gates; module always compiled, used by the crate's audio seam and the cfg(macos) CoreAudio integration. The crate builds and tests green on Linux (fallback lanes only).

## 9. Open questions for the next agent
- The seam-unification `[PROPOSAL]` (lib.rs:47-50) predates four copies — if the Lead unifies, F-002's poison policy becomes a single-file fix; until then check whether platform-linux/windows copies have the same blackout (they do — same bytes).
- Does any feature-matrix row claim macOS audio device notifications? If yes it is FALSE-by-unreachability (0/252 verified baseline suggests it).
