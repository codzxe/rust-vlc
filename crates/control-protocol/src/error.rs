//! The typed protocol-error taxonomy (card 7.J1; doc 05 §2.13 bridging).
//!
//! Every failure mode of the control plane is a variant here — never a
//! stringly error, never a panic. [`ProtocolError::to_command_error`]
//! projects the taxonomy onto the ONE command layer's `CommandError`
//! (stage [`Stage::Control`], doc 05 §2.13) so hosts answer with the
//! same vocabulary every other host uses.
//!
//! Redaction (V8): token material NEVER enters an error message — the
//! auth variants carry no payload beyond the fact of failure.

use commands::error::CommandError;
use commands::error::ErrorCode;
use commands::error::Stage;

/// WebSocket close codes this server emits (RFC 6455 §7.4 + the
/// doc 05 §5.4 application range 4400+).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CloseCode {
    /// 1000 — normal, deliberate close.
    Normal,
    /// 1001 — the server's bounded wait expired (idle or stalled peer);
    /// no session can wedge the single-connection server forever.
    GoingAway,
    /// 1002 — protocol error (malformed frame, bad handshake).
    ProtocolError,
    /// 1007 — text payload was not valid UTF-8.
    InvalidUtf8,
    /// 1009 — frame/message exceeded the configured cap.
    TooBig,
    /// 4400 — lumen-control application-level protocol error
    /// (bad envelope, non-monotonic request id).
    AppProtocolError,
    /// 4401 — token authentication failed (doc 05 §5.4).
    Unauthorized,
    /// 4402 — no mutually supported `lumen-control` major version.
    VersionMismatch,
}

impl CloseCode {
    /// The wire value (RFC 6455 §5.5.1 big-endian u16).
    pub const fn as_u16(self) -> u16 {
        match self {
            CloseCode::Normal => 1000,
            CloseCode::GoingAway => 1001,
            CloseCode::ProtocolError => 1002,
            CloseCode::InvalidUtf8 => 1007,
            CloseCode::TooBig => 1009,
            CloseCode::AppProtocolError => 4400,
            CloseCode::Unauthorized => 4401,
            CloseCode::VersionMismatch => 4402,
        }
    }
}

impl std::fmt::Display for CloseCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_u16())
    }
}

/// Every typed failure of the control plane.
#[derive(Debug)]
pub enum ProtocolError {
    /// Construction asked to bind a non-loopback address (card 7.J1
    /// acceptance: refused at construction, not warned about).
    NonLoopbackBind {
        /// The rejected address, as given.
        requested: String,
    },
    /// OS-level I/O failure (bind/listen/accept/read/write). The inner
    /// std error is preserved for the log sink; `Display` stays
    /// redaction-safe (no paths beyond the operation name).
    Io {
        /// What was being attempted.
        what: &'static str,
        /// The OS error.
        source: std::io::Error,
    },
    /// The HTTP upgrade request did not satisfy RFC 6455 §4.2.1.
    HttpUpgrade {
        /// Machine-stable reason token (never echoes raw request bytes).
        reason: &'static str,
    },
    /// A frame violated RFC 6455 framing (mask, RSV, control-frame
    /// rules, truncation, close-code validity).
    Frame {
        /// The close code the violation maps to.
        code: CloseCode,
        /// Redaction-safe explanation.
        reason: String,
    },
    /// A frame or reassembled message exceeded its cap (D-1: bounded
    /// memory everywhere; oversized claims never allocate).
    MessageTooLarge {
        /// The enforced cap in bytes.
        limit: u64,
        /// The claimed size.
        claimed: u64,
    },
    /// The first application frame was not a valid `hello`.
    NotHello,
    /// `hello` arrived after the session was already established.
    HelloTwice,
    /// The client named a protocol other than `lumen-control`.
    UnknownProtocol {
        /// The offered name (client-controlled; length-capped on parse).
        offered: String,
    },
    /// No mutually supported major version (doc 01 "Never": silent
    /// incompatibility).
    VersionMismatch {
        /// The majors the client offered.
        client: Vec<u32>,
        /// The majors this server supports.
        server: Vec<u32>,
    },
    /// Token authentication failed (wrong token, or a frame arrived
    /// before authentication).
    Unauthorized {
        /// True when a lockout is active (doc 05 §5.4: 5 failures ⇒
        /// 60 s).
        locked_out: bool,
    },
    /// A text frame was not valid UTF-8 where UTF-8 is required
    /// (RFC 6455 §8.1).
    InvalidUtf8,
    /// A payload was not valid JSON, or not the envelope shape.
    BadEnvelope {
        /// Redaction-safe detail (serde message, capped).
        detail: String,
    },
    /// A socket read/write deadline expired (the bounded-wait
    /// guarantee: an idle or stalled peer closes the session with
    /// 1001 instead of wedging the single-connection server forever).
    TimedOut {
        /// What was being attempted.
        what: &'static str,
    },
    /// The connection closed (peer close frame or EOF).
    Closed,
}

impl ProtocolError {
    /// The close code this error closes the WS connection with.
    pub fn close_code(&self) -> CloseCode {
        match self {
            ProtocolError::NonLoopbackBind { .. }
            | ProtocolError::Io { .. }
            | ProtocolError::Closed => CloseCode::Normal,
            ProtocolError::TimedOut { .. } => CloseCode::GoingAway,
            ProtocolError::HttpUpgrade { .. } => CloseCode::ProtocolError,
            ProtocolError::Frame { code, .. } => *code,
            ProtocolError::MessageTooLarge { .. } => CloseCode::TooBig,
            ProtocolError::NotHello | ProtocolError::HelloTwice => CloseCode::AppProtocolError,
            ProtocolError::UnknownProtocol { .. } => CloseCode::AppProtocolError,
            ProtocolError::VersionMismatch { .. } => CloseCode::VersionMismatch,
            ProtocolError::Unauthorized { .. } => CloseCode::Unauthorized,
            ProtocolError::InvalidUtf8 => CloseCode::InvalidUtf8,
            ProtocolError::BadEnvelope { .. } => CloseCode::AppProtocolError,
        }
    }

    /// Whether the connection can survive this error (a typed
    /// application-level error frame, no close).
    pub fn is_recoverable(&self) -> bool {
        matches!(self, ProtocolError::BadEnvelope { .. })
    }

    /// Project onto the ONE command layer's error vocabulary (doc 05
    /// §2.13): stage `control`, the closest `ErrorCode`. Token material
    /// never enters the message (V8).
    pub fn to_command_error(&self) -> CommandError {
        let code = match self {
            ProtocolError::NonLoopbackBind { .. } => ErrorCode::InvalidArgument,
            ProtocolError::Io { .. } => ErrorCode::Io,
            ProtocolError::TimedOut { .. } => ErrorCode::Network,
            ProtocolError::HttpUpgrade { .. } | ProtocolError::Frame { .. } => {
                ErrorCode::InvalidArgument
            }
            ProtocolError::MessageTooLarge { .. } => ErrorCode::ResourceLimit,
            ProtocolError::NotHello
            | ProtocolError::HelloTwice
            | ProtocolError::BadEnvelope { .. } => ErrorCode::InvalidArgument,
            ProtocolError::UnknownProtocol { .. } => ErrorCode::InvalidArgument,
            ProtocolError::VersionMismatch { .. } => ErrorCode::InvalidArgument,
            ProtocolError::Unauthorized { .. } => ErrorCode::PermissionDenied,
            ProtocolError::InvalidUtf8 => ErrorCode::InvalidArgument,
            ProtocolError::Closed => ErrorCode::Canceled,
        };
        CommandError::new(Stage::Control, code, self.to_string())
    }
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolError::NonLoopbackBind { requested } => write!(
                f,
                "non-loopback bind refused: {requested} (control server binds 127.0.0.1/::1 only)"
            ),
            ProtocolError::Io { what, source } => write!(f, "{what}: {source}"),
            ProtocolError::TimedOut { what } => {
                write!(
                    f,
                    "socket deadline expired while {what} (idle or stalled peer)"
                )
            }
            ProtocolError::HttpUpgrade { reason } => {
                write!(f, "websocket upgrade rejected: {reason}")
            }
            ProtocolError::Frame { reason, .. } => write!(f, "frame protocol violation: {reason}"),
            ProtocolError::MessageTooLarge { limit, claimed } => write!(
                f,
                "payload exceeds the {limit}-byte cap (claimed {claimed})"
            ),
            ProtocolError::NotHello => {
                // lumen-allow-literal: protocol name (doc 05 §5.4 wire vocabulary), not product identity
                write!(f, "first frame must be a lumen-control hello")
            }
            ProtocolError::HelloTwice => write!(f, "hello after session start"),
            ProtocolError::UnknownProtocol { offered } => write!(
                f,
                // lumen-allow-literal: protocol name (doc 05 §5.4 wire vocabulary), not product identity
                "unknown protocol name (expected lumen-control, offered {})",
                crate::protocol::redact_name(offered)
            ),
            ProtocolError::VersionMismatch { client, server } => write!(
                f,
                "no mutually supported major: client {:?} vs server {:?}",
                client, server
            ),
            ProtocolError::Unauthorized { locked_out } => {
                if *locked_out {
                    write!(f, "token rejected; server locked out (too many failures)")
                } else {
                    write!(f, "token rejected")
                }
            }
            ProtocolError::InvalidUtf8 => write!(f, "text payload is not valid UTF-8"),
            ProtocolError::BadEnvelope { detail } => write!(f, "bad envelope: {detail}"),
            ProtocolError::Closed => write!(f, "connection closed"),
        }
    }
}

impl std::error::Error for ProtocolError {}

impl From<std::io::Error> for ProtocolError {
    fn from(source: std::io::Error) -> Self {
        ProtocolError::Io {
            what: "control-protocol i/o",
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_codes_are_the_documented_values() {
        assert_eq!(CloseCode::Normal.as_u16(), 1000);
        assert_eq!(CloseCode::GoingAway.as_u16(), 1001);
        assert_eq!(CloseCode::ProtocolError.as_u16(), 1002);
        assert_eq!(CloseCode::InvalidUtf8.as_u16(), 1007);
        assert_eq!(CloseCode::TooBig.as_u16(), 1009);
        // doc 05 §5.4 application range.
        assert_eq!(CloseCode::AppProtocolError.as_u16(), 4400);
        assert_eq!(CloseCode::Unauthorized.as_u16(), 4401);
        assert_eq!(CloseCode::VersionMismatch.as_u16(), 4402);
    }

    #[test]
    fn auth_errors_map_to_permission_denied() {
        let err = ProtocolError::Unauthorized { locked_out: false };
        let command_error = err.to_command_error();
        assert_eq!(command_error.code, ErrorCode::PermissionDenied);
        assert_eq!(command_error.stage, Stage::Control);
        assert_eq!(command_error.summary(), "[control.permission-denied]");
    }

    #[test]
    fn oversized_maps_to_resource_limit() {
        let err = ProtocolError::MessageTooLarge {
            limit: 1024,
            claimed: 999_999,
        };
        assert_eq!(err.to_command_error().code, ErrorCode::ResourceLimit);
        assert_eq!(err.close_code(), CloseCode::TooBig);
    }

    #[test]
    fn version_mismatch_maps_to_close_4402() {
        let err = ProtocolError::VersionMismatch {
            client: vec![2, 3],
            server: vec![1],
        };
        assert_eq!(err.close_code(), CloseCode::VersionMismatch);
        assert!(err.to_string().contains("mutually supported major"));
    }

    #[test]
    fn timeouts_map_to_going_away_and_network() {
        let err = ProtocolError::TimedOut {
            what: "reading a frame",
        };
        assert_eq!(err.close_code(), CloseCode::GoingAway);
        assert_eq!(err.close_code().as_u16(), 1001);
        assert_eq!(err.to_command_error().code, ErrorCode::Network);
        assert!(err.to_string().contains("deadline"));
    }

    #[test]
    fn only_bad_envelope_is_recoverable() {
        assert!(ProtocolError::BadEnvelope { detail: "x".into() }.is_recoverable());
        assert!(!ProtocolError::Unauthorized { locked_out: false }.is_recoverable());
        assert!(
            !ProtocolError::VersionMismatch {
                client: vec![],
                server: vec![1]
            }
            .is_recoverable()
        );
    }
}
