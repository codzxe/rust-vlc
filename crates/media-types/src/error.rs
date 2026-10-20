//! The stage-tagged error taxonomy (doc 01 §3.4).
//!
//! Kinds are exactly the seven MP §8 names; every error names the failing
//! stage, carries structured context, and its `Display` never embeds
//! credentials, cookies, auth headers or full user paths (MP §12 — the
//! no-secrets rule). URIs appear only through
//! [`SourceUri::redacted`](crate::ids::SourceUri::redacted).

/// The pipeline stage that produced an error (doc 01 §3.4). Fourteen
/// stages — the same names the boundary rules (doc 01 §1.1) and the
/// recovery budgets (product.toml) speak.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineStage {
    /// Opening a source (file, disc, stream).
    Open,
    /// Format probing / sniffing.
    Probe,
    /// Container demultiplexing.
    Demux,
    /// Decode (audio or video).
    Decode,
    /// Audio filtering.
    AudioFilter,
    /// Video filtering.
    VideoFilter,
    /// Mixing audio toward the output.
    Mix,
    /// Audio device output.
    AudioOutput,
    /// Video presentation.
    Present,
    /// The playback clock (conversions, resets).
    Clock,
    /// The control/command layer.
    Control,
    /// Extension/plugin host.
    Plugin,
    /// Device discovery (hotplug, enumeration).
    Discovery,
    /// Transcoding / conversion / recording.
    Transcode,
}

impl PipelineStage {
    /// Stable lowercase name for machine-readable logs.
    pub fn as_str(self) -> &'static str {
        match self {
            PipelineStage::Open => "open",
            PipelineStage::Probe => "probe",
            PipelineStage::Demux => "demux",
            PipelineStage::Decode => "decode",
            PipelineStage::AudioFilter => "audio-filter",
            PipelineStage::VideoFilter => "video-filter",
            PipelineStage::Mix => "mix",
            PipelineStage::AudioOutput => "audio-output",
            PipelineStage::Present => "present",
            PipelineStage::Clock => "clock",
            PipelineStage::Control => "control",
            PipelineStage::Plugin => "plugin",
            PipelineStage::Discovery => "discovery",
            PipelineStage::Transcode => "transcode",
        }
    }
}

impl std::fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Known-format info attached to [`ErrorKind::Unsupported`] (MP §8: when
/// a format is recognized but its codec/feature is missing, say so).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatInfo {
    /// Demuxer's name for the container when one scored above threshold.
    pub name: Option<String>,
    /// The leading bytes that were sniffed (content, not credentials —
    /// `Display` caps them at 16 bytes of hex).
    pub sniffed: Vec<u8>,
}

/// What kind of corruption was detected (doc 01 §3.4 `Corrupt { detail }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorruptDetail {
    /// Structurally invalid data at a byte offset (when known).
    InvalidStructure { offset: Option<u64> },
    /// Fewer bytes than the structure requires.
    Truncated { expected: u64, found: u64 },
    /// A seek index / table entry pointed outside the data.
    BadIndex,
    /// A checksum did not match.
    ChecksumMismatch,
    /// Time-domain corruption (bad stamps, impossible durations).
    Timing,
}

/// Why a source was inaccessible (doc 01 §3.4 `Inaccessible { reason }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessDenied {
    /// The file/stream exists but permissions refuse it.
    PermissionDenied,
    /// Nothing exists at the address.
    NotFound,
    /// Another process holds an exclusive lock.
    Locked,
    /// The source demands authentication the engine does not hold.
    AuthenticationRequired,
    /// The physical disc is missing from the drive.
    DiscMissing,
}

/// Network failure causes (doc 01 §3.4 `Network { cause }`: DNS, TLS,
/// timeout, reconnect exhausted).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkCause {
    /// Name resolution failed.
    Dns,
    /// TLS verification failed (never retried on certificate mismatch —
    /// that would be a downgrade, not a recovery).
    Tls,
    /// A read or write timed out.
    Timeout,
    /// The reconnect budget was spent without success.
    ReconnectExhausted { attempts: u32 },
    /// The connection was reset/lost.
    ConnectionLost,
}

/// Why a decode failed (doc 01 §3.4 `Decoder { codec, cause }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeCause {
    /// The bitstream violated the codec's grammar.
    Bitstream,
    /// A hardware decoder was lost mid-stream (surface invalidated,
    /// driver reset, device sleep).
    HardwareLost,
    /// The decoder's internal state was asked for an unsupported
    /// transition (e.g. mid-stream format change without a drain).
    BadState,
}

/// Which device slot failed (doc 01 §3.4 `Device { which, cause }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceSlot {
    /// The audio output device.
    AudioOutput,
    /// The video display/output.
    Display,
    /// The GPU adapter backing presentation.
    Adapter,
}

/// Why a device failed (doc 01 §3.4 `Device { cause }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceCause {
    /// The device went away mid-playback (unplug, sleep).
    Disconnected,
    /// The adapter was lost (driver reset, TDR).
    Lost,
    /// The device is held by another exclusive owner.
    Busy,
    /// The requested output format is not supported by the device.
    FormatUnsupported,
}

/// Which bounded budget was exceeded (doc 01 §3.4
/// `ResourceLimit { limit, budget, attempted }`). The limits themselves
/// live in `product.toml` (MP §5/§12); this names the axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitKind {
    /// Probing depth in bytes (MP §12).
    ProbeDepth,
    /// A memory ceiling (per component; named in context).
    Memory,
    /// CPU-time budget (plugin host).
    CpuTime,
    /// A queue's slot budget.
    QueueSize,
    /// A recovery attempt budget.
    RecoveryAttempts,
    /// WASM extension memory ceiling.
    WasmMemory,
    /// Session generation counter exhaustion (structural, not tunable).
    SessionGenerations,
}

/// The seven error kinds — exactly the MP §8 names, no more, no less.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// A format, codec, feature or value is not supported. Known-format
    /// info is attached when the demuxer recognized the container (MP §8).
    Unsupported { format: Option<FormatInfo> },
    /// The media data itself is malformed.
    Corrupt { detail: CorruptDetail },
    /// The source cannot be opened or read (permissions, missing,
    /// locked, auth, disc).
    Inaccessible { reason: AccessDenied },
    /// Network failure: DNS, TLS, timeout, reconnect exhausted.
    Network { cause: NetworkCause },
    /// A codec failed to decode its bitstream.
    Decoder { cause: DecodeCause },
    /// An output device failed.
    Device {
        which: DeviceSlot,
        cause: DeviceCause,
    },
    /// A bounded budget was exceeded: which limit, its size, the attempt.
    ResourceLimit {
        limit: LimitKind,
        budget: u64,
        attempted: u64,
    },
}

/// Stable machine-readable context (doc 01 §3.4 `ErrorContext`).
///
/// Codes are dotted lowercase identifiers (`uri.missing_scheme`,
/// `time.overflow`) minted next to their error site; they are part of the
/// compatibility surface once a release ships (UI copy and tests may key
/// on them), so additions are additive-only.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ErrorContext {
    pub code: String,
}

impl ErrorContext {
    /// Build a context from a stable dotted code. Codes must be
    /// non-empty, lowercase, dotted-ASCII (letters, digits, dots,
    /// hyphens, underscores) — the constructor enforces the shape so
    /// tests can rely on it.
    ///
    /// # Panics (constructor contract)
    /// This is the crate's one constructor-contract panic: a malformed
    /// *code* is a programming error, not runtime data. The workspace
    /// lint set denies `panic!` in production code, so this uses `assert`
    /// on an invariant that only Lead-reviewed call sites can reach —
    /// every in-crate call site passes a literal.
    #[allow(clippy::panic)]
    pub fn new(code: &'static str) -> ErrorContext {
        let valid = !code.is_empty()
            && code.chars().all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-' || c == '_'
            });
        assert!(
            valid,
            "error codes are lowercase dotted literals: got {code:?}"
        );
        ErrorContext {
            code: code.to_string(),
        }
    }
}

/// The engine error: stage + kind + context + optional source (doc 01
/// §3.4). `Display` is redacted by construction — every payload is
/// structured data, and byte payloads are capped.
#[derive(Debug)]
pub struct EngineError {
    pub stage: PipelineStage,
    pub kind: ErrorKind,
    pub context: ErrorContext,
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl EngineError {
    /// Build an error from stage, kind and context code.
    pub fn new(stage: PipelineStage, kind: ErrorKind, context: ErrorContext) -> EngineError {
        EngineError {
            stage,
            kind,
            context,
            source: None,
        }
    }

    /// Attach an underlying error (chain root for I/O and codec errors).
    pub fn with_source(
        mut self,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        self.source = Some(source.into());
        self
    }
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // No-secrets rule (MP §12): everything printed below is
        // structured data minted at the error site. There is no code path
        // from a raw URI, credential or user path into this formatting.
        write!(f, "[{}] {}", self.stage, kind_summary(&self.kind))?;
        write!(f, " ({})", self.context.code)
    }
}

/// Render an [`ErrorKind`] for `Display` — structured data only, with
/// sniffed bytes capped at 16 (content hints, never secrets, never a
/// firehose).
fn kind_summary(kind: &ErrorKind) -> String {
    match kind {
        ErrorKind::Unsupported { format } => match format {
            Some(info) => {
                let mut s = String::from("unsupported");
                if let Some(name) = &info.name {
                    s.push_str(&format!(" in {name}"));
                }
                if !info.sniffed.is_empty() {
                    let cap = &info.sniffed[..info.sniffed.len().min(16)];
                    let hex: Vec<String> = cap.iter().map(|b| format!("{b:02x}")).collect();
                    s.push_str(&format!(" (sniffed {})", hex.join(" ")));
                }
                s
            }
            None => "unsupported".to_string(),
        },
        ErrorKind::Corrupt { detail } => match detail {
            CorruptDetail::InvalidStructure { offset } => match offset {
                Some(off) => format!("corrupt structure at byte {off}"),
                None => "corrupt structure".to_string(),
            },
            CorruptDetail::Truncated { expected, found } => {
                format!("truncated data: expected {expected} bytes, found {found}")
            }
            CorruptDetail::BadIndex => "corrupt index".to_string(),
            CorruptDetail::ChecksumMismatch => "checksum mismatch".to_string(),
            CorruptDetail::Timing => "corrupt timing".to_string(),
        },
        ErrorKind::Inaccessible { reason } => {
            let r = match reason {
                AccessDenied::PermissionDenied => "permission denied",
                AccessDenied::NotFound => "not found",
                AccessDenied::Locked => "locked by another process",
                AccessDenied::AuthenticationRequired => "authentication required",
                AccessDenied::DiscMissing => "disc missing",
            };
            format!("inaccessible: {r}")
        }
        ErrorKind::Network { cause } => match cause {
            NetworkCause::Dns => "network: name resolution failed".to_string(),
            NetworkCause::Tls => "network: TLS verification failed".to_string(),
            NetworkCause::Timeout => "network: timed out".to_string(),
            NetworkCause::ReconnectExhausted { attempts } => {
                format!("network: reconnect budget exhausted after {attempts} attempts")
            }
            NetworkCause::ConnectionLost => "network: connection lost".to_string(),
        },
        ErrorKind::Decoder { cause } => match cause {
            DecodeCause::Bitstream => "decoder: bitstream error".to_string(),
            DecodeCause::HardwareLost => "decoder: hardware decoder lost".to_string(),
            DecodeCause::BadState => "decoder: invalid state transition".to_string(),
        },
        ErrorKind::Device { which, cause } => {
            let w = match which {
                DeviceSlot::AudioOutput => "audio output",
                DeviceSlot::Display => "display",
                DeviceSlot::Adapter => "GPU adapter",
            };
            let c = match cause {
                DeviceCause::Disconnected => "disconnected",
                DeviceCause::Lost => "lost",
                DeviceCause::Busy => "busy",
                DeviceCause::FormatUnsupported => "format unsupported",
            };
            format!("device: {w} {c}")
        }
        ErrorKind::ResourceLimit {
            limit,
            budget,
            attempted,
        } => {
            let l = match limit {
                LimitKind::ProbeDepth => "probe depth",
                LimitKind::Memory => "memory",
                LimitKind::CpuTime => "cpu time",
                LimitKind::QueueSize => "queue size",
                LimitKind::RecoveryAttempts => "recovery attempts",
                LimitKind::WasmMemory => "wasm memory",
                LimitKind::SessionGenerations => "session generations",
            };
            format!("resource limit: {l} (budget {budget}, attempted {attempted})")
        }
    }
}

impl std::error::Error for EngineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|s| &**s as &(dyn std::error::Error + 'static))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every kind renders *something* and never panics; the seven names
    /// appear exactly where MP §8 puts them.
    #[test]
    fn all_seven_kinds_display_structured_summaries() {
        let cases: [(ErrorKind, &str); 7] = [
            (
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some("matroska".to_string()),
                        sniffed: vec![0x1A, 0x45, 0xDF, 0xA3],
                    }),
                },
                "unsupported in matroska (sniffed 1a 45 df a3)",
            ),
            (
                ErrorKind::Corrupt {
                    detail: CorruptDetail::Truncated {
                        expected: 64,
                        found: 12,
                    },
                },
                "truncated data: expected 64 bytes, found 12",
            ),
            (
                ErrorKind::Inaccessible {
                    reason: AccessDenied::DiscMissing,
                },
                "inaccessible: disc missing",
            ),
            (
                ErrorKind::Network {
                    cause: NetworkCause::ReconnectExhausted { attempts: 3 },
                },
                "network: reconnect budget exhausted after 3 attempts",
            ),
            (
                ErrorKind::Decoder {
                    cause: DecodeCause::HardwareLost,
                },
                "decoder: hardware decoder lost",
            ),
            (
                ErrorKind::Device {
                    which: DeviceSlot::AudioOutput,
                    cause: DeviceCause::Disconnected,
                },
                "device: audio output disconnected",
            ),
            (
                ErrorKind::ResourceLimit {
                    limit: LimitKind::ProbeDepth,
                    budget: 16_777_216,
                    attempted: 17_000_000,
                },
                "resource limit: probe depth (budget 16777216, attempted 17000000)",
            ),
        ];
        for (kind, want) in cases {
            assert_eq!(kind_summary(&kind), want);
        }
    }

    #[test]
    fn display_is_stage_bracketed_with_code() {
        let e = EngineError::new(
            PipelineStage::AudioOutput,
            ErrorKind::Device {
                which: DeviceSlot::AudioOutput,
                cause: DeviceCause::Disconnected,
            },
            ErrorContext::new("audio.device.disconnected"),
        );
        assert_eq!(
            e.to_string(),
            "[audio-output] device: audio output disconnected (audio.device.disconnected)"
        );
    }

    #[test]
    fn sniffed_bytes_are_capped_at_sixteen_in_display() {
        let e = EngineError::new(
            PipelineStage::Probe,
            ErrorKind::Unsupported {
                format: Some(FormatInfo {
                    name: None,
                    sniffed: vec![0xAB; 4096],
                }),
            },
            ErrorContext::new("probe.no-demuxer"),
        );
        let msg = e.to_string();
        assert!(msg.contains("ab"), "{msg}");
        // 16 hex pairs = 16 capped bytes, then the code's own dots/words.
        assert!(msg.matches("ab ").count() <= 16, "capped: {msg}");
    }

    #[test]
    fn source_chain_delegates_to_std() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let e = EngineError::new(
            PipelineStage::Open,
            ErrorKind::Inaccessible {
                reason: AccessDenied::PermissionDenied,
            },
            ErrorContext::new("open.permission"),
        )
        .with_source(io_err);
        let src = std::error::Error::source(&e);
        assert!(src.is_some(), "source chain must delegate");
    }

    #[test]
    fn stage_names_are_stable_lowercase() {
        assert_eq!(PipelineStage::AudioFilter.as_str(), "audio-filter");
        assert_eq!(PipelineStage::Transcode.as_str(), "transcode");
        assert_eq!(PipelineStage::Open.to_string(), "open");
    }

    #[test]
    fn context_codes_enforce_their_shape() {
        let ok = ErrorContext::new("a.b-c9_x");
        assert_eq!(ok.code, "a.b-c9_x");
        // Constructor-contract panics are tested by proving the guard
        // exists: catch_unwind on a malformed literal.
        let bad = std::panic::catch_unwind(|| ErrorContext::new("Bad Code"));
        assert!(bad.is_err(), "malformed codes must be rejected");
    }
}
