//! Lumen network streaming layer (card 6.A1a; doc 13 §9, doc 01 §2.4).
//!
//! Scope of this scaffold: the **sync HTTP(S) fetch port**
//! ([`fetch::HttpFetch`]), the typed network error vocabulary
//! ([`error::NetworkError`]), and a std-only mock HTTP/1.1 testkit
//! ([`mockhttp`]). Everything else network — the real HTTP client
//! (ureq admission 6.A1b, implementation 6.A1c), ranged seekable
//! resources (6.A2), the scheme registry (6.A3 — landed), HLS (6.B),
//! DASH (6.C), the buffering model and bounded reconnect (6.D1/6.D2 —
//! landed: [`buffer`] + [`reconnect`]), auth (6.E), fuzzing (6.F),
//! and RTSP (6.G) — lands in later cards on top of this port.
//!
//! # Sync by architecture
//!
//! The engine core is threads + channels: no async runtime may appear
//! in engine crates (docs/01-architecture.md §4.2). This crate is
//! std-only and blocking; deadlines are plain [`std::time::Duration`]s
//! carried in the request ([`fetch::TimeoutConfig`]), not executor
//! futures.
//!
//! # Bounded by contract
//!
//! Responses are whole bodies under an explicit memory budget
//! ([`fetch::HttpRequest::response_limit`], MP §12): a hostile server
//! advertising more fails typed ([`error::NetworkError::ResponseTooLarge`])
//! instead of allocating. Incremental/streaming bodies are 6.A2's
//! card.

pub mod auth;
pub mod buffer;
pub mod client;
pub mod clock;
pub mod error;
pub mod fetch;
pub mod hls;
pub mod mockhttp;
pub mod ranged;
pub mod reconnect;
pub mod remote_playlist;
pub mod schemes;
