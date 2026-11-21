# AUDIT: apps/cli/src/host.rs#part1 (lines 1–646)
| Field | Value |
|---|---|
| Auditor | W13-A09 |
| Date | 2026-11-21 |
| Lines / functions | 646 (of 1941; boundary pinned from audit/waves/wave-00/assignments.tsv:809 — part2=647–1292 A08, part3=1293–1941 A10) / 34 items |
| File verdict | REAL |
| Reachable from shipped entry point? | YES. `lumen` main.rs → run.rs:152/192/308 `CliHost::new()` (REPL/one-shot/machine mode) and apps/desktop `lumen-desktop` main → shell.rs (pump_wallclock at shell.rs:280, set_present_tap at shell.rs:187). Every pump/accessor has a production or shipped-binary consumer (rg evidence below). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | Read host.rs 1–330/330–649/649–768/885–900/1050–1179; rg `CliHost::new|use crate::host` (run.rs:57,152,192,308; dispatch.rs:33); rg `set_present_tap|pump_wallclock|set_event_throttle` over repo (desktop shell.rs:187/280 — my part2 file; first-vertical harness.rs:237/265); rg `RUN_PUMPS` (only apps/cli/tests/* + shell.rs:388 doc); rg `pump_until_state|pump_until\(|settle\(` (run.rs:216/280, shell.rs:523, desktop dispatch.rs:347, harness); rg `position_us|track_snapshot|headless_accounting|screenshot_png` (run.rs:259, dispatch.rs:137, harness.rs:290-323, cli tests); rg `FreezeClock|Seeking` in playback-engine + sed state.rs:440–540 (Playing\|Seek→Buffering with FreezeClock — the anchor-clear design verified); sed seek.rs:25–104 (seven-step flow, clock re-anchor on first audio frame); rg TickBudget (pipeline.rs:472-475: demux 64 / packets 16 / flush 8 — module-doc claim verified); sed audio-output/src/null.rs:292–310 (render writes INTO the buffer, paused → no-op); rg `reopen` (help.rs:57 stale text vs dispatch.rs:131 native seek). |

## 1. One-paragraph verdict
Part 1 of the CLI host — the struct, the observation accessors, and the two pacers (deterministic `pump`/`advance` and wall-bounded `pump_wallclock`) — is genuine, carefully documented machinery, and I traced every public item to a live consumer: the REPL (`run.rs`), the desktop shell (`lumen-desktop`), or the first-vertical evidence harness. The wall-anchor design holds up under hostile analysis: the seek's `FreezeClock` window is a real `Playing → Buffering` transition (playback-engine state.rs:523-533), so the anchor clears exactly when the module doc claims, and pause/underrun/rate-change re-anchoring is covered by the match at 547-569. The per-poll bounds the module doc states (demux ≤ 64, decode ≤ 16/limb) match `TickBudget::default()` verbatim. What I have are two hygiene-grade defects: a fresh `Vec` allocated per rendered quantum in the wall pacer right next to the file's own zero-alloc brag, and a test-only `pub const` in the production namespace. No tests live in this range (the test module starts in part3), so this machinery's coverage is entirely secondhand.

## 2. What actually works (verified)
- **Pump anatomy matches the doc**: `pump()` (457-461) = `pump_pipeline(); advance_time(); drain_tail_at_end()`; `pump_pipeline` = poll + drain + error surfacing (447-451) — exactly the module doc's steps 1-3. Doc claim "demux ≤ 64 events, decode ≤ 16 packets per limb" = `TickBudget::default()` (crates/playback-engine/src/pipeline.rs:472-473) — rg verified.
- **Seek narrative is true in code**: module doc 29-74 ("in-pipeline … the reopen fallback does not exist") matches dispatch.rs:131 `Command::PlaybackSeek { target } => host.dispatch_seek(*target)` → `handle.seek` (host.rs:1106-1108, part2) and `seek_boundary` (pipeline_host.rs:860/1095). The engine side freezes the clock via the real `Playing|Seek→Buffering` row (state.rs:523-533), and `refresh_wall_anchor`'s `(_, false, _, _)` arm clears the anchor in that window — the comment's "the seek's frozen window" claim is verified against the state table.
- **Wall-anchor state machine (543-570)**: all four arms traced — not-Playing clears; same-rate Playing keeps; re-entry/rate-change re-anchors at the device counter; limb-less Playing clears. No path leaves a stale anchor.
- **Due-frames arithmetic (590-597)**: u128 intermediates, `saturating_add`, `try_from(...).unwrap_or(u64::MAX)` — no overflow/panic path; the `as u64` on nanos cannot overflow before 584 years.
- **Consumers verified**: `settle` → run.rs:216/280 + shell.rs:523 + desktop dispatch.rs:347; `advance` → run.rs:252 (the `advance` verb); `pump_wallclock` → shell.rs:280 (the desktop session thread); `set_present_tap` → shell.rs:187 (the LUF1 lane — my frame_tap.rs file is the consumer side); `position_us` → run.rs:259; `track_snapshot` → dispatch.rs:137; `headless_accounting`/`screenshot_png` → first-vertical harness.rs:290-323 + cli tests. `set_event_throttle` → harness.rs:237 only — documented as the evidence-harness seam ("production hosts leave it unset").
- Baseline execution of this machinery: every CLI smoke (INDEX "MP4 SMOKE TEST", rc-smoke #20) drives `CliHost` end-to-end; `shell::tests::pacer_proof_pump_once_drives_a_real_session_to_end` (apps/desktop, baseline test-workspace.log.txt:2361) exercises `pump_wallclock_at` on a real session.

## 3. Findings

### F-001 [S4] — Fresh per-quantum buffer allocation in the wall pacer, next to the file's own zero-alloc discipline
Where: apps/cli/src/host.rs:613 (twin at :725, part2's range)
Code:
```
                    let mut buffer = vec![0.0_f32; RENDER_QUANTUM * channels];
                    if let Err(error) = device.render(&mut buffer) {
                        guard.note_video_error(error);
                        break;
                    }
```
What is wrong: every rendered quantum in `advance_time_wall` (the desktop session-thread pacer, shell.rs:280) allocates and zeroes a new `RENDER_QUANTUM × channels` f32 buffer (~11.8 KB stereo at 1470 frames), then `NullDevice::render` overwrites it (render writes INTO `out`, null.rs:301-306). At 48 kHz that is ~33 allocations/s of dead zeroing. The same file's drain path boasts the D-1 reuse discipline (outbox_scratch, lines 182-186: "steady state allocates nothing on the drain path") — the render path has no such scratch.
Why it matters: pure churn in the GUI hot path; inconsistent with the file's own allocation discipline. No correctness impact (hence S4).
What correct looks like: a reused member scratch (the outbox_scratch pattern) sized once per session (channels known at open), or a device API that pulls into a caller-owned reusable buffer.
Confidence: HIGH — Verification: read (null.rs:292-310 render semantics confirmed; call site shell.rs:280 confirmed).

### F-002 [S4] — `pub const RUN_PUMPS` exists only for tests
Where: apps/cli/src/host.rs:145-146
Code:
```
/// The pump bound for run-to-end (the scripted session's `advance`).
pub const RUN_PUMPS: usize = 512;
```
What is wrong: rg over the whole repo shows its only uses are `apps/cli/tests/{sync_scenarios.rs:568, pipeline_accounting.rs:54,169, native_seek.rs:175,229,255,294,306,446,524,593}` and a doc mention in apps/desktop/src/shell.rs:388. No production path reads it — run.rs's `advance` verb takes user ticks, and the desktop pacer is wall-bounded.
Why it matters: test scaffolding exported in the production API surface of the lib (`lumen_cli::host::RUN_PUMPS`); a future reader will assume a production bound exists where there is none.
What correct looks like: move it to a test-support module or the test files themselves.
Confidence: HIGH — Verification: read + rg (all use sites listed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc | 1-75 | REAL | — | pump anatomy, per-poll bounds (64/16), determinism scope, seek narrative all verified against engine code; "reopen does not exist" TRUE (dispatch.rs:131) but contradicted by shipped help text (see §6) |
| `const AWAIT_PUMPS` | 101-103 | REAL | — | 64-pump bound, fixture-calibrated; exhaustion is silent pacing (documented), not an error |
| `struct HeadlessAccounting` | 107-143 | REAL | — | 14 u64 counters, Copy — fine |
| `const RUN_PUMPS` | 145-146 | REAL | S4 | F-002: test-only |
| `type StateStep` | 148-149 | REAL | — | trivial alias |
| `struct CliHost` | 151-200 | REAL | — | 15 fields, all owned/traced; single-writer invariant documented |
| `struct WallAnchor` | 202-218 | REAL | — | Copy anchor {at, frames, rate} |
| `impl Default` | 220-224 | REAL | — | delegates to new |
| `CliHost::new` | 226-251 | REAL | — | all fields initialized; Engine::new |
| `set_event_throttle` | 253-262 | REAL | — | documented test seam; sole caller first-vertical harness.rs:237 |
| `set_present_tap` | 264-278 | REAL | — | stash-then-ride-first-consumer; consumer shell.rs:187 (desktop LUF1); part2 open_session:944-946 installs |
| `settle` | 282-298 | REAL | — | pipeline-only until non-transient; callers run.rs:216/280, shell.rs:523, desktop dispatch.rs:347 |
| `state` | 300-304 | REAL | — | wire mapping delegate |
| `session_id` / `generation` | 306-316 | REAL | — | delegates |
| `events` / `state_history` | 318-328 | REAL | — | slice accessors |
| `position_us` | 330-359 | REAL | — | audio-master reads counter (deterministic); limb-less latches last-presented/anchor; wall elapsed never reported (doc verified honest) |
| `facts` | 361-365 | REAL | — | accessor |
| `track_snapshot` | 362-372 | REAL | — | clones Vec (cold path, `tracks` command) |
| `engine_stats` | 374-378 | REAL | — | delegate to engine.stats() |
| `audio_lane_counters` | 380-395 | REAL | — | locks consumer; None before session |
| `headless_accounting` | 397-425 | REAL | — | requires video limb (documented); harness consumer |
| `screenshot_png` | 427-435 | REAL | — | Option<Result> nesting; harness consumer |
| `pump_pipeline` | 439-451 | REAL | — | poll + drain + surface error; no time advance (determinism contract) |
| `pump` | 453-461 | REAL | — | pipeline + advance + tail drain |
| `pump_pipeline_until` | 463-476 | REAL | — | pred-first, bounded |
| `pump_until` | 478-489 | REAL | — | full-pump variant |
| `pump_until_state` | 491-494 | REAL | — | thin wrapper; used by part3 tests |
| `advance` | 496-501 | REAL | — | the `advance` verb (run.rs:252) |
| `pump_wallclock` | 503-526 | REAL | — | desktop session-thread tick (shell.rs:280) |
| `pump_wallclock_at` | 528-536 | REAL | — | injectable-now; ordering refresh→pipeline→advance verified |
| `refresh_wall_anchor` | 538-570 | REAL | — | 4-arm match traced; seek window clears via real state row (state.rs:523-533) |
| `advance_time_wall` | 572-634 | REAL | S4 | due-frames math overflow-safe; paused guard; bounded loops; F-001 alloc |
| `drain_outputs` (head; body 647-684 in part2) | 636-645 | REAL | — | straddles the 646 boundary: doc + signature + take/clear/reuse in my range; match arms (state_history, last_fatal, TracksChanged enrichment, staleness filter) are A08's — flagged, not double-counted |

## 5. Test honesty
No `#[cfg(test)]` code exists in lines 1-646 (the module starts at ~line 1500, part3). Coverage of this range is secondhand: part3 tests (`pacer_proof…` lives in desktop; host tests at 1703+), apps/cli/tests/* (pump_until_state, RUN_PUMPS loops), first-vertical harness, and the baseline CLI smokes. Nothing in my range fakes anything — but note the asymmetry: the wall pacer's only in-repo proof is apps/desktop's `pacer_proof_pump_once_drives_a_real_session_to_end` (baseline test-workspace.log.txt:2361 `… ok`) — that is a desktop-crate test, i.e. the CLI's GUI pacer is proven only by another crate's test.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "state moves only inside Engine::poll; one pump = poll + drain + advance" | host.rs:7-20 | REAL | pump()/pump_pipeline() bodies; TickBudget 64/16 (pipeline.rs:472-473) |
| "nothing reads a wall clock [audio-bearing path]; every observable a pure function of the pump count" | host.rs:22-24 | REAL | advance_time (part2) reads no Instant; position_us reads the counter clock; wall reads exist only in pump_wallclock (documented GUI pacer) |
| Seek: "submits the engine's Seek … seven-step flow IN-PIPELINE … the reopen fallback does not exist" | host.rs:29-74 | REAL | dispatch.rs:131 → dispatch_seek (1059) → handle.seek:1106; engine seek.rs:25-104; state.rs:523-533 |
| `lumen --help` seek line: "seek (v0: reopen at the target through the engine's real transitions)" | help.rs:57 (A10's file) | **FALSE/stale** | contradicts the verified native-seek implementation above — the shipped help misdescribes seek to users; handed to A10 |
| "The pure CLI binary never calls set_present_tap" | host.rs:270-272 | REAL | rg: only apps/desktop/src/shell.rs:187 calls it |

## 7. Dead / duplicated / unreachable code
- `RUN_PUMPS` (F-002) — test-only.
- `set_event_throttle` — test-harness-only seam, explicitly documented as such (253-259); not dead, intentional.
- `pump_until_state` — used by part3 tests only; thin wrapper, acceptable.
- No unreachable production path found in 1-646.

## 8. Dependencies & flags
No feature gates. Imports: playback-engine, commands, media-clock, audio-output(null), pipeline_host — all workspace-internal. No unsafe, no I/O, no threads in this range.

## 9. Open questions for the next agent
- A08 (host.rs#part2): `advance_time`:725 has the same per-quantum alloc (F-001 twin); `audio_rate_hz` (772) — confirm it cannot return 0 while a device exists (device implies geometry>0 — I traced open_audio_session but the rate source is yours); `open_session`'s comment "(also the seek reopen)" (889) is stale — no reopen exists.
- A10 (help.rs): the shipped seek help line claims "reopen" — false vs dispatch_seek; also `exit.rs` — who maps the failed-open exit code? My mkv experiments got EXIT=65 (decode refusal at open) while the MP4 smoke's decode-refusal exits 0 — the divergence is in run/exit wiring, likely your files.
- Desktop wave: `lumen-desktop` (apps/desktop main) builds in baseline (#3 desktop tests ran) — confirm it is a shipped entry point or only a dev harness for the tauri shell.
