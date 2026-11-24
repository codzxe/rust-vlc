# AUDIT: apps/desktop/src/shell.rs
| Field | Value |
|---|---|
| Auditor | W13-A10 |
| Date | 2025-06-27 |
| Lines / functions | 871 / 23 production items + 13 tests |
| File verdict | REAL-UNPROVEN (library: real, engine-driven, test-proven headlessly; NOT reachable from any shipped entry point — sole consumer is compile-broken and workspace-excluded) |
| Reachable from shipped entry point? | NO. Release lane builds `-p desktop` (baseline cargo-build-release-desktop.txt: "Finished `release` profile [optimized] target(s) in 1.62s"), but the shipped `desktop` binary (src/main.rs) never constructs `ShellRuntime` — it prints a report and exits 86. The only consumer is `apps/desktop-tauri/src/app.rs:93` (`use desktop::shell::{DEFAULT_EVENT_CAPACITY, ShellRuntime}`), a crate EXCLUDED from the workspace (root Cargo.toml:51 `exclude = ["apps/desktop-tauri"]`) and NOT in the release lane — and that crate cannot compile (F-002). |
| Findings | S0: 1 S1: 0 S2: 1 S3: 3 S4: 2 |
| Commands run | rg (ShellRuntime/pump_session/PACER_TICK/putImageData/session_pump callers, repo-wide); sed quote verification (all quotes below); ls apps/, apps/desktop/; baseline citations: cargo-test-workspace.txt:2564,2586 + 13 `test shell::tests` lines; cargo-build-desktop-tauri.txt:310 (gdk-sys); cargo-build-release-desktop.txt (exit 0) |

## 1. One-paragraph verdict
The runtime itself is genuine: `dispatch` really routes through the single `lumen_dispatch` into a real engine, the two pacer proofs (`pacer_proof_pump_once_drives_a_real_session_to_end`, `frame_channel_proof_pump_once_ships_luf1_records`) drive a real AVI/MJPEG/PCM fixture to `Ended` through nothing but `pump_once` and assert LUF1 records flow — both pass in the orchestrator baseline. But this is a headless bridge library, not an app: the shipped `desktop` binary never touches it, and the one composition that would (the tauri wrapper in the excluded `apps/desktop-tauri` crate) calls an undefined function (`spawn_session_pump`, app.rs:301 — the defined name is `spawn_event_pump`, app.rs:190) and therefore cannot compile even on a GTK-equipped host, while INFRA-BLOCKED.md claims "the crate compiles in its dedicated CI lane". The docs also sell `PACER_TICK` (30.625 ms) as the GUI ticker cadence when the real (broken) ticker uses a 10 ms `SESSION_TICK`. So: real engine seam, provable headlessly; zero shipped reachability; the "GUI plays video" story rests on a compile-broken excluded crate.

## 2. What actually works (verified)
— verified by audit/runs/baseline/cargo-test-workspace.txt:2586 `test shell::tests::pacer_proof_pump_once_drives_a_real_session_to_end ... ok` and :2564 `test shell::tests::frame_channel_proof_pump_once_ships_luf1_records ... ok` (13/13 shell tests ok in baseline). These run the real engine + real fixture through `open_path` → `dispatch` → `pump_once`; not mocks. (Not re-run: cargo-build-budget; baseline cited per spec §2.)

## 3. Findings
F-001 [S0]
Where: apps/desktop-tauri/src/app.rs:299-302 (consumer of this file's composition root)
Code:
```
            apply_window_spec(app.handle())?;
            spawn_session_pump(app.handle().clone());
            open_argv_files(app.handle());
```
What is wrong: `spawn_session_pump` has no definition anywhere in the repository — the ticker function is `fn spawn_event_pump(app: AppHandle)` (app.rs:190; repo-wide `rg "fn spawn" apps/desktop-tauri/` → only that hit; no glob imports in app.rs). This is E0425: the ADR-004 admission wrapper — the ONLY entry point that constructs `ShellRuntime`, drives `pump_session`, and ships LUF1 frames to the webview canvas — cannot compile on ANY host, GTK or not. Why it matters: INFRA-BLOCKED.md claims "the crate compiles in its dedicated CI lane (`tauri-check`, Linux + webkit2gtk)" — that claim is false against the current tree (or the lane is stale/red); the sandbox baseline only fails earlier at gdk-sys (cargo-build-desktop-tauri.txt:310 `error: failed to run custom build command for 'gdk-sys v0.18.2'`), which masks the break. Every desktop "GUI plays / GUI shows video" narrative routes through this crate. What correct looks like: rename the call to `spawn_event_pump` (or define the alias) and re-run the tauri-check lane. Confidence: HIGH — Verification: read + rg (static proof; cargo build impossible here: no GTK + cargo-build-budget).

F-002 [S2]
Where: apps/desktop/src/shell.rs:101-115, 153-340 (whole `ShellRuntime`) vs apps/desktop/src/main.rs:85-87
Code:
```
    eprintln!("no webview host compiled: ADR-004 pending (records/tauri-wry-2.x.md)");
    ExitCode::from(WEBVIEW_PENDING)
```
What is wrong: in the release lane (`cargo build --release --locked -p lumen-cli -p desktop`) the shipped desktop binary never constructs `ShellRuntime` — it prints a skeleton report and exits 86. `ShellRuntime` (dispatch, pacers, frame channel, dialog/drop funnels) is dead code in every shipped artifact; its only consumer is the workspace-excluded, compile-broken crate of F-001. Why it matters: the entire w27/w31 shell+canvas machinery ships to nobody; the honest shipped desktop UX is a console message and exit 86. What correct looks like: either admit the tauri crate into a buildable lane (fix F-001 + CI) or stop describing `desktop` as the app layer of record. Confidence: HIGH — Verification: read (main.rs has no ShellRuntime reference; rg confirms sole consumer) + baseline cargo-build-release-desktop.txt exit 0.

F-003 [S3]
Where: apps/desktop/src/shell.rs:10-19 vs 252-257 vs apps/desktop-tauri/src/app.rs:109
Code:
```
//! nothing pumps, so a wall-clock thread calling `pump_once` every
//! [`PACER_TICK`] (one render quantum, ≈30.6 ms) is what makes GUI
//! playback progress at all: time-ticks flow, the position
```
vs
```
const SESSION_TICK: Duration = Duration::from_millis(10);
```
What is wrong: the module header says the GUI ticker calls `pump_once` every `PACER_TICK` (30.625 ms). The file's own body (252-257) says the production ticker drives `pump_session` instead; the actual wrapper ticks every 10 ms (`SESSION_TICK`). `PACER_TICK` (L80-81) has zero production callers — only its self-pinning test (L752-760) references it. Why it matters: three mutually inconsistent cadence stories in the desktop lane; a reader tuning the pacer will tune a dead constant. What correct looks like: header rewritten to `pump_session`/10 ms, `PACER_TICK` deleted or wired. Confidence: HIGH — Verification: read + rg (PACER_TICK referenced only in shell.rs).

F-004 [S3]
Where: apps/desktop/src/shell.rs:279-282
Code:
```
    pub fn pump_session(&mut self) {
        self.shell.engine_host().pump_wallclock();
        self.pump_events();
    }
```
What is wrong: `pump_session` — the one method the production ticker depends on ("the production cadence seam, w27") — is exercised by NO test in this file (all 13 tests drive `pump_once` or the queue directly; rg `pump_session` in shell.rs → definition + doc mentions only). Its wall-bounded semantics live entirely in `CliHost::pump_wallclock` (other agents' files). Why it matters: the tested seam is the whole-ring headless variant; the shipped-GUI seam is untested here and its consumer is compile-broken (F-001), so no lane anywhere executes it end-to-end. What correct looks like: a pump_session-driven variant of the pacer proof (wall-bounded, deterministic via injected clock or bounded loop). Confidence: HIGH — Verification: read + rg.

F-005 [S3]
Where: apps/desktop/src/shell.rs:175-186
Code:
```
                let tap = std::sync::Arc::new(
                    crate::frame_tap::FrameTap::new()
                        .unwrap_or_else(|err| unreachable!("the default config is valid: {err}")),
                );
```
What is wrong: `unreachable!` on a fallible constructor inside the dispatch path, under the wrapper's shell mutex (app.rs:150) — if `FrameTap::new` ever fails, the panic poisons `ShellState` and every subsequent command answers `StatePoisoned` (app dead, no recovery). Today the default config is infallible (frame_tap.rs:53-56 "never for the default"), so the trigger is latent. Why it matters: pattern risk (baseline counts 4126 `unreachable!` tree-wide); the blast radius here is the whole GUI, not one request. What correct looks like: typed refusal response (the file already has `refusal_response` for exactly this shape). Confidence: HIGH — Verification: read.

F-006 [S4]
Where: apps/desktop/src/shell.rs:29-31
Code:
```
//! window build itself — webkit2gtk-4.1 is absent from this sandbox
//! (ADR-004; `apps/desktop/tauri/INFRA-BLOCKED.md`). The runtime is
```
What is wrong: cited path `apps/desktop/tauri/INFRA-BLOCKED.md` does not exist; the file lives at `apps/desktop-tauri/INFRA-BLOCKED.md` (moved per its own header). Doc rot in the composition root's primary pointer. Confidence: HIGH — Verification: `ls apps/desktop/tauri` → No such file or directory.

F-007 [S4]
Where: apps/desktop/src/shell.rs:163-166
Code:
```
        let response = lumen_dispatch(&mut self.shell, request);
        let _ = self.host.observe_response(&response);
```
What is wrong: the `ResponseObservation` (`log_ok`, `generation_bump` — host.rs:167-186) is discarded: a §4.3 echo-legality violation or generation bump is never surfaced anywhere (wrapper returns the response regardless). The staleness state update itself does happen inside `observe_response`, so this is observability loss, not correctness loss. Confidence: HIGH — Verification: read (host.rs:167-186 signature checked).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DEFAULT_EVENT_CAPACITY | 57-65 | REAL | — | const 64 via NonZeroUsize; `None => loop {}` compile-time arm, documented |
| PACER_TICK | 80-81 | DEAD | S3 | no production caller (F-003); pinned only by its own test |
| DialogOutcome | 84-97 | REAL | — | 3-state enum, used by pick_and_open + tests |
| ShellRuntime (struct) | 101-115 | REAL-UNPROVEN | S2 | composition real; unreachable from shipped lane (F-002) |
| Default::default | 117-121 | REAL | — | delegates to new(DEFAULT_EVENT_CAPACITY) |
| new | 126-134 | REAL | — | constructs host+shell+sink+taps |
| engine_host | 139-141 | REAL | — | test/diagnostic accessor; used by tests (settle, session_id) |
| next_request_id | 145-147 | REAL | — | delegates to host |
| dispatch | 153-167 | REAL | S4 | single-command routing + frame-channel interception; observation discarded (F-007) |
| bind_frame_channel | 175-195 | REAL | S3 | tap install + one-shot id; unreachable! hazard (F-005) |
| take_frame_channel_binding | 202-204 | REAL | — | one-shot take; consumed by (broken) wrapper ticker |
| frame_channel_bound | 210-211 | REAL | — | cheap guard; note: true after take (tap persists) — matches doc |
| take_frame_records | 219-224 | REAL | — | drains LUF1 records, oldest first |
| pump_events | 229-234 | REAL | — | §4.2-filtered drain via DeliverThrough |
| pump_once | 258-261 | REAL | — | whole-ring pump; headless proof semantics; tested |
| pump_session | 279-282 | REAL-UNPROVEN | S3 | production seam; untested here, consumer compile-broken (F-001/F-004) |
| take_pending_events | 286-288 | REAL | — | empties queue |
| event_stats | 292-294 | REAL | — | honest accounting; `conserves()` asserted in tests |
| open_path | 300-304 | REAL | — | single funnel; None for unclassifiable paths |
| on_files_dropped | 308-324 | REAL | — | drop funnel; errors → typed refusal_response (NotReady) |
| pick_and_open | 329-340 | REAL | — | cancel no-op; Pending typed; unlinked dialog real |
| DeliverThrough::forward | 350-357 | REAL | — | kept→queue, stale→counted drop |
| refusal_response | 363-374 | REAL | — | typed NotReady envelope; NotReady for a host-seam failure is a semantic stretch but honest |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| dispatch_routes_observes_and_emits | YES | real engine command; asserts applied value + egress conservation |
| config_writes_round_trip_through_dispatch | YES | set→get through real dispatch, real config store |
| invalid_config_writes_refuse_typed | YES | asserts InvalidArgument on unknown key |
| engine_events_reach_the_bounded_queue_in_order | YES | real fixture; asserts non-empty, seq order, monotonic drain |
| playlist_files_load_the_list_through_the_command | YES | asserts UnsupportedForHost + command=playlist.load |
| pacer_proof_pump_once_drives_a_real_session_to_end | YES | real fixture to Ended via pump_once only; asserts Playing/time-ticks/EOS/Ended |
| frame_channel_bind_is_intercepted_by_the_shell | YES | asserts Unit completion + one-shot take + bound |
| frame_channel_proof_pump_once_ships_luf1_records | YES | asserts records>0 + LUF1 magic through real decode lane |
| pacer_tick_is_one_render_quantum | WEAK | pins a constant to itself (30_625 µs literal is the only independent anchor); would not catch semantic drift |
| non_openable_paths_are_noops | YES | asserts None for 2 non-media paths |
| drops_funnel_through_the_runtime | PARTIAL | proves the funnel (Some returned), deliberately ignores the response body ("what is proven here is the FUNNEL, not the decode") — honest but weak |
| dialog_picks_dispatch_cancels_noop_and_pending | YES | 3 scripted arms, asserts typed outcomes |
| egress_queue_is_bounded_drop_oldest_and_conserves | YES | drives BoundedEventSink directly; asserts [3,4] kept, stats conserve |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "shell.rs:280 is the real-time ticker" | Wave 3 (task prompt) | PARTLY FALSE | L279-282 `pump_session` is the 2-line tick BODY; the ticker THREAD is apps/desktop-tauri/src/app.rs:190-232 at 10 ms `SESSION_TICK` (app.rs:109) — and it is compile-broken (F-001) |
| "shell.rs:160-162 has the REAL 2D canvas putImageData" | Wave 12 (task prompt) | FALSE as stated | L160-162 is the `SessionSetFrameChannel` interception (quoted above) — no canvas code in this file; putImageData is ui/src/components/player/MediaSurface.tsx:227 (`ctx.putImageData(image, 0, 0);`), gated on `__TAURI_INTERNALS__` (MediaSurface.tsx:14-16 "In a plain browser it NEVER renders"). shell.rs:160-162 is the Rust half of the same lane |
| "a wall-clock thread calling pump_once every PACER_TICK ... makes GUI playback progress" | shell.rs:15-19 | FALSE (stale) | real ticker: pump_session every 10 ms (F-003); also compile-broken (F-001) |
| "The runtime is the seam those calls delegate to; every method here runs under cargo test today" | shell.rs:31-32 | REAL | baseline: 13/13 shell tests ok (cargo-test-workspace.txt) |
| "the crate compiles in its dedicated CI lane (tauri-check)" | apps/desktop-tauri/INFRA-BLOCKED.md:4-5 | FALSE (against current tree) | `spawn_session_pump` undefined → E0425 (F-001) |
| "the 40 ms event pump" | INFRA-BLOCKED.md §A | STALE | SESSION_TICK = 10 ms (app.rs:109) — belongs to W14-A04's file, recorded here for the cadence contradiction set |

## 7. Dead / duplicated / unreachable code
- `PACER_TICK`: dead in production (F-003).
- Whole `ShellRuntime`: unreachable from shipped release lane (F-002).
- `engine_host()` accessor: production callers none (test/diagnostic only; wrapper uses `set_present_tap` via `engine_host()` at bind time — that path is inside the broken crate).

## 8. Dependencies & flags
- No feature gates in this file; depends on crate-local dispatch/host/files/frame_tap + `lumen_cli::pipeline_host::RENDER_QUANTUM` (for the dead constant) + `commands` wire types. No unsafe. `#[allow(clippy::empty_loop)]` at L62 is a targeted, explained lint suppression (compile-time dead arm), not a hidden one.

## 9. Open questions for the next agent
- W14-A04 (apps/desktop-tauri/*): confirm F-001 (spawn_session_pump E0425) against tauri-check CI history; the lane is workflow_dispatch/manual for tauri-launch.yml but tauri-check sits in ci.yml — did the last green run predate the rename to spawn_event_pump?
- Who tests `CliHost::pump_wallclock` (the real production pump)? If untested, F-004 doubles.
- ui/src/client/frame-channel.ts: does `detectFrameChannelBackend` degrade silently when the bind command is refused by a non-desktop host? (Cross-lane, W12 already touched.)
