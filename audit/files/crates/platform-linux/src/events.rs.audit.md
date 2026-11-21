# AUDIT: crates/platform-linux/src/events.rs (whole file, 153 lines)

| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-21 |
| Lines / functions | 153 / 11 units (1 enum, 1 struct, 9 methods) |
| File verdict | UNREACHABLE (real, correct, executed-by-tests bounded queue; the crate has zero consumers — no shipped entry point constructs it) |
| Reachable from shipped entry point? | NO — crate-wide fact established in my lib.rs.audit.md header (`rg -l "platform_linux"` outside the crate → only a docs gate log and the baseline test log; no Cargo.toml in apps/ or crates/ declares the dependency). In-crate the queue IS wired: `AudioDevices::subscribe(Arc<DeviceEventQueue>)` (audio.rs:46) is implemented by the real PulseAudio FFI watch (pulse.rs:623-723, callback `on_subscription_event` at pulse.rs:231-264 pushes `PlatformEvent`s) and by the typed fallback (audio.rs:87-89, refuses). Queue mechanics executed: `orchrun-test-workspace-nofailfast.log.txt:3663-3673` — `event_queue_capacity_is_clamped_to_one`, `event_queue_drains_fifo_and_empties`, `event_queue_is_bounded_and_counts_drops`, `event_queue_is_thread_safe_for_callback_threads` all `ok` (platform_linux seam_behaviour target). The FFI production push path (live PulseAudio server) is REAL-UNPROVEN: the sandbox has no server — `pulse_subscribe_without_a_server_is_the_typed_error` (linux_integrations.rs:421-445) exercises only the ServiceUnavailable arm; the Ok arm carries a `[VERIFY]` marker. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | Read tool over whole file (verbatim quotes); `rg -n "DeviceEventQueue\|PlatformEvent" crates/platform-linux/src/` (wiring: audio.rs trait + fallback, pulse.rs producer — 4 push sites: AudioSetChanged ×2, AudioDeviceAdded, AudioDeviceRemoved; **no Linux producer for `AudioDeviceStateChanged`/`AudioDefaultChanged`**); `sed -n '600,760p' + '100,130p' + '225,268p' pulse.rs` (subscribe/watch/translation traced); `rg -n "DeviceEventQueue\|drain\|dropped" tests/seam_behaviour.rs tests/linux_integrations.rs` + sed of test bodies (4 queue tests + fallback test); `rg -n "7\.A2" docs/` (resolves: docs/13-agent-wave-program.md:705); `rg -n "D-3\|bounded" docs/engineering-directives.md` + sed (D-1 vs D-3 definitions); `rg -n "AudioDeviceStateChanged\|AudioDefaultChanged" crates/platform-windows/src/ crates/platform-macos/src/` (siblings DO produce both variants: wasapi.rs:481/534, coreaudio.rs:573 — vocabulary is per-crate duplicated, not shared); `rg -n "event_queue" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (execution proof) |

## 1. One-paragraph verdict
A small, disciplined, honest data structure: the bounded drop-oldest queue does exactly what its module docs promise, every claim I could pin is pinned by a test that runs in the no-fail-fast baseline (FIFO order, drop accounting `dropped + len == pushed` under 4-thread concurrency, capacity clamp, bound hold), and the producer half genuinely exists in the crate (PulseAudio FFI subscription → `on_subscription_event` → `queue.push`). The defects are at the edges: (1) a poisoned mutex turns the lane into a permanently silent void that `drain`/`len`/`is_empty` report as a healthy empty queue with no error surface and no recovery — the "honest, non-crashing choice" comment is honest only for `push`'s return value; (2) the `dropped`/`pushed` counters conflate bound-evictions with poison-refusals against their own doc contracts; (3) `with_capacity` never pre-allocates the deque, so the push lock-discipline comment's "no allocation" claim is false during ring growth inside the critical section; (4) the header cites "engineering directive D-3: bounded memory everywhere" — the bounded-memory directive is **D-1** (engineering-directives.md:26-31); D-3 is dirty-world ingestion (line 51); (5) two of the five `PlatformEvent` variants (`AudioDeviceStateChanged`, `AudioDefaultChanged`) have zero Linux producers — they are the Windows/macOS halves of a per-crate-duplicated vocabulary, dead code in this crate by mirroring design.

## 2. What actually works (verified)
— Bounded drop-oldest semantics — verified: push (92-110) evicts `pop_front` only when `len >= capacity`, always admits the newest, returns `false` on eviction; `event_queue_is_bounded_and_counts_drops` pins `dropped()==3 / pushed()==6 / len()==3` for 6 pushes into capacity 3 — `ok` at baseline log line 3669.
— Capacity clamp — verified: `capacity.max(1)` (76) + `event_queue_capacity_is_clamped_to_one` asserts capacity 1 and second-push-false — `ok` at 3666.
— Concurrency accounting invariant — verified: `event_queue_is_thread_safe_for_callback_threads` (seam_behaviour.rs:311-340) pushes 4×100 from spawned threads and asserts `dropped() + len() == 400` and `len() <= 64` — `ok` at 3673. The invariant is designed to survive the poison branch (both counters move, len does not).
— FIFO drain — verified: `event_queue_drains_fifo_and_empties` asserts the exact 5-element order and post-drain emptiness — `ok` at 3668.
— Real in-crate producer — verified by read: pulse.rs `subscribe` (623-723) primes an index→id map, registers `on_subscription_event`, waits the bounded ack loop, keeps the `Watch` alive with a documented Drop teardown order; the callback (pulse.rs:231-264) translates facilities to exactly 3 of the 5 variants. Live-server arm REAL-UNPROVEN (no server on sandbox; `[VERIFY]` in the integration test).
— ARCH-6 honored — verified: the only import is `audio_output::DeviceId` (23); no OS types in the public surface.

## 3. Findings

### F-001 [S3]
Where: crates/platform-linux/src/events.rs:104-109 (with 113-119, 123-128)
Code:
```rust
        } else {
            // A poisoned queue means a consumer panicked mid-drain;
            // dropping the event is the honest, non-crashing choice.
            self.dropped.fetch_add(1, Ordering::Relaxed);
            false
        }
```
What is wrong: The poison branch is honest for `push` (returns `false`), but every consumer-side accessor then masks the broken state as health: `drain()` (113-119) returns an empty `Vec`, `len()` (123-128) returns `0`, `is_empty()` returns `true` — and a mutex stays poisoned forever, so the lane is permanently dead while polling consumers read "no events". No accessor returns an error, sets a flag, or clears the poison; only `push` increments a counter, and a consumer that only drains can never detect the death at all.
Why it matters: A single panic mid-drain silently kills the entire device-notification lane with zero observability — the exact "silent lie" shape the crate's own docs call a fabrication everywhere else. Mitigating: poisoning requires a panic inside the two tiny critical sections (`pop_front`/`push_back` of a `Copy` enum cannot panic; `drain(..).collect()` can only abort on allocation, not unwind), so the path is near-unreachable in practice, and the crate itself is unreachable from shipped binaries — hence S3, not S2.
What correct looks like: Either (a) recover: on poison, `into_inner()`-style takeover or replace the mutex contents (a bounded notification queue is cheap to rebuild) and expose a `is_broken()`/poison counter; or (b) at minimum make `drain` return the poison fact (e.g. `Result<Vec<PlatformEvent>, Poisoned>` or an event-shaped `PlatformEvent::LaneBroken`) instead of a convincing empty batch.
Confidence: HIGH — Verification: read (all three consumer accessors traced; no other poison-awareness exists in the file).

### F-002 [S4]
Where: crates/platform-linux/src/events.rs:142-152
Code:
```rust
    /// Monotonic count of events dropped by the bound (oldest-first).
    #[must_use = "pure accessor"]
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Monotonic count of events ever pushed.
    #[must_use = "pure accessor"]
    pub fn pushed(&self) -> u64 {
        self.pushed.load(Ordering::Relaxed)
    }
```
What is wrong: Both counters conflate two different fates. `dropped` is documented as "dropped by the bound (oldest-first)" but the poison branch (107) also increments it — those events were refused by a broken mutex, not evicted by the capacity bound. `pushed` is documented as "events ever pushed" but counts poison-refused events that never entered the queue (93 increments before the lock is taken). A diagnostician reading `dropped` after a poison episode will misattribute the loss to burst pressure.
Why it matters: These counters are the queue's only observability (D-1's "explicit rejection" accounting); muddling their meaning under the rare-but-real poison path corrupts exactly the evidence needed to diagnose that path.
What correct looks like: Either separate counters (`dropped_by_bound`, `refused_poisoned`) or docs that say "dropped by the bound OR refused on a poisoned queue"; likewise `pushed` → "push attempts" or move the increment into the success arm.
Confidence: HIGH — Verification: read (increment sites at 93, 100, 107 vs both doc comments).

### F-003 [S4]
Where: crates/platform-linux/src/events.rs:73-79 (with 94-95)
Code:
```rust
    pub fn with_capacity(capacity: usize) -> Arc<DeviceEventQueue> {
        Arc::new(DeviceEventQueue {
            inner: Mutex::new(VecDeque::new()),
            capacity: capacity.max(1),
```
and the claim it contradicts:
```rust
            // Lock discipline: short critical section, no callbacks, no
            // allocation — push from an OS callback thread is safe.
```
What is wrong: `with_capacity` stores the bound but allocates a zero-capacity `VecDeque::new()`; the deque's ring grows lazily inside `push`'s critical section (first pushes allocate, then doubling reallocations until the ring can hold `capacity` items — for the default 128, several mallocs under the lock during the first hotplug burst). The "no allocation" line of the lock-discipline comment is therefore false during growth. The obvious pre-allocation (`VecDeque::with_capacity(capacity.max(1))`) — the very thing the constructor's name promises — was skipped.
Why it matters: The comment is a load-bearing concurrency-safety rationale ("push from an OS callback thread is safe" justified by, inter alia, "no allocation"); a false premise in a safety argument rots trust even where the conclusion survives (allocation on a normal OS callback thread is legal, merely latency). Blast radius today: none (crate unreachable; allocations are finite and stop after warm-up).
What correct looks like: `inner: Mutex::new(VecDeque::with_capacity(capacity.max(1)))` and re-word the comment to "no allocation after warm-up (the ring is pre-sized at construction)".
Confidence: HIGH — Verification: read (std `VecDeque::new()` is documented zero-capacity; growth path of `push_back` standard).

### F-004 [S4]
Where: crates/platform-linux/src/events.rs:6-10
Code:
```rust
//! the queue is `Mutex` + `VecDeque` with a **hard capacity**: when
//! full, the oldest event is dropped and a monotonic
//! `dropped` counter increments (engineering directive D-3: bounded
//! memory everywhere; a notification flood can never grow the
//! process). Consumers `drain` the whole batch — the natural shape
```
What is wrong: The directive citation is wrong. In `docs/engineering-directives.md`, the bounded-memory mandate ("Strict memory boundedness. Every queue, buffer, cache, and collection MUST have a hard, unbreachable maximum capacity") is **D-1** (lines 26-31, under "D-1. Memory & Allocation Discipline"); **D-3** is "Defensive Ingestion & The 'Dirty World' Principle" (line 51) — parsing robustness, nothing to do with queues. The same misnumber appears in audio.rs:39 ("the queue owns its memory bound, D-3").
Why it matters: A citation that points at the wrong directive is noise in an audit trail built on directive references; a reader following it lands on dirty-world parsing.
What correct looks like: "engineering directive D-1 (strict memory boundedness)". Same fix in audio.rs:39.
Confidence: HIGH — Verification: read (both directive definitions sed-verified in docs/engineering-directives.md:14-35 and 51+).

### F-005 [S4]
Where: crates/platform-linux/src/events.rs:42-56
Code:
```rust
    AudioDeviceStateChanged {
        /// The device's stable id.
        id: DeviceId,
        /// `true` when the device became active.
        active: bool,
    },
    /// The default audio device changed (`None` = none selected).
    AudioDefaultChanged {
        /// The new default, if the OS reported one.
        id: Option<DeviceId>,
    },
```
What is wrong: In this crate, `AudioDeviceStateChanged` and `AudioDefaultChanged` have zero producers. The only push sites are pulse.rs:246/261 (`AudioSetChanged` ×2), 253 (`AudioDeviceAdded`), 256 (`AudioDeviceRemoved`); a default-device change arrives as facility==SERVER and is deliberately degraded to `AudioSetChanged` ("rather than guessing a translation", pulse.rs:243-248). The variants' own docs describe Windows (`DEVICE_STATE_NOTPRESENT↔ACTIVE`) and CoreAudio semantics, and the sibling crates that DO produce them carry duplicated copies of this entire enum (platform-windows/src/events.rs:42/49, produced at wasapi.rs:481/534; platform-macos/src/events.rs:42/49, produced at coreaudio.rs:573). Bonus asymmetry: even `AudioDeviceAdded` is near-dead on Linux — a genuinely new sink's index is absent from the primed map, so it surfaces as `AudioSetChanged`; the `Some(id)` arm fires only on stale index reuse, where it would emit the *removed* device's id.
Why it matters: Dead public vocabulary in an enum whose doc says "already translated into engine vocabulary" overstates what the Linux integration can say; the mirroring is a documented `[PROPOSAL]` debt (lib.rs:44-47), so this is accepted-by-design duplication, flagged for the unification decision, not condemned.
What correct looks like: Either cfg-gate the Linux enum down to the three produced variants, or land the proposed shared `platform-events` vocabulary crate before the three copies drift.
Confidence: HIGH — Verification: read + grep (all push sites enumerated; sibling producers verified).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `PlatformEvent` (enum, 5 variants) | 27-57 | REAL (3 variants) / dead vocab (2) | S4 (F-005) | `Copy`+`Eq` — value semantics right for a queue; Linux produces 3/5; `AudioDeviceAdded` itself only via stale-index reuse (F-005 note). |
| `DeviceEventQueue` (struct) | 61-67 | REAL | — | Mutex + bound + two atomics; `#[derive(Debug)]` fine (all fields Debug). |
| `with_capacity` | 72-80 | REAL | S4 (F-003) | Clamps to ≥1 (test-pinned); fails to pre-size the deque the name advertises; returns `Arc` directly — forces the shared shape, deliberate. |
| `new` | 84-87 | REAL | — | 128 default; doc "one hotplug storm's worth" is judgment, not test-pinned — acceptable. |
| `push` | 92-110 | REAL | S3 (F-001), S4 (F-002/003) | Drop-oldest, always admits newest; `Relaxed` atomics sound (mutex serializes contents; counters are independent diagnostics); poison branch increments both counters and returns false — invariant-preserving but conflating. No panic paths (Copy enum ops). |
| `drain` | 113-119 | REAL | S3 (F-001) | Allocates the result Vec under the lock (no no-alloc claim made for drain — fine); poison → convincing empty batch. |
| `len` | 122-128 | REAL | S3 (F-001) | Poison → 0, indistinguishable from empty. |
| `is_empty` | 131-134 | REAL | — | Delegates to len; inherits the poison mask. |
| `capacity` | 137-140 | REAL | — | Pure accessor. |
| `dropped` | 143-146 | REAL | S4 (F-002) | Doc vs counter conflation. |
| `pushed` | 148-152 | REAL | S4 (F-002) | Counts attempts, docs say events. |

## 5. Test honesty (queue tests live in tests/seam_behaviour.rs, one mirrored suite per platform crate)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `event_queue_drains_fifo_and_empties` (241) | YES | Exact 5-element order vector equality + post-drain empty. Mutation check: swap push_back→push_front fails the order assert; a drain that doesn't clear fails the second drain. Baseline ok (log 3668). |
| `event_queue_is_bounded_and_counts_drops` (274) | YES | Pins pushed/dropped/len at 6-into-3 and after drain. Mutation: remove pop_front → dropped()==0 fails; off-by-one in the >= comparison flips the count. Baseline ok (3669). |
| `event_queue_capacity_is_clamped_to_one` (296) | YES | capacity()==1 and second-push-false pinned. Mutation: drop the `.max(1)` → capacity()==0, and push would evict the only event → len assert fails. Baseline ok (3666). |
| `event_queue_is_thread_safe_for_callback_threads` (311) | YES | 4 threads × 100 pushes; asserts pushed()==400, len()<=64, and the conservation law dropped()+len()==400 — catches lost/double-counted accounting and bound breaches under real contention. Baseline ok (3673). |
| `unavailable_audio_devices_returns_the_typed_reason` (21) | YES | Fallback's `subscribe(queue)` must return the exact typed reason — a fake success fails. Baseline ok. |
| Coverage holes | — | The poison path (F-001/F-002) is untested and untestable through the public API without a deliberate panic helper; no test covers a live-server push (REAL-UNPROVEN, `[VERIFY]` in linux_integrations.rs:435); the mirrored per-crate test triples (baseline 3663/3690/3718) run identical suites — drift between the three enum copies would surface only as compile errors, not test failures. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "card 7.A2; the event-plumbing half of the notification lane" | events.rs:1-2 | REAL | docs/13-agent-wave-program.md:705 defines 7.A2 as platform-linux device enum + notifications; this file is that half. |
| "when full, the oldest event is dropped and a monotonic dropped counter increments (… a notification flood can never grow the process)" | events.rs:5-10 | REAL (mechanics) / FALSE (citation) | Mechanics test-pinned (§5); the "(engineering directive D-3: bounded memory everywhere)" citation points at the wrong directive — boundedness is D-1 (F-004). |
| "Consumers drain the whole batch — the natural shape for a UI tick or an engine poll" | events.rs:10-11 | REAL | `drain()` API; no in-crate consumer exists (crate unreachable) — shape claim only. |
| "The queue is deliberately dumb: it knows nothing about PulseAudio or DRM" | events.rs:13-14 | REAL | Only import is `audio_output::DeviceId` (line 23). |
| "The OS integration translates native callbacks into PlatformEvents and pushes; tests drive the same push path" | events.rs:14-17 | REAL (3/5 variants) / REAL-UNPROVEN (live server) | pulse.rs:231-264 translation traced; live-server arm unexecuted on sandbox (no server). Two variants never translated on Linux (F-005). |
| "minimum 1 — a zero capacity is clamped up, never a silent black hole" | events.rs:70-71 | REAL | Clamp test, baseline ok. |
| "Returns false when the queue was full and the oldest event was dropped to make room (… the dropped counter has already moved)" | events.rs:89-91 | REAL | Bounded-drops test pins the return and counter. |
| "A poisoned queue means a consumer panicked mid-drain; dropping the event is the honest, non-crashing choice" | events.rs:105-106 | SHALLOW | Honest for push's return value; drain/len/is_empty then mask the death as emptiness (F-001) — "honest" overstates. |
| "Lock discipline: short critical section, no callbacks, no allocation" | events.rs:94-95 | FALSE ("no allocation") | Ring growth allocates under the lock until warmed (F-003). |
| audio.rs:39 "bounded — the queue owns its memory bound, D-3" | audio.rs:38-40 | FALSE (citation) | Same D-1/D-3 misnumber (F-004). |

## 7. Dead / duplicated / unreachable code
- Crate-wide unreachable (header proof) — every unit here inherits it.
- In-crate dead vocabulary: `AudioDeviceStateChanged`, `AudioDefaultChanged` (F-005) — produced only by the duplicated sibling enums in platform-windows/macos.
- The entire `PlatformEvent` enum is triplicated across the three platform crates (documented `[PROPOSAL]` mirroring, lib.rs:44-47) — accepted debt, flagged for the unification gate.
- Poison branch (104-109): near-unreachable by construction (no panicking op inside either critical section) — dead-in-practice code whose masking behavior is F-001.

## 8. Dependencies & flags
Imports: std only (`VecDeque`, atomics, `Arc`/`Mutex`) + `audio_output::DeviceId` (engine vocabulary — the one cross-crate edge, honoring ARCH-6). No unsafe, no feature flags, no cfg gates (the enum's Windows-flavored variants are not cfg'd out on Linux — F-005). `[lints] workspace = true` inherited. `Ordering::Relaxed` on all four atomic ops is correct for independent monotonic diagnostics; queue-content consistency rides the mutex.

## 9. Open questions for the next agent
1. pulse.rs owner (this wave): the primed `index_ids` map is never refreshed after subscribe — confirm whether a sink added then removed then re-added (index reuse) can push `AudioDeviceAdded` carrying the *previous* device's id (pulse.rs:251-253 with the stale map), and whether a changed sink's TYPE_CHANGE deliberately degrades to `AudioSetChanged` (246-261 says yes). My read says yes to both; it is your file's finding to own.
2. Consumer-side masking (F-001) becomes S2 the day any host composes this crate and drains on a UI tick — the composition owner should demand a poison surface before wiring.
3. The `[PROPOSAL]` vocabulary unification (three PlatformEvent copies) should be sequenced before the copies drift; the Windows copy already produces variants the Linux copy cannot.
4. Directive-citation sweep: D-3-for-boundedness misnumbering exists in at least events.rs:8 and audio.rs:39 — grep the crate (and platform-windows/macos, which mirror the comments) for "D-3" and re-point to D-1.
