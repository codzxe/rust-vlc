//! # control-server — the optional loopback control surface (card 7.J1)
//!
//! The host app behind [`control_protocol`]'s wire server (doc 01 §2.8
//! registry row; doc 05 §5.4): it owns everything the protocol crate
//! deliberately does not —
//!
//! * **config** ([`config`]): the operator's `control-server.toml`
//!   (bind / port / read-only / fixed-token / `sensitive_allow`
//!   grants) PLUS the product-model gate: `product.toml`'s
//!   `[features] control-server` must be `true` for the server to start
//!   at all (doc 07 rule F2 — config alone enables nothing; the Lead
//!   flips the flag together with the feature-matrix row to ratify).
//! * **dispatch** ([`dispatch`]): the event-loop bridge WS ↔ command
//!   layer. v1 embeds no engine: it serves the catalog-introspection
//!   subset (`system.get-engine-info`, `system.list-commands`) from the
//!   live catalog and answers every other command with a typed
//!   `not-ready` — an honest gap, never a silent no-op (doc 01 §2.8
//!   (d)). The engine-embedding join is the desktop composition card's.
//! * **logging** ([`logging`]): structured logging — one JSON object
//!   per record, machine-parseable, token-free (V8: the token reaches
//!   the controlling terminal through its single sanctioned reader,
//!   never the log stream).
//! * **run** ([`run`]): the accept/serve event loop with a session cap
//!   for scripted use.
//!
//! Everything transport-shaped — WS framing, handshake, token compare,
//! deny-by-default policy, request validation — is
//! [`control_protocol`]'s; this app composes, it does not re-implement
//! (ARCH-5).
//!
//! ## Honest scope (card 7.J1)
//!
//! Deep security review is deferred to Phase 8 (card 8.A2); the
//! `[PROPOSAL]` rate limits and the Origin allowlist of doc 05 §5.4 are
//! recorded gaps. No signal handling (std-only): SIGTERM/SIGINT end the
//! process and its sessions — acceptable for a local control surface.

pub mod config;
pub mod dispatch;
pub mod logging;
pub mod run;

pub use config::AppError;
pub use config::load_config_file;
pub use config::resolve_server_config;
pub use dispatch::CatalogDispatch;
pub use logging::JsonStderr;
pub use logging::LogRecord;
pub use logging::LogSink;
pub use logging::MemoryLog;
pub use run::RunOutcome;
pub use run::serve_loop;
