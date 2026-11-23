# AUDIT: crates/media-clock/src/clock.rs
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-23 |
| Lines / functions | 1178 / 26 production callables + 24 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — lumen-cli main -> run::run -> dispatch -> CliHost -> pipeline_host::open_audio_session (apps/cli/src/pipeline_host.rs:1605 `MediaClock::audio_master` over the null sink) / :1630 `wall_clock` -> `MediaClock::system_monotonic`; engine drives it via `PlaybackClock` (crates/playback-engine/src/session.rs:67 imports; :1652/:1655/:2157 `clock.pause()/resume()`, :2818 `clock.now()`); web host same shape (apps/web/src/host.rs:1737) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |

## 1. One-paragraph verdict
This is the load-bearing clock and it is sound: anchor arithmetic is fully
checked (i128 with saturation), the monotone clamp (`last_ns`) makes `now()`
never regress, source regression and rate changes become typed
`TimeError::Discontinuity` holds rather than invented positions, pause/resume
re-anchor correctly, and no panic path exists inside any critical section (the
poisoning-recovery comment at :604-606 is actually true — verified by walking
every locked call). The deviations from doc 03 §4.7's sketch (f64 rate,
seqlock wait-free readers, two-variant master) are all recorded inline in the
module docs. Defects found are minor: a stale reference to a nonexistent test
file, a slightly misleading `found` value in the broken-device discontinuity
error, and the doc's "wait-free" claim that v0 does not meet.

## 2. What actually works (verified)
- Baseline (audit/runs/baseline/cargo-test-workspace.txt:3552-3554):
  "test result: ok. 92 passed; 0 failed" for media-clock unit tests (this
  file's 24 among them at lines 3496-3513: anchor_arithmetic_tracks_the_counter,
  counter_regression_holds_position_never_backwards, extreme_positions_
  saturate_monotonically, pause_freezes_resume_continues_without_jump etc.).
- Reachability verified with rg: construction sites in apps/cli/src/pipeline_host.rs
  (:1605, :1630) and apps/web/src/host.rs:1737; trait consumers in
  crates/playback-engine/src/{session,tracks,seek}.rs (imports at :67/:135/:146).
- Arithmetic walk-through (read): `checked_div_round` (:414-427) rounds ties
  away from zero correctly for ±; `read_source` (:674-700) converts samples to
  ns in i128 with a zero-rate guard; `extrapolate` (:368-400) checks mul, div
  and add, saturating to i128::MAX instead of wrapping.

## 3. Findings
F-001 [S3]
Where: crates/media-clock/src/clock.rs:836-837
Code:
```
    /// A deterministic fake counter (unit-level; the acceptance tests
    /// use testkit virtual time — `tests/virtual_time.rs`).
```
What is wrong: `crates/media-clock/tests/virtual_time.rs` does not exist — the
tests directory contains only `zero_alloc_backends.rs` (verified by ls). The
same phantom reference appears in source.rs:7-8 ("virtual time in tests via an
adapter over testkit's VirtualClock") and lib.rs. Doc 03 §4.7(f) demands
"virtual-clock determinism" tests; the only virtual-time consumers live in
other crates' fakes. Why it matters: the claimed home of the clock's
virtual-time acceptance tests is dead air; a reader auditing coverage is
misdirected. Confidence: HIGH — Verification: ls + rg over the crate.

F-002 [S4]
Where: crates/media-clock/src/clock.rs:374-377
Code:
```
        let Reading::Usable { ns, rate_hz, .. } = reading else {
            return Extrapolation::Discontinuity {
                found_ns: i128::from(self.last_ns),
            };
        };
```
What is wrong: for an Unusable reading (zero-rate broken device) the reported
`found` in `TimeError::Discontinuity` equals the held position (`last_ns`) —
the error says "found X" where the source produced no value at all. Slightly
misleading diagnostics; the Discontinuity itself is correct behavior.
Confidence: HIGH — Verification: read + broken_source_is_unusable_not_panic
test (:1014-1030) confirms the hold+error contract.

F-003 [S4]
Where: crates/media-clock/src/clock.rs:38-47
Code:
```
//! No dedicated thread. One short internal [`Mutex`] guards the anchor;
//! critical sections are one source read plus pure checked arithmetic —
//! never held across I/O (sources contractually non-blocking,
```
What is wrong: doc 03 §4.7(c) specifies "readers (`now()`) are wait-free
(seqlock-style or two-word atomic — no locks on the read path)". v0 takes a
mutex on every `now()`. The code openly records this as "Phase-3 hardening"
(:46-47), so this is an honest, recorded deviation — but the doc claim remains
unmet in the shipped build. Confidence: HIGH — Verification: read; every
`PlaybackClock` method locks (:772-828).

F-004 [S4]
Where: crates/media-clock/src/clock.rs:552-568
Code:
```
    pub fn resume(&self) {
        let mut core = self.lock_core();
        if !core.paused || core.backend.ended {
            return;
        }
        let reading = read_source(&core.source);
        core.anchor_media_ns = core.frozen_ns;
        adopt_anchor(&mut core, &reading);
```
What is wrong (observation, not a bug): `resume` after a counter reset while
paused silently adopts the new counter domain (adopt_anchor overwrites
last_source_ns), whereas the same reset while running is a typed
Discontinuity. Deliberate per the module doc (:22-25 "it does not trust the
source to have frozen") but the asymmetry is undocumented at the resume site.
Confidence: MEDIUM — Verification: read of adopt_anchor (:656-668) and the
counter_regression test (:955-982).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Display for ClockMaster | 111-121 | REAL | — | tested backends.rs:2111-2115 |
| Source::master | 260-283 | REAL | — | exhaustive 5-way |
| Core::extrapolate | 365-401 | REAL | — | checked mul/div/add, i128::MAX saturation |
| naive_position | 403-409 | REAL | — | saturating, diagnostic only |
| checked_div_round | 414-427 | REAL | — | ties away from zero; ± verified by hand |
| position_nanos | 435-441 | REAL | — | checked both ways |
| saturate_position | 443-446 | REAL | — | clamp then exact cast |
| clock_state | 450-458 | REAL | — | Ended outranks Paused |
| MediaClock::audio_master | 479-490 | REAL | — | reachable (pipeline_host.rs:1605) |
| MediaClock::system_monotonic | 499-513 | REAL | — | reachable (pipeline_host.rs:1630) |
| MediaClock::build | 515-534 | REAL | — | unusable first reading holds, never guesses |
| MediaClock::pause | 542-550 | REAL | — | no-op under Ended |
| MediaClock::resume | 558-568 | REAL | S4 | F-004 asymmetry |
| MediaClock::is_paused | 573-575 | REAL | — | Ended reports false (documented) |
| MediaClock::set_generation | 581-583 | REAL | — | no epoch bump (documented) |
| MediaClock::anchor | 586-602 | REAL | — | introspection; tested :1156-1169 |
| MediaClock::lock_core | 607-612 | REAL | — | poison recovery; no panic sources inside CS |
| snapshot_locked | 618-629 | REAL | — | shared by snapshot/mark_ended/seek_back |
| anchor_core | 633-650 | REAL | — | zeroed source anchors |
| adopt_anchor | 656-668 | REAL | — | keeps old anchor on Unusable |
| read_source | 674-700 | REAL | — | i128 conversion, zero-rate guard |
| adopt_if_awaiting | 706-720 | REAL | — | only reachable via 3.B2 switches (dormant in product) |
| now_locked | 725-744 | REAL | — | monotone clamp; updates baseline on sound reads |
| try_now_locked | 748-769 | REAL | S4 | F-002 found value |
| PlaybackClock::now/try_now/master | 771-786 | REAL | — | hot path, one lock |
| PlaybackClock::reanchor | 788-805 | REAL | — | epoch bump, clamp reset, freeze carried; Ended survives |
| PlaybackClock::set_rate | 807-823 | REAL | — | re-anchors at current pos, no jump (tested :1056-1068) |
| PlaybackClock::snapshot | 825-828 | REAL | — | |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| anchor_arithmetic_scales_by_rate (:901) | YES | 5/4 rate over 4 counter-seconds = exact 5e9 |
| counter_regression_holds_position_never_backwards (:955) | YES | reset + partial advance must hold, try_now Discontinuity, re-anchor cures |
| sample_rate_change_is_a_discontinuity (:985) | YES | 48k→44.1k with same counts must hold |
| extreme_positions_saturate_monotonically (:1035) | YES | i64::MAX saturation, equal-at-worst, typed Overflow |
| set_rate_never_jumps_and_rescales (:1056) | YES | no-jump then 2x slope |
| pause_freezes_resume_continues_without_jump (:1081) | YES | source runs while paused; clock must not |
| send_sync_object_safe (:1172) | YES | trait-object compile check |
All pass in baseline. No rerun (cargo budget); asserts are on computed values.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "now() = anchor + counter_elapsed × rate; device counter the only trusted progress source" | docs/03-engine-plan.md §4.7(e) | REAL | extrapolate :388-399; test :891 |
| "wait-free (seqlock) readers" | docs/03-engine-plan.md §4.7(c) | FALSE for v0 (recorded deviation) | Mutex on every read; clock.rs:46-47 admits it |
| "set_rate(rate: f64)" sketch | docs/03-engine-plan.md §4.7(b) | Deviation, recorded | Rate + Result; clock.rs:54-56 |
| "Pause: now() freezes; resume continues from same media time" | docs/03-engine-plan.md §4.7(e) | REAL | pause/resume :542-568; test :1081 |
| "overflow saturates at i64 bound, monotone, try_now reports Overflow" | docs/03-engine-plan.md §4.7(f) | REAL | :443-446, :756-760; test :1035 |
| "virtual-clock determinism tests" | docs/03-engine-plan.md §4.7(f) | PAPER-ONLY in this crate | tests/virtual_time.rs does not exist (F-001) |
| "ClockMasterChanged events on fallback are the playback-engine's to emit" | clock.rs:85-88 | REAL (comment) but engine never emits | see backends.rs audit F-001 |

## 7. Dead / duplicated / unreachable code
`adopt_if_awaiting` and the `awaiting_anchor` flag are only set by the 3.B2
switch paths (backends.rs), which no production code reaches — dormant but
harmless (a plain 3.B1 clock has it permanently false). No other dead code.

## 8. Dependencies & flags
media-types only; std Mutex; no unsafe; no features. Rate validation funnels
through `validate_rate` (lib.rs:141-143 = `Rate::new`, rejects zero
components, u32 fields make negative denominators unrepresentable).

## 9. Open questions for the next agent
- Who paces the null sink's counter in real time? Answer for the record: the
  CLI settle pump (apps/cli/src/host.rs:703-750) renders quanta in a
  CPU-speed loop with no wall pacing (apps/cli/src/run.rs:7-19) — so in the
  shipped CLI the "audio-master" position is a step counter advanced at
  decode speed, not real time; the 19ms full-playback baseline
  (audit/runs/baseline/mp4-smoke.md) is the direct evidence. The
  system_monotonic fallback (InstantSource) IS real time — the two masters
  disagree in kind.
- video-engine's scheduler consumes `now()` — does it assume wall-time
  monotonicity that the step-counter master violates between renders?
