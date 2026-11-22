# AUDIT: crates/media-clock/src/source.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 119 / 4 production items + 2 tests |
| File verdict | REAL — two injectable time-source traits plus the production `InstantSource`; all production-reachable |
| Reachable from shipped entry point? | YES: `InstantSource` is the wall-clock source of the CLI's video-only sessions (`wall_clock()` → `MediaClock::system_monotonic(Arc::new(InstantSource::new()), …)`, apps/cli/src/pipeline_host.rs:1632-1638) and the event-throttle default (playback-engine/src/events.rs:567-568); `SampleCounterSource` is implemented by the audio-output null sink (`sink.clock_source()`, pipeline_host.rs:1610) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 (zero — "what was checked" below) |
| Commands run | Read/Grep (quotes verified with unfiltered tools) |

## 1. One-paragraph verdict

Twenty-nine lines of contract documentation and ninety lines of
trait-plus-adapter, all correct. The design fact that matters for the
wave-3 question is stated at :21-28: the sample counter is monotonic
*while the device runs* and a regressing counter is answered by the
clock holding position and reporting `TimeError::Discontinuity` —
never a panic. `InstantSource::now_ns` converts `u128 → u64` by
saturation with the rationale in a comment (:83-85). Both in-file
tests are genuine monotonicity checks; the second documents that an
earlier cross-source comparison flaked ~1-in-6 and was rewritten —
honest test archaeology in place of a hidden ignore.

## 2. What actually works (verified)

— Reachability chains (Grep): `InstantSource::new` at
apps/cli/src/pipeline_host.rs:1636 (the no-audio clock) and
playback-engine/src/events.rs:568 (`default_throttle_source`);
`SampleCounterSource` consumed by `MediaClock::audio_master`
(pipeline_host.rs:1610, implemented on the null sink in audio-output).
— `now_ns` (:82-86): `u64::try_from(elapsed_ns).unwrap_or(u64::MAX)` —
no wrap, comment states the 584-year rationale.
— `InstantSource: Clone` — cloning copies the origin, so all clones
share one time base; correct for a monotonic source.

## 3. Findings

None. What was checked (per the no-clean-bill-without-proof rule):
- `SampleCounterSource::sample_count`/`sample_rate_hz` (:35-46): pure
  trait, contract documented (rate 0 = unusable, handled upstream);
  the panic-free promise lives in clock.rs (other agent's file —
  flagged as cross-file, not verified here).
- `MonotonicSource::now_ns` (:51-55): pure trait.
- `InstantSource::new`/`Default` (:66-79): both anchor at `Instant::now()`;
  `Default` delegates — no divergence.
- `now_ns` (:81-87): saturation conversion verified correct; no
  arithmetic overflow path.
- Tests (:93-118): both assert non-regression of consecutive reads —
  they would fail if the source regressed; they cannot flake (std
  `Instant` is monotonic; equality allowed).
- No panics, no `unwrap` on fallible I/O, no blocking, no threads.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SampleCounterSource` (trait) | 35-46 | REAL | — | contract: monotonic while running, frozen while paused |
| `MonotonicSource` (trait) | 51-55 | REAL | — | contract: never decreases |
| `InstantSource::new` / `Default` | 66-79 | REAL | — | anchored at construction |
| `MonotonicSource for InstantSource::now_ns` | 81-87 | REAL | — | u128→u64 saturating; comment honest |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `instant_source_is_monotonic_without_sleeping` | YES | three consecutive reads, `>=` asserted (a regression fails) |
| `instant_source_default_matches_new` | YES | both sources checked for non-regression; comment documents the flaky predecessor it replaced |

Not executed by me (no cargo); W00 workspace run green. UNVERIFIED
per-test.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the clock reads them inside its short internal lock … may not be I/O" | :12-14 | REAL (contract) | implementors: null sink atomics / `Instant` |
| regressing counter → hold + `TimeError::Discontinuity`, never panic | :23-28 | UNVERIFIED here | implemented in clock.rs `try_now` (other agent's file) |
| "Tests never use this — they inject virtual time" | :59-60 | REAL | engine tests use `ZeroMonotonic` (events.rs:585) / testkit virtual clocks; this file's own unit tests are the source's own, not engine tests |

## 7. Dead / duplicated / unreachable code

None.

## 8. Dependencies & flags

`std::time::Instant` only. No feature gates; ships in the CLI binary.

## 9. Open questions for the next agent

- Wave 3 A02 (clock.rs): confirm the `Discontinuity`-on-regression
  behavior this file's docs promise, and that `sample_rate_hz() == 0`
  is actually handled as "unusable" somewhere (the contract is stated
  here; the handling must exist in the clock).
