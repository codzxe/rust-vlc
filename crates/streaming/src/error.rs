//! Typed network error vocabulary for the streaming layer (card 6.A1a).
//!
//! House style follows `input`'s error module (card 2.A1): a typed,
//! thiserror-free enum with stable dotted context codes, structured
//! `Display`, and a [`std::error::Error`] impl that keeps the OS error
//! in the `source` chain. This vocabulary is *transport-level* — it
//! says why an HTTP fetch failed (DNS, deadlines, TLS, redirects,
//! framing, bounds). The bridge into the engine taxonomy
//! (`media_types`' `NetworkCause` / `EngineError`, doc 01 §3.4) is card
//! 6.A1c's job: the real client maps each variant there when it mints
//! pipeline-stage errors.
//!
//! # No secrets
//!
//! Variants never carry URLs, header values, or body bytes: `Display`
//! stays structured data and redaction stays the URI layer's job
//! (MP §12), exactly like `input`'s vocabulary.

use std::error::Error;
use std::fmt;
use std::io;

/// Typed transport-level failure of an HTTP fetch (card 6.A1a).
///
/// Every variant has a stable dotted code ([`NetworkError::code`]) so
/// logs and tests can match on vocabulary, not on `Display` text.
#[derive(Debug)]
pub enum NetworkError {
    /// Name resolution failed (`dns-failure`).
    DnsFailure,
    /// The TCP connect did not complete within the connect deadline.
    ConnectTimeout,
    /// The peer refused the connection (nothing listening).
    ConnectRefused,
    /// A response read exceeded the read deadline.
    ReadTimeout,
    /// A request write exceeded the write deadline.
    WriteTimeout,
    /// TLS handshake or certificate verification failed (never retried
    /// on certificate mismatch — that would be a downgrade).
    TlsFailure,
    /// The redirect budget was spent before a final response
    /// (`too-many-redirects`).
    TooManyRedirects {
        /// The configured maximum redirects.
        limit: u32,
        /// How many were followed when the budget ran out.
        followed: u32,
    },
    /// A redirect chain revisited a URL (`redirect-loop`).
    RedirectLoop {
        /// Requests issued before the revisit was detected.
        hops: u32,
    },
    /// A non-success HTTP status surfaced as an error (`http-status`).
    HttpStatus {
        /// The status code (e.g. 404).
        code: u16,
        /// `code` is in 400..=499.
        is_client_error: bool,
        /// `code` is in 500..=599.
        is_server_error: bool,
    },
    /// The server rejected the requested byte range — HTTP 416
    /// (`range-not-satisfiable`).
    RangeNotSatisfiable,
    /// The body ended before the advertised `Content-Length`
    /// (`body-truncated`): never a silent EOF, never a panic.
    BodyTruncated {
        /// The advertised length.
        expected: u64,
        /// Bytes actually received before the stream ended.
        received: u64,
    },
    /// A 206 response carried a `Content-Range` we could not parse
    /// (`invalid-content-range`).
    InvalidContentRange,
    /// The URL scheme is not one this layer fetches
    /// (`unsupported-scheme`).
    UnsupportedScheme {
        /// The rejected scheme, lowercased (`"ftp"` and friends).
        scheme: String,
    },
    /// The response would exceed the response memory budget (MP §12)
    /// (`response-too-large`).
    ResponseTooLarge {
        /// The configured budget in bytes.
        limit: u64,
        /// The advertised or observed size in bytes.
        actual: u64,
    },
    /// The peer spoke something that is not HTTP/1.x framing — a
    /// malformed status line, header, or `Location` reference.
    ProtocolViolation {
        /// Structured, static detail (never caller data).
        detail: &'static str,
    },
    /// Any other socket-level I/O failure (`io-failure`); the OS error
    /// rides the `source` chain untouched.
    Io {
        /// The underlying error, preserved.
        source: io::Error,
    },
}

impl NetworkError {
    /// The stable dotted context code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            NetworkError::DnsFailure => "streaming.net.dns-failure",
            NetworkError::ConnectTimeout => "streaming.net.connect-timeout",
            NetworkError::ConnectRefused => "streaming.net.connect-refused",
            NetworkError::ReadTimeout => "streaming.net.read-timeout",
            NetworkError::WriteTimeout => "streaming.net.write-timeout",
            NetworkError::TlsFailure => "streaming.net.tls-failure",
            NetworkError::TooManyRedirects { .. } => "streaming.net.too-many-redirects",
            NetworkError::RedirectLoop { .. } => "streaming.net.redirect-loop",
            NetworkError::HttpStatus { .. } => "streaming.net.http-status",
            NetworkError::RangeNotSatisfiable => "streaming.net.range-not-satisfiable",
            NetworkError::BodyTruncated { .. } => "streaming.net.body-truncated",
            NetworkError::InvalidContentRange => "streaming.net.invalid-content-range",
            NetworkError::UnsupportedScheme { .. } => "streaming.net.unsupported-scheme",
            NetworkError::ResponseTooLarge { .. } => "streaming.net.response-too-large",
            NetworkError::ProtocolViolation { .. } => "streaming.net.protocol-violation",
            NetworkError::Io { .. } => "streaming.net.io-failure",
        }
    }

    /// Mint an [`NetworkError::HttpStatus`] with the class flags
    /// computed from the code.
    pub fn http_status(code: u16) -> NetworkError {
        NetworkError::HttpStatus {
            code,
            is_client_error: (400..=499).contains(&code),
            is_server_error: (500..=599).contains(&code),
        }
    }

    /// Whether this failure is any deadline expiry (connect/read/write).
    pub fn is_timeout(&self) -> bool {
        matches!(
            self,
            NetworkError::ConnectTimeout | NetworkError::ReadTimeout | NetworkError::WriteTimeout
        )
    }

    /// The HTTP status code when this error carries one (416 folds into
    /// [`NetworkError::RangeNotSatisfiable`], so only
    /// [`NetworkError::HttpStatus`] reports here).
    pub fn status_code(&self) -> Option<u16> {
        match self {
            NetworkError::HttpStatus { code, .. } => Some(*code),
            _ => None,
        }
    }
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            NetworkError::DnsFailure => "DNS resolution failed".to_string(),
            NetworkError::ConnectTimeout => "connection attempt timed out".to_string(),
            NetworkError::ConnectRefused => "connection refused".to_string(),
            NetworkError::ReadTimeout => "response read timed out".to_string(),
            NetworkError::WriteTimeout => "request write timed out".to_string(),
            NetworkError::TlsFailure => "TLS handshake or verification failed".to_string(),
            NetworkError::TooManyRedirects { limit, followed } => {
                format!("redirect budget exhausted (limit {limit}, followed {followed})")
            }
            NetworkError::RedirectLoop { hops } => {
                format!("redirect loop detected after {hops} requests")
            }
            NetworkError::HttpStatus {
                code,
                is_client_error,
                is_server_error,
            } => {
                let class = if *is_client_error {
                    "client error"
                } else if *is_server_error {
                    "server error"
                } else {
                    "unexpected status"
                };
                format!("HTTP status {code} ({class})")
            }
            NetworkError::RangeNotSatisfiable => {
                "server rejected the requested byte range (416)".to_string()
            }
            NetworkError::BodyTruncated { expected, received } => {
                format!("body ended early: expected {expected} bytes, received {received}")
            }
            NetworkError::InvalidContentRange => "malformed Content-Range header".to_string(),
            NetworkError::UnsupportedScheme { scheme } => {
                format!("unsupported URL scheme {scheme:?}")
            }
            NetworkError::ResponseTooLarge { limit, actual } => format!(
                "response exceeds the memory budget (limit {limit} bytes, advertised {actual})"
            ),
            NetworkError::ProtocolViolation { detail } => {
                format!("protocol violation: {detail}")
            }
            NetworkError::Io { .. } => "socket I/O failure".to_string(),
        };
        write!(f, "network fetch failed: {text} ({})", self.code())
    }
}

impl Error for NetworkError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            NetworkError::Io { source } => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    /// Every variant has a distinct, stable dotted code — the whole
    /// vocabulary, one table (mirrors `input`'s constructor table).
    #[test]
    fn codes_are_stable_and_distinct() {
        let all = [
            NetworkError::DnsFailure,
            NetworkError::ConnectTimeout,
            NetworkError::ConnectRefused,
            NetworkError::ReadTimeout,
            NetworkError::WriteTimeout,
            NetworkError::TlsFailure,
            NetworkError::TooManyRedirects {
                limit: 10,
                followed: 10,
            },
            NetworkError::RedirectLoop { hops: 4 },
            NetworkError::http_status(404),
            NetworkError::RangeNotSatisfiable,
            NetworkError::BodyTruncated {
                expected: 100,
                received: 40,
            },
            NetworkError::InvalidContentRange,
            NetworkError::UnsupportedScheme {
                scheme: "ftp".to_string(),
            },
            NetworkError::ResponseTooLarge {
                limit: 8,
                actual: 64,
            },
            NetworkError::ProtocolViolation {
                detail: "malformed status line",
            },
            NetworkError::Io {
                source: io::Error::new(io::ErrorKind::Unsupported, "os error 999"),
            },
        ];
        let mut codes: Vec<&str> = all.iter().map(NetworkError::code).collect();
        let total = codes.len();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), total, "every variant has a distinct code");
        for err in &all {
            assert!(err.code().starts_with("streaming.net."));
        }
    }

    /// The status constructor computes the class flags it claims.
    #[test]
    fn http_status_classifies_client_and_server() {
        for code in [400, 404, 416, 499] {
            let e = NetworkError::http_status(code);
            match e {
                NetworkError::HttpStatus {
                    code: c,
                    is_client_error,
                    is_server_error,
                } => {
                    assert_eq!(c, code);
                    assert!(is_client_error, "{code} is a client error");
                    assert!(!is_server_error);
                }
                other => unreachable!("wrong variant: {other:?}"),
            }
            assert_eq!(e.status_code(), Some(code));
        }
        let server = NetworkError::http_status(503);
        match server {
            NetworkError::HttpStatus {
                is_client_error,
                is_server_error,
                ..
            } => {
                assert!(!is_client_error);
                assert!(is_server_error, "503 is a server error");
            }
            other => unreachable!("wrong variant: {other:?}"),
        }
        let other = NetworkError::http_status(302);
        assert_eq!(other.status_code(), Some(302));
        assert!(!other.is_timeout());
        assert!(NetworkError::ReadTimeout.is_timeout());
        assert!(NetworkError::ConnectTimeout.is_timeout());
        assert!(NetworkError::WriteTimeout.is_timeout());
        assert!(!NetworkError::DnsFailure.is_timeout());
    }

    /// `Display` is structured, carries no caller data, and ends with
    /// the dotted code (MP §12; `input`'s display shape).
    #[test]
    fn display_is_structured_with_code_suffix() {
        let e = NetworkError::BodyTruncated {
            expected: 1024,
            received: 512,
        };
        assert_eq!(
            e.to_string(),
            "network fetch failed: body ended early: expected 1024 bytes, received 512 \
             (streaming.net.body-truncated)"
        );
        let scheme = NetworkError::UnsupportedScheme {
            scheme: "gopher".to_string(),
        };
        assert_eq!(
            scheme.to_string(),
            "network fetch failed: unsupported URL scheme \"gopher\" \
             (streaming.net.unsupported-scheme)"
        );
    }

    /// The OS error rides the `source` chain untouched.
    #[test]
    fn io_variant_keeps_the_source_chain() {
        let e = NetworkError::Io {
            source: io::Error::new(io::ErrorKind::ConnectionAborted, "ECONNABORTED"),
        };
        let chained = Error::source(&e).map(|s| s.to_string());
        assert_eq!(
            chained.map(|s| s.contains("ECONNABORTED")),
            Some(true),
            "the OS truth rides the chain"
        );
        assert_eq!(e.code(), "streaming.net.io-failure");
    }

    /// `NetworkError` is `Send + Sync` so it can cross thread boundaries
    /// (the engine moves fetch results over channels, doc 01 §4.2).
    #[test]
    fn error_is_send_and_sync() {
        fn assert_bounds<T: Send + Sync>() {}
        assert_bounds::<NetworkError>();
        let e: NetworkError = NetworkError::DnsFailure;
        let joined = std::thread::spawn(move || e.code()).join();
        match joined {
            Ok(code) => assert_eq!(code, "streaming.net.dns-failure"),
            Err(_) => unreachable!("error crossed the thread boundary"),
        }
    }
}
