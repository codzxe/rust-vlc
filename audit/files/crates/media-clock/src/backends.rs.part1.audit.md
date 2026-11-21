# AUDIT: crates/media-clock/src/backends.rs#part1 (lines 1–529)
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-21 |
| Lines / functions | 529 audited (1-529; boundary cuts after `EventRing::new` — push/drain and `plan_fallback` (:559) are part2/A04; `impl MediaClock` backend methods are parts 2-4) / 26 items in range |
| File verdict | SPLIT — REAL (vocabulary + `BackendCore::no_fallback` path: shipped via 3.B1 constructors) / UNREACHABLE (card 3.B2 multi-backend surface: master-loss fallback, video steering, Ended latch, external master — zero callers outside media-clock's own tests) |
| Reachable from shipped entry point? | PARTIAL. Shipped: `apps/cli/src/pipeline_host.rs:1605` `MediaClock::audio_master(...)` / `:1630` `system_monotonic(...)` → `clock.rs:486/:509` → `BackendCore::no_fallback()` (:479-492) — executed in every playback smoke run (INDEX.md AVI/MP4 transcripts). NOT shipped: `rg 'with_backends\|on_master_lost\|mark_ended\|seek_back\|observe_video\|switch_master'` over crates+apps (excluding media-clock itself) → ZERO hits; the module doc admits it (:72-79 handoff note: "playback-engine currently constructs single-master clocks… That rewiring is a playback-engine card"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `sed -n '1,530p'` (chunk) · `rg -n 'with_backends\|on_master_lost\|mark_ended\|seek_back\|observe_video\|switch_master' crates apps --glob '*.rs'` (zero external callers — the F-001 proof) · `rg -n 'BackendCore\|MonotonicKind' clock.rs` (:76 import, :486/:509 no_fallback — shipped path) · `rg -n 'MediaClock::audio_master\|system_monotonic' apps/cli/src` (:1605/:1630 — shipped entry chain) · `sed -n '3204,3210p' + '3302,3310p' orchrun…log` (media_clock 92/92 unit incl. backends::tests; zero_alloc_backends 1/1) · `rg -n -i 'master' docs/feature-matrix.toml` (no shipped-feature claim about clock master switching — only HLS/volume hits) · `rg -n 'fn .*determin\|plan_fallback'` (determinism tests exist :1166/:1452, part2+) |

## 1. One-paragraph verdict
Part 1 defines the card 3.B2 backend vocabulary (master kinds, loss bits, the demotion order, the typed `MasterLoss`/`MasterSwitch`/`VideoSteer`/`SeekBackError` outcomes, the fixed 8-slot `EventRing`, and `BackendCore`) — all of it carefully typed, allocation-free by construction, and tested in-crate (92/92 media-clock unit tests and the zero-alloc backend-surface test pass in the no-fail-fast baseline). But the module's headline features are dead code in the shipped product: nothing outside media-clock constructs a backends clock, reports a lost master, steers video, latches Ended, or seeks back — the shipped `lumen` uses only the 3.B1 single-master constructors, which wire `BackendCore::no_fallback()` (every backend `None`, so every loss is `NoFallback`). The module doc says so itself in the handoff note, which is honest disclosure — but a reader of doc 13's card 3.B2 story would believe master-loss demotion exists in the product; it exists only as a library waiting for a playback-engine card that has not landed.

## 2. What actually works (verified)
— Typed vocabulary completeness: every refusal/no-op has a typed outcome (`MasterLoss::{Demoted,Recorded,NoFallback}`, `MasterSwitch::{Switched,Already}`, `SwitchError::{NotWired,Lost}`, `SeekBackError::{NotEnded,Overflow}`, `VideoSteer::{Snapped,Jumped,NotMaster,Paused,Ended}`) — no silent swallowing anywhere in the surface (:260-412).
— `DEMOTION_ORDER` (:212-217) matches the documented chain "audio → external-slotted-after-audio → video → free" (:26-27, :209-211).
— `WiredBackends::source_for` (:426-446): audio→`Source::Samples`, external/video/free/system→`Source::Monotonic` with the truthful `MonotonicKind` label; `Arc` clones are refcount work (no allocation — the :414-415 claim is correct).
— `MasterKind::loss_bit` (:182-189): `Free`/`System` return 0 — matches "not losable" (:181, :111-112).
— `DEFAULT_VIDEO_SNAP_NS = 40_000_000` = one 25 fps frame — the :94-95 claim is arithmetically right.
— `EventRing` (:509-529): fixed `[Option<MasterChanged>; 8]`, `const fn new`, Copy — allocation-free by construction; overflow policy (lost-count receipt, :516-518) implemented in part2.
— Zero-alloc claim (:51-56 "measured…, not argued"): `tests/zero_alloc_backends.rs::backend_surface_is_zero_allocation_after_warm_up` ran in the baseline and PASSED (orchrun log :3302-3306: 1 passed / 0 failed).
— In-crate behavioral tests for the surface exist and pass: `backends::tests::auto_picks_audio_when_wired`, `auto_picks_video_for_audioless_with_video`, `ended_latch_freezes_and_seek_back_re_arms_paused_not_free_running` (orchrun log :3207-3210), determinism tests `fallback_plan_is_deterministic_over_the_full_matrix` (:1166) and `loss_scripts_are_deterministic_across_identical_clocks` (:1452).

## 3. Findings

### F-001 [S3]
Where: crates/media-clock/src/backends.rs:70-80
Code:
```
//! # Engine wiring (handoff note)
//!
//! playback-engine currently constructs single-master clocks and
//! drives seeks through `reanchor` + `set_generation`; adopting the
//! backend surface means constructing via
//! [`MediaClock::with_backends`], reporting stream losses through
//! [`MediaClock::on_master_lost`], steering per presented frame in
//! video-master mode, calling [`MediaClock::mark_ended`] at EOF and
//! [`MediaClock::seek_back`] for the play-from-ended restart. That
//! rewiring is a playback-engine card; this crate's 3.B1 API stays
//! byte-compatible.
```
What is wrong: The module's entire reason to exist — master switching, loss fallback, video steering, the Ended latch, external-master mode — is wired to nothing. Exhaustive search: `rg 'with_backends|on_master_lost|mark_ended|seek_back|observe_video|switch_master'` across `crates/` and `apps/` (excluding media-clock's own src/tests) returns ZERO hits. The shipped `lumen` constructs clocks exclusively via `MediaClock::audio_master` (apps/cli/src/pipeline_host.rs:1605) and `system_monotonic` (:1630), both of which build `BackendCore::no_fallback()` (clock.rs:486/:509) — wired all-`None`, so the demotion chain can never run and every loss report would be `NoFallback`.
Why it matters: ~1,300 lines (this file, 2,118 total) of typed, tested clock-governance code ship as inert library weight: when the audio device disappears mid-playback (the exact hotplug scenario crates/platform-* and the hotplug hooks were built for), none of this fallback executes — the engine's own hotplug lane handles it separately. Doc 13 card 3.B2's story is a library, not a product behavior. Honest disclosure keeps this at S3, not S0 — the file does not lie; the capability simply does not exist in any shipped binary.
What correct looks like: Either the playback-engine rewiring card lands (construct via `with_backends`, call `mark_ended` at EOF, `seek_back` on play-from-ended), or every card-3.B2 API gains a `#[doc]` "not yet consumed by any host" banner so doc readers do not infer product behavior.
Confidence: HIGH — Verification: read + exhaustive rg (zero callers); shipped constructor chain traced end-to-end; baseline smoke transcripts cited for the 3.B1 path.

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-80 | REAL (disclosure) | S3 | The handoff note is the honest F-001 confession; determinism/bounded/threading claims verified where they touch this chunk. |
| `DEFAULT_VIDEO_SNAP_NS` | 94-100 | REAL | — | 40 ms = one 25 fps frame; `[PROPOSAL]` per-media config noted. |
| `MASTER_EVENT_CAPACITY` | 102-106 | REAL | — | 8 slots + lost-count receipt. |
| `MonotonicKind` | 115-128 | REAL | — | Shipped: clock.rs:256-278 uses it for every Monotonic source (System variant in the 3.B1 path). |
| `MasterStream` + `as_str` | 133-152 | REAL-UNPROVEN | — | Wire names stable; no shipped caller (engine never reports losses). |
| `MasterKind` + `label`/`loss_bit` | 158-190 | REAL | — | Free/System loss_bit 0 — correct "not losable" semantics. |
| `From<MasterStream> for MasterKind` | 192-200 | REAL | — | Trivial, correct. |
| `LOST_AUDIO/EXTERNAL/VIDEO` bits | 202-207 | REAL | — | Bitset vocabulary; no overlap. |
| `DEMOTION_ORDER` | 209-217 | REAL-UNPROVEN | — | Matches documented chain; consumed by `plan_fallback` (:559, part2 — A04 to verify the planner logic). |
| `MasterChanged` | 222-238 | REAL-UNPROVEN | — | Copy, fixed-size; carries the position receipt + epoch + generation. |
| `MasterChangeReason` + `as_str` | 241-258 | REAL-UNPROVEN | — | Two reasons; stable names. |
| `MasterLoss` | 262-281 | REAL-UNPROVEN | — | Three typed outcomes; `NoFallback`'s "frozen truth, never fabricated" semantics are the honest choice. |
| `InitialMaster` | 285-302 | REAL-UNPROVEN | — | Auto default; external never chosen silently — matches :287-291. |
| `MasterTarget` + `as_str` | 304-325 | REAL-UNPROVEN | — | System/Free excluded with documented rationale. |
| `MasterSwitch` | 327-338 | REAL-UNPROVEN | — | Switched/Already (idempotent no-op typed). |
| `SwitchError` | 340-348 | REAL-UNPROVEN | — | NotWired/Lost; clock unchanged. |
| `SeekBackError` + `Display` | 350-384 | REAL-UNPROVEN | — | NotEnded carries state+position (diagnosable); Overflow carries the TimeError. |
| `VideoSteer` | 386-412 | REAL-UNPROVEN | — | Snapped (no epoch bump, monotone clamp absorbs) vs Jumped (epoch bump) vs three non-adoption reasons — matches the video-master design. |
| `WiredBackends` | 416-422 | REAL | — | Shipped in the all-`None` form via no_fallback; Clone = refcount only. |
| `WiredBackends::source_for` | 424-446 | REAL | — | Correct kind labeling per master; no allocation. |
| `WiredBackends::wired` | 448-456 | REAL | — | Free/System → wall presence — **open question for A04**: is `wall` mandatory in `with_backends`? If not, the Free floor could be unwired and the chain could dead-end above `NoFallback` (constructor validation is part2). |
| `BackendCore` | 460-471 | REAL | — | Fixed-size state; `ended` is `pub(crate)` (clock.rs reads it). |
| `BackendCore::no_fallback` | 473-492 | REAL | — | **Shipped**: both 3.B1 constructors use it. |
| `BackendCore::multi` | 494-503 | REAL-UNPROVEN | — | Tests-only constructor path (part2 `with_backends`). |
| `EventRing` + `new` | 506-529 | REAL-UNPROVEN | — | const fn, fixed slots; push/drain/lost-count in part2. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `backends::tests::auto_picks_{audio,video,system_wall}…` (orchrun :3207-3210) | YES | Auto-selection matrix — would fail if `InitialMaster::Auto` mis-prioritized. (Part2 bodies; vocabulary from this chunk.) |
| `backends::tests::ended_latch_freezes_and_seek_back_re_arms_paused_not_free_running` (orchrun :3210) | YES | Pins the Ended-latch freeze + paused (not free-running) re-arm — the module's headline behavioral claim, executed and passing. |
| `tests/zero_alloc_backends.rs::backend_surface_is_zero_allocation_after_warm_up` (orchrun :3302-3306) | YES | Counting allocator over the backend surface — would fail on any allocation; passed. |
| Determinism tests `fallback_plan_is_deterministic_over_the_full_matrix` (:1166), `loss_scripts_are_deterministic_across_identical_clocks` (:1452) | YES | Full-matrix plan equality + twin-clock loss scripts — passed within the 92/92. |
All 92 media-clock unit tests + 1 zero-alloc test pass in the no-fail-fast baseline (orchrun :3204-3306). Caveat: all of this exercises the surface **in-crate only** — see F-001.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Steady-state tick paths… allocate nothing — measured by the counting global allocator in tests/zero_alloc_backends.rs, not argued" | backends.rs:51-56 | REAL | Test executed and passed in baseline (orchrun :3302-3306). |
| "The same loss script over two identically-wired clocks yields identical master sequences, events and epochs (unit-tested)" | backends.rs:44-47 | REAL | `loss_scripts_are_deterministic_across_identical_clocks` (:1452) passed in the 92/92. |
| "The fallback plan is a pure function of (wired backends, lost kinds, current master)" | backends.rs:44-46 | REAL | `fallback_plan_is_deterministic_over_the_full_matrix` (:1166); planner body in part2 (A04 verifies). |
| Master-loss demotion exists for real playback | backends.rs:23-33 (module story) | UNREACHABLE | Zero shipped callers (F-001); handoff note :72-79 admits engine rewiring is a future card. |
| No feature-matrix row claims clock master switching as shipped | docs/feature-matrix.toml (searched "master") | N/A (no claim) | Only HLS-master-playlist and master-volume hits — no inflation at the matrix level. |

## 7. Dead / duplicated / unreachable code
The entire card 3.B2 surface beyond `BackendCore::no_fallback` + `MonotonicKind` is unreachable from shipped entry points (F-001): `WiredBackends::multi` construction, `DEMOTION_ORDER` consumption, `EventRing` writes, `MasterChanged` events, all typed switch/loss/steer outcomes. No duplication found in range.

## 8. Dependencies & flags
`media_types` (GenerationId, time types), `crate::clock` (Core/Source/snapshot helpers), `crate::mapping::NANO_BASE`, `crate::source` (MonotonicSource, SampleCounterSource) — all used. No feature gates, no unsafe, no external deps.

## 9. Open questions for the next agent
- A04 (part2, :530-1058): does `MediaClock::with_backends` REQUIRE a wall source (making the `Free` floor always wired), and does `plan_fallback` (:559) honor `DEMOTION_ORDER` with the loss bitset correctly (skip lost, stop at Free)?
- Wave 16 A05 owns `tests/zero_alloc_backends.rs` — my chunk cites its baseline pass; the owner should confirm which surface points it covers (switch/loss/steer paths).
- If the orchestrator wants the F-001 story quantified: which doc 13 / doc 03 sections describe card 3.B2 as product behavior? A doc-audit agent should map the claim trail.
