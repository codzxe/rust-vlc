# AUDIT: crates/platform-macos/src/events.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2026-11-21 |
| Lines / functions | 153 / 15 production units (1 enum with 5 variants, 1 struct, 9 methods) + 4 executed tests in tests/seam_behaviour.rs |
| File verdict | UNREACHABLE (real, correct, well-tested bounded queue; zero production consumers — the crate is a Cargo.lock leaf) |
| Reachable from shipped entry point? | NO. Same leaf-crate proof as lib.rs (no package depends on platform-macos). Within the crate the queue is wired into `AudioDevices::subscribe` (audio.rs:46) and the CoreAudio listener (macos/coreaudio.rs:573,576,585) — but no host compiles any of it. Only executed path: 4 queue tests in tests/seam_behaviour.rs (drain FIFO, bounded drops, capacity clamp, 4-thread concurrency), all green in the no-fail-fast baseline (orchrun 3691-3703). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | full read via Read tool; `diff crates/platform-macos/src/events.rs crates/platform-linux/src/events.rs` and vs `platform-windows` (byte-identical except card id line 2 — doc drift evidence); `rg -n "PlatformEvent::" crates/platform-macos/src/macos/coreaudio.rs` (macOS integration only ever pushes AudioDefaultChanged + AudioSetChanged); `rg -n "AudioDeviceStateChanged" crates/platform-linux/src/linux/*.rs crates/platform-windows/src/windows/*.rs` (StateChanged emitted only by the Windows lane, wasapi.rs:481); `sed -n '4,11p;13,17p;39,47p;92,109p;113,119p'` (quote verification); baseline orchrun lines 3691-3703 (4/4 queue tests green); VecDeque warm-up allocation verified by std semantics (no allocation until first push_back — cited below) |

## 1. One-paragraph verdict
The queue itself is exactly what it claims: a `Mutex<VecDeque>` with a
hard capacity, oldest-first drop, exact monotonic accounting
(`pushed == len + dropped` holds on every path including the poison
path — verified by trace), capacity clamped to ≥1, and a genuinely
useful 4-test battery that ran green. But the module doc is a verbatim
copy of the Linux lane's doc: it tells the reader that events arrive
from "the PulseAudio event loop here" in a crate that contains no
PulseAudio code — the actual macOS producer is CoreAudio HAL
property-listener threads — and the three lane files are
byte-identical except for the card id, proving the doc was never
adapted. Second, `push()`'s "no allocation" lock-discipline claim is
false during warm-up: the `VecDeque::new()` starts with no buffer, so
the first pushes allocate the ring buffer *inside* the critical
section, on whichever thread pushes first — which the comment itself
says may be the OS callback thread. Third, the mutex-poison behavior
(permanent black hole: `drain()` and `len()` pretend empty forever,
`push()` silently counts events as dropped) is documented only at
`push` and has zero test coverage in any committed run.

## 2. What actually works (verified)
— Bounded-drop semantics — verified by trace and executed test:
`event_queue_is_bounded_and_counts_drops` pushes 6 into capacity 3,
asserts len 3 / dropped 3, and after drain asserts `dropped + len == 3`
(tests/seam_behaviour.rs:225-237); ran green (orchrun 3693). The
newest events survive (pop_front before push_back, events.rs:97-102).
— Accounting invariant — verified by trace: `pushed` increments before
the lock (93), and on the poison path BOTH `pushed` (93) and `dropped`
(107) increment, so `pushed == len + dropped` holds on every path;
the 4×100-push concurrency test asserts it exactly (400 =
dropped + len) after joining (tests/seam_behaviour.rs:253-281, green,
orchrun 3695).
— FIFO drain — verified by executed test `event_queue_drains_fifo_and_empties`
(192-222): exact 5-event order pinned, second drain empty. Green.
— Capacity clamp — verified by executed test
`event_queue_capacity_is_clamped_to_one` (240-250): `with_capacity(0)`
→ `capacity() == 1`, second push reports `false`. Green.
— D-3 bounded memory — verified: no allocation path can grow the deque
past `capacity` (len is capped by construction: drop happens before
push when at capacity).

## 3. Findings

### F-001 [S3]
Where: crates/platform-macos/src/events.rs:4-6 (and 13-14, 39-41)
Code:
```rust
//! OS notification callbacks arrive on foreign threads (the OS
//! callback thread on Windows, the PulseAudio event loop here), so
//! the queue is `Mutex` + `VecDeque` with a **hard capacity**:
```
(the companion:)
```rust
//! The queue is deliberately dumb: it knows nothing about PulseAudio
//! or DRM.
```
What is wrong: this is the Linux lane's module doc, pasted verbatim
into the macOS crate. There is no PulseAudio event loop "here" — this
crate's only event producer is the CoreAudio HAL property-listener
callback (macos/coreaudio.rs:545 `on_property_change`, invoked on a
HAL thread), and there is no DRM anywhere in platform-macos (its GPU
integration is Metal). Proof of copy-paste: `diff` against
platform-linux/src/events.rs and platform-windows/src/events.rs shows
all three files byte-identical except line 2's card id (7.A1/7.A2/
7.A3). The drift extends to `AudioDeviceStateChanged`'s variant doc,
which explains only Windows `DEVICE_STATE_NOTPRESENT`↔`ACTIVE`
semantics (39-41) — and in fact that variant is dead vocabulary in
this lane: the macOS integration never pushes it (it pushes only
`AudioDefaultChanged` and `AudioSetChanged`, coreaudio.rs:573,576,585;
only the Windows lane emits StateChanged, wasapi.rs:481).
Why it matters: the module doc is the only place a maintainer learns
where events come from; it names the wrong OS, the wrong event
library, and implies an event (StateChanged) this lane never produces.
The audit's core mission is claims that don't match code; this is a
small one, but it is exactly that class — documentation asserting a
producer the crate does not contain.
What correct looks like: "(the OS callback thread on Windows, the HAL
property-listener thread here)"; "it knows nothing about CoreAudio or
Metal"; and the StateChanged doc should name which lanes actually emit
it (Windows WASAPI; macOS/Linux surface set changes as AudioSetChanged).
Confidence: HIGH — Verification: read + diff of all three lanes +
grep of every `PlatformEvent::` construction site in the crate.

### F-002 [S4]
Where: crates/platform-macos/src/events.rs:94-95
Code:
```rust
        // Lock discipline: short critical section, no callbacks, no
        // allocation — push from an OS callback thread is safe.
```
What is wrong: the "no allocation" claim is false during warm-up.
`with_capacity`/`new` construct `Mutex::new(VecDeque::new())`
(events.rs:75) — `VecDeque::new()` allocates nothing; the ring buffer
is allocated lazily by the first `push_back` calls (std: "Creating a
VecDeque with new is allocation-free; the first push_back allocates"),
doubling until internal capacity ≥ 128. So the first handful of pushes
allocate *inside* the mutex critical section, on whichever thread
pushes first — which, per this module's own story, is the OS callback
thread. (On a CoreAudio HAL *property-listener* thread a malloc is
acceptable — it is not a realtime render callback — so the safety
conclusion survives; the stated reason does not.)
Why it matters: the comment is the invariant a future editor will
preserve; "no allocation" invites relying on it (e.g. moving the queue
into a realtime path) when it only holds after warm-up.
What correct looks like: `VecDeque::with_capacity(capacity)` at
construction (moves the allocation to the consumer thread that builds
the queue) and a comment saying "no allocation after construction".
Confidence: HIGH — Verification: read + std VecDeque semantics
(lazy first allocation); the warm-up window is untestable in this
sandbox (no runs permitted) but follows from the std contract.
UNVERIFIED only in the sense that no execution observed the
allocation; the std guarantee is definitive.

### F-003 [S4]
Where: crates/platform-macos/src/events.rs:104-108 (and 113-118, 124-127)
Code:
```rust
        } else {
            // A poisoned queue means a consumer panicked mid-drain;
            // dropping the event is the honest, non-crashing choice.
            self.dropped.fetch_add(1, Ordering::Relaxed);
            false
        }
```
(the companions:)
```rust
    pub fn drain(&self) -> Vec<PlatformEvent> {
        if let Ok(mut queue) = self.inner.lock() {
            let drained: Vec<PlatformEvent> = queue.drain(..).collect();
            return drained;
        }
        Vec::new()
    }
```
What is wrong: the poison behavior is a permanent black hole and is
undocumented everywhere except `push`. After one consumer panic
mid-drain, the mutex stays poisoned forever (no `into_inner`, no
recovery anywhere): `drain()` returns empty forever, `len()` reports 0
forever while events sit unreachable in the deque, and `push()` counts
every future event into `dropped`. Only `push`'s comment explains the
choice; `drain` and `len` silently pretend emptiness. Zero test
coverage: none of the three `else` branches is executed by any
committed test run (the 4 queue tests never poison the mutex).
Why it matters: untested, undocumented failure-mode behavior in the
one concurrency primitive of the notification lane; a consumer that
panicked once mid-drain leaves a queue that lies about being empty
rather than reporting death (e.g. via `try_lock` + a `Dead` state or
`is_poisoned` surfacing).
What correct looks like: either document the black-hole contract on
`drain`/`len` too, or recover on poison (`lock().unwrap_or_else(|e|
e.into_inner())` — the deque is still structurally valid after a
drain-panic since `drain` is atomic w.r.t. the guard) and add a test
that panics a drainer and asserts subsequent behavior.
Confidence: HIGH — Verification: read (all three poison branches
traced; no recovery path exists in the file; test coverage checked
against the executed baseline list).
Confidence note: whether a panic mid-`drain(..)` can leave the deque
half-drained is a std question (it cannot — the guard drops and the
drain iterator's drops restore... actually `drain` un-dropped elements
are moved back on panic per std docs); the conservative black hole is
still a design choice, not a necessity.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc | 1-17 | FALSE-in-parts | S3 | F-001: PulseAudio/DRM doc is the Linux lane's story; actual producer is CoreAudio HAL threads. |
| `PlatformEvent` enum | 27-57 | REAL | S3 | Shared cross-lane vocabulary; Added/Removed/StateChanged never emitted by the macOS lane (see §7); variant docs drift (F-001). |
| `DeviceEventQueue` struct | 61-67 | REAL | — | Mutex + usize capacity + two AtomicU64 counters; `Debug` derive ok. |
| `with_capacity` | 72-80 | REAL, EXECUTED | — | Returns `Arc` (forces the shared shape `subscribe` needs); clamp `capacity.max(1)` tested. |
| `new` | 84-87 | REAL, EXECUTED | — | 128 default; magic number justified by doc (one hotplug storm). |
| `push` | 92-110 | REAL, EXECUTED | S4 | F-002 warm-up allocation under the lock; drop-oldest-before-push verified; poison path untested (F-003); returns `false` on drop — return value ignored by coreaudio.rs:585 (fine, counters are the observability). |
| `drain` | 113-119 | REAL, EXECUTED | S4 | Oldest-first via VecDeque drain; poison → empty Vec forever (F-003). |
| `len` | 122-128 | REAL, EXECUTED | S4 | Poison → 0 while events may be buffered (F-003). |
| `is_empty` | 131-134 | REAL | — | Delegates to len. |
| `capacity` | 137-140 | REAL, EXECUTED | — | Pure accessor. |
| `dropped` | 144-146 | REAL, EXECUTED | — | Relaxed load; monotonic by construction. |
| `pushed` | 149-152 | REAL, EXECUTED | — | Relaxed load; incremented pre-lock so a concurrent reader can transiently see `pushed > len + dropped` (benign stats race — no doc claims a continuous invariant; tests read only quiescently). |

## 5. Test honesty (the 4 queue tests live in tests/seam_behaviour.rs)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `event_queue_drains_fifo_and_empties` (192) | YES | Exact 5-event order + empties. Mutation check: swap push_back/push_front order → order assert fails. EXECUTED green (orchrun 3691). |
| `event_queue_is_bounded_and_counts_drops` (225) | YES | len/dropped counts pinned at 3/3, `pushed` monotonic per round, `dropped+len==3` after drain. Mutation: stop dropping → len 6 fails. EXECUTED green. |
| `event_queue_capacity_is_clamped_to_one` (240) | YES | `capacity()==1`, second push returns false, dropped==1. Mutation: remove clamp → panic or capacity 0. EXECUTED green. |
| `event_queue_is_thread_safe_for_listener_threads` (253) | YES | 4 threads × 100 pushes, joins, exact accounting 400 = dropped+len, bound ≤64 holds under concurrency. Mutation: remove Mutex → data race/len>64 (or UB). EXECUTED green. |
| Not tested: poison paths (F-003), warm-up allocation (F-002, unobservable from this side of the API), negative capacity (usize — impossible). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "callbacks arrive ... (the PulseAudio event loop here)" | module doc 4-5 | FALSE (for this crate) | F-001: no PulseAudio in platform-macos; producer is coreaudio.rs listener. |
| "it knows nothing about PulseAudio or DRM" | module doc 13-14 | FALSE framing | True in spirit (queue is OS-agnostic) but names the Linux lane's concerns only. |
| "hard capacity: when full, the oldest event is dropped and a monotonic dropped counter increments (D-3)" | module doc 6-10 | REAL | Code 97-102; executed test pins it. |
| "a zero capacity is clamped up, never a silent black hole" | with_capacity doc 70-71 | REAL | `capacity.max(1)` (76); executed test. |
| "no allocation — push from an OS callback thread is safe" | push comment 94-95 | FALSE during warm-up | F-002. |
| "Consumers drain the whole batch — the natural shape for a UI tick" | module doc 10-11 | REAL-UNPROVEN | No consumer exists anywhere in the repo (crate is a leaf); shape is plausible but unexercised. |

## 7. Dead / duplicated / unreachable code
- Whole module unreachable from shipped entry points (crate leaf —
  see lib.rs audit Field row).
- Triple-duplicated file: events.rs is byte-identical across
  platform-macos/linux/windows except the card id (verified by diff).
  Unlike the seam modules (which must differ per OS), this file has no
  OS-specific code at all — it is the textbook candidate for the
  Lead's proposed shared L5 crate (lib.rs:47-50 [PROPOSAL]); until
  then, three copies must be kept in sync by hand, and the doc has
  already drifted (F-001).
- `AudioDeviceAdded`/`AudioDeviceRemoved`/`AudioDeviceStateChanged`
  are never constructed by macOS code (only AudioDefaultChanged /
  AudioSetChanged, coreaudio.rs:573-585; StateChanged is Windows-only,
  wasapi.rs:481). Not flagged dead: they are the pinned cross-lane
  vocabulary (mirrored deliberately so lanes cannot drift — the
  tests' stated purpose), and a future CoreAudio itemized diff would
  use Added/Removed. Recorded as lane-scope vocabulary, not dead code.

## 8. Dependencies & flags
- Deps: `audio_output::DeviceId` only (plus std). No features, no
  unsafe, no cfg — the file compiles identically on every OS (part of
  why triple-copy drift went unnoticed: nothing per-lane anchors the
  doc).
- Consistent with boundaries.toml (no external deps).

## 9. Open questions for the next agent
- The transient `pushed > len + dropped` read window (pushed
  increments pre-lock, table note on `pushed`): if any future consumer
  asserts the invariant continuously (e.g. a UI gauge), it will flap
  — the fix is incrementing pushed inside the lock or documenting the
  quiescent-only guarantee. No current consumer, so no finding.
- Whoever audits platform-linux/src/events.rs and
  platform-windows/src/events.rs: F-001 (doc drift) and F-002/F-003
  apply verbatim to your copies — cross-reference, don't re-derive.
  For the Windows lane the "PulseAudio event loop here" sentence is
  wrong in a third way (its producer is the WASAPI COM callback
  thread).
