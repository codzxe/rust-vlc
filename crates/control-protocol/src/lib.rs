//! # control-protocol — the versioned loopback wire protocol (card 7.J1)
//!
//! The remote-control protocol behind `apps/control-server` (doc 01 §2.3
//! registry row, layer L1; doc 05 §5.4; MP §3.5 + §12). Everything a
//! control host needs, and nothing it does not:
//!
//! * [`server`] — an **in-tree minimal RFC 6455 WebSocket server**: TCP
//!   listener, HTTP upgrade handshake, frame codec (FIN/opcode/mask,
//!   16/64-bit length forms, fragmentation, ping/pong/close control
//!   frames), UTF-8 validation for text frames. Loopback-only by
//!   construction: [`server::ControlServer::bind`] refuses any
//!   non-`127.0.0.1`/`::1` address with a typed error (doc 05 §5.4
//!   "deny-by-default is structural, not advisory").
//! * [`protocol`] — the **`lumen-control/1` application handshake**:
//!   first frame must be `hello` naming the protocol and the supported
//!   major versions; mismatch is a typed error + close, never UB (doc 01
//!   §2.3 control-protocol "Never" list). Payloads ride the ONE command
//!   layer: requests are `commands::WireRequest`, responses
//!   `commands::WireResponse` — re-used, never re-implemented (ARCH-5).
//! * [`token`] — per-start 256-bit token authentication with
//!   constant-time comparison; wrong token disconnects with close code
//!   4401 and feeds the failure/backoff book (5 failures ⇒ 60 s lockout,
//!   doc 05 §5.4).
//! * [`policy`] — **deny-by-default sensitive ops**: the catalog commands
//!   that mutate settings, touch filesystem paths, or change plugin
//!   state are refused over the wire unless the server-side config
//!   explicitly grants them (doc 05 §2.13 `permission-denied`).
//! * Request validation: every inbound command is deserialized against
//!   the `commands` catalog types (the typed form OF
//!   `schemas/commands.schema.json`); malformed input is a typed
//!   `protocol-error`/error response and the connection survives.
//! * [`client`] — the in-tree WS conformance client used by this crate's
//!   and the app's tests (doc 01 §2.3 (e): "wire fuzzing, version
//!   negotiation, round-trip properties").
//!
//! ## In-tree implementations (ADR-011 ethos)
//!
//! SHA-1 ([`sha1`]) and base64 ([`base64`]) are implemented in-crate
//! (~100 lines each, specified by RFC 3174 / RFC 4648 and pinned by
//! external test vectors). SHA-1 serves exactly the RFC 6455
//! `Sec-WebSocket-Accept` computation — it is not a general security
//! primitive and must not be used as one.
//!
//! ## Honest scope (card 7.J1)
//!
//! Deep security review is deferred to Phase 8 (card 8.A2). The
//! `[PROPOSAL]`-numbered rate limits of doc 05 §5.4 (100 req/s burst
//! 200, 8 in-flight requests) and the Origin allowlist are recorded
//! gaps, not silent omissions — see [`server::Limits`]. The server
//! serves connections serially (one at a time): a control surface for a
//! local single-user product, bounded by design (D-1).
//!
//! ## What this crate never does
//!
//! No engine logic (the host supplies the dispatch), no non-loopback
//! binding, no unauthenticated sensitive ops, no silent version
//! incompatibility, no `unsafe`/`unwrap`/`expect`/`panic` (workspace
//! lints).

pub mod base64;
pub mod client;
pub mod error;
pub mod frame;
pub mod http;
pub mod policy;
pub mod protocol;
pub mod server;
pub mod sha1;
pub mod token;

pub use error::{CloseCode, ProtocolError};
pub use policy::{PolicyFile, ServerPolicy, is_sensitive, sensitive_command_ids};
pub use protocol::{
    CAPABILITIES_CURRENT, ClientFrame, PROTOCOL_NAME, PROTOCOL_VERSION, ProtocolErrorCode,
    SUPPORTED_VERSIONS, ServerFrame, Welcome, capabilities_current,
};
pub use server::{
    AuthLimits, ControlServer, Dispatch, Limits, ServerConfig, SessionSummary, Timeouts,
    TokenSource,
};
pub use token::{constant_time_eq, generate_token};
