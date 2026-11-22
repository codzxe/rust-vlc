# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_07_av_sync.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 109 / 1 `#[test]` fn + 2 helper fns |
| File verdict | SHALLOW (real delivery-lockstep check, but it is not the A/V *sync* doc 03 §4.10 describes) |
| Reachable from shipped entry point? | N/A (test file); drives the real engine + recording sinks |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:0 |
| Commands run | full-file read; cross-check of what "drift" is measured against doc 03 §4.10's presentation-error semantics (my scheduler.rs audit: `e_v = pts − now` at scheduler.rs:830) and the scenario host's sink shape (recording ring, not a presenter) |

## 1. One-paragraph verdict

The scenario plays the parent end-to-end and asserts, at the 25/50/75% checkpoints and at end-of-media, that the last-delivered audio PTS and last-delivered video PTS are within 40 ms, plus exactly one `end-of-media` and zero error events. As a pipeline-honesty test it is real: the values are the decoders' own stamps read from the recording ring, the checkpoints are reached on the consumed-audio position truth, and a gross reordering or limb starvation would fail it. But as "A/V sync" it measures the wrong thing: **stamp proximity of interleaved delivery**, not presentation-time sync. Doc 03 §4.10's tolerance is about the presentation error `e_v = pts − now` measured at the present against the audio-master clock (implemented in the video scheduler's SyncSample, my scheduler.rs audit) — a property this test cannot observe because the scenario host's sink records frames instead of presenting them on a clock. For a well-interleaved AVI, delivery lockstep is close to guaranteed by the container itself; the test is a delivery-ordering check wearing a sync badge.

## 2. What actually works (verified)

- The checkpoint mechanism (82): `pump_pipeline_until(position_us × 1000 ≥ target)` on the consumed-audio position — the honest media-time truth.
- `last_stamps` (29-42) reads the recording ring backwards for the latest per-kind stamp — no fabricated values.
- Zero-error-events and single-end-of-media asserts (95-104) make the run's cleanliness part of the contract.
- W00 baseline: green in the 1751-pass run — cited, not re-run (ADAPTATION).

## 3. Findings

### F-001 [S3] — The scenario is labeled "A/V sync (doc 03 §4.10 tolerance)" but measures delivery-stamp proximity, not presentation sync

Where: crates/playback-engine/tests/scenarios_b/scenario_07_av_sync.rs:1-12 (the claim) vs 29-56 (what is measured).

Code (7-11, verbatim):
```rust
//! The parent plays end-to-end; at each media-time checkpoint (the
//! 25/50/75% marks of the declared 2 s), the last-delivered audio
//! stamp and the last-delivered video stamp are within one video
//! frame duration plus one audio quantum of in-flight grace
//! (≤ 40 ms — doc 03 §4.10's tolerance for the 30 fps / 30 chunk-per-second
```

Code (50-55, verbatim):
```rust
    let drift = (audio - video).abs();
    assert!(
        drift <= TOLERANCE_NS,
```

What is wrong: `drift` here is the distance between the two limbs' most recent *delivered* PTS — a property of the demux interleaving plus decode pacing, nearly guaranteed by a well-formed interleaved AVI. Doc 03 §4.10's A/V-sync tolerance is the presentation error against the audio-master clock (`e_v = pts − now`, computed in the scheduler at crates/video-engine/src/scheduler.rs:829-839 and surfaced as `SyncSample`), which no recording sink can observe. The module doc's "every value here is the decoders' own pts" is honest about the data source but the headline ("A/V sync … doc 03 §4.10 tolerance") claims the stronger property. The real presentation-sync tests live elsewhere (video-engine scheduler suite; apps/cli/tests/sync_scenarios.rs over the audio-master clock).

Why it matters: doc 10 §6.4 row 7 ("instrumented sinks, full playback — drift within tolerance") can be cited as evidence that A/V sync works, when this test would pass even if the presentation clock drifted arbitrarily (it never consults a clock). Claim-ledger inflation: SHALLOW evidence for a REAL-titled capability.

What correct looks like: rename the scenario "A/V delivery lockstep", and have the true sync check (SyncSample error_ema bounds over the run) live in a host that presents against the clock — or note the split in the module doc.

Confidence: HIGH — Verification: read + cross-file comparison of the two drift definitions.

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `DURATION_NS`/`TOLERANCE_NS` consts | 23-26 | REAL-UNPROVEN | — | 2 s / 40 ms |
| `last_stamps` | 29-42 | REAL-UNPROVEN | — | recording-ring backwards scan |
| `assert_sync` | 45-56 | SHALLOW | S3 | F-001: stamp distance, not presentation error |
| `full_playback_keeps_av_stamps_within_tolerance_at_checkpoints` | 59-108 | SHALLOW | — | checkpoints + end-of-media + clean-run asserts |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `full_playback_keeps_av_stamps…` | YES — for delivery disorders | gross reordering, limb starvation, duplicated stamps break the 40 ms window; **clock drift does not** (no clock consulted) |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A/V sync … drift within doc 03 §4.10 tolerance at checkpoints" | module docs 1-3 | FALSE (as presentation sync) / REAL (as delivery lockstep) | F-001 |
| "The engine never fabricates a stamp" | module docs 11-12 | REAL | values read from the decoders' PTS in the ring |

## 7. Dead / duplicated / unreachable code

None.

## 8. Dependencies & flags

`commands`, `testkit`; the scenarios_b host. Always runs.

## 9. Open questions for the next agent

- apps/cli/tests/sync_scenarios.rs — the audio-master presentation-sync suite (governor-armed, test-only arming per my resample.rs audit F-001): it is the actual §4.10 evidence; wave 16 should weigh it above this scenario.
- doc 10 §6.4 row 7's wording — does it say "drift" unqualified? If so, the claim ledger should downgrade this row's evidence.
