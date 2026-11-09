//! The sync HTTP fetch port (card 6.A1a).
//!
//! [`HttpFetch`] is the streaming layer's seam: everything network
//! (the ureq-based real client of 6.A1c, the ranged seekable resource
//! of 6.A2, HLS/DASH segment fetchers of 6.B/6.C) talks through this
//! trait, so transports stay swappable and testable against the mock
//! HTTP/1.1 testkit ([`crate::mockhttp`]).
//!
//! # Sync by architecture
//!
//! The engine core is threads + channels — no async runtime may appear
//! in engine crates (docs/01-architecture.md §4.2). This trait is
//! deliberately synchronous and blocking, with all deadlines expressed
//! as plain [`Duration`]s in the request.
//!
//! # Ownership
//!
//! Request and response types are plain and owned (no lifetimes):
//! they cross thread boundaries later (a fetch result is sent down a
//! channel to the input worker), so borrowing would buy nothing.

use std::time::Duration;

use crate::error::NetworkError;

/// The default per-response memory budget: 8 MiB (MP §12 — bounded
/// memory is a contract, not a suggestion).
pub const DEFAULT_RESPONSE_LIMIT: u64 = 8 * 1024 * 1024;

/// The default redirect budget (curl/ureq territory: 10).
pub const DEFAULT_MAX_REDIRECTS: u32 = 10;

/// HTTP methods the port carries today. Only GET exists for v0 — the
/// enum is closed on purpose so later cards (POST for 6.E auth,
/// HEAD for 6.A2 probing) grow it without touching call sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    /// `GET` — the only method the streaming layer issues for v0.
    Get,
}

impl HttpMethod {
    /// The method token as it appears on the request line.
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
        }
    }
}

/// A byte range for `Range:` requests (RFC 9110 §14).
///
/// End-exclusive in the bounded form, matching Rust's `a..b` shape;
/// the wire form (inclusive end) is produced by
/// [`ByteRange::to_header_value`]. Suffix ranges (`bytes=-500`) are
/// card 6.A2's addition, not guessed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteRange {
    /// `bytes=a-b` — `start` inclusive, `end_exclusive` exclusive.
    Bounded {
        /// First byte offset requested.
        start: u64,
        /// One past the last byte offset requested.
        end_exclusive: u64,
    },
    /// `bytes=a-` — from `start` to the end of the representation.
    From {
        /// First byte offset requested.
        start: u64,
    },
}

impl ByteRange {
    /// The `Range:` header value for this range (inclusive wire form).
    pub fn to_header_value(&self) -> String {
        match self {
            ByteRange::Bounded {
                start,
                end_exclusive,
            } => format!("bytes={start}-{}", end_exclusive.saturating_sub(1)),
            ByteRange::From { start } => format!("bytes={start}-"),
        }
    }
}

/// Connect/read/write deadlines for one fetch.
///
/// Three independent budgets: a connect deadline that bounds
/// `TcpStream::connect`, a read deadline that bounds each response
/// read, and a write deadline that bounds each request write.
/// `Hash` so the real client can pool ureq agents per deadline
/// config (6.A1c).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeoutConfig {
    /// Deadline for establishing the connection.
    pub connect: Duration,
    /// Deadline per response read.
    pub read: Duration,
    /// Deadline per request write.
    pub write: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        TimeoutConfig {
            connect: Duration::from_secs(5),
            read: Duration::from_secs(5),
            write: Duration::from_secs(5),
        }
    }
}

/// How the transport treats redirect responses (3xx with `Location`).
///
/// `Follow` chases the chain under [`RedirectPolicy`]; `Manual` hands
/// the 3xx response back to the caller — the HLS master-playlist and
/// auth flows (6.B1/6.E1) resolve `Location` themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RedirectMode {
    /// Follow redirect chains under the policy budget (the default).
    #[default]
    Follow,
    /// Do not follow: the 3xx response (with its `Location` header)
    /// surfaces to the caller as `Ok`.
    Manual,
}

/// Redirect-following policy.
///
/// `max_redirects` bounds the chain; `cross_host` records whether
/// cross-host hops are allowed (enforced by the real client, 6.A1c —
/// the SSRF review of 6.H1 owns the origin/scheme widening);
/// `mode` selects following vs manual surfacing. The mock testkit's
/// client honors the bound and the loop detector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RedirectPolicy {
    /// Maximum redirects to follow before
    /// [`NetworkError::TooManyRedirects`].
    pub max_redirects: u32,
    /// Whether redirects to a different host are permitted. When
    /// `false`, a cross-host hop fails with
    /// [`NetworkError::CrossHostRedirect`] instead of following.
    pub cross_host: bool,
    /// Whether redirect chains are followed or surfaced to the
    /// caller (6.B1 HLS / 6.E1 auth resolve `Location` themselves).
    pub mode: RedirectMode,
}

impl Default for RedirectPolicy {
    fn default() -> Self {
        RedirectPolicy {
            max_redirects: DEFAULT_MAX_REDIRECTS,
            cross_host: true,
            mode: RedirectMode::Follow,
        }
    }
}

/// A parsed absolute `http`/`https` URL — the port's typed validation
/// surface. v0 keeps what a fetch needs: scheme, host, port, and the
/// path+query. IPv6 literals, userinfo, and fragments are later cards'
/// scope and are rejected rather than mis-parsed (MP §8: no silent
/// failure).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    /// `"http"` or `"https"`, lowercased.
    pub scheme: String,
    /// The host (reg-name or IPv4 literal).
    pub host: String,
    /// The explicit or scheme-default port.
    pub port: u16,
    /// Path plus optional `?query`, always starting with `/`.
    pub path_and_query: String,
}

impl Url {
    /// Parse and validate an absolute `http`/`https` URL.
    ///
    /// # Errors
    /// [`NetworkError::UnsupportedScheme`] for any other scheme;
    /// [`NetworkError::ProtocolViolation`] for malformed structure
    /// (missing `://`, empty host, bad port, spaces, control bytes,
    /// IPv6 literals, userinfo).
    pub fn parse(text: &str) -> Result<Url, NetworkError> {
        Self::parse_options(text, false).map(|(url, _)| url)
    }

    /// Parse a URL, **accepting and stripping userinfo**
    /// (`http://user:pass@host/path`) — the auth layer's surface
    /// (card 6.E1). [`Url::parse`] keeps rejecting userinfo (v0
    /// behavior, pinned); this constructor returns the canonical
    /// credential-free URL plus the userinfo as a [`Userinfo`] pair
    /// when present, so [`Display`](Url::fmt) can never leak a
    /// password that arrived by URL.
    ///
    /// # Errors
    /// As [`Url::parse`], except userinfo is accepted instead of
    /// rejected.
    pub fn parse_with_userinfo(text: &str) -> Result<(Url, Option<Userinfo>), NetworkError> {
        Self::parse_options(text, true)
    }

    /// Shared parser: `accept_userinfo` selects between the v0
    /// strict surface and the auth layer's userinfo-accepting one.
    fn parse_options(text: &str, accept_userinfo: bool) -> Result<(Url, Option<Userinfo>), NetworkError> {
        if text.chars().any(|c| c.is_ascii_control() || c == ' ') {
            return Err(NetworkError::ProtocolViolation {
                detail: "URL contains spaces or control bytes",
            });
        }
        let (scheme, rest) = text
            .split_once("://")
            .ok_or(NetworkError::ProtocolViolation {
                detail: "URL is not absolute (missing '://')",
            })?;
        let scheme = scheme.to_ascii_lowercase();
        if scheme != "http" && scheme != "https" {
            return Err(NetworkError::UnsupportedScheme { scheme });
        }
        let (authority, path_and_query) = match rest.split_once('/') {
            Some((authority, tail)) => (authority, format!("/{tail}")),
            None => (rest, "/".to_string()),
        };
        let mut userinfo = None;
        let authority = if let Some((userinfo_part, host_part)) = authority.rsplit_once('@') {
            if !accept_userinfo {
                return Err(NetworkError::ProtocolViolation {
                    detail: "userinfo in URL is not supported in v0",
                });
            }
            if host_part.is_empty() {
                return Err(NetworkError::ProtocolViolation {
                    detail: "URL has an empty host",
                });
            }
            let (username, password) = match userinfo_part.split_once(':') {
                Some((user, pass)) => (user.to_string(), pass.to_string()),
                None => (userinfo_part.to_string(), String::new()),
            };
            if username.is_empty() {
                return Err(NetworkError::ProtocolViolation {
                    detail: "URL userinfo has an empty username",
                });
            }
            userinfo = Some(Userinfo { username, password });
            host_part
        } else {
            authority
        };
        if authority.contains('[') || authority.contains(']') {
            return Err(NetworkError::ProtocolViolation {
                detail: "IPv6 URL literals are not supported in v0",
            });
        }
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) => {
                let port = port
                    .parse::<u16>()
                    .map_err(|_| NetworkError::ProtocolViolation {
                        detail: "invalid port",
                    })?;
                (host, port)
            }
            None => (authority, if scheme == "https" { 443 } else { 80 }),
        };
        if host.is_empty() {
            return Err(NetworkError::ProtocolViolation {
                detail: "URL has an empty host",
            });
        }
        Ok((
            Url {
                scheme,
                host: host.to_string(),
                port,
                path_and_query,
            },
            userinfo,
        ))
    }

    /// `host:port` as sent in the `Host:` header.
    pub fn authority(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Resolve a `Location:` reference against this URL.
    ///
    /// Absolute URLs and root-relative references (`/path`) resolve;
    /// bare relative references and fragments are v0-unsupported and
    /// return a typed error instead of guessing a base.
    ///
    /// # Errors
    /// [`NetworkError::ProtocolViolation`] for unsupported reference
    /// shapes; [`Url::parse`]'s errors for absolute references.
    pub fn join(&self, location: &str) -> Result<Url, NetworkError> {
        if location.starts_with("http://") || location.starts_with("https://") {
            return Url::parse(location);
        }
        if location.starts_with('/') {
            if location.chars().any(|c| c.is_ascii_control() || c == ' ') {
                return Err(NetworkError::ProtocolViolation {
                    detail: "Location reference contains spaces or control bytes",
                });
            }
            let mut next = self.clone();
            next.path_and_query = location.to_string();
            return Ok(next);
        }
        Err(NetworkError::ProtocolViolation {
            detail: "relative Location references without a leading '/' are unsupported in v0",
        })
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}://{}{}",
            self.scheme,
            self.authority(),
            self.path_and_query
        )
    }
}

/// `user[:password]` userinfo extracted from a URL (card 6.E1,
/// [`Url::parse_with_userinfo`]).
///
/// Carries a secret: [`Debug`](std::fmt::Debug) is hand-written and
/// redacts the password — a userinfo logged by accident never leaks
/// the credential (MP §12).
#[derive(Clone, PartialEq, Eq)]
pub struct Userinfo {
    /// The username (never empty — the parser rejects that shape).
    pub username: String,
    /// The password (may be empty: `http://user@host/`).
    pub password: String,
}

impl std::fmt::Debug for Userinfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Userinfo")
            .field("username", &self.username)
            .field("password", &"[redacted]")
            .finish()
    }
}

/// One HTTP fetch request (card 6.A1a): plain, owned, sync.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    /// The method — `GET` for v0.
    pub method: HttpMethod,
    /// The absolute `http`/`https` URL as given by the caller; the
    /// transport validates it ([`Url::parse`]) before connecting.
    pub url: String,
    /// Extra headers, in order, as `(name, value)` pairs. The
    /// transport adds `Host`, `Connection`, and `Range` itself.
    pub headers: Vec<(String, String)>,
    /// The optional byte range for a `Range:` request.
    pub range: Option<ByteRange>,
    /// Connect/read/write deadlines.
    pub timeouts: TimeoutConfig,
    /// Redirect-following policy.
    pub redirects: RedirectPolicy,
    /// Per-response memory budget (MP §12); responses advertising more
    /// than this fail with [`NetworkError::ResponseTooLarge`] instead
    /// of allocating.
    pub response_limit: u64,
}

impl HttpRequest {
    /// A `GET` request for `url` with default deadlines, redirect
    /// policy, and the 8 MiB response budget.
    pub fn get(url: impl Into<String>) -> HttpRequest {
        HttpRequest {
            method: HttpMethod::Get,
            url: url.into(),
            headers: Vec::new(),
            range: None,
            timeouts: TimeoutConfig::default(),
            redirects: RedirectPolicy::default(),
            response_limit: DEFAULT_RESPONSE_LIMIT,
        }
    }

    /// Add a header (builder style).
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Set the byte range (builder style).
    pub fn with_range(mut self, range: ByteRange) -> Self {
        self.range = Some(range);
        self
    }

    /// Set all three deadlines (builder style).
    pub fn with_timeouts(mut self, timeouts: TimeoutConfig) -> Self {
        self.timeouts = timeouts;
        self
    }

    /// Set the redirect policy (builder style).
    pub fn with_redirects(mut self, redirects: RedirectPolicy) -> Self {
        self.redirects = redirects;
        self
    }

    /// Set only the redirect mode (builder style) — the budget and
    /// cross-host flag keep their current values.
    pub fn with_redirect_mode(mut self, mode: RedirectMode) -> Self {
        self.redirects.mode = mode;
        self
    }

    /// Set the per-response memory budget (builder style).
    pub fn with_response_limit(mut self, limit: u64) -> Self {
        self.response_limit = limit;
        self
    }
}

/// One HTTP response: status + reason, headers, body bytes. Plain and
/// owned — no lifetimes, it crosses thread boundaries later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    /// The status code (200, 206, 416, …).
    pub status: u16,
    /// The reason phrase as sent (may be empty; never parsed further).
    pub reason: String,
    /// Response headers in wire order, as `(name, value)` pairs with
    /// names as sent (case preserved).
    pub headers: Vec<(String, String)>,
    /// The response body.
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Whether the status is 2xx.
    pub fn is_success(&self) -> bool {
        (200..=299).contains(&self.status)
    }

    /// First value of a header, ASCII case-insensitive.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// The sync HTTP fetch port (card 6.A1a).
///
/// One method, object-safe, `Send + Sync`: an [`HttpFetch`] handle is
/// shared across worker threads and each `fetch` call blocks the
/// calling thread until the response body is fully received (or fails
/// typed). Streaming *incremental* bodies is 6.A2's concern — this
/// port returns whole bodies under the [`HttpRequest::response_limit`]
/// budget.
///
/// # Errors
/// Every transport failure is typed — see [`NetworkError`]. No panic,
/// no silent truncation, no unbounded allocation on hostile input.
pub trait HttpFetch: Send + Sync {
    /// Fetch `request` to completion.
    ///
    /// # Errors
    /// [`NetworkError`] — DNS, deadlines, TLS, redirects, framing,
    /// bounds, or I/O (the OS error preserved in the `source` chain).
    fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError>;

    /// Fetch with authentication challenges surfaced (card 6.E1): a
    /// `401` response is returned as `Ok` — `WWW-Authenticate`
    /// header intact — instead of the typed
    /// [`NetworkError::HttpStatus`] error, so the auth layer
    /// ([`crate::auth`]) can answer the challenge. Every other status
    /// keeps `fetch`'s classification.
    ///
    /// The default delegates to `fetch` (the classified shape):
    /// transports that do not override it simply never hand the auth
    /// layer a challenge body, and the auth layer degrades to its
    /// conservative basic-auth fallback — documented, never a panic.
    /// Wrapping order matters: compose `RetryFetch` **around**
    /// `AuthFetch`, not inside it, or challenges stop surfacing.
    ///
    /// # Errors
    /// Transport-level failures only (DNS, deadlines, TLS, framing,
    /// redirect budget); any complete response — 401 included — is
    /// `Ok`.
    fn fetch_with_challenge(
        &self,
        request: HttpRequest,
    ) -> Result<HttpResponse, NetworkError> {
        self.fetch(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    /// The port is object-safe and shareable — pinned here because the
    /// whole layer (mock client now, ureq client in 6.A1c) hangs off it.
    #[test]
    fn port_is_object_safe_send_sync() {
        fn object_safe(_: &dyn HttpFetch) {}
        fn shareable<T: HttpFetch>() {}

        // Object safety: the trait can be used as `dyn HttpFetch`.
        let _ = object_safe as fn(&dyn HttpFetch);
        // Send + Sync bounds: a handle is shareable across worker
        // threads (doc 01 §4.2); MockClient pins the bound for real.
        shareable::<crate::mockhttp::MockClient>();
    }

    /// Range wire forms: bounded becomes inclusive-end, open stays open.
    #[test]
    fn byte_range_wire_forms() {
        assert_eq!(
            ByteRange::Bounded {
                start: 100,
                end_exclusive: 200
            }
            .to_header_value(),
            "bytes=100-199"
        );
        assert_eq!(
            ByteRange::From { start: 4096 }.to_header_value(),
            "bytes=4096-"
        );
    }

    /// URL parsing: the accepted shapes and their defaults.
    #[test]
    fn url_parse_accepts_http_and_https() {
        let plain = Url::parse("http://example.com/media/song.flac?x=1").must();
        assert_eq!(plain.scheme, "http");
        assert_eq!(plain.host, "example.com");
        assert_eq!(plain.port, 80);
        assert_eq!(plain.path_and_query, "/media/song.flac?x=1");
        assert_eq!(plain.authority(), "example.com:80");
        assert_eq!(
            plain.to_string(),
            "http://example.com:80/media/song.flac?x=1"
        );

        let tls = Url::parse("https://cdn.example.org:8443/seg001.ts").must();
        assert_eq!(tls.port, 8443);
        assert_eq!(tls.path_and_query, "/seg001.ts");

        let bare = Url::parse("https://host").must();
        assert_eq!(bare.path_and_query, "/");
        assert_eq!(bare.port, 443);
    }

    /// URL parsing: every rejected shape is typed, never a guess
    /// (MP §8).
    #[test]
    fn url_parse_rejects_hostile_shapes_typed() {
        let scheme = Url::parse("ftp://example.com/file");
        assert!(matches!(
            scheme,
            Err(NetworkError::UnsupportedScheme { ref scheme }) if scheme == "ftp"
        ));
        for bad in [
            "example.com/file",
            "http:///path",
            "http://host:notaport/",
            "http://[::1]:8080/",
            "http://user:pw@host/",
            "http://ho st/",
            "http://host/\u{0}",
        ] {
            assert!(
                matches!(Url::parse(bad), Err(NetworkError::ProtocolViolation { .. })),
                "{bad:?} must be a typed protocol violation"
            );
        }
    }

    /// Location resolution: absolute and root-relative resolve, the
    /// v0-unsupported shapes are typed errors.
    #[test]
    fn url_join_resolves_redirect_targets() {
        let base = Url::parse("http://127.0.0.1:8080/playlist/entries.m3u8").must();
        let root_relative = base.join("/media/seg001.ts").must();
        assert_eq!(root_relative.path_and_query, "/media/seg001.ts");
        assert_eq!(root_relative.host, base.host);

        let absolute = base.join("https://other.example.net:9/x?y=2").must();
        assert_eq!(absolute.scheme, "https");
        assert_eq!(absolute.path_and_query, "/x?y=2");

        for bad_ref in ["seg001.ts", "#frag", ""] {
            assert!(
                matches!(
                    base.join(bad_ref),
                    Err(NetworkError::ProtocolViolation { .. })
                ),
                "{bad_ref:?} must be rejected typed"
            );
        }
    }

    /// Builder defaults: deadlines, redirect budget, response limit.
    #[test]
    fn request_defaults_are_bounded() {
        let req = HttpRequest::get("http://example.com/a");
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.timeouts, TimeoutConfig::default());
        assert_eq!(req.timeouts.read, Duration::from_secs(5));
        assert_eq!(req.redirects.max_redirects, DEFAULT_MAX_REDIRECTS);
        assert!(req.redirects.cross_host);
        assert_eq!(req.redirects.mode, RedirectMode::Follow);
        let manual =
            HttpRequest::get("http://example.com/a").with_redirect_mode(RedirectMode::Manual);
        assert_eq!(manual.redirects.mode, RedirectMode::Manual);
        assert_eq!(manual.redirects.max_redirects, DEFAULT_MAX_REDIRECTS);
        assert_eq!(req.response_limit, DEFAULT_RESPONSE_LIMIT);
        assert!(req.headers.is_empty());
        assert!(req.range.is_none());
    }
}
