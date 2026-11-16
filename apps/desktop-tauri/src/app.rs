//! The ADR-004 admission wrapper (issue #67, 2026-11-13) — the tauri
//! half of the desktop shell, in the workspace-EXCLUDED crate
//! (`apps/desktop-tauri`; see its Cargo.toml header for why the wrapper
//! lives here and not behind a feature in apps/desktop).
//!
//! # What this module fulfils
//!
//! The verbatim signature promised in `apps/desktop/src/dispatch.rs`'s
//! module docs:
//!
//! ```ignore
//! #[tauri::command]
//! async fn lumen_dispatch(state: tauri::State<ShellState>,
//!                         request: CommandRequest)
//!                         -> Result<CommandResponse, CommandError> {
//!     // lock the ShellHost, call this module's lumen_dispatch
//! }
//! // ... .invoke_handler(tauri::generate_handler![lumen_dispatch]))  // EXACTLY ONE
//! ```
//!
//! Concrete wire types: `commands::WireRequest = CommandRequest<Command>`,
//! `commands::WireResponse = CommandResponse<CommandResult>` — the SAME
//! envelope every other host speaks (web, CLI, control server); no
//! desktop-private command path exists (doc 05 §5.1 single-command rule,
//! enforced by `tests/single_command.rs` — the §7.4 anti-bypass test,
//! which reads THIS file's handler registration and fails on any second
//! command).
//!
//! # Ordering discipline (doc 05 §4.3 guarantee 2 / §5.1 "events")
//!
//! One `Mutex<ShellRuntime>` serializes dispatches AND event drains; the
//! pending-event queue is emitted to the webview in engine order (`seq`
//! monotonic). Invoke responses ride tauri's invoke channel; engine
//! events ride the event channel — the shell's own order
//! (event-to-event, by `seq`) is total under the lock, which is the
//! guarantee the UI's correlation machinery consumes (§1.3).
//!
//! # The session pacer (card 3.B3) — the event pump that DRIVES
//!
//! The engine is synchronous single-writer: it advances ONLY when a
//! host pumps it (`Engine::poll` + the null-device time advance —
//! the CLI host's own `pump` semantics), and between two invokes
//! NOTHING pumps. The shell's ticker thread (`spawn_event_pump`)
//! therefore IS the session thread: every [`PUMP_INTERVAL`] tick it
//! locks the runtime, runs one [`ShellRuntime::pump_once`] (a FULL
//! engine pump + the event drain), then emits whatever the drain
//! queued — so media time advances, time-ticks flow, and EOS arrives
//! without waiting for the next invoke. Without it the GUI could
//! open/pause/seek but playback froze between interactions (the
//! card-3.B3 gap). Both the command path and the pacer call
//! [`forward_pending`] under the same lock, so there is exactly one
//! ordered egress (§4.3 guarantee 2 rides the lock, not the timer).
//!
//! # File-open seams (doc 05 §2.7 shell-routed surface)
//!
//! * drag-drop: `on_window_event` → [`ShellRuntime::on_files_dropped`]
//!   with the production [`PathDragDrop`] handler;
//! * argv files at startup (the `lumen-tauri file.mkv` form) →
//!   [`ShellRuntime::open_path`].
//!
//! The drop/argv receipts are observed here for honesty; the UI's truth
//! source is the event stream (playlist.changed etc. via the pump) — a
//! bare `WireResponse` is never emitted on the event channel (it is not
//! a `WireEvent`; §5.1 keeps the two envelopes separate).
//!
//! # Window
//!
//! `apply_window_spec` consumes `product.toml [window]` (the REAL
//! config; the first-paint `tauri.conf.json` values are fallbacks only)
//! at setup: inner/min sizes from [`WindowBuildSpec`], maximize when
//! configured. The idle-title → l10n-catalog binding joins with the
//! presenter card; until then the conf's first-paint title stands
//! (never a second source of truth — 4.H1's design note).
//!
//! `pick_and_open` (native dialog) is NOT wired here: it needs a dialog
//! plugin admission of its own (capabilities stay `core:default` only,
//! no plugins — the 4.H1 posture); recorded as a follow-up row in the
//! admission record. In-webview file inputs, drag-drop and argv cover
//! the open paths the launch verdicts exercise.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, DragDropEvent, Emitter, Manager, State, WindowEvent};

use commands::WireRequest;
use commands::WireResponse;
use desktop::files::PathDragDrop;
use desktop::shell::{DEFAULT_EVENT_CAPACITY, PACER_TICK, ShellRuntime};
use desktop::window_config::parse_window_table;

/// The engine → webview event channel (doc 05 §5.1: the shell forwards
/// every engine event here). Kept in one place; the UI transport listens
/// on the same name (`ui/src/client/tauri-transport.ts`).
// lumen-allow-literal: transport contract string — the shell↔UI IPC channel name both sides must carry literally (doc 05 §5.1, mirrored in ui/src/client/tauri-transport.ts); a wire-protocol constant, not a config value — renaming it is a coordinated two-sided change.
pub const EVENT_CHANNEL: &str = "lumen://event";

/// The pacer cadence: one null-device render quantum
/// ([`PACER_TICK`] — `RENDER_QUANTUM` = 1470 frames ≈ 30.6 ms at the
/// 48 kHz nominal, derived from the CLI host's own constant, not a
/// second clock), so the ticker advances the engine at the quantum
/// the null device renders per pump. Fast enough for 30 fps progress
/// ticks, slow enough to batch; both halves share one lock so cadence
/// is not an ordering input (§4.3 guarantee 2 rides the lock, not the
/// timer).
const PUMP_INTERVAL: Duration = PACER_TICK;

/// The managed state: one [`ShellRuntime`] behind one lock — THE
/// serialization point of the whole shell (commands + pump + drops).
pub struct ShellState(pub Mutex<ShellRuntime>);

/// Transport-level failure envelope for the single command. App-level
/// refusals are NOT this: they ride inside [`WireResponse`] (the
/// truthful-UI split, doc 05 §1.4). Only state poisoning fails here.
#[derive(Debug, serde::Serialize)]
#[serde(tag = "error", rename_all = "kebab-case")]
pub enum ShellError {
    /// The ShellState mutex was poisoned by a panic in another handler.
    StatePoisoned,
}

impl std::fmt::Display for ShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellError::StatePoisoned => {
                write!(
                    f,
                    "shell state poisoned (a handler panicked; restart the app)"
                )
            }
        }
    }
}

impl std::error::Error for ShellError {}

/// THE command (doc 05 §5.1) — the only one ever registered.
///
/// Locks the runtime, dispatches through the same routing body every
/// host uses, then drains pending events in order.
#[tauri::command]
async fn lumen_dispatch(
    app: AppHandle,
    state: State<'_, ShellState>,
    request: WireRequest,
) -> Result<WireResponse, ShellError> {
    let mut runtime = state.0.lock().map_err(|_| ShellError::StatePoisoned)?;
    let response = runtime.dispatch(&request);
    forward_pending(&app, &mut runtime);
    Ok(response)
}

/// Drain pending events to the webview, in engine order.
///
/// Emits happen UNDER the lock: event-to-event order (by `seq`) must be
/// total, and an emit is a fast queue-to-webview handoff — worth holding
/// the lock for; a slow webview cannot reorder the engine's truth.
fn forward_pending(app: &AppHandle, runtime: &mut ShellRuntime) {
    runtime.pump_events();
    for event in runtime.take_pending_events() {
        // lumen-allow-literal: diagnostic text (transport failure notice), not identity wiring
        if let Err(error) = app.emit(EVENT_CHANNEL, &event) {
            eprintln!("event emit failed on the shell channel: {error}");
        }
    }
}

/// The session pacer ticker (card 3.B3): every [`PUMP_INTERVAL`] it
/// locks the runtime, advances the engine one FULL pump
/// ([`ShellRuntime::pump_once`] — poll + time advance + end-tail
/// play-out) and then emits what the drain queued. This thread IS
/// the session thread — between invokes nothing else pumps, so
/// playback progresses only while this loop runs. Pause-safe
/// unconditionally (a frozen clock pulls nothing).
///
/// w31.1: the ticker is also the frame shipper. After each pump it
/// takes the webview's channel bind (if the UI sent a fresh
/// `session.set-frame-channel` since the last tick) and builds the
/// send-side [`FrameChannel`], then drains the frame tap's mailbox
/// and ships the NEWEST record as one raw binary message (the
/// one-frame-per-tick contract — the webview composites at the
/// pacer's own cadence; the mailbox's latest-wins bound absorbed
/// anything older). Both halves run OUTSIDE the runtime lock: the
/// channel build and the send are webview-side work, never engine
/// work. Stops silently when the state is poisoned (the app is
/// dying anyway).
fn spawn_event_pump(app: AppHandle) {
    std::thread::spawn(move || {
        // The send side of the webview's frame channel — rebuilt on
        // every bind (a webview reload invalidates the old callback
        // id; the UI re-binds and the next tick takes the new one).
        let mut frame_channel: Option<crate::frame_channel::FrameChannel> = None;
        loop {
            std::thread::sleep(PUMP_INTERVAL);
            let state = app.state::<ShellState>();
            let Ok(mut runtime) = state.0.lock() else {
                return;
            };
            // Drive first, then drain: the pump advances the engine
            // AND records the events; forward_pending ships them.
            runtime.pump_once();
            forward_pending(&app, &mut runtime);
            // w31.1: take the bind + the drained records under the
            // lock, ship after releasing it (webview-side work).
            let bind = runtime.take_frame_channel_binding();
            let mut records = Vec::new();
            if frame_channel.is_some() {
                runtime.take_frame_records(&mut records);
            }
            drop(runtime);
            if let Some(id) = bind {
                match crate::frame_channel::frame_channel(&app, id) {
                    Ok(channel) => frame_channel = Some(channel),
                    Err(error) => {
                        // lumen-allow-literal: diagnostic text (the bind failure notice the CI launch smokes grep), not identity wiring
                        eprintln!("frame channel bind failed: {error}");
                    }
                }
            }
            if let (Some(channel), Some(record)) = (&frame_channel, records.last()) {
                if let Err(error) = crate::frame_channel::send_record(channel, record) {
                    // lumen-allow-literal: diagnostic text (the ship failure notice), not identity wiring
                    eprintln!("frame channel send failed (dropping until the next bind): {error}");
                    frame_channel = None;
                }
            }
        }
    });
}

/// Apply the REAL window config (product.toml [window]) at setup. The
/// conf's windows[0] is the first-paint fallback, not a second source of
/// truth (4.H1 design note): sizes + maximize are re-asserted here.
fn apply_window_spec(app: &AppHandle) -> tauri::Result<()> {
    // The honest-parse rule: config drift never invents a window — a
    // malformed [window] table is a hard startup failure carrying the
    // parser's own message. `expect_used` is denied crate-wide (the
    // workspace lint posture), so the old `.expect("…")` face becomes a
    // returned `tauri::Error` (Io variant wraps any custom message).
    let config = parse_window_table(include_str!("../../../product.toml")).map_err(|error| {
        tauri::Error::Io(std::io::Error::other(format!(
            "product.toml [window]: honest-parse rule — config drift never invents a window: {error}"
        )))
    })?;
    let spec = config.build_spec();
    if let Some(window) = app.get_webview_window("main") {
        // Tauri 2 sizing API: `set_size`/`set_min_size` take a `Size`
        // (Logical pixels here — the conf's width/height are logical and
        // the product spec is logical by definition, 4.H1). The Tauri-1
        // (w,h) tuple methods died on the first real compile (CI
        // tauri-check, run 43384519598).
        window.set_size(tauri::LogicalSize::new(
            f64::from(spec.inner_size.0),
            f64::from(spec.inner_size.1),
        ))?;
        window.set_min_size(Some(tauri::LogicalSize::new(
            f64::from(spec.min_inner_size.0),
            f64::from(spec.min_inner_size.1),
        )))?;
        if spec.maximized {
            window.maximize()?;
        }
    }
    Ok(())
}

/// Open media files handed as argv at startup (`lumen-tauri file.mkv`).
fn open_argv_files(app: &AppHandle) {
    let state = app.state::<ShellState>();
    let Ok(mut runtime) = state.0.lock() else {
        return;
    };
    for arg in std::env::args().skip(1) {
        let path = PathBuf::from(arg);
        if path.is_file() {
            // Receipt observed for honesty; the UI's truth is the event
            // stream (§5.1: responses and events never share a channel).
            let _receipt = runtime.open_path(&path);
        }
    }
    forward_pending(app, &mut runtime);
}

/// The admission entry: build, register EXACTLY ONE command, run.
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .manage(ShellState(Mutex::new(ShellRuntime::new(
            DEFAULT_EVENT_CAPACITY,
        ))))
        .invoke_handler(tauri::generate_handler![lumen_dispatch])
        .setup(|app| {
            // The setup closure receives `&mut App`; the helpers speak in
            // `&AppHandle` (they are also called from the drag-drop
            // handler, which only has a handle) — `app.handle()` is the
            // bridge. Passing `app` itself died on the first real compile
            // (CI tauri-check, run 43384519598, two E0308s).
            apply_window_spec(app.handle())?;
            spawn_event_pump(app.handle().clone());
            open_argv_files(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            let WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) = event else {
                return;
            };
            let app = window.app_handle();
            let state = app.state::<ShellState>();
            let Ok(mut runtime) = state.0.lock() else {
                return;
            };
            let mut handler = PathDragDrop;
            // Receipt observed for honesty; the UI's truth is the event
            // stream (the drop's open dispatch surfaces as playlist
            // events through the pump, §5.1).
            let _receipt = runtime.on_files_dropped(&mut handler, paths);
            forward_pending(app, &mut runtime);
        })
        .run(tauri::generate_context!())
}
