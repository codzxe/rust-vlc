//! The mock fetch/Response test double (card 7.H4; test-only,
//! `#[cfg(test)]` by the parent module) — the browser lane's mirror of
//! `crates/streaming`'s `mockhttp` harness (card 6.A1a): scripted
//! routes, a request log tests assert on, and the same typed error
//! vocabulary the real transport answers — one discipline, no sockets.
//!
//! # What the mock IS
//!
//! The REAL transport on this lane is the Worker's JS glue calling
//! `fetch()` (doc 09 §6.1); the wasm module has no imports and no
//! sockets. [`MockFetch`] is the in-process double that answers
//! through the SAME [`FetchPort`](crate::io::fetch_port::FetchPort)
//! contract the message lane rides, so native-cfg tests exercise the
//! real request/response types over every server shape the seekability
//! matrix names:
//!
//! | [`RouteBehavior`] | Serves | The matrix row it models |
//! |---|---|---|
//! | [`HonorsRange`](RouteBehavior::HonorsRange) | `206` + agreeing `Content-Range` (`bytes a-b/total`), the clamped span | the accept-ranges server |
//! | [`IgnoresRange`](RouteBehavior::IgnoresRange) | `200` + `Content-Length` + the whole body | the no-range server (forward-only) |
//! | [`TruncatesRange`](RouteBehavior::TruncatesRange) | `206` + unknown-total `Content-Range` (`bytes a-b/*`) with the body cut mid-span | the partial/truncating server (a connection dying mid-chunk: the header promised the span, the body stops short, the total was never declared) |
//! | [`RedirectsTo`](RouteBehavior::RedirectsTo) | a `302` chain the MOCK follows itself | redirects (the browser's `fetch()` follows them — `final_url`/`redirected` are the observables) |
//! | [`Status`](RouteBehavior::Status) | a fixed status, empty body | the fixed-status row (404/500/…) |
//! | [`Fails`](RouteBehavior::Fails) | `fetch()` rejects | the network error (DNS, CORS, offline) |
//!
//! # Redirect following (the browser's own job)
//!
//! A real `fetch()` follows redirect chains before the promise
//! resolves; the caller observes `response.url` + `response.redirected`.
//! The mock mirrors that verbatim: chains are followed internally (up
//! to [`MAX_REDIRECTS`], the Chromium-class budget `[VERIFY
//! per-browser]`), a surfaced 3xx is impossible from THIS transport,
//! and the over-budget chain is the typed network refusal.
//!
//! # The memory budget (the glue's mirror)
//!
//! The JS glue reads `response.arrayBuffer()` and enforces the
//! request's `response_limit` before staging. The mock enforces the
//! same bound on every answer shape (the advertised span of a `206`,
//! the delivered body of a `200`) — [`FetchError::ResponseTooLarge`],
//! never an unbounded allocation. The staged source sets that limit to
//! its window budget on every request, so the bound IS the lane's
//! bounded-memory contract, in both transports.
//!
//! # The request log
//!
//! Every fetch records the path, the `Range:` header and the budget
//! ([`LoggedFetch`]) — the evidence the matrix tests assert on (the
//! probe was ranged, the seek re-fetched, the redirect chain was
//! walked). Mirrors mockhttp's `LoggedRequest` discipline.

use std::sync::Mutex;

use crate::io::fetch_port::{FetchError, FetchPort, FetchRequest, FetchResponse};

/// The redirect-following budget — the browser's own (20 on
/// Chromium-class engines `[VERIFY per-browser]`); a longer chain is
/// the typed network refusal, exactly what `fetch()` answers.
pub const MAX_REDIRECTS: usize = 20;

/// One scripted server behavior (see the module table).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteBehavior {
    /// The accept-ranges server: ranged requests answer `206` with an
    /// agreeing `Content-Range` and the clamped span; rangeless
    /// requests answer `200` with the whole body.
    HonorsRange,
    /// The no-range server: every request answers `200` with the
    /// whole body (`Range:` ignored — the forward-only shape).
    IgnoresRange,
    /// The truncating server: ranged requests answer `206` with an
    /// unknown-total `Content-Range` (`bytes a-b/*`) and HALF the
    /// promised span — the mid-stream truncation, never a silent EOF.
    TruncatesRange,
    /// A redirect hop: the mock follows to `to` (a path), recording
    /// the observables a real `fetch()` would surface.
    RedirectsTo(&'static str),
    /// A fixed status with an empty body (404, 500, 503, …).
    Status(u16),
    /// `fetch()` itself rejects (network error: DNS, CORS, offline) —
    /// the typed [`FetchError::Network`] with the scripted detail.
    Fails(&'static str),
}

/// One recorded fetch (the request log's row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggedFetch {
    /// The path as requested (pre-redirect).
    pub path: String,
    /// The `Range:` header the request carried, if any.
    pub range_header: Option<String>,
    /// The response memory budget the request declared.
    pub response_limit: u64,
}

/// The mock transport: scripted routes + the request log. `Clone` is
/// not needed (tests share one handle); the log lives behind a
/// `Mutex` so `fetch(&self, …)` matches the port's shared contract.
#[derive(Debug, Default)]
pub struct MockFetch {
    routes: Vec<(String, RouteBehavior, Vec<u8>)>,
    log: Mutex<Vec<LoggedFetch>>,
}

impl MockFetch {
    /// A transport with no routes (every path answers 404).
    #[must_use]
    pub fn new() -> MockFetch {
        MockFetch::default()
    }

    /// A transport over scripted routes: `(path, behavior, body)`.
    #[must_use]
    pub fn with_routes(routes: Vec<(&str, RouteBehavior, &[u8])>) -> MockFetch {
        MockFetch {
            routes: routes
                .into_iter()
                .map(|(path, behavior, body)| (path.to_string(), behavior, body.to_vec()))
                .collect(),
            log: Mutex::new(Vec::new()),
        }
    }

    /// The recorded fetches, in arrival order (test evidence).
    #[must_use]
    pub fn log(&self) -> Vec<LoggedFetch> {
        match self.log.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// How many fetches were issued (the log's length).
    #[must_use]
    pub fn fetches(&self) -> usize {
        match self.log.lock() {
            Ok(guard) => guard.len(),
            Err(poisoned) => poisoned.into_inner().len(),
        }
    }

    /// The recorded `Range:` headers, in order (the probe/seek
    /// evidence the matrix asserts on).
    #[must_use]
    pub fn range_headers(&self) -> Vec<Option<String>> {
        self.log().into_iter().map(|row| row.range_header).collect()
    }

    /// The route for `path`, if one is scripted.
    fn route(&self, path: &str) -> Option<&(String, RouteBehavior, Vec<u8>)> {
        self.routes
            .iter()
            .find(|(route_path, _, _)| route_path == path)
    }

    /// Serve one request: log it, walk the redirect chain, answer the
    /// terminal route (the whole browser-shaped lifecycle in-process).
    fn serve(&self, request: &FetchRequest) -> Result<FetchResponse, FetchError> {
        let origin = origin_of(&request.url);
        let mut path = path_of(&request.url).to_string();
        let mut redirected = false;
        for _ in 0..=MAX_REDIRECTS {
            self.record_redirect_hop(&path, request);
            let Some((_, behavior, body)) = self.route(&path) else {
                // No route: the 404 shape (mockhttp's unmatched-path
                // posture — route matching, not script exhaustion).
                return Ok(FetchResponse {
                    id: request.id,
                    status: 404,
                    headers: Vec::new(),
                    body: Vec::new(),
                    final_url: format!("{origin}{path}"),
                    redirected,
                });
            };
            match behavior {
                RouteBehavior::RedirectsTo(next) => {
                    redirected = true;
                    path = next.to_string();
                    continue;
                }
                RouteBehavior::Fails(detail) => {
                    return Err(FetchError::Network {
                        detail: (*detail).to_string(),
                    });
                }
                RouteBehavior::Status(status) => {
                    return Ok(FetchResponse {
                        id: request.id,
                        status: *status,
                        headers: Vec::new(),
                        body: Vec::new(),
                        final_url: format!("{origin}{path}"),
                        redirected,
                    });
                }
                RouteBehavior::HonorsRange => {
                    return Ok(self.answer_honoring(request, origin, &path, body, redirected, true));
                }
                RouteBehavior::IgnoresRange => {
                    return Ok(
                        self.answer_honoring(request, origin, &path, body, redirected, false)
                    );
                }
                RouteBehavior::TruncatesRange => {
                    return self.answer_truncating(request, origin, &path, body, redirected);
                }
            }
        }
        // The chain overran the browser's own budget: fetch() answers
        // a network error, and so does the mock.
        Err(FetchError::Network {
            detail: format!("redirect chain exceeded the {MAX_REDIRECTS}-hop budget"),
        })
    }

    /// Log one hop (the request as issued against `path`).
    fn record_redirect_hop(&self, path: &str, request: &FetchRequest) {
        let row = LoggedFetch {
            path: path.to_string(),
            range_header: request.range_header(),
            response_limit: request.response_limit,
        };
        match self.log.lock() {
            Ok(mut guard) => guard.push(row),
            Err(poisoned) => poisoned.into_inner().push(row),
        }
    }

    /// The `206`/`200` answer of a body-owning route. `honors` picks
    /// the accept-ranges shape (`206` + the clamped span) over the
    /// no-range shape (`200` + the whole body).
    fn answer_honoring(
        &self,
        request: &FetchRequest,
        origin: &str,
        path: &str,
        body: &[u8],
        redirected: bool,
        honors: bool,
    ) -> FetchResponse {
        let total = body.len() as u64;
        match (honors, request.range) {
            (true, Some(range)) if range.start >= total => {
                // Nothing exists at the offset: the 416 shape (the
                // empty resource IS start=0 on an empty body).
                FetchResponse {
                    id: request.id,
                    status: 416,
                    headers: vec![("Content-Range".to_string(), format!("bytes */{total}"))],
                    body: Vec::new(),
                    final_url: format!("{origin}{path}"),
                    redirected,
                }
            }
            (true, Some(range)) => {
                // The clamped span: end inclusive, at most the last
                // byte (a request past the end serves to the end —
                // RFC 9110 §14.2's satisfiable prefix).
                let end = range.end_exclusive.min(total).saturating_sub(1);
                let span = end.saturating_sub(range.start).saturating_add(1);
                let slice = &body[range.start as usize..=(end as usize)];
                FetchResponse {
                    id: request.id,
                    status: 206,
                    headers: vec![
                        (
                            "Content-Range".to_string(),
                            format!("bytes {}-{}/{}", range.start, end, total),
                        ),
                        ("Content-Length".to_string(), span.to_string()),
                        ("Accept-Ranges".to_string(), "bytes".to_string()),
                    ],
                    body: slice.to_vec(),
                    final_url: format!("{origin}{path}"),
                    redirected,
                }
            }
            (_, _) => {
                // Rangeless (or the no-range server): the whole body.
                FetchResponse {
                    id: request.id,
                    status: 200,
                    headers: vec![
                        ("Content-Length".to_string(), total.to_string()),
                        ("Accept-Ranges".to_string(), "none".to_string()),
                    ],
                    body: body.to_vec(),
                    final_url: format!("{origin}{path}"),
                    redirected,
                }
            }
        }
    }

    /// The truncating answer: `206` + `bytes a-b/*` (the total was
    /// never declared) with HALF the promised span delivered — the
    /// mid-stream truncation the detection must refuse typed.
    fn answer_truncating(
        &self,
        request: &FetchRequest,
        origin: &str,
        path: &str,
        body: &[u8],
        redirected: bool,
    ) -> Result<FetchResponse, FetchError> {
        let Some(range) = request.range else {
            // Rangeless requests have no span to lie about: the whole
            // body, honestly (documented — the row models a ranged
            // truncation only).
            return Ok(self.answer_honoring(request, origin, path, body, redirected, false));
        };
        let total = body.len() as u64;
        if range.start >= total {
            return Ok(FetchResponse {
                id: request.id,
                status: 416,
                headers: vec![("Content-Range".to_string(), format!("bytes */{total}"))],
                body: Vec::new(),
                final_url: format!("{origin}{path}"),
                redirected,
            });
        }
        let end = range.end_exclusive.min(total).saturating_sub(1);
        let span = end.saturating_sub(range.start).saturating_add(1);
        // Half the span (at least one byte): the connection died
        // mid-chunk, after the header promised the whole span.
        let delivered = span.div_ceil(2).clamp(1, span);
        let slice = &body[range.start as usize..(range.start + delivered) as usize];
        Ok(FetchResponse {
            id: request.id,
            status: 206,
            headers: vec![
                (
                    "Content-Range".to_string(),
                    format!("bytes {}-{}/{}", range.start, end, "*"),
                ),
                ("Content-Length".to_string(), delivered.to_string()),
            ],
            body: slice.to_vec(),
            final_url: format!("{origin}{path}"),
            redirected,
        })
    }
}

impl FetchPort for MockFetch {
    fn fetch(&self, request: &FetchRequest) -> Result<FetchResponse, FetchError> {
        let response = self.serve(request)?;
        enforce_budget(request, &response)?;
        Ok(response)
    }
}

/// The memory budget the glue mirrors: every answer's advertised or
/// delivered size must fit the request's `response_limit` (a `206`'s
/// Content-Range span is the advertisement; the body is the delivery;
/// a `200`'s Content-Length is the advertisement).
fn enforce_budget(request: &FetchRequest, response: &FetchResponse) -> Result<(), FetchError> {
    let advertised: Option<u64> = match response.status {
        206 => response.header("Content-Range").and_then(|value| {
            value
                .split_once('/')
                .and_then(|(span, _)| span.split_once('-'))
                .and_then(|(_, end)| end.trim().parse::<u64>().ok())
                .map(|end| end.saturating_add(1))
        }),
        200 => response
            .header("Content-Length")
            .and_then(|value| value.trim().parse::<u64>().ok()),
        _ => None,
    };
    if let Some(advertised) = advertised
        && advertised > request.response_limit
    {
        return Err(FetchError::ResponseTooLarge {
            limit: request.response_limit,
            actual: advertised,
        });
    }
    let delivered = response.body.len() as u64;
    if delivered > request.response_limit {
        return Err(FetchError::ResponseTooLarge {
            limit: request.response_limit,
            actual: delivered,
        });
    }
    Ok(())
}

/// The origin of a URL (`scheme://host[:port]`), or the whole string
/// when no `://` separates (the mock's URLs are test vectors).
fn origin_of(url: &str) -> &str {
    match url.split_once("://") {
        Some((_, rest)) => match rest.find('/') {
            Some(at) => &url[..url.len() - rest.len() + at],
            None => url,
        },
        None => "",
    }
}

/// The path of a URL (from the first `/` after the authority),
/// `/` when the URL carries none.
fn path_of(url: &str) -> &str {
    match url.split_once("://") {
        Some((_, rest)) => match rest.find('/') {
            Some(at) => &rest[at..],
            None => "/",
        },
        None => match url.find('/') {
            Some(at) => &url[at..],
            None => url,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::fetch_port::{DEFAULT_WINDOW_BYTES, RangeRequest};
    use crate::io::seekability::{Seekability, detect_seekability};
    use testkit::{Must, assert_err};

    /// The default-budget request shape (the staged source always
    /// sends its own budget; tests send the window budget).
    fn request(id: u64, url: &str, range: Option<RangeRequest>) -> FetchRequest {
        FetchRequest {
            id,
            url: url.to_string(),
            range,
            response_limit: DEFAULT_WINDOW_BYTES as u64,
        }
    }

    /// THE MATRIX, end to end through the double — row 1: the
    /// accept-ranges server answers the probe `206` + an agreeing
    /// `Content-Range`, and the detection says seekable.
    #[test]
    fn accept_ranges_route_answers_206_and_detects_ranged() {
        let transport = MockFetch::with_routes(vec![(
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            "/a.bin",
            RouteBehavior::HonorsRange,
            b"0123456789".as_slice(),
        )]);
        let probe = request(
            1,
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            "https://media.example/a.bin",
            Some(RangeRequest {
                start: 0,
                end_exclusive: 4,
            }),
        );
        let response = transport.fetch(&probe).must();
        assert_eq!(response.status, 206);
        assert_eq!(response.header("Content-Range"), Some("bytes 0-3/10"));
        assert_eq!(response.body, b"0123");
        assert!(!response.redirected);
        assert_eq!(
            detect_seekability(&probe, &response).must(),
            Seekability::Ranged { total: Some(10) }
        );
        // The probe was logged with its Range header (the evidence).
        assert_eq!(
            transport.range_headers(),
            vec![Some("bytes=0-3".to_string())]
        );
    }

    /// THE MATRIX, row 2 — the no-range server: the ranged probe
    /// answers `200` with the whole body; detection says unseekable.
    #[test]
    fn no_range_route_answers_200_and_detects_unseekable() {
        let transport = MockFetch::with_routes(vec![(
            "/b.bin",
            RouteBehavior::IgnoresRange,
            b"abcdef".as_slice(),
        )]);
        let probe = request(
            2,
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            "https://media.example/b.bin",
            Some(RangeRequest {
                start: 0,
                end_exclusive: 4,
            }),
        );
        let response = transport.fetch(&probe).must();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"abcdef");
        assert_eq!(
            detect_seekability(&probe, &response).must(),
            Seekability::Unseekable { length: Some(6) }
        );
        assert_eq!(transport.log().first().must().path, "/b.bin");
    }

    /// THE MATRIX, row 3 — the truncating server: `206` + `bytes
    /// a-b/*` with half the span; detection refuses the typed
    /// truncation (never a silent EOF).
    #[test]
    fn truncating_route_refuses_the_typed_truncation() {
        let transport = MockFetch::with_routes(vec![(
            "/c.bin",
            RouteBehavior::TruncatesRange,
            b"0123456789".as_slice(),
        )]);
        let probe = request(
            3,
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            "https://media.example/c.bin",
            Some(RangeRequest {
                start: 0,
                end_exclusive: 4,
            }),
        );
        let response = transport.fetch(&probe).must();
        assert_eq!(response.status, 206);
        assert_eq!(response.header("Content-Range"), Some("bytes 0-3/*"));
        assert_eq!(response.body.len(), 2, "half the promised span");
        assert_eq!(
            assert_err(detect_seekability(&probe, &response), "mid-stream cut"),
            FetchError::Truncated {
                expected: 4,
                got: 2
            }
        );
    }

    /// THE MATRIX, row 4 — redirects: the mock FOLLOWS the chain (the
    /// browser's job) and surfaces `final_url`/`redirected`; the
    /// detection never sees a 3xx. A chain over the browser's own
    /// budget is the typed network refusal.
    #[test]
    fn redirect_chain_is_followed_with_observables() {
        let transport = MockFetch::with_routes(vec![
            (
                "/hop",
                RouteBehavior::RedirectsTo("/final.bin"),
                b"".as_slice(),
            ),
            (
                "/final.bin",
                RouteBehavior::HonorsRange,
                b"final-bytes".as_slice(),
            ),
        ]);
        let probe = request(
            4,
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            "https://media.example/hop",
            Some(RangeRequest {
                start: 0,
                end_exclusive: 4,
            }),
        );
        let response = transport.fetch(&probe).must();
        assert_eq!(
            response.status, 206,
            "the chain resolved to the final route"
        );
        assert_eq!(response.body, b"fina");
        assert!(response.redirected, "the observables say a hop happened");
        // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
        assert_eq!(response.final_url, "https://media.example/final.bin");
        // Both hops were logged, in order.
        let log = transport.log();
        assert_eq!(
            log.iter().map(|row| row.path.as_str()).collect::<Vec<_>>(),
            vec!["/hop", "/final.bin"]
        );
        // The detection runs on the FINAL answer (never a 3xx).
        assert_eq!(
            detect_seekability(&probe, &response).must(),
            Seekability::Ranged {
                total: Some("final-bytes".len() as u64)
            }
        );
        // The over-budget chain: the typed network refusal.
        let looped = MockFetch::with_routes(vec![
            ("/a", RouteBehavior::RedirectsTo("/b"), b"".as_slice()),
            ("/b", RouteBehavior::RedirectsTo("/a"), b"".as_slice()),
        ]);
        let err = assert_err(
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            looped.fetch(&request(5, "https://media.example/a", None)),
            "the loop must terminate",
        );
        assert!(matches!(err, FetchError::Network { .. }), "{err}");
    }

    /// THE MATRIX, row 5 — fixed statuses: the typed HTTP refusal,
    /// branchable like the native `NetworkError::HttpStatus`.
    #[test]
    fn fixed_status_route_answers_the_typed_http_refusal() {
        let transport =
            MockFetch::with_routes(vec![("/gone", RouteBehavior::Status(503), b"".as_slice())]);
        let response = transport
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            .fetch(&request(6, "https://media.example/gone", None))
            .must();
        assert_eq!(response.status, 503);
        assert_eq!(
            assert_err(
                // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
                detect_seekability(&request(6, "https://media.example/gone", None), &response),
                "uninterpreted status"
            ),
            FetchError::HttpStatus { status: 503 }
        );
    }

    /// The network failure: `fetch()` rejected — the typed
    /// [`FetchError::Network`], the detail as scripted.
    #[test]
    fn failing_route_answers_the_typed_network_refusal() {
        let transport = MockFetch::with_routes(vec![(
            "/dead",
            RouteBehavior::Fails("dns: NXDOMAIN"),
            b"".as_slice(),
        )]);
        let err = assert_err(
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            transport.fetch(&request(7, "https://media.example/dead", None)),
            "network failure",
        );
        assert_eq!(
            err,
            FetchError::Network {
                detail: "dns: NXDOMAIN".to_string()
            }
        );
    }

    /// The unmatched path answers 404 (route matching, not script
    /// exhaustion — mockhttp's own posture).
    #[test]
    fn unmatched_path_answers_404() {
        let transport = MockFetch::new();
        let response = transport
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            .fetch(&request(8, "https://media.example/nothing", None))
            .must();
        assert_eq!(response.status, 404);
    }

    /// The clamping rule: a range past the end serves to the end (the
    /// satisfiable prefix, RFC 9110 §14.2), and a range starting past
    /// the end answers 416 (the empty-resource shape at 0 on an empty
    /// body).
    #[test]
    fn ranges_clamp_to_the_end_and_refuse_past_it() {
        let transport = MockFetch::with_routes(vec![(
            "/small.bin",
            RouteBehavior::HonorsRange,
            b"01234567".as_slice(),
        )]);
        let over = request(
            9,
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            "https://media.example/small.bin",
            Some(RangeRequest {
                start: 4,
                end_exclusive: 4096,
            }),
        );
        let response = transport.fetch(&over).must();
        assert_eq!(response.header("Content-Range"), Some("bytes 4-7/8"));
        assert_eq!(response.body, b"4567");
        let past = request(
            10,
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            "https://media.example/small.bin",
            Some(RangeRequest {
                start: 8,
                end_exclusive: 12,
            }),
        );
        let response = transport.fetch(&past).must();
        assert_eq!(response.status, 416);
        // The empty resource: start 0 on an empty body is 416 too —
        // honored as Empty by the detection.
        let empty = MockFetch::with_routes(vec![(
            "/empty.bin",
            RouteBehavior::HonorsRange,
            b"".as_slice(),
        )]);
        let probe = request(
            11,
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            "https://media.example/empty.bin",
            Some(RangeRequest {
                start: 0,
                end_exclusive: 4,
            }),
        );
        let response = empty.fetch(&probe).must();
        assert_eq!(response.status, 416);
        assert_eq!(
            detect_seekability(&probe, &response).must(),
            Seekability::Empty
        );
    }

    /// The budget mirror: an answer whose advertised span (206) or
    /// delivered body (200) exceeds the request's limit is the typed
    /// over-budget refusal — the glue's enforcement, mirrored.
    #[test]
    fn the_budget_is_enforced_on_every_answer_shape() {
        let transport = MockFetch::with_routes(vec![(
            "/big.bin",
            RouteBehavior::IgnoresRange,
            b"x".repeat(1024).as_slice(),
        )]);
        let tight = FetchRequest {
            id: 12,
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            url: "https://media.example/big.bin".to_string(),
            range: None,
            response_limit: 64,
        };
        let err = assert_err(transport.fetch(&tight), "over budget");
        assert_eq!(
            err,
            FetchError::ResponseTooLarge {
                limit: 64,
                actual: 1024
            }
        );
        // A 206's advertised span over the budget refuses too.
        let ranged = MockFetch::with_routes(vec![(
            "/lying.bin",
            RouteBehavior::HonorsRange,
            b"y".repeat(2048).as_slice(),
        )]);
        let probe = FetchRequest {
            id: 13,
            // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
            url: "https://media.example/lying.bin".to_string(),
            range: Some(RangeRequest {
                start: 0,
                end_exclusive: 2048,
            }),
            response_limit: 1024,
        };
        let err = assert_err(ranged.fetch(&probe), "lying span");
        assert!(matches!(err, FetchError::ResponseTooLarge { .. }), "{err}");
    }

    /// The mock implements the port (the one-vocabulary claim: the
    /// same types ride the port contract and the message lane).
    #[test]
    fn the_mock_speaks_the_port_contract() {
        let transport = MockFetch::with_routes(vec![(
            "/port.bin",
            RouteBehavior::HonorsRange,
            b"0123456789".as_slice(),
        )]);
        let port = &transport as &dyn FetchPort;
        let response = port
            .fetch(&request(
                14,
                // lumen-allow-literal: reserved example host — mock-transport test vector, never a shipped endpoint
                "https://media.example/port.bin",
                Some(RangeRequest {
                    start: 2,
                    end_exclusive: 5,
                }),
            ))
            .must();
        assert_eq!(response.body, b"234");
        assert_eq!(response.header("Content-Range"), Some("bytes 2-4/10"));
    }
}
