# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_07_av_sync.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2025-06-14 |
| Lines / functions | 108 / 4 (last_stamps, assert_sync, the test, consts) |
| File verdict | REAL — the A/V-sync scenario over the REAL engine and real AVI fixture (MJPEG+PCM parent), executed green; what it measures is delivery-interleaving drift at media-time checkpoints, a proxy for presentation sync |
| Reachable from shipped entry point? | N/A (test). The engine/demux/decoder stack under it is shipped code; the command layer is the scenarios_b LOCAL dispatch (see host.rs audit F-001 — served-set superset; scenario 07 itself uses only SessionOpen/settle/pump/Close, all CLI-served shapes). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p playback-engine --test scenarios_b -- scenario_07` -> `1 passed; 0 failed ... finished in 0.09s`, exit 0; baseline: same test ok in the 23-test suite (5.06s) |

## 1. One-paragraph verdict
A genuine, tight A/V-sync check: the real 2 s MJPEG+PCM parent plays end-to-end through the real engine; at the 25/50/75% media-time checkpoints (media time = the consumed-audio sample counter, the null-device truth) the last-delivered audio pts and last-delivered video pts must sit within 40 ms (one video frame + one audio quantum); at end-of-media the final stamps must agree within the same tolerance, exactly one `playback.end-of-media` may fire, and the error list must be empty. I executed it (0.09 s, pass). Two scope notes: `assert_sync` panics via `unreachable!` when a limb has delivered nothing (test-style acceptable), and the "sync" measured is delivery interleaving under the harness's no-wall-clock pump — real-time presentation drift (video-engine presenting against the audio-master clock) is not exercised by this scenario; the 40 ms figure borrowed from doc 03 §4.10 happens to be checkable on the delivery order, which is a necessary but not sufficient condition for presentation sync.

## 2. What actually works (verified)
- verified by `cargo test -p playback-engine --test scenarios_b -- scenario_07` -> `test scenario_07_av_sync::full_playback_keeps_av_stamps_within_tolerance_at_checkpoints ... ok`, exit 0.
- Mutation reasoning: if the engine delivered video limb frames far behind audio (e.g., a queue-priming bias), `drift <= TOLERANCE_NS` fails at the checkpoint with both stamps printed; a duplicated/missing EOM trips the `== 1` count; any pipeline error trips the empty-errors assert.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_07_av_sync.rs:44-49
Code:
```
fn assert_sync(host: &ScenarioHost, checkpoint: &str) {
    let (audio, video) = last_stamps(host);
    let (Some(audio), Some(video)) = (audio, video) else {
        unreachable!("{checkpoint}: both limbs have delivered frames");
    };
```
What is wrong: a limb-with-no-frames condition aborts via `unreachable!` panic rather than a descriptive `assert!` failure; if the video limb never delivered, the panic message is less actionable than an assert naming the limb. Test-only blast radius.
Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_07_av_sync.rs:3-13, 75-79
Code:
```
//! The parent plays end-to-end; at each media-time checkpoint (the
//! 25/50/75% marks of the declared 2 s), the last-delivered audio
//! stamp and the last-delivered video stamp are within one video
//! frame duration plus one audio quantum of in-flight grace
//! (≤ 40 ms — doc 03 §4.10's tolerance for the 30 fps / 30 chunk-per-second
//! interleaved parent)
```
What is wrong: the measured property is last-delivered-stamp distance at pump boundaries (delivery interleaving) under a sink that accepts frames the moment they decode — there is no wall-clock presentation, no video-engine scheduler in this composition. Doc 03 §4.10's subject is presentation drift; this scenario proves the necessary precondition (delivery order stays tight), not the presentation sync itself. The module doc is explicit about what is measured, so this is a scope note rather than a lie — but the scenario's name (and doc 10 §6.4 row 7, "A/V sync") promises more than the mechanism delivers.
Confidence: HIGH — Verification: read + host.rs determinism model (media time advances only via sink audio consumption).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DURATION_NS / TOLERANCE_NS consts | 21-26 | REAL | — | 2 s parent; 40 ms = video frame + audio quantum |
| last_stamps | 27-41 | REAL | — | reverse ring scan per limb; ring capped 1024 (host D-2) |
| assert_sync | 43-55 | REAL | S4 | F-001 unreachable!; abs distance ≤ tolerance with both stamps in the message |
| full_playback_keeps_av_stamps_within_tolerance_at_checkpoints | 58-108 | REAL | S4 | open → settle → 3 checkpoint pumps (position-driven, reached asserted) → run_to_terminal (Ended + pump count) → end-of-media sync + exactly-one-EOM + empty errors → clean close; F-002 scope |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| full_playback_keeps_av_stamps... | YES | drift/count/error/close assertions all bind to real engine behavior (mutation reasoning above); executed green |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "drift within doc 03 §4.10 tolerance at checkpoints" (doc 10 §6.4 row 7, quoted in main.rs table) | scenarios_b/main.rs:22 | PARTIAL | 40 ms enforced on delivery stamps (F-002); presentation sync untested here |
| "The engine never fabricates a stamp: every value here is the decoders' own pts" | scenario_07:14-15 | REAL | stamps come from the frame ring populated in ObservingSink::accept from decoder frames (host.rs:659-676) |

## 7. Dead / duplicated / unreachable code
- None.

## 8. Dependencies & flags
- commands/testkit + the host module. No features. Uses only CLI-served command shapes (SessionOpen/SessionClose) plus the host's pump API.

## 9. Open questions for the next agent
- Where IS presentation-sync tested (video-engine presenting against the audio-master clock under drift)? video-engine's suites (crates/video-engine/tests) are the likely home — if no test there measures presentation drift either, doc 03 §4.10 compliance is UNVERIFIED tree-wide.
- The `unreachable!`-free rewrite of assert_sync is a one-line fix for a future pass.
