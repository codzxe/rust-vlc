//! The typed fetch seam (card 7.H4; doc 09 §6.1) — the browser twin
//! of `crates/streaming`'s `HttpFetch` port (card 6.A1a), shaped for
//! what the browser `fetch()` actually answers.
//!
//! # Shape (what differs from the native port, and why)
//!
//! - **Redirects are the transport's business.** `fetch()` follows
//!   redirect chains itself; the caller observes `redirected` +
//!   `final_url` (the [`FetchResponse`] fields). The native port's
//!   `RedirectPolicy` budgets have no browser analog — the browser's
//!   own budget applies (20 on Chromium-class engines
//!   `[VERIFY per-browser]`). A 3xx therefore never surfaces here
//!   from a real transport; the mock mirrors that (it follows its
//!   scripted chains), and a surfaced 3xx is a typed protocol
//!   violation.
//! - **Whole bodies, bounded.** Like the 6.A1a port this is a
//!   whole-body contract under [`FetchRequest::response_limit`] (the
//!   ReadableStream incremental lane is a later card's follow-up —
//!   see doc 09 §6.1). The JS glue reads `response.arrayBuffer()`
//!   and enforces the limit before staging.
//! - **A correlation id.** Requests ride the Worker MessagePort
//!   round trip (the glue serves them asynchronously), so every
//!   [`FetchRequest`] carries the id its [`FetchResponse`] must
//!   echo — the same discipline the command layer's `requestId`
//!   uses.
//! - **URL validation is minimal.** The browser's own URL parser is
//!   authoritative; [`validate_http_url`] only rejects early what
//!   would certainly fail (wrong scheme, control bytes, empty rest)
//!   so the typed refusal lands at `session.open` time, not after a
//!   round trip.
//!
//! [`FetchPort`] itself is the test seam: the mock double implements
//! it; production rides the port-contract messages. One vocabulary.

use media_types::EngineError;
use media_types::error::ErrorContext;
use media_types::ids::SourceUri;
use serde::{Deserialize, Serialize};

/// Default window/staging budget for the web lane: **8 MiB**. This is
/// the web-tuned mirror of streaming's 64 MiB `DEFAULT_WINDOW_BYTES`
/// (which mirrors `product.toml`'s `limits.cache_network_mib = 64`):
/// wasm32 memory ceilings on low-memory devices (doc 09 §2.4) make
/// the desktop default the wrong number on this host. The budget caps
/// lazily staged windows — it is never allocated up front.
pub const DEFAULT_WINDOW_BYTES: usize = 8 * 1024 * 1024;

/// Default read-ahead / request granularity: 256 KiB per `Range:`
/// request — the same CDN-friendly knob streaming's ranged resource
/// uses (card 6.A2a), kept identical so the two lanes behave alike.
pub const DEFAULT_CHUNK_BYTES: usize = 256 * 1024;

/// The stage tag every fetch-lane error carries (sources fail while
/// being opened or read — the input-layer convention, doc 03 §4.1(d);
/// the mirror of `ranged.rs`'s `STAGE`).
const STAGE: media_types::PipelineStage = media_types::PipelineStage::Open;

/// Mint a stage-`Open` engine error with a stable dotted code (the
/// input constructor pattern; the taxonomy stays media-types').
pub(crate) fn mint(kind: media_types::ErrorKind, code: &'static str) -> EngineError {
    EngineError::new(STAGE, kind, ErrorContext::new(code))
}

/// One byte-range request (RFC 9110 §14) — the mirror of streaming's
/// `ByteRange::Bounded`, end-exclusive like Rust's `a..b`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RangeRequest {
    /// First byte offset requested.
    pub start: u64,
    /// One past the last byte offset requested.
    pub end_exclusive: u64,
}

impl RangeRequest {
    /// The `Range:` header value (inclusive wire form).
    #[must_use]
    pub fn to_header_value(self) -> String {
        format!(
            "bytes={}-{}",
            self.start,
            self.end_exclusive.saturating_sub(1)
        )
    }
}

/// One fetch request minted by the engine side (surfaced upstream for
/// the Worker's JS glue to serve — see the module docs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchRequest {
    /// The correlation id the [`FetchResponse`] must echo.
    pub id: u64,
    /// The absolute `http`/`https` URL.
    pub url: String,
    /// The optional byte range (`Range:` header).
    pub range: Option<RangeRequest>,
    /// Per-response memory budget (MP §11): a body advertising or
    /// delivering more than this fails
    /// [`FetchError::ResponseTooLarge`] instead of allocating.
    pub response_limit: u64,
}

impl FetchRequest {
    /// A request for `url` with the default response budget.
    #[must_use]
    pub fn new(id: u64, url: impl Into<String>) -> FetchRequest {
        FetchRequest {
            id,
            url: url.into(),
            range: None,
            response_limit: DEFAULT_CHUNK_BYTES as u64,
        }
    }

    /// Set the byte range (builder style).
    #[must_use]
    pub fn with_range(mut self, range: RangeRequest) -> FetchRequest {
        self.range = Some(range);
        self
    }

    /// The `Range:` header this request carries, if any.
    #[must_use]
    pub fn range_header(&self) -> Option<String> {
        self.range.map(RangeRequest::to_header_value)
    }
}

/// One fetch response: what the browser `fetch()` resolves to —
/// status, headers, whole body, and the redirect observables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchResponse {
    /// The correlation id (echoes [`FetchRequest::id`]).
    pub id: u64,
    /// The status code (200, 206, 416, …).
    pub status: u16,
    /// Response headers as `(name, value)` pairs, names as sent (case
    /// preserved).
    pub headers: Vec<(String, String)>,
    /// The body bytes (whole-body contract).
    pub body: Vec<u8>,
    /// The URL after redirect following (`response.url`).
    pub final_url: String,
    /// Whether the transport followed at least one redirect
    /// (`response.redirected`).
    pub redirected: bool,
}

impl FetchResponse {
    /// First value of a header, ASCII case-insensitive.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// The typed failure vocabulary of the fetch lane (the mirror of
/// streaming's `NetworkError`, reduced to what this lane can see).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    /// A non-2xx/3xx-final status the lane does not interpret
    /// (404, 500, …). `416` is NOT here — the probe honors it as the
    /// empty-resource shape (see [`crate::io::seekability`]).
    HttpStatus {
        /// The status as answered.
        status: u16,
    },
    /// The body exceeded the request's memory budget (MP §11) —
    /// refused before allocation.
    ResponseTooLarge {
        /// The budget that was exceeded.
        limit: u64,
        /// The size that exceeded it (advertised or delivered).
        actual: u64,
    },
    /// A malformed or self-inconsistent response (a `206` without a
    /// parseable `Content-Range`, a `Content-Range` disagreeing with
    /// the request, a lying `Content-Length`, a surfaced 3xx).
    ProtocolViolation {
        /// The stable sentence naming the breach.
        detail: String,
    },
    /// A body shorter than its promised span, not ending at the
    /// resource end (the wire-end shape is honest, a mid-resource
    /// short body is a truncation — never a silent EOF; the
    /// file-source contract, mirrored from `ranged.rs`'s
    /// `Corrupt::Truncated`).
    Truncated {
        /// The span the response promised.
        expected: u64,
        /// The bytes actually delivered.
        got: u64,
    },
    /// The transport refused the request before a response (network
    /// error, DNS, CORS, offline) — `fetch()` rejected.
    Network {
        /// The transport's own message (already redacted by the glue
        /// — it never carries credentials).
        detail: String,
    },
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::HttpStatus { status } => write!(f, "http status {status}"),
            FetchError::ResponseTooLarge { limit, actual } => {
                write!(f, "response body {actual} B exceeds the {limit} B budget")
            }
            FetchError::ProtocolViolation { detail } => write!(f, "protocol violation: {detail}"),
            FetchError::Truncated { expected, got } => {
                write!(f, "truncated body: {got} of the promised {expected} bytes")
            }
            FetchError::Network { detail } => write!(f, "network error: {detail}"),
        }
    }
}

impl FetchError {
    /// The engine-error form (stage `Open`, typed code) — the lane's
    /// refusals surface through the open path like any source error.
    /// The kind mapping mirrors streaming's: HTTP statuses map to the
    /// network-cause vocabulary (`ConnectionLost` — the same mapping
    /// `ranged.rs` applies to `NetworkError::HttpStatus`).
    #[must_use]
    pub fn as_engine_error(&self) -> EngineError {
        let kind = match self {
            FetchError::HttpStatus { .. } | FetchError::Network { .. } => {
                media_types::ErrorKind::Network {
                    cause: media_types::error::NetworkCause::ConnectionLost,
                }
            }
            FetchError::ResponseTooLarge { .. } => media_types::ErrorKind::ResourceLimit {
                limit: media_types::error::LimitKind::Memory,
                budget: DEFAULT_WINDOW_BYTES as u64,
                attempted: 0,
            },
            FetchError::ProtocolViolation { .. } => {
                media_types::ErrorKind::Unsupported { format: None }
            }
            FetchError::Truncated { expected, got } => media_types::ErrorKind::Corrupt {
                detail: media_types::error::CorruptDetail::Truncated {
                    expected: *expected,
                    found: *got,
                },
            },
        };
        mint(kind, "web.fetch.transport").with_source(std::io::Error::other(self.to_string()))
    }
}

/// The in-process fetch seam (the mirror of streaming's `HttpFetch`
/// port): the mock test double implements it, and the same types ride
/// the Worker port contract in production (see the module docs —
/// ONE vocabulary, both transports).
pub trait FetchPort {
    /// Issue `request` to completion (whole body under its budget).
    ///
    /// # Errors
    /// [`FetchError`] — typed, never a panic, never a silent
    /// truncation.
    fn fetch(&self, request: &FetchRequest) -> Result<FetchResponse, FetchError>;
}

/// Early typed validation of an `http`/`https` URL (the shapes that
/// would certainly fail): wrong scheme, control bytes, or an empty
/// remainder refuse here, at `session.open` time. The browser's URL
/// parser stays authoritative for everything else.
///
/// # Errors
/// [`EngineError`] (stage `Open`, `web.fetch.url`) — the typed
/// early refusal.
pub fn validate_http_url(url: &str) -> Result<SourceUri, EngineError> {
    let lower = url.to_ascii_lowercase();
    // lumen-allow-literal: wire-protocol scheme prefix — transport vocabulary, not a shipped endpoint
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(mint(
            media_types::ErrorKind::Unsupported { format: None },
            "web.fetch.url",
        )
        .with_source(std::io::Error::other(format!(
            "source scheme `{}` is not an http(s) URL",
            url.split(':').next().unwrap_or("")
        ))));
    }
    let rest = url.split_once("://").map_or("", |(_, rest)| rest);
    if rest.is_empty() || url.chars().any(|c| c.is_ascii_control() || c == ' ') {
        return Err(mint(
            media_types::ErrorKind::Unsupported { format: None },
            "web.fetch.url",
        )
        .with_source(std::io::Error::other(
            "the URL is empty, or contains spaces or control bytes",
        )));
    }
    SourceUri::parse(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::{Must, assert_err};

    /// The wire form of a range request is the inclusive RFC 9110
    /// shape (the same as streaming's `ByteRange::to_header_value`).
    #[test]
    fn range_header_is_inclusive_wire_form() {
        let request =
            // lumen-allow-literal: reserved example host — typed-validation test vector, never a shipped endpoint
            FetchRequest::new(7, "https://media.example/a.avi").with_range(RangeRequest {
                start: 0,
                end_exclusive: 256 * 1024,
            });
        assert_eq!(
            request.range_header().as_deref(),
            Some("bytes=0-262143"),
            "end-exclusive start, inclusive wire end"
        );
        assert_eq!(FetchRequest::new(1, "u").range_header(), None);
    }

    /// Early URL validation: http(s) passes, everything else refuses
    /// typed at open time (never after a round trip).
    #[test]
    fn url_validation_is_early_and_typed() {
        // lumen-allow-literal: reserved example host — typed-validation test vector, never a shipped endpoint
        validate_http_url("https://media.example/a.avi").must();
        // lumen-allow-literal: reserved example host — typed-validation test vector, never a shipped endpoint
        validate_http_url("http://media.example:8080/a.avi?x=1").must();
        // lumen-allow-literal: reserved example host — typed-validation test vector, never a shipped endpoint
        for bad in ["ftp://media.example/a.avi", "https://", "https://a b/x"] {
            let err = assert_err(validate_http_url(bad), bad);
            assert_eq!(err.stage, STAGE, "{bad}");
        }
        // The parsed URI keeps its scheme (the L0 grammar, not a
        // closed enum).
        // lumen-allow-literal: reserved example host — typed-validation test vector, never a shipped endpoint
        let uri = validate_http_url("http://media.example/a.avi").must();
        assert_eq!(uri.scheme().as_str(), "http");
    }

    /// The response's header lookup is case-insensitive (the same
    /// discipline as the native port's `HttpResponse::header`).
    #[test]
    fn response_header_lookup_is_case_insensitive() {
        let response = FetchResponse {
            id: 1,
            status: 206,
            headers: vec![("Content-Range".to_string(), "bytes 0-9/100".to_string())],
            body: vec![0; 10],
            // lumen-allow-literal: reserved example host — typed-validation test vector, never a shipped endpoint
            final_url: "https://media.example/a.avi".to_string(),
            redirected: false,
        };
        assert_eq!(response.header("content-range"), Some("bytes 0-9/100"));
        assert_eq!(response.header("CONTENT-RANGE"), Some("bytes 0-9/100"));
        assert_eq!(response.header("accept-ranges"), None);
    }

    /// The typed error vocabulary carries its facts (branchable, like
    /// the native `NetworkError`).
    #[test]
    fn fetch_errors_carry_their_facts() {
        assert_eq!(
            FetchError::HttpStatus { status: 404 }.to_string(),
            "http status 404"
        );
        let err = FetchError::ResponseTooLarge {
            limit: 8,
            actual: 9,
        }
        .as_engine_error();
        assert_eq!(err.stage, STAGE);
        assert_eq!(err.context.code.as_str(), "web.fetch.transport");
    }
}
