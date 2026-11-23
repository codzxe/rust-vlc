# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_03_pause_resume.rs
| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-17 |
| Lines / functions | 113 / 1 (1 #[test]) |
| File verdict | REAL |
| Reachable from shipped entry point? | Test file over the real engine + command layer (`Command::PlaybackPause/Play` through the scenario host's `engine.poll()` loop); the pause/resume path is the same state machine the shipped hosts drive (apps/cli/src/host.rs pump). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | baseline cargo-test-workspace.txt:4392 (passes); sed scenarios_b/host.rs:1107-1112 (pump_pipeline = poll + drain, frame delivery happens on poll); sed docs/10-testing-ci-quality.md:683 (row 3). |

## 1. One-paragraph verdict
This scenario does exactly what doc 10 §6.4 row 3 demands — "zero samples while paused; resume at same media time" — through the real command layer and real engine. The freeze half is a genuine behavior pin: in this harness `pump_pipeline` IS the frame-delivery driver (host.rs:1107-1112: `engine.poll()` + drain — the pre-pause `pump_pipeline_until(≥10 samples)` proves frames flow on poll), so asserting the audio counter, the position, and the frame ring are all bit-frozen across 8 polls would fail if pause leaked consumption. The resume half pins media-time continuity (first post-pause stamp ≤ one quantum + 1 ms past the held stamp) and the media still ends honestly. The one weakness: the continuity check is an upper bound only and cannot detect a backward restart that re-emits audio (see F-001).

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/cargo-test-workspace.txt:4392` -> "scenario_03_pause_resume::pause_freezes_samples_and_resume_continues_at_same_media_time ... ok".
— verified by sed host.rs:1107-1112: `pump_pipeline` = `engine.poll(); drain_outputs();` — no time-advance bypass; poll drives delivery in this harness (pre-pause loop reaches ≥10 samples through it), so the paused freeze is a real pause proof, not a vacuous no-time-passed proof.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_03_pause_resume.rs:90-102
Code:
```
    let resumed = {
        let ring = host.sink().must().frame_ring();
        ring.iter()
            .filter(|(_, audio, _)| *audio)
            .map(|(_, _, pts)| *pts)
            .find(|pts| *pts > last_audio_pts)
```
What is wrong: The continuity assertion is one-sided. `find(|pts| *pts > last_audio_pts)` skips any post-resume frames whose stamps went BACKWARD, and the bound (`resumed - last_audio_pts <= quantum + 1ms`) has no lower bound. A pause bug that re-emitted the audio timeline from an earlier point (duplicate audio) would be invisible: the replayed frames at pts ≤ last_audio_pts are skipped, and the first crossing past last_audio_pts lands within the bound. Why it matters: "resume at the same media time" is the row's criterion, and backward re-emission violates it silently. Blast radius limited (no-duplication is pinned elsewhere by sample counters in the soak/session suites; a large backward jump would still break the ends-honestly timing). What correct looks like: assert the FIRST post-pause ring entry (by index after the pause boundary) has pts ≥ last_audio_pts, plus the existing upper bound. Confidence: HIGH — Verification: read.
(S4 also covers the hard-coded fixture knowledge `quantum = 2_000_000_000 / 60` — documented in-comment as the parent fixture's shape; acceptable for a scenario but silently coupled to the fixture.)

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| pause_freezes_samples_and_resume_continues_at_same_media_time | 20-113 | REAL | S4 | Open→play→pause (3-way frozen: counter/position/ring across 8 polls)→resume (continuity bound)→Ended→Close. F-001 one-sided bound. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| pause_freezes_samples… | YES for the freeze half (exact equalities on three independent observables across polls that demonstrably deliver while playing); YES for forward-jump resume (upper bound); NO for backward re-emission (F-001). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| Row 3: "pause / resume | virtual clock freeze | zero samples while paused; resume at same media time | transcript + sink counters" | docs/10-testing-ci-quality.md:683 | REAL | Test asserts exactly this; sink counters used; passes in baseline. |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
commands, testkit (Must, serialize_tests), scenarios_b host. No feature gates.

## 9. Open questions for the next agent
- None blocking. If anyone extends the pause scenarios (rate change, pause-during-buffering), lift F-001's bound into a two-sided check at the same time.
