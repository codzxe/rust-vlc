# AUDIT: crates/platform-windows/src/events.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-21 |
| Lines / functions | 153 / 12 (1 enum 5 variants, 1 struct + 8 methods, module docs; 0 tests in file, 4 in tests/seam_behaviour.rs) |
| File verdict | UNREACHABLE (real, tested code; crate orphaned — dxgi.rs F-001) |
| Reachable from shipped entry point? | NO (platform-windows is a dependency of no app/engine crate — dxgi.rs audit F-001; the queue's only live consumers are the crate's own tests and the cfg(windows) WASAPI module that itself is unlinked) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | `diff` of this file against crates/platform-linux/src/events.rs (byte-identical except one doc line — F-002) · baseline log: 4 event_queue_* tests passed (:3721-3726 within the 16/16 suite) · lock/poisoning path review |

## 1. One-paragraph verdict
A correct little bounded queue: hard capacity clamped to ≥1, oldest-drop under flood with a monotonic dropped counter, short no-allocation critical sections safe for OS callback threads, honest poison handling (event dropped + counted, never a crash or block), and four real tests including a thread-safety hammer — all passing in the no-fail-fast baseline. Two defects: the crate-wide orphan status means no shipped build can ever push or drain this queue (dxgi.rs F-001, cited); and the file is a byte-for-byte copy of platform-linux's events.rs (verified by diff, one doc line differs) — leaving Linux documentation ("the PulseAudio event loop here", PulseAudio/CoreAudio examples in `AudioSetChanged`) inside the Windows crate, which misleads any reader about what "here" is.

## 2. What actually works (verified)
— 4 event tests passed in the baseline (`orchrun-test-workspace-nofailfast.log.txt:3721-3726`): `event_queue_is_bounded_and_counts_drops`, `event_queue_drains_fifo_and_empties`, `event_queue_capacity_is_clamped_to_one`, `event_queue_is_thread_safe_for_callback_threads`.
— Boundedness verified by reading: `push` (:92-110) pops the oldest at capacity before pushing (hard bound, D-3), increments `dropped` on both the capacity path and the poison path; the critical section is a lock + len check + pop + push — no callbacks, no allocation.
— Poison semantics verified: `push` on a poisoned lock counts the drop and returns `false` (:104-109); `drain`/`len` return empty/0 rather than panicking — a consumer panic mid-drain cannot wedge the OS callback thread.
- `with_capacity(0)` clamps to 1 (:76) — pinned by `event_queue_capacity_is_clamped_to_one`.

## 3. Findings
### F-001 [S3]
Where: crates/platform-windows/src/events.rs:59-67 (the queue nobody ships)
Code:
```
pub struct DeviceEventQueue {
    inner: Mutex<VecDeque<PlatformEvent>>,
    capacity: usize,
    dropped: AtomicU64,
    pushed: AtomicU64,
}
```
What is wrong / why it matters / what correct looks like: identical crate-level defect as dxgi.rs F-001 — platform-windows is linked by nothing; this "event-plumbing half of the notification lane" has no producer in any shipped build (the WASAPI notifier that would push into it is cfg(windows) and equally unlinked). Cite dxgi.rs audit §3 F-001 for the evidence chain (Cargo.toml exhaustion).
Confidence: HIGH — Verification: same as dxgi.rs F-001.
### F-002 [S4]
Where: crates/platform-windows/src/events.rs:4-5 and :53-55
Code (events.rs:4-6):
```
//! OS notification callbacks arrive on foreign threads (the OS
//! callback thread on Windows, the PulseAudio event loop here), so
//! the queue is `Mutex` + `VecDeque` with a **hard capacity**: when
```
Code (events.rs:53-55):
```
    /// The device set changed in a way the OS did not itemize —
    /// re-enumerate (PulseAudio server events, CoreAudio
    /// `kAudioHardwarePropertyDevices` changes).
```
What is wrong: `diff` proves this file is a byte-for-byte copy of `crates/platform-linux/src/events.rs` (sole difference: "card 7.A1" vs "card 7.A2" in line 2). The Windows crate's module docs therefore say "the PulseAudio event loop here" and cite PulseAudio/CoreAudio as the `AudioSetChanged` producers — platform-wrong documentation carried by copy-paste. The duplication itself is documented policy (lib.rs: "seam vocabulary ... deliberately mirrored across the three platform-* crates ... [PROPOSAL] to unify"), which keeps this at hygiene level.
Why it matters: a maintainer reading the Windows crate is told the PulseAudio event loop is "here"; the per-platform producer examples are fiction for two of the three crates that carry them.
What correct looks like: per-platform doc lines (Windows: "the WASAPI notification thread"), or the promised L5 unification so the doc exists once.
Confidence: HIGH — Verification: diff run (exit 0 with the single-line filter).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-17 | REAL | S4 | F-002 copy-paste platform soup; the design claims (bounded, dumb queue, sanctioned test double) are accurate |
| `enum PlatformEvent` (5 variants) | 27-57 | REAL | — | OS-neutral vocabulary (ARCH-6); AudioDeviceStateChanged doc correctly Windows-specific; AudioSetChanged doc wrong-platform (F-002) |
| `DeviceEventQueue` struct | 61-67 | REAL | S3 | F-001 orphan |
| `with_capacity` | 73-80 | REAL | — | clamp-to-1 tested; returns Arc for callback-thread sharing |
| `new` | 85-87 | REAL | — | 128 default, rationale documented |
| `push` | 92-110 | REAL | — | drop-oldest + counters; poison-safe; table note: `dropped` conflates capacity-drops with poison-drops (a stats consumer cannot distinguish) — minor, documented behavior |
| `drain` | 113-119 | REAL | — | whole-batch take, poison → empty |
| `len` / `is_empty` | 123-134 | REAL | — | poison → 0 |
| `capacity` / `dropped` / `pushed` | 138-152 | REAL | — | Relaxed loads — fine for stats; `pushed` counts pushes incl. dropped ones (documented: "events ever pushed") |

## 5. Test honesty (4 tests in tests/seam_behaviour.rs, all passed — baseline :3721-3726)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `event_queue_is_bounded_and_counts_drops` | YES | overfill asserts drop count + retention bound |
| `event_queue_drains_fifo_and_empties` | YES | order + emptiness asserts |
| `event_queue_capacity_is_clamped_to_one` | YES | capacity-0 queue still retains one event |
| `event_queue_is_thread_safe_for_callback_threads` | YES | concurrent push/drain hammer (test name at :3725; passed) |
| Gap | — | no test for the poisoned-lock paths (:104-109, :117-118) — poison behavior is UNTESTED (would need a panicking consumer) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "when full, the oldest event is dropped and a monotonic dropped counter increments ... a notification flood can never grow the process" | events.rs:6-10 | REAL | push :97-102 read; bounded test passed |
| "the OS callback thread on Windows, the PulseAudio event loop here" | events.rs:4-5 | FALSE (platform) | F-002: this is the Windows crate; text copied from platform-linux |
| "tests drive the same push path with simulated events (the production path is identical)" | events.rs:15-17 | REAL | push is the sole entry; tests passed |

## 7. Dead / duplicated / unreachable code
- Whole file unreachable in shipped builds (F-001).
- Duplicated byte-for-byte in platform-linux/src/events.rs (and presumably platform-macos) — F-002; the third copy should be checked by that crate's auditor.

## 8. Dependencies & flags
- std + audio-output (DeviceId) only. No unsafe, no cfg gates inside the file (the file itself is not cfg-gated — only the `windows/` FFI modules are; so this file DOES compile on Linux, which is why its tests run and pass in the baseline).

## 9. Open questions for the next agent
- platform-macos auditor: confirm the third events.rs copy and its doc's platform references (CoreAudio "here"?).
- If the L5 unification lands: the poison-path tests (missing, §5 gap) should be added once, in the shared crate.
- audio-output wave: `DeviceId` minting stability ("same minting as enumeration") — is the id truly stable across re-enumeration? The queue's `AudioDeviceRemoved{id}` semantics depend on it.
