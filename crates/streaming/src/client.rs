//! ureq adapter — the real HTTP(S) transport behind [`HttpFetch`]
//! (card 6.A1b admission; hardened by 6.A1c).
//!
//! [`UreqClient`] is a thin, sync, rustls-backed implementation of the
//! 6.A1a port. ureq 3.5.0 is the measured admission candidate
//! (`docs/dependencies/records/ureq-3.5.0.md`): sync by design (no
//! async runtime — doc 01 §4.2), rustls TLS with verification always
//! on, bounded redirects, whole-body reads under the port's
//! `response_limit` budget (MP §12).
//!
//! # Redirect following is the adapter's, not ureq's (6.A1c)
//!
//! The agent is configured with `max_redirects(0)` +
//! `http_status_as_error(false)` (measured in ureq 3.5.0: with a zero
//! redirect budget ureq never follows — `run.rs` errors on excess
//! redirects only when `max_redirects > 0 && max_redirects_will_error`
//! — and with status-as-error off, every status surfaces as a
//! response), so each 3xx lands **here** and this adapter follows it
//! hop by hop. That buys what ureq's follower cannot express:
//!
//! * [`RedirectPolicy::cross_host`] enforced — a cross-host hop under
//!   a same-host policy fails typed
//!   ([`NetworkError::CrossHostRedirect`]), never followed.
//! * loop detection over a bounded visited-set of canonical URLs
//!   ([`NetworkError::RedirectLoop`]), distinct from budget exhaustion.
//! * an **exact** followed count ([`NetworkError::TooManyRedirects`]):
//!   the adapter issued every hop, so it knows the number (ureq 3.5.0
//!   exposes no `redirects()` count accessor — measured; its
//!   `ResponseExt::get_redirect_history` needs `save_redirect_history`
//!   and is unused because the loop is ours).
//! * [`RedirectMode::Manual`] — the 3xx response (with its `Location`
//!   header) returns to the caller un-followed (6.B1 HLS master
//!   playlists, 6.E1 auth).
//!
//! `Authorization` headers are never re-sent on a followed hop — the
//! parity of ureq's `RedirectAuthHeaders::Never` default (measured);
//! 6.E1 owns any finer same-host policy.
//!
//! # Agents are pooled per deadline config (6.A1c)
//!
//! A process-wide cache keyed by [`TimeoutConfig`] hands out
//! `Arc<Agent>`s (ureq pools connections per agent), so repeat fetches
//! with the same deadlines reuse connections when the server allows
//! keep-alive. The mock testkit's dialect is `Connection: close`, so
//! the suite exercises the reconnect path; keep-alive reuse rides on
//! ureq's own pool.
//!
//! # Honest limits (measured, recorded)
//!
//! * `TimeoutConfig::write` has no ureq 3.5.0 knob — only
//!   `timeout_connect` / `timeout_global` / `timeout_per_call` exist
//!   (measured builder surface). The per-call budget — armed with
//!   `timeouts.read` — covers "DNS lookup, connection setup, sending
//!   the request, and receiving the response headers and body"
//!   (measured doc text), so a stalled request write cannot outlive
//!   it; a timeout ureq attributes to a write phase
//!   (`SendRequest`/`SendBody`/`PerWrite`) maps to
//!   [`NetworkError::WriteTimeout`]. A dedicated write deadline waits
//!   on an upstream knob.
//! * The response reason phrase is `http::StatusCode::canonical_reason`
//!   — ureq/`http` do not preserve non-canonical wire reasons. The
//!   port tolerates an empty reason; this is the recorded divergence.
//! * `Location` resolution is the port's typed `Url::join`: absolute
//!   and root-relative references only (v0). ureq's own follower
//!   resolves more shapes; the port's typed refusal is the contract.
//! * An https→http redirect is a typed refusal
//!   ([`NetworkError::CrossSchemeRedirect`]), same host or not — a
//!   TLS downgrade is the classic MITM vector (security review 6.H1
//!   F1, fixed wave 19). Upgrades (http→https) stay followable.
//! * 1xx final responses and non-`Location` 3xx (300, 304) surface as
//!   `Ok` for the caller to judge — mock-client parity.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::{Arc, Mutex, OnceLock};

use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
use ureq::{Agent, Timeout};

use crate::error::NetworkError;
use crate::fetch::{HttpFetch, HttpRequest, HttpResponse, RedirectMode, TimeoutConfig, Url};

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

/// Translate a port request's deadlines into a ureq
/// [`ureq::config::Config`].
///
/// Wiring (measured against ureq 3.5.0's builder surface):
/// `timeouts.connect` → `timeout_connect`, `timeouts.read` →
/// `timeout_per_call` (bounds one full exchange, reset per hop the
/// adapter follows). `timeouts.write` has no ureq knob — see the
/// module docs for the recorded decision. Redirects are **never**
/// followed by ureq here (`max_redirects(0)`, measured: a zero budget
/// returns the 3xx response instead of erroring), and statuses are
/// never converted to errors (`http_status_as_error(false)`) — the
/// adapter classifies both itself so the port's
/// [`RedirectPolicy`] is enforced hop by hop.
pub(crate) fn agent_config(timeouts: TimeoutConfig) -> ureq::config::Config {
    ureq::config::Config::builder()
        .timeout_connect(Some(timeouts.connect))
        .timeout_per_call(Some(timeouts.read))
        .max_redirects(0)
        .http_status_as_error(false)
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
///
/// Defense in depth: with `http_status_as_error(false)` and
/// `max_redirects(0)` (this adapter's configuration), the
/// `StatusCode` / `TooManyRedirects` / `RedirectFailed` arms are
/// unreachable — they stay mapped so a config regression degrades to
/// typed errors, never to the fallback.
#[allow(clippy::too_many_lines)]
fn map_ureq_error(error: ureq::Error, redirect_limit: u32) -> NetworkError {
    match error {
        ureq::Error::StatusCode(416) => NetworkError::RangeNotSatisfiable,
        ureq::Error::StatusCode(code) => NetworkError::http_status(code),
        // Unreachable while ureq never follows (the adapter is the
        // follower and reports the count it measured itself).
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
        ureq::Error::RedirectFailed => NetworkError::ProtocolViolation {
            detail: "redirect could not be followed",
        },
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

/// The redirect statuses this adapter follows — the mock testkit's
/// dialect, exactly (301/302/303/307/308; 300 and 304 are not
/// `Location`-carrying redirects and surface to the caller).
fn is_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

/// Whether a redirect hop would weaken the transport: the current
/// hop is `https` and the target is `http` (the parser lowercases
/// and only ever mints those two schemes, so this is the one
/// downgrade shape the port can express).
///
/// Security review 6.H1 F1 (wave 19 fix): a TLS downgrade is the
/// classic MITM vector, so it is refused **unconditionally** —
/// before and independent of the cross-host check, whatever the
/// host policy says, and with no `RedirectPolicy` opt-in (no
/// caller has a legitimate reason to weaken its own transport).
/// The reverse direction (http→https) is an upgrade and always
/// followable.
fn is_scheme_downgrade(url: &Url, next: &Url) -> bool {
    url.scheme == "https" && next.scheme == "http"
}

/// Single-hop status classification (mock-client parity): 2xx is
/// `Ok`, 416 is the typed range rejection, other 4xx/5xx carry the
/// class flags, and everything else (1xx, non-`Location` 3xx)
/// surfaces as-is for the caller to judge.
fn classify(response: HttpResponse) -> Result<HttpResponse, NetworkError> {
    if (200..=299).contains(&response.status) {
        return Ok(response);
    }
    if response.status == 416 {
        return Err(NetworkError::RangeNotSatisfiable);
    }
    if (400..=599).contains(&response.status) {
        return Err(NetworkError::http_status(response.status));
    }
    Ok(response)
}

/// Issue exactly one request (ureq never follows) and read the whole
/// body under the response budget.
///
/// `drop_authorization` implements ureq's `RedirectAuthHeaders::Never`
/// parity: a followed hop never re-sends `Authorization` (6.E1 owns
/// any finer same-host policy).
fn single_hop(
    agent: &Agent,
    url: &Url,
    request: &HttpRequest,
    drop_authorization: bool,
) -> Result<HttpResponse, NetworkError> {
    let mut builder = agent.get(url.to_string());
    for (name, value) in &request.headers {
        if drop_authorization && name.eq_ignore_ascii_case("authorization") {
            continue;
        }
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

/// The ureq-backed real HTTP(S) client (card 6.A1b; hardened 6.A1c).
///
/// Cheap to construct (a `Copy` handle over a process-wide agent
/// pool); implements the 6.A1a [`HttpFetch`] port synchronously. See
/// the module docs for the redirect architecture and the recorded
/// honest limits.
#[derive(Debug, Clone, Copy, Default)]
pub struct UreqClient;

impl UreqClient {
    /// Construct the client.
    pub fn new() -> UreqClient {
        UreqClient
    }

    /// The process-wide agent pool, keyed by deadline config (ureq
    /// pools connections per agent, so same-deadline fetches reuse
    /// sockets when the server allows keep-alive).
    fn shared_agent(timeouts: TimeoutConfig) -> Arc<Agent> {
        static CACHE: OnceLock<Mutex<HashMap<TimeoutConfig, Arc<Agent>>>> = OnceLock::new();
        let mut guard = CACHE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard
            .entry(timeouts)
            .or_insert_with(|| Arc::new(Agent::new_with_config(agent_config(timeouts))))
            .clone()
    }
}

impl HttpFetch for UreqClient {
    fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
        self.fetch_inner(request, false)
    }

    /// Challenge surfacing (card 6.E1): a final 401 is `Ok` with
    /// its `WWW-Authenticate` header intact, so the auth layer
    /// (`crate::auth`) can answer the challenge.
    fn fetch_with_challenge(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
        self.fetch_inner(request, true)
    }
}

impl UreqClient {
    /// The shared fetch loop; `surface_challenge` turns the final
    /// 401 into an `Ok` response (see `fetch_with_challenge`).
    fn fetch_inner(
        &self,
        request: HttpRequest,
        surface_challenge: bool,
    ) -> Result<HttpResponse, NetworkError> {
        // Validate the URL through the port's typed surface first —
        // scheme and structure violations never reach the transport.
        let mut url = Url::parse(&request.url)?;
        let agent = Self::shared_agent(request.timeouts);

        // Visited canonical URLs, bounded by max_redirects + 1 entries
        // (the loop exits at the budget first) — bounded-memory loop
        // detection (6.A1c).
        let mut visited: HashSet<String> = HashSet::new();
        visited.insert(url.to_string());
        let mut followed: u32 = 0;
        let mut drop_authorization = false;

        loop {
            let response = single_hop(&agent, &url, &request, drop_authorization)?;

            if !is_redirect(response.status) {
                if surface_challenge && response.status == 401 {
                    return Ok(response);
                }
                return classify(response);
            }

            // Manual mode: the 3xx (with its Location header) is the
            // answer — 6.B1 HLS / 6.E1 auth resolve it themselves.
            if request.redirects.mode == RedirectMode::Manual {
                return Ok(response);
            }

            let Some(location) = response.header("location").map(str::to_string) else {
                return Err(NetworkError::ProtocolViolation {
                    detail: "redirect status without a Location header",
                });
            };
            if followed >= request.redirects.max_redirects {
                // `followed` is exact: this adapter issued every hop.
                return Err(NetworkError::TooManyRedirects {
                    limit: request.redirects.max_redirects,
                    followed,
                });
            }
            let next = url.join(&location)?;

            // Scheme-downgrade refusal (security review 6.H1, F1 —
            // wave 19): an https hop never follows a redirect down
            // to http, same host or not. Checked before the
            // cross-host gate: transport safety outranks host
            // policy, and no policy combination may opt in.
            if is_scheme_downgrade(&url, &next) {
                return Err(NetworkError::CrossSchemeRedirect { hops: followed + 1 });
            }

            // Cross-host enforcement (RedirectPolicy::cross_host):
            // hosts compare case-insensitively; a same-host port
            // change is followed (scheme safety is settled above —
            // downgrades refused, upgrades allowed).
            let cross_host = !next.host.eq_ignore_ascii_case(&url.host);
            if cross_host && !request.redirects.cross_host {
                return Err(NetworkError::CrossHostRedirect { hops: followed + 1 });
            }

            // Loop detection: a canonical URL about to be revisited.
            let canonical = next.to_string();
            if visited.contains(&canonical) {
                return Err(NetworkError::RedirectLoop {
                    hops: visited.len() as u32,
                });
            }
            visited.insert(canonical);
            followed += 1;
            drop_authorization = true;
            url = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::{ByteRange, RedirectPolicy};
    use crate::mockhttp::{MockClient, MockConfig, MockServer};
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

    fn follow_policy(max_redirects: u32, cross_host: bool) -> RedirectPolicy {
        RedirectPolicy {
            max_redirects,
            cross_host,
            mode: RedirectMode::Follow,
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

    /// 4xx and 5xx map to the http-status kind with the class flags
    /// computed (is_client_error / is_server_error).
    #[test]
    fn status_class_flags_are_mapped() {
        for (code, client_error, server_error) in [(404, true, false), (503, false, true)] {
            let mut config = MockConfig::body(b"no such thing".to_vec());
            config.force_status = Some(code);
            let server = spawn(config);
            let error = assert_err(
                UreqClient::new().fetch(HttpRequest::get(server.url("/missing"))),
                "error statuses must be typed",
            );
            match error {
                NetworkError::HttpStatus {
                    code: c,
                    is_client_error,
                    is_server_error,
                } => {
                    assert_eq!(c, code);
                    assert_eq!(is_client_error, client_error);
                    assert_eq!(is_server_error, server_error);
                }
                other => unreachable!("wrong variant: {other:?}"),
            }
            assert_eq!(error.code(), "streaming.net.http-status");
            assert_eq!(error.status_code(), Some(code));
            assert!(!error.is_timeout());
        }
    }

    /// A 3-hop redirect chain lands on the final body, and the server
    /// log records every hop (the adapter — not ureq — followed them).
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

    /// The redirect budget is enforced with an exact followed count:
    /// 5-hop chain, budget 2 → the typed error carrying the two hops
    /// this adapter actually issued.
    #[test]
    fn too_many_redirects_when_bound_exceeded() {
        let mut config = MockConfig::body(b"never reached".to_vec());
        config.redirects = 5;
        let server = spawn(config);
        let request = HttpRequest::get(server.url("/start")).with_redirects(follow_policy(2, true));
        let error = assert_err(
            UreqClient::new().fetch(request),
            "redirect budget must be enforced",
        );
        match error {
            NetworkError::TooManyRedirects { limit, followed } => {
                assert_eq!(limit, 2);
                assert_eq!(followed, 2, "the count is exact, not approximated");
            }
            other => unreachable!("wrong variant: {other:?}"),
        }
        // Exactly the budget's worth of hops hit the wire.
        assert_eq!(server.requests().len(), 3);
        assert_eq!(error.code(), "streaming.net.too-many-redirects");
    }

    /// Cross-host refusal: a redirect from 127.0.0.1 to localhost (a
    /// different host string, same machine) is refused under a
    /// same-host policy, and the second server is never contacted.
    #[test]
    fn cross_host_redirect_refused_when_policy_forbids() {
        let target = spawn(MockConfig::body(b"across hosts".to_vec()));
        // Same socket, different host string — exactly the shape a
        // cross-host policy must refuse.
        let cross_host_url = target
            .url("/media/other.flac")
            .replace("127.0.0.1", "localhost");
        let mut origin = MockConfig::body(Vec::new());
        origin.absolute_redirect = Some(cross_host_url);
        let origin = spawn(origin);

        let request =
            HttpRequest::get(origin.url("/start")).with_redirects(follow_policy(5, false));
        let error = assert_err(
            UreqClient::new().fetch(request),
            "cross-host hop must be refused under a same-host policy",
        );
        match error {
            NetworkError::CrossHostRedirect { hops } => assert_eq!(hops, 1),
            other => unreachable!("wrong variant: {other:?}"),
        }
        assert_eq!(error.code(), "streaming.net.cross-host-redirect");
        assert_eq!(
            origin.requests().len(),
            1,
            "the origin answered exactly once"
        );
        assert!(
            target.requests().is_empty(),
            "the cross-host target must never be contacted"
        );
    }

    /// Cross-host allowed: the same topology follows to the target's
    /// body when the policy permits cross-host hops.
    #[test]
    fn cross_host_redirect_followed_when_policy_allows() {
        let target = spawn(MockConfig::body(b"across hosts".to_vec()));
        let cross_host_url = target
            .url("/media/other.flac")
            .replace("127.0.0.1", "localhost");
        let mut origin = MockConfig::body(Vec::new());
        origin.absolute_redirect = Some(cross_host_url);
        let origin = spawn(origin);

        let request = HttpRequest::get(origin.url("/start")).with_redirects(follow_policy(5, true));
        let response = UreqClient::new().fetch(request).must();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"across hosts");
        assert_eq!(origin.requests().len(), 1);
        assert_eq!(target.requests().len(), 1);
    }

    /// Security review 6.H1 F1 (wave 19): an https→http redirect is
    /// a typed refusal, never followed — same host or not, whatever
    /// the host policy says. A TLS downgrade is the classic MITM
    /// vector.
    ///
    /// The ureq adapter itself cannot be fronted with TLS in-tree
    /// (the mock testkit is a plaintext socket — the same honest
    /// limit `https_activation.rs` records), so the wire-level
    /// refusal is pinned on the testkit client, whose fetch loop is
    /// the documented parity copy of this adapter's; the adapter's
    /// own gate is pinned by `scheme_downgrade_gate_pins_transport_safety`.
    #[test]
    fn https_to_http_redirect_downgrade_is_refused_not_followed() {
        // The "downgraded" target: plaintext http, ready to serve.
        let target = spawn(MockConfig::body(b"downgraded bytes".to_vec()));
        let mut origin = MockConfig::body(Vec::new());
        origin.absolute_redirect = Some(target.url("/media/leak.flac"));
        let origin = spawn(origin);
        // The origin is fetched over an https-scheme URL (the testkit
        // client transports plaintext regardless — exactly the shape
        // that lets the Location's absolute http target express the
        // downgrade).
        let https_start = origin.url("/start").replacen("http://", "https://", 1);

        // A permissive policy — cross-host allowed, budget 5 — must
        // still not buy a transport downgrade.
        let request =
            HttpRequest::get(https_start).with_redirects(follow_policy(5, true));
        let error = assert_err(
            MockClient::new().fetch(request),
            "an https-to-http redirect must be refused, never followed",
        );
        match error {
            NetworkError::CrossSchemeRedirect { hops } => assert_eq!(hops, 1),
            other => unreachable!("wrong variant: {other:?}"),
        }
        assert_eq!(error.code(), "streaming.net.cross-scheme-redirect");
        assert!(!error.is_retryable(), "a policy refusal is not weather");
        assert_eq!(
            origin.requests().len(),
            1,
            "the https origin answered exactly once"
        );
        assert!(
            target.requests().is_empty(),
            "the downgraded http target must never be contacted"
        );
    }

    /// The production gate the redirect loop consults before every
    /// followed hop: https→http (same host or not) is the only
    /// refused scheme transition; upgrades and same-scheme hops
    /// pass. Pinned separately from the wire-level test because the
    /// ureq adapter cannot be fronted with TLS in-tree (plaintext
    /// mock testkit — see
    /// `https_to_http_redirect_downgrade_is_refused_not_followed`).
    #[test]
    fn scheme_downgrade_gate_pins_transport_safety() {
        let https = Url::parse("https://cdn.example.org/a.flac").must();
        // The exact hostile shapes from the review: `Url::join`
        // accepts an absolute http Location against an https base.
        let same_host_downgrade = https.join("http://cdn.example.org/b.flac").must();
        let cross_host_downgrade = https.join("http://mirror.example.net/b.flac").must();
        assert!(
            is_scheme_downgrade(&https, &same_host_downgrade),
            "a same-host https→http hop is a transport downgrade"
        );
        assert!(
            is_scheme_downgrade(&https, &cross_host_downgrade),
            "a cross-host https→http hop is a transport downgrade too"
        );

        // Everything else passes the gate: the upgrade …
        let http = Url::parse("http://cdn.example.org/a.flac").must();
        let upgrade = http.join("https://cdn.example.org/b.flac").must();
        assert!(!is_scheme_downgrade(&http, &upgrade));
        // … same-scheme hops, both directions of scheme …
        let https_hop = https.join("https://cdn.example.org/b.flac").must();
        let http_hop = http.join("http://cdn.example.org/b.flac").must();
        assert!(!is_scheme_downgrade(&https, &https_hop));
        assert!(!is_scheme_downgrade(&http, &http_hop));
        // … and a plain http start can never trip an https-only rule.
        assert!(!is_scheme_downgrade(&http, &same_host_downgrade));
    }

    /// A cross-host hop into a self-looping second server is a loop,
    /// not a budget exhaustion: the revisit is detected after two
    /// issued requests, one per server. (A pure A→B→A alternation
    /// needs mutual spawn-time URL references the one-shot
    /// `MockConfig` cannot express; the visited-set semantics
    /// exercised here are identical.)
    #[test]
    fn cross_host_chain_into_loop_is_detected_as_redirect_loop() {
        let mut b_config = MockConfig::body(b"never reached".to_vec());
        b_config.self_loop = true;
        let server_b = spawn(b_config);

        let mut a_config = MockConfig::body(Vec::new());
        a_config.absolute_redirect = Some(server_b.url("/b"));
        let server_a = spawn(a_config);

        let error = assert_err(
            UreqClient::new().fetch(HttpRequest::get(server_a.url("/start"))),
            "the loop must be typed, not chased to the budget",
        );
        match error {
            NetworkError::RedirectLoop { hops } => assert_eq!(hops, 2),
            other => unreachable!("wrong variant: {other:?}"),
        }
        assert_eq!(error.code(), "streaming.net.redirect-loop");
        assert_eq!(server_a.requests().len(), 1);
        assert_eq!(server_b.requests().len(), 1);
    }

    /// A self-redirect (302 to the same URL) is a loop detected
    /// before the second request.
    #[test]
    fn self_loop_is_detected_as_redirect_loop() {
        let mut config = MockConfig::body(b"never".to_vec());
        config.self_loop = true;
        let server = spawn(config);
        let error = assert_err(
            UreqClient::new().fetch(HttpRequest::get(server.url("/loop"))),
            "self-loop must be typed",
        );
        match error {
            NetworkError::RedirectLoop { hops } => assert_eq!(hops, 1),
            other => unreachable!("wrong variant: {other:?}"),
        }
        assert_eq!(server.requests().len(), 1);
        assert_eq!(error.code(), "streaming.net.redirect-loop");
    }

    /// Manual mode: the 302 surfaces to the caller with its Location
    /// header, and nothing is followed.
    #[test]
    fn manual_mode_surfaces_the_redirect_response() {
        let mut config = MockConfig::body(b"after hops".to_vec());
        config.redirects = 2;
        let server = spawn(config);
        let request =
            HttpRequest::get(server.url("/start")).with_redirect_mode(RedirectMode::Manual);
        let response = UreqClient::new().fetch(request).must();
        assert_eq!(response.status, 302);
        assert_eq!(response.header("Location"), Some("/r1"));
        let logged = server.requests();
        assert_eq!(logged.len(), 1, "manual mode must not follow");
        assert_eq!(logged[0].path, "/start");
    }

    /// Authorization is never re-sent on a followed hop (ureq's
    /// `RedirectAuthHeaders::Never` parity) — the second request the
    /// server logs must not carry it.
    #[test]
    fn authorization_is_dropped_on_followed_hops() {
        let mut config = MockConfig::body(b"body".to_vec());
        config.redirects = 1;
        let server = spawn(config);
        let request = HttpRequest::get(server.url("/start"))
            .with_header("Authorization", "Bearer secret-token");
        let response = UreqClient::new().fetch(request).must();
        assert_eq!(response.status, 200);
        let logged = server.requests();
        assert_eq!(logged.len(), 2);
        assert_eq!(
            logged[0].header("Authorization"),
            Some("Bearer secret-token")
        );
        assert!(
            logged[1].header("Authorization").is_none(),
            "followed hops must not re-send credentials"
        );
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

    /// The transport safety pins: TLS verification is ALWAYS ON (the
    /// single TlsConfig construction site never flips ureq's
    /// `disable_verification` hatch), ureq itself never follows
    /// redirects (the adapter does, to enforce the port's policy),
    /// and statuses surface for the adapter's own classification.
    #[test]
    fn agent_config_pins_transport_safety() {
        let tls = tls_config();
        assert!(
            !tls.disable_verification(),
            "the only TLS config path must keep verification on"
        );
        let config = agent_config(TimeoutConfig::default());
        assert_eq!(
            config.max_redirects(),
            0,
            "ureq must never follow — the adapter owns the hops"
        );
        assert!(
            !config.http_status_as_error(),
            "statuses classify in the adapter, not as ureq errors"
        );
    }

    /// The agent pool hands out the same agent for the same deadline
    /// config (connection reuse rides on ureq's per-agent pool).
    #[test]
    fn shared_agent_is_cached_per_deadline_config() {
        let one = UreqClient::shared_agent(TimeoutConfig::default());
        let two = UreqClient::shared_agent(TimeoutConfig::default());
        assert!(
            Arc::ptr_eq(&one, &two),
            "same deadlines must share one agent"
        );
        let other = UreqClient::shared_agent(fast_timeouts(Duration::from_secs(1)));
        assert!(
            !Arc::ptr_eq(&one, &other),
            "different deadlines get their own agent"
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

    /// LIVE-network redirect follow — **ignored by default**, same
    /// run command. Measures the adapter's own follower against a
    /// public chain: httpbin answers 302 → 302 → 200.
    #[test]
    #[ignore = "live network required: run with --ignored"]
    fn live_httpbin_redirect_chain_lands_on_200() {
        let response = UreqClient::new()
            .fetch(HttpRequest::get("https://httpbin.org/redirect/2"))
            .must();
        assert_eq!(response.status, 200, "two hops must land on the final body");
        assert!(!response.body.is_empty());
    }
}
