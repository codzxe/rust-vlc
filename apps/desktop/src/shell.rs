//! The app-layer shell runtime (issue #3 — the 4.H1 app-layer wiring):
//! the exact composition the ADR-004 tauri wrapper drives. Everything
//! the webview host needs, proven headlessly:
//!
//! - [`ShellRuntime::dispatch`] — one UI-originated command: allocate
//!   the request id (§4.3 echo legality), route through THE single
//!   [`lumen_dispatch`](crate::dispatch::lumen_dispatch), observe the
//!   response (§4.2/§4.3 bookkeeping), then drain every new engine
//!   event through the staleness filter into the bounded egress queue.
//! - [`ShellRuntime::open_path`] / [`on_files_dropped`] /
//!   [`pick_and_open`] — the file-open acquisitions (dialog,
//!   drag-drop, direct path) funneling through the SAME command —
//!   there is no second file-open mechanism.
//! - [`ShellRuntime::take_pending_events`] — the emitter half: what
//!   the wrapper emits on `"lumen://event"`, in engine order.
//!
//! What this build cannot prove (the honest deferral): the real
//! `tauri::command` wrapper, the `app_handle.emit` calls, and the
//! window build itself — webkit2gtk-4.1 is absent from this sandbox
//! (ADR-004; `apps/desktop/tauri/INFRA-BLOCKED.md`). The runtime is
//! the seam those calls delegate to; every method here runs under
//! `cargo test` today.

use std::num::NonZeroUsize;
use std::path::Path;
use std::path::PathBuf;

use commands::envelope::RequestId;
use commands::{WireEvent, WireRequest, WireResponse};

use crate::dispatch::BoundedEventSink;
use crate::dispatch::EventQueueStats;
use crate::dispatch::EventSink;
use crate::dispatch::ShellHost;
use crate::dispatch::lumen_dispatch;
use crate::dispatch::open_path_request;
use crate::error::HostError;
use crate::files::DragDropHandler;
use crate::files::NativeFileDialog;
use crate::host::DesktopHost;
use crate::host::EventDelivery;

/// The default egress capacity (the mailbox default's reasoning: one
/// event per command class plus headroom for a burst of time-ticks;
/// overflow drops the OLDEST event, counted — order is never violated).
pub const DEFAULT_EVENT_CAPACITY: NonZeroUsize = {
    match NonZeroUsize::new(64) {
        Some(value) => value,
        // The impossible arm is a compile-time dead end (the 2.C4
        // convention), not a banned-macro escape hatch.
        #[allow(clippy::empty_loop)] // clippy's suggested `panic!()` is workspace-denied
        None => loop {},
    }
};

/// What one dialog attempt concluded.
#[derive(Debug, Clone)]
pub enum DialogOutcome {
    /// A path was picked and its open command dispatched; the wire
    /// response is the engine's answer (media: `session.open`;
    /// playlist: `playlist.load`). Boxed — the wire response is the
    /// enum's largest field by far and the outcome is moved, not
    /// stored in bulk (clippy's own advice).
    Opened(Box<WireResponse>),
    /// The user canceled the modal — a no-op, never an error.
    Canceled,
    /// The webview layer is not linked in this build (the typed
    /// ADR-004 pending state).
    Pending(HostError),
}

/// The composed shell: correlation (host.rs) + engine routing
/// (dispatch.rs) + the bounded event egress, in one drivable object.
pub struct ShellRuntime {
    host: DesktopHost,
    shell: ShellHost,
    events: BoundedEventSink,
}

impl Default for ShellRuntime {
    fn default() -> Self {
        Self::new(DEFAULT_EVENT_CAPACITY)
    }
}

impl ShellRuntime {
    /// A fresh runtime with an explicit egress capacity.
    #[must_use = "pure constructor"]
    pub fn new(event_capacity: NonZeroUsize) -> ShellRuntime {
        ShellRuntime {
            host: DesktopHost::new(),
            shell: ShellHost::new(),
            events: BoundedEventSink::with_capacity(event_capacity),
        }
    }

    /// The engine-driving host (test/diagnostic access; every command
    /// still flows through [`Self::dispatch`]).
    #[must_use = "the adopted composition pattern"]
    pub fn engine_host(&mut self) -> &mut lumen_cli::host::CliHost {
        self.shell.engine_host()
    }

    /// Allocate the next request id (monotonic from 1; the §4.3 log
    /// learns it, so responses to it are legal).
    pub fn next_request_id(&mut self) -> RequestId {
        self.host.next_request_id()
    }

    /// THE dispatch path for UI-originated commands: route through the
    /// single `lumen_dispatch`, observe the response, then drain every
    /// new engine event (§4.2 filter first) into the bounded queue —
    /// the wrapper's per-invoke body.
    pub fn dispatch(&mut self, request: &WireRequest) -> WireResponse {
        let response = lumen_dispatch(&mut self.shell, request);
        let _ = self.host.observe_response(&response);
        self.pump_events();
        response
    }

    /// Drain new engine events through the §4.2 staleness filter into
    /// the bounded egress queue (also what a periodic wrapper tick
    /// calls between invokes).
    pub fn pump_events(&mut self) {
        let host = &mut self.host;
        let events = &mut self.events;
        let mut deliver = DeliverThrough { host, events };
        let _ = self.shell.forward_new_events(&mut deliver);
    }

    /// The events awaiting webview emission, in engine order (each
    /// becomes one `lumen://event` emit; the queue empties).
    pub fn take_pending_events(&mut self) -> Vec<WireEvent> {
        self.events.take()
    }

    /// The egress accounting (bounded-queue honesty).
    #[must_use = "observable evidence"]
    pub fn event_stats(&self) -> &EventQueueStats {
        self.events.stats()
    }

    /// Open one shell-acquired path (the direct funnel: dialog pick,
    /// drop translation, or any host integration). Media files open a
    /// session; playlist files load the list; anything else is the
    /// honest no-op (`None`).
    pub fn open_path(&mut self, path: &Path) -> Option<WireResponse> {
        let request_id = self.host.next_request_id();
        let request = open_path_request(request_id, path)?;
        Some(self.dispatch(&request))
    }

    /// Translate one drop event's paths through the handler and
    /// dispatch the result (a drop with no openable file is a no-op).
    pub fn on_files_dropped(
        &mut self,
        handler: &mut dyn DragDropHandler,
        paths: &[PathBuf],
    ) -> Option<WireResponse> {
        let request_id = self.host.next_request_id();
        match handler.on_files_dropped(paths, request_id) {
            Ok(Some(request)) => Some(self.dispatch(&request)),
            Ok(None) => None,
            Err(error) => {
                // The seam's own typed failure: surfaced as the wire
                // error envelope's shape demands a response — the
                // honest refusal, echoed for the allocated id.
                Some(refusal_response(request_id, &error))
            }
        }
    }

    /// Run the native file dialog and dispatch the pick (the wrapper's
    /// dialog-button body). Cancel is a no-op; the not-linked build
    /// answers the typed pending state.
    pub fn pick_and_open(&mut self, dialog: &mut dyn NativeFileDialog) -> DialogOutcome {
        match dialog.pick_media_file() {
            Ok(Some(path)) => match self.open_path(&path) {
                Some(response) => DialogOutcome::Opened(Box::new(response)),
                // The dialog's filter owns pickability; a picked path
                // that classifies to nothing is the honest no-op.
                None => DialogOutcome::Canceled,
            },
            Ok(None) => DialogOutcome::Canceled,
            Err(error) => DialogOutcome::Pending(error),
        }
    }
}

/// The §4.2-filtered delivery: stale events are counted and refused,
/// current events enter the bounded queue (composition, not a copy).
struct DeliverThrough<'a> {
    host: &'a mut DesktopHost,
    events: &'a mut BoundedEventSink,
}

impl EventSink for DeliverThrough<'_> {
    fn forward(&mut self, event: &WireEvent) {
        match self.host.deliver_event(event) {
            EventDelivery::Kept => self.events.forward(event),
            EventDelivery::DroppedStale => self.events.note_stale(),
        }
    }
}

/// The seam-failure answer: a typed error envelope shaped as the wire
/// response the UI already knows how to render (the dialog/drop seams
/// fail host-side, never engine-side — the UI still gets a wire
/// answer, never silence).
fn refusal_response(request_id: RequestId, error: &HostError) -> WireResponse {
    WireResponse {
        request_id,
        session: None,
        generation: commands::envelope::Generation(0),
        result: Err(commands::CommandError::new(
            commands::Stage::Control,
            commands::ErrorCode::NotReady,
            error.to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commands::Command;
    use commands::CommandOutcome;
    use commands::ErrorCode;
    use std::path::PathBuf;
    use testkit::Must;

    /// A scripted dialog — the test half of the acquisition seam.
    struct ScriptedDialog {
        answer: Result<Option<PathBuf>, HostError>,
    }

    impl NativeFileDialog for ScriptedDialog {
        fn pick_media_file(&mut self) -> Result<Option<PathBuf>, HostError> {
            self.answer.clone()
        }
    }

    fn request(id: RequestId, command: Command) -> WireRequest {
        WireRequest {
            request_id: id,
            command,
        }
    }

    /// The full dispatch loop over the REAL engine: a served command
    /// (the settings seam — no session needed) answers completed and
    /// the §4.3 log accepts it because the runtime allocated the id.
    #[test]
    fn dispatch_routes_observes_and_emits() {
        let mut runtime = ShellRuntime::default();
        let id = runtime.next_request_id();
        let response = runtime.dispatch(&request(
            id,
            Command::ConfigSetSetting {
                area: "interface".to_string(),
                key: "theme".to_string(),
                value: commands::catalog::types::SettingValue::String("dark".to_string()),
            },
        ));
        assert_eq!(response.request_id, id);
        match response.result {
            Ok(CommandOutcome::Completed(commands::CommandResult::SettingApplied {
                applied,
                ..
            })) => {
                assert_eq!(
                    applied,
                    commands::catalog::types::SettingValue::String("dark".to_string())
                );
            }
            other => unreachable!("config.set-setting must complete: {other:?}"),
        }
        // The runtime's egress queue conserves: nothing was offered by
        // a class-S settings command.
        assert!(runtime.event_stats().conserves());
    }

    /// The config round-trip over the single command (issue #3,
    /// handoff d): set → get answers the applied value through the
    /// same dispatch path the UI uses.
    #[test]
    fn config_writes_round_trip_through_dispatch() {
        let mut runtime = ShellRuntime::default();
        let set_id = runtime.next_request_id();
        let response = runtime.dispatch(&request(
            set_id,
            Command::ConfigSetSetting {
                area: "interface".to_string(),
                key: "theme".to_string(),
                value: commands::catalog::types::SettingValue::String("dark".to_string()),
            },
        ));
        match response.result {
            Ok(CommandOutcome::Completed(commands::CommandResult::SettingApplied {
                applied,
                ..
            })) => assert_eq!(
                applied,
                commands::catalog::types::SettingValue::String("dark".to_string())
            ),
            other => unreachable!("set must complete: {other:?}"),
        }
        let get_id = runtime.next_request_id();
        let response = runtime.dispatch(&request(
            get_id,
            Command::ConfigGetSetting {
                area: "interface".to_string(),
                key: "theme".to_string(),
            },
        ));
        match response.result {
            Ok(CommandOutcome::Completed(commands::CommandResult::SettingResult {
                value, ..
            })) => assert_eq!(
                value,
                commands::catalog::types::SettingValue::String("dark".to_string())
            ),
            other => unreachable!("get must complete: {other:?}"),
        }
    }

    /// An invalid settings write refuses typed (the engine validates
    /// against the embedded schema — never a silent coercion).
    #[test]
    fn invalid_config_writes_refuse_typed() {
        let mut runtime = ShellRuntime::default();
        let id = runtime.next_request_id();
        let response = runtime.dispatch(&request(
            id,
            Command::ConfigSetSetting {
                area: "no-such-area".to_string(),
                key: "no-such-key".to_string(),
                value: commands::catalog::types::SettingValue::Bool(true),
            },
        ));
        match response.result {
            Err(error) => assert_eq!(error.code, ErrorCode::InvalidArgument, "{error}"),
            Ok(_) => unreachable!("an unknown settings key must refuse"),
        }
    }

    /// A real open → settle → events flow reaches the bounded egress
    /// queue in engine order (the live feed the wrapper emits).
    #[test]
    fn engine_events_reach_the_bounded_queue_in_order() {
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/generated/valid/avi_mjpeg_pcm.avi"
        );
        let mut runtime = ShellRuntime::default();
        let response = match runtime.open_path(&PathBuf::from(fixture)) {
            Some(response) => response,
            None => unreachable!("a media file must classify"),
        };
        let _ = response;
        runtime.engine_host().settle();
        runtime.pump_events();
        let events = runtime.take_pending_events();
        assert!(!events.is_empty(), "a settled open must produce events");
        let seqs: Vec<u64> = events.iter().map(|event| event.seq).collect();
        let mut sorted = seqs.clone();
        sorted.sort_unstable();
        assert_eq!(seqs, sorted, "engine order preserved end to end");
        // Monotonic drain: nothing re-delivered.
        assert!(runtime.take_pending_events().is_empty());
        assert!(runtime.event_stats().conserves());
        assert_eq!(runtime.event_stats().taken, events.len() as u64);
    }

    /// A playlist file loads the list, not a session (4.D1's handoff):
    /// the request reaches the engine through the same single command
    /// and answers the honest typed refusal for the unserved id.
    #[test]
    fn playlist_files_load_the_list_through_the_command() {
        let mut runtime = ShellRuntime::default();
        let response = match runtime.open_path(&PathBuf::from("/tmp/queue.m3u")) {
            Some(response) => response,
            None => unreachable!("a playlist file must classify"),
        };
        match response.result {
            Err(error) => {
                assert_eq!(error.code, ErrorCode::UnsupportedForHost, "{error}");
                assert_eq!(
                    error.details.get("command").map(String::as_str),
                    Some("playlist.load")
                );
            }
            Ok(_) => unreachable!("playlist.load is not served by this engine build"),
        }
        assert_eq!(runtime.engine_host().session_id(), None);
    }

    /// Non-openable paths are honest no-ops (never a session error the
    /// UI must handle).
    #[test]
    fn non_openable_paths_are_noops() {
        let mut runtime = ShellRuntime::default();
        assert!(
            runtime
                .open_path(&PathBuf::from("/tmp/README.md"))
                .is_none()
        );
        assert!(
            runtime
                .open_path(&PathBuf::from("/tmp/queue.unknown-ext"))
                .is_none()
        );
    }

    /// The drop funnel through the REAL handler: first openable file
    /// wins, playlist classification included.
    #[test]
    fn drops_funnel_through_the_runtime() {
        let mut runtime = ShellRuntime::default();
        let mut dnd = crate::files::PathDragDrop;
        let response = match runtime.on_files_dropped(
            &mut dnd,
            &[
                PathBuf::from("/tmp/notes.txt"),
                PathBuf::from("/tmp/movie.avi"),
            ],
        ) {
            Some(response) => response,
            None => unreachable!("a media file was dropped"),
        };
        // The open path's own honest answer decides (the fixture is
        // real); what is proven here is the FUNNEL, not the decode.
        let _ = response;
        // A drop with nothing openable is a no-op.
        assert!(
            runtime
                .on_files_dropped(&mut dnd, &[PathBuf::from("/tmp/README.md")])
                .is_none()
        );
    }

    /// The dialog wiring: a pick dispatches through the same command;
    /// cancel is a no-op; the not-linked build answers the typed
    /// pending state.
    #[test]
    fn dialog_picks_dispatch_cancels_noop_and_pending() {
        let mut runtime = ShellRuntime::default();
        // Canceled: a no-op, never an error.
        let mut canceled = ScriptedDialog { answer: Ok(None) };
        assert!(matches!(
            runtime.pick_and_open(&mut canceled),
            DialogOutcome::Canceled
        ));
        // Pending: the typed ADR-004 state, verbatim.
        let mut pending = ScriptedDialog {
            answer: Err(HostError::WebviewPending { adr: "ADR-004" }),
        };
        match runtime.pick_and_open(&mut pending) {
            DialogOutcome::Pending(error) => {
                assert_eq!(error, HostError::WebviewPending { adr: "ADR-004" });
            }
            other => unreachable!("no webview is linked: {other:?}"),
        }
        // Picked: the file-open funnel (an unserved path class still
        // answers through the command — here a playlist, refused typed).
        let mut picked = ScriptedDialog {
            answer: Ok(Some(PathBuf::from("/tmp/queue.pls"))),
        };
        match runtime.pick_and_open(&mut picked) {
            DialogOutcome::Opened(response) => match response.result {
                Err(error) => assert_eq!(error.code, ErrorCode::UnsupportedForHost, "{error}"),
                Ok(_) => unreachable!("playlist.load is not served by this engine build"),
            },
            other => unreachable!("a playlist file was picked: {other:?}"),
        }
    }

    /// The bounded egress queue: overflow drops the OLDEST event and
    /// the stats conserve (the mailbox discipline, applied to events).
    #[test]
    fn egress_queue_is_bounded_drop_oldest_and_conserves() {
        let capacity = NonZeroUsize::new(2).must();
        let mut sink = BoundedEventSink::with_capacity(capacity);
        // Drive the queue directly through the EventSink trait (the
        // runtime path is covered above; this pins the bound itself).
        for seq in 0..5u64 {
            sink.forward(&WireEvent {
                seq,
                session: None,
                generation: None,
                origin: None,
                kind: commands::EventKind::PlaybackEndOfMedia,
            });
        }
        let drained = sink.take();
        assert_eq!(
            drained.iter().map(|event| event.seq).collect::<Vec<_>>(),
            vec![3, 4],
            "the oldest events dropped, order preserved"
        );
        let stats = sink.stats();
        assert_eq!(stats.offered, 5);
        assert_eq!(stats.dropped_overflow, 3);
        assert!(stats.conserves());
        assert_eq!(stats.dropped_stale, 0);
    }
}
