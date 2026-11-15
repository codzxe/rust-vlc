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

/// The Windows verbatim-path marker `std::fs::canonicalize` prefixes
/// drive paths with on Windows (`\\?\C:\…`): not path components, just
/// the OS API's "disable name processing" switch. Stripped before
/// shape detection in [`SourceUri::from_file_path`].
const VERBATIM_PREFIX: &str = "\\\\?\\";

/// The verbatim UNC spelling (`\\?\UNC\server\share\…`): canonicalize's
/// form for a network filesystem path — refused typed by
/// [`SourceUri::from_file_path`] (UNC paths are not local-file
/// sources).
const VERBATIM_UNC_PREFIX: &str = "\\\\?\\UNC\\";

/// Plain UNC paths start with two backslashes (`\\server\share\…`).
const UNC_PREFIX: &str = "\\\\";

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

    /// Build a `file:` URI from a local path's text — the one shared,
    /// platform-aware path→URI constructor (wave 30's Windows open
    /// fix; every former hand-rolled `encode + "file://{…}"` site
    /// delegates here).
    ///
    /// **Pure** (the L0 charter, doc 01 §2.2): no I/O, no
    /// canonicalization — callers canonicalize first and pass
    /// `canonical.to_str()`; non-UTF-8 paths stay the caller's typed
    /// refusal (this crate takes `&str`). The constructor never
    /// guesses a base: relative input is refused, not absolutized.
    ///
    /// # Shape rules (RFC 8089; unit-tested against adversarial fixtures)
    ///
    /// 1. **Verbatim prefixes** (Windows `canonicalize` output): the
    ///    `\\?\` drive-verbatim marker strips (`\\?\C:\Users\a` →
    ///    `C:\Users\a`); `\\?\UNC\server\share\…` is a network path —
    ///    typed refusal.
    /// 2. **Drive forms are the only backslash gate**: a path matching
    ///    `^[A-Za-z]:[/\]` (`C:\Users\a`, `C:/Users/a`) has its `\`
    ///    rewritten to `/`, then gains the empty-authority leading `/`
    ///    (`/C:/Users/a` — RFC 8089 §E.3.1's "local window" shape; the
    ///    resolver-side mirror in `crates/input` strips it back). The
    ///    drive letter and its colon are structural and stay verbatim;
    ///    the percent-encoding keep-set never widens for them.
    /// 3. **Everything else is POSIX and byte-exact**: only
    ///    `ALPHA / DIGIT / '-' / '.' / '_' / '~' / '/'` survive
    ///    verbatim; every other byte percent-encodes as `%HH` (RFC
    ///    3986 §2.1). A legal `\` inside a POSIX filename is *data* —
    ///    encoded `%5C`, decoding back to the same byte. Plain UNC
    ///    (`\\server\share\…`) is refused typed, never mis-encoded.
    ///
    /// # Errors
    /// [`EngineError`] with kind [`Unsupported`](ErrorKind::Unsupported)
    /// and stage `Open`: `uri.file.unc-path` (UNC network path — not a
    /// local-file source), `uri.file.relative-path` (the path is not
    /// absolute; callers canonicalize), or the defensive
    /// `uri.file.invalid` (the encoded form failed [`SourceUri::parse`]
    /// — impossible by construction, never unwrapped). No constructor
    /// carries caller text: `Display` stays structured and redaction
    /// stays the host's job (MP §12).
    pub fn from_file_path(path: &str) -> Result<SourceUri, EngineError> {
        let refusal = |code: &'static str| {
            EngineError::new(
                PipelineStage::Open,
                ErrorKind::Unsupported { format: None },
                ErrorContext::new(code),
            )
        };
        // 1. Verbatim prefixes: the UNC spelling is a network path;
        //    the drive spelling drops its marker and continues.
        let path = if path.starts_with(VERBATIM_UNC_PREFIX) {
            return Err(refusal("uri.file.unc-path"));
        } else {
            path.strip_prefix(VERBATIM_PREFIX).unwrap_or(path)
        };
        // 2. Plain `\\server\share` is UNC — a network path, not a
        //    local-file source.
        if path.starts_with(UNC_PREFIX) {
            return Err(refusal("uri.file.unc-path"));
        }
        // 3. Shape detection. The drive-letter form is the ONLY shape
        //    whose backslashes are separators (a POSIX `\` is filename
        //    data — byte-exact). Its `/C:` prefix is structural (RFC
        //    8089 §E.3.1's `/C:/…` "local window" shape): this
        //    normalization emits the letter and colon verbatim and the
        //    frozen keep-set below never sees them, so POSIX encoding
        //    is unchanged from the retired hand-rolls. POSIX absolutes
        //    encode byte-exact; anything else is relative — the
        //    constructor never guesses a base (callers canonicalize).
        let encoded = if is_windows_drive(path) {
            let normalized = path.replace('\\', "/");
            let (drive, rest) = normalized.split_at(2);
            format!("/{drive}{}", encode_file_path(rest))
        } else if path.starts_with('/') {
            encode_file_path(path)
        } else {
            return Err(refusal("uri.file.relative-path"));
        };
        // 4. Parse can only fail on an internal invariant break (all
        //    spaces and control bytes were escaped in step 3): a
        //    defensive typed mint, never an unwrap.
        SourceUri::parse(&format!("file://{encoded}")).map_err(|_| refusal("uri.file.invalid"))
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

/// The Windows drive-letter shape `^[A-Za-z]:[/\]` — the only input
/// whose backslashes are separators. Everything else is POSIX, where a
/// `\` inside a filename is legal data (byte-exact through
/// [`encode_file_path`]'s `%5C`).
fn is_windows_drive(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\')
}

/// Percent-encode a path for a `file:` URI path slot (RFC 3986 §2.1 /
/// §3.3): keep `unreserved` plus `/`; escape every other byte as `%HH`
/// uppercase.
///
/// The keep-set is deliberately conservative and **frozen**: POSIX
/// outputs are the byte-exactness contract every URI-equality pin rides
/// on, so Windows handling lives in
/// [`SourceUri::from_file_path`]'s pre-encoding normalization — never
/// in a wider keep-set.
fn encode_file_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        let keep = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/');
        if keep {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
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
        // lumen-allow-literal: URL parse/redaction test vector — data under test
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

    // ---- SourceUri::from_file_path (the shared path→URI constructor) --------

    #[test]
    fn from_file_path_is_byte_exact_for_posix() {
        // The exact vectors the CLI's retired hand-rolled encoder pinned
        // (`path_encoder_escapes_the_delimiters`): space, `%`, `?`, `#`
        // and non-ASCII all percent-encode; unreserved bytes and `/`
        // survive verbatim. POSIX outputs are the compatibility
        // contract — byte-identical to every former hand-roll.
        let cases = [
            ("/a b/c%d?e#f", "file:///a%20b/c%25d%3Fe%23f"),
            ("/plain/path.avi", "file:///plain/path.avi"),
            ("/üño", "file:///%C3%BC%C3%B1o"),
            ("/media/My Volume", "file:///media/My%20Volume"),
        ];
        for (path, want) in cases {
            let uri = SourceUri::from_file_path(path).unwrap_or_else(|e| unreachable!("{e}"));
            assert_eq!(uri.raw(), want, "{path:?}");
            assert_eq!(uri.scheme().as_str(), "file");
        }
    }

    #[test]
    fn from_file_path_encodes_a_posix_backslash_as_data() {
        // A `\` inside a POSIX filename is NOT a separator: it
        // percent-encodes to `%5C` and decodes back to the same byte —
        // the reason backslash conversion is gated on the drive-letter
        // shape alone (a POSIX `x\y` must never become `x/y`).
        let uri = SourceUri::from_file_path("/tmp/a\\b x.mp4") // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            uri.raw(),
            "file:///tmp/a%5Cb%20x.mp4" // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        );
        // Round-trip: the resolver's decode returns the original bytes.
        let path = uri
            .raw()
            .strip_prefix("file://")
            .unwrap_or_else(|| unreachable!("from_file_path mints file://"));
        assert_eq!(
            mirror_decode(path),
            "/tmp/a\\b x.mp4" // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        );
    }

    #[test]
    fn from_file_path_strips_the_windows_verbatim_prefix() {
        // Windows canonicalize emits `\\?\C:\…`; the marker strips, the
        // drive form slash-normalizes, the space percent-encodes, and
        // the empty-authority `/` prefixes (RFC 8089 §E.3.1).
        let uri = SourceUri::from_file_path("\\\\?\\C:\\Users\\a b\\x.mp4") // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            uri.raw(),
            "file:///C:/Users/a%20b/x.mp4" // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        );
    }

    #[test]
    fn from_file_path_normalizes_drive_forms() {
        // With and without content after the root; both slash spellings
        // (`C:\` and `C:/x`); lowercase drive letters too.
        let cases = [
            ("C:\\", "file:///C:/"), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            ("C:/x", "file:///C:/x"), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            ("C:\\Users\\a", "file:///C:/Users/a"), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            ("e:\\Media\\film.mp4", "file:///e:/Media/film.mp4"), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        ];
        for (path, want) in cases {
            let uri = SourceUri::from_file_path(path).unwrap_or_else(|e| unreachable!("{e}"));
            assert_eq!(uri.raw(), want, "{path:?}");
        }
    }

    #[test]
    fn from_file_path_rejects_unc_paths_typed() {
        // UNC network paths are not local-file sources: both the
        // verbatim and the plain spelling refuse with the same typed
        // code (never a mis-encoded URI that a resolver would have to
        // reject later).
        for path in [
            "\\\\?\\UNC\\server\\share\\x.mp4", // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            "\\\\server\\share\\x.mp4", // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        ] {
            match SourceUri::from_file_path(path) {
                Err(e) => {
                    assert_eq!(e.context.code, "uri.file.unc-path", "{path:?}");
                    assert_eq!(e.stage, PipelineStage::Open, "{path:?}");
                    assert!(
                        matches!(e.kind, ErrorKind::Unsupported { format: None }),
                        "{path:?}: {e}"
                    );
                    assert_eq!(e.to_string(), "[open] unsupported (uri.file.unc-path)");
                }
                Ok(_) => unreachable!("{path:?} must be refused"),
            }
        }
    }

    #[test]
    fn from_file_path_rejects_relative_paths_typed() {
        // The constructor never guesses a base: callers canonicalize.
        // `C:` without a separator is not the drive form (no boundary
        // after the colon) and not absolute — refused the same way.
        for path in ["relative/x.mkv", "", "C:", "\\no-slash"] {
            match SourceUri::from_file_path(path) {
                Err(e) => {
                    assert_eq!(e.context.code, "uri.file.relative-path", "{path:?}");
                    assert_eq!(e.stage, PipelineStage::Open, "{path:?}");
                    assert!(
                        matches!(e.kind, ErrorKind::Unsupported { .. }),
                        "{path:?}: {e}"
                    );
                }
                Ok(_) => unreachable!("{path:?} must be refused"),
            }
        }
    }

    #[test]
    fn from_file_path_round_trips_through_the_resolver_rules() {
        // Round-trip proof without a dev-dependency on `input` (L2 —
        // the reverse edge is forbidden): the mirror below is input's
        // decode + drive rule, and every constructed URI decodes back
        // to the path the caller handed in (drive forms to their
        // slash-normalized spelling — what `PathBuf` receives).
        let cases = [
            ("/tmp/a b/c%d.mp4", "/tmp/a b/c%d.mp4"), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            ("/tmp/100%.mkv", "/tmp/100%.mkv"), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            ("\\\\?\\C:\\Users\\a b\\x.mp4", "C:/Users/a b/x.mp4"), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            ("C:\\Users\\a", "C:/Users/a"), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            ("E:\\", "E:/"), // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
        ];
        for (path, want) in cases {
            let uri = SourceUri::from_file_path(path).unwrap_or_else(|e| unreachable!("{e}"));
            assert_eq!(mirror_resolve(&uri), want, "{path:?}");
        }
    }

    /// Test-only mirror of `input`'s resolver (`crates/input/src/uri.rs`):
    /// empty/`localhost` authority folds away, the path percent-decodes,
    /// then the RFC 8089 §E.3.1 drive rule strips `/C:/…` back to
    /// `C:/…`. Kept in lockstep by
    /// `from_file_path_round_trips_through_the_resolver_rules`.
    fn mirror_resolve(uri: &SourceUri) -> String {
        let after_scheme = uri
            .raw()
            .strip_prefix("file:")
            .unwrap_or_else(|| unreachable!("from_file_path mints the file scheme"));
        let path = match after_scheme.strip_prefix("//") {
            Some(rest) => {
                let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
                let authority = &rest[..end];
                assert!(
                    authority.is_empty() || authority.eq_ignore_ascii_case("localhost"),
                    "from_file_path never mints an authority: {authority:?}"
                );
                &rest[end..]
            }
            None => after_scheme,
        };
        let decoded = mirror_decode(path);
        // input's exact condition: `/C:/…` (and `/C:\…`) — leading `/`
        // plus drive letter plus `:` plus a boundary — drops the `/`.
        let bytes = decoded.as_bytes();
        if bytes.len() >= 3
            && bytes[0] == b'/'
            && bytes[1].is_ascii_alphabetic()
            && bytes[2] == b':'
            && (bytes.len() == 3 || bytes[3] == b'/' || bytes[3] == b'\\')
        {
            decoded[1..].to_string()
        } else {
            decoded
        }
    }

    /// Test-only RFC 3986 percent-decoder (input's, mirrored). The
    /// constructor's output always carries well-formed `%HH` escapes.
    fn mirror_decode(text: &str) -> String {
        let bytes = text.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' {
                let hi = bytes.get(i + 1).and_then(|b| (*b as char).to_digit(16));
                let lo = bytes.get(i + 2).and_then(|b| (*b as char).to_digit(16));
                match (hi, lo) {
                    (Some(hi), Some(lo)) => {
                        out.push((hi * 16 + lo) as u8);
                        i += 3;
                    }
                    _ => unreachable!("from_file_path emits well-formed escapes"),
                }
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        }
        String::from_utf8(out).unwrap_or_else(|e| unreachable!("{e}"))
    }

    // ---- SourceUri: redaction (MP §12 — adversarial) ----------------------------

    #[test]
    fn redaction_hides_userinfo_passwords() {
        let uri = SourceUri::parse("http://alice:secret@example.org/stream") // lumen-allow-literal: URL parse/redaction test vector — data under test
            .unwrap_or_else(|e| unreachable!("{e}"));
        let r = uri.redacted();
        assert_eq!(r, "http://***@example.org/stream", "userinfo fully masked"); // lumen-allow-literal: URL parse/redaction test vector — data under test
        assert!(!r.contains("alice"), "username must not leak");
        assert!(!r.contains("secret"), "password must not leak");
    }

    #[test]
    fn redaction_hides_user_only_userinfo() {
        let uri =
            SourceUri::parse("http://bob@example.org/x").unwrap_or_else(|e| unreachable!("{e}")); // lumen-allow-literal: URL parse/redaction test vector — data under test
        assert_eq!(uri.redacted(), "http://***@example.org/x"); // lumen-allow-literal: URL parse/redaction test vector — data under test
    }

    #[test]
    fn redaction_masks_secret_query_values_only() {
        let uri = SourceUri::parse(
            "https://cdn.example.org/hls/index.m3u8?token=ABCDE123&segment=42&SIG=deadbeef", // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
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
        let uri = SourceUri::parse("https://example.org/p?start=10&fmt=mp4#frag") // lumen-allow-literal: URL parse/redaction test vector — data under test
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            uri.redacted(),
            "https://example.org/p?start=10&fmt=mp4#frag" // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
        );
    }

    #[test]
    fn redaction_hides_full_user_file_paths() {
        let uri = SourceUri::parse("file:///home/alice/secret-dir/movie.mkv") // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
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
        let uri = SourceUri::parse("http://eve:hunter2@host/p?secret=1") // lumen-allow-literal: URL parse/redaction test vector — data under test
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert!(!uri.to_string().contains("hunter2"));
        assert!(!format!("{uri:?}").contains("eve"));
    }

    #[test]
    fn redaction_handles_authorityless_and_port_forms() {
        let urn = SourceUri::parse("urn:isbn:0451450523").unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(urn.redacted(), "urn:isbn:0451450523");
        let with_port =
            SourceUri::parse("http://example.org:8080/v").unwrap_or_else(|e| unreachable!("{e}")); // lumen-allow-literal: URL parse/redaction test vector — data under test
        assert_eq!(with_port.redacted(), "http://example.org:8080/v"); // lumen-allow-literal: URL parse/redaction test vector — data under test
    }

    #[test]
    fn raw_is_preserved_for_the_access_layer() {
        let uri = SourceUri::parse("http://alice:pw@host/p?token=x") // lumen-allow-literal: URL parse/redaction test vector — data under test
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(uri.raw(), "http://alice:pw@host/p?token=x"); // lumen-allow-literal: URL parse/redaction test vector — data under test
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
