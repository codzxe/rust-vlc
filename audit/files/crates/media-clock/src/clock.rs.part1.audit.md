# AUDIT: crates/media-clock/src/clock.rs#part1 (lines 1-589)
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-21 |
| Lines / functions | 589 / 25 items (2 enums + 2 Display/state helpers, 4 structs, 1 trait +6 methods, 3 crate-internal enums, Core struct + extrapolate, 6 free fns, MediaClock +7 methods; module docs) |
| File verdict | REAL (audio-master and system-monotonic paths shipped + executed; card 3.B2 master variants UNREACHABLE in shipped builds — F-001) |
| Reachable from shipped entry point? | YES for AudioOutput/SystemMonotonic masters (chain: `lumen` video limb → apps/cli/src/pipeline_host.rs:1605 `MediaClock::audio_master(sink.clock_source(), ...)`; audio-less lane :1629-1634 `MediaClock::system_monotonic`); NO for VideoOutput/External/FreeRunning (zero callers outside media-clock) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | `rg -n "with_backends\|observe_video\|mark_ended\|seek_back" apps/ crates/` (no callers outside media-clock) · `rg -n "audio_master\|system_monotonic" apps/` (CLI wiring at pipeline_host.rs:1605/1630) · arithmetic review of extrapolate/checked_div_round/naive_position against Rate{num:u32, den:u32} (media-types time.rs:839-841) · `rg -n "enum MonotonicKind"` (4 variants — Source::master total) · backends.rs:857-864 read (mark_ended clears `paused` — upholds part1's is_paused doc) · baseline log: clock::tests 18/18 + backends::tests passed (:3204-3260, 92 tests) |

## 1. One-paragraph verdict
This is the best-engineered file audited so far in my set: the audio-master clock's arithmetic is done in i128 with checked mul/div and i64-saturated positions, regression and sample-rate-change detection compare against the last sound reading rather than the anchor (catching mid-stream device resets), pause/resume are genuinely jump-free, and the poison-recovery lock discipline matches its documentation. Every part1 behavior claim I could check without running code is upheld by a passing baseline test (18 clock::tests + the backends suite, 92 tests total, all ok in the no-fail-fast run). Two findings: the card 3.B2 multi-master surface (VideoOutput/External/FreeRunning constructors via `with_backends`, the Ended latch, video steers) is real, tested code that no shipped entry point can reach — the CLI wires only `audio_master` and `system_monotonic`; and one diagnostic nit (`naive_position`'s saturation sentinel is sign-blind, harmless today because its inputs are provably non-negative).

## 2. What actually works (verified)
— media_clock unittests: 92 tests ran, all ok, incl. every part1 behavior: `anchor_arithmetic_tracks_the_counter`, `anchor_arithmetic_scales_by_rate`, `monotonic_master_tracks_injected_wall_time`, `reanchor_jumps_are_expected_and_epoch_bounded`, `reanchor_overflow_leaves_clock_unchanged`, `counter_regression_holds_position_never_backwards`, `sample_rate_change_is_a_discontinuity`, `broken_source_is_unusable_not_panic`, `extreme_positions_saturate_monotonically`, `set_rate_never_jumps_and_rescales`, `set_rate_rejects_zero_components`, `pause_freezes_resume_continues_without_jump`, `pause_and_resume_are_idempotent`, `reanchor_while_paused_moves_the_freeze`, `rate_survives_pause_resume`, `snapshot_is_consistent_and_carries_context`, `anchor_introspection_reflects_the_master`, `send_sync_object_safe` (`audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3204-3260`).
— Shipped wiring: the AVI first-vertical lane's clock is `MediaClock::audio_master` (pipeline_host.rs:1605-1608) and the audio-less lane `system_monotonic` (:1629-1634) — this file's part1 constructors run in the shipped binary.
— Arithmetic soundness (hand-proof): `Rate{num: u32, den: u32}` (media-types time.rs:839-841, num validated non-zero) ⇒ `delta ≥ 0` (ns ≥ last_source_ns ≥ anchor), `scaled = delta × num ≥ 0` ⇒ `media_delta ≥ 0` ⇒ i128 overflow only in the positive direction, where `Extrapolation::Running(i128::MAX)` → `saturate_position` → `i64::MAX` — the saturation is always in the true direction, upholding the doc's "monotonic even at the clamp" (clock.rs:34-36).
— `checked_div_round` (411-427): ties-away-from-zero correct for both signs; `denom ≤ 0` refused (rate validated at boundaries makes it exhaustive-only).
— `is_paused` doc ("Ended reports false", :570-572) upheld: `mark_ended` clears `paused` (`core.paused = false;` backends.rs:864, verified by reading; test `mark_ended_is_idempotent_and_reports_the_same_freeze` passed).

## 3. Findings
### F-001 [S3]
Where: crates/media-clock/src/clock.rs:93-108 (variants), 463 (with_backends pointer)
Code (clock.rs:93-108):
```
    /// Video frames drive the clock (card 3.B2): the clock
    /// interpolates on the injected monotonic source between
    /// [`MediaClock::observe_video`] steers, and the audio side
    /// resyncs to video within bounded limits (the existing ±1000
    /// ppm [`crate::DriftCorrection`] budget — no new mechanism).
    VideoOutput,
```
Code (apps/cli/src/pipeline_host.rs:1605-1606):
```
    let clock = Arc::new(MediaClock::audio_master(
        sink.clock_source(),
```
What is wrong: `ClockMaster::VideoOutput`, `ClockMaster::External`, `ClockMaster::FreeRunning` — and the entire card 3.B2 surface they gate (`MediaClock::with_backends`, `observe_video`, `mark_ended`, `seek_back`, the Ended latch in `Core.backend`) — have zero callers outside the media-clock crate. The shipped CLI constructs only `audio_master` (pipeline_host.rs:1605) and `system_monotonic` (:1630). rg over apps/ and crates/ for `with_backends|observe_video|mark_ended|seek_back` returns only media-clock internals/tests (the pipeline.rs `observe_video_end` and streaming `seek_back_*` hits are unrelated names).
Why it matters: a third of this file's public vocabulary (and a whole companion module) is production-dead — a video-only session in the shipped build uses `system_monotonic` wall time, not the video-steered master the 3.B2 design describes. Real, tested code (backends.rs suite passed) waiting for an engine wiring that does not exist.
What correct looks like: the engine's video limb should construct the clock with the video backend for audio-less media (the 3.B2 fallback chain), or the variants should be feature-gated until wired.
Confidence: HIGH — Verification: rg exhaustions + CLI call sites read.

### F-002 [S4]
Where: crates/media-clock/src/clock.rs:403-409
Code:
```
fn naive_position(core: &Core, source_ns: i128) -> i128 {
    let delta = source_ns.saturating_sub(core.anchor_source_ns);
    let scaled = delta.saturating_mul(i128::from(core.rate.num));
    let media_delta = checked_div_round(scaled, i128::from(core.rate.den)).unwrap_or(i128::MAX);
```
What is wrong: `unwrap_or(i128::MAX)` is sign-blind: a `None` (overflow inside `checked_div_round`) on a negative `scaled` would produce a +i128::MAX "best-effort" diagnostic — the wrong sign and the wrong magnitude for error context. Unreachable today because `delta ≥ 0` and `rate.num ≥ 1` make `scaled ≥ 0` always (so a None can only mean +overflow, where MAX is right), but the guard documents nothing and would silently mislead if a signed rate ever landed.
Why it matters: diagnostics-only path; zero user impact today.
What correct looks like: `unwrap_or(if scaled < 0 { i128::MIN } else { i128::MAX })` or a comment pinning the non-negativity invariant it relies on.
Confidence: HIGH — Verification: read + sign analysis against Rate{u32,u32}.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-69 | REAL | — | semantics §(e) claims each verified by a passing test (§2); deviations list honest |
| `enum ClockMaster` + Display | 82-121 | REAL | S3 | AudioOutput/SystemMonotonic shipped; VideoOutput/External/FreeRunning unreachable (F-001); Display strings fine |
| `enum ClockState` | 124-137 | REAL | — | Ended semantics documented; mark_ended interplay verified (§2) |
| `struct ClockSnapshot` | 141-168 | REAL | — | every field consumed (snapshot_locked :618-629); epoch-bump rules documented and test-covered (`reanchor_jumps_...epoch_bounded`) |
| `struct ClockAnchor` | 173-186 | REAL | — | consumed by `anchor()` (:586, body in part2) |
| `trait PlaybackClock` (6 methods) | 192-242 | REAL | — | object-safe (test passed); implemented by MediaClock (part2 impl :778+); held as `Arc<dyn PlaybackClock>` in CLI (pipeline_host.rs:1326) |
| `enum Source` + `master()` | 248-283 | REAL | — | total over MonotonicKind's 4 variants (verified backends.rs:116-126); source inside Core's lock as documented |
| `enum Reading` | 287-297 | REAL | — | Unusable = zero-rate device — typed hold, never a guess (test `broken_source_is_unusable_not_panic`) |
| `struct Core` (13 fields) | 302-349 | REAL | — | every field consumed in part2/backends.rs (rg: awaiting_anchor at backends.rs:901+, backend everywhere); last_source_ns-vs-anchor regression baseline is the right design |
| `enum Extrapolation` | 352-363 | REAL | — | carries i128 raw for caller saturation |
| `Core::extrapolate` | 365-401 | REAL | — | paused/ended freeze; Unusable→Discontinuity; regression/scale-change→Discontinuity; checked mul/div/add with i128::MAX sentinel (sign-safe given Rate is u32/u32 — §2) |
| `naive_position` | 403-409 | REAL | S4 | F-002 sign-blind sentinel (unreachable today) |
| `checked_div_round` | 411-427 | REAL | — | correct ties-away both signs; None only on i128 edge |
| `position_nanos` | 435-441 | REAL | — | checked both ways; Overflow typed with base context |
| `saturate_position` | 444-446 | REAL | — | clamp-then-cast is exact |
| `clock_state` | 450-458 | REAL | — | Ended outranks Paused — matches mark_ended clearing paused |
| `struct MediaClock` | 464-466 | REAL | — | one Mutex<Core> as documented |
| `MediaClock::audio_master` | 479-490 | REAL | — | SHIPPED (pipeline_host.rs:1605); no_fallback backend |
| `MediaClock::system_monotonic` | 499-513 | REAL | — | SHIPPED (pipeline_host.rs:1630) |
| `MediaClock::build` | 515-534 | REAL | — | validate → read → anchor → adopt; unusable-first-read leaves honest hold (documented, tested) |
| `MediaClock::pause` | 542-550 | REAL | — | idempotent; no-op under Ended; freeze computed from current reading under lock |
| `MediaClock::resume` | 558-568 | REAL | — | re-anchors at (frozen, current reading) — jump-free by construction (test passed); epoch += 1; no-op under Ended |
| `MediaClock::is_paused` | 573-575 | REAL | — | Ended→false upheld (mark_ended clears the flag) |
| `MediaClock::set_generation` | 581-583 | REAL | — | no epoch bump — documented rationale |
| `MediaClock::anchor` | 586-589(+part2) | REAL | — | CHUNK BOUNDARY: signature + first lines in part1; body :590-602 audited by A05 (part2) |

## 5. Test honesty (tests live at :831-1178, part2's range, but they are the execution evidence for part1's functions; all passed — baseline :3243-3260)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `anchor_arithmetic_tracks_the_counter` / `_scales_by_rate` | YES | exact position asserts against fake counter advance |
| `counter_regression_holds_position_never_backwards` | YES | fake counter reset mid-stream; asserts hold (this pins last_source_ns-vs-anchor design) |
| `sample_rate_change_is_a_discontinuity` | YES | source swaps rate_hz mid-stream; asserts Discontinuity |
| `broken_source_is_unusable_not_panic` | YES | zero-rate source; asserts hold, no panic |
| `extreme_positions_saturate_monotonically` | YES | huge counts; asserts clamp + monotonic |
| `pause_freezes_resume_continues_without_jump` / `_idempotent` / `rate_survives_pause_resume` / `reanchor_while_paused_moves_the_freeze` | YES | exact no-jump asserts around pause/resume |
| `set_rate_never_jumps_and_rescales` / `_rejects_zero_components` | YES | position continuity + typed rejection |
| `snapshot_is_consistent_and_carries_context` | YES | field agreement under lock |
| `anchor_introspection_reflects_the_master` | YES | at_sample/at_wall_ns per master |
| `send_sync_object_safe` | WEAK (compile-time) | trait-object safety assert — fine as a bound check |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "now() = anchor + (counter_elapsed × rate) within one anchor epoch" | clock.rs:15 | REAL | anchor_arithmetic tests passed |
| "Monotonic within one anchor epoch — never backwards, equal at worst" | clock.rs:18-19 | REAL | regression test passed; last_ns clamp in now_locked (part2) |
| "Pause: now() freezes. Resume ... no discontinuity" | clock.rs:22-25 | REAL | pause/resume tests passed |
| "try_now reports TimeError::Discontinuity ... until the session re-anchors" | clock.rs:29-31 | REAL (part2 body) | sample_rate_change/broken_source tests passed |
| "extreme sample counts ... saturate at the i64 bound (monotonic even at the clamp) ... Overflow" | clock.rs:34-36 | REAL | extreme_positions test passed; sign analysis §2 |
| "critical sections are one source read plus pure checked arithmetic — never held across I/O" | clock.rs:40-42 | REAL | every part1 critical section read (pause/resume/build); source contract lives in source.rs (my file #10) |
| "a poisoned lock is recovered via PoisonError::into_inner" | clock.rs:44-46 | REAL | lock_core :607-612 (part2 lines, read for verification) |
| Card 3.B2 masters (video/external/free) extend the clock | clock.rs:63-69 | REAL code / UNREACHABLE shipped | F-001: no callers outside crate |

## 7. Dead / duplicated / unreachable code
- `ClockMaster::VideoOutput/External/FreeRunning` + `with_backends`/`observe_video`/`mark_ended`/`seek_back` surface: unreachable from shipped entry points — F-001 (well-tested; backends suite passed).
- `checked_div_round`'s negative branches: unreachable given non-negative inputs — correct generality, not dead weight (kept as note, not a finding).

## 8. Dependencies & flags
- Depends only on media-types (time), std, and crate-internal modules (backends, mapping, source). No feature gates. Media-clock is a workspace member built by baseline #1 and its 92 unittests + 3 doctests passed (:3204-3260, :6425-6429).

## 9. Open questions for the next agent
- A05 (part2, lines 590-1178): `anchor()`'s monotonic arm does `saturate_position(core.anchor_source_ns.max(0)) as u64` (:590-593) — check the `at_wall_ns` u64 semantics for pre-anchor-epoch sources; and verify `now_locked`'s `last_ns` update on Paused extrapolations.
- Engine wave: who calls `set_generation` and does the snapshot's generation actually gate consumers (doc 01 §3.2)? I found the engine wiring outside my chunk.
- The ±1000 ppm `DriftCorrection` budget mentioned at :96-97 — which crate owns it and is it reachable? (referenced as "existing" — verify it exists).

## Chunk boundary note
Part1 = lines 1-589. Line 589 is inside `MediaClock::anchor` (starts :586; body :590-602 → part2/A05). The `#[cfg(test)]` module starts :831 (part2). No part1 line audited beyond 589; part2 functions were read only to verify part1's doc claims (marked in §2).
