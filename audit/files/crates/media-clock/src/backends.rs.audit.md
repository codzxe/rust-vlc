# AUDIT: crates/media-clock/src/backends.rs
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-23 |
| Lines / functions | 2117 / 30 production callables + 36 tests |
| File verdict | UNREACHABLE (real, tested code; zero shipped-entry-point callers) |
| Reachable from shipped entry point? | NO — exported API never called by lumen-cli/desktop (proof below) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |

## 1. One-paragraph verdict
The code is genuinely implemented and heavily tested: the deterministic fallback
planner (`plan_fallback`), the master-switch seam (`switch_locked`), video
steering (`observe_video`), the Ended latch (`mark_ended`/`seek_back`) and the
bounded 8-slot event ring all do real arithmetic with i128-checked math, typed
errors and no panics found. But the entire card-3.B2 surface is dead weight in
the shipped product: no code outside `media-clock`'s own tests ever calls
`with_backends`/`on_master_lost`/`switch_master`/`observe_video`/`mark_ended`/
`seek_back`/`take_master_change`. The module's own doc admits it, and the
shipped apps construct 3.B1 single-master clocks instead — so the player has no
master-loss fallback, no video-master mode, no Ended latch, and no
`ClockMasterChanged` events, contrary to doc 03 §4.7(d)/(f) which promise the UI
is told on fallback.

## 2. What actually works (verified)
- Baseline `cargo test -p media-clock` (audit/runs/baseline/cargo-test-workspace.txt:3552-3566):
  92 unit tests pass, incl. all 36 `backends::tests::*`; doctest
  `backends::ClockBackends (line 582)` passes; `tests/zero_alloc_backends.rs` ->
  `backend_surface_is_zero_allocation_after_warm_up ... ok` (line 3559), so the
  module-doc zero-alloc claim is measured, not argued.
- `rg -n "with_backends|on_master_lost|observe_video|mark_ended|seek_back|switch_master|take_master_change|ClockBackends"` over the whole tree (excluding audit/): every hit is in `crates/media-clock` itself (src, lib.rs re-export, tests/zero_alloc_backends.rs) — zero hits in `apps/` or any other crate.

## 3. Findings
F-001 [S2]
Where: crates/media-clock/src/backends.rs:70-80
Code:
```
//! # Engine wiring (handoff note)
//!
//! playback-engine currently constructs single-master clocks and
//! drives seeks through `reanchor` + `set_generation`; adopting the
//! backend surface means constructing via
//! [`MediaClock::with_backends`], reporting stream losses through
```
What is wrong: the entire 3.B2 feature (master-loss demotion chain, video-master
steering, Ended latch/seek-back, master-change events) is never wired into any
shipped engine. Shipped construction is `apps/cli/src/pipeline_host.rs:1605`
(`MediaClock::audio_master(sink.clock_source(), ...)` — over the **null** sink's
counter) and `:1629` `wall_clock()` (`MediaClock::system_monotonic(...)`); the
desktop app adopts the CLI host as a library (`apps/desktop/Cargo.toml`: `lumen-cli = { path = "../cli" }`). Why it matters: doc 03 §4.7(d) says on
audio-less fallback "the UI is told (truthful state, not a silent
approximation)" and §4.7(f) "audio-less media keeps playing on system time and
emits `ClockMasterChanged`" — no such event exists (`rg ClockMasterChanged` over
crates/: only doc comments; `crates/playback-engine/src/event.rs:47-49` defers
it as "Phase 3+ (`2.C3`/`7.C1`, `3.B2`)"; `crates/commands/src/catalog/event.rs`
has no master event). Mid-playback device loss in the shipped player therefore
has no fallback path at all. docs/gates/phase3-exit.md:106 marks 3.B2
"COMPLETE (merged)" — true at the crate-card scope (doc 13 row 3.B2 names this
file), false at the feature level. What correct looks like: the playback-engine
rewiring this doc names, or a doc/matrix that does not imply the player has
master-loss fallback. Confidence: HIGH — Verification: ran rg over the tree.

F-002 [S3]
Where: crates/media-clock/src/backends.rs:979-984
Code:
```
    let Some(next_source) = core.backend.wired.source_for(to) else {
        // Unreachable by construction: every caller validated the
        // target against the same wired set (plan_fallback /
        // switch_master). Defensive: refuse rather than invent.
        return (from, from);
    };
```
What is wrong: if this branch were ever reached, `on_master_lost` would return
`MasterLoss::Demoted { from, to }` with `from == to` and no event recorded — a
lying receipt with no trace. Currently unreachable (both callers validate the
target against the same `wired` set first), so S3 not S2. What correct looks
like: a typed impossible-state panic path or an event/lost-count receipt, not a
silent `(from, from)`. Confidence: HIGH — Verification: read both callers
(:746-755, :775-788).

F-003 [S4]
Where: crates/media-clock/src/backends.rs:700-704
Code:
```
            InitialMaster::Auto => [MasterKind::Audio, MasterKind::Video, MasterKind::System]
                .into_iter()
                .find(|kind| wired.wired(*kind))
                .unwrap_or(MasterKind::System),
```
What is wrong: `wall` is unconditionally `Some` at :697 (`wall:
Some(backends.wall)`), so `wired(System)` is always true and the `unwrap_or`
arm is dead code. Harmless; hygiene. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/media-clock/src/backends.rs:212-217 vs :26
Code:
```
const DEMOTION_ORDER: [MasterKind; 4] = [
    MasterKind::Audio,
    MasterKind::External,
    MasterKind::Video,
    MasterKind::Free,
];
```
What is wrong: the module doc (:26-29) and doc 13 card describe external
slotted after audio as `[PROPOSAL]`, and the order hard-codes it — but no
WASAPI/CoreAudio-style adapter exists anywhere: the only non-test
`MonotonicSource` implementors are `InstantSource` (wall, media-clock), the web
host's manually-advanced `PumpClock` (apps/web/src/host.rs:206, an event-throttle
counter) and playback-engine's wasm32 `ZeroMonotonic` (events.rs:585, constant
`0` — a stub source in shipped code). The `external` slot can only ever be fed a
fake; naming it in `DEMOTION_ORDER` bakes a proposal into policy. Confidence:
HIGH — Verification: `rg "impl .*MonotonicSource" crates/ apps/` (Bash output
note: the harness strips `[m` from displayed lines; `#[must_use]` displays as
`#ust_use]` — verified via Read tool, not a repo defect).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MasterStream::as_str | 145-152 | REAL | — | stable names, tested :2102 |
| MasterKind::label | 171-179 | REAL | — | 5-way mapping, exhaustive |
| MasterKind::loss_bit | 182-189 | REAL | — | System/Free = 0 (unlosable) |
| From<MasterStream> | 192-200 | REAL | — | total |
| MasterChangeReason::as_str | 252-258 | REAL | — | tested :2106-2109 |
| Display for SeekBackError | 370-384 | REAL | — | error context present |
| WiredBackends::source_for | 426-446 | REAL | — | Arc clones, no alloc of new objects |
| WiredBackends::wired | 449-456 | REAL | — | ignores losses (by design) |
| BackendCore::no_fallback | 479-492 | REAL | — | used by 3.B1 constructors (clock.rs) |
| BackendCore::multi | 495-503 | REAL | — | |
| EventRing::new | 522-529 | REAL | — | const, fixed 8 slots |
| EventRing::push | 532-540 | REAL | — | overflow counted, verified :2045-2066 |
| EventRing::pop | 543-551 | REAL | — | FIFO index math correct |
| plan_fallback | 559-572 | REAL | — | pure; exhaustive 640-cell matrix test :1166-1231 |
| ClockBackends::new | 623-632 | REAL | — | wall required |
| ClockBackends::audio/external/video/video_snap_ns/initial | 636-668 | REAL | — | builder, no validation needed pre-build |
| MediaClock::with_backends | 689-719 | REAL (unreachable in product) | S2 | see F-001 |
| MediaClock::on_master_lost | 736-756 | REAL (unreachable) | S2 | records loss then plans; idempotent |
| MediaClock::switch_master | 769-789 | REAL (unreachable) | — | typed NotWired/Lost refusals |
| MediaClock::observe_video | 804-843 | REAL (unreachable) | — | i128 error math; snap/jump split at <= bound (tested :1971-1983) |
| MediaClock::mark_ended | 857-867 | REAL (unreachable) | — | freezes incl. wall masters; idempotent |
| MediaClock::seek_back | 881-912 | REAL (unreachable) | — | re-arms paused; typed NotEnded/Overflow |
| MediaClock::is_ended | 915-917 | REAL (unreachable) | — | |
| MediaClock::take_master_change | 922-924 | REAL (unreachable) | — | FIFO drain, no consumer in product |
| MediaClock::master_changes_lost | 929-931 | REAL (unreachable) | — | |
| Source::master_kind | 936-956 | REAL | — | exhaustive 5-way |
| switch_locked | 967-1012 | REAL | S3 | F-002 defensive lie on impossible path |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fallback_plan_is_deterministic_over_the_full_matrix (:1166) | YES | brute-forces 16 wirings × 8 loss sets × 5 kinds, asserts plan equals first-available |
| master_loss_chain_audio_video_free_runs_with_events (:1238) | YES | asserts carried positions, epoch bumps, full event payloads |
| ended_latch_freezes_and_seek_back_re_arms_paused_not_free_running (:1741) | YES | the card's trap: resume-after-ended must not free-run; asserts Paused + held position + epoch |
| switches_across_i64_position_edges_carry_saturated (:1911) | YES | i64::MIN/MAX edges, saturation not wrap |
| event_ring_overflow_counts_instead_of_growing (:2045) | YES | direct ring unit test of the lost-count receipt |
| vocabulary_strings_are_stable (:2101) | WEAK | string tautology, but cheap wire-stability pin |
All 36 pass in baseline (cargo-test-workspace.txt:3463-3566 region). Mutation
reasoning only (no rerun) — but the asserts are on computed values, not echoes.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "3.B2 clock backends COMPLETE (merged)" | docs/gates/phase3-exit.md:106 | REAL at crate scope / feature absent | code + tests exist; rg shows zero engine callers |
| "Missing audio track → master is SystemMonotonic, and the UI is told; emits ClockMasterChanged" | docs/03-engine-plan.md §4.7(d),(f) | FALSE (engine side) | CLI falls back via `wall_clock` (pipeline_host.rs:1629) with no event; no ClockMasterChanged variant in crates/commands |
| "allocation-free steady state, measured" | backends.rs:53-58 | REAL | baseline zero_alloc_backends test ok |
| "audio resyncs to video within ±1000 ppm DriftCorrection budget" | backends.rs:14-17 | REAL-UNPROVEN (crate) / unreachable (product) | drift.rs is crate-internal; engine never runs video-master mode |

## 7. Dead / duplicated / unreachable code
The whole public surface beyond what 3.B1 uses: `with_backends`, `on_master_lost`,
`switch_master`, `observe_video`, `mark_ended`, `seek_back`, `is_ended`,
`take_master_change`, `master_changes_lost`, `ClockBackends`, `MasterTarget`,
`MasterSwitch`, `SwitchError`, `SeekBackError`, `VideoSteer`, `MasterLoss`,
`InitialMaster`, `MonotonicKind::VideoWall/External/Free` — compiled, tested,
exported (lib.rs:117), never called in product. `BackendCore::no_fallback` and
the 2-variant core ARE reachable (3.B1 constructors in clock.rs).

## 8. Dependencies & flags
`media-types` only (Cargo.toml:19-20); no features, no unsafe, no std beyond
sync/fmt. No feature gating — the dead surface ships in every build.

## 9. Open questions for the next agent
- state.rs/pipeline.rs: does the engine have its own EOF/Ended handling that
  duplicates `mark_ended` semantics without the latch (pipeline.rs:2591
  `observe_video_end` is a different, pipeline-local method — check overlap)?
- Does any scenarios_b test (device_loss, 12) exercise master loss through the
  real engine, or only through media-clock fakes?
- apps/web/src/host.rs:1737 also builds `MediaClock::audio_master` — same
  single-master shape (not audited in detail here; another agent's file).
