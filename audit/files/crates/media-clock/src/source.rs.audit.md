# AUDIT: crates/media-clock/src/source.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2025-06-14 |
| Lines / functions | 119 / 7 (2 traits, InstantSource + 2 impls, 2 tests) |
| File verdict | REAL — the injectable-time-source vocabulary of the shipped clock; both traits and the production source are wired into shipped binaries |
| Reachable from shipped entry point? | YES, both halves. (1) `SampleCounterSource` → implemented by crates/audio-output/src/counter.rs (production), consumed via `MediaClock::audio_master(sink.clock_source(), ...)` in apps/cli/src/pipeline_host.rs:1605 (every audio session in the shipped CLI). (2) `MonotonicSource`/`InstantSource` → apps/cli/src/pipeline_host.rs:1629-1637 `wall_clock` builds `MediaClock::system_monotonic(Arc::new(InstantSource::new()), ...)` (every audio-less session); also the default event-throttle source in playback-engine events.rs:567. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/media_clock-7012bdf535a3032b source::` -> 2 passed, exit 0 (prebuilt lib test binary); reachability sweeps (pipeline_host.rs:557/1605/1629 cited) |

## 1. One-paragraph verdict
Small, clean, and fully shipped: the two truth-source traits (sample-counter for audio-master, monotonic for the audio-less fallback) plus the production `InstantSource` with an honest saturating u128→u64 conversion ("a wrap would be a backwards jump after ~584 years; saturation is the honest cap"). The trait contracts document the hard parts — non-blocking reads under the clock's short lock, monotonic-while-running with regression handled as a detectable discontinuity rather than a panic, and sample_rate 0 treated as unusable. Both unit tests pass (direct binary run). The only note is that the traits' contracts (non-blocking, monotonic) are conventions the compiler cannot check — every impl I sampled (audio-output counter.rs, the CLI's clock_source, test fakes) honors them, but a blocking impl would stall the clock lock with no type-level guard.

## 2. What actually works (verified)
- verified by `./target/debug/deps/media_clock-7012bdf535a3032b source::` -> `2 passed; 0 failed` (both monotonicity tests).
- verified by grep: shipped CLI wiring — pipeline_host.rs:1605 (`audio_master(sink.clock_source(), ...)`), 1629-1637 (`wall_clock` over `InstantSource`), 557 (`clock_source() -> Arc<dyn SampleCounterSource>`); audio-output/src/counter.rs implements `SampleCounterSource`.
- Hand-checked `now_ns`: `u64::try_from(elapsed_nanos).unwrap_or(u64::MAX)` — saturates, never wraps; `Instant` monotonic by std guarantee.

## 3. Findings
F-001 [S4]
Where: crates/media-clock/src/source.rs:12-17, 51-55
Code:
```
//! Both traits must be **non-blocking**: the clock reads them inside its
//! short internal lock (doc 03 §4.7 c — the lock is never held across
//! I/O, so the reads themselves may not be I/O). The production
```
What is wrong: the non-blocking and monotonicity requirements are doc-only contracts — nothing in the type system enforces them (no `#[must_use]`-style guard exists for "is this read cheap"). All current impls honor them (audio-output atomic counter; test atomics), but a future impl doing I/O inside `now_ns` would hold the clock's lock across it with zero diagnostics. Acceptable for a trait seam; noted as an unguarded invariant.
Confidence: HIGH — Verification: read + sampled all impls found by grep.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SampleCounterSource (trait) | 35-46 | REAL | — | sample_count (monotonic while running, frozen paused) + sample_rate_hz (>0; 0 = unusable → discontinuity) |
| MonotonicSource (trait) | 51-55 | REAL | S4 | F-001 doc-only contract |
| InstantSource::new / Default | 66-79 | REAL | — | anchored at construction |
| MonotonicSource for InstantSource | 81-87 | REAL | — | saturating cast, no wrap |
| tests::instant_source_is_monotonic_without_sleeping | 93-103 | REAL | — | no-sleep monotonicity (equality allowed at ns resolution) |
| tests::instant_source_default_matches_new | 105-118 | REAL | — | documents the fixed flake (old test compared across sources, ~1-in-6 flake under load) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| both tests | WEAKLY | they guard monotonicity and usability, not rate semantics or the discontinuity path (those live in clock.rs's suite — 90 other tests in the same binary); adequate for this file's scope |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The clock's truth is injected, never ambient" | source.rs:3-8 | REAL | both shipped constructors take an injected source (pipeline_host.rs:1605/1630) |
| "a paused device freezes it"; regression → Discontinuity, never backwards | source.rs:21-28 | REAL | clock.rs:725-766 now_locked/try_now_locked implement hold-last-position + Discontinuity (read during recovery/host audits) |
| "Tests never use this [InstantSource]" | source.rs:59-60 | REAL | grep: every test impl is a SharedNow/FrozenNow/FakeMonotonic atomic; InstantSource appears only in production paths (events.rs default, CLI wall_clock) — one wasm note in events.rs:1555 confirms the split |

## 7. Dead / duplicated / unreachable code
- None. Everything in the file is shipped or shipped-consumed.

## 8. Dependencies & flags
- std only (Instant). No features. media-clock is a production dep of playback-engine (and the CLI uses it directly).

## 9. Open questions for the next agent
- clock.rs (the extrapolation/discontinuity machinery that consumes these traits) and audio-output/src/counter.rs (the production SampleCounterSource impl) belong to other agents — the rate-semantics comment in counter.rs ("44 100 ...") is worth a units check there.
- The wasm32 throttle split (worklog 9a13c39: ZeroMonotonic instead of InstantSource because std Instant::now panics on wasm32) — verify the web app actually relies on it (apps/web audit).
