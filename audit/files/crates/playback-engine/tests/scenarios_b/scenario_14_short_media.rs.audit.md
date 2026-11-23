# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_14_short_media.rs
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-23 |
| Lines / functions | 245 / 3 callables (3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test) — runs in the workspace lane: baseline cargo-test-workspace.txt:4406-4408 (all 3 ok) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## 1. One-paragraph verdict
A clean, real test of the doc 03 §4.6 short-media race at two seams: real tiny
AVIs built by genuine RIFF byte surgery on the committed parent (host.rs
build_short_avi walks LIST/movi/idx1 and patches declared counts — verified by
reading it), and scripted `timeline://` demux timelines for the 1-packet and
degenerate 0-packet edges. Every observable is pinned: trail must end
Playing→Ended, every Buffering episode must exit to Playing (never stuck, never
directly to Ended), exactly one end-of-media, zero error events, exact frame
counts (2 and 6), duration ranges (33.0-33.4 ms, 99.9-100.1 ms). This is the
test that exercises the `Buffering + EofReached → Playing` row my state.rs
audit flagged as the short-media escape — the row is real and this proves it
against the real engine. Only defect: a settle/drive asymmetry between the
three tests.

## 2. What actually works (verified)
- Baseline (:4406-4408): `one_frame_and_tenth_second_clips_end_cleanly ... ok`,
  `one_packet_timeline_ends_cleanly ... ok`,
  `zero_packet_timeline_ends_cleanly_with_no_frames ... ok`.
- Fixture construction verified real by reading host.rs:2162-2202:
  build_short_avi asserts the RIFF/LIST/movi/idx1 structure while walking it,
  keeps the first 2×frames chunks, records per-kind counts — this is honest
  byte surgery, not a fixture copy or a mock.
- Cross-reference: the trail law (:44-60) directly pins the state-machine rows
  `Buffering+EofReached → Playing` and `Playing+Drained → Ended`
  (state.rs:441-446, :512-517) against the real engine — the "stuck in
  Buffering" failure mode the module doc describes is exactly the wedge my
  state.rs analysis looked for and ruled out.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_14_short_media.rs:216-222 vs :176-177
Code:
```
    assert!(
        matches!(&open.result, Ok(CommandOutcome::Accepted { .. })),
        "the zero-packet timeline opens: {:?}",
        open.result
    );
    let (pumps, terminal) = host.run_until_terminal(128);
```
What is wrong: the two timeline tests are asymmetric — `one_packet_timeline`
calls `host.settle()` before driving (:176); `zero_packet_timeline` goes
straight from the open assert to `run_until_terminal(128)`, so its pump budget
also absorbs the transient-exit pumps and its `pumps` count is not comparable
with the sibling's. Harmless today (128 is generous), but the asymmetry invites
copy-paste drift. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| assert_short_media_trail | 44-60 | REAL | — | last pair (Playing, Ended); every Buffering episode exits to Playing |
| play_short_file | 65-126 | REAL | — | open accepted, settle, duration via MediaGetDuration, terminal Ended, 1× end-of-media, 0 errors, exact frames, trail law, close ok |
| one_frame_and_tenth_second_clips_end_cleanly | 131-153 | REAL | — | 2 frames/33.3 ms and 6 frames/100 ms, duration ranges pinned |
| one_packet_timeline_ends_cleanly | 158-196 | REAL | — | timeline://1x33333; 1 audio frame, clean trail |
| zero_packet_timeline_ends_cleanly_with_no_frames | 204-245 | REAL | S4 | empty media: 0 frames, 1 end-of-media, 0 errors — the honest-empty contract; F-001 asymmetry |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| one_frame_and_tenth_second_clips_end_cleanly | YES | a short clip wedged in Buffering fails settle (:85-89) or the trail law; a dropped frame fails the exact count; a bogus duration fails the range |
| one_packet_timeline_ends_cleanly | YES | exact single-frame delivery + trail |
| zero_packet_timeline_ends_cleanly_with_no_frames | YES | empty media must still reach Ended with exactly one terminal event and zero errors — a hang exhausts 128 pumps, an error-event leak fails |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "short media: edge cases per doc 03 §4.6 (Ended vs Buffering race)" | docs/10-testing-ci-quality.md §6.4 row 14 | REAL | the race asserted at both seams; baseline green |
| "EOF before watermark answers immediate Playing, then a quick drain to Ended — never a session stuck in Buffering" | scenario_14 doc :4-9 | REAL | trail law :44-60; the EofReached row exists (state.rs:441-446) |
| "an honest empty media, never a hang and never a stuck Buffering" | :198-203 | REAL | zero-packet test green in baseline |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
commands catalog, testkit, crate::host (ScenarioHost, TempDir byte surgery,
timeline:// scripted demux). Counting-allocator serialization per test. No
env knobs.

## 9. Open questions for the next agent
- The `timeline://<n>x<step>` URL grammar lives in host.rs — is its parsing
  pinned anywhere (a malformed timeline URL's typed refusal), or only the
  happy shapes used here?
- The real-AVI half builds clips of 1..=30 frame pairs (host.rs:2163-2166);
  only 1 and 3 are exercised. The upper end (30 pairs = 1 s, still under a
  plausible watermark?) is untested — where does the short-media escape stop
  being needed? (Pipeline watermark config, another agent's file.)
