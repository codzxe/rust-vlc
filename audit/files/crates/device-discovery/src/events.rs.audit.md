# AUDIT: crates/device-discovery/src/events.rs
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-21 |
| Lines / functions | 306 / 16 (9 prod items + diff_volumes + 6 tests + 1 helper, see §4) |
| File verdict | UNREACHABLE (correct, tested, bounded queue — but the crate is linked into no shipped binary at all) |
| Reachable from shipped entry point? | **NO — the crate is not even linked.** `rg "device.discovery" --glob Cargo.toml apps/ crates/` → only *comment text* in the three `platform-*` manifests ("the manifest is an allowlist, those edges join when their cards land"); every platform crate's actual `[dependencies]` is `audio-output` + `media-types` only (verified platform-linux/-windows/-macos). No app depends on it. `device-discovery` builds as a bare workspace member (baseline build log :14) and its rlib is linked into nothing. The `VolumeEventQueue`'s only consumers are `monitor.rs` (same crate, itself unstarted by anyone — `rg "VolumeMonitor" apps/ crates/` → device-discovery's own files + its test) and `tests/volume_monitor.rs`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read (whole file); `rg -l "device-discovery" --glob Cargo.toml .` + `rg -n "^\[dependencies\]" -A 8 crates/platform-{linux,windows,macos}/Cargo.toml` (no real dep edges); `rg -n "VolumeEventQueue\|diff_volumes\|VolumeEvent" crates/ apps/ -g '!events.rs'` (consumers: same crate + test only); `rg -rn "VolumeMonitor" apps/ crates/ -l` (no host starts the monitor); `rg -n "events::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (6/6 ok :2386-2391); `sed -n '2590,2606p' docs/feature-matrix.toml` (sd-directory-watch row). |

## 1. One-paragraph verdict
A small, disciplined module: a `Mutex<VecDeque>` queue with a hard capacity, oldest-drop + monotonic `dropped`/`pushed` counters, zero-capacity clamped to 1 ("never a silent black hole"), poisoned-lock behavior documented on `push`, and a pure `diff_volumes` that expresses changes as removed-then-added pairs with correct ordering — all pinned by 6/6 green tests in the baseline run. Two blemishes: the push comment claims "no allocation" while `VecDeque::push_back` amortized-allocates during ring growth (F-002), and — the wave's verdict — the entire crate is dead weight in the product: no shipped binary links it, no host starts the monitor, and the lib-doc's promised relay into `devices.volume-added`/`-removed` catalog rows exists nowhere (F-001). The matrix row for this lane (`sd-directory-watch`) honestly says not_started — under-claiming finished library work, the mirror image of network-dash's over-claim.

## 2. What actually works (verified)
— `events::tests` 6/6 `ok` (`queue_bounds_and_counts_drop_oldest` — asserts drop order, counters, and the `false` return; `zero_capacity_is_clamped_not_a_black_hole`; `diff_reports_adds_and_removals`; `diff_expresses_changes_as_removed_then_added`; `diff_of_identical_snapshots_is_empty`; `volume_id_accessor_covers_both_variants`) — `orchrun-test-workspace-nofailfast.log.txt:2386-2391`.
— Bounded-memory claim verified by reading: the queue can never exceed `capacity` entries (drop-before-push at 106-111); counters are `AtomicU64` with `Relaxed` (stats, no ordering requirement); critical sections hold no callbacks and perform no unbounded work.
— `diff_volumes` semantics hand-checked: before=[a,b], after=[b,c] → [Removed(a), Added(c)] (test-pinned); changed label/kind/root → Removed (before order) then Added (after order); identical snapshots → empty.

## 3. Findings

### F-001 [S3]
Where: crates/device-discovery/src/events.rs:1-11 (module) with crates/device-discovery/src/lib.rs:10-12
Code:
```
//! The bounded volume-event queue: where mount-table diffs land (card
//! 7.D1; the event-plumbing half of the volume lane).
```
```
//! [`events::VolumeEvent`]s — the source the engine host relays into
//! the catalog's `devices.volume-added` / `devices.volume-removed`
//! rows (the relay itself is host wiring; the catalog emission ledger
//! records the split).
```
What is wrong: The promised consumer — "the engine host relays into the catalog's rows" — does not exist. No crate declares a dependency on `device-discovery` (all `platform-*` manifests carry only `audio-output` + `media-types`; their comments say the device-discovery edge "joins when their cards land"), no app links it, and `VolumeMonitor` has no caller outside the crate's own tests. The crate compiles in the workspace build and then reaches zero shipped binaries; everything in this file is dead weight in the product.
Why it matters: A user plugging a USB stick gets no `devices.volume-added` event anywhere — the capability this file exists for is unreachable, and the crate's build/test cost is pure overhead in every `cargo build --workspace`. Anyone reading lib.rs's relay story believes host wiring exists ("the relay itself is host wiring" — there is no host wiring).
What correct looks like: Either the platform adapters take the dependency and a host starts the monitor with the relay (the crate's own stated plan), or the crate is marked workspace-excluded/`publish = false`-with-a-comment until its card lands — not silently compiled and silently unused.
Confidence: HIGH — Verification: rg over every Cargo.toml's real `[dependencies]` + monitor caller map (header).
### F-002 [S4]
Where: crates/device-discovery/src/events.rs:102-111
Code:
```
        self.pushed.fetch_add(1, Ordering::Relaxed);
        // Lock discipline: short critical section, no callbacks, no
        // allocation — push from the watcher thread is safe.
        if let Ok(mut queue) = self.inner.lock() {
            let dropped_one = queue.len() >= self.capacity;
            if dropped_one {
                queue.pop_front();
                self.dropped.fetch_add(1, Ordering::Relaxed);
            }
            queue.push_back(event);
```
What is wrong: The comment says "no allocation", but `queue.push_back(event)` allocates whenever the `VecDeque`'s ring buffer must grow — which happens repeatedly until the deque reaches `capacity` (128 by default) and its head-drop equilibrium. The true and intended property is *bounded* allocation (never beyond capacity+ring margin), not *no* allocation.
Why it matters: The comment is a latency claim ("push from the watcher thread is safe" — i.e., non-blocking-ish); a reader auditing the watcher thread's allocation behavior is misled on the growth path. Wrong invariant in a comment is how the real invariant later gets "fixed" away.
What correct looks like: Comment says "bounded allocation: the ring grows to the capacity bound and then stops; no per-push allocation in steady state", or construct the deque with `VecDeque::with_capacity(capacity)` in `with_capacity` to make the comment literally true.
Confidence: HIGH — Verification: read (VecDeque growth semantics; no `with_capacity` in the constructor at 83-88).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-17 | REAL | — | Bounds + counters + drain shape; "tests drive the same push path" — true (verified in §5). |
| `VolumeEvent` (enum) | 30-43 | REAL | — | Added{volume}/Removed{id}; catalog-shaped per doc 05 §6.2 (wire mapping lives in volume.rs, cross-ref). |
| `volume_id` / `is_added` / `is_removed` | 45-66 | REAL | — | Pure accessors; both variants covered by test. |
| `VolumeEventQueue` (struct) | 70-76 | REAL | — | Mutex + VecDeque + capacity + two atomics. |
| `with_capacity` | 81-89 | REAL | — | `capacity.max(1)` clamp documented; returns Arc (shared shape). Note: no `VecDeque::with_capacity` (F-002). |
| `new` | 93-96 | REAL | — | 128. |
| `push` | 101-119 | REAL | S4 (F-002) | Drop-oldest + counters + `false` return; poisoned path documented and counted. `pushed` counts attempts (a poisoned push counts as both pushed and dropped — "events ever pushed" is a slight overstatement, noted not filed). |
| `drain` | 122-127 | REAL | — | Whole-batch take, oldest first; poisoned → empty Vec *silently* (push documents its poison behavior, drain does not — noted not filed: only reachable after a consumer panicked mid-drain). |
| `len` / `is_empty` / `capacity` / `dropped` / `pushed` | 129-160 | REAL | — | Pure observation; poisoned → 0. |
| `diff_volumes` | 178-205 | REAL | — | O(n·m) nested `find` — fine at mount-table scale (tens); change = label/kind/root (id is the key); removed-first ordering documented and test-pinned; duplicate ids in one snapshot resolve to first match (tolerance, harmless). |
| tests (6 + `volume` helper) | 207-306 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `queue_bounds_and_counts_drop_oldest` | YES | Capacity-2 exact sequence: drop order ("a" dropped, "b" first in drain), `dropped()==1`, `pushed()==3`, the `false` return, emptiness after drain — five distinct assertions on one scenario. |
| `zero_capacity_is_clamped_not_a_black_hole` | YES | Pins the max(1) clamp — a black-hole queue fails `len()==1`. |
| `diff_reports_adds_and_removals` | YES | Exact event vector for add+remove. |
| `diff_expresses_changes_as_removed_then_added` | YES | Order + payload of the change pair (label AND kind change). |
| `diff_of_identical_snapshots_is_empty` | YES | No-spurious-events. |
| `volume_id_accessor_covers_both_variants` | YES | Accessor total. |
Execution: 6/6 `ok` — orchrun-test-workspace-nofailfast.log.txt:2386-2391.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the source the engine host relays into the catalog's devices.volume-added/-removed rows" | lib.rs:10-12 | **PAPER-ONLY** (F-001) | No host wiring exists; crate linked into nothing (header rg). |
| "a mount storm can never grow the process" | events.rs:7-10 | REAL | Drop-before-push bound read + `queue_bounds_and_counts_drop_oldest` green. |
| "no callbacks, no allocation" (push) | events.rs:103-104 | SHALLOW | No callbacks ✓; allocation is bounded, not absent (F-002). |
| sd-directory-watch = "not_started", evidence "" | feature-matrix.toml:2601-2602 | Under-claim (product-accurate) | Library implemented + 6/6 green, but nothing links it — the row is honest about the product and hides the library, the mirror of network-dash's over-claim (see my remote_playlist.rs F-002 for the bookkeeping contradiction). |

## 7. Dead / duplicated / unreachable code
- The entire crate is unreachable from shipped entry points (F-001) — compiled by `--workspace`, linked by nothing.
- Deliberate near-duplication of the wave-22 platform `DeviceEventQueue` (documented at :4-6 as intentional for a future seam unification) — accepted, cross-referenced.

## 8. Dependencies & flags
- std-only (VecDeque/Mutex/atomics/Arc) + `crate::volume::{Volume, VolumeId}`. No feature gates, no unsafe, no unwrap/expect (poison handled explicitly). Crate builds standalone (no dependents), so nothing can flag-gate it out either.

## 9. Open questions for the next agent
- W10-A06 (`volume.rs`) / W10-A03 (`monitor.rs` — wait, monitor.rs is W10-A06? re-check: monitor.rs is assigned to a06): does the monitor's poll loop ever push more than `capacity` events per tick, making the drop-oldest path reachable in practice (my F-002 note about ring growth interacts with steady-state)?
- Whoever audits the platform adapters (wave for 7.A2): the manifests' comments promise the device-discovery edge "joins when their cards land" — verify whether any later-wave manifest actually adds it; if not, this crate is permanently dead weight and F-001 should be escalated.
- Matrix bookkeeping: sd-directory-watch (not_started) vs network-dash (partial) — one standard needed; flag to the wave summary.
