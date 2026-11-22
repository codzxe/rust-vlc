# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_14_short_media.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 245 / 3 `#[test]`s + assert_short_media_trail + play_short_file helpers |
| File verdict | **REAL** — the EOF-before-watermark race pinned on real tiny AVIs, a scripted 1-packet timeline, and the degenerate zero-packet edge |
| Reachable from shipped entry point? | Subject: real engine + real AVI demux/codecs for the two file cases (fixtures built by *documented byte surgery* from the committed parent — `build_short_avi`, header 14-21); scripted timelines use the suite's named fake demuxer. Test runs under `cargo test`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read of the full file. No cargo (static). |

## 1. One-paragraph verdict

The short-media scenario earns its pass criteria with a genuinely adversarial shape: the `assert_short_media_trail` helper (44-60) walks the *entire* state history and asserts both the terminal edge (`Playing → Ended` — never a terminal `Buffering`) and that every `Buffering` episode the session did enter was left *to `Playing`* — the exact "start with what exists" rule of doc 03 §4.6, checked on the observed trail rather than the final state alone. The two real-AVI cases (1-frame, 0.1 s) assert delivered-frame counts, duration ranges (33 000-33 400 µs and 99 900-100 100 µs — honest tolerance for frame-period rounding), one end-of-media, zero errors, and clean closes. The 1-packet timeline repeats the race at the demux seam, and the zero-packet test pins the degenerate edge honestly labeled "pinned observed behavior" (199-203): empty media still reaches a legal `Ended` with exactly one end-of-media and zero errors — an honest empty, never a hang.

## 2. What was checked

- The trail check inspects all windows (51-59), not just the last pair — a Buffering→Ended escape anywhere fails.
- Fixture provenance: derived from the committed parent by declared-count patching, explicitly *well-formed* (18-19) so the pass shape is a clean play-out — damage coverage is scenario 9's job (correctly separated).
- The `settle` assertion (85-89) allows `Playing | Ended` — correct for a clip that may drain during settle; the stuck-`Buffering` failure is named in the message.
- Zero-packet case asserts `frames_accepted == 0` — the negative space is pinned, not assumed.
- All three tests share the serialize_tests guard; per-test hang bounds (256/128/128).

## 3. Findings

None. (Checked: mock-of-the-subject — file cases are real-container; tautologies — none; overclaiming — the header scopes damage to scenario 9 and labels the zero-packet pin as observed behavior.)

## 4. Per-function table

| Item | Lines | Verdict | Notes |
|---|---|---|---|
| assert_short_media_trail | 44-60 | REAL | whole-trail §4.6 rule check |
| play_short_file | 65-126 | REAL | open/settle/duration/terminal/frames/EoM/errors/close with per-case names in every message |
| one_frame_and_tenth_second_clips_end_cleanly | 131-153 | REAL | two real AVIs + duration ranges |
| one_packet_timeline_ends_cleanly | 159-196 | REAL | the race at the fake seam |
| zero_packet_timeline_ends_cleanly_with_no_frames | 205-245 | REAL | degenerate edge pinned |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| one_frame_and_tenth_second_clips_end_cleanly | YES | counts, durations, trail shape, EoM uniqueness |
| one_packet_timeline_ends_cleanly | YES | same shape at the seam |
| zero_packet_timeline_ends_cleanly_with_no_frames | YES | terminal + zero frames + EoM + no errors — catches the hang/stuck-buffering regressions the scenario exists for |

UNVERIFIED by run (no cargo); W00 baseline consistency applies.

## 6-9

- Doc claim "a clip whose whole payload fits under the high-watermark must start with what exists" (4-9): implemented as the trail assertion — faithful.
- No dead code.
- Open question: `build_short_avi` lives in host.rs (not this agent's file) — its byte-surgery correctness underpins the two real-AVI cases; wave 16 should verify the patched headers are truly well-formed (the scenario asserts the *behavior*, which would catch malformed fixtures only if the demuxer rejects them).
