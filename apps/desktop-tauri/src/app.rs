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
//! host pumps it (`Engine::poll` + the time advance), and between two
//! invokes NOTHING pumps. The shell's ticker thread (`spawn_event_pump`)
//! therefore IS the session thread: every [`SESSION_TICK`] it locks
//! the runtime, runs one [`ShellRuntime::pump_session`] (a
//! **wall-bounded** engine pump + the event drain — the device renders
//! only the quanta the wall has become owed since the Playing anchor,
//! never the whole ring), then emits whatever the drain queued — so
//! media time advances at 1.0× real time, time-ticks flow, and EOS
//! arrives without waiting for the next invoke. The engine-side
//! decode park (`FrameConsumer::ready_for`, w27) keeps the audio ring
//! bounded between ticks, so the cadence is stable under tick jitter:
//! a slow tick catches up only to the wall, never past it. Both the
//! command path and the pacer call [`forward_pending`] under the same
//! lock, so there is exactly one ordered egress (§4.3 guarantee 2
//! rides the lock, not the timer).
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
use desktop::shell::{ShellRuntime, DEFAULT_EVENT_CAPACITY};
use desktop::window_config::parse_window_table;
use desktop::wire::{MAGIC, MIN_HEADER_LEN, VERSION};

/// The engine → webview event channel (doc 05 §5.1: the shell forwards
/// every engine event here). Kept in one place; the UI transport listens
/// on the same name (`ui/src/client/tauri-transport.ts`).
// lumen-allow-literal: transport contract string — the shell↔UI IPC channel name both sides must carry literally (doc 05 §5.1, mirrored in ui/src/client/tauri-transport.ts); a wire-protocol constant, not a config value — renaming it is a coordinated two-sided change.
pub const EVENT_CHANNEL: &str = "lumen://event";

/// The session-thread cadence: fast enough to render every 33 ms
/// audio quantum on schedule (the wall-bounded advance never renders
/// more than the wall is owed, so a faster tick costs only a cheap
/// no-op poll) and to keep 30 fps progress ticks and the frame
/// channel's latest-wins shipping smooth. Both halves share one lock
/// so cadence is not an ordering input (§4.3 guarantee 2 rides the
/// lock, not the timer).
const SESSION_TICK: Duration = Duration::from_millis(10);

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

// ---------------------------------------------------------------------------
// FX6-A02 — the ticker lane's stamps + receipt surface (FIX-PLAN §3
// F1.2)
//
// The audited defect (apps/desktop-tauri/src/app.rs.audit.md, F-005c;
// wave-FX6 row FX6-A02): the 10 ms ticker shipped `records.last()`
// only — the intermediate frames were dropped silently, no policy
// made the choice, no timestamp was read off the shipped record, and
// no counter observed the lane. FX6-A03's bounded lane
// (`crate::frame_channel::FrameLane`, landed first in this wave) now
// carries the policy and the counters between the ticker's drain and
// the webview dispatch (offer/take_newest, drop-oldest,
// flush-on-rebind, conserving stats — see the frame_channel module
// docs); the mailbox's own produced/dropped counters and drop
// attribution are FX6-A01's surface (`apps/desktop/src/mailbox.rs`,
// landed: `drain_to`, `frames_produced`/`frames_dropped`). THIS unit
// is the remaining half of row A02: the shipped record's own wire
// stamps read and latched where the lane can see them (the
// timestamp passthrough, made observable), and the periodic
// produced-vs-dropped receipt over the bounded lane's counters —
// the measurement surface the F1.2 hardware run reads off stderr.
// ---------------------------------------------------------------------------

/// How often the lane's stats receipt hits stderr, in shipped records
/// (128 ships ≈ 4 s at 30 fps): the measurement granularity a hardware
/// run greps the receipts at — F1.2's "instrument: frames produced vs
/// frames dropped" bullet. Failure notices stay immediate; this
/// cadence carries the counters.
const FRAME_RECEIPT_EVERY: u64 = 128;

/// The wire stamps read off a shipped record's header — the F1.2
/// timestamp passthrough. A LUF1 v1 record carries `frame_id`,
/// `pts_ns` and `emitted_ns` at fixed offsets (apps/desktop/src/
/// wire.rs's layout table); the pre-FX6 ticker never read them, so the
/// shipped record's timestamps were invisible to every receipt. A
/// pure header peek: no payload copy, no decoder state (the mailbox
/// keeps the frames' stamps through its drain — FX6-A01's drain
/// passthrough — and this reads them back off the record).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordStamps {
    /// Monotonic per-stream frame id — the freshness key: the freshest
    /// record the mailbox yielded is the highest id, whatever the
    /// drain order.
    pub frame_id: u64,
    /// The scheduled presentation slot, ns on the media clock's base
    /// (the A/V-sync observable).
    pub pts_ns: i64,
    /// The bridge's serialize instant, ns on its monotonic emit clock
    /// (the transport-latency accounting origin).
    pub emitted_ns: u64,
}

/// Read one drained record's stamps (the frozen v1 offsets: `frame_id`
/// at 24, `pts_ns` at 32, `emitted_ns` at 40, little-endian, inside the
/// 48-byte fixed header). `None` when the bytes are not a readable
/// LUF1 v1 record — a lane bug the caller reports (a `None` latch
/// shows in the receipt): the crate's discipline is that nothing
/// unreadable rides the lane unannounced.
fn record_stamps(record: &[u8]) -> Option<RecordStamps> {
    if record.len() < MIN_HEADER_LEN
        || &record[0..4] != &MAGIC[..]
        || u16::from_le_bytes([record[6], record[7]]) != VERSION
    {
        return None;
    }
    let field = |from: usize| {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&record[from..from + 8]);
        u64::from_le_bytes(bytes)
    };
    Some(RecordStamps {
        frame_id: field(24),
        pts_ns: field(32) as i64,
        emitted_ns: field(40),
    })
}

/// The dispatch observer — row A02's half of the ticker lane: the
/// stamps latched off the record the dispatch half took for the ship,
/// and the periodic produced-vs-dropped receipt built over FX6-A03's
/// bounded-lane counters (see the section docs above).
#[derive(Debug, Default)]
pub struct ShipLog {
    last_stamps: Option<RecordStamps>,
}

impl ShipLog {
    /// Latch the stamps off the record being dispatched — read from
    /// the record's own header bytes (the lane ships the exact bytes
    /// it was offered; this reads the timestamps that rode them, the
    /// F1.2 passthrough made observable). An unreadable record (a
    /// lane bug — the tap's encoder validated every record it
    /// drained) latches `None`: the receipt then carries the counters
    /// without a stamp line — the anomaly visible, never silent.
    pub fn note_dispatched(&mut self, record: &[u8]) {
        self.last_stamps = record_stamps(record);
    }

    /// The stamps latched off the last dispatched record.
    #[must_use = "observable evidence"]
    pub fn last_stamps(&self) -> Option<RecordStamps> {
        self.last_stamps
    }

    /// The one-line lane receipt (the app's stderr diagnostic
    /// convention — the same face the bind/ship failure notices use):
    /// the bounded lane's produced-vs-dropped counters plus the last
    /// dispatched record's stamps. The dispatch half prints it every
    /// 128 ships (FRAME_RECEIPT_EVERY) — the measurement cadence
    /// the F1.2 hardware run greps stderr at; FX6-A03's stall notice
    /// stays immediate, this one carries the totals.
    #[must_use = "the receipt line, ready to print"]
    pub fn receipt(&self, lane: &crate::frame_channel::FrameLaneStats) -> String {
        let stamps = match self.last_stamps {
            Some(stamps) => format!(
                " last: frame_id {} pts_ns {} emitted_ns {}",
                stamps.frame_id, stamps.pts_ns, stamps.emitted_ns
            ),
            None => String::new(),
        };
        format!(
            "shipped {} produced {} dropped {} (overflow {} superseded {} stale {}){stamps}",
            lane.shipped,
            lane.offered,
            lane.dropped_overflow + lane.dropped_superseded + lane.dropped_stale,
            lane.dropped_overflow,
            lane.dropped_superseded,
            lane.dropped_stale
        )
    }
}

/// The session pacer ticker (card 3.B3): every [`SESSION_TICK`] it
/// locks the runtime, advances the engine one **wall-bounded** pump
/// ([`ShellRuntime::pump_session`] — poll + the wall-owed time
/// advance + end-tail play-out) and then emits what the drain
/// queued. This thread IS the session thread — between invokes
/// nothing else pumps, so playback progresses only while this loop
/// runs. Pause-safe unconditionally (a frozen clock pulls nothing).
///
/// w31.1: the ticker is also the frame shipper. After each pump it
/// takes the webview's channel bind (if the UI sent a fresh
/// `session.set-frame-channel` since the last tick) and builds the
/// send-side [`FrameChannel`], then drains the frame tap's mailbox
/// through the bounded [`crate::frame_channel::FrameLane`] and ships
/// the newest record as one raw binary message (the
/// one-frame-per-tick contract — the webview composites at the
/// pacer's own cadence; the lane's latest-wins bound drops the
/// superseded records by policy, counted, where the old
/// `records.last()` discarded them silently). Both halves run
/// OUTSIDE the runtime lock: the channel build and the send are
/// webview-side work, never engine work. Stops silently when the
/// state is poisoned (the app is dying anyway).
fn spawn_event_pump(app: AppHandle) {
    std::thread::spawn(move || {
        // The send side of the webview's frame channel — rebuilt on
        // every bind (a webview reload invalidates the old callback
        // id; the UI re-binds and the next tick takes the new one).
        let mut frame_channel: Option<crate::frame_channel::FrameChannel> = None;
        // FX6-A03 (FIX-PLAN §3 F1.2 / W14-A07): the bounded ship side
        // — the queue feeding the tauri Channel is an explicit,
        // counted lane (latest-wins, drop-oldest on overflow), not
        // the implicit records buffer whose `last()` ship discarded
        // the rest silently.
        let mut lane = crate::frame_channel::FrameLane::new();
        // FX6-A02: the dispatch observer — the dispatched record's
        // stamps latched per dispatch, and the periodic
        // produced-vs-dropped receipt built over the lane's counters
        // (see [`ShipLog`]).
        let mut ship_log = ShipLog::default();
        // Reused across ticks (drained in place — the allocation
        // persists, not a fresh `Vec::new()` per tick).
        let mut records: Vec<Vec<u8>> = Vec::new();
        // One stall notice per full-lane episode (the transition),
        // never per tick.
        let mut lane_full_notified = false;
        loop {
            std::thread::sleep(SESSION_TICK);
            let state = app.state::<ShellState>();
            let Ok(mut runtime) = state.0.lock() else {
                return;
            };
            // Drive first, then drain: the pump advances the engine
            // AND records the events; forward_pending ships them.
            runtime.pump_session();
            forward_pending(&app, &mut runtime);
            // w31.1/FX6-A03: take the bind + the drained records
            // under the lock, ship after releasing it (webview-side
            // work). A fresh bind counts as a drain trigger too —
            // the lane holds the tick's records while the channel is
            // rebuilt, so the first frame after a (re)bind no longer
            // waits a tick.
            let bind = runtime.take_frame_channel_binding();
            if frame_channel.is_some() || bind.is_some() {
                runtime.take_frame_records(&mut records);
            }
            drop(runtime);
            if let Some(id) = bind {
                match crate::frame_channel::frame_channel(&app, id) {
                    Ok(channel) => {
                        // A bind is a fresh generation (the webview
                        // reloaded): anything still queued is from
                        // the previous page — flushed and counted,
                        // never shipped stale.
                        lane.flush_stale();
                        frame_channel = Some(channel);
                    }
                    Err(error) => {
                        // lumen-allow-literal: diagnostic text (the bind failure notice the CI launch smokes grep), not identity wiring
                        eprintln!("frame channel bind failed: {error}");
                    }
                }
            }
            for record in records.drain(..) {
                lane.offer(record);
            }
            // The dispatch half: ship the lane's newest record (the
            // one-frame-per-tick contract — the webview composites at
            // the pacer's own cadence; the superseded older records
            // are counted drops at the lane, never a silent
            // `records.last()` discard). FX6-A02: the dispatched
            // record's stamps are latched before the bytes move (the
            // timestamp passthrough), and every FRAME_RECEIPT_EVERY
            // ships the lane's produced-vs-dropped counters hit stderr
            // with them.
            if let Some(channel) = &frame_channel {
                if let Some(record) = lane.take_newest() {
                    ship_log.note_dispatched(&record);
                    match crate::frame_channel::send_record(channel, record) {
                        Ok(()) => {
                            let stats = lane.stats();
                            if stats.shipped % FRAME_RECEIPT_EVERY == 0 {
                                // lumen-allow-literal: diagnostic text (the lane's periodic stats receipt — counters + the dispatched record's stamps), not identity wiring
                                eprintln!("frame lane: {}", ship_log.receipt(&stats));
                            }
                        }
                        Err(error) => {
                            // lumen-allow-literal: diagnostic text (the ship failure notice), not identity wiring
                            eprintln!(
                                "frame channel send failed (dropping until the next bind): {error}"
                            );
                            frame_channel = None;
                        }
                    }
                }
            }
            // The stall notice (the lane's occupancy instrumentation,
            // the crate's logged-diagnostics convention): a full lane
            // is the ship-side stall signature — offers keep flowing
            // while the dispatch stopped taking. One line per
            // episode, totals in the line.
            if lane.is_full() {
                if !lane_full_notified {
                    let stats = lane.stats();
                    // lumen-allow-literal: diagnostic text (the stall notice), not identity wiring
                    eprintln!(
                        "frame lane at capacity ({} queued): dropping oldest records by policy (shipped {}, overflow-dropped {})",
                        lane.queued(),
                        stats.shipped,
                        stats.dropped_overflow
                    );
                    lane_full_notified = true;
                }
            } else {
                lane_full_notified = false;
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
