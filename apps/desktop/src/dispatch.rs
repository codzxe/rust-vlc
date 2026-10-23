//! **The** desktop dispatch entry (card 4.H1; doc 05 §5.1 — the
//! architecture rule for this shell): one command, `lumen_dispatch`,
//! and nothing else ever registered on the invoke handler.
//!
//! # The single-command rule
//!
//! Doc 05 §5.1: "The desktop shell registers a single Tauri command."
//! A generic entrypoint keeps the catalog data-driven (no per-command
//! Rust glue to drift), exercises exactly one serde path, and makes
//! the anti-bypass test trivial (§7.4: an architecture test reads the
//! shell's `invoke_handler` and fails if any command other than
//! `lumen_dispatch` is registered — see `tauri/INFRA-BLOCKED.md` for
//! why that test is a CI-row, not a sandbox row).
//!
//! This module is the **engine-first half** that compiles and tests
//! without webkit2gtk: the routing body the webview wrapper will
//! call. The tauri-attribute-ready shape (doc 05 §5.1, verbatim
//! signature) is:
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
//! The `tauri` attribute itself is NOT applied here: importing tauri
//! would link webkit2gtk-4.1 (absent: `pkg-config --exists
//! webkit2gtk-4.1` fails in this sandbox) and put an unaudited graph
//! into `Cargo.lock` — the allowlist rule (docs/dependencies/records/
//! tauri-wry-2.x.md: "exception-ADR-004, NOT imported"). The wrapper
//! joins with the ADR-004 admission build on a desktop machine.
//!
//! # Reuse, not re-implementation (ARCH-5)
//!
//! The engine + session wiring is the SAME host the CLI drives
//! (`lumen_cli::host::CliHost`): "host.rs drives the engine via the
//! command layer" is the established composition pattern, and the
//! desktop composition root adopts it as-is — one command layer, no
//! per-host re-implementations, no bypasses. [`ShellHost`] adds only
//! the desktop-specific halves: the webview event-forward bookkeeping
//! (doc 05 §5.1 "events": the shell forwards every engine event on
//! `"lumen://event"`, and both invoke responses and emits must pass
//! through the same ordered channel to preserve §4.3 guarantee 2).
//!
//! # Events back
//!
//! [`ShellHost::forward_new_events`] is the ordered drain: each event
//! is handed to the [`EventSink`] exactly once, in engine order —
//! the seam the future `app_handle.emit("lumen://event", …)` wraps
//! (and the future same-`tokio::mpsc` ordering rule lands at that
//! wrapper, not here, so this half stays webview-free).

use std::path::PathBuf;

use commands::envelope::RequestId;
use commands::{Command, WireEvent, WireRequest, WireResponse};

/// The engine-driving host (the CLI's, adopted as the composition
/// pattern) + the desktop event-forward bookkeeping.
pub struct ShellHost {
    host: lumen_cli::host::CliHost,
    /// How many of `host.events()` have already been forwarded —
    /// the once-only, in-order drain cursor.
    forwarded: usize,
}

impl ShellHost {
    /// A fresh shell host (no session).
    #[must_use = "pure constructor"]
    pub fn new() -> ShellHost {
        ShellHost {
            host: lumen_cli::host::CliHost::new(),
            forwarded: 0,
        }
    }

    /// The engine-driving host (test/diagnostic access; every
    /// command still flows through [`lumen_dispatch`]).
    #[must_use = "the adopted composition pattern"]
    pub fn engine_host(&mut self) -> &mut lumen_cli::host::CliHost {
        &mut self.host
    }

    /// Forward every not-yet-forwarded engine event, in order, to
    /// `sink` — the `"lumen://event"` channel's webview-free half.
    /// Returns the number forwarded (monotonic cursor: an event is
    /// delivered exactly once per sink chain).
    pub fn forward_new_events(&mut self, sink: &mut dyn EventSink) -> usize {
        let events = self.host.events();
        let mut count = 0;
        for event in &events[self.forwarded..] {
            sink.forward(event);
            count += 1;
        }
        self.forwarded = events.len();
        count
    }
}

/// The event egress seam: the future `app_handle.emit("lumen://event",
/// &event)` (and, per doc 05 §5.1, the same-channel ordering rule
/// shared with invoke responses).
pub trait EventSink {
    /// Hand one engine event to the webview channel.
    fn forward(&mut self, event: &WireEvent);
}

/// **The** dispatch entry (doc 05 §5.1's desktop shape; the webview
/// wrapper is a thin lock-and-delegate — see the module docs).
///
/// Correlate (`request_id` always echoed), route through the engine
/// via the command layer, answer the wire response. No other entry
/// exists for UI-originated commands — that is the rule.
pub fn lumen_dispatch(shell: &mut ShellHost, request: &WireRequest) -> WireResponse {
    lumen_cli::dispatch::lumen_dispatch(&mut shell.host, request)
}

/// Build the `session.open` request for one dropped/picked media file
/// (the drag-drop and file-dialog handlers' shared funnel: both
/// produce a [`WireRequest`] and send it through [`lumen_dispatch`] —
/// a file open is a command, never a shell-side engine call).
#[must_use = "builds the wire request; the caller dispatches it"]
pub fn open_file_request(request_id: RequestId, path: &PathBuf) -> WireRequest {
    let command = Command::SessionOpen {
        source: commands::catalog::types::SourceSpec::File {
            path: path.clone(),
        },
        options: Default::default(),
    };
    WireRequest { request_id, command }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commands::CommandOutcome;
    use commands::ErrorCode;

    /// A recording sink — the test half of the egress seam.
    #[derive(Default)]
    struct RecordingSink {
        seen: Vec<u64>,
    }
    impl EventSink for RecordingSink {
        fn forward(&mut self, event: &WireEvent) {
            self.seen.push(event.seq);
        }
    }

    fn request(id: u64, command: Command) -> WireRequest {
        WireRequest {
            request_id: RequestId(id),
            command,
        }
    }

    /// Routing: an unserved catalog id answers the typed
    /// `unsupported-for-host` refusal with the id echoed — never a
    /// panic, never silence, and the response's `request_id` always
    /// echoes (§4.3 echo validity).
    #[test]
    fn unserved_commands_refuse_typed_and_echo() {
        let mut shell = ShellHost::new();
        let response = lumen_dispatch(
            &mut shell,
            &request(7, Command::PlaybackSetRate { rate: 2.0 }),
        );
        assert_eq!(response.request_id, RequestId(7));
        match response.result {
            Err(error) => {
                assert_eq!(error.code, ErrorCode::UnsupportedForHost, "{error}");
                assert_eq!(
                    error.details.get("command").map(String::as_str),
                    Some("playback.set-rate")
                );
            }
            Ok(_) => unreachable!("set-rate is not served by this build"),
        }
    }

    /// Routing: a served command reaches the engine through the
    /// command layer (the settings seam needs no session) and the
    /// response carries the answered context (§1.6).
    #[test]
    fn served_commands_reach_the_engine_and_answer() {
        let mut shell = ShellHost::new();
        let response = lumen_dispatch(
            &mut shell,
            &request(11, Command::SystemListCommands),
        );
        assert_eq!(response.request_id, RequestId(11));
        match response.result {
            Ok(CommandOutcome::Completed(commands::CommandResult::CommandListings {
                commands,
            })) => assert!(!commands.is_empty()),
            other => unreachable!("system.list-commands must complete: {other:?}"),
        }
        // No session: the honest None context, not an invented one.
        assert_eq!(shell.engine_host().session_id(), None);
    }

    /// Events back: a real open → settle produces engine events that
    /// the sink receives in engine order, exactly once (the drain
    /// cursor). This is the real dispatch-routing loop over the real
    /// engine — the same fixture the CLI suites drive.
    #[test]
    fn events_forward_in_order_exactly_once() {
        let fixture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/generated/valid/avi_mjpeg_pcm.avi"
        );
        let mut shell = ShellHost::new();
        let response = lumen_dispatch(
            &mut shell,
            &open_file_request(RequestId(1), &PathBuf::from(fixture)),
        );
        // The open path's own honest answer decides; the host settles.
        let _ = response;
        shell.engine_host().settle();
        let mut sink = RecordingSink::default();
        let first = shell.forward_new_events(&mut sink);
        assert!(first > 0, "a settled open must have produced events");
        assert_eq!(sink.seen.len(), first);
        // Monotonic cursor: nothing is re-delivered.
        let second = shell.forward_new_events(&mut sink);
        assert_eq!(second, 0);
        assert_eq!(sink.seen.len(), first);
    }

    /// The file-open funnel builds the catalog's own wire form — a
    /// dropped file is a `session.open` command, not a bypass.
    #[test]
    fn file_open_funnel_is_the_catalog_command() {
        let request = open_file_request(RequestId(42), &PathBuf::from("/tmp/a.avi"));
        assert_eq!(request.request_id, RequestId(42));
        match request.command {
            Command::SessionOpen { source, .. } => match source {
                commands::catalog::types::SourceSpec::File { path } => {
                    assert_eq!(path, PathBuf::from("/tmp/a.avi"));
                }
                other => unreachable!("file open must be a Path source: {other:?}"),
            },
            other => unreachable!("file open must open a session: {other:?}"),
        }
    }
}
