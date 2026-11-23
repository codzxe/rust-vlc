# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_08_track_switching.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-23 |
| Lines / functions | 250 / 1 (single two-arm scenario test) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test binary `scenarios_b`, 23 tests, all green in baseline cargo-test-workspace.txt:4386-4390). Drives the real committed fixture `avi_mjpeg_2pcm.avi` (1 MJPEG + 2 PCM with deliberately distinct audio shapes) through the real command layer. Ran live: prebuilt `scenarios_b` binary filtered on `track_switching` -> 1 passed, 0 failed (exit 0). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `./target/debug/deps/scenarios_b-bee4b59599013918 track_switching` -> 1 passed (exit 0); `rg` over scenarios_b/host.rs helper bounds |

## 1. One-paragraph verdict
A real behavioral scenario over the committed multi-track fixture with sharp assertions: the honest 3-track list, the class-A switch receipt, the `tracks.changed` terminal's payload as the authoritative selection truth, the no-old-frames-after-the-boundary ring invariant (with the documented last-one grace), video continuity across the audio switch, and a deselect arm proving audio stops while video continues with zero errors. The three defects are all assertion-discipline: the captured switch generation is explicitly discarded without any check, two bounded `pump_pipeline_until` awaits ignore their return values (one creating a narrow vacuous-pass window), and the header narrates one continuous session while the code runs two (the deselect arm re-opens fresh, admitted only in an inline comment).

## 2. What actually works (verified)
— verified by running the prebuilt scenarios_b binary filtered to this scenario: `test scenario_08_track_switching::audio_track_switch_mints_generation_with_no_old_frames_after ... ok` (exit 0, 0.04s).
— verified the awaits are hang-proof: `ScenarioHost::pump_pipeline_until` is bounded by `AWAIT_PUMPS` (host.rs:1125-1133) and answers bool; `run_until_terminal(512)` is explicitly bounded and asserted `== Ended`.
— verified the terminal-payload claim chain the test relies on: the `tracks.changed` wire event exists in the emitted catalog (`ENGINE_EMITTED_ROWS = 15`, events.rs:638) and the events_contract twin asserts the generation match this scenario skips.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_08_track_switching.rs:82-85 and 180
Code:
```
    let generation_at_switch = match &switch.result {
        Ok(CommandOutcome::Accepted { generation, .. }) => *generation,
        other => unreachable!("the switch accepts: {other:?}"),
    };
...
    let _ = generation_at_switch;
```
What is wrong: The header (line 9) claims "the class-A receipt carries the post-bump generation" — the test captures that generation and then explicitly discards it (`let _ =`). Nothing here verifies it is post-bump (i.e., differs from the pre-switch generation), or that it matches the terminal's generation (the events_contract.rs twin, `track_selection_completes_with_the_changed_terminal`, does assert exactly that — this scenario re-proves none of it).
Why it matters: The scenario's name says "mints generation"; the mint is matched structurally (an `Accepted` outcome exists) but never checked numerically. A regression that stopped bumping the generation on switch would pass this file.
What correct looks like: Capture the open-generation (or read the receipt of a no-op command), then `assert!(generation_at_switch > generation_at_open)` and match the terminal's envelope generation.
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_08_track_switching.rs:65-71 and 201-207
Code:
```
    host.pump_pipeline_until(|host| {
        host.sink()
            .must()
            .frame_ring()
            .iter()
            .any(|(track, audio, _)| *audio && *track == FROM_TRACK)
    });
```
What is wrong: Both await results are ignored (the same file's other await at lines 101-106 does it correctly: `let changed = …; assert!(changed, …)`). The second one (deselect arm, lines 201-207: "wait for any audio frame") creates a vacuous-pass window: if audio never flowed in that fresh session, `audio_at_deselect` reads 0 and the later `assert_eq!(audio_accepted(), audio_at_deselect, "no audio frame after the deselect")` passes 0 == 0 — the deselect's real claim (audio STOPPED) is then unproven. (Total audio breakage would fail the first arm's `first_new` `.must()` at line 151-154, so the window is narrow — audio working in arm 1 but silent in arm 2.)
Why it matters: Awaits that can silently time out are exactly how scenarios rot into tautologies.
What correct looks like: `assert!(host.pump_pipeline_until(…), "the default audio track flowed");`
Confidence: HIGH — Verification: read + traced both arms' failure paths.

F-003 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_08_track_switching.rs:14-16 vs 177-187
Code:
```
//! switch (the unchanged kind's generation floor). A deselect
//! (`None`) stops the audio limb while video continues; the media
//! ends honestly.
...
    // The deselect shape on a fresh session: switch to None stops
    // the audio flow while video continues.
    let mut host = ScenarioHost::new();
```
What is wrong: The header narrates a single continuous session ("…A deselect (None) stops the audio limb while video continues; the media ends honestly"), but the deselect runs on a brand-new `ScenarioHost` after `SessionClose` — the first session's media already `Ended` before any deselect (line 138-143). The inline comment (177-179) admits the fresh-open shape; the header does not.
Why it matters: A reader crediting the file with "deselect mid-media on the same session" is misled — that path is untested here.
What correct looks like: Header: "…a deselect on a fresh session stops the audio limb while video continues" (or test the same-session deselect).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| audio_track_switch_mints_generation_with_no_old_frames_after (arm 1: switch) | 32-175 | REAL | S4 | Honest list, terminal payload, ring boundary invariant, video continuity — all behavioral over the real fixture. F-001 (discarded generation), F-002 (unasserted await #1). |
| same test (arm 2: deselect) | 177-250 | REAL | S4 | Audio-stop + video-continue + zero-error asserts on a fresh session. F-002 (unasserted await #2, vacuous window), F-003 (session-shape drift). |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| audio_track_switch_mints_generation… | YES for the core invariants | Wrong-track decode observable via the distinct audio shapes → ring boundary assert; missing terminal → `assert!(changed)`; missing new-track frames → `.must()` panic; video stall → `video_after > 0`; deselect-as-error → `count_events(playback.error) == 0` violated. NO for the generation mint (F-001) and the deselect's stop-claim in the narrow arm-2-silent window (F-002). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the class-A receipt carries the post-bump generation" | header line 9 | REAL-UNPROVEN here | Structural match only; value discarded (F-001). Proven by the events_contract twin. |
| "the sink's ring shows the NEW track's audio frames from the boundary forward with NO old-track audio frame after the first new one" | header lines 10-13 | REAL | Lines 150-162 assert exactly this. |
| "the video limb kept decoding through the audio switch" | header lines 13-14 | REAL | Lines 164-175. |
| "A deselect (None) stops the audio limb while video continues; the media ends honestly" | header lines 14-16 | PARTLY | Deselect stop + video continue proven (226-247) — but on a FRESH session, not the narrated continuous one (F-003); "the media ends honestly" refers to arm 1's `Ended` terminal (138-143). |
| "NOT asserted via tracks.list: … neither this local host nor the real CLI refreshes it on the switch terminal in v0" | lines 94-100 | REAL | The post-switch `TracksList` assert (131-135) checks only the count, consistent with the open-time-mirror caveat; the CLI-side claim matches apps/cli/src/host.rs's `track_snapshot` design (drain_outputs builds tracks only from `TracksChanged` engine events, host.rs:661-673). |

## 7. Dead / duplicated / unreachable code
- None beyond the discarded binding (F-001). The `[PROPOSAL]` CLI follow-up note (refresh the snapshot from `tracks.changed`) is an honest registered gap, not dead code.

## 8. Dependencies & flags
- commands (catalog types, envelope), testkit (Must, fixture_path, serialize_tests), crate::host (ScenarioHost, dispatch, count_events, MULTITRACK). Real fixture dependency: `tests/fixtures/generated/valid/avi_mjpeg_2pcm.avi` (committed, per scenarios.toml and demux fixtures). No feature gates.

## 9. Open questions for the next agent
- The same-session deselect-mid-media path (deselect while Playing, then continue) is untested here — the closest coverage is this file's fresh-session arm and scenario_07's A/V sync shapes. If track_switch.rs / track_switch_fixture.rs (already audited by other agents) do not cover it either, it is a real coverage hole in the shipped surface.
- `ScenarioHost` (tests/scenarios_b/host.rs, ~1200 loc) is the load-bearing harness for all 16 scenarios — I audited only the two helpers this scenario leans on (`pump_pipeline_until` bounded, `frame_ring`, `audio_accepted`, `settle`); the host itself needs its own audit if unassigned.
