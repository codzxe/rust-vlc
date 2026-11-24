# AUDIT: apps/cli/tests/native_seek.rs
| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-24 |
| Lines / functions | 606 / 18 (9 helpers + 9 tests) |
| File verdict | REAL (all 9 tests executed and passing; strong exact pins) |
| Reachable from shipped entry point? | N/A (test file) — it drives the SHIPPED entry: `lumen_dispatch` (apps/cli/src/lib.rs:43 "the one entry") through `CliHost` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `./target/debug/deps/native_seek-e90b17571d5abbd7` → 9 passed, exit 0, 1.92s; `rg -n 'fn reanchor' crates/demux/src/` → mp4 absent, avi/ogg/mkv present; NDJSON probes on `./bin/lumen --control-stdio` (AVI: position 1.0s after 50% seek; MP4: position 0.0 — see F-001) |

## 1. One-paragraph verdict
This is the strongest test file audited in this batch: every test asserts exact landings, state-trail edge counts, generation arithmetic, and accounting counters through the real dispatch entry, and all 9 pass (executed). The hostile-focus answer: in-session seek DOES work for AVI (MJPEG+PCM, 7 tests incl. a 5-seek storm) and OGG (Opus + Vorbis reanchor regression) — and I re-confirmed on the shipped release binary that an AVI 50% seek reports `time: 1000000, position: 0.5` with the session id surviving. MP4 remains broken exactly as Wave 5 recorded (S0): `crates/demux/src/mp4/` still implements no `reanchor`, and my NDJSON probe shows the wire accepting the seek (gen minted) while the position stays `0` / `0.0` — the failure rides the event stream (D4). The suite's gaps: no MP4 lane (the broken one), no MKV lane (reanchor implemented at demux/src/mkv/mod.rs:1026 but unproven at host level), and the shipped `--help` still advertises the OLD reopen composition this suite exists to forbid.

## 2. What actually works (verified)
— `./target/debug/deps/native_seek-e90b17571d5abbd7` → `test result: ok. 9 passed; 0 failed` (1.92s): epoch survival, generation minting, exact landings (1_000_000 / 266_667 / storm vector [500_000, 1_500_000, 1_000_000, 266_667, 1_766_667]), tail play-outs to exact durations (2_000_000 / 400_000 / 1_500_000 µs), typed refusals (Ended, no-session), accounting counters (scheduler_dropped_stale 15/16, queue_published 1/5, frames_accepted 90).
— Shipped-binary probe: `(open avi_mjpeg_pcm.avi; sleep 1; seek 50%; sleep 1; time; close) | ./bin/lumen --control-stdio` → `{"time":1000000,"position":0.5,...}` — native seek works end-to-end on the release binary for AVI.

## 3. Findings
F-001 [S3]
Where: apps/cli/tests/native_seek.rs:33-44 (fixture choice) — with crates/demux/src/mp4/ (no `reanchor`)
Code: `/// The AVI parent fixture (class A): MJPEG+PCM, 2.000 s, 320x240\n/// @30 fps, stereo s16le 44.1 kHz — every frame a keyframe, so the\n/// landing granularity is the 33.333 ms frame grid.\nfn parent() -> SourceSpec {`
What is wrong: The "native-seek acceptance suite" covers exactly two of the eleven wired containers (AVI, OGG). The container whose in-session seek is broken — MP4, Wave 5's S0 — has NO lane, so the suite cannot fail on the known defect; MKV's demuxer reanchor (demux/src/mkv/mod.rs:1026) is also unproven at this level. Why it matters: an acceptance suite titled generically that skips the broken lane gives false comfort; `rg -n 'fn reanchor' crates/demux/src/` shows mp4 absent, and my release-binary MP4 probe (open sample.mp4, seek 50%, sleep 2s, time) answers `{"time":0,"position":0.0}` — the seek is accepted (gen minted) but the position never moves. Wave 5's S0 re-confirmed by execution, 2026-11-24. What correct looks like: an MP4 lane asserting the typed failure (or the fix), plus an MKV/WebM lane. Confidence: HIGH — Verification: ran (probe + rg; the MP4-lane absence is read).

F-002 [S3]
Where: apps/cli/src/help.rs:57 (contradicts this suite's core invariant, module doc L10-14)
Code (help.rs:55-58): `(\n        "seek 90s | +10s | -1m | 50% | chapter 3",\n        "playback.seek",\n        "seek (v0: reopen at the target through the engine's real transitions)",\n    ),`
What is wrong: The user-facing `lumen --help` documents seek as the reopen composition ("v0: reopen at the target") — exactly the D-4/D-7 forbidden shape this suite pins as absent (`teardown_edges == 0`, `open_edges == 1`, session id survives). My release-binary probe shows the native flow (session stays 1, gen mints +1), so the help text describes neither the code nor the behavior. Why it matters: the shipped doc misdescribes the most-invariant-heavy behavior in the seek surface. What correct looks like: update help.rs to "in-pipeline seek; the session survives". Confidence: HIGH — Verification: ran (`--help` output) + read (help.rs:57).

F-003 [S4]
Where: apps/cli/tests/native_seek.rs:600-605
Code: `let (accepted, refused, stale_dropped) = host.audio_lane_counters().must();\n    assert_eq!(refused, 0, "frames refused: {accepted} accepted");\n    assert!(\n        stale_dropped < 3,\n        "straggler drops bounded: {stale_dropped} of {accepted} accepted"\n    );`
What is wrong: The Vorbis lane bounds straggler drops with `< 3` where the Opus twin asserts `== 0` (L533-534) and the AVI lanes assert exact counts — a weaker, non-deterministic pin (honestly commented as "may honestly drop accept-window stragglers"). A regression dropping 2 old-generation frames every seek would pass. Confidence: HIGH — Verification: read + ran (test passes; counter values not printed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parent` | 36-44 | REAL | — | AVI fixture path; exists (verified `ls`). |
| `dispatch` | 48-60 | REAL | — | Wire dispatch helper; `unreachable!` on failure — correct for a success-path helper. |
| `opened` | 63-75 | REAL | — | Open + settle through real path. |
| `seek` | 78-80 | REAL | — | One-liner through the wire. |
| `seek_completed_position` / `_positions` | 84-103 | REAL | — | Event-stream filters for the terminal. |
| `teardown_edges` | 110-122 | REAL | — | Counts the 3 forbidden reopen edges. |
| `open_edges` | 127-132 | REAL | — | Idle→Opening counter (reopen = >1). |
| `ogg_opened` | 473-491 | REAL | — | OGG fixtures exist (verified `ls`); the `../../crates/...` relative path resolves to repo root. |
| `native_seek_keeps_the_epoch_and_closes_the_accounting` | 143-200 | REAL | — | Executed; exact landing 1_000_000, counters 15/1/0/0/0, 90/45. |
| `native_seek_reports_the_actual_not_the_requested` | 206-232 | REAL | — | Executed; 266_667 terminal vs 266_666 clock (ns→µs truncation, explained). |
| `paused_seek_completes_and_resumes_from_the_landing` | 237-258 | REAL | — | Executed; paused carve-out + resume to 2_000_000. |
| `backward_seek_below_start_time_lands_and_plays_out` | 265-297 | REAL | — | Executed; watermark dies at re-anchor (SelectDemuxer::reanchor clears skip). |
| `seek_from_ended_refuses_typed_at_the_old_position` | 303-337 | REAL | — | Executed; D4 refusal shape, no fabricated terminal. |
| `seek_without_a_session_refuses_not_ready` | 340-359 | REAL | — | Executed; dispatch-table refusal. |
| `rapid_repeated_seeks_compose_in_pipeline` | 384-457 | REAL | — | Executed; 5-seek storm, exact landing vector, gen +5, queue_published 5. |
| `ogg_opus_in_session_seek_...` | 501-535 | REAL | — | Executed; regression pin (opus.head.missing closed), landing 0, end 400_000. |
| `ogg_vorbis_in_session_seek_...` | 542-606 | REAL | — | Executed; landing 0 (page-start fix), end 1_500_000; F-003 loose bound. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| native_seek_keeps_the_epoch... | YES | Exact session/gen/landing/counter pins; a reopen answers session=2, open_edges=2. Executed. |
| native_seek_reports_the_actual... | YES | Exact 266_667/266_666 pair. Executed. |
| paused_seek_completes... | YES | Paused-state invariant + landing + resume end. Executed. |
| backward_seek_below_start_time... | YES | Standing watermark would starve pre-roll → state != Ended. Executed. |
| seek_from_ended_refuses... | YES | Typed refusal + untouched position/session. Executed. |
| seek_without_a_session... | YES | Simple refusal. Executed. |
| rapid_repeated_seeks... | YES | Exact 5-landing vector + gen arithmetic + counters. Executed. |
| ogg_opus_... | YES | Regression: header-less reconfigure → terminal None → assert fails. Executed. |
| ogg_vorbis_... | YES | Same seam; landing pin catches the page-END regression. Executed. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "playback.seek runs the ENGINE's seven-step flow … in-pipeline, the session epoch surviving" | native_seek.rs:1-6 | REAL | 9/9 executed; release-binary AVI probe: session 1 survives, gen 0→1, position 1.0s. |
| "seek (v0: reopen at the target …)" | help.rs:57 | FALSE | Native flow observed (probe + suite); help text stale (F-002). |
| Wave 5 F-001 [S0] "in-session MP4 seek always fails (reanchor unimplemented)" | audit/runs/W05-A01.log.md:33 | REAL (still standing) | `rg 'fn reanchor' crates/demux/src/` → no mp4 hit; executed probe 2026-11-24: seek accepted, position stays 0.0. |

## 7. Dead / duplicated / unreachable code
None. All helpers are used; no `#[ignore]`, no cfg gates.

## 8. Dependencies & flags
No feature gates. Test-only deps: `commands`, `lumen_cli` (dispatch + host), `testkit`. Fixtures: `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi`, `tests/fixtures/generated/valid/ogg_opus_48k_mono.opus`, `crates/codecs/tests/fixtures/vorbis_sine_44100_stereo.ogg` — all present (verified with ls).

## 9. Open questions for the next agent
- Why does the MP4 probe's `media.get-current-time` stay at 0 even before any seek (2 s of wall time)? Either the release binary has no h264/aac decoders (baseline engine-info says mjpeg+pcm_s16le only — the "which registry lies" open question) and the session walks to end without playing, or the clock never ticks. Owner: apps/cli engine-info / pipeline_host registry audit.
- MKV in-session seek at host level: untested here; mkv/mod.rs:1026 reanchor looks real (compute-then-commit). A lane like the OGG one would close it.
