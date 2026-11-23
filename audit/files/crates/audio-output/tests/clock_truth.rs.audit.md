# AUDIT: crates/audio-output/tests/clock_truth.rs
| Field | Value |
|---|---|
| Auditor | W08-A02 |
| Date | 2026-11-15 |
| Lines / functions | 136 / 1 test fn (+ closure `drive_tick`) |
| File verdict | REAL (honest seam test; executed fresh: 1 passed; production components, no mocks of the thing tested) |
| Reachable from shipped entry point? | Test file — runs in `cargo test -p audio-output`; the components it drives (NullBackend, SampleCounter, MediaClock::audio_master) are the same production types the shipped CLI wires (apps/cli/src/pipeline_host.rs:1598-1609) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p audio-output --test clock_truth` → `1 passed; 0 failed` (fresh run, 0.02s); sed verification of constants in tests/common/mod.rs:224-228 |

## 1. One-paragraph verdict
The hostile focus was "clock_truth test honesty" — the test is honest. It opens the production `NullBackend` in trait mode, hands the session's real `SampleCounter` to the real `media_clock::MediaClock::audio_master`, then drives 60 ticks of write→render and demands exact rational results: 44 100 instants must land on exactly 1_000_000_000 ns (which requires exact rational scaling, since 1e9/44100 is not an integer), pause must freeze the clock, resume must continue to exactly 2 s with no discontinuity, and the final ledger (sample_count == 88 200, buffered == 0, underruns == 0) must close. Nothing is mocked: the null device is the production adapter, not a test fake, and the test would fail under every counter/clock mutation I tried by reasoning. The single limitation: everything runs on one thread, so the atomicity/ordering half of the "truth source" contract is invisible to this suite (see ring.rs audit F-002 — shared root cause).

## 2. What actually works (verified)
— `cargo test -p audio-output --test clock_truth` → `test the_device_sample_counter_drives_the_audio_master_clock ... ok; 1 passed; 0 failed`.
— Constants verified: `TONE_RATE = 44_100`, `TONE_SAMPLES_PER_TICK = 1470` (44100/30), `TONE_TOTAL_SAMPLES = 88_200` (tests/common/mod.rs:224-228) — 30 ticks × 1470 = 44 100 instants = 1 s at 44.1 kHz; the 1 s/2 s asserts are exact-rational checks, not approximations.

## 3. Findings
F-001 [S4]
Where: crates/audio-output/tests/clock_truth.rs:86-91 (whole-file property)
Code:
```
    let mut last = clock.now().ticks;
    for tick in 0..ticks / 2 {
        drive_tick(&mut backend, &mut device, tick);
        let now = clock.now();
        assert!(now.ticks >= last, "clock regressed at tick {tick}");
        last = now.ticks;
    }
```
What is wrong: the test drives sink-writes and device-renders on the SAME thread, so the counter update and the clock read are sequentially ordered by construction. The doc 03 §4.7 contract this test is named after ("device sample counters (read atomically); readers (`now()`) are wait-free", docs/03-engine-plan.md:544) is about CONCURRENT readers; this suite cannot detect an ordering regression (e.g. `Relaxed` counter publishes) because there is no concurrent observer. Also no rate≠1/1 and no anchor≠0 variant — the conversion math is proven only at rate 1 anchored at 0. Why it matters: it is the AC cited in the feature matrix as the clock evidence (docs/feature-matrix.toml:2361 "AC suites ... clock_truth"); its scope is narrower than its name. What correct looks like: one threaded variant (writer thread + clock-reader thread asserting monotonic reads), one rate=3/2 and one non-zero-anchor case. Confidence: HIGH — Verification: read + crate-wide rg (zero `thread::spawn` in crates/audio-output).

## 4. Per-function table (test file)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| the_device_sample_counter_drives_the_audio_master_clock | 26-136 | REAL | S4 | full honesty analysis below; executed 1/1 |
| drive_tick (closure) | 57-81 | REAL | — | writes 1470 stereo frames, renders them, asserts `fully_served()` each tick |

## 5. Test honesty
| Would it fail if code were broken? | Mutation | Reason |
|---|---|---|
| YES | counter never advances | clock stuck at 0 → 1 s assert fails |
| YES | counter counts bytes/samples instead of frames | 2× error → 1 s assert lands at 0.5 s or 2 s |
| YES | clock rounds instead of exact rational (1e9/44100 non-integer) | 44 100 instants ≠ exactly 1_000_000_000 ns |
| YES | pause does not no-op renders | `stats == RenderStats::default()` assert fails |
| YES | resume double-counts / counter rewinds | 2 s assert or `now.ticks >= last` fails; `sample_count == 88_200` fails |
| YES | ring drops/dups frames | `fully_served()` per tick, `buffered_frames()==0`, `sample_count()==TONE_TOTAL_SAMPLES` fail |
| NO | counter publishes with weak ordering under concurrency | single-threaded drive — ordering invisible (F-001) |
Verdict: not a tautology; no mock of the thing tested; the docstring's four claims (rate, monotonic, freeze, no discontinuity) are each directly asserted.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the audio output's sample counter is the media-clock's truth (doc 03 §4.7)" | clock_truth.rs:1-5 | REAL | test executed; quote matches docs/03-engine-plan.md:553 |
| "the production audio-output adapter, not a test fake" | clock_truth.rs:12-13 | TRUE | NullBackend/SampleCounter/MediaClock are the production types (null.rs, counter.rs, media-clock crate), not test doubles |
| matrix platform-aout-cpal evidence: "AC suites sine_sink_hash/callback_budget/clock_truth" | docs/feature-matrix.toml:2361 | REAL for the seam claim | this test passes against production types; note the "output" is the null device — consistent with the row's own "partial-seam, not done" note |

## 7. Dead / duplicated / unreachable code
None in this file. (Shared `common/mod.rs` is W08-A09's assignment; only the TONE_* constants and no sine hashing are used here — the ramp payload at lines 62-66 deliberately ignores the sine plan, as documented.)

## 8. Dependencies & flags
dev-deps: audio_output (path), media-clock, media-types, testkit (Must). No features. No cfg gates. Runs in the standard `cargo test -p audio-output` lane (baseline: 4744 passed workspace-wide).

## 9. Open questions for the next agent
- media-clock crate auditors: does MediaClock itself have the rate≠1/anchor≠0/concurrent-reader coverage this seam test lacks? If not, the "only trusted progress source" contract is only ever proven at rate 1, anchor 0, single thread.
