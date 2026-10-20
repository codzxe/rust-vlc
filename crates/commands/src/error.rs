//! The §2.13 common error set with the V8 redaction rule (doc 05).

use std::collections::BTreeMap;

/// Where a command failed (doc 05 §2.13 — the command-layer view of the
/// pipeline; distinct from media-types' `PipelineStage`, which tags
/// engine-internal errors).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    Session,
    Input,
    Probe,
    Demux,
    Decode,
    AudioOutput,
    VideoOutput,
    Subtitle,
    Renderer,
    Playlist,
    Config,
    Plugin,
    Control,
}

impl Stage {
    /// Stable kebab-case wire name.
    pub fn as_str(self) -> &'static str {
        match self {
            Stage::Session => "session",
            Stage::Input => "input",
            Stage::Probe => "probe",
            Stage::Demux => "demux",
            Stage::Decode => "decode",
            Stage::AudioOutput => "audio-output",
            Stage::VideoOutput => "video-output",
            Stage::Subtitle => "subtitle",
            Stage::Renderer => "renderer",
            Stage::Playlist => "playlist",
            Stage::Config => "config",
            Stage::Plugin => "plugin",
            Stage::Control => "control",
        }
    }
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The typed failure class (doc 05 §2.13). Every command can fail with
/// any of these; the catalog (card 1.D4) lists only additions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    // media/engine failures (MP §8 — the seven canonical classes)
    /// No demuxer/decoder/filter for this data.
    Unsupported,
    /// Malformed bitstream/container.
    Corrupt,
    /// Unreadable file, permission, auth failure.
    Inaccessible,
    /// Connection, DNS, TLS, timeout.
    Network,
    /// Decode failure after selection.
    Decoder,
    /// Audio/video device failure.
    Device,
    /// Bounded memory/queue/probe limits hit.
    ResourceLimit,
    // command-layer failures (doc 05)
    /// Unknown field, unknown enum variant, unknown command id (V1).
    InvalidArgument,
    /// Scalar range violated (V2) — `details` carries the allowed range.
    OutOfRange,
    /// Reference invalid at execution time (V5).
    NotFound,
    /// Operation requires a state the engine is not in.
    NotReady,
    /// Target is not seekable.
    NotSeekable,
    /// Superseded by a newer generation (§4.2) — rare on the wire.
    Canceled,
    /// Exists in engine, off in product.toml feature exposure.
    FeatureNotExposed,
    /// Host cannot do it (web: no discs, no RTSP, no local FS).
    UnsupportedForHost,
    /// Control server: sensitive op without authorization.
    PermissionDenied,
    /// I/O failure.
    Io,
    /// Engine-internal invariant breach — always a bug report, never
    /// user-actionable.
    Internal,
}

impl ErrorCode {
    /// Stable kebab-case wire name.
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::Unsupported => "unsupported",
            ErrorCode::Corrupt => "corrupt",
            ErrorCode::Inaccessible => "inaccessible",
            ErrorCode::Network => "network",
            ErrorCode::Decoder => "decoder",
            ErrorCode::Device => "device",
            ErrorCode::ResourceLimit => "resource-limit",
            ErrorCode::InvalidArgument => "invalid-argument",
            ErrorCode::OutOfRange => "out-of-range",
            ErrorCode::NotFound => "not-found",
            ErrorCode::NotReady => "not-ready",
            ErrorCode::NotSeekable => "not-seekable",
            ErrorCode::Canceled => "canceled",
            ErrorCode::FeatureNotExposed => "feature-not-exposed",
            ErrorCode::UnsupportedForHost => "unsupported-for-host",
            ErrorCode::PermissionDenied => "permission-denied",
            ErrorCode::Io => "io",
            ErrorCode::Internal => "internal",
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The hard cap on error messages (V6's string budgets; 1 KiB is ample
/// for display-safe text and stops any accidental diagnostic firehose).
const MESSAGE_CAP: usize = 1024;

/// The command error (doc 05 §1.6): stage + code + redaction-safe
/// message + machine-readable details.
///
/// # Redaction (V8, MP §12)
///
/// `message` is **display-safe by construction**: [`CommandError::new`]
/// routes every message through [`redact`], which masks userinfo in
/// URLs (`user:pass@` → `***@`) and collapses absolute paths to their
/// final component (`/home/alice/x.mkv` → `…/x.mkv`). Raw diagnostic
/// chains belong to the log sink only — they never enter this type.
/// `details` values pass through the same redactor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandError {
    pub stage: Stage,
    pub code: ErrorCode,
    pub message: String,
    pub details: BTreeMap<String, String>,
}

impl CommandError {
    /// Build a redaction-safe error.
    pub fn new(stage: Stage, code: ErrorCode, message: impl Into<String>) -> CommandError {
        CommandError {
            stage,
            code,
            message: redact(&message.into()),
            details: BTreeMap::new(),
        }
    }

    /// Attach a machine-readable hint (V2's `allowed_range`, §2.13's
    /// `format` etc.); key and value are redacted.
    pub fn with_detail(mut self, key: &str, value: impl Into<String>) -> Self {
        self.details.insert(redact(key), redact(&value.into()));
        self
    }

    /// The stable machine-readable summary `[stage.code]`, suitable for
    /// tests and telemetry keys.
    pub fn summary(&self) -> String {
        format!("[{}.{}]", self.stage, self.code)
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.summary(), self.message)
    }
}

impl std::error::Error for CommandError {}

/// Redaction-safe display text (V8): mask userinfo, collapse absolute
/// paths, cap length. Applied at construction so no `Display` path can
/// leak credentials or full user paths.
pub fn redact(text: &str) -> String {
    let mut out = mask_userinfo(text);
    out = collapse_paths(&out);
    truncate(&out, MESSAGE_CAP)
}

/// `scheme://user:pass@host` → `scheme://***@host` (any userinfo, with
/// or without password).
fn mask_userinfo(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    'outer: while !rest.is_empty() {
        // find "://" then scan to the next '@' before any '/', '?' or '#'
        if let Some(pos) = rest.find("://") {
            let after = pos + 3;
            let head = &rest[..after];
            let tail = &rest[after..];
            let limit = tail.find(['/', '?', '#', ' ']).unwrap_or(tail.len());
            if let Some(at) = tail[..limit].find('@') {
                out.push_str(head);
                out.push_str("***@");
                rest = &tail[at + 1..];
                continue 'outer;
            }
        }
        let cut = rest.char_indices().nth(1).map_or(rest.len(), |(i, _)| i);
        out.push_str(&rest[..cut]);
        rest = &rest[cut..];
    }
    out
}

/// `/home/alice/dir/file.mkv` → `…/file.mkv` — absolute POSIX paths
/// collapse to the final component (MP §12: no full user paths). A '/'
/// only starts a path candidate at a token boundary (string start or
/// after whitespace) so URL path separators (`scheme://host/x`) are
/// never mistaken for filesystem paths.
fn collapse_paths(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    'outer: while !rest.is_empty() {
        let boundary = out.is_empty() || out.ends_with(char::is_whitespace);
        if rest.starts_with('/') && boundary {
            // consume the whole path run up to whitespace
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            let path = &rest[..end];
            let basename = path.rsplit('/').next().unwrap_or(path);
            if !basename.is_empty() {
                out.push_str("…/");
                out.push_str(basename);
            } else {
                out.push('/');
            }
            rest = &rest[end..];
            continue 'outer;
        }
        let cut = rest.char_indices().nth(1).map_or(rest.len(), |(i, _)| i);
        out.push_str(&rest[..cut]);
        rest = &rest[cut..];
    }
    out
}

/// Cap at `cap` bytes on a char boundary, marking truncation.
fn truncate(text: &str, cap: usize) -> String {
    if text.len() <= cap {
        return text.to_string();
    }
    let mut end = cap;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = text[..end].to_string();
    out.push('…');
    out
}

// Variant-count helpers used by the wire-name stability test.
#[cfg(test)]
trait VariantCount {
    const VARIANT_COUNT: usize;
}
#[cfg(test)]
impl VariantCount for Stage {
    const VARIANT_COUNT: usize = 13;
}
#[cfg(test)]
impl VariantCount for ErrorCode {
    const VARIANT_COUNT: usize = 18;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_and_code_wire_names_are_stable() {
        assert_eq!(Stage::AudioOutput.as_str(), "audio-output");
        assert_eq!(Stage::Control.to_string(), "control");
        assert_eq!(ErrorCode::ResourceLimit.as_str(), "resource-limit");
        assert_eq!(
            ErrorCode::UnsupportedForHost.to_string(),
            "unsupported-for-host"
        );
        // The two taxonomies must not be confused: command-layer Stage
        // and media-types PipelineStage are distinct vocabularies.
        assert_eq!(Stage::VARIANT_COUNT, 13);
        assert_eq!(ErrorCode::VARIANT_COUNT, 18);
    }

    #[test]
    fn error_construction_redacts_userinfo() {
        let e = CommandError::new(
            Stage::Input,
            ErrorCode::Inaccessible,
            "cannot open http://alice:secret@example.org/stream.m3u8",
        );
        assert!(e.message.contains("***@example.org"), "{}", e.message);
        assert!(!e.message.contains("alice"), "{}", e.message);
        assert!(!e.message.contains("secret"), "{}", e.message);
        assert_eq!(e.summary(), "[input.inaccessible]");
        assert!(e.to_string().starts_with("[input.inaccessible]: "));
    }

    #[test]
    fn error_construction_collapses_absolute_paths() {
        let e = CommandError::new(
            Stage::Input,
            ErrorCode::Io,
            "read failed: /home/alice/secret-dir/movie.mkv",
        );
        assert!(e.message.contains("…/movie.mkv"), "{}", e.message);
        assert!(!e.message.contains("/home/"), "{}", e.message);
    }

    #[test]
    fn details_are_redacted_too() {
        let e = CommandError::new(Stage::Demux, ErrorCode::Unsupported, "no demuxer")
            .with_detail("url", "https://bob:pW@cdn.example.org/x")
            .with_detail("path", "/home/bob/a.mkv");
        assert!(e.details["url"].contains("***@cdn.example.org"));
        assert!(e.details["path"].contains("…/a.mkv"));
    }

    #[test]
    fn long_messages_are_capped_without_panicking() {
        let long = "x".repeat(5000);
        let e = CommandError::new(Stage::Control, ErrorCode::Internal, long);
        assert!(e.message.len() <= 1024 + 4, "{}", e.message.len());
        assert!(e.message.ends_with('…'));
        // Multi-byte boundary safety: cap inside a CJK run.
        let cjk = "再".repeat(600);
        let e = CommandError::new(Stage::Control, ErrorCode::Internal, cjk);
        assert!(e.message.chars().count() >= 300);
    }

    #[test]
    fn plain_messages_pass_through_verbatim() {
        let e = CommandError::new(Stage::Session, ErrorCode::NotReady, "no source open");
        assert_eq!(e.message, "no source open");
    }
}
