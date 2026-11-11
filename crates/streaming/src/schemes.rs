//! Scheme dispatch registry (card 6.A3) — the single point mapping a
//! source reference to an access strategy.
//!
//! Everything the player can be asked to open arrives here first
//! (doc 13 §9 wave 6.A; doc 06 §1.2 "access factory" row):
//!
//! * **bare local paths** and **`file:` URIs** →
//!   [`AccessKind::LocalFile`] (the normalized local path). Opening
//!   stays with the `input` crate's file access at the composition
//!   site — `streaming` and `input` are same-layer (L2) siblings, so
//!   this crate names the strategy and hands over the path instead of
//!   reaching sideways (the boundaries.toml same-layer rule; the CLI
//!   open-path composition card owns that join).
//! * **`http:` / `https:` URLs** → [`AccessKind::Http`] (the port's
//!   validated, normalized [`Url`]), openable through
//!   [`SchemeRegistry::open_http`] as the A1b/A1c/A2a/A2b composition:
//!   the hardened [`UreqClient`](crate::client::UreqClient) transport
//!   feeding the ranged seekable
//!   [`RangedResource`](crate::ranged::RangedResource).
//! * **every other scheme** (`ftp:`, `rtsp:`, `data:`, …) → the typed
//!   refusal [`SchemeError::Unsupported`] naming the scheme. The
//!   per-scheme *policy* (ftp blocked v1, RTSP family pending 6.G1-I)
//!   lives in the record `docs/formats/network-http.md`, not in code:
//!   the registry is uniform, the record is the decision ledger.
//!
//! # Normalization (card 6.A3: "URL parsing/normalization")
//!
//! * Schemes are matched ASCII case-insensitively and lowercased
//!   (RFC 3986 §3.1), both here and in [`Url::parse`].
//! * `http`/`https` normalization is the port's: default ports (80 /
//!   443) made explicit, path defaults to `/`.
//! * `file:` URIs: an empty or `localhost` authority folds away; the
//!   path is percent-decoded; `file:/path` and `file:///path` are the
//!   same resource.
//! * A single ASCII letter before the first `:` (the `c:` Windows
//!   drive-letter shape) is a **local path**, not a scheme — the
//!   one-look disambiguation rule, recorded in the scheme record.
//!
//! # Credential-free logging (card 6.A3)
//!
//! [`SchemeError`] never embeds the source reference: its `Display`
//! is structured data (scheme name / static detail / authority host)
//! plus the dotted code, mirroring [`NetworkError`]'s no-secrets rule.
//! URI-shaped credentials are refused *before* dispatch — the port's
//! [`Url::parse`] rejects userinfo outright — and the log-facing URI
//! form for composition sites is `SourceUri::redacted` (media-types,
//! MP §12).

use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use media_types::error::{ErrorContext, FormatInfo};
use media_types::{EngineError, ErrorKind, PipelineStage};

use crate::client::UreqClient;
use crate::clock::WallClock;
use crate::error::NetworkError;
use crate::fetch::{HttpFetch, Url};
use crate::ranged::{RangedConfig, RangedResource};
use crate::reconnect::{RetryFetch, RetryPolicy};

/// The access strategy a source reference dispatches to (card 6.A3).
///
/// Plain and owned: the composition site (the CLI open path) matches
/// on it once and opens through the owning layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessKind {
    /// Local filesystem access: a bare path or a normalized `file:`
    /// URI. Open through the `input` crate's file access at the
    /// composition site (see the module docs for the layering note).
    LocalFile {
        /// The normalized local path (percent-decoded for `file:`
        /// URIs; passed through verbatim for bare paths —
        /// canonicalization is the composition site's job, matching
        /// the existing CLI open path).
        path: PathBuf,
    },
    /// HTTP(S) network access: the port's validated URL, openable as
    /// the ranged seekable resource via
    /// [`SchemeRegistry::open_http`].
    Http {
        /// The parsed, normalized `http`/`https` URL.
        url: Url,
    },
}

/// Typed refusal from scheme dispatch (card 6.A3).
///
/// House style (the [`NetworkError`] pattern): thiserror-free, stable
/// dotted codes ([`SchemeError::code`]), structured `Display`, and a
/// [`std::error::Error`] impl that keeps the underlying parse failure
/// in the `source` chain. Never carries the source reference itself
/// — no URI, no userinfo, no query (credential-free logging).
#[derive(Debug)]
pub enum SchemeError {
    /// The scheme is known to the player but no access strategy is
    /// registered for it (`streaming.scheme.unsupported`). Names the
    /// scheme; the *policy* behind each refusal (blocked v1, pending
    /// investigation) is the record's truth, `docs/formats/
    /// network-http.md`.
    Unsupported {
        /// The refused scheme, lowercased (`"ftp"`, `"rtsp"`, `"data"`, …).
        scheme: String,
    },
    /// The source reference is structurally malformed
    /// (`streaming.scheme.malformed`): empty, control bytes, an
    /// invalid `file:` URI shape, or an `http(s)` URL the port
    /// refused (the [`NetworkError`] rides the `source` chain).
    Malformed {
        /// Structured, static detail (never caller data).
        detail: &'static str,
        /// The port's parse failure for `http(s)` sources, preserved.
        source: Option<Box<NetworkError>>,
    },
    /// A `file:` URI named a non-local authority
    /// (`streaming.scheme.file-host`): `file://remotehost/media` is a
    /// network filesystem reference this player does not open. The
    /// authority is named (hosts are not credentials).
    RemoteFileHost {
        /// The non-local authority as written (host or host:port).
        host: String,
    },
}

impl SchemeError {
    /// The stable dotted context code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            SchemeError::Unsupported { .. } => "streaming.scheme.unsupported",
            SchemeError::Malformed { .. } => "streaming.scheme.malformed",
            SchemeError::RemoteFileHost { .. } => "streaming.scheme.file-host",
        }
    }

    /// Bridge into the engine taxonomy (doc 01 §3.4), stage `Open`,
    /// mirroring `plugin_api::scheme_not_served`'s shape: the scheme
    /// name rides [`ErrorKind::Unsupported`]'s format info so engine
    /// surfaces can name it. The composition card consumes this; the
    /// registry's own API stays [`SchemeError`].
    #[must_use]
    pub fn into_engine_error(self) -> EngineError {
        let (kind, code) = match &self {
            SchemeError::Unsupported { scheme } => (
                ErrorKind::Unsupported {
                    format: Some(FormatInfo {
                        name: Some(scheme.clone()),
                        sniffed: Vec::new(),
                    }),
                },
                "streaming.scheme.unsupported",
            ),
            SchemeError::Malformed { .. } => (
                ErrorKind::Unsupported { format: None },
                "streaming.scheme.malformed",
            ),
            SchemeError::RemoteFileHost { .. } => (
                ErrorKind::Unsupported { format: None },
                "streaming.scheme.file-host",
            ),
        };
        match self {
            SchemeError::Malformed {
                source: Some(inner),
                ..
            } => EngineError::new(PipelineStage::Open, kind, ErrorContext::new(code))
                .with_source(inner),
            _ => EngineError::new(PipelineStage::Open, kind, ErrorContext::new(code)),
        }
    }
}

impl fmt::Display for SchemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            SchemeError::Unsupported { scheme } => {
                format!("URL scheme {scheme:?} has no access strategy")
            }
            SchemeError::Malformed { detail, .. } => {
                format!("malformed source reference: {detail}")
            }
            SchemeError::RemoteFileHost { host } => {
                format!("file URI names a non-local authority {host:?}")
            }
        };
        write!(f, "scheme dispatch failed: {text} ({})", self.code())
    }
}

impl Error for SchemeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SchemeError::Malformed {
                source: Some(inner),
                ..
            } => Some(inner.as_ref()),
            _ => None,
        }
    }
}

/// The scheme dispatch registry (card 6.A3): the single point mapping
/// scheme → access strategy, plus the HTTP(S) access-plugin
/// composition behind it.
///
/// [`SchemeRegistry::classify`] is pure and total — no I/O, no
/// sockets, no clock: every source reference is either a strategy
/// ([`AccessKind`]) or a typed refusal ([`SchemeError`]). The
/// transport handle is carried only for
/// [`SchemeRegistry::open_http`], so tests and alternative stacks
/// inject the mock testkit (or any [`HttpFetch`]) via
/// [`SchemeRegistry::with_fetch`].
pub struct SchemeRegistry {
    /// The HTTP(S) transport every `open_http` rides (the ureq-backed
    /// client in production; the mock testkit in tests).
    fetch: Arc<dyn HttpFetch>,
}

impl Default for SchemeRegistry {
    fn default() -> Self {
        SchemeRegistry::new()
    }
}

impl SchemeRegistry {
    /// The production registry: the hardened ureq transport
    /// (cards 6.A1b/6.A1c — rustls TLS with verification always on,
    /// bounded redirects, cross-host policy, loop detection).
    pub fn new() -> SchemeRegistry {
        SchemeRegistry {
            fetch: Arc::new(UreqClient::new()),
        }
    }

    /// A registry over an injected transport — the mock testkit's
    /// client, or an alternative stack behind the same port.
    pub fn with_fetch(fetch: Arc<dyn HttpFetch>) -> SchemeRegistry {
        SchemeRegistry { fetch }
    }

    /// A registry over an injected transport **wrapped in the bounded
    /// reconnect layer** (card 6.D2): every `open_http` fetch — the
    /// probe, the window fetches, the linear-scan bodies — retries
    /// retryable transport weather under `policy`'s bounded backoff,
    /// paced by `clock` (tests inject `FakeClock`; production passes
    /// `SystemClock::shared`). The composition is the 6.D2 adoption
    /// point: the ranged resource and everything above it adopt
    /// retry by receiving this registry's transport, unchanged.
    ///
    /// Hosts that need the generation handle (seek/reopen
    /// invalidation) construct the [`RetryFetch`] themselves and pass
    /// it to [`SchemeRegistry::with_fetch`] — the handle is the
    /// wrapper's to give ([`RetryFetch::generation`]).
    ///
    /// # Errors
    /// [`RetryPolicy::validate`]'s verdict, verbatim.
    pub fn with_retrying_fetch(
        fetch: Arc<dyn HttpFetch>,
        policy: RetryPolicy,
        clock: Arc<dyn WallClock>,
    ) -> Result<SchemeRegistry, &'static str> {
        Ok(SchemeRegistry {
            fetch: Arc::new(RetryFetch::new_checked(fetch, policy, clock)?),
        })
    }

    /// The single scheme-dispatch point (pure, total, no I/O).
    ///
    /// # Errors
    /// [`SchemeError::Unsupported`] for every scheme that is neither
    /// `file` nor `http`/`https`; [`SchemeError::Malformed`] for
    /// structurally invalid references; [`SchemeError::RemoteFileHost`]
    /// for `file:` URIs naming a non-local authority.
    pub fn classify(source: &str) -> Result<AccessKind, SchemeError> {
        if source.is_empty() {
            return Err(SchemeError::Malformed {
                detail: "the source reference is empty",
                source: None,
            });
        }
        if source.chars().any(|c| c.is_ascii_control()) {
            return Err(SchemeError::Malformed {
                detail: "the source reference contains control bytes",
                source: None,
            });
        }
        match split_scheme(source) {
            // A bare path (no scheme prefix): local file access,
            // passed through verbatim.
            None => Ok(AccessKind::LocalFile {
                path: PathBuf::from(source),
            }),
            Some((scheme, rest)) => match scheme.as_str() {
                "file" => classify_file_uri(rest),
                "http" => Url::parse(source)
                    .map(|url| AccessKind::Http { url })
                    .map_err(|err| SchemeError::Malformed {
                        detail: "the http(s) URL was refused by the fetch port",
                        source: Some(Box::new(err)),
                    }),
                // ADR-011 (accepted 2026-11-09): the ureq+rustls(ring)
                // E2-class deviation is the ratified exception, so https
                // classifies through the same fetch-port parse as http and
                // rides the transport's always-verifying rustls TLS. The
                // pre-ratification seam (`HTTPS_ADR011_ADMITTED` flip
                // point + the `HttpsNotAdmitted` refusal vocabulary) is
                // deleted per the ADR's acceptance instructions — history:
                // `docs/adr/ADR-011-tls-https.md` §"The seam".
                "https" => Url::parse(source)
                    .map(|url| AccessKind::Http { url })
                    .map_err(|err| SchemeError::Malformed {
                        detail: "the http(s) URL was refused by the fetch port",
                        source: Some(Box::new(err)),
                    }),
                other => Err(SchemeError::Unsupported {
                    scheme: other.to_string(),
                }),
            },
        }
    }

    /// Open an HTTP(S) source as the ranged seekable resource — the
    /// card-6.A3 access-plugin composition: this registry's transport
    /// (the ureq-backed client from 6.A1b/6.A1c) feeding
    /// [`RangedResource`] (6.A2a ranged windows + 6.A2b linear-scan
    /// fallback), serving the engine's [`SourceStream`] contract.
    ///
    /// The probe request goes out here; classify first when the
    /// source string is untrusted (an unsupported scheme must be
    /// refused *before* any socket — pinned by test).
    ///
    /// # Errors
    /// [`EngineError`] (stage `Open`) — the ranged resource's own
    /// typed refusals (config bounds, transport failures, the
    /// no-range policy), see [`RangedResource::open`].
    pub fn open_http(
        &self,
        url: &Url,
        config: RangedConfig,
    ) -> Result<RangedResource, EngineError> {
        RangedResource::open(Arc::clone(&self.fetch), &url.to_string(), config)
    }
}

/// Split `source` into `(scheme, rest)` when it carries a scheme
/// prefix, else `None` (a bare local path).
///
/// A scheme prefix is the first `:` followed-by-nothing special case
/// of RFC 3986 §3.1: an ASCII letter, then letters/digits/`+`/`-`/`.`,
/// lowercased. Two recorded disambiguation rules:
///
/// * A **single-letter** prefix (`c:` — the Windows drive-letter
///   shape) is a local path, not a scheme.
/// * A prefix that fails the scheme grammar (`/media/x:y` — the
///   colon sits inside a path segment) leaves the whole reference a
///   local path. A *valid* scheme prefix (`name.tar:x`) is a URI per
///   RFC 3986 §4.2's first-segment rule and will be refused typed as
///   an unsupported scheme — recorded in the scheme record.
fn split_scheme(source: &str) -> Option<(String, &str)> {
    let (prefix, rest) = source.split_once(':')?;
    let mut chars = prefix.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return None,
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')) {
        return None;
    }
    if prefix.len() == 1 {
        // `c:` — the drive-letter shape, not a scheme.
        return None;
    }
    Some((prefix.to_ascii_lowercase(), rest))
}

/// Classify the `rest` of a `file:` URI (after `file:`, authority
/// unstripped). RFC 8089 shapes: `file:///path`, `file://localhost/
/// path`, `file:/path` — all fold to the same local path.
fn classify_file_uri(rest: &str) -> Result<AccessKind, SchemeError> {
    let path = if let Some(after) = rest.strip_prefix("//") {
        // An authority is present: empty or `localhost` folds away;
        // anything else is a network filesystem reference.
        let (authority, path) = match after.split_once('/') {
            Some((authority, tail)) => (authority, format!("/{tail}")),
            None => (after, String::new()),
        };
        if authority.is_empty() || authority.eq_ignore_ascii_case("localhost") {
            path
        } else {
            return Err(SchemeError::RemoteFileHost {
                host: authority.to_string(),
            });
        }
    } else if rest.starts_with('/') {
        rest.to_string()
    } else {
        return Err(SchemeError::Malformed {
            detail: "a file URI must carry an absolute path",
            source: None,
        });
    };
    if path.is_empty() || path == "/" {
        return Err(SchemeError::Malformed {
            detail: "the file URI has an empty path",
            source: None,
        });
    }
    let decoded = percent_decode(&path)?;
    Ok(AccessKind::LocalFile {
        path: PathBuf::from(decoded),
    })
}

/// Percent-decode a `file:` URI path (RFC 3986 §2.4).
///
/// # Errors
/// [`SchemeError::Malformed`] for invalid `%`-escapes, and — the
/// recorded v1 limit — for escapes that do not decode to UTF-8
/// (non-UTF-8 filesystem paths reach the player through the bare-path
/// arm, which passes OS strings through natively; only the URI
/// grammar is UTF-8 here).
fn percent_decode(path: &str) -> Result<String, SchemeError> {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = |b: u8| match b {
                b'0'..=b'9' => Some(b - b'0'),
                b'a'..=b'f' => Some(b - b'a' + 10),
                b'A'..=b'F' => Some(b - b'A' + 10),
                _ => None,
            };
            let (hi, lo) = match (bytes.get(i + 1).copied(), bytes.get(i + 2).copied()) {
                (Some(h), Some(l)) => (hex(h), hex(l)),
                _ => (None, None),
            };
            match (hi, lo) {
                (Some(hi), Some(lo)) => {
                    out.push((hi << 4) | lo);
                    i += 3;
                }
                _ => {
                    return Err(SchemeError::Malformed {
                        detail: "the file URI carries an invalid percent-escape",
                        source: None,
                    });
                }
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| SchemeError::Malformed {
        detail: "the file URI percent-escapes do not decode to UTF-8",
        source: None,
    })
}

/// One fuzz pass over untrusted source-reference bytes (card 6.A3
/// `+FB`; doc 04 §8.5 registry name `fuzz_scheme_classify`).
///
/// The harness contract is absence-of-panic/hang/unbounded-
/// allocation, nothing else (the adts precedent): `classify` is total
/// by construction — every input is a strategy or a typed refusal —
/// and this pass plus the deterministic in-crate sweeps prove it on
/// mutated shapes. Non-UTF-8 bytes return early (the registry's
/// grammar is the `&str` surface; raw-OS-path sources take the
/// bare-path arm at the composition site, never this parser).
///
/// Bound: inputs above [`FUZZ_INPUT_LIMIT`] return without parsing —
/// the recorded composition-site budget for one source reference.
pub fn fuzz_scheme_classify(data: &[u8]) {
    if data.len() > FUZZ_INPUT_LIMIT {
        return;
    }
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = SchemeRegistry::classify(text);
}

/// Fuzz-input ceiling: 64 KiB, the testkit's hostile-head bound
/// (`mockhttp::HEAD_LIMIT` territory). Source references arrive from
/// argv/config at the composition site and are OS-capped long before
/// this; the ceiling keeps the fuzz contract explicit anyway.
pub const FUZZ_INPUT_LIMIT: usize = 64 * 1024;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FakeClock;
    use crate::mockhttp::{MockClient, MockConfig, MockServer, ScriptedStatus};
    use media_types::ids::SourceUri;
    use plugin_api::SourceStream;
    use testkit::Must;

    // -- classification positives --------------------------------------

    /// Bare paths — absolute, relative, and the drive-letter shape —
    /// dispatch to local file access verbatim.
    #[test]
    fn bare_paths_classify_local() {
        for source in [
            "/media/corpus/a.wav",
            "media/a.wav",
            "./a.wav",
            "c:/Users/alice/a.avi", // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            "name with spaces.wav",
        ] {
            match SchemeRegistry::classify(source) {
                Ok(AccessKind::LocalFile { path }) => {
                    assert_eq!(path, PathBuf::from(source), "{source:?} passes through");
                }
                other => unreachable!("bare path must classify local: {other:?}"),
            }
        }
    }

    /// `file:` URIs normalize to local paths: authority forms fold,
    /// percent-escapes decode, scheme case is irrelevant.
    #[test]
    fn file_uris_normalize_to_local_paths() {
        for (source, want) in [
            ("file:///media/a.wav", "/media/a.wav"),
            ("file://localhost/media/a.wav", "/media/a.wav"),
            ("file://LOCALHOST/media/a.wav", "/media/a.wav"),
            ("file:/media/a.wav", "/media/a.wav"),
            ("FILE:///media/a.wav", "/media/a.wav"),
            ("file:///media/a%20b.wav", "/media/a b.wav"),
            ("file:///media/100%2Fpercent.wav", "/media/100/percent.wav"),
        ] {
            match SchemeRegistry::classify(source) {
                Ok(AccessKind::LocalFile { path }) => {
                    assert_eq!(path, PathBuf::from(want), "{source:?} normalizes");
                }
                other => unreachable!("{source:?} must classify local: {other:?}"),
            }
        }
    }

    /// `http(s)` URLs classify through the port's parser and come
    /// back normalized (scheme folded, default ports, path defaulted).
    #[test]
    fn http_urls_classify_normalized() {
        // lumen-allow-literal: URL parse/redaction test vector — data under test
        match SchemeRegistry::classify("HTTP://Example.com") {
            Ok(AccessKind::Http { url }) => {
                assert_eq!(url.scheme, "http");
                assert_eq!(url.host, "Example.com");
                assert_eq!(url.port, 80, "the scheme default port is explicit");
                assert_eq!(url.path_and_query, "/");
            }
            other => unreachable!("http URL must classify: {other:?}"),
        }
        // lumen-allow-literal: URL parse/redaction test vector — data under test
        match SchemeRegistry::classify("https://cdn.example.org:8443/seg/x.mp4?range=1") {
            Ok(AccessKind::Http { url }) => {
                assert_eq!(url.scheme, "https");
                assert_eq!(url.port, 8443);
                assert_eq!(url.path_and_query, "/seg/x.mp4?range=1");
            }
            other => unreachable!("https URL must classify: {other:?}"),
        }
    }

    /// ADR-011 acceptance pin (2026-11-09): https classifies exactly
    /// like http — the same [`AccessKind::Http`] strategy, the same
    /// fetch-port parse and scheme-default normalization — so every
    /// https source rides the identical `open_http` composition (the
    /// ureq transport's always-verifying rustls TLS in production).
    /// The pre-ratification seam (`HTTPS_ADR011_ADMITTED` + the
    /// `HttpsNotAdmitted` refusal vocabulary) is deleted per the
    /// ADR's acceptance instructions; this pin exists so a drifted
    /// re-introduction of scheme-specific https refusal fails loud,
    /// not silently.
    #[test]
    fn https_classifies_exactly_like_http_post_adr011() {
        for source in [
            "https://cdn.example.org/live/master.m3u8", // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
            "HTTPS://Example.com", // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
            "https://cdn.example.org:8443/seg/x.mp4?range=1", // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
        ] {
            match SchemeRegistry::classify(source) {
                Ok(AccessKind::Http { url }) => {
                    assert_eq!(url.scheme, "https");
                }
                other => unreachable!("admitted https must classify: {other:?}"),
            }
        }
        // lumen-allow-literal: URL parse/redaction test vector — data under test
        match SchemeRegistry::classify("https://cdn.example.org/live/master.m3u8") {
            Ok(AccessKind::Http { url }) => {
                assert_eq!(url.port, 443, "the scheme default port is explicit");
                assert_eq!(url.path_and_query, "/live/master.m3u8");
            }
            other => unreachable!("admitted https must classify: {other:?}"),
        }
        // Structurally invalid https URLs refuse through the *shared*
        // port parse exactly as http does — no scheme-specific
        // refusal arm remains anywhere in dispatch.
        // lumen-allow-literal: URL parse/redaction test vector — data under test
        match SchemeRegistry::classify("https://") {
            Err(SchemeError::Malformed { .. }) => {}
            other => unreachable!("an empty authority must be malformed: {other:?}"),
        }
    }

    // -- typed refusals -------------------------------------------------

    /// Every scheme that is neither file nor http(s) is refused typed,
    /// naming the scheme, with the same stable code.
    #[test]
    fn unsupported_schemes_are_refused_typed() {
        for (source, scheme) in [
            ("ftp://example.com/media/a.wav", "ftp"),
            ("rtsp://cam.example.org/stream.sdp", "rtsp"),
            ("rtp://224.1.1.1:5004", "rtp"),
            ("udp://@239.255.12.42:1234", "udp"),
            ("data:audio/wav;base64,UklGRg==", "data"),
            ("gopher://example.org/0/media", "gopher"),
            ("mms://stream.example.org/live", "mms"),
            ("sftp://example.org/media/a.wav", "sftp"),
            ("https+invalid://example.org/x", "https+invalid"),
        ] {
            match SchemeRegistry::classify(source) {
                Err(SchemeError::Unsupported { scheme: got }) => {
                    assert_eq!(got, scheme, "{source:?} names its scheme");
                }
                other => unreachable!("{source:?} must refuse typed: {other:?}"),
            }
        }
    }

    /// Malformed references are typed with static detail; an http(s)
    /// URL the port refused keeps the port's error in the source
    /// chain.
    #[test]
    fn malformed_sources_are_typed_with_the_port_chain() {
        for source in [
            "",
            "file://",
            "file:///",
            "file:relative.wav",
            "file:///a/%zz.wav",
        ] {
            match SchemeRegistry::classify(source) {
                Err(SchemeError::Malformed { .. }) => {}
                other => unreachable!("{source:?} must refuse malformed: {other:?}"),
            }
        }
        // Spaces in an http URL: the port's violation, chained.
        // lumen-allow-literal: URL parse/redaction test vector — data under test
        let err = match SchemeRegistry::classify("http://ho st/media") {
            Err(err @ SchemeError::Malformed { .. }) => err,
            other => unreachable!("spacey URL must refuse: {other:?}"),
        };
        let chained = Error::source(&err).map(|s| s.to_string());
        assert_eq!(
            chained
                .as_deref()
                .map(|s| s.contains("streaming.net.protocol-violation")),
            Some(true),
            "the port's refusal rides the chain: {chained:?}"
        );
        // Non-UTF-8 percent-escapes: the recorded v1 URI-grammar limit.
        assert!(matches!(
            SchemeRegistry::classify("file:///a/%ff%fe.wav"),
            Err(SchemeError::Malformed { .. })
        ));
    }

    /// A `file:` URI naming a non-local authority refuses typed with
    /// the authority named (hosts are not credentials).
    #[test]
    fn remote_file_authority_refuses_typed() {
        match SchemeRegistry::classify("file://nas.example.org/media/a.wav") {
            Err(SchemeError::RemoteFileHost { host }) => {
                assert_eq!(host, "nas.example.org");
            }
            other => unreachable!("remote host must refuse: {other:?}"),
        }
    }

    // -- house-style pins ------------------------------------------------

    /// Every variant has a distinct, stable dotted code (the
    /// error.rs table convention).
    #[test]
    fn codes_are_stable_and_distinct() {
        let all = [
            SchemeError::Unsupported {
                scheme: "ftp".to_string(),
            },
            SchemeError::Malformed {
                detail: "test",
                source: None,
            },
            SchemeError::RemoteFileHost {
                host: "nas".to_string(),
            },
        ];
        let mut codes: Vec<&str> = all.iter().map(SchemeError::code).collect();
        let total = codes.len();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), total, "every variant has a distinct code");
        for err in &all {
            assert!(err.code().starts_with("streaming.scheme."));
        }
    }

    /// `Display` is structured, names the scheme, ends with the code,
    /// and never carries credentials from the source reference.
    #[test]
    fn display_is_structured_and_credential_free() {
        assert_eq!(
            SchemeError::Unsupported {
                scheme: "ftp".to_string()
            }
            .to_string(),
            "scheme dispatch failed: URL scheme \"ftp\" has no access strategy \
             (streaming.scheme.unsupported)"
        );
        // Userinfo-bearing sources: the http arm refuses at the port
        // (no credentials anywhere in the error), the unsupported arm
        // names only the scheme.
        let http = testkit::assert_err(
            SchemeRegistry::classify("http://user:secret@example.org/a"), // lumen-allow-literal: URL parse/redaction test vector — data under test
            "userinfo-bearing http URL refuses",
        );
        assert!(http.to_string().contains("scheme dispatch failed"));
        assert!(!http.to_string().contains("secret"));
        let rtsp = testkit::assert_err(
            SchemeRegistry::classify("rtsp://user:secret@example.org/a"),
            "userinfo-bearing rtsp URL refuses",
        );
        assert!(rtsp.to_string().contains("\"rtsp\""));
        assert!(!rtsp.to_string().contains("secret"));
        // Credential-shaped queries never surface either: the
        // redacted URI form is media-types' (SourceUri::redacted).
        let tokened = SourceUri::parse("https://cdn.example.org/x?token=secret").must(); // lumen-allow-literal: URL parse/redaction test vector — data under test
        assert_eq!(tokened.redacted(), "https://cdn.example.org/x?token=***"); // lumen-allow-literal: URL parse/redaction test vector — data under test
    }

    /// The engine bridge names the scheme (the
    /// `plugin_api::scheme_not_served` shape) and keeps the stage.
    #[test]
    fn engine_bridge_names_the_scheme() {
        let err = SchemeError::Unsupported {
            scheme: "rtsp".to_string(),
        }
        .into_engine_error();
        assert_eq!(err.stage, PipelineStage::Open);
        assert_eq!(err.context.code, "streaming.scheme.unsupported");
        match err.kind {
            ErrorKind::Unsupported { format: Some(info) } => {
                assert_eq!(info.name.as_deref(), Some("rtsp"));
                assert!(info.sniffed.is_empty());
            }
            other => unreachable!("wrong kind: {other:?}"),
        }
        let chained = SchemeError::Malformed {
            detail: "test",
            source: Some(Box::new(NetworkError::ProtocolViolation { detail: "test" })),
        }
        .into_engine_error();
        assert!(
            chained.source.is_some(),
            "the port chain survives the bridge"
        );
    }

    /// `SchemeError` is `Send + Sync` (fetch results cross threads,
    /// doc 01 §4.2 — the error.rs pin, mirrored).
    #[test]
    fn error_is_send_and_sync() {
        fn assert_bounds<T: Send + Sync>() {}
        assert_bounds::<SchemeError>();
        let e = SchemeError::Unsupported {
            scheme: "ftp".to_string(),
        };
        let joined = std::thread::spawn(move || e.code()).join();
        match joined {
            Ok(code) => assert_eq!(code, "streaming.scheme.unsupported"),
            Err(_) => unreachable!("error crossed the thread boundary"),
        }
    }

    // -- the access-plugin composition -----------------------------------

    /// The registry opens an HTTP source end-to-end over the mock
    /// testkit: classify → `open_http` → `SourceStream` reads, with
    /// the probe's `Range` on the server log.
    #[test]
    fn registry_opens_http_through_the_mock_testkit() {
        let body: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();
        let server = MockServer::spawn(MockConfig::body(body.clone())).must();
        let registry = SchemeRegistry::with_fetch(Arc::new(MockClient));
        let url = match SchemeRegistry::classify(&server.url("media/sine.wav")) {
            Ok(AccessKind::Http { url }) => url,
            other => unreachable!("mock http URL must classify: {other:?}"),
        };
        let config = RangedConfig {
            window_bytes: 4096,
            chunk_bytes: 1024,
            ..RangedConfig::default()
        };
        let mut resource = registry.open_http(&url, config).must();
        assert_eq!(resource.size(), Some(body.len() as u64));
        let mut sink = Vec::new();
        loop {
            let chunk = resource.read(777).must();
            if chunk.is_empty() {
                break;
            }
            sink.extend_from_slice(&chunk);
        }
        assert_eq!(sink, body, "the payload round-trips byte-exact");
        let requests = server.requests();
        assert!(
            requests
                .first()
                .and_then(|r| r.header("Range"))
                .is_some_and(|v| v == "bytes=0-1023"),
            "the probe's Range is on the wire: {:?}",
            requests.first().map(|r| r.headers.clone())
        );
    }

    /// An unsupported scheme is refused by `classify` — before any
    /// socket exists to talk to (the registry never opens for a
    /// scheme it does not serve; `classify` is pure and total).
    #[test]
    fn unsupported_scheme_refuses_before_any_socket() {
        // No server is spawned: reaching this assertion at all means
        // no I/O happened.
        match SchemeRegistry::classify("ftp://example.invalid/a.wav") {
            Err(err) => match &err {
                SchemeError::Unsupported { scheme } => {
                    assert_eq!(scheme, "ftp");
                    assert_eq!(
                        err.to_string(),
                        "scheme dispatch failed: URL scheme \"ftp\" has no access strategy \
                         (streaming.scheme.unsupported)"
                    );
                }
                other => unreachable!("wrong variant: {other:?}"),
            },
            Ok(kind) => unreachable!("ftp must refuse pre-socket: {kind:?}"),
        }
    }

    // -- the 6.D2 adoption point ------------------------------------------

    /// The reconnect layer composed into the registry's fetch path,
    /// end-to-end over the deterministic mock testkit: a source whose
    /// first two fetches are scripted retry weather (503, then 500)
    /// still opens and round-trips byte-exact — the ranged resource
    /// and every window fetch above the transport are unchanged, and
    /// the bounded backoff is paced by the injected clock (the sleeps
    /// advance virtual time instantly, so the schedule is asserted,
    /// never waited).
    #[test]
    fn retrying_registry_opens_through_scripted_weather() {
        let body: Vec<u8> = (0..2048u32).map(|i| (i % 249) as u8).collect();
        let weather = ScriptedStatus::new(vec![503, 500]);
        let server = MockServer::spawn(MockConfig {
            body: body.clone(),
            scripted_status: vec![("/media/sine.wav".to_owned(), weather.clone())],
            ..MockConfig::default()
        })
        .must();
        let clock = FakeClock::new();
        let policy = RetryPolicy {
            max_retries: 3,
            initial_ms: 25,
            max_ms: 100,
        };
        let registry = SchemeRegistry::with_retrying_fetch(
            Arc::new(MockClient),
            policy,
            Arc::new(clock.clone()),
        )
        .must();
        let url = match SchemeRegistry::classify(&server.url("media/sine.wav")) {
            Ok(AccessKind::Http { url }) => url,
            other => unreachable!("mock http URL must classify: {other:?}"),
        };
        let mut resource = registry.open_http(&url, RangedConfig::default()).must();
        assert_eq!(resource.size(), Some(body.len() as u64));
        let mut sink = Vec::new();
        loop {
            let chunk = resource.read(512).must();
            if chunk.is_empty() {
                break;
            }
            sink.extend_from_slice(&chunk);
        }
        assert_eq!(sink, body, "the payload round-trips through the weather");
        assert_eq!(weather.served(), 2, "both weathers answered, then recovery");
        // Full jitter can draw a zero (an instant retry, honestly a
        // no-op sleep): the boundedness is what is provable — at
        // most one sleep per weather, every sleep under the policy
        // ceiling.
        assert!(
            clock.sleeps().len() <= 2,
            "at most one backoff sleep per weather: {:?}",
            clock.sleeps()
        );
        assert!(
            clock.sleeps().iter().all(|ms| *ms <= 100),
            "every sleep under the policy ceiling: {:?}",
            clock.sleeps()
        );
        assert!(
            server.requests().len() >= 3,
            "probe + retries on the wire: {} requests",
            server.requests().len()
        );
    }

    /// The fail-fast boundary of the same composition: a zero-retry
    /// policy propagates the weather typed instead of absorbing it —
    /// the adoption point never *hides* a refusal, it only bounds the
    /// recovery attempts.
    #[test]
    fn zero_retry_policy_propagates_the_weather_typed() {
        let weather = ScriptedStatus::new(vec![503]);
        let server = MockServer::spawn(MockConfig {
            body: b"BODY".to_vec(),
            scripted_status: vec![("/media/sine.wav".to_owned(), weather.clone())],
            ..MockConfig::default()
        })
        .must();
        let clock = FakeClock::new();
        let registry = SchemeRegistry::with_retrying_fetch(
            Arc::new(MockClient),
            RetryPolicy {
                max_retries: 0,
                initial_ms: 25,
                max_ms: 100,
            },
            Arc::new(clock.clone()),
        )
        .must();
        let url = match SchemeRegistry::classify(&server.url("media/sine.wav")) {
            Ok(AccessKind::Http { url }) => url,
            other => unreachable!("mock http URL must classify: {other:?}"),
        };
        let err = match registry.open_http(&url, RangedConfig::default()) {
            Ok(_) => unreachable!("fail-fast must refuse through the weather"),
            Err(e) => e,
        };
        assert_eq!(err.context.code, "streaming.ranged.fetch-failed");
        assert_eq!(weather.served(), 1, "the single weather was answered");
        assert!(
            clock.sleeps().is_empty(),
            "no retries means no backoff sleeps"
        );
    }

    // -- the record cross-check ------------------------------------------

    /// The scheme-support record (`docs/formats/network-http.md`) and
    /// the registry agree: every `supported` row's probe classifies,
    /// every `refused` row's probe is the typed unsupported refusal
    /// naming that row's scheme. The table lives between the
    /// `scheme-registry` markers; the record is the decision ledger,
    /// this test is the enforcement.
    #[test]
    fn record_table_matches_the_registry() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/formats/network-http.md");
        let text = std::fs::read_to_string(&path).must();
        let table = text
            .split("<!-- scheme-registry-begin -->")
            .nth(1)
            .and_then(|after| after.split("<!-- scheme-registry-end -->").next())
            .must();
        let mut rows = 0;
        for line in table.lines().filter(|l| l.starts_with("| `")) {
            rows += 1;
            // | `scheme` | status | probe | … |
            let mut cells = line.trim_start_matches('|').split('|');
            let scheme = cells.next().must().trim().trim_matches('`').to_string();
            let status = cells.next().must().trim().to_string();
            let probe = cells.next().must().trim().to_string();
            match status.as_str() {
                "supported" => {
                    assert!(
                        SchemeRegistry::classify(&probe).is_ok(),
                        "record says {scheme:?} is supported; probe {probe:?} must classify"
                    );
                }
                "refused" => match SchemeRegistry::classify(&probe) {
                    Err(SchemeError::Unsupported { scheme: got }) => {
                        assert_eq!(got, scheme, "the refusal names the record's scheme");
                    }
                    other => unreachable!(
                        "record says {scheme:?} is refused; probe {probe:?}: {other:?}"
                    ),
                },
                other => unreachable!("unknown record status {other:?} on row {scheme:?}"),
            }
        }
        assert!(rows >= 12, "the full scheme table is present ({rows} rows)");
    }

    // -- +FB fuzz-lite ----------------------------------------------------

    /// Deterministic mutation sweep over scheme-shaped seeds: bit
    /// flips, truncations, and splices. Absence of panic is the
    /// contract (the adts fuzz-lite convention); the seeds are the
    /// registry's whole grammar surface.
    #[test]
    fn fuzz_lite_mutations_never_panic() {
        let seeds = [
            "file:///media/a%20b.wav",
            "file://localhost/x",
            "file://nas.example.org/y",
            "http://user:pw@example.com:8080/a?x=1", // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
            "https://cdn.example.org/seg/001.ts", // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
            "ftp://example.com/f",
            "rtsp://cam.example.org:554/stream.sdp",
            "data:audio/wav;base64,UklGRg==",
            "c:/Users/alice/media/a.avi", // lumen-allow-literal: path/URI test vector — data under test, not a compiled-in path
            "/media/plain/path.wav",
            "weird.name.tar:suffix",
            "",
        ];
        let mut passes = 0usize;
        for seed in seeds {
            let bytes = seed.as_bytes();
            fuzz_scheme_classify(bytes);
            passes += 1;
            // Bit-flip family.
            for i in 0..bytes.len() {
                let mut mutated = bytes.to_vec();
                mutated[i] ^= 0x20;
                fuzz_scheme_classify(&mutated);
                passes += 1;
            }
            // Truncation family.
            for end in 0..bytes.len() {
                fuzz_scheme_classify(&bytes[..end]);
                passes += 1;
            }
            // Splice family (seed head + seed tail, every split).
            for split in 0..bytes.len() {
                let mut mutated = bytes[..split].to_vec();
                mutated.extend_from_slice(&bytes[bytes.len() - split.min(bytes.len())..]);
                fuzz_scheme_classify(&mutated);
                passes += 1;
            }
        }
        assert!(passes > 300, "the sweep actually ran ({passes} passes)");
    }
}
