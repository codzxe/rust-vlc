# apps/desktop/src/shell.rs — the shell runtime + the session pacer (W13-A02)

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A02 |
| File | `apps/desktop/src/shell.rs` (843 LOC; 348 production / 495 test) |
| Reachability | lumen-tauri binary → `ShellState(Mutex<ShellRuntime>)` (apps/desktop-tauri/src/app.rs:288) → every invoke + every pacer tick hits `ShellRuntime::dispatch` / `pump_once` here → `CliHost` → engine. This file is the GUI's entire engine-facing surface. |
| Verdict | **Real, and the engine's committed proof surface for the GUI.** The pacer drives the true CLI pump (`CliHost::pump`), the frame tap drains real LUF1 records, and the two flagship tests (`pacer_proof_pump_once_drives_a_real_session_to_end`, `frame_channel_proof_pump_once_ships_luf1_records`) are the repo's honest evidence that GUI playback progresses and frames flow. Runtime pixels remain UNVERIFIED (headless proofs only; no webview). |

## Per-function table (production half, lines 34-346)

| Function | Lines | What it does (sed-verified) |
|---|---|---|
| `DEFAULT_EVENT_CAPACITY` | 57-65 | const 64 (bounded event egress; overflow drops OLDEST, counted). |
| `PACER_TICK` | 80-81 | `Duration::from_micros((lumen_cli::pipeline_host::RENDER_QUANTUM as u64) * 1_000_000 / 48_000)` — pinned to `30_625` µs by test (line 725). |
| `ShellRuntime::new` | 126-134 | `DesktopHost` + `ShellHost` + `BoundedEventSink` + `frame_tap: None, frame_channel: None`. |
| `ShellRuntime::dispatch` | 153-167 | intercepts `Command::SessionSetFrameChannel` (lines 160-162) → `bind_frame_channel`; everything else → `lumen_dispatch(&mut self.shell, request)` → `host.observe_response` → `pump_events`. |
| `bind_frame_channel` | 175-195 | first bind constructs `FrameTap` (179-185), `self.shell.engine_host().set_present_tap(tap)` (187), stores the channel id (188), answers class-S `Unit` completion (189-194). Stash-safe pre-open per its doc (171-174). |
| `take_frame_channel_binding` | 202-204 | one-shot `Option<u32>::take` for the wrapper's ticker. |
| `frame_channel_bound` | 210-212 | `self.frame_channel.is_some() || self.frame_tap.is_some()`. **Not called by the ticker** — test-only (line 639). |
| `take_frame_records` | 219-224 | drains the tap's mailbox as complete LUF1 records, oldest first; 0 when unbound. |
| `pump_events` | 229-234 | `forward_new_events` through the §4.2 staleness filter (`DeliverThrough`, 315-329) into the bounded queue. |
| `pump_once` | 251-254 | `self.shell.engine_host().pump(); self.pump_events();` — the CLI `advance` verb verbatim. |
| `take_pending_events` / `event_stats` | 258-266 | the emitter half + conservation accounting. |
| `open_path` | 272-276 | `open_path_request` → `self.dispatch(&request)` — the single funnel for dialog picks, drops, argv. |
| `on_files_dropped` | 280-296 | handler → request → `dispatch`; failure = typed refusal response (335-346). |
| `pick_and_open` | 301-312 | native-dialog flow; NOT wired in the tauri shell (app.rs:75-79 — dialog plugin unadmitted). |

## The pacer verdict (trace question)

`pump_once` → `CliHost::pump` (apps/cli/src/host.rs:429-434) = `engine.poll()` + output drain + `advance_time` (null-device time advance, video scheduler tick per render quantum, sed-verified host.rs:553-570) + end-tail play-out. **This is real engine frame timing — the scheduler presents at the media clock's slot boundary — driven on a 30.625 ms wall-clock cadence by the wrapper's ticker.** It is not a sleep-loop stub: the sleep only sets cadence; the presents are the engine scheduler's own slot decisions. The doc's own honest framing (shell.rs:74-79): "the null-device stand-in for the real audio callback's clock (ADR-005's real backends will own this cadence from the device side)."

The committed proof:
- `pacer_proof_pump_once_drives_a_real_session_to_end` (shell.rs:547-620): real AVI fixture, open+play through the single command, ONLY `pump_once` ticks → asserts `saw_playing`, `time_ticks > 0`, `saw_end_of_media`, `ended` (lines 604-617).
- `frame_channel_proof_pump_once_ships_luf1_records` (shell.rs:650-717): bind → open → play → pump ticks → asserts `records > 0` with `"LUF1 frame records must flow through the pacer (the GUI-shows-video bar)"` (707-710) and `"every record carries the LUF1 magic"` (711-714).

## Findings

- **S2 — the GUI frame proof fixture is AVI/MJPEG+PCM, not the MP4/H.264+AAC lane the orchestrator executed.** Both shell.rs flagship tests hard-code `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` (lines 486-488, 551-554, 651-654). No committed test drives LUF1 records through the H.264 software lane (the w31.6 claim's most likely user surface). The tap is codec-agnostic above the converter, so the gap is thin — but the recorded GUI-shows-video evidence bar was never run on the codec the baseline smoke ran.
- **S3 — `frame_channel_bound` is dead in production.** Its doc claims "the ticker's cheap guard" (shell.rs:207-209) but the ticker (app.rs:188-230) never calls it; its only caller is a test (shell.rs:639). Doc claims a consumer that does not exist.
- **S3 — bind-after-open frame loss.** The UI mounts its canvas and sends `session.set-frame-channel` only after the session leaves `idle` (see PlayerScreen.tsx audit); `set_present_tap` is state-safe for a mid-session install, but frames presented before the bind are never tapped and cannot be recovered. First ~1-2 ticks of playback are structurally lost. (Static analysis; runtime magnitude UNVERIFIED.)
- **S4 — `unreachable!` in `bind_frame_channel`/`FrameTap::default`** (lines 181, 102) — justified by the impossible-arm convention and denied-panic lint posture; recorded.
- **S4 — bounded egress drops the OLDEST events at capacity 64** (lines 56-57) — a slow webview during a burst (e.g., seek + track events) silently drops events with only stats counting. Consistent with the design; user-visible symptom would be a stale UI.

## Test honesty

The test half (348-843) is dense and real: 11 tests incl. the two flagship proofs above, `frame_channel_bind_is_intercepted_by_the_shell` (628-640 — engine never sees the bind command), `pacer_tick_is_one_render_quantum` (724-731), drop funnel (754-776), dialog scripting (782-812), bounded-queue drop-oldest conservation (817-842). No mocked engine — `ShellRuntime::default()` builds the real `CliHost`. These are the GUI's committed tests; they stop at the shell boundary by design ("What this build cannot prove", lines 27-32).

## Doc claims

- Module docs (lines 10-19) match code. Worklog w31.1 (worklog.md:2270-2276) claims "the GUI shows video ... Headless proofs: bind-intercept, open→play→pump→LUF1-records-to-Ended through the GUI's own funnel" — matches the two tests exactly; no overclaim at this layer.
- KNOWN-LIMITS.md:50 (w30.7 "~30 ms pacer") matches `PACER_TICK` 30_625 µs.

## Dead code / deps / flags

- Dead-in-production: `frame_channel_bound` (S3 above), `pick_and_open`/`DialogOutcome` (unwired dialog, admitted at app.rs:75-79), `engine_host` (diagnostic access used by tests).
- Deps: `commands`, `lumen_cli` (via ShellHost), `crate::{dispatch,error,files,host,frame_tap}` — no tauri types (std-only boundary respected).

## Open questions

1. `pump_once` cadence vs wall-clock drift: no drift observation reaches the pacer layer (the CLI's governor observes drift only in its own runner). Long sessions may accumulate display lag vs the media clock; the engine's scheduler will drop/hold frames (its own policy), but nothing at the shell measures GUI-side lag. The prototype's `LatencyAccountant` exists in mailbox.rs but is not consumed by the shipped path.
