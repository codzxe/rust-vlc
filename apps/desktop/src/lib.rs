//! The Lumen desktop composition-root skeleton (card 2.D3, doc 13
//! wave 2.D; doc 01 §2.7 registry row — L6, "Phase 2 skeleton →
//! Phase 4 product"; the ADR-003 evidence vehicle).
//!
//! # What this crate is
//!
//! The **variant-B frame-streaming prototype core** (doc 08 §7
//! candidate B; ADR-003 decision 2: "frame-streaming B is a
//! transport of the same presented frames — the presentation
//! scheduler owns timing either way"):
//!
//! - [`wire`]      — the v1 record protocol (transport-agnostic
//!   byte framing; the spec table is in the module docs and the
//!   canvas-side reference implementation
//!   `prototype/lumen-frames.mjs` speaks it byte-for-byte).
//! - [`bridge`]    — [`CanvasBridge`], the [`Presenter`] that
//!   serializes presentation decisions into records.
//! - [`transport`] — the [`FrameTransport`] seam (null / capture /
//!   the mailbox) + the typed ADR-004 pending states for the real
//!   webview hosts.
//! - [`mailbox`]   — the bounded latest-wins engine→webview
//!   hand-off with drop accounting and the latency ledger.
//! - [`host`]      — the command-layer correlation seam (doc 05 §4,
//!   host-side half) + ADR-003's per-OS landing as queryable data.
//!
//! # What this crate is not (the honest deferral)
//!
//! **No webview is compiled.** `tauri`/`wry` are allowlist §5
//! "exception-ADR-004" (pre-audited, not imported; ADR-004 status:
//! planned), and this build environment cannot honestly host them:
//! no `webkit2gtk-4.1`/`gtk+-3.0` dev packages (pkg-config probes in
//! `docs/dependencies/records/tauri-wry-2.x.md`), no display server,
//! no GPU stack (the 2.D1 record). Importing them would also put an
//! unaudited graph into `Cargo.lock` against the doc 11 same-PR
//! allowlist rule. The skeleton therefore ships the seam, not the
//! shell — the 2.C3 cpal / 2.D1 wgpu precedent — and
//! `docs/prototypes/video-in-webview.md` carries the run-book for a
//! desktop machine.
//!
//! # Evidence posture (card AC)
//!
//! Measured *here*: protocol determinism (ordering, drop accounting,
//! chunk-alignment independence, golden bytes), cross-language
//! conformance (Rust encoder → the committed JS decoder via
//! `prototype/harness.mjs`, run under Node), and virtual-time
//! end-to-end latency through the real scheduler. NOT measurable
//! here: every per-webview cell (WebView2 / WKWebView / WebKitGTK)
//! — no desktop, no browsers; those cells are recorded as pending
//! with exact run commands, never fabricated.
//!
//! # Example
//!
//! ```
//! use desktop::mailbox::{FrameMailbox, MailboxConfig};
//! use desktop::transport::MailboxTransport;
//! use testkit::Must;
//!
//! # fn main() {
//! let transport = MailboxTransport::new(MailboxConfig::default()).must();
//! let mailbox_len = transport.mailbox().len();
//! assert_eq!(mailbox_len, 0);
//! # }
//! ```
//!
//! # Never
//!
//! Engine logic in the shell (doc 01 §2.7 d); bypassing the command
//! layer for UI-facing operations (ARCH-5); `unsafe`; external
//! dependencies (std-only by C5 until ADR-004 changes that).

pub mod bridge;
pub mod dispatch;
pub mod error;
pub mod files;
pub mod host;
pub mod mailbox;
pub mod transport;
pub mod window_config;
pub mod wire;

pub use bridge::{BridgeStats, CanvasBridge, EmitClock, MonotonicEmitClock};
pub use dispatch::{EventSink, ShellHost, lumen_dispatch, open_file_request};
pub use error::{HostError, WireError};
pub use files::{DragDropHandler, NativeDialog, NativeFileDialog, PathDragDrop, is_media_path};
pub use host::{
    DesktopHost, EventDelivery, PresentationVariant, ResponseObservation, StrategyReport, TargetOs,
    strategy_for,
};
pub use mailbox::{
    FrameMailbox, LatencyAccountant, LatencySummary, MailboxConfig, MailboxError, MailboxStats,
    ShippedFrame,
};
pub use transport::{
    CaptureTransport, FrameTransport, MailboxTransport, NullTransport, TransportError,
    WebviewHostKind,
};
pub use window_config::{WindowConfig, WindowConfigError, parse_window_table};
pub use wire::{
    DecodedFrame, FormatCode, FrameFlags, FrameHeader, MAGIC, MAX_DIMENSION, MAX_HEADER_LEN,
    MAX_PAYLOAD_LEN, MIN_HEADER_LEN, VERSION, WireDecoder,
};
