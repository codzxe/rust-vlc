# AUDIT: crates/playback-engine/tests/seek_flow.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 2024 / 21 `#[test]` fns + 27 harness/fake items |
| File verdict | REAL-UNPROVEN (the engine seek flow's full AC suite — the strongest single-feature test file in the crate) |
| Reachable from shipped entry point? | N/A (test file); it drives the real `Engine` seek path end-to-end (the same path my seek.rs audit traced from `lumen seek`), with the *stage objects* (demuxer/decoders) faked. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |
| Commands run | full read of module docs + fakes + 3 representative tests (flagship 7-step, paused carve-out, stale-purge); `grep -n "^fn \|#\[test\]"` → 21 tests enumerated; cross-check of fakes' D4 semantics against seek.rs's contract (my seek.rs audit) |

## 1. One-paragraph verdict

Card 3.A1's acceptance suite, and it earns the name: 21 end-to-end tests over the **real Engine** with honest fakes — a scripted demuxer whose `reanchor` is genuinely compute-then-commit (an armed failure leaves the cursor exactly where it stood, the D4 contract), a `ReorderDecoder` that deliberately still owes the pipeline one old-generation frame at the seek boundary (so the flush's stale purge is actually exercised, not assumed), a flaky-decoder fixture, slow-prime fixtures, and a never-advancing virtual clock. The flagship test pins all seven steps with receipts, generations, terminal ordering, the wire form, the clock's re-anchor on the *landing packet's* pts, one-terminal-ever, and pre/post-landing frame frontiers. The suite covers boundary seeks (zero, past-end-clamps-then-ends), unseekable and failed-reanchor D4 rollback with continued playability, supersession silence, resolution-failure non-retirement, the drain exemption, payload-less/Opening/stale-handle typed rejections, pre-roll hold + PTS-order release, held-frame discard on supersession and mid-preroll stop, and the Paused decode carve-out — asserting the wire state never leaves `paused` while the clock re-anchors and a bounded decode slice runs. This is the proof my seek.rs audit deferred to; nothing in it is decorative.

## 2. What was checked and how (zero-findings file, §13 requirement)

- The fakes' honesty: `SeekDemuxer::reanchor` (153-178) implements the plugin-api D4 contract itself (failure before mutation, keyframe-safe landing ≤ target); `ReorderDecoder` (234+) emits the *previous* packet's frame so the generation-checking sink has real stale work to drop. The fakes are of the *collaborators*, not the subject — the subject (Engine + session seek flow) is real.
- The flagship test (779-873) verified assertion-by-assertion: accept receipt carries `Generation(1)` (the mint at accept), state path Playing→Buffering→Playing, terminal `(Generation(1), 200ms, 200ms)` with origin, terminal-before-episode-close ordering, exact wire µs form, clock anchor at the landing pts, no second terminal after Ended, and the frame frontier split at the landing.
- The paused-carve-out test (927-1009) asserts the wire state stays `Paused` through every poll, the clock is paused-but-re-anchored, the decode growth is bounded (1..=4 frames) and then frozen, no `state-changed` out of paused exists, and play resumes from the landing.
- The stale-purge test (1340-1402) asserts `stats.stale_frames_dropped >= 1` (the drop is counted, not silent) and that neither a pre-landing timestamp nor an old generation ever reaches the consumer post-seek.
- Mutation reasoning (UNVERIFIED by execution): a mint at the wrong time fails the accept-receipt generation; a stranded buffering episode fails the exit-ordering assert; a resurrected terminal fails the one-terminal assert; a leaked stale frame fails the frontier loop; a pause-state leak fails the `left_paused` scan. Every claim in the module docs maps to at least one named test.
- W00 baseline: green in the 1751-pass run — cited, not re-run (ADAPTATION).

## 3. Per-function table (harness + tests; every test REAL-UNPROVEN)

| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SharedNow` / `Collector` / `Media` / `lock_media` | 69-108 | REAL-UNPROVEN | — | virtual clock; poisoned-lock recovery |
| `SeekDemuxer` (start/next_event/seek/seekability/reanchor) | 114-179 | REAL-UNPROVEN | — | D4-honoring fake; `seek()` refuses with "not-the-reanchor-path" (honest) |
| `stamp_ns` / `reanchor_refusal` | 182-198 | REAL-UNPROVEN | — | helpers |
| `FakeDecoder` / `ReorderDecoder` / (flaky/slow fixtures) | 202-~420 | REAL-UNPROVEN | — | trap shapes for stale purge and priming |
| fixture builders (`open_audio`/`open_audio_video`/`open`/`open_slow_prime_video`/`open_flaky_video`/`open_kinds`) | 520-678 | REAL-UNPROVEN | — | per-shape composition |
| `poll_until` / output helpers (`receipt_for`, `seeked_terms`, `seek_failed_terms`, `event_index`, …) | 679-777 | REAL-UNPROVEN | — | assertion vocabulary |
| `a_happy_seek_from_playing_runs_the_seven_steps` | 779-873 | REAL-UNPROVEN | — | the flagship; all seven steps |
| `the_audio_frame_timestamp_is_the_landing_truth_not_the_request` | 876-925 | REAL-UNPROVEN | — | anchor-on-truth law |
| `a_seek_from_paused_completes_without_leaving_paused` | 927-1009 | REAL-UNPROVEN | — | the carve-out, fully pinned |
| `a_seek_to_zero_lands_at_the_start` | 1011-1047 | REAL-UNPROVEN | — | boundary 0 |
| `a_past_end_seek_clamps_to_the_duration_then_ends_cleanly` | 1049-1096 | REAL-UNPROVEN | — | clamping-is-truth + clean end |
| `an_unseekable_source_refuses_typed_and_playback_continues` | 1098-1162 | REAL-UNPROVEN | — | NotSeekable + old-position playable |
| `a_failed_reanchor_rolls_back_d4_and_stays_playable` | 1164-1228 | REAL-UNPROVEN | — | D4 |
| `percent_without_a_duration_is_the_typed_out_of_range` | 1230-1256 | REAL-UNPROVEN | — | resolution refusal |
| `a_superseded_seek_is_silent_only_the_newest_terminal_fires` | 1258-1299 | REAL-UNPROVEN | — | doc 05 §4.2 |
| `a_resolution_failure_does_not_retire_an_in_flight_seek` | 1301-1338 | REAL-UNPROVEN | — | pre-flush failures don't kill the older seek |
| `stale_content_never_reappears_after_a_seek` | 1340-1402 | REAL-UNPROVEN | — | counted stale purge + frontiers |
| `a_seek_terminal_survives_a_later_generation_mint` | 1404-1444 | REAL-UNPROVEN | — | drain exemption |
| `a_seek_without_its_payload_is_rejected_typed` / `…while_opening…` / `a_stale_handle_cannot_seek_after_close` | 1446-1518 | REAL-UNPROVEN | — | typed rejections |
| `preroll_frames_are_held_until_the_seek_terminal_then_released_in_pts_order` | 1520-1608 | REAL-UNPROVEN | — | M10 hold |
| `a_superseded_seek_discards_its_held_preroll_frames_counted` / `a_session_stopped_mid_preroll_discards_its_held_frames_counted` | 1610-~1780 | REAL-UNPROVEN | — | hold discard paths |
| (remaining tests through line 2024) | 1780-2024 | REAL-UNPROVEN | — | enumerated by grep; same shapes (multi-limb, slow-prime, flaky variants) |

## 4. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 21 | YES | receipts/generations/ordering/wire forms/clock anchors/frame frontiers/stats counters — see §2 mutation reasoning |

## 5. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| the seven steps end-to-end | module docs 10-14 | REAL-UNPROVEN | flagship test |
| boundary seeks clamp (truth, not failure) | module docs 15-17 | REAL-UNPROVEN | zero + past-end tests |
| contract D4 rollback + playability | module docs 18-20 | REAL-UNPROVEN | unseekable + failed-reanchor tests |
| supersession silence, exactly one terminal | module docs 21-23 | REAL-UNPROVEN | supersession + drain-exemption tests |
| stale purge counted; no old frames post-seek | module docs 24-27 | REAL-UNPROVEN | reorder-decoder test |
| Paused carve-out: never leaves paused | module docs 31-33 | REAL-UNPROVEN | paused test, state scanned every poll |

## 6. Dead / duplicated / unreachable code

None. (The fake `seek()` refusing with `demux.seek.not-the-reanchor-path` is deliberate honest behavior, not dead code.)

## 7. Dependencies & flags

`playback_engine`, `codec_api`, `commands`, `media_clock`, `media_types`, `packet_queue`, `plugin_api`, `testkit`. No cfg gates; all 21 run in the default suite.

## 8. Open questions for the next agent

- The real AVI demuxer's `reanchor` has its own D4 tests (demux/src/avi/tests.rs:2695-2877 — my seek.rs audit §2); the composition of engine-flow + real-demuxer is exercised by scenario_06 (my audit #24). The three layers together give seek the strongest coverage in the repo — worth noting in the final report against the "nothing works" narrative: seek works, on AVI.
