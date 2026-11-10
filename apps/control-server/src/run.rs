//! The accept/serve event loop — card 7.J1 (doc 05 §5.4).
//!
//! A deliberately boring loop: accept one connection, run it to
//! completion ([`ControlServer::serve_one`]: upgrade → hello → requests
//! → close), log the [`SessionSummary`], repeat. Single-connection by
//! design (the protocol crate's bounded posture); `max_sessions` caps
//! the loop for scripted/smoke use (`None` = serve until the process
//! is terminated — std-only, no signal handling: SIGTERM/SIGINT end
//! the process and its in-flight session, acceptable for a local
//! control surface).

use control_protocol::ControlServer;
use control_protocol::ProtocolError;
use control_protocol::server::Dispatch;

use crate::logging::Level;
use crate::logging::LogRecord;
use crate::logging::LogSink;
use crate::logging::session_record;

/// What the loop did before returning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunOutcome {
    /// Sessions served to completion.
    pub sessions: u32,
    /// Whether the session cap stopped the loop (`false` = the loop
    /// was asked for unbounded service and is only returning because
    /// the caller capped it or the caller's test ended the stream).
    pub capped: bool,
}

/// Serve connections until the cap (or forever when `max_sessions` is
/// `None`). Every finished session is emitted to `sink` as a
/// `session-end` record; a transport-level [`ProtocolError`] (accept
/// trouble, listener gone) is returned after a `serve-failed` record.
///
/// # Errors
/// [`ProtocolError`] verbatim from `serve_one` — the caller decides
/// process fate (the binary exits; a library host may restart).
pub fn serve_loop(
    server: &mut ControlServer,
    dispatch: &mut dyn Dispatch,
    sink: &mut dyn LogSink,
    max_sessions: Option<u32>,
) -> Result<RunOutcome, ProtocolError> {
    let mut served: u32 = 0;
    loop {
        if max_sessions.is_some_and(|cap| served >= cap) {
            return Ok(RunOutcome {
                sessions: served,
                capped: true,
            });
        }
        match server.serve_one(dispatch) {
            Ok(summary) => {
                sink.emit(&session_record(&summary));
                served = served.saturating_add(1);
            }
            Err(source) => {
                sink.emit(
                    &LogRecord::new(Level::Error, "serve-failed").with("error", source.to_string()),
                );
                return Err(source);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commands::WireRequest;
    use commands::WireResponse;
    use commands::catalog::types::EngineInfo;
    use control_protocol::ServerConfig;
    use control_protocol::TokenSource;

    fn engine_info() -> EngineInfo {
        EngineInfo {
            engine_version: "0.1.0-run-test".to_owned(),
            catalog_version: commands::catalog::spec::catalog_checksum().to_string(),
            // lumen-allow-literal: protocol version string is wire vocabulary (doc 05 §5.4)
            protocol_version: "lumen-control/1".to_owned(),
            host: "control-server".to_owned(),
            features: Vec::new(),
            plugins: Vec::new(),
            decoders: Vec::new(),
            sessions: Vec::new(),
        }
    }

    /// A dispatch that answers everything with class-S Unit.
    struct UnitDispatch;

    impl Dispatch for UnitDispatch {
        fn dispatch(&mut self, request: WireRequest) -> WireResponse {
            WireResponse {
                request_id: request.request_id,
                session: None,
                generation: commands::envelope::Generation(0),
                result: Ok(commands::envelope::CommandOutcome::Completed(
                    commands::CommandResult::Unit,
                )),
            }
        }
    }

    #[test]
    fn the_cap_stops_the_loop_after_serving_that_many_sessions() {
        let mut config = ServerConfig::strict(engine_info());
        config.token = TokenSource::Fixed("run-loop-test".to_owned());
        let mut server = match ControlServer::bind(config) {
            Ok(server) => server,
            Err(err) => unreachable!("bind failed: {err}"),
        };
        // No clients connect: with cap 0 the loop must return
        // immediately without ever blocking on accept.
        let mut sink = crate::logging::MemoryLog::new();
        let mut dispatch = UnitDispatch;
        let outcome = match serve_loop(&mut server, &mut dispatch, &mut sink, Some(0)) {
            Ok(outcome) => outcome,
            Err(err) => unreachable!("cap-0 loop failed: {err}"),
        };
        assert_eq!(outcome.sessions, 0);
        assert!(outcome.capped);
        assert!(sink.records.is_empty());
    }
}
