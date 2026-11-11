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

use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::path::Path;

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

impl Default for ShellHost {
    fn default() -> Self {
        Self::new()
    }
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

/// Observable accounting for [`BoundedEventSink`] (the mailbox
/// pattern's conservation discipline, applied to engine events).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EventQueueStats {
    /// Kept events that entered the queue (post §4.2 filter).
    pub offered: u64,
    /// Events handed to the webview emitter ([`BoundedEventSink::take`]).
    pub taken: u64,
    /// Events dropped because the queue was full (drop-oldest,
    /// counted — order of the survivors is never violated).
    pub dropped_overflow: u64,
    /// Events refused by the §4.2 staleness filter (never queued).
    pub dropped_stale: u64,
}

impl EventQueueStats {
    /// Conservation identity: every offered event is taken or
    /// overflow-dropped, exactly once (stale refusals never offer).
    #[must_use = "pure arithmetic"]
    pub fn conserves(&self) -> bool {
        self.offered == self.taken + self.dropped_overflow
    }
}

/// The bounded engine-event queue between the drain and the webview
/// emitter (issue #3 — the mailbox pattern applied to the command
/// layer's event half): the wrapper cannot always `emit` the instant
/// [`ShellHost::forward_new_events`] drains, so up to `capacity`
/// events wait here. Bounded by construction; when full the OLDEST
/// event is dropped and counted (never a block, never a reorder);
/// [`EventQueueStats`] keeps the drops honest arithmetic.
#[derive(Debug)]
pub struct BoundedEventSink {
    capacity: usize,
    queue: VecDeque<WireEvent>,
    stats: EventQueueStats,
}

impl BoundedEventSink {
    /// Build a bounded sink (capacity ≥ 1 by type).
    #[must_use = "pure constructor"]
    pub fn with_capacity(capacity: NonZeroUsize) -> BoundedEventSink {
        BoundedEventSink {
            capacity: capacity.get(),
            queue: VecDeque::with_capacity(capacity.get()),
            stats: EventQueueStats::default(),
        }
    }

    /// Take every queued event, in order (the emitter half: each one
    /// becomes one `lumen://event` emit).
    pub fn take(&mut self) -> Vec<WireEvent> {
        let drained = self.queue.len() as u64;
        self.stats.taken += drained;
        self.queue.drain(..).collect()
    }

    /// The observable accounting (the compatibility matrix's cells).
    #[must_use = "observable evidence"]
    pub fn stats(&self) -> &EventQueueStats {
        &self.stats
    }

    /// Record one §4.2 stale refusal (the filter runs before the
    /// queue; the count lives here so one stats object tells the
    /// whole egress story).
    pub fn note_stale(&mut self) {
        self.stats.dropped_stale += 1;
    }

    fn offer(&mut self, event: &WireEvent) {
        if self.queue.len() == self.capacity && self.queue.pop_front().is_some() {
            self.stats.dropped_overflow += 1;
        }
        self.queue.push_back(event.clone());
        self.stats.offered += 1;
    }
}

impl EventSink for BoundedEventSink {
    fn forward(&mut self, event: &WireEvent) {
        self.offer(event);
    }
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
pub fn open_file_request(request_id: RequestId, path: &Path) -> WireRequest {
    let command = Command::SessionOpen {
        source: commands::catalog::types::SourceSpec::File {
            path: path.to_path_buf(),
        },
        options: Default::default(),
    };
    WireRequest {
        request_id,
        command,
    }
}

/// Build the `playlist.load` request for one playlist file (4.D1's
/// handoff: parse lives engine-side in crates/playlists; the shell
/// only translates the path into the command — `append: false`, an
/// open replaces the queue).
#[must_use = "builds the wire request; the caller dispatches it"]
pub fn playlist_load_request(request_id: RequestId, path: &Path) -> WireRequest {
    WireRequest {
        request_id,
        command: Command::PlaylistLoad {
            path: path.to_path_buf(),
            append: false,
        },
    }
}

/// The path-kind rule both acquisition sides share (files.rs's
/// classifier is the twin): a media file opens a session, a playlist
/// file loads the list, anything else is not openable at all (the
/// honest `None` — a dropped README is a no-op, not an error).
#[must_use = "pure classifier"]
pub fn open_path_request(request_id: RequestId, path: &Path) -> Option<WireRequest> {
    if crate::files::is_playlist_path(path) {
        return Some(playlist_load_request(request_id, path));
    }
    if crate::files::is_media_path(path) {
        return Some(open_file_request(request_id, path));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use commands::CommandOutcome;
    use commands::ErrorCode;
    use std::path::PathBuf;

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
        let response = lumen_dispatch(&mut shell, &request(11, Command::SystemListCommands));
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
        let request = open_file_request(RequestId(42), &PathBuf::from("/tmp/a.avi")); // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        assert_eq!(request.request_id, RequestId(42));
        match request.command {
            Command::SessionOpen { source, .. } => match source {
                commands::catalog::types::SourceSpec::File { path } => {
                    assert_eq!(path, PathBuf::from("/tmp/a.avi")); // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
                }
                other => unreachable!("file open must be a Path source: {other:?}"),
            },
            other => unreachable!("file open must open a session: {other:?}"),
        }
    }

    /// The path-kind funnel (issue #3 — 4.D1's handoff): media →
    /// `session.open`, playlist → `playlist.load` (append: false),
    /// neither → the honest `None`. One rule, both acquisition sides.
    #[test]
    fn open_path_request_classifies_by_kind() {
        // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        match open_path_request(RequestId(7), &PathBuf::from("/tmp/movie.mkv")) {
            Some(request) => match request.command {
                Command::SessionOpen { .. } => assert_eq!(request.request_id, RequestId(7)),
                other => unreachable!("media opens a session: {other:?}"),
            },
            None => unreachable!("a media file is openable"),
        }
        // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        match open_path_request(RequestId(8), &PathBuf::from("/tmp/queue.m3u")) {
            Some(request) => match request.command {
                Command::PlaylistLoad { path, append } => {
                    assert_eq!(path, PathBuf::from("/tmp/queue.m3u")); // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
                    assert!(!append, "an open replaces the queue");
                }
                other => unreachable!("a playlist file loads the list: {other:?}"),
            },
            None => unreachable!("a playlist file is openable"),
        }
        assert!(open_path_request(RequestId(9), &PathBuf::from("/tmp/notes.txt")).is_none()); // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
    }
}
