//! The control server session loop — card 7.J1 (doc 05 §5.4).
//!
//! [`ControlServer::bind`] is the acceptance-critical constructor: it
//! refuses any non-loopback bind address with a typed
//! [`ProtocolError::NonLoopbackBind`] BEFORE touching the OS (doc 05
//! §5.4: "deny-by-default is structural, not advisory" — a startup
//! error, not a warning), and re-verifies the bound address afterwards
//! (belt and braces: a hostile resolver must not turn `localhost`
//! into something else).
//!
//! The session loop ([`ControlServer::serve_one`]) drives ONE
//! connection at a time over the in-tree transport stack
//! ([`crate::http`] upgrade → [`crate::frame`] codec →
//! [`crate::protocol`] application frames):
//!
//! 1. HTTP upgrade (RFC 6455 §4.2; failure = HTTP error response, the
//!    connection never becomes a WebSocket);
//! 2. the `lumen-control` `hello`: protocol name, version negotiation
//!    (mismatch = typed protocol-error + close 4402, never UB), token
//!    presentation (constant-time compare; wrong token = close 4401
//!    and a failure-book entry — [`AuthLimits`]: 5 failures ⇒ 60 s
//!    lockout; frames before authentication are refused);
//! 3. requests: envelope shape validation (unknown fields/ids are
//!    typed errors and the connection SURVIVES), `requestId`
//!    monotonicity (anti-replay — a breach closes the connection),
//!    the deny-by-default [`crate::policy`] check, then the host's
//!    [`Dispatch`];
//! 4. control frames: ping answered by pong, close echoed and honored.
//!
//! The server is deliberately single-connection, synchronous: a
//! control surface for a local single-user product (bounded by design,
//! D-1). Concurrency/rate caps and the Origin allowlist are recorded
//! gaps for 8.A2 — see [`Limits`]'s honest-scope notes.

use std::io::Write;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::net::TcpStream;
use std::net::ToSocketAddrs;
use std::time::Duration;
use std::time::Instant;

use commands::WireRequest;
use commands::WireResponse;
use commands::catalog::types::EngineInfo;

use crate::error::CloseCode;
use crate::error::ProtocolError;
use crate::frame::MessageAssembler;
use crate::frame::Opcode;
use crate::frame::close_payload;
use crate::frame::parse_close_payload;
use crate::frame::read_frame;
use crate::frame::write_frame;
use crate::http;
use crate::policy::ServerPolicy;
use crate::policy::request_command_id;
use crate::protocol::ClientFrame;
use crate::protocol::ProtocolErrorCode;
use crate::protocol::ServerFrame;
use crate::protocol::Welcome;
use crate::protocol::negotiate_version;
use crate::protocol::validate_request_shape;
use crate::token::EntropySource;
use crate::token::Token;
use crate::token::constant_time_eq;
use crate::token::generate_token;

/// Byte caps (doc 05 §5.4: "Every frame: max 1 MiB"). A frame or
/// reassembled message above its cap is a typed
/// [`ProtocolError::MessageTooLarge`] close — the claim is rejected
/// BEFORE any allocation (D-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Per-frame payload cap.
    pub max_frame_bytes: u64,
    /// Per-reassembled-message cap.
    pub max_message_bytes: u64,
}

impl Limits {
    /// The documented defaults: 1 MiB per frame and per message.
    pub const DEFAULT_MAX_FRAME_BYTES: u64 = 1024 * 1024;
    /// Default message cap (same 1 MiB).
    pub const DEFAULT_MAX_MESSAGE_BYTES: u64 = 1024 * 1024;

    /// The documented defaults.
    pub const fn documented() -> Limits {
        Limits {
            max_frame_bytes: Limits::DEFAULT_MAX_FRAME_BYTES,
            max_message_bytes: Limits::DEFAULT_MAX_MESSAGE_BYTES,
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Limits::documented()
    }
}

/// Authentication failure policy (doc 05 §5.4): 5 failures impose a
/// 60 s lockout and a log entry. The failure book is SERVER-GLOBAL
/// (one attacker, many connections — the lockout must not reset per
/// socket). Durations are configurable so tests do not sleep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthLimits {
    /// Failures before the lockout engages.
    pub max_failures: u32,
    /// How long the lockout lasts.
    pub lockout: Duration,
}

impl AuthLimits {
    /// The documented policy: 5 failures, 60 s.
    pub const fn documented() -> AuthLimits {
        AuthLimits {
            max_failures: 5,
            lockout: Duration::from_secs(60),
        }
    }
}

impl Default for AuthLimits {
    fn default() -> Self {
        AuthLimits::documented()
    }
}

/// Socket read/write deadlines (the bounded-wait guarantee): an idle
/// or stalled peer ends the session with a typed 1001 close instead of
/// wedging the single-connection server forever. `None` disables a
/// deadline (tests that drive byte-at-a-time frames may want that).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    /// Read deadline per socket operation (idle peer bound).
    pub read: Option<Duration>,
    /// Write deadline per socket operation (stalled peer bound).
    pub write: Option<Duration>,
}

impl Timeouts {
    /// The documented defaults: 60 s read (a control client may idle
    /// between requests, but not forever), 10 s write.
    pub const fn documented() -> Timeouts {
        Timeouts {
            read: Some(Duration::from_secs(60)),
            write: Some(Duration::from_secs(10)),
        }
    }
}

impl Default for Timeouts {
    fn default() -> Self {
        Timeouts::documented()
    }
}

/// The failure/backoff book (doc 05 §5.4). Private to the server; the
/// lockout state is observable only through typed errors and
/// [`SessionSummary`].
#[derive(Debug, Default)]
struct FailureBook {
    failures: u32,
    locked_until: Option<Instant>,
}

impl FailureBook {
    /// Whether a lockout is currently active.
    fn is_locked(&self) -> bool {
        match self.locked_until {
            Some(deadline) => Instant::now() < deadline,
            None => false,
        }
    }

    /// Record one failure; `true` when THIS failure engaged the
    /// lockout (the caller closes 4401 either way).
    fn note_failure(&mut self, limits: &AuthLimits) -> bool {
        self.failures = self.failures.saturating_add(1);
        if self.failures >= limits.max_failures {
            let fresh = self.locked_until.is_none() || !self.is_locked();
            self.locked_until = Some(Instant::now() + limits.lockout);
            return fresh;
        }
        false
    }
}

/// Where the session token comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenSource {
    /// The operator pinned a token in config (tests, scripted setups).
    Fixed(String),
    /// Generated per server start (256 bits; doc 05 §5.4 "per start",
    /// i.e. one per control-server run session) and surfaced to the
    /// HOST, which logs it to the controlling terminal — the token
    /// never appears in this crate's own errors or traces (V8).
    Generated,
}

/// Everything the server needs at construction.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Bind host: exactly `127.0.0.1`, `::1` or `localhost` (resolved
    /// and re-verified); anything else is refused at construction.
    pub bind: String,
    /// Bind port; `0` = ephemeral (the address is queryable).
    pub port: u16,
    /// The deny-by-default sensitive-op policy.
    pub policy: ServerPolicy,
    /// Token source.
    pub token: TokenSource,
    /// Frame/message caps.
    pub limits: Limits,
    /// Socket read/write deadlines.
    pub timeouts: Timeouts,
    /// Auth failure/lockout policy.
    pub auth_limits: AuthLimits,
    /// The `engineInfo` block this host advertises in `welcome`.
    pub engine_info: EngineInfo,
}

impl ServerConfig {
    /// The strict default server: loopback, ephemeral port, generated
    /// token, every sensitive op denied, documented caps. The host
    /// overrides fields from its config file.
    pub fn strict(engine_info: EngineInfo) -> ServerConfig {
        ServerConfig {
            bind: "127.0.0.1".to_owned(),
            port: 0,
            policy: ServerPolicy::deny_all_sensitive(),
            token: TokenSource::Generated,
            limits: Limits::documented(),
            timeouts: Timeouts::documented(),
            auth_limits: AuthLimits::documented(),
            engine_info,
        }
    }
}

/// The host's dispatch seam: one validated request in, one response
/// out. The server owns validation, policy and ordering; the host owns
/// execution (ARCH-5: the vocabulary is the ONE command layer's).
pub trait Dispatch {
    /// Execute `request` and answer. Implementations must not panic
    /// (workspace lints); failures are typed `WireResponse` errors.
    fn dispatch(&mut self, request: WireRequest) -> WireResponse;
}

/// `impl Dispatch` for a closure (tests, small hosts).
impl<F> Dispatch for F
where
    F: FnMut(WireRequest) -> WireResponse,
{
    fn dispatch(&mut self, request: WireRequest) -> WireResponse {
        self(request)
    }
}

/// What one finished session did (structured logging / tests).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummary {
    /// The peer address.
    pub remote: String,
    /// Whether the token handshake completed.
    pub authenticated: bool,
    /// The negotiated major (present once `welcome` was sent).
    pub version: Option<u32>,
    /// Requests dispatched to the host.
    pub requests_served: u32,
    /// Ops refused by the deny-by-default policy.
    pub ops_denied: u32,
    /// Recoverable protocol errors answered (connection survived).
    pub protocol_errors: u32,
    /// Auth failures across the server's life so far.
    pub auth_failures: u32,
    /// Whether the server is locked out after this session.
    pub locked_out: bool,
    /// How the session ended.
    pub close: SessionEnd,
}

/// How a session ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEnd {
    /// Peer sent a close frame (the code it carried, if valid).
    PeerClosed(Option<u16>),
    /// The server closed with this code (typed error path).
    ServerClosed(CloseCode),
    /// Transport EOF without a close frame.
    Eof,
}

/// The loopback control server (card 7.J1).
#[derive(Debug)]
pub struct ControlServer {
    listener: TcpListener,
    config: ServerConfig,
    token: Token,
    entropy: EntropySource,
    failures: FailureBook,
}

impl ControlServer {
    /// Construct and bind. THE acceptance gate: a non-loopback bind
    /// request is a typed error, never a socket (doc 05 §5.4 startup
    /// error rule).
    ///
    /// # Errors
    /// [`ProtocolError::NonLoopbackBind`] for any host other than
    /// `127.0.0.1`, `::1`, `localhost` (resolved to a loopback
    /// address) or `[::1]`; [`ProtocolError::Io`] for OS trouble.
    /// After binding, the bound address is re-verified loopback — a
    /// resolver surprise fails loudly instead of silently listening.
    pub fn bind(config: ServerConfig) -> Result<ControlServer, ProtocolError> {
        let bind_host = config.bind.trim();
        let candidate = match bind_host {
            "127.0.0.1" | "::1" | "[::1]" | "localhost" => bind_host,
            other => {
                return Err(ProtocolError::NonLoopbackBind {
                    requested: other.to_owned(),
                });
            }
        };
        let addr = format!("{candidate}:{}", config.port);
        let resolved = addr
            .to_socket_addrs()
            .map_err(|source| ProtocolError::Io {
                what: "resolving bind address",
                source,
            })?
            .collect::<Vec<_>>();
        if resolved.is_empty() {
            let source = std::io::Error::other("no address resolved for the loopback bind");
            return Err(ProtocolError::Io {
                what: "resolving bind address",
                source,
            });
        }
        let mut last_err: Option<ProtocolError> = None;
        let mut bound: Option<(TcpListener, SocketAddr)> = None;
        for socket in resolved {
            if !socket.ip().is_loopback() {
                // `localhost` resolved to something unexpected.
                return Err(ProtocolError::NonLoopbackBind { requested: addr });
            }
            match TcpListener::bind(socket) {
                Ok(listener) => {
                    let address = listener.local_addr().map_err(|source| ProtocolError::Io {
                        what: "reading local addr",
                        source,
                    })?;
                    bound = Some((listener, address));
                    break;
                }
                Err(source) => {
                    last_err = Some(ProtocolError::Io {
                        what: "binding loopback listener",
                        source,
                    });
                }
            }
        }
        let (listener, bound_addr) = match bound {
            Some(pair) => pair,
            None => {
                return match last_err {
                    Some(err) => Err(err),
                    None => {
                        let source = std::io::Error::other("no bindable loopback address");
                        Err(ProtocolError::Io {
                            what: "binding loopback listener",
                            source,
                        })
                    }
                };
            }
        };
        // Belt and braces: whatever the OS handed back must be loopback.
        if !bound_addr.ip().is_loopback() {
            return Err(ProtocolError::NonLoopbackBind {
                requested: bound_addr.to_string(),
            });
        }
        let (token, entropy) = match &config.token {
            TokenSource::Fixed(token) => (Token(token.clone()), EntropySource::OsRandom),
            TokenSource::Generated => generate_token(),
        };
        Ok(ControlServer {
            listener,
            config,
            token,
            entropy,
            failures: FailureBook::default(),
        })
    }

    /// The address actually bound (host tests point clients here).
    ///
    /// # Errors
    /// [`ProtocolError::Io`] if the OS cannot report it.
    pub fn local_addr(&self) -> Result<SocketAddr, ProtocolError> {
        self.listener
            .local_addr()
            .map_err(|source| ProtocolError::Io {
                what: "reading local addr",
                source,
            })
    }

    /// The live token (host logs it to the controlling terminal; this
    /// is the ONLY sanctioned reader — never an error path, V8).
    pub fn token(&self) -> &str {
        self.token.0.as_str()
    }

    /// Which entropy source produced the token (log WITH the token,
    /// never instead of the warning the weak path deserves).
    pub fn entropy_source(&self) -> EntropySource {
        self.entropy
    }

    /// Whether the auth failure book currently locks new sessions out.
    pub fn is_locked_out(&self) -> bool {
        self.failures.is_locked()
    }

    /// Serve exactly one connection to completion (accept → upgrade →
    /// session → close), then return its summary. Single-connection by
    /// design (doc 05 §5.4's local single-user surface; the host loops).
    ///
    /// # Errors
    /// [`ProtocolError::Io`] for accept/transport trouble. Protocol
    /// breaches are NOT errors here — they are typed closes recorded
    /// in the summary.
    pub fn serve_one(
        &mut self,
        dispatch: &mut dyn Dispatch,
    ) -> Result<SessionSummary, ProtocolError> {
        let (mut stream, remote) = self.listener.accept().map_err(|source| ProtocolError::Io {
            what: "accepting a control connection",
            source,
        })?;
        // Bounded waits on both directions BEFORE any session work: a
        // silent peer can never wedge the (single-connection) server.
        let timeouts = self.config.timeouts;
        stream
            .set_read_timeout(timeouts.read)
            .map_err(|source| ProtocolError::Io {
                what: "setting the read deadline",
                source,
            })?;
        stream
            .set_write_timeout(timeouts.write)
            .map_err(|source| ProtocolError::Io {
                what: "setting the write deadline",
                source,
            })?;
        let mut session = Session::new(&self.config, &self.token, remote.to_string());
        let end = session.run(&mut stream, dispatch, &mut self.failures);
        Ok(SessionSummary {
            remote: session.remote,
            authenticated: session.authenticated,
            version: session.version,
            requests_served: session.requests_served,
            ops_denied: session.ops_denied,
            protocol_errors: session.protocol_errors,
            auth_failures: self.failures.failures,
            locked_out: self.failures.is_locked(),
            close: end,
        })
    }
}

/// One live connection's state.
struct Session {
    remote: String,
    policy: ServerPolicy,
    limits: Limits,
    auth_limits: AuthLimits,
    engine_info: EngineInfo,
    token: Token,
    authenticated: bool,
    version: Option<u32>,
    established: bool,
    last_request_id: Option<u64>,
    requests_served: u32,
    ops_denied: u32,
    protocol_errors: u32,
    subscriptions: Vec<String>,
}

impl Session {
    fn new(config: &ServerConfig, token: &Token, remote: String) -> Session {
        Session {
            remote,
            policy: config.policy.clone(),
            limits: config.limits,
            auth_limits: config.auth_limits,
            engine_info: config.engine_info.clone(),
            token: Token(token.0.clone()),
            authenticated: false,
            version: None,
            established: false,
            last_request_id: None,
            requests_served: 0,
            ops_denied: 0,
            protocol_errors: 0,
            subscriptions: Vec::new(),
        }
    }

    /// The whole session state machine. Returns how it ended.
    fn run(
        &mut self,
        stream: &mut TcpStream,
        dispatch: &mut dyn Dispatch,
        failures: &mut FailureBook,
    ) -> SessionEnd {
        // 1. HTTP upgrade (RFC 6455 §4.2).
        let upgrade = match http::read_upgrade_request(stream) {
            Ok(upgrade) => upgrade,
            Err(ProtocolError::Closed) => return SessionEnd::Eof,
            Err(err) => {
                let status = match &err {
                    ProtocolError::HttpUpgrade {
                        reason: "request-head-too-large",
                    } => 431,
                    _ => 400,
                };
                let bytes = http::error_response(status, "Bad Request");
                let _ = stream.write_all(&bytes);
                let _ = stream.shutdown(std::net::Shutdown::Both);
                return SessionEnd::ServerClosed(err.close_code());
            }
        };
        let accept = http::accept_value(&upgrade.key);
        if let Err(err) = http::write_all(stream, &http::response_101(&accept)) {
            return SessionEnd::ServerClosed(err.close_code());
        }

        // 2. Frame loop.
        let mut assembler = MessageAssembler::new(self.limits.max_message_bytes);
        loop {
            let frame = match read_frame(stream, self.limits.max_frame_bytes) {
                Ok(Some(frame)) => frame,
                Ok(None) => return SessionEnd::Eof,
                Err(err) => return self.fail_session(stream, err),
            };
            match frame.opcode {
                Opcode::Ping => {
                    // RFC 6455 §5.5.2: answer with a pong, same payload.
                    if let Err(err) = write_frame(stream, true, Opcode::Pong, &frame.payload) {
                        return self.fail_session(stream, err);
                    }
                }
                Opcode::Pong => {}
                Opcode::Close => {
                    let code = match parse_close_payload(&frame.payload) {
                        Ok(parsed) => parsed.map(|(code, _)| code),
                        Err(err) => return self.fail_session(stream, err),
                    };
                    // RFC 6455 §5.5.1: echo the close (the peer's code,
                    // or an empty payload when it sent none), then the
                    // TCP close follows.
                    let reply = match code {
                        Some(code) => code.to_be_bytes().to_vec(),
                        None => Vec::new(),
                    };
                    let _ = write_frame(stream, true, Opcode::Close, &reply);
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    return SessionEnd::PeerClosed(code);
                }
                _ => {
                    let message = match assembler.push(frame) {
                        Ok(Some(message)) => message,
                        Ok(None) => continue,
                        Err(err) => return self.fail_session(stream, err),
                    };
                    if message.opcode != Opcode::Text {
                        // Binary app frames are not part of
                        // lumen-control/1 (JSON text only, doc 05 §5.4).
                        self.protocol_errors = self.protocol_errors.saturating_add(1);
                        let error = self.error_frame(
                            ProtocolErrorCode::BadFrame,
                            // lumen-allow-literal: protocol name + major — wire vocabulary, not product identity
                            "lumen-control/1 speaks JSON text frames only",
                        );
                        if let Err(err) = self.send_json(stream, &error) {
                            return self.fail_session(stream, err);
                        }
                        continue;
                    }
                    let text = match String::from_utf8(message.payload) {
                        Ok(text) => text,
                        Err(_) => {
                            return self.fail_session(stream, ProtocolError::InvalidUtf8);
                        }
                    };
                    match self.handle_text(stream, &text, dispatch, failures) {
                        Flow::Continue => {}
                        Flow::End(end) => return end,
                    }
                }
            }
        }
    }

    /// One application text frame.
    fn handle_text(
        &mut self,
        stream: &mut TcpStream,
        text: &str,
        dispatch: &mut dyn Dispatch,
        failures: &mut FailureBook,
    ) -> Flow {
        let value: serde_json::Value = match serde_json::from_str(text) {
            Ok(value) => value,
            Err(err) => {
                return self.recoverable(
                    stream,
                    ProtocolErrorCode::BadJson,
                    &format!(
                        "payload is not valid JSON: {}",
                        crate::protocol::redact_name(&err.to_string())
                    ),
                );
            }
        };
        if !self.established {
            return self.handle_hello(stream, value, failures);
        }
        // Envelope-shape validation runs on the RAW value, BEFORE the
        // typed parse: the flatten-based `ClientFrame::Request` cannot
        // carry `deny_unknown_fields`, and an unknown field would be
        // silently DROPPED by the typed round-trip — the re-serialized
        // request the old post-parse check saw was already clean
        // (found by the conformance suite: `"extra":true` dispatched).
        if let Some(err) = value
            .get("type")
            .and_then(serde_json::Value::as_str)
            .filter(|kind| *kind == "request")
            .and_then(|_| validate_request_shape(&value).err())
        {
            return self.recoverable(stream, ProtocolErrorCode::InvalidRequest, &err.message);
        }
        match serde_json::from_value::<ClientFrame>(value) {
            Ok(ClientFrame::Hello { .. }) => self.fail_with(stream, ProtocolError::HelloTwice),
            Ok(ClientFrame::Request { request }) => self.handle_request(stream, request, dispatch),
            Ok(ClientFrame::Subscribe { events }) => {
                for kind in events {
                    if !self.subscriptions.contains(&kind) {
                        // Bounded: the message cap bounds the union of
                        // kinds (each kind costs ≥ its JSON bytes).
                        self.subscriptions.push(kind);
                    }
                }
                Flow::Continue
            }
            Ok(ClientFrame::Unsubscribe { events }) => {
                if events.is_empty() {
                    self.subscriptions.clear();
                } else {
                    self.subscriptions.retain(|kind| !events.contains(kind));
                }
                Flow::Continue
            }
            Err(err) => {
                let message = format!(
                    "unknown or malformed frame: {}",
                    crate::protocol::redact_name(&err.to_string())
                );
                self.recoverable(stream, ProtocolErrorCode::UnknownType, &message)
            }
        }
    }

    /// The pre-establishment `hello` path.
    fn handle_hello(
        &mut self,
        stream: &mut TcpStream,
        value: serde_json::Value,
        failures: &mut FailureBook,
    ) -> Flow {
        let hello = match serde_json::from_value::<ClientFrame>(value) {
            Ok(ClientFrame::Hello {
                protocol,
                versions,
                token,
            }) => (protocol, versions, token),
            Ok(_) => {
                return self.fail_with(stream, ProtocolError::NotHello);
            }
            Err(_) => {
                return self.fail_with(stream, ProtocolError::NotHello);
            }
        };
        let (protocol, versions, token) = hello;
        if protocol != crate::protocol::PROTOCOL_NAME {
            return self.fail_with(stream, ProtocolError::UnknownProtocol { offered: protocol });
        }
        let Some(version) = negotiate_version(&versions) else {
            return self.fail_with(
                stream,
                ProtocolError::VersionMismatch {
                    client: versions,
                    server: crate::protocol::SUPPORTED_VERSIONS.to_vec(),
                },
            );
        };
        // Token check. Read-only mode admits tokenless sessions to the
        // fixed safe subset (policy::READ_ONLY_SAFE).
        match token {
            Some(presented) => {
                if failures.is_locked() {
                    return self
                        .fail_with(stream, ProtocolError::Unauthorized { locked_out: true });
                }
                if !constant_time_eq(presented.as_bytes(), self.token.0.as_bytes()) {
                    let engaged = failures.note_failure(&self.auth_limits);
                    let _ = engaged; // the summary carries the state
                    return self.fail_with(
                        stream,
                        ProtocolError::Unauthorized {
                            locked_out: failures.is_locked(),
                        },
                    );
                }
                self.authenticated = true;
            }
            None => {
                if !self.policy.read_only() {
                    // No token, no read-only mode: refuse before any
                    // command executes (deny-by-default, doc 05 §5.4).
                    return self
                        .fail_with(stream, ProtocolError::Unauthorized { locked_out: false });
                }
                self.authenticated = false;
            }
        }
        // Welcome (doc 05 §5.4: { v, engineInfo, capabilities }).
        let welcome = ServerFrame::Welcome(Welcome {
            version,
            engine_info: self.engine_info.clone(),
            capabilities: crate::protocol::capabilities_current(
                self.policy.read_only() && !self.authenticated,
                self.limits.max_frame_bytes,
                self.limits.max_message_bytes,
            ),
        });
        self.established = true;
        self.version = Some(version);
        match self.send_json(stream, &welcome) {
            Ok(()) => Flow::Continue,
            Err(err) => self.fail_with(stream, err),
        }
    }

    /// One request frame (monotonicity → policy → dispatch; the raw
    /// envelope SHAPE was validated in [`Self::handle_text`] before the
    /// typed parse — the re-serialized form is provably clean by then).
    fn handle_request(
        &mut self,
        stream: &mut TcpStream,
        request: WireRequest,
        dispatch: &mut dyn Dispatch,
    ) -> Flow {
        // Anti-replay: strictly increasing requestId per connection
        // (doc 05 §4.1/§5.4) — a breach is a protocol close.
        let id = request.request_id.0;
        if let Some(last) = self.last_request_id
            && id <= last
        {
            self.protocol_errors = self.protocol_errors.saturating_add(1);
            let error = self.error_frame(
                ProtocolErrorCode::NonMonotonicRequestId,
                "requestId must be strictly increasing within a connection",
            );
            let _ = self.send_json(stream, &error);
            return Flow::End(self.close_with(stream, CloseCode::AppProtocolError));
        }
        self.last_request_id = Some(id);
        // Deny-by-default sensitive ops (the connection survives; only
        // the op is refused — policy.rs's contract).
        let command_id = request_command_id(&request);
        let allowed = match command_id {
            Some(id) => self.policy.check(id, self.authenticated),
            None => Err(commands::CommandError::new(
                commands::error::Stage::Control,
                commands::error::ErrorCode::Internal,
                "validated request carries no catalog id",
            )),
        };
        if let Err(err) = allowed {
            self.ops_denied = self.ops_denied.saturating_add(1);
            let error = self.error_frame(ProtocolErrorCode::Denied, &err.message);
            return match self.send_json(stream, &error) {
                Ok(()) => Flow::Continue,
                Err(err) => self.fail_with(stream, err),
            };
        }
        let response = dispatch.dispatch(request);
        self.requests_served = self.requests_served.saturating_add(1);
        let frame = ServerFrame::Response { response };
        match self.send_json(stream, &frame) {
            Ok(()) => Flow::Continue,
            Err(err) => self.fail_with(stream, err),
        }
    }

    /// A recoverable application error: typed frame, connection lives.
    fn recoverable(
        &mut self,
        stream: &mut TcpStream,
        code: ProtocolErrorCode,
        message: &str,
    ) -> Flow {
        self.protocol_errors = self.protocol_errors.saturating_add(1);
        let frame = self.error_frame(code, message);
        match self.send_json(stream, &frame) {
            Ok(()) => Flow::Continue,
            Err(err) => self.fail_with(stream, err),
        }
    }

    /// A typed `protocol-error` frame value.
    fn error_frame(&self, code: ProtocolErrorCode, message: &str) -> ServerFrame {
        ServerFrame::ProtocolError {
            code,
            message: crate::protocol::redact_name(message),
        }
    }

    /// Serialize + write one server frame as a text WS frame.
    fn send_json(&self, stream: &mut TcpStream, frame: &ServerFrame) -> Result<(), ProtocolError> {
        let text = serde_json::to_string(frame).map_err(|source| ProtocolError::Io {
            what: "serializing a server frame",
            source: std::io::Error::other(source.to_string()),
        })?;
        write_frame(stream, true, Opcode::Text, text.as_bytes())
    }

    /// Typed failure ending the SESSION (the `run` loop's entry into
    /// [`Self::fail_with`], which always ends the flow).
    fn fail_session(&mut self, stream: &mut TcpStream, err: ProtocolError) -> SessionEnd {
        match self.fail_with(stream, err) {
            Flow::End(end) => end,
            Flow::Continue => unreachable!("fail_with always ends the session"),
        }
    }

    /// Typed failure: send protocol-error + close frame, shut down.
    fn fail_with(&mut self, stream: &mut TcpStream, err: ProtocolError) -> Flow {
        let code = match &err {
            ProtocolError::Unauthorized { .. } => ProtocolErrorCode::Unauthenticated,
            ProtocolError::VersionMismatch { .. } => ProtocolErrorCode::VersionMismatch,
            ProtocolError::UnknownProtocol { .. } => ProtocolErrorCode::UnknownProtocol,
            ProtocolError::NotHello => ProtocolErrorCode::NotHello,
            ProtocolError::HelloTwice => ProtocolErrorCode::HelloTwice,
            ProtocolError::TimedOut { .. } => ProtocolErrorCode::Internal,
            _ => ProtocolErrorCode::BadFrame,
        };
        let frame = self.error_frame(code, &err.to_string());
        let _ = self.send_json(stream, &frame);
        Flow::End(self.close_with(stream, err.close_code()))
    }

    /// Write the close frame + TCP shutdown for `code`.
    fn close_with(&mut self, stream: &mut TcpStream, code: CloseCode) -> SessionEnd {
        let payload = close_payload(code, &err_reason(code));
        let _ = write_frame(stream, true, Opcode::Close, &payload);
        let _ = stream.shutdown(std::net::Shutdown::Both);
        SessionEnd::ServerClosed(code)
    }
}

/// The close-reason text (short, redaction-safe).
fn err_reason(code: CloseCode) -> String {
    match code {
        CloseCode::Normal => "session complete".to_owned(),
        CloseCode::GoingAway => "socket deadline expired (idle or stalled peer)".to_owned(),
        CloseCode::ProtocolError => "websocket protocol violation".to_owned(),
        CloseCode::InvalidUtf8 => "text payload is not valid UTF-8".to_owned(),
        CloseCode::TooBig => "payload exceeded the configured cap".to_owned(),
        // lumen-allow-literal: protocol name — wire vocabulary, not product identity
        CloseCode::AppProtocolError => "lumen-control application protocol error".to_owned(),
        CloseCode::Unauthorized => "token authentication failed".to_owned(),
        CloseCode::VersionMismatch => "no mutually supported protocol major".to_owned(),
    }
}

/// Session-loop control flow.
enum Flow {
    Continue,
    End(SessionEnd),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_loopback_binds_are_refused_at_construction() {
        for bad in ["0.0.0.0", "192.168.1.5", "example.com", "::", "[::]", ""] {
            let mut config = ServerConfig::strict(test_engine_info());
            config.bind = bad.to_owned();
            match ControlServer::bind(config) {
                Err(ProtocolError::NonLoopbackBind { requested }) => {
                    assert_eq!(requested, bad, "the offending address is reported");
                }
                other => unreachable!("bind {bad:?} accepted: {:?}", other.is_ok()),
            }
        }
    }

    #[test]
    fn loopback_binds_succeed_on_both_families() {
        for good in ["127.0.0.1", "::1", "localhost"] {
            let mut config = ServerConfig::strict(test_engine_info());
            config.bind = good.to_owned();
            match ControlServer::bind(config) {
                Ok(server) => {
                    let address = match server.local_addr() {
                        Ok(address) => address,
                        Err(err) => unreachable!("local_addr failed: {err}"),
                    };
                    assert!(address.ip().is_loopback(), "{good} → {address}");
                }
                Err(err) => unreachable!("bind {good:?} refused: {err}"),
            }
        }
    }

    #[test]
    fn failure_book_engages_the_documented_lockout() {
        let limits = AuthLimits {
            max_failures: 3,
            lockout: Duration::from_millis(50),
        };
        let mut book = FailureBook::default();
        assert!(!book.is_locked());
        assert!(!book.note_failure(&limits));
        assert!(!book.note_failure(&limits));
        assert!(!book.is_locked(), "two failures must not lock yet");
        assert!(
            book.note_failure(&limits),
            "the third failure engages the lockout"
        );
        assert!(book.is_locked());
        std::thread::sleep(Duration::from_millis(60));
        assert!(!book.is_locked(), "the lockout expires");
        // After expiry one more failure re-locks (failures persist).
        let mut book2 = FailureBook {
            failures: 2,
            locked_until: None,
        };
        assert!(book2.note_failure(&limits));
    }

    #[test]
    fn documented_limits_are_the_doc_05_numbers() {
        assert_eq!(Limits::documented().max_frame_bytes, 1024 * 1024);
        assert_eq!(Limits::documented().max_message_bytes, 1024 * 1024);
        let auth = AuthLimits::documented();
        assert_eq!(auth.max_failures, 5);
        assert_eq!(auth.lockout, Duration::from_secs(60));
    }

    /// A minimal EngineInfo for construction tests.
    fn test_engine_info() -> EngineInfo {
        EngineInfo {
            engine_version: "0.1.0-test".to_owned(),
            catalog_version: commands::catalog::spec::catalog_checksum().to_string(),
            protocol_version: format!(
                "{}/{}",
                crate::protocol::PROTOCOL_NAME,
                crate::protocol::PROTOCOL_VERSION
            ),
            host: "test".to_owned(),
            features: Vec::new(),
            plugins: Vec::new(),
            decoders: Vec::new(),
            sessions: Vec::new(),
        }
    }
}
