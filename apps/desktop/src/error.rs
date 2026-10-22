//! Typed errors for the desktop skeleton (card 2.D3; the doc 01 §3.4
//! vocabulary applied at L6).
//!
//! Two families: [`WireError`] for the frame-streaming protocol
//! (malformed records are *data* errors — a hostile or buggy webview
//! feed must never crash the host), and [`HostError`] for the
//! composition-root seam. Both carry structured payloads only; the
//! engine-vocabulary mapping (stage `Present`, stable dotted codes)
//! lives at the presenter boundary in `bridge.rs`.

/// A malformed frame-streaming record (data, not a bug).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    /// The stream does not start with the v1 magic `"LUF1"`.
    BadMagic {
        /// The four offending bytes (hex-stable in `Display`).
        found: [u8; 4],
        /// Byte offset of the record start in the fed stream.
        offset: u64,
    },
    /// A record declares a version this decoder does not speak.
    UnsupportedVersion {
        /// The declared version.
        found: u16,
        /// The version this decoder speaks.
        supported: u16,
    },
    /// `header_len` outside the legal window
    /// [`MIN_HEADER_LEN`](crate::wire::MIN_HEADER_LEN)..=[`MAX_HEADER_LEN`](crate::wire::MAX_HEADER_LEN).
    BadHeaderLen {
        /// The declared length.
        found: u16,
    },
    /// A pixel-format code outside the v1 table (including the YUV
    /// family and hardware handles the bridge refuses — see
    /// `bridge.rs`).
    UnknownFormatCode {
        /// The offending code.
        found: u8,
    },
    /// A dimension or payload-length field that cannot describe a
    /// real frame (zero dims, cap violations).
    BadGeometry {
        /// Human-readable reason (a closed set — geometry validation
        /// has five outcomes, all named).
        reason: &'static str,
    },
    /// The declared payload length disagrees with `width·height·4`.
    PayloadLengthMismatch {
        /// The declared payload length.
        declared: u32,
        /// The geometry-implied (or actual-buffer) length.
        implied: u64,
    },
    /// A reserved field carried non-zero bits (v1 is strict: unknown
    /// extensions must not silently pass as zero-semantics).
    ReservedNotZero {
        /// The offending field name.
        field: &'static str,
    },
    /// The stream ended inside a record (fewer bytes than the record
    /// declared). Pending bytes stay buffered; more input may fix it.
    Truncated {
        /// Bytes still required to complete the current record.
        missing: u64,
    },
    /// Bytes left over after one complete record handed to a
    /// one-record decode (a second record, or a partial one — the
    /// caller's chunking is wrong: one record per hand-off is the
    /// producer's contract).
    TrailingBytes {
        /// Bytes left over after the complete record.
        count: u64,
    },
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WireError::BadMagic { found, offset } => write!(
                f,
                "frame stream: bad magic 0x{:02x}{:02x}{:02x}{:02x} at offset {offset}",
                found[0], found[1], found[2], found[3]
            ),
            WireError::UnsupportedVersion { found, supported } => write!(
                f,
                "frame stream: version {found} is not supported (decoder speaks {supported})"
            ),
            WireError::BadHeaderLen { found } => write!(
                f,
                "frame stream: header_len {found} outside the legal window"
            ),
            WireError::UnknownFormatCode { found } => {
                write!(f, "frame stream: unknown pixel-format code {found}")
            }
            WireError::BadGeometry { reason } => {
                write!(f, "frame stream: bad geometry ({reason})")
            }
            WireError::PayloadLengthMismatch { declared, implied } => write!(
                f,
                "frame stream: payload_len {declared} disagrees with the geometry ({implied})"
            ),
            WireError::ReservedNotZero { field } => {
                write!(f, "frame stream: reserved field {field:?} is not zero")
            }
            WireError::Truncated { missing } => write!(
                f,
                "frame stream: truncated record ({missing} bytes missing; feed more input)"
            ),
            WireError::TrailingBytes { count } => write!(
                f,
                "frame stream: {count} trailing bytes after a complete record \
                 (one record per hand-off)"
            ),
        }
    }
}

impl std::error::Error for WireError {}

/// A host-seam failure (composition-root wiring, not stream data).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostError {
    /// The webview host is not available in this build — the typed
    /// ADR-004 pending state (this sandbox: no webkit2gtk-4.1 dev
    /// packages, no display; `docs/dependencies/records/tauri-wry-2.x.md`).
    WebviewPending {
        /// Which ADR gates the import.
        adr: &'static str,
    },
    /// The frame mailbox rejected a frame structurally (capacity
    /// zero — a configuration bug, not stream data).
    MailboxMisconfigured {
        /// Human-readable reason.
        reason: &'static str,
    },
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostError::WebviewPending { adr } => write!(
                f,
                "desktop host: webview layer pending {adr} (no tauri/wry import in this build)"
            ),
            HostError::MailboxMisconfigured { reason } => {
                write!(f, "desktop host: frame mailbox misconfigured ({reason})")
            }
        }
    }
}

impl std::error::Error for HostError {}
