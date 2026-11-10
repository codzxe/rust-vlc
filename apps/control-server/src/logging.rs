//! Structured logging — card 7.J1 (doc 10 §5 observability posture).
//!
//! One JSON object per record on the sink — machine-parseable, stable
//! keys, no token material (V8: the token's single sanctioned reader is
//! the controlling-terminal print in `main`, never the log stream).
//! Sinks are a trait so tests capture records without touching real
//! stderr; the production sink ([`JsonStderr`]) writes one line per
//! record with a trailing newline flush.

use std::io::Write;

use control_protocol::SessionSummary;
use control_protocol::server::SessionEnd;

/// Severity (stable lowercase strings on the wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Routine operation (startup, session summaries).
    Info,
    /// Degraded but running (recoverable protocol errors, refusals).
    Warn,
    /// The serve loop failed.
    Error,
}

impl Level {
    /// The stable wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Error => "error",
        }
    }
}

/// One structured record: an event name plus ordered key/value fields.
/// Values are pre-rendered strings; the sink JSON-escapes them.
#[derive(Debug, Clone)]
pub struct LogRecord {
    /// Severity.
    pub level: Level,
    /// Stable event name (`session-end`, `startup`, …).
    pub event: &'static str,
    /// Ordered (key, value) pairs.
    pub fields: Vec<(&'static str, String)>,
}

impl LogRecord {
    /// Build a record.
    pub fn new(level: Level, event: &'static str) -> LogRecord {
        LogRecord {
            level,
            event,
            fields: Vec::new(),
        }
    }

    /// Append one field (builder style).
    pub fn with(mut self, key: &'static str, value: impl Into<String>) -> LogRecord {
        self.fields.push((key, value.into()));
        self
    }

    /// The record as a JSON object (ts first, then level/event/fields).
    fn to_json(&self, ts: f64) -> serde_json::Value {
        let mut object = serde_json::Map::new();
        if let Ok(value) = serde_json::value::to_value(ts) {
            object.insert("ts".to_owned(), value);
        }
        object.insert(
            "level".to_owned(),
            serde_json::Value::String(self.level.as_str().to_owned()),
        );
        object.insert(
            "event".to_owned(),
            serde_json::Value::String(self.event.to_owned()),
        );
        for (key, value) in &self.fields {
            object.insert((*key).to_owned(), serde_json::Value::String(value.clone()));
        }
        serde_json::Value::Object(object)
    }
}

/// Render one record as a single JSON line (trailing `\n`) — the
/// production sink's exact wire format, public so the app's tests pin
/// the line discipline and key set without capturing real stderr.
pub fn render_line(record: &LogRecord, ts: f64) -> String {
    let mut line = record.to_json(ts).to_string();
    line.push('\n');
    line
}

/// Where records go. Implementations must not panic (workspace lints).
pub trait LogSink {
    /// Emit one record.
    fn emit(&mut self, record: &LogRecord);
}

/// The production sink: JSON lines on stderr (stderr so a controlling
/// terminal sees sessions interleaved with the token print, while
/// stdout stays clean for the bound-address line).
#[derive(Debug)]
pub struct JsonStderr {
    inner: std::io::Stderr,
}

impl Default for JsonStderr {
    fn default() -> Self {
        JsonStderr {
            inner: std::io::stderr(),
        }
    }
}

impl JsonStderr {
    /// New stderr sink.
    pub fn new() -> JsonStderr {
        JsonStderr::default()
    }

    /// The current unix timestamp (seconds, millisecond precision) —
    /// `0.0` if the clock is before the epoch (logged as-is; never a
    /// panic).
    fn now() -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or_default()
    }
}

impl LogSink for JsonStderr {
    fn emit(&mut self, record: &LogRecord) {
        let _ = write!(self.inner, "{}", render_line(record, Self::now()));
        let _ = self.inner.flush();
    }
}

/// An in-memory capture sink (tests assert the structured stream
/// without touching real stderr).
#[derive(Debug, Default)]
pub struct MemoryLog {
    /// The captured records, in emit order.
    pub records: Vec<LogRecord>,
}

impl MemoryLog {
    /// New empty capture.
    pub fn new() -> MemoryLog {
        MemoryLog::default()
    }
}

impl LogSink for MemoryLog {
    fn emit(&mut self, record: &LogRecord) {
        self.records.push(record.clone());
    }
}

/// The structured record for one finished session (the event loop's
/// per-connection heartbeat). Level: `warn` when the session ended in
/// a protocol close or auth failure, `info` otherwise.
pub fn session_record(summary: &SessionSummary) -> LogRecord {
    let level = match summary.close {
        SessionEnd::PeerClosed(_) | SessionEnd::Eof => Level::Info,
        SessionEnd::ServerClosed(_) => {
            if summary.auth_failures > 0 && !summary.authenticated {
                Level::Warn
            } else {
                Level::Info
            }
        }
    };
    LogRecord::new(level, "session-end")
        .with("remote", summary.remote.clone())
        .with("authenticated", summary.authenticated.to_string())
        .with(
            "version",
            summary.version.map(|v| v.to_string()).unwrap_or_default(),
        )
        .with("requests_served", summary.requests_served.to_string())
        .with("ops_denied", summary.ops_denied.to_string())
        .with("protocol_errors", summary.protocol_errors.to_string())
        .with("auth_failures", summary.auth_failures.to_string())
        .with("locked_out", summary.locked_out.to_string())
        .with("close", close_kind(summary.close))
}

/// The stable close-kind string (no dynamic payload — the codes
/// themselves are in the summary for richer consumers).
fn close_kind(close: SessionEnd) -> String {
    match close {
        SessionEnd::PeerClosed(code) => format!("peer-closed:{}", code.unwrap_or_default()),
        SessionEnd::ServerClosed(code) => format!("server-closed:{code}"),
        SessionEnd::Eof => "eof".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use control_protocol::CloseCode;

    #[test]
    fn records_serialize_to_stable_json_lines() {
        let record = LogRecord::new(Level::Warn, "session-end")
            .with("remote", "127.0.0.1:9999")
            .with("authenticated", "false");
        let line = render_line(&record, 1234.5);
        assert!(line.ends_with('\n'));
        let json = line.trim_end();
        assert!(json.contains(r#""ts":1234.5"#), "{json}");
        assert!(json.contains(r#""level":"warn""#), "{json}");
        assert!(json.contains(r#""event":"session-end""#), "{json}");
        assert!(json.contains(r#""remote":"127.0.0.1:9999""#), "{json}");
        // Values are JSON-escaped (never an injection vector).
        let hostile = LogRecord::new(Level::Error, "session-end").with("remote", "x\"\ny");
        let json = hostile.to_json(0.0).to_string();
        assert_eq!(json.matches('\n').count(), 0, "{json}");
    }

    #[test]
    fn session_summaries_map_to_records_with_close_kinds() {
        let summary = SessionSummary {
            remote: "127.0.0.1:1".to_owned(),
            authenticated: true,
            version: Some(1),
            requests_served: 3,
            ops_denied: 1,
            protocol_errors: 2,
            auth_failures: 0,
            locked_out: false,
            close: SessionEnd::ServerClosed(CloseCode::GoingAway),
        };
        let record = session_record(&summary);
        assert_eq!(record.event, "session-end");
        assert_eq!(record.level, Level::Info);
        let pairs: Vec<(&str, &str)> = record
            .fields
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();
        assert!(pairs.contains(&("close", "server-closed:1001")));
        assert!(pairs.contains(&("requests_served", "3")));
        // An auth-failed session is a warn.
        let summary = SessionSummary {
            remote: "127.0.0.1:1".to_owned(),
            authenticated: false,
            version: None,
            requests_served: 0,
            ops_denied: 0,
            protocol_errors: 0,
            auth_failures: 1,
            locked_out: false,
            close: SessionEnd::ServerClosed(CloseCode::Unauthorized),
        };
        assert_eq!(session_record(&summary).level, Level::Warn);
    }

    #[test]
    fn memory_sink_captures_in_order() {
        let mut sink = MemoryLog::new();
        sink.emit(&LogRecord::new(Level::Info, "startup"));
        sink.emit(&LogRecord::new(Level::Error, "serve-failed"));
        assert_eq!(sink.records.len(), 2);
        assert_eq!(sink.records[0].event, "startup");
        assert_eq!(sink.records[1].level, Level::Error);
    }
}
