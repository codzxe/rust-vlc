# AUDIT: crates/media-clock/src/source.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-21 |
| Lines / functions | 119 / 9 (2 traits + 3 methods, 1 struct + 3 impls/methods, module docs; 2 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (both seams on shipped lanes: `SampleCounterSource` — implemented by `crates/audio-output/src/counter.rs:84` (`SampleCounter`), fed to `MediaClock::audio_master` at apps/cli/src/pipeline_host.rs:1605-1606; `MonotonicSource`/`InstantSource` — `wall_clock()` constructs it at pipeline_host.rs:1629-1634, plus `playback-engine/src/events.rs:567` `default_throttle_source()`) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | `rg -n "InstantSource" apps/ crates/` (production users: pipeline_host.rs:1631, playback-engine/events.rs:567; clock tests use fakes — claim checked) · `rg -n "impl SampleCounterSource"` (audio-output counter.rs:84) · baseline log: `source::tests::instant_source_*` 2/2 ok (:3283-3284) inside media_clock's 92/92 |

## 1. One-paragraph verdict
The smallest file in my set and the cleanest: two well-documented injection traits (non-blocking contract, monotonicity contract, zero-rate-is-unusable semantics — each upheld by the consuming clock code verified in my clock.rs part1 audit), plus a 25-line `InstantSource` whose only arithmetic is a documented u128→u64 saturation. Both seams are on shipped lanes and executed by the baseline. The single finding is a doc sentence contradicted by the file's own test module: "Tests never use this — they inject virtual time instead" sits eleven lines above two tests that use `InstantSource` to test `InstantSource`.

## 2. What actually works (verified)
— Both tests passed in the no-fail-fast baseline: `source::tests::instant_source_is_monotonic_without_sleeping`, `source::tests::instant_source_default_matches_new` (`orchrun-test-workspace-nofailfast.log.txt:3283-3284`; media_clock suite 92/92 at :3204-3260).
— Shipped execution: audio-less media lanes read `InstantSource::now_ns` through `MediaClock::system_monotonic` (pipeline_host.rs:1629-1634); audio lanes read `SampleCounterSource` through `audio_master` (:1605-1606); the events throttle uses it too (playback-engine/src/events.rs:567).
— Cross-file contract upheld (verified in my clock.rs#part1 audit): `sample_rate_hz() == 0` → `Reading::Unusable` → clock holds + reports Discontinuity (`broken_source_is_unusable_not_panic` passed); counter regression → hold-not-backwards (`counter_regression_holds_position_never_backwards` passed).
— `now_ns` saturation: `u64::try_from(elapsed().as_nanos()).unwrap_or(u64::MAX)` — never wraps, comment states the 584-year rationale; the saturated-wall-source path is itself tested in backends (`saturated_u64_wall_source_is_handled`, passed).

## 3. Findings
### F-001 [S4]
Where: crates/media-clock/src/source.rs:57-60 vs :89-118
Code (source.rs:57-60):
```
/// The production [`MonotonicSource`]: `std::time::Instant` elapsed
/// since construction (monotonic by std's guarantee on all supported
/// platforms). Tests never use this — they inject virtual time instead
/// (doc 03 §8, "virtual-clock determinism").
```
Code (source.rs:97-98):
```
        let source = InstantSource::new();
        let a = source.now_ns();
```
What is wrong: the doc claims "Tests never use this", and the file's own `#[cfg(test)]` module (:89-118) constructs `InstantSource` twice to test it. The claim is true for the clock's behavioral tests (clock.rs/backends.rs use `VirtualClock`-backed fakes and a saturated custom source — verified by grep), but as written it is contradicted eleven lines below by this file's own tests.
Why it matters: doc-precision only — a reader auditing test determinism is told no test touches `InstantSource`, then finds two that do (correctly, to test the type itself).
What correct looks like: "The clock's behavioral tests never use this — they inject virtual time (doc 03 §8); this type's own smoke tests are the exception."
Confidence: HIGH — Verification: read both sites; grep of clock.rs/backends.rs test modules for `InstantSource` (doc mentions only).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-28 | REAL | — | the non-blocking and monotonicity contracts are each upheld by consuming code + passing tests (§2) |
| `trait SampleCounterSource::sample_count` | 35-39 | REAL | — | prod impl audio-output/counter.rs:84; monotonic-while-running contract tested via clock regression test |
| `SampleCounterSource::sample_rate_hz` | 41-45 | REAL | — | `> 0` contract; zero → Unusable verified in clock.rs (test passed) |
| `trait MonotonicSource::now_ns` | 51-55 | REAL | — | monotonic-period contract; prod impl below + fakes |
| `InstantSource` + `new` + `Default` | 61-79 | REAL | — | origin at construction; Default==new (tested); Clone shares the origin (same timeline — sensible, undocumented but harmless) |
| `MonotonicSource for InstantSource::now_ns` | 81-87 | REAL | — | saturating u128→u64, documented; monotonic by std guarantee |
| tests (2) | 93-118 | REAL | — | sleep-free monotonicity smokes; the second test's comment honestly records the flaky original and the Lead fix |

## 5. Test honesty (2 tests, both passed — baseline :3283-3284)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `instant_source_is_monotonic_without_sleeping` | YES (for real regressions) | three consecutive reads asserted non-decreasing — a wrapping/random impl fails; a constant impl passes but is not a plausible mutation of `Instant::elapsed` |
| `instant_source_default_matches_new` | YES (same class) | per-source pair-wise non-regression; the comment documents why the old cross-source comparison flaked (1-in-6) and was rewritten — an honest test-evolution record, not evidence laundering |
| Gap | — | no test exercises the u64::MAX saturation branch (would need a 584-year uptime fake — the backends suite covers the saturated-wall path with a custom source instead) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Both traits must be non-blocking: the clock reads them inside its short internal lock" | source.rs:12-14 | REAL | InstantSource::now_ns is a pure elapsed read; audio-output counter answers from an atomic (per its doc); clock critical sections verified in clock.rs part1 |
| "A counter that regresses ... the clock detects it, holds ... reports Discontinuity" | source.rs:21-27 | REAL | clock.rs extrapolate + regression test passed |
| "MonotonicSource::now_ns must be monotonic, period" | source.rs:28 | REAL (contract) | InstantSource upholds; fakes upholds |
| "Tests never use this — they inject virtual time instead" | source.rs:59-60 | FALSE (in-file) | F-001: this file's own tests use it; true only for the clock's behavioral suites |

## 7. Dead / duplicated / unreachable code
- None. Both traits have production implementations (audio-output counter; InstantSource at two shipped call sites), and every item is consumed.

## 8. Dependencies & flags
- std only (`std::time::Instant`). No unsafe, no feature gates. Built + tested in baseline (media_clock 92/92 + 2 source tests).

## 9. Open questions for the next agent
- audio-output wave (counter.rs): does the atomic device counter genuinely freeze while paused (the `sample_count` doc's "frozen while it is paused") — the clock's pause-correctness leans on it.
- playback-engine events.rs auditor: `default_throttle_source()` uses InstantSource — wall-time throttling in tests that inject it, or ambient time in shipped paths? (Determinism question for the event outbox.)
