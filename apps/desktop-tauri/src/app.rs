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
//! # The event pump
//!
//! Engine-originated events (progress, timing, playlist changes) arrive
//! between commands; a 40 ms ticker thread (`spawn_event_pump`) drains
//! them to the webview without waiting for the next invoke. Both the
//! command path and the pump call [`forward_pending`] under the same
//! lock, so there is exactly one ordered egress.
//!
//! # File-open seams (doc 05 §2.7 shell-routed surface)
//!
//! * drag-drop: `on_window_event` → [`ShellRuntime::on_files_dropped`]
//!   with the production [`PathDragDrop`] handler;
//! * argv files at startup (the `lumen-tauri file.mkv` form) →
//!   [`ShellRuntime::open_path`].
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
use desktop::shell::{DEFAULT_EVENT_CAPACITY, ShellRuntime};
use desktop::window_config::parse_window_table;

/// The engine → webview event channel (doc 05 §5.1: the shell forwards
/// every engine event here). Kept in one place; the UI transport listens
/// on the same name (`ui/src/client/tauri-transport.ts`).
pub const EVENT_CHANNEL: &str = "lumen://event";

/// The pump cadence: fast enough for 30 fps progress ticks, slow enough
/// to batch; both halves share one lock so cadence is not an ordering
/// input (§4.3 guarantee 2 rides the lock, not the timer).
const PUMP_INTERVAL: Duration = Duration::from_millis(40);

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
                write!(f, "shell state poisoned (a handler panicked; restart the app)")
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

/// 40 ms ticker: keeps engine-originated events flowing between invokes.
/// Stops silently when the state is poisoned (the app is dying anyway).
fn spawn_event_pump(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(PUMP_INTERVAL);
        let state = app.state::<ShellState>();
        let Ok(mut runtime) = state.0.lock() else {
            return;
        };
        forward_pending(&app, &mut runtime);
    });
}

/// Apply the REAL window config (product.toml [window]) at setup. The
/// conf's windows[0] is the first-paint fallback, not a second source of
/// truth (4.H1 design note): sizes + maximize are re-asserted here.
fn apply_window_spec(app: &AppHandle) -> tauri::Result<()> {
    let config = parse_window_table(include_str!("../../../product.toml"))
        .expect("product.toml [window]: honest-parse rule — config drift never invents a window");
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
