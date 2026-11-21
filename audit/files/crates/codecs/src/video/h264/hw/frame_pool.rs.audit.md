# AUDIT: crates/codecs/src/video/h264/hw/frame_pool.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-21 |
| Lines / functions | 367 / 13 production items + 5 tests |
| File verdict | UNREACHABLE (real, well-tested bounded pool; only consumers are the two OS hw lanes, both policy-off and caller-less in shipped builds) |
| Reachable from shipped entry point? | NO. Consumers: `hw/vaapi/session.rs:372,693,930` and `hw/videotoolbox/session.rs:480,515,755` — both lanes compile per-OS but ship unreachable (product.toml:169 `hw-decode = false`, `hw_decode_policy = "software-only"`; `H264Decoder::from_setting` has zero production callers — see my vaapi/session.rs#part4 audit §Reachability). Tests are host-runnable (pure Rust) and executed in baseline. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "HwSurfacePool\|frame_pool\|MAX_SURFACES" --type rust crates/` (consumer census: 2 hw sessions only); `rg -n "frame_pool::tests" audit/runs/baseline/test-workspace.log.txt` (5/5 `ok`, 563-567); `sed -n '925,932p' vaapi/session.rs` + `sed -n '752,757p' videotoolbox/session.rs` (pool-replacement pattern, F-002); full-file read |

## 1. One-paragraph verdict
A small, disciplined generic surface-slot pool: bounded capacity with a hard 64-slot cap, typed exhaustion, generation-tagged handles that make stale ids no-ops, and lazy slot growth inside pre-reserved capacity. All five tests are content-strong (lifecycle, clamp, drain invalidation, garbage-handle robustness, 400-cycle steady-state recycling) and all passed in the baseline host run. The code is correct for every reachable input I could construct; the one internal defect is theoretical (generation wraparound after 2³² drains silently breaks handle minting). The serious issue is a consumer-side contract violation: both hw sessions REPLACE the pool at configure time (`self.pool = HwSurfacePool::new(…)`) instead of calling `drain()`, resetting the generation to 0 — which re-enables exactly the stale-handle aliasing the generation exists to prevent. The whole subsystem ships unreachable today, so the blast radius is deferred to whenever 7.B activates.

## 2. What actually works (verified)
— 5/5 tests `ok` in baseline (test-workspace.log.txt:563-567); each would fail on a concrete mutation (double-emit accepted → 278 fails; recycle broken → 291/365 fail; drain aliasing → 319-325 fail; cap ignored → 302/352 fail).
— Handle algebra verified by hand: `handle_of`/`split_handle` (236-249) round-trip (gen,idx); stale generation rejected at 244; index bound at 248; garbage handles (u64::MAX, 1<<32, small ints) all no-ops — pinned by the garbage test (329-336).
— `acquire` growth path (152-155) pushes only within `capacity`, and `new` pre-reserves (106), so no reallocation churn.

## 3. Findings
### F-001 [S4]
Where: crates/codecs/src/video/h264/hw/frame_pool.rs:236-238
Code:
```
    fn handle_of(&self, index: usize) -> u64 {
        (self.generation << 32) | u32::try_from(index).unwrap_or(u32::MAX) as u64
    }
```
What is wrong: `generation` is bumped with `wrapping_add` (229) but the handle packs it into the top 32 bits. After 2³² `drain()` calls, `generation << 32` silently truncates (value-overflow on `<<` is not checked in either profile), so `handle_of` mints handles whose high half no longer equals `self.generation` — `split_handle` (244) then rejects the pool's own fresh handles: acquire "succeeds" but `emit`/`release`/`resource` on the returned handle are permanent no-ops. Two pools in different wrap epochs could also mint identical handles.
Why it matters: The invariant "ids never alias" (234-235) fails at the wrap point. Practically unreachable (a drain per stream renegotiation; 4·10⁹ renegotiations per process), so S4 — but the failure is silent and would surface as an inexplicable "surface never frees" hang, not a crash.
What correct looks like: `generation` as u32 with `wrapping_add` and pack/unpack in u32 halves, refusing acquire once the generation wraps (or assert the shift is lossless).
Confidence: HIGH — Verification: read + shift-overflow semantics of Rust (`<<` value overflow never panics).
### F-002 [S3]
Where: crates/codecs/src/video/h264/hw/vaapi/session.rs:930 and crates/codecs/src/video/h264/hw/videotoolbox/session.rs:755 (consumers of this file's contract — primary ownership: vaapi session parts (W06-A06/A08/A09), videotoolbox session (W06-A02/A08); recorded here because the violation defeats this file's core invariant)
Code:
```
        self.surfaces = surfaces;
        self.pool = HwSurfacePool::new(count);
```
What is wrong: `HwSurfacePool` provides `drain()` precisely for renegotiation — "all handles minted so far become permanently invalid (the pool generation advances past them — ids never alias)" (223-226). Both sessions instead REPLACE the pool object at configure, and `new` resets `generation` to 0 (107). A handle emitted to the codec layer before a reconfigure (generation 0, index i) is byte-identical to a handle the new pool mints for slot i — `split_handle` accepts it. A late `release()` of the stale handle then frees a live slot of the new pool whose OS surface may still be referenced by the presenter — the silent-overwrite corruption the module doc promises can never happen (14-15).
Why it matters: The pool's headline safety property is voided by its only two consumers at their first SPS-change/reconfigure; today the lanes are unreachable (hw-decode off), so no shipped impact — but this is exactly the landmine 7.B activation would step on, and it contradicts the module's "bounded by construction / never a silent overwrite" contract.
What correct looks like: `self.pool.drain()` then rebuild capacity via a dedicated `resize`/re-`drain` path that preserves the generation, or make `HwSurfacePool::new` start at a monotonically sourced generation (e.g. a process-wide atomic).
Confidence: HIGH — Verification: read of both call sites + `new` (103-110) resetting generation to 0.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MAX_SURFACES` | 47 | REAL | — | 64; rationale comment (driver DPB sizes) plausible, UNVERIFIED. |
| `SlotState` | 51-59 | REAL | — | Three-state lifecycle, exhaustive matches downstream. |
| `PoolExhausted` + Display/Error | 63-81 | REAL | — | Typed, Copy, informative fields. |
| `HwSurfacePool::new` | 103-110 | REAL | F-002 (as the reset source) | Clamps to cap; pre-reserves; generation = 0. |
| `capacity` / `in_use` / `emitted` | 114-134 | REAL | — | O(n) over ≤64 slots — fine. |
| `acquire` | 142-162 | REAL | — | Free-slot-first recycle, lazy growth within capacity, typed exhaustion with emitted count. |
| `emit` | 170-181 | REAL | — | Decoding→Emitted only; stale/free/double-emit → false, no panic. |
| `release` | 186-198 | REAL | — | Frees any live slot (flush path documented); drops resource. |
| `holds_resource` | 205-212 | REAL | — | Live-slot busy predicate; `T: PartialEq` bound only here. |
| `resource` | 216-221 | REAL | — | Borrowed lookup, None on stale. |
| `drain` | 227-230 | REAL | F-001 (wrap) | Clears + bumps generation — the correct renegotiation primitive the sessions don't use (F-002). |
| `handle_of` / `split_handle` | 236-249 | REAL | F-001 | Generation/index pack; stale rejection correct until wrap. |
| tests ×5 | 260-366 | REAL | — | All content-strong, all executed in baseline (563-567); steady-state test runs 400 cycles and asserts the cap every round. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `acquire_emit_release_lifecycle_is_bounded_and_typed` | YES | Typed exhaustion fields asserted; double emit/release no-ops; recycle produces identical handle. |
| `capacity_is_clamped_to_the_safety_cap` | YES | Clamp + zero-capacity typed refusal asserted. |
| `drain_invalidates_every_prior_handle` | YES | Pre-drain handles must be no-ops; generation must advance. (Note: does NOT cover F-002's replace-pattern — it tests drain, the sessions don't use it.) |
| `garbage_handles_are_noops_never_panics` | YES | Four hostile handle shapes rejected. |
| `steady_state_acquire_release_recycles_without_growth` | YES | Cap asserted every round ×100; final state exact. |
| Run status | 5/5 `ok`, baseline :563-567; none ignored. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decode-side memory is bounded by construction … never an unbounded allocation, never a silent overwrite" | frame_pool.rs:10-15 | REAL for this file / VOIDED by consumers | Pool itself enforces cap + typed exhaustion (142-161); but F-002's replace-pattern re-enables silent aliasing at the session layer. |
| "every handle minted before a drain is permanently stale (no aliasing)" | frame_pool.rs:91-92, 223-226 | REAL until 2³² drains | F-001 wrap; and only if consumers actually call drain — they don't (F-002). |
| "It is *not* a pointer: it is a pool slot id" | frame_pool.rs:33-38 | REAL | Handle = (generation, index) packing (236-238). |

## 7. Dead / duplicated / unreachable code
- File unreachable in shipped builds (hw lanes off). Not cfg-gated (compiles on all targets; harmless dead code in the binary).

## 8. Dependencies & flags
Std-only (`std::fmt`); no unsafe; no feature gates; generic `T` keeps OS types out. Host-runnable tests (no OS/driver dependency) — that's why they execute in baseline while the consumers' driver paths cannot.

## 9. Open questions for the next agent
- W06-A06/A08/A09 (vaapi session parts) and W06-A02/A08 (videotoolbox): does any path hold an emitted `H264HwFrame.surface` across `configure()` (SPS change/seek)? If yes, F-002 is a live corruption recipe the moment hw-decode flips on; if no (frames flushed before reconfigure), downgrade F-002 to S4. The `H264HwFrame` producer side is decode.rs (W06-A01).
- W06-A01 (h264/decode.rs): the emitted frames' `surface: u64` provenance — same pool handle? Confirm the frame's lifetime vs pool reconfigure.
