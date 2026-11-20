# apps/desktop-tauri/src/app.rs — THE tauri wrapper (W13-A02)

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W13-A02 |
| File | `apps/desktop-tauri/src/app.rs` (320 LOC) |
| Reachability | lumen-tauri binary → `main.rs::main` → `app::run()` (line 286) → tauri Builder → webview (ui/dist) ↔ `lumen_dispatch` IPC ↔ `ShellRuntime` (apps/desktop/src/shell.rs) → engine. Runtime GUI behavior UNVERIFIED here (no webview in sandbox). |
| Verdict | **Real single-command shell + real pacer ticker + real frame shipper. Not a stub.** The engine-driving loop is the proven CLI pump; the frame path is wired end to end at the source level. The runtime half (webview emit/receive, window) is exactly what the repo itself records as unproven on real hardware. |

## Per-function table (>100 LOC file)

| Function | Lines | What it does (sed-verified) |
|---|---|---|
| `lumen_dispatch` | 142-152 | `#[tauri::command] async fn` — locks `Mutex<ShellRuntime>`, `runtime.dispatch(&request)`, `forward_pending(&app, &mut runtime)`, returns `WireResponse`. Transport-level failure only on mutex poison (`ShellError::StatePoisoned`, lines 118-121). |
| `forward_pending` | 159-167 | `runtime.pump_events(); for event in runtime.take_pending_events() { app.emit(EVENT_CHANNEL, &event) }` — emits UNDER the lock; emit failure = `eprintln!` (lines 162-165). |
| `spawn_event_pump` | 188-230 | the session pacer + frame shipper (below, in depth). |
| `apply_window_spec` | 235-266 | parses `product.toml [window]` via `include_str!("../../../product.toml")` (line 241), honest-parse rule; `set_size`/`set_min_size`/`maximize` on webview `main`. |
| `open_argv_files` | 269-283 | argv files at startup → `runtime.open_path(&path)` per file, then `forward_pending`. |
| `run` | 286-320 | `.manage(ShellState(Mutex::new(ShellRuntime::new(DEFAULT_EVENT_CAPACITY))))` (lines 288-290); `.invoke_handler(tauri::generate_handler![lumen_dispatch])` (line 291); setup: window spec, ticker, argv files (lines 298-300); `on_window_event` DragDrop → `runtime.on_files_dropped(&mut PathDragDrop, paths)` (lines 316). |

## The command + event channel

Sed-verified quotes:
- Line 97: `pub const EVENT_CHANNEL: &str = "lumen://event";` — mirrored in `ui/src/client/tauri-transport.ts:31` (`LUMEN_EVENT_CHANNEL = "lumen://event"`).
- Lines 142-152 (the whole command body — see index audit). Exactly one command is registered (line 291), enforced by `tests/single_command.rs`.

This is the SAME command layer the CLI uses: `ShellRuntime::dispatch` → `crate::dispatch::lumen_dispatch` → `lumen_cli::dispatch::lumen_dispatch(&mut shell.host, request)` (sed-verified `apps/desktop/src/dispatch.rs:211-213`). File-open in the GUI is therefore `session.open` on the same engine — no desktop-private engine path. Confirmed by `apps/desktop/src/dispatch.rs:215-231` (`open_file_request` builds `Command::SessionOpen { source: SourceSpec::File { path } }`).

## The pacer (card 3.B3 / w31.1 ticker) — in depth

```
std::thread::spawn(move || {
    let mut frame_channel: Option<crate::frame_channel::FrameChannel> = None;
    loop {
        std::thread::sleep(PUMP_INTERVAL);
        let state = app.state::<ShellState>();
        let Ok(mut runtime) = state.0.lock() else { return; };
        runtime.pump_once();
        forward_pending(&app, &mut runtime);
        let bind = runtime.take_frame_channel_binding();
        let mut records = Vec::new();
        if frame_channel.is_some() {
            runtime.take_frame_records(&mut records);
        }
        drop(runtime);
        if let Some(id) = bind {
            match crate::frame_channel::frame_channel(&app, id) {
                Ok(channel) => frame_channel = Some(channel),
                Err(error) => { eprintln!("frame channel bind failed: {error}"); }
            }
        }
        if let (Some(channel), Some(record)) = (&frame_channel, records.last()) {
            if let Err(error) = crate::frame_channel::send_record(channel, record) {
                eprintln!("frame channel send failed (dropping until the next bind): {error}");
                frame_channel = None;
            }
        }
    }
});
```
(app.rs:188-230; sed-verified — this is the verbatim control flow.)

**What it actually paces:**
- `runtime.pump_once()` = `self.shell.engine_host().pump()` + `self.pump_events()` (sed-verified `apps/desktop/src/shell.rs:251-254`), and `CliHost::pump` = `pump_pipeline` (engine `poll` + output drain) + `advance_time` + `drain_tail_at_end` (sed-verified `apps/cli/src/host.rs:429-434`). `advance_time` ticks the video scheduler per rendered null-audio quantum — sed-verified `apps/cli/src/host.rs:553-570` ("Tick BEFORE the render: the scheduler's slot decision must see the clock as it stands at the slot boundary").
- `PUMP_INTERVAL = PACER_TICK` (line 107); `PACER_TICK = Duration::from_micros(RENDER_QUANTUM * 1_000_000 / 48_000)` = `30_625` µs (sed-verified `apps/desktop/src/shell.rs:80-81, 725`). So this is NOT a sleep-loop stub in the "fake timing" sense — it drives the engine's real scheduler at the null-device render quantum. But it IS a wall-clock sleep loop: drift between wall time and the media clock is uncorrected per tick (no catch-up/skip policy at this layer; the engine's own scheduler decides what presents).

**Frame shipping policy — the real finding:**
- One record per tick: `records.last()` (app.rs:221) — the NEWEST record is shipped and every other drained record is dropped ("the one-frame-per-tick contract — the webview composites at the pacer's own cadence; the mailbox's latest-wins bound absorbed anything older", app.rs:180-184).
- Ceiling arithmetic: 1 frame / 30.625 ms = **32.65 fps cap** for ANY content, and a decoder that produces 2+ frames between ticks permanently loses all but the newest. 60 fps source material cannot display at 60 fps through this shell. There is no PTS-driven presentation in the webview (MediaSurface draws immediately on arrival; `MediaSurface.tsx:18-25` states "the surface does no timing of its own").

## Findings (S0-S4)

- **S1 — audio is absent from this surface by construction.** Nothing in this file or the shell library opens an OS audio device; the engine's audio session is `NullBackend::open_stream(&format, RING_FRAMES, NullConfig { capture: false })` (sed-verified `apps/cli/src/pipeline_host.rs:1555-1556`). The pacer's tick IS the null device's render quantum. "Hear audio through lumen-tauri" is FALSE at HEAD. See the audio-output index audit for the crate-level verdict.
- **S2 — the event half of the GUI depends on a runtime-unverified tauri API shape.** The UI's event ingestion (`plugin:event|listen` with `target: { kind: "Any" }`) carries the repo's own `[VERIFY at the ADR-004 admission build]` marker (`ui/src/client/tauri-transport.ts:93-98`), and `INFRA-BLOCKED.md` §B.2 records "the event-frame round-trip) remain manual". If the frame shape is wrong, the GUI shows nothing even though every layer below is proven. UNVERIFIED in this sandbox (no webview).
- **S3 — frame-ship halves have no committed integration coverage.** `frame_channel.rs::frame_channel`/`send_record` are never executed by any test (headless proofs stop at `ShellRuntime::take_frame_records`); the only coverage is the CI compile lane. A typo in the `__CHANNEL__:<id>` reference form would surface only on a real desktop run. (Static read of `frame_channel.rs:40` against the UI shim's `FrameChannelShim` construction (`frame-channel.ts:315-318`) shows they agree — id from `transformCallback`, `__CHANNEL__:<id>` rehydration — but agreement is unproven at runtime.)
- **S3 — ticker thread has no shutdown and no join.** `spawn_event_pump` runs forever; on mutex poison it silently `return`s (app.rs:197-199) leaving the app alive with a dead session thread — playback freezes with no diagnostic. Deliberate ("the app is dying anyway", line 186-187) but the poison case is reachable from any panicking handler while the process continues.
- **S4 — `eprintln!` failure faces for emit/bind/send** (lines 164, 217, 224). Consistent with the crash-face design, but a webview-side failure is invisible to the user (no event reaches the UI; playback silently stops progressing).
- **S4 — on the bind tick itself, records are skipped** (`if frame_channel.is_some()` guard, app.rs:208) — one ~30 ms tick of frames lost on the first bind only. Negligible; recorded for completeness.

## Test honesty

- The wrapper's own behavior (invoke → dispatch → events) is tested ONLY headlessly through `apps/desktop/src/shell.rs` tests on `ShellRuntime` — the tauri attribute layer is untested by unit tests and unproven at runtime (repo's own statement: `apps/desktop/src/shell.rs:27-32` "What this build cannot prove (the honest deferral): the real `tauri::command` wrapper, the `app_handle.emit` calls, and the window build itself").
- The §7.4 anti-bypass test does scan this file's registration (`tests/single_command.rs:23`).

## Doc claims

- INFRA-BLOCKED.md "40 ms event pump" vs `PUMP_INTERVAL` = 30_625 µs (see index audit, S4 drift).
- KNOWN-LIMITS.md row (line 50) claims w30.7 "the ~30 ms pacer (`ShellRuntime::pump_once`) so GUI playback actually progresses" — matches the code.

## Dead code / deps / flags

- No dead code in this file. All functions are called (`apply_window_spec`, `open_argv_files`, `spawn_event_pump` from `run`'s setup, lines 298-300).
- Deps: `tauri`, `desktop`, `commands`, `serde` only — no plugin crates, no dialog plugin (pick_and_open deliberately unwired; app.rs:75-79).

## Open questions

1. Is the 32.65 fps ship ceiling acceptable for the product? 60 fps content ships every other frame at best (latest-wins), and nothing records the drop count at the ship layer (`BridgeStats` exists at the tap, but the ticker's own "dropped older records" decimation is uncounted).
2. On multi-window or webview reload mid-playback, the ticker rebuilds the channel from the next bind — the window between reload and re-bind loses frames silently. UNVERIFIED.
