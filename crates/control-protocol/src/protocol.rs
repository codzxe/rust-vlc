//! The `lumen-control/1` application protocol — card 7.J1 (doc 05 §5.4).
//!
//! On top of the WebSocket text frames rides a small JSON envelope:
//!
//! ```json
//! {"type":"hello","protocol":"lumen-control","versions":[1],"token":"…"}
//! {"type":"welcome","v":1,"engineInfo":{…},"capabilities":{…}}
//! {"type":"request","requestId":1,"cmd":"playback.play","args":{}}
//! {"type":"response","requestId":1,"gen":0,"completed":…}
//! {"type":"event","seq":1,"kind":"…"}
//! {"type":"subscribe","events":["timing.seek-completed"]}
//! {"type":"protocol-error","code":"not-hello","message":"…"}
//! ```
//!
//! The request/response/event payloads are the ONE command layer's
//! [`WireRequest`]/[`WireResponse`]/[`WireEvent`] (commands crate),
//! re-used here — never re-implemented (ARCH-5). Version negotiation
//! (doc 05 §6.3): the client's `hello` lists its supported majors; the
//! server picks the highest MUTUAL major and answers `welcome { v }`;
//! no mutual major ⇒ typed protocol-error + close 4402, never silent
//! incompatibility. One major per connection.
//!
//! Strictness beyond serde (doc 05 §5.4 "unknown fields/IDs rejected"):
//! [`validate_request_shape`] checks the raw JSON object's key set
//! before typed parsing, because `#[serde(flatten)]` envelopes cannot
//! carry `deny_unknown_fields` themselves.

use commands::catalog::types::EngineInfo;
use commands::WireEvent;
use commands::WireRequest;
use commands::WireResponse;
use serde::Deserialize;
use serde::Serialize;

/// The protocol name (doc 05 §5.4's versioned transport).
pub const PROTOCOL_NAME: &str = "lumen-control";

/// The major this server speaks.
pub const PROTOCOL_VERSION: u32 = 1;

/// Every major this server can serve (doc 05 §6.3: two majors means
/// two listeners/paths during migrations — v1 has exactly one).
pub const SUPPORTED_VERSIONS: &[u32] = &[PROTOCOL_VERSION];

/// Machine-stable protocol-error codes (kebab-case on the wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProtocolErrorCode {
    /// The first frame was not a `hello`.
    NotHello,
    /// A `hello` arrived after the session was established.
    HelloTwice,
    /// The payload was not valid JSON (or not an object).
    BadJson,
    /// The envelope `type` is not one this server knows.
    UnknownType,
    /// The `type` is known but the frame's timing/shape is wrong.
    BadFrame,
    /// No mutually supported major version.
    VersionMismatch,
    /// The protocol name is not `lumen-control`.
    UnknownProtocol,
    /// A frame arrived before token authentication completed.
    Unauthenticated,
    /// `requestId` was not strictly increasing (anti-replay, §4.1/§5.4).
    NonMonotonicRequestId,
    /// The request failed catalog validation (typed detail rides the
    /// message; the connection survives).
    InvalidRequest,
    /// The op is denied by the server's policy.
    Denied,
    /// Server-side trouble (never leaks internals).
    Internal,
}

impl ProtocolErrorCode {
    /// The stable wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            ProtocolErrorCode::NotHello => "not-hello",
            ProtocolErrorCode::HelloTwice => "hello-twice",
            ProtocolErrorCode::BadJson => "bad-json",
            ProtocolErrorCode::UnknownType => "unknown-type",
            ProtocolErrorCode::BadFrame => "bad-frame",
            ProtocolErrorCode::VersionMismatch => "version-mismatch",
            ProtocolErrorCode::UnknownProtocol => "unknown-protocol",
            ProtocolErrorCode::Unauthenticated => "unauthenticated",
            ProtocolErrorCode::NonMonotonicRequestId => "non-monotonic-request-id",
            ProtocolErrorCode::InvalidRequest => "invalid-request",
            ProtocolErrorCode::Denied => "denied",
            ProtocolErrorCode::Internal => "internal",
        }
    }
}

impl std::fmt::Display for ProtocolErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The capabilities block of `welcome` — what THIS server enforces
/// (doc 05 §5.4's negotiated reality; honest numbers, no aspirational
/// fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    /// Whether the fixed token was checked (false only in read-only
    /// mode's unauthenticated subset).
    pub token_auth: bool,
    /// Whether unauthenticated sessions get the read-only subset.
    pub read_only: bool,
    /// Per-frame byte cap.
    pub max_frame_bytes: u64,
    /// Per-message (reassembled) byte cap.
    pub max_message_bytes: u64,
}

/// The current capabilities for a policy/limits pair.
pub fn capabilities_current(read_only: bool, max_frame: u64, max_message: u64) -> Capabilities {
    Capabilities {
        token_auth: true,
        read_only,
        max_frame_bytes: max_frame,
        max_message_bytes: max_message,
    }
}

/// The capabilities this crate's own defaults advertise (1 MiB caps,
/// token auth on, write mode) — what `welcome` carries unless the host
/// config says otherwise.
pub const CAPABILITIES_CURRENT: Capabilities = Capabilities {
    token_auth: true,
    read_only: false,
    max_frame_bytes: crate::server::Limits::DEFAULT_MAX_FRAME_BYTES,
    max_message_bytes: crate::server::Limits::DEFAULT_MAX_MESSAGE_BYTES,
};

/// `welcome` payload (doc 05 §5.4: `{ v, engineInfo, capabilities }`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Welcome {
    /// The negotiated major.
    #[serde(rename = "v")]
    pub version: u32,
    /// The host's self-description (`system.get-engine-info` payload).
    pub engine_info: EngineInfo,
    /// What this server enforces.
    pub capabilities: Capabilities,
}

/// Client → server frames.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ClientFrame {
    /// The required first frame: protocol name, supported majors, token.
    Hello {
        /// Must equal [`PROTOCOL_NAME`].
        protocol: String,
        /// The client's supported majors (doc 05 §6.3).
        versions: Vec<u32>,
        /// The session token (absent only in read-only mode).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token: Option<String>,
    },
    /// A command request (the ONE command layer's envelope, flattened).
    Request {
        #[serde(flatten)]
        request: WireRequest,
    },
    /// Subscribe to event kinds (empty list = all).
    Subscribe {
        /// Event ids (`timing.seek-completed`, …).
        events: Vec<String>,
    },
    /// Drop a prior subscription.
    Unsubscribe {
        /// Event ids; empty = all.
        events: Vec<String>,
    },
}

/// Server → client frames.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ServerFrame {
    /// The answer to a successful `hello`.
    Welcome(Welcome),
    /// A command response (echoes `requestId`).
    Response {
        #[serde(flatten)]
        response: WireResponse,
    },
    /// An engine event push.
    Event {
        #[serde(flatten)]
        event: WireEvent,
    },
    /// A typed application-level error. For [`ProtocolErrorCode::InvalidRequest`]
    /// and `Denied` the connection survives; the codes that name
    /// handshake/framing breaches precede a close.
    ProtocolError {
        /// Machine-stable code.
        code: ProtocolErrorCode,
        /// Redaction-safe message (V8: no token material, no paths).
        message: String,
    },
}

/// Pick the highest mutually supported major (doc 05 §6.3).
pub fn negotiate_version(client: &[u32]) -> Option<u32> {
    client.iter().copied().filter(|v| SUPPORTED_VERSIONS.contains(v)).max()
}

/// The exact key set a `request` frame may carry (doc 05 §5.4 "unknown
/// fields … rejected" — the flatten envelope cannot enforce this
/// itself). The catalog's own `deny_unknown_fields` cover the `args`
/// object's interior.
pub const REQUEST_KEYS: [&str; 4] = ["type", "requestId", "cmd", "args"];

/// Validate a raw (already-size-capped) JSON value as a request frame
/// shape: an object whose keys ⊆ [`REQUEST_KEYS`] with a number
/// `requestId` and string `cmd`.
///
/// # Errors
/// A redaction-safe [`commands::CommandError`] (`invalid-argument`,
/// stage `control`) describing the first violation.
pub fn validate_request_shape(value: &serde_json::Value) -> Result<(), commands::CommandError> {
    use commands::error::ErrorCode;
    use commands::error::Stage;
    let object = match value.as_object() {
        Some(object) => object,
        None => {
            return Err(commands::CommandError::new(
                Stage::Control,
                ErrorCode::InvalidArgument,
                "request frame must be a JSON object",
            ))
        }
    };
    for key in object.keys() {
        if !REQUEST_KEYS.contains(&key.as_str()) {
            return Err(commands::CommandError::new(
                Stage::Control,
                ErrorCode::InvalidArgument,
                format!("unknown field {key:?} in the request envelope"),
            ));
        }
    }
    match object.get("requestId").map(serde_json::Value::as_u64) {
        Some(Some(_)) => {}
        _ => {
            return Err(commands::CommandError::new(
                Stage::Control,
                ErrorCode::InvalidArgument,
                "requestId must be a non-negative integer",
            ))
        }
    }
    match object.get("cmd").and_then(serde_json::Value::as_str) {
        Some(text) if !text.is_empty() => {}
        _ => {
            return Err(commands::CommandError::new(
                Stage::Control,
                ErrorCode::InvalidArgument,
                "cmd must be a non-empty string",
            ))
        }
    }
    Ok(())
}

/// Length-cap a client-controlled name before it enters an error
/// message (V8 discipline: bounded, no echo beyond 64 chars).
pub fn redact_name(name: &str) -> String {
    let mut out: String = name.chars().take(64).collect();
    if out.len() < name.len() {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use commands::envelope::RequestId;
    use commands::Command;

    #[test]
    fn negotiation_picks_the_highest_mutual_major() {
        assert_eq!(negotiate_version(&[1]), Some(1));
        assert_eq!(negotiate_version(&[2, 1]), Some(1));
        assert_eq!(negotiate_version(&[2, 3]), None);
        assert_eq!(negotiate_version(&[]), None);
    }

    #[test]
    fn client_frame_request_roundtrips() {
        let request = WireRequest {
            request_id: RequestId(7),
            command: Command::PlaybackPlay,
        };
        let frame = ClientFrame::Request { request: request.clone() };
        let json = match serde_json::to_string(&frame) {
            Ok(json) => json,
            Err(err) => unreachable!("serialize failed: {err}"),
        };
        assert!(json.contains(r#""type":"request""#), "{json}");
        assert!(json.contains(r#""requestId":7"#), "{json}");
        assert!(json.contains(r#""cmd":"playback.play""#), "{json}");
        match serde_json::from_str::<ClientFrame>(&json) {
            Ok(ClientFrame::Request { request: back }) => assert_eq!(back, request),
            other => unreachable!("roundtrip failed: {other:?}"),
        }
    }

    #[test]
    fn hello_roundtrips_with_and_without_token() {
        let hello = ClientFrame::Hello {
            protocol: PROTOCOL_NAME.to_owned(),
            versions: vec![1],
            token: Some("secret".to_owned()),
        };
        let json = match serde_json::to_string(&hello) {
            Ok(json) => json,
            Err(err) => unreachable!("serialize failed: {err}"),
        };
        assert!(json.contains(r#""protocol":"lumen-control""#));
        match serde_json::from_str::<ClientFrame>(&json) {
            Ok(ClientFrame::Hello { protocol, versions, token }) => {
                assert_eq!(protocol, "lumen-control");
                assert_eq!(versions, [1]);
                assert_eq!(token.as_deref(), Some("secret"));
            }
            other => unreachable!("roundtrip failed: {other:?}"),
        }
        // Without a token the field is absent (read-only clients).
        let hello = ClientFrame::Hello {
            protocol: PROTOCOL_NAME.to_owned(),
            versions: vec![1],
            token: None,
        };
        let json = match serde_json::to_string(&hello) {
            Ok(json) => json,
            Err(err) => unreachable!("serialize failed: {err}"),
        };
        assert!(!json.contains("token"), "{json}");
    }

    #[test]
    fn server_frame_protocol_error_shape() {
        let frame = ServerFrame::ProtocolError {
            code: ProtocolErrorCode::VersionMismatch,
            message: "no mutually supported major".to_owned(),
        };
        let json = match serde_json::to_string(&frame) {
            Ok(json) => json,
            Err(err) => unreachable!("serialize failed: {err}"),
        };
        assert!(json.contains(r#""type":"protocol-error""#), "{json}");
        assert!(json.contains(r#""code":"version-mismatch""#), "{json}");
    }

    #[test]
    fn request_shape_validation_rejects_unknown_fields_and_bad_ids() {
        let good: serde_json::Value = match serde_json::from_str(
            r#"{"type":"request","requestId":1,"cmd":"playback.play"}"#,
        ) {
            Ok(value) => value,
            Err(err) => unreachable!("{err}"),
        };
        assert!(validate_request_shape(&good).is_ok());
        let unknown_field: serde_json::Value = match serde_json::from_str(
            r#"{"type":"request","requestId":1,"cmd":"playback.play","extra":true}"#,
        ) {
            Ok(value) => value,
            Err(err) => unreachable!("{err}"),
        };
        match validate_request_shape(&unknown_field) {
            Err(err) => {
                assert_eq!(err.code, commands::error::ErrorCode::InvalidArgument);
                assert!(err.message.contains("extra"), "{}", err.message);
            }
            Ok(()) => unreachable!("unknown field accepted"),
        }
        let bad_id: serde_json::Value = match serde_json::from_str(
            r#"{"type":"request","requestId":"one","cmd":"playback.play"}"#,
        ) {
            Ok(value) => value,
            Err(err) => unreachable!("{err}"),
        };
        assert!(validate_request_shape(&bad_id).is_err());
        let not_object: serde_json::Value = serde_json::Value::Bool(true);
        assert!(validate_request_shape(&not_object).is_err());
    }

    #[test]
    fn redact_name_bounds_client_controlled_text() {
        assert_eq!(redact_name("short"), "short");
        let long = "x".repeat(200);
        let capped = redact_name(&long);
        assert!(capped.chars().count() <= 65);
        assert!(capped.ends_with('…'));
    }
}
