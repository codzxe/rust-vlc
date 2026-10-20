//! Identity types (doc 01 §3.1).
//!
//! Sources, streams, tracks and generations — the nouns of the pipeline —
//! plus the validated, redactable [`SourceUri`] that anchors MP §12's
//! no-secrets rule.

use crate::error::{EngineError, ErrorContext, ErrorKind, PipelineStage};
use crate::time::TrackTiming;

/// A validated URI scheme (RFC 3986 grammar: `ALPHA *( ALPHA / DIGIT /
/// "+" / "-" / "." )`, ASCII case-insensitive, stored lowercase).
///
/// The set is deliberately **open**: protocols are plugins (MP §6), so
/// schemes are data, not a closed enum. Construction validates the grammar
/// and normalizes case; comparison is exact on the normalized form.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Scheme(String);

impl Scheme {
    /// Validate and normalize a scheme string.
    ///
    /// # Errors
    /// [`EngineError`] with kind [`Unsupported`](ErrorKind::Unsupported)
    /// and stage `Open` when the string is empty or violates the RFC 3986
    /// scheme grammar.
    pub fn parse(scheme: &str) -> Result<Scheme, EngineError> {
        let invalid = || {
            EngineError::new(
                PipelineStage::Open,
                ErrorKind::Unsupported { format: None },
                ErrorContext::new("uri.scheme.invalid"),
            )
        };
        if scheme.is_empty() {
            return Err(invalid());
        }
        let lowered = scheme.to_ascii_lowercase();
        let mut chars = lowered.chars();
        match chars.next() {
            Some(first) if first.is_ascii_alphabetic() => {}
            _ => return Err(invalid()),
        }
        if !chars.all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') {
            return Err(invalid());
        }
        Ok(Scheme(lowered))
    }

    /// The normalized scheme string (lowercase, valid grammar).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The credential-shaped query-parameter names [`SourceUri::redacted`]
/// masks. Parameter *names* stay visible (they are not secrets and keep
/// logs debuggable); only the *values* are replaced.
const SECRET_QUERY_KEYS: [&str; 10] = [
    "token",
    "key",
    "apikey",
    "api_key",
    "auth",
    "signature",
    "sig",
    "password",
    "secret",
    "session",
];

/// The redaction marker substituted for secret values and userinfo.
const REDACTED: &str = "***";

/// A validated source URI.
///
/// `Display`/`Debug` show a redacted form (MP §12): credentials and full
/// user paths never appear in logs or errors. The raw form exists only in
/// memory for the access layer that needs to open the source.
///
/// # Redaction rules (all unit-tested against adversarial fixtures)
///
/// 1. **Userinfo** — `user:pass@` and `user@` both collapse to `***@`.
/// 2. **Local paths** — for the `file` scheme only the final path
///    component survives, prefixed `…/` (no `/home/alice/...` in logs).
/// 3. **Credential-shaped query values** — `?token=…` becomes
///    `?token=***` for the parameter names listed above; other query
///    parameters pass through verbatim (queries are stream-plumbing, not
///    credentials, and hiding them entirely would make logs useless).
/// 4. Everything else (scheme, host, port, path, fragment) is kept
///    verbatim.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SourceUri {
    scheme: Scheme,
    raw: String,
}

impl SourceUri {
    /// Parse and validate a source URI.
    ///
    /// Accepts any `scheme:rest` where `scheme` passes [`Scheme::parse`]
    /// and `rest` contains no ASCII control characters and no spaces.
    /// This is deliberately permissive about *path* content (protocols are
    /// plugins; scheme-specific validation belongs to the access layer,
    /// MP §6) while enforcing the two invariants every consumer relies on:
    /// a syntactically valid scheme and a printable remainder.
    ///
    /// # Errors
    /// [`EngineError`] with kind [`Unsupported`](ErrorKind::Unsupported)
    /// and stage `Open` when the string has no `scheme:` prefix, an
    /// invalid scheme, or control characters/spaces in the remainder.
    pub fn parse(s: &str) -> Result<SourceUri, EngineError> {
        let bad = |code: &'static str| {
            EngineError::new(
                PipelineStage::Open,
                ErrorKind::Unsupported { format: None },
                ErrorContext::new(code),
            )
        };
        let (scheme, rest) = s.split_once(':').ok_or_else(|| bad("uri.missing_scheme"))?;
        let scheme = Scheme::parse(scheme).map_err(|_| bad("uri.scheme.invalid"))?;
        if rest.chars().any(|c| c.is_ascii_control() || c == ' ') {
            return Err(bad("uri.control_chars"));
        }
        Ok(SourceUri {
            scheme,
            raw: s.to_string(),
        })
    }

    /// The validated scheme (open set — protocols are plugins, MP §6).
    pub fn scheme(&self) -> &Scheme {
        &self.scheme
    }

    /// The redacted form for logs, errors and UI. See the type docs for
    /// the exact rules; the implementation is tested against adversarial
    /// fixtures (`redaction_is_adversarially_tested` and friends).
    pub fn redacted(&self) -> String {
        let (authority, path_query_frag) = split_authority(&self.raw);
        let mut out = String::with_capacity(self.raw.len());
        out.push_str(self.scheme.as_str());
        out.push(':');
        if let Some(auth) = authority {
            out.push_str("//");
            if let Some((_userinfo, hostport)) = auth.rsplit_once('@') {
                out.push_str(REDACTED);
                out.push('@');
                out.push_str(hostport);
            } else {
                out.push_str(auth);
            }
        }
        if self.scheme.as_str() == "file" {
            out.push_str(&redact_file_path(path_query_frag));
        } else {
            out.push_str(&redact_query_values(path_query_frag));
        }
        out
    }

    /// The raw URI, unchanged. **Only** the access layer may call this —
    /// anything user-visible goes through [`redacted`](Self::redacted).
    pub fn raw(&self) -> &str {
        &self.raw
    }
}

/// Split `scheme://authority/rest` into `(Some(authority), rest)` when an
/// authority component is present, else `(None, everything-after-scheme)`.
fn split_authority(uri: &str) -> (Option<&str>, &str) {
    let after_scheme = uri.split_once(':').map(|(_, rest)| rest).unwrap_or("");
    match after_scheme.strip_prefix("//") {
        Some(rest) => match rest.find(['/', '?', '#']) {
            Some(end) => (Some(&rest[..end]), &rest[end..]),
            None => (Some(rest), ""),
        },
        None => (None, after_scheme),
    }
}

/// `file` scheme: keep only the final path component, prefixed `…/`.
/// Query and fragment (rare but legal) are preserved through the query
/// redactor.
fn redact_file_path(path_query_frag: &str) -> String {
    let (path, suffix) = match path_query_frag.find(['?', '#']) {
        Some(i) => (&path_query_frag[..i], &path_query_frag[i..]),
        None => (path_query_frag, ""),
    };
    let mut out = String::with_capacity(path.len() + suffix.len() + 3);
    let basename = path.rsplit('/').next().unwrap_or(path);
    if path == "/" {
        out.push('/');
    } else if path.starts_with('/') {
        // Absolute: keep the leading slash so `file:///…/x.mkv` keeps its
        // three-slash shape (scheme, empty authority, absolute path).
        out.push_str("/…/");
        out.push_str(basename);
    } else if !path.is_empty() {
        out.push_str("…/");
        out.push_str(basename);
    }
    out.push_str(&redact_query_values(suffix));
    out
}

/// Replace the values of credential-shaped query parameters with `***`.
/// Non-query text passes through verbatim.
fn redact_query_values(text: &str) -> String {
    let Some(qpos) = text.find('?') else {
        return text.to_string();
    };
    let (head, query_and_frag) = text.split_at(qpos);
    let (query, frag) = match query_and_frag.find('#') {
        Some(f) => (&query_and_frag[..f], &query_and_frag[f..]),
        None => (query_and_frag, ""),
    };
    let mut out = String::with_capacity(text.len());
    out.push_str(head);
    out.push('?');
    let mut first = true;
    for pair in query[1..].split('&') {
        if !first {
            out.push('&');
        }
        first = false;
        let (key, _value) = match pair.split_once('=') {
            Some((k, v)) => (k, v),
            None => (pair, ""),
        };
        if SECRET_QUERY_KEYS.contains(&key.to_ascii_lowercase().as_str()) {
            out.push_str(key);
            out.push('=');
            out.push_str(REDACTED);
        } else {
            out.push_str(pair);
        }
    }
    out.push_str(frag);
    out
}

impl std::fmt::Display for SourceUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // MP §12: Display is the redacted form — the raw form never
        // reaches logs, errors or the UI through any Display/Debug path.
        f.write_str(&self.redacted())
    }
}

impl std::fmt::Debug for SourceUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SourceUri({})", self.redacted())
    }
}

/// Demuxer stream index, stable within one generation (doc 01 §3.1).
///
/// A new open/seek/stop/source replacement/track switch bumps the
/// [`GenerationId`] and may reuse stream indices; consumers that hold
/// `StreamId`s across generations must pair them with the generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StreamId(u32);

impl StreamId {
    /// Construct from a demuxer stream index.
    pub fn new(index: u32) -> StreamId {
        StreamId(index)
    }

    /// The raw demuxer stream index.
    pub fn index(self) -> u32 {
        self.0
    }
}

/// Engine track handle, stable across a session (doc 01 §3.1).
///
/// Unlike [`StreamId`], a `TrackId` is never reused within a session,
/// even across generations — it is the long-lived handle every command,
/// event and packet carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TrackId(u32);

impl TrackId {
    /// Construct a track handle.
    pub fn new(handle: u32) -> TrackId {
        TrackId(handle)
    }

    /// The raw handle value.
    pub fn handle(self) -> u32 {
        self.0
    }
}

/// Monotonic per session; bumped on open/seek/stop/source
/// replacement/track switch (doc 01 §3.1, MP §8).
///
/// The engine owns bumping; this type only guarantees distinctness by
/// construction (callers compare, they never parse).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GenerationId(u64);

impl GenerationId {
    /// The initial generation of a session.
    pub const FIRST: GenerationId = GenerationId(0);

    /// Construct from a raw counter value.
    pub fn new(counter: u64) -> GenerationId {
        GenerationId(counter)
    }

    /// The raw counter value.
    pub fn counter(self) -> u64 {
        self.0
    }

    /// The next generation in the monotonic sequence (checked; a u64
    /// counter that has run its full course does not wrap — it errors).
    ///
    /// # Errors
    /// [`EngineError`] with kind
    /// [`ResourceLimit`](ErrorKind::ResourceLimit) and stage `Control`
    /// when the counter is exhausted.
    pub fn next(self) -> Result<GenerationId, EngineError> {
        self.0.checked_add(1).map(GenerationId).ok_or_else(|| {
            EngineError::new(
                PipelineStage::Control,
                ErrorKind::ResourceLimit {
                    limit: crate::error::LimitKind::SessionGenerations,
                    budget: u64::MAX,
                    attempted: u64::MAX,
                },
                ErrorContext::new("generation.exhausted"),
            )
        })
    }
}

/// What a track carries (doc 01 §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaKind {
    /// Audio samples.
    Audio,
    /// Video pictures.
    Video,
    /// Subtitle cues.
    Subtitle,
    /// Chapters, timed metadata and other non-AV streams.
    Data,
}

/// A BCP 47 language tag (validated, ASCII case-normalized).
///
/// v1 accepts the common `lang` / `lang-REGION` / `lang-Script` shapes
/// (2–8 letter primary subtag, then subtags of letters/digits separated by
/// `-`); full BCP 47 is a data problem owned by the localization crate,
/// not the vocabulary crate — mismatched shapes are rejected early with
/// an actionable error instead of silently matching nothing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LanguageTag(String);

impl LanguageTag {
    /// Validate a language tag.
    ///
    /// # Errors
    /// [`EngineError`] with kind [`Unsupported`](ErrorKind::Unsupported)
    /// and stage `Demux` when the tag is empty, has empty subtags, or a
    /// primary subtag that is not 2–8 ASCII letters.
    pub fn parse(tag: &str) -> Result<LanguageTag, EngineError> {
        let invalid = || {
            EngineError::new(
                PipelineStage::Demux,
                ErrorKind::Unsupported { format: None },
                ErrorContext::new("track.language.invalid"),
            )
        };
        if tag.is_empty() || tag.starts_with('-') || tag.ends_with('-') || tag.contains("--") {
            return Err(invalid());
        }
        let mut subtags = tag.split('-');
        let primary = subtags.next().ok_or_else(invalid)?;
        if !(2..=8).contains(&primary.len()) || !primary.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(invalid());
        }
        if !subtags.all(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric())) {
            return Err(invalid());
        }
        // Normalize: primary lowercase; then every subtag — 4 ASCII
        // letters are a script (Titlecase), 2 ASCII letters are a region
        // (UPPERCASE), anything else lowercase. This is canonical BCP 47
        // casing applied position-independently.
        let mut parts: Vec<String> = tag.split('-').map(str::to_string).collect();
        for part in parts.iter_mut() {
            if part.chars().all(|c| c.is_ascii_alphabetic()) {
                if part.len() == 2 {
                    *part = part.to_ascii_uppercase();
                } else if part.len() == 4 {
                    let mut chars = part.chars();
                    if let Some(first) = chars.next() {
                        let mut cased = String::with_capacity(part.len());
                        cased.extend(first.to_uppercase());
                        cased.extend(chars.flat_map(|c| c.to_lowercase()));
                        *part = cased;
                    }
                } else {
                    *part = part.to_ascii_lowercase();
                }
            }
        }
        parts[0] = parts[0].to_ascii_lowercase();
        Ok(LanguageTag(parts.join("-")))
    }

    /// The normalized tag string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for LanguageTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// An opaque codec capability descriptor issued by the codec registry
/// (doc 01 §3.1: "opaque capability string from the registry").
///
/// `media-types` deliberately does not know what codecs exist: the string
/// is minted by the registry crate, compared for equality here, and
/// rendered in errors for the user. Non-empty by construction.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CodecDescriptor(String);

impl CodecDescriptor {
    /// Wrap a registry-issued descriptor string.
    ///
    /// # Errors
    /// [`EngineError`] with kind [`Unsupported`](ErrorKind::Unsupported)
    /// and stage `Demux` when the descriptor is empty or contains control
    /// characters (registry contract violations surface at the boundary).
    pub fn new(descriptor: &str) -> Result<CodecDescriptor, EngineError> {
        if descriptor.is_empty() || descriptor.chars().any(|c| c.is_ascii_control()) {
            return Err(EngineError::new(
                PipelineStage::Demux,
                ErrorKind::Unsupported { format: None },
                ErrorContext::new("track.codec.descriptor_invalid"),
            ));
        }
        Ok(CodecDescriptor(descriptor.to_string()))
    }

    /// The descriptor string (already user-renderable; never a secret).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CodecDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Track flags carried from container metadata (doc 01 §3.1).
///
/// The set is intentionally tiny: flags *every* container family can
/// express or default sensibly. Format-specific oddities stay in their
/// demuxer and surface as track metadata, not vocabulary flags.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct TrackFlags {
    /// The container marked this track as the default for its kind.
    pub default_on: bool,
    /// The track is marked forced (e.g. forced subtitles).
    pub forced: bool,
    /// The track carries visual descriptions/accessibility data.
    pub accessibility: bool,
    /// The track is a still-image track (rare; e.g. cover art audio files
    /// with an attached video stream).
    pub still_image: bool,
}

impl TrackFlags {
    /// No flags set.
    pub const NONE: TrackFlags = TrackFlags {
        default_on: false,
        forced: false,
        accessibility: false,
        still_image: false,
    };
}

/// One decoded track of a source, as reported by the engine (doc 01 §3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    /// Long-lived engine handle (stable across the session).
    pub id: TrackId,
    /// Demuxer stream index (stable only within one generation).
    pub stream: StreamId,
    /// What the track carries.
    pub kind: MediaKind,
    /// BCP 47 language tag when the container declares one.
    pub language: Option<LanguageTag>,
    /// Opaque codec capability descriptor from the registry.
    pub codec: CodecDescriptor,
    /// Human-readable title from media content — data, not UI copy
    /// (localization applies at render time, never here).
    pub title: Option<String>,
    /// Container-declared flags (default-on, forced, ...).
    pub flags: TrackFlags,
    /// Span, time base and known gaps.
    pub timing: TrackTiming,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Scheme ----------------------------------------------------------------

    #[test]
    fn scheme_parses_and_normalizes_case() {
        let s = Scheme::parse("HTtPs").unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(s.as_str(), "https");
        assert_eq!(s.to_string(), "https");
    }

    #[test]
    fn scheme_rejects_bad_grammar() {
        for bad in ["", "1http", "ht tp", "http:", "héllo", "-x"] {
            assert!(Scheme::parse(bad).is_err(), "{bad:?} must be rejected");
        }
    }

    // ---- SourceUri: parse --------------------------------------------------------

    #[test]
    fn source_uri_requires_a_scheme() {
        match SourceUri::parse("no-scheme-here") {
            Err(e) => {
                assert!(matches!(e.kind, ErrorKind::Unsupported { .. }));
                assert_eq!(e.stage, PipelineStage::Open);
                assert_eq!(e.context.code, "uri.missing_scheme");
            }
            Ok(_) => unreachable!("must fail without a scheme"),
        }
    }

    #[test]
    fn source_uri_rejects_control_characters_and_spaces() {
        for bad in ["http://a\tb", "http://a b/c", "file:///x\ny"] {
            assert!(SourceUri::parse(bad).is_err(), "{bad:?} must be rejected");
        }
    }

    #[test]
    fn source_uri_accepts_plugin_schemes() {
        // Open set: protocols are plugins (MP §6) — an unknown-but-valid
        // scheme parses.
        let uri = SourceUri::parse("totally-new-protocol://host/path")
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(uri.scheme().as_str(), "totally-new-protocol");
    }

    // ---- SourceUri: redaction (MP §12 — adversarial) ----------------------------

    #[test]
    fn redaction_hides_userinfo_passwords() {
        let uri = SourceUri::parse("http://alice:secret@example.org/stream")
            .unwrap_or_else(|e| unreachable!("{e}"));
        let r = uri.redacted();
        assert_eq!(r, "http://***@example.org/stream", "userinfo fully masked");
        assert!(!r.contains("alice"), "username must not leak");
        assert!(!r.contains("secret"), "password must not leak");
    }

    #[test]
    fn redaction_hides_user_only_userinfo() {
        let uri =
            SourceUri::parse("http://bob@example.org/x").unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(uri.redacted(), "http://***@example.org/x");
    }

    #[test]
    fn redaction_masks_secret_query_values_only() {
        let uri = SourceUri::parse(
            "https://cdn.example.org/hls/index.m3u8?token=ABCDE123&segment=42&SIG=deadbeef",
        )
        .unwrap_or_else(|e| unreachable!("{e}"));
        let r = uri.redacted();
        assert!(r.contains("token=***"), "token value masked: {r}");
        assert!(
            r.contains("SIG=***"),
            "signature masked case-insensitively: {r}"
        );
        assert!(r.contains("segment=42"), "non-secret query survives: {r}");
        assert!(!r.contains("ABCDE123"), "token value must not leak");
        assert!(!r.contains("deadbeef"), "sig value must not leak");
    }

    #[test]
    fn redaction_keeps_benign_queries_and_fragments() {
        let uri = SourceUri::parse("https://example.org/p?start=10&fmt=mp4#frag")
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            uri.redacted(),
            "https://example.org/p?start=10&fmt=mp4#frag"
        );
    }

    #[test]
    fn redaction_hides_full_user_file_paths() {
        let uri = SourceUri::parse("file:///home/alice/secret-dir/movie.mkv")
            .unwrap_or_else(|e| unreachable!("{e}"));
        let r = uri.redacted();
        assert_eq!(r, "file:///…/movie.mkv");
        assert!(!r.contains("alice"), "user name must not leak");
        assert!(!r.contains("home"), "directory tree must not leak");
    }

    #[test]
    fn redaction_file_scheme_still_masks_query_secrets() {
        let uri = SourceUri::parse("file:///media/data/set.mkv?token=hunter2")
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(uri.redacted(), "file:///…/set.mkv?token=***");
    }

    #[test]
    fn redaction_display_and_debug_are_redacted() {
        // MP §12: no Display/Debug path ever shows the raw form.
        let uri = SourceUri::parse("http://eve:hunter2@host/p?secret=1")
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert!(!uri.to_string().contains("hunter2"));
        assert!(!format!("{uri:?}").contains("eve"));
    }

    #[test]
    fn redaction_handles_authorityless_and_port_forms() {
        let urn = SourceUri::parse("urn:isbn:0451450523").unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(urn.redacted(), "urn:isbn:0451450523");
        let with_port =
            SourceUri::parse("http://example.org:8080/v").unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(with_port.redacted(), "http://example.org:8080/v");
    }

    #[test]
    fn raw_is_preserved_for_the_access_layer() {
        let uri = SourceUri::parse("http://alice:pw@host/p?token=x")
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(uri.raw(), "http://alice:pw@host/p?token=x");
    }

    // ---- ids --------------------------------------------------------------------

    #[test]
    fn ids_are_distinct_newtypes() {
        let s = StreamId::new(3);
        let t = TrackId::new(3);
        // Different types cannot be compared — this *is* the test; the
        // values below just pin the accessors.
        assert_eq!(s.index(), 3);
        assert_eq!(t.handle(), 3);
    }

    #[test]
    fn generation_bumps_and_errors_at_exhaustion() {
        let g = GenerationId::FIRST;
        let g1 = g.next().unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(g1.counter(), 1);
        assert!(GenerationId::new(u64::MAX).next().is_err());
    }

    // ---- LanguageTag -------------------------------------------------------------

    #[test]
    fn language_tag_normalizes_canonical_casing() {
        assert_eq!(
            LanguageTag::parse("EN")
                .unwrap_or_else(|e| unreachable!("{e}"))
                .as_str(),
            "en"
        );
        assert_eq!(
            LanguageTag::parse("zh-hans-cn")
                .unwrap_or_else(|e| unreachable!("{e}"))
                .as_str(),
            "zh-Hans-CN"
        );
        assert_eq!(
            LanguageTag::parse("PT-br")
                .unwrap_or_else(|e| unreachable!("{e}"))
                .as_str(),
            "pt-BR"
        );
    }

    #[test]
    fn language_tag_rejects_malformed() {
        for bad in ["", "-en", "en-", "e--n", "1n", "toolongprimarysubtag"] {
            assert!(LanguageTag::parse(bad).is_err(), "{bad:?} must be rejected");
        }
    }

    // ---- CodecDescriptor -----------------------------------------------------------

    #[test]
    fn codec_descriptor_rejects_empty_and_control_chars() {
        assert!(CodecDescriptor::new("").is_err());
        assert!(CodecDescriptor::new("mp4a\t40.2").is_err());
        let ok = CodecDescriptor::new("mp4a.40.2").unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(ok.as_str(), "mp4a.40.2");
    }

    // ---- TrackFlags -----------------------------------------------------------------

    #[test]
    fn track_flags_none_is_all_clear() {
        assert_eq!(TrackFlags::NONE, TrackFlags::default());
        let none = TrackFlags::NONE;
        assert!(!none.default_on && !none.forced && !none.accessibility && !none.still_image);
    }
}
