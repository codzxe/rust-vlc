//! ureq adapter — the real HTTP(S) transport behind [`HttpFetch`]
//! (card 6.A1b: validate-then-admit spike; hardening is 6.A1c).
//!
//! [`UreqClient`] is a thin, sync, rustls-backed implementation of the
//! 6.A1a port. ureq 3.5.0 is the measured admission candidate
//! (`docs/dependencies/records/ureq-3.5.0.md`): sync by design (no
//! async runtime — doc 01 §4.2), rustls TLS with verification always
//! on, bounded redirects, whole-body reads under the port's
//! `response_limit` budget (MP §12).
//!
//! # Spike scope (honest limits, 6.A1c owns the hardening)
//!
//! * One `ureq::Agent` is built **per fetch** — no connection pooling
//!   yet (ureq pools per-agent; per-request agents re-connect every
//!   time, which the ranged resource of 6.A2 will feel and 6.A1c fixes
//!   by caching agents keyed by config).
//! * `TimeoutConfig::write` has no ureq 3.5 knob (measured: ureq
//!   exposes `timeout_connect`/`timeout_global`/`timeout_per_call`
//!   only). GET requests carry no body, so the write deadline is
//!   carried but not armed; wiring it (or upstream adding
//!   `timeout_write`) is 6.A1c scope.
//! * Redirect-loop *detection* is not observable from ureq (it reports
//!   only `Error::TooManyRedirects`); the port's
//!   [`NetworkError::RedirectLoop`] stays the mock client's vocabulary.
//!   `RedirectPolicy::cross_host` is likewise carried but not enforced
//!   here (6.A1c/6.H1 SSRF review).
//! * The response reason phrase is `http::StatusCode::canonical_reason`
//!   — ureq/`http` do not preserve non-canonical wire reasons. The
//!   port tolerates an empty reason; this is the recorded divergence.

use std::io::Read;

use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
use ureq::{Agent, Timeout};

use crate::error::NetworkError;
use crate::fetch::{HttpFetch, HttpRequest, HttpResponse, RedirectPolicy, TimeoutConfig, Url};

/// The one TLS configuration [`UreqClient`] ever builds.
///
/// Verification is **always on**: this function never touches ureq's
/// `disable_verification` escape hatch (measured present in ureq
/// 3.5.0's `TlsConfig` builder — see the admission record's TLS
/// section). Roots are Mozilla's compiled-in `webpki-roots` bundle
/// (deterministic across platforms; no OS store probing), provider is
/// rustls — never native-tls.
pub(crate) fn tls_config() -> TlsConfig {
    TlsConfig::builder()
        .provider(TlsProvider::Rustls)
        .root_certs(RootCerts::WebPki)
        .build()
}

/// Translate a port request's deadlines and redirect budget into a
/// ureq [`ureq::config::Config`].
///
/// Wiring (measured against ureq 3.5.0's builder surface):
/// `timeouts.connect` → `timeout_connect`, `timeouts.read` →
/// `timeout_per_call` (bounds one full exchange, reset per redirect
/// hop). `timeouts.write` has no ureq knob — see the module docs.
/// Redirects follow up to `redirects.max_redirects` hops and error on
/// excess (`max_redirects_will_error`), matching the port's
/// [`NetworkError::TooManyRedirects`] contract.
pub(crate) fn agent_config(
    timeouts: TimeoutConfig,
    redirects: RedirectPolicy,
) -> ureq::config::Config {
    ureq::config::Config::builder()
        .timeout_connect(Some(timeouts.connect))
        .timeout_per_call(Some(timeouts.read))
        .max_redirects(redirects.max_redirects)
        .max_redirects_will_error(true)
        .tls_config(tls_config())
        .build()
}

/// Map a socket-level [`std::io::Error`] from the body read into the
/// typed vocabulary.
fn map_io_error(source: std::io::Error) -> NetworkError {
    match source.kind() {
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => {
            // A read deadline fired mid-body (Linux surfaces socket
            // timeouts as EAGAIN/WouldBlock).
            NetworkError::ReadTimeout
        }
        std::io::ErrorKind::ConnectionRefused => NetworkError::ConnectRefused,
        _ => NetworkError::Io { source },
    }
}

/// Map a [`ureq::Error`] into the typed [`NetworkError`] vocabulary.
fn map_ureq_error(error: ureq::Error, redirect_limit: u32) -> NetworkError {
    match error {
        ureq::Error::StatusCode(416) => NetworkError::RangeNotSatisfiable,
        ureq::Error::StatusCode(code) => NetworkError::http_status(code),
        // ureq reports no followed-count; the budget it spent is the
        // limit it was given (recorded divergence, see module docs).
        ureq::Error::TooManyRedirects => NetworkError::TooManyRedirects {
            limit: redirect_limit,
            followed: redirect_limit,
        },
        ureq::Error::HostNotFound => NetworkError::DnsFailure,
        ureq::Error::Timeout(Timeout::Connect) => NetworkError::ConnectTimeout,
        ureq::Error::Timeout(Timeout::Resolve) => NetworkError::DnsFailure,
        ureq::Error::Timeout(Timeout::PerRead) | ureq::Error::Timeout(Timeout::RecvResponse) => {
            NetworkError::ReadTimeout
        }
        ureq::Error::Timeout(Timeout::PerWrite)
        | ureq::Error::Timeout(Timeout::SendRequest)
        | ureq::Error::Timeout(Timeout::SendBody) => NetworkError::WriteTimeout,
        // Global/PerCall/Await100 and any future timer variant: the
        // response did not arrive inside the armed budget.
        ureq::Error::Timeout(_) => NetworkError::ReadTimeout,
        ureq::Error::Rustls(_) | ureq::Error::Tls(_) | ureq::Error::Pem(_) => {
            NetworkError::TlsFailure
        }
        ureq::Error::Protocol(_) => NetworkError::ProtocolViolation {
            detail: "peer sent malformed HTTP/1.x",
        },
        ureq::Error::Http(_) => NetworkError::ProtocolViolation {
            detail: "invalid HTTP header",
        },
        ureq::Error::BadUri(_) => NetworkError::ProtocolViolation {
            detail: "transport rejected the URL",
        },
        ureq::Error::LargeResponseHeader(actual, max) => NetworkError::ResponseTooLarge {
            limit: max as u64,
            actual: actual as u64,
        },
        // Unreachable for GET-without-body (ureq refuses to replay
        // bodies across 307/308); typed rather than guessed.
        ureq::Error::RedirectFailed => NetworkError::ProtocolViolation {
            detail: "redirect could not be followed",
        },
        // ureq's no-reason connection fallback; the kind is the honest
        // approximation (recorded in the admission record).
        ureq::Error::ConnectionFailed => NetworkError::Io {
            source: std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "ureq connection failed (no reason reported)",
            ),
        },
        ureq::Error::Io(source) => map_io_error(source),
        // `Error` is `#[non_exhaustive]`: future variants keep their
        // Display in the source chain instead of compiling away.
        other => NetworkError::Io {
            source: std::io::Error::other(other.to_string()),
        },
    }
}

/// The ureq-backed real HTTP(S) client (card 6.A1b).
///
/// Stateless and cheap to construct; implements the 6.A1a
/// [`HttpFetch`] port synchronously. See the module docs for the
/// spike-scope limits (pooling, write deadline, loop detection).
#[derive(Debug, Clone, Copy, Default)]
pub struct UreqClient;

impl UreqClient {
    /// Construct the client.
    pub fn new() -> UreqClient {
        UreqClient
    }
}

impl HttpFetch for UreqClient {
    fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
        // Validate the URL through the port's typed surface first —
        // scheme and structure violations never reach the transport.
        let url = Url::parse(&request.url)?;

        let agent = Agent::new_with_config(agent_config(request.timeouts, request.redirects));

        let mut builder = agent.get(url.to_string());
        for (name, value) in &request.headers {
            builder = builder.header(name.as_str(), value.as_str());
        }
        if let Some(range) = request.range {
            builder = builder.header("Range", range.to_header_value());
        }

        let response = builder
            .call()
            .map_err(|e| map_ureq_error(e, request.redirects.max_redirects))?;

        let status = response.status();
        let headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_string(),
                    String::from_utf8_lossy(value.as_bytes()).into_owned(),
                )
            })
            .collect();

        // Advertised size first: a hostile Content-Length never
        // allocates (MP §12).
        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<u64>().ok());
        if let Some(advertised) = content_length
            && advertised > request.response_limit
        {
            return Err(NetworkError::ResponseTooLarge {
                limit: request.response_limit,
                actual: advertised,
            });
        }

        // Whole-body read under the budget: read at most limit+1 bytes
        // so an over-budget body is detected, never allocated.
        let limit = request.response_limit;
        let mut body = Vec::new();
        let reader = response.into_body().into_reader();
        reader
            .take(limit.saturating_add(1))
            .read_to_end(&mut body)
            .map_err(map_io_error)?;
        let received = u64::try_from(body.len()).unwrap_or(u64::MAX);
        if received > limit {
            return Err(NetworkError::ResponseTooLarge {
                limit,
                actual: received,
            });
        }
        if let Some(advertised) = content_length
            && received < advertised
        {
            // Never a silent EOF (MP §8): the body ended early.
            return Err(NetworkError::BodyTruncated {
                expected: advertised,
                received,
            });
        }

        Ok(HttpResponse {
            status: status.as_u16(),
            reason: status.canonical_reason().unwrap_or_default().to_string(),
            headers,
            body,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::ByteRange;
    use crate::mockhttp::{MockConfig, MockServer};
    use std::time::Duration;
    use testkit::{Must, assert_err};

    fn spawn(config: MockConfig) -> MockServer {
        MockServer::spawn(config).must()
    }

    fn fast_timeouts(read: Duration) -> TimeoutConfig {
        TimeoutConfig {
            connect: Duration::from_secs(2),
            read,
            write: Duration::from_secs(2),
        }
    }

    /// 200 roundtrip over a real localhost socket: status, body,
    /// and a parsed Content-Length.
    #[test]
    fn ok_200_roundtrip() {
        let server = spawn(MockConfig::body(b"lumen roundtrip".to_vec()));
        let response = UreqClient::new()
            .fetch(HttpRequest::get(server.url("/media/one.flac")))
            .must();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"lumen roundtrip");
        assert_eq!(response.header("Content-Length"), Some("15"));
        server.requests(); // server stays alive through the assertion
    }

    /// Range honored end-to-end: the wire form the server saw, the
    /// 206 status, the sliced body, and the Content-Range echo.
    #[test]
    fn range_206_partial_is_honored() {
        let mut config = MockConfig::body(b"0123456789".to_vec());
        config.honor_range = true;
        let server = spawn(config);
        let response = UreqClient::new()
            .fetch(
                HttpRequest::get(server.url("/media/ranged.bin")).with_range(ByteRange::Bounded {
                    start: 2,
                    end_exclusive: 6,
                }),
            )
            .must();
        assert_eq!(response.status, 206);
        assert_eq!(response.body, b"2345");
        assert_eq!(response.header("Content-Range"), Some("bytes 2-5/10"));
        let logged = server.requests();
        assert_eq!(logged.len(), 1);
        assert_eq!(logged[0].header("Range"), Some("bytes=2-5"));
    }

    /// 416 maps to the typed range rejection.
    #[test]
    fn status_416_maps_to_range_not_satisfiable() {
        let mut config = MockConfig::body(b"short".to_vec());
        config.honor_range = true;
        let server = spawn(config);
        let error = assert_err(
            UreqClient::new().fetch(HttpRequest::get(server.url("/x")).with_range(
                ByteRange::Bounded {
                    start: 100,
                    end_exclusive: 200,
                },
            )),
            "416 must be typed",
        );
        assert!(matches!(error, NetworkError::RangeNotSatisfiable));
        assert_eq!(error.code(), "streaming.net.range-not-satisfiable");
    }

    /// A 3-hop redirect chain lands on the final body, and the server
    /// log records every hop.
    #[test]
    fn redirect_chain_of_3_lands_on_final_body() {
        let mut config = MockConfig::body(b"after three hops".to_vec());
        config.redirects = 3;
        let server = spawn(config);
        let response = UreqClient::new()
            .fetch(HttpRequest::get(server.url("/start")))
            .must();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"after three hops");
        let logged = server.requests();
        let paths: Vec<&str> = logged.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(paths, vec!["/start", "/r1", "/r2", "/r3"]);
    }

    /// The redirect budget is enforced: 5-hop chain, budget 2 → the
    /// typed too-many-redirects error carrying the configured limit.
    #[test]
    fn too_many_redirects_when_bound_exceeded() {
        let mut config = MockConfig::body(b"never reached".to_vec());
        config.redirects = 5;
        let server = spawn(config);
        let request = HttpRequest::get(server.url("/start")).with_redirects(RedirectPolicy {
            max_redirects: 2,
            cross_host: true,
        });
        let error = assert_err(
            UreqClient::new().fetch(request),
            "redirect budget must be enforced",
        );
        match error {
            NetworkError::TooManyRedirects { limit, followed } => {
                assert_eq!(limit, 2);
                assert_eq!(followed, 2);
            }
            other => unreachable!("wrong variant: {other:?}"),
        }
        assert_eq!(error.code(), "streaming.net.too-many-redirects");
    }

    /// Read deadline: a 400 ms delayed response against a 150 ms read
    /// budget fails typed (short durations, real socket).
    #[test]
    fn read_timeout_on_delayed_response() {
        let mut config = MockConfig::body(b"late".to_vec());
        config.delay = Some(Duration::from_millis(400));
        let server = spawn(config);
        let request = HttpRequest::get(server.url("/slow"))
            .with_timeouts(fast_timeouts(Duration::from_millis(150)));
        let error = assert_err(
            UreqClient::new().fetch(request),
            "delayed response must hit the read deadline",
        );
        assert!(
            matches!(error, NetworkError::ReadTimeout),
            "expected ReadTimeout, got {error:?}"
        );
        assert!(error.is_timeout());
    }

    /// Garbage first line → typed framing violation, never a panic.
    #[test]
    fn garbage_first_line_maps_to_protocol_violation() {
        let mut config = MockConfig::body(Vec::new());
        config.garbage_first_line = true;
        let server = spawn(config);
        let error = assert_err(
            UreqClient::new().fetch(HttpRequest::get(server.url("/garbage"))),
            "garbage framing must be typed",
        );
        assert!(matches!(error, NetworkError::ProtocolViolation { .. }));
        assert_eq!(error.code(), "streaming.net.protocol-violation");
    }

    /// The memory budget is enforced against the advertised
    /// Content-Length before any allocation.
    #[test]
    fn response_over_budget_is_rejected() {
        let server = spawn(MockConfig::body(vec![7u8; 100]));
        let request = HttpRequest::get(server.url("/big")).with_response_limit(10);
        match assert_err(
            UreqClient::new().fetch(request),
            "over-budget response must be rejected",
        ) {
            NetworkError::ResponseTooLarge { limit, actual } => {
                assert_eq!(limit, 10);
                assert_eq!(actual, 100);
            }
            other => unreachable!("wrong variant: {other:?}"),
        }
    }

    /// The port's URL validation runs before the transport: non-HTTP
    /// schemes are typed rejections.
    #[test]
    fn unsupported_scheme_is_typed() {
        let error = assert_err(
            UreqClient::new().fetch(HttpRequest::get("ftp://example.com/file")),
            "non-HTTP schemes must be typed",
        );
        assert!(matches!(
            error,
            NetworkError::UnsupportedScheme { ref scheme } if scheme == "ftp"
        ));
    }

    /// TLS verification is ALWAYS ON: the single TlsConfig construction
    /// site never flips ureq's `disable_verification` hatch (measured
    /// present upstream in ureq 3.5.0 — this pins our side shut), the
    /// provider is rustls, and the redirect budget is what we passed.
    #[test]
    fn tls_verification_is_always_on() {
        let tls = tls_config();
        assert!(
            !tls.disable_verification(),
            "the only TLS config path must keep verification on"
        );
        let config = agent_config(
            TimeoutConfig::default(),
            RedirectPolicy {
                max_redirects: 3,
                cross_host: true,
            },
        );
        assert_eq!(config.max_redirects(), 3);
        assert!(
            config.max_redirects_will_error(),
            "excess redirects must error, not silently return the 30x"
        );
    }

    /// LIVE-network test — **ignored by default** (a down network must
    /// not fail the suite; it must skip loudly when run deliberately).
    /// Run once via:
    ///
    /// ```text
    /// cargo test -p streaming live_ -- --ignored --nocapture
    /// ```
    ///
    /// Measures the admission's real-TLS claim: rustls + webpki-roots
    /// against a public HTTPS endpoint, expecting 200.
    #[test]
    #[ignore = "live network required: run with --ignored"]
    fn live_example_com_returns_200() {
        let response = UreqClient::new()
            .fetch(HttpRequest::get("https://example.com/"))
            .must();
        assert_eq!(response.status, 200);
        assert!(!response.body.is_empty());
        // Measured live 2026-10-30: example.com answers
        // `Content-Type: text/html; charset=utf-8` (lowercase utf-8) —
        // compare case-insensitively, the port preserves wire case.
        let content_type = response
            .header("Content-Type")
            .unwrap_or_default()
            .to_ascii_lowercase();
        assert_eq!(content_type, "text/html; charset=utf-8");
    }
}
