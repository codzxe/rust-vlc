//! The transport seam: where encoded records leave the process (card
//! 2.D3; the `[PROPOSAL]` boundary ADR-004 owns the real webview
//! transports).
//!
//! The protocol (`wire.rs`) is transport-agnostic by construction; a
//! [`FrameTransport`] is "somewhere bytes go". v0 ships the headless
//! trio — [`NullTransport`] (throughput measurement), the shared
//! [`MailboxTransport`] (the bounded latest-wins hand-off the webview
//! consumer drains) and [`CaptureTransport`] (conformance fixtures)
//! — plus the typed pending state for the real hosts:
//!
//! - **Tauri IPC channel** (`tauri::ipc::Channel<Vec<u8>>` or the v2
//!   streaming channel): the intended variant-B desktop transport,
//!   gated on ADR-004 (OS webview exception) ratification plus a
//!   build environment with webkit2gtk-4.1 dev packages — this
//!   sandbox has neither (see
//!   docs/dependencies/records/tauri-wry-2.x.md). `[PROPOSAL]`
//! - **WebSocket / custom `lumen://` protocol**: the documented
//!   alternative for a no-IPC host `[PROPOSAL]`.
//! - **`SharedArrayBuffer` ring**: the zero-copy-adjacent desktop
//!   option where COOP/COEP are host-controlled (doc 08 §7.1 B pros)
//!   `[PROPOSAL: protocol v2]`.

use crate::error::WireError;
use crate::mailbox::{FrameMailbox, MailboxConfig, MailboxError, ShippedFrame};
use crate::wire::WireDecoder;

/// Somewhere encoded frame records go (the ADR-004 seam).
///
/// # Contract for implementors
///
/// - `send` **never blocks the presenter**: the engine's render path
///   calls it at presentation time; a slow consumer must be absorbed
///   by buffering *inside* the transport (the [`FrameMailbox`] is the
///   reference implementation of that discipline), never by stalling
///   the scheduler (doc 03 §4.10 c).
/// - Failures are typed; a transport that cannot take bytes says so
///   and the presenter degrades (audio continues — the doc 03 §4.10 d
///   rule).
pub trait FrameTransport: Send {
    /// Ship one encoded record.
    ///
    /// # Errors
    /// Implementation-defined typed failures (see [`TransportError`]).
    fn send(&mut self, record: &[u8]) -> Result<(), TransportError>;

    /// A stable name for truthful capability reporting (ADR-003
    /// decision 4: the active presentation path is a queryable fact).
    fn name(&self) -> &'static str;
}

/// What a transport can fail with (kept small: every real host wraps
/// its own error into [`TransportError::Hosted`]; the headless trio
/// uses the structured variants).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    /// The backing mailbox is structurally unusable (zero capacity —
    /// a configuration bug).
    MailboxMisconfigured {
        /// Human-readable reason.
        reason: &'static str,
    },
    /// A host-provided failure (already redacted, structured).
    Hosted {
        /// The host's own redacted message.
        message: String,
    },
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::MailboxMisconfigured { reason } => {
                write!(f, "transport: mailbox misconfigured ({reason})")
            }
            TransportError::Hosted { message } => write!(f, "transport: {message}"),
        }
    }
}

impl std::error::Error for TransportError {}

impl From<MailboxError> for TransportError {
    fn from(err: MailboxError) -> TransportError {
        match err {
            MailboxError::Misconfigured { reason } => {
                TransportError::MailboxMisconfigured { reason }
            }
        }
    }
}

/// The measurement transport: accepts and discards, counts bytes and
/// records (the encode-side half of the throughput evidence in
/// `docs/prototypes/video-in-webview.md` §4).
#[derive(Debug, Clone, Default)]
pub struct NullTransport {
    records: u64,
    bytes: u64,
}

impl NullTransport {
    /// A fresh discarding transport.
    #[must_use = "pure constructor"]
    pub fn new() -> NullTransport {
        NullTransport::default()
    }

    /// Records shipped so far.
    #[must_use = "observable evidence"]
    pub fn records(&self) -> u64 {
        self.records
    }

    /// Bytes shipped so far (headers + payloads).
    #[must_use = "observable evidence"]
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
}

impl FrameTransport for NullTransport {
    fn send(&mut self, record: &[u8]) -> Result<(), TransportError> {
        self.records += 1;
        self.bytes += record.len() as u64;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "null"
    }
}

/// The conformance transport: keeps every encoded record (bounded by
/// the caller's discipline — it is a test/fixture type, never wired
/// into a playback session).
#[derive(Debug, Clone, Default)]
pub struct CaptureTransport {
    records: Vec<Vec<u8>>,
}

impl CaptureTransport {
    /// A fresh capturing transport.
    #[must_use = "pure constructor"]
    pub fn new() -> CaptureTransport {
        CaptureTransport::default()
    }

    /// All shipped records, in order.
    #[must_use = "observable evidence"]
    pub fn records(&self) -> &[Vec<u8>] {
        &self.records
    }
}

impl FrameTransport for CaptureTransport {
    fn send(&mut self, record: &[u8]) -> Result<(), TransportError> {
        self.records.push(record.to_vec());
        Ok(())
    }

    fn name(&self) -> &'static str {
        "capture"
    }
}

/// The desktop hand-off transport: the bounded latest-wins
/// [`FrameMailbox`] wearing the [`FrameTransport`] hat, so the bridge
/// can hand records to the very queue the (real or simulated) webview
/// consumer drains — the engine→webview boundary of variant B.
///
/// Records are *decoded* into the mailbox (the consumer side wants
/// frames, not bytes; the mailbox is in-process). The decode is the
/// C7 fix's zero-allocation shape: **one** [`WireDecoder`] lives for
/// the transport's whole life (the old code built a fresh one per
/// `send`), and each record is decoded **in place** — slice-window
/// header parse, one copy into a pooled payload buffer, no
/// pending-buffer round trip, no per-frame allocation after warm-up
/// (measured by `tests/zero_alloc_transport.rs`). The double decode
/// cost of a byte transport → byte consumer is measured and recorded
/// in the prototype doc; the real Tauri IPC host will hand the raw
/// record bytes to the webview instead — the seam keeps both shapes
/// honest.
pub struct MailboxTransport {
    mailbox: FrameMailbox,
    /// THE decoder, reused across frames (D-1/D-8: one stream, one
    /// decoder — never a fresh one per record).
    decoder: WireDecoder,
}

impl MailboxTransport {
    /// Build from a mailbox configuration.
    ///
    /// # Errors
    /// [`TransportError::MailboxMisconfigured`] when the config is
    /// structurally invalid (zero capacity).
    pub fn new(config: MailboxConfig) -> Result<MailboxTransport, TransportError> {
        Ok(MailboxTransport {
            mailbox: FrameMailbox::new(config)?,
            decoder: WireDecoder::new(),
        })
    }

    /// Shared access to the mailbox (the consumer side).
    #[must_use = "the consumer half of the seam"]
    pub fn mailbox(&self) -> &FrameMailbox {
        &self.mailbox
    }

    /// Exclusive access for draining (the consumer side).
    #[must_use = "the consumer half of the seam"]
    pub fn mailbox_mut(&mut self) -> &mut FrameMailbox {
        &mut self.mailbox
    }
}

impl FrameTransport for MailboxTransport {
    fn send(&mut self, record: &[u8]) -> Result<(), TransportError> {
        // Direct hand-off (the C7 fix): the reused decoder parses the
        // record in place — slice-window header, one copy into a
        // pooled payload buffer, zero allocation after warm-up. A
        // record the protocol cannot parse is a host-side bug (the
        // encoder produced it); surface it as a hosted transport
        // failure rather than shipping garbage.
        let frame = self
            .decoder
            .decode_record(record)
            .map_err(|err: WireError| TransportError::Hosted {
                message: err.to_string(),
            })?;
        self.mailbox.offer(frame).map_err(TransportError::from)
    }

    fn name(&self) -> &'static str {
        "mailbox"
    }
}

/// The typed pending state for the real webview transports (the
/// ADR-004 seam's honest face — the 2.C3 `UnsupportedBackend`
/// precedent: enumerate nothing, never fake success).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebviewHostKind {
    /// Tauri v2 IPC channel (`tauri::ipc::Channel`) — the variant-B
    /// desktop transport. Pending ADR-004.
    TauriChannel,
    /// A `lumen://` custom protocol / WebSocket stream. Pending
    /// ADR-004 + protocol v2.
    CustomProtocol,
    /// A `SharedArrayBuffer` ring under host-controlled COOP/COEP.
    /// Pending protocol v2.
    SharedMemory,
}

impl WebviewHostKind {
    /// The ADR that gates every real webview import.
    pub const GATING_ADR: &'static str = "ADR-004";

    /// The honest one-line status for capability reports.
    #[must_use = "pure reporter"]
    pub fn status(self) -> &'static str {
        match self {
            WebviewHostKind::TauriChannel => {
                "pending ADR-004: tauri/wry not imported (no webkit2gtk-4.1 dev packages / no display in the build sandbox)"
            }
            WebviewHostKind::CustomProtocol => {
                "pending ADR-004 + wire protocol v2 (origin handshake)"
            }
            WebviewHostKind::SharedMemory => {
                "pending wire protocol v2 (SharedArrayBuffer ring + COOP/COEP)"
            }
        }
    }
}

/// What the consumer side of a [`ShippedFrame`] hand-off observes
/// (re-exported for transport users; see `mailbox.rs` for the type).
pub type ConsumerFrame = ShippedFrame;
