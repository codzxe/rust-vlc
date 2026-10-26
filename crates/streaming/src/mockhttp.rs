//! Mock HTTP/1.1 testkit — **test-only; never used by production code
//! paths** (card 6.A1a).
//!
//! A minimal HTTP/1.1 server on an ephemeral localhost port, built from
//! `std::net::TcpListener` + threads (sync-only, doc 01 §4.2; no async
//! runtime, no external crates). Tests configure one
//! [`MockConfig`] per server: fixed body, Range honoring, a redirect
//! chain, per-response delay, mid-body truncation, or a garbage status
//! line. [`MockServer`] records every request it receives in a log
//! tests assert on, and [`MockClient`] is the thin adapter that speaks
//! the mock's dialect *through* [`HttpFetch`](crate::fetch::HttpFetch),
//! so tests exercise the real port — the same surface the ureq client
//! (6.A1c) and ranged resource (6.A2) will implement.
//!
//! The mock client implements a simplified transport contract: 2xx is
//! `Ok`, 416 on a ranged request is
//! [`NetworkError::RangeNotSatisfiable`], other 4xx/5xx are
//! [`NetworkError::HttpStatus`], and redirects are followed under the
//! request's [`RedirectPolicy`](crate::fetch::RedirectPolicy) with
//! revisit detection. The real client's policy decisions land in
//! 6.A1c — the mock only needs to be *typed and deterministic*.
//!
//! # Shutdown
//!
//! Drop closes the listener: the accept loop polls a stop flag
//! (non-blocking accept) and exits, joining cleanly. Handler threads
//! bound themselves with a 2 s read deadline and always answer with
//! `Connection: close`, so no thread outlives its connection by more
//! than one deadline.

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::error::NetworkError;
use crate::fetch::{HttpFetch, HttpRequest, HttpResponse, Url};

/// How long a handler thread waits for a request head before giving up
/// (keeps dropped-test threads bounded).
const SERVER_READ_DEADLINE: Duration = Duration::from_secs(2);

/// Cap on request/response head bytes: 64 KiB, hostile-input bound.
const HEAD_LIMIT: usize = 64 * 1024;

/// What the mock server received, as recorded in the request log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggedRequest {
    /// Request method as sent (`"GET"`).
    pub method: String,
    /// Path (with query, if any) as sent.
    pub path: String,
    /// Headers in wire order, names as sent.
    pub headers: Vec<(String, String)>,
}

impl LoggedRequest {
    /// First value of a recorded header, ASCII case-insensitive.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// One mock server's behavior (card 6.A1a scope).
///
/// A server serves **one** scripted behavior for every request — tests
/// with several behaviors spawn several servers (ports are cheap and
/// ephemeral), which keeps routing trivial and deterministic.
#[derive(Debug, Clone)]
pub struct MockConfig {
    /// The body served for successful responses (and the source for
    /// Range slices).
    pub body: Vec<u8>,
    /// Honor `Range:` when `true` (206 + `Content-Range`, or 416 when
    /// unsatisfiable); when `false`, Range is ignored and the full
    /// body is served with 200 (the no-range-fallback shape of 6.A2b).
    pub honor_range: bool,
    /// Redirect chain length: an entry request (any path not of the
    /// form `/r<n>`) gets `302 → /r1`, `/r{i}` gets `302 → /r{i+1}`
    /// until `i == redirects`, which serves the body. `0` serves the
    /// body directly.
    pub redirects: u32,
    /// Every response is `302` to the *same* path — a deliberate
    /// redirect loop for the loop-detector test.
    pub self_loop: bool,
    /// Every response is `302` to this **absolute** URL — cross-host
    /// redirect tests (and A→B→A ping-pong loops) point this at a
    /// second server. Takes precedence over `self_loop` and the
    /// `redirects` chain.
    pub absolute_redirect: Option<String>,
    /// Delay before each response is written (read-deadline tests).
    pub delay: Option<Duration>,
    /// Serve the body with this status instead of `200` (the plain,
    /// non-Range path) — error-class mapping tests (404/503).
    pub force_status: Option<u16>,
    /// Advertise `Content-Length: body.len()` but send only the first
    /// `truncate_body_at` body bytes, then tear the connection down —
    /// mid-body truncation (`body-truncated`).
    pub truncate_body_at: Option<usize>,
    /// Answer with a raw garbage first line instead of a status line —
    /// framing-error shape (`protocol-violation`).
    pub garbage_first_line: bool,
}

impl Default for MockConfig {
    fn default() -> Self {
        MockConfig {
            body: Vec::new(),
            honor_range: true,
            redirects: 0,
            self_loop: false,
            absolute_redirect: None,
            delay: None,
            force_status: None,
            truncate_body_at: None,
            garbage_first_line: false,
        }
    }
}

impl MockConfig {
    /// A config serving `body` with Range honored (the default shape).
    pub fn body(body: Vec<u8>) -> MockConfig {
        MockConfig {
            body,
            ..MockConfig::default()
        }
    }
}

/// A running mock HTTP/1.1 server. Drop stops it cleanly.
pub struct MockServer {
    addr: SocketAddr,
    log: Arc<Mutex<Vec<LoggedRequest>>>,
    stop: Arc<AtomicBool>,
    accept_thread: Option<JoinHandle<()>>,
}

impl MockServer {
    /// Bind an ephemeral localhost port and start serving `config`.
    ///
    /// # Errors
    /// The `io::Result` of binding/priming the listener — a broken
    /// test environment, not a product condition.
    pub fn spawn(config: MockConfig) -> std::io::Result<MockServer> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;
        let log = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let config = Arc::new(config);

        let accept_thread = {
            let log = Arc::clone(&log);
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || accept_loop(listener, config, log, stop))
        };

        Ok(MockServer {
            addr,
            log,
            stop,
            accept_thread: Some(accept_thread),
        })
    }

    /// An absolute URL for `path` on this server.
    pub fn url(&self, path: &str) -> String {
        format!("http://{}/{}", self.addr, path.trim_start_matches('/'))
    }

    /// A [`MockClient`] pointed at this server.
    pub fn client(&self) -> MockClient {
        MockClient
    }

    /// A snapshot of every request the server received, in order.
    pub fn requests(&self) -> Vec<LoggedRequest> {
        match self.log.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.accept_thread.take() {
            let _ = thread.join();
        }
    }
}

/// Accept until the stop flag is set; one handler thread per
/// connection (each answered with `Connection: close`).
fn accept_loop(
    listener: TcpListener,
    config: Arc<MockConfig>,
    log: Arc<Mutex<Vec<LoggedRequest>>>,
    stop: Arc<AtomicBool>,
) {
    while !stop.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, _)) => {
                let config = Arc::clone(&config);
                let log = Arc::clone(&log);
                // Detached: bounded by its read deadline + response.
                let _ = std::thread::spawn(move || handle(stream, config, log));
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(_) => {
                // Transient accept failure (e.g. EMFILE): back off,
                // re-check the stop flag, keep serving.
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

/// Serve one request on one connection, then close.
fn handle(mut stream: TcpStream, config: Arc<MockConfig>, log: Arc<Mutex<Vec<LoggedRequest>>>) {
    let _ = stream.set_read_timeout(Some(SERVER_READ_DEADLINE));
    let HeadOutcome::Head(head, _rest) = read_head(&mut stream) else {
        return; // client vanished, stalled, or over the head limit
    };
    let request = parse_request_head(&head);
    match log.lock() {
        Ok(mut guard) => guard.push(request.clone()),
        Err(poisoned) => {
            let mut guard = poisoned.into_inner();
            guard.push(request.clone());
        }
    }
    if let Some(delay) = config.delay {
        std::thread::sleep(delay);
    }
    let _ = write_response(&mut stream, &config, &request);
    let _ = stream.shutdown(Shutdown::Both);
}

/// Outcome of trying to read a head (request or response) off a
/// socket. `Head` carries any bytes received *after* the blank line
/// (TCP may coalesce head and body in one segment — those bytes are
/// body, not garbage).
enum HeadOutcome {
    /// The head text plus leftover post-terminator bytes.
    Head(String, Vec<u8>),
    /// The peer closed before terminating a head.
    Eof,
    /// The read deadline expired first.
    Timeout,
    /// The head exceeded [`HEAD_LIMIT`].
    OverLimit,
}

/// Read a head up to the blank line, bounded by [`HEAD_LIMIT`].
fn read_head(stream: &mut TcpStream) -> HeadOutcome {
    let mut raw: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&raw[..pos]).into_owned();
            let rest = raw[pos + 4..].to_vec();
            return HeadOutcome::Head(head, rest);
        }
        if raw.len() > HEAD_LIMIT {
            return HeadOutcome::OverLimit;
        }
        match stream.read(&mut chunk) {
            Ok(0) => return HeadOutcome::Eof,
            Ok(n) => raw.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                return HeadOutcome::Timeout;
            }
            Err(_) => return HeadOutcome::Eof,
        }
    }
}

/// Parse a request head into method, path, headers (best effort — the
/// log records what arrived, malformed or not).
fn parse_request_head(head: &str) -> LoggedRequest {
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or_default().to_string();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_string(), value.trim().to_string()))
        .collect();
    LoggedRequest {
        method,
        path,
        headers,
    }
}

/// Compose and write the configured response.
fn write_response(
    stream: &mut TcpStream,
    config: &MockConfig,
    request: &LoggedRequest,
) -> std::io::Result<()> {
    if config.garbage_first_line {
        return stream.write_all(b"THIS IS NOT HTTP/1.1 AT ALL\r\n\r\n");
    }
    if let Some(target) = &config.absolute_redirect {
        return stream.write_all(redirect_head(target).as_bytes());
    }
    if config.self_loop {
        return stream.write_all(redirect_head(&request.path).as_bytes());
    }
    if config.redirects > 0 {
        let hop = request
            .path
            .strip_prefix("/r")
            .and_then(|tail| tail.parse::<u32>().ok());
        match hop {
            Some(hop) if hop < config.redirects => {
                return stream.write_all(redirect_head(&format!("/r{}", hop + 1)).as_bytes());
            }
            Some(hop) if hop == config.redirects => { /* fall through: serve the body */ }
            _ => return stream.write_all(redirect_head("/r1").as_bytes()),
        }
    }
    if let Some(cut) = config.truncate_body_at {
        let cut = cut.min(config.body.len());
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            config.body.len()
        )?;
        stream.write_all(&config.body[..cut])?;
        return stream.flush();
    }
    serve_body(stream, config, request)
}

/// The `302` hop response (relative `Location`, no body).
fn redirect_head(location: &str) -> String {
    format!(
        "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}

/// Serve the body, honoring or ignoring `Range:` per config.
fn serve_body(
    stream: &mut TcpStream,
    config: &MockConfig,
    request: &LoggedRequest,
) -> std::io::Result<()> {
    let range = request.header("range").and_then(parse_range_spec);
    if config.honor_range
        && let Some((start, end_inclusive)) = range
    {
        let len = config.body.len() as u64;
        if len == 0 || start >= len {
            write!(
                stream,
                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{len}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )?;
            return stream.flush();
        }
        let last = end_inclusive.map_or(len - 1, |end| end.min(len - 1));
        let slice = &config.body[start as usize..=last as usize];
        write!(
            stream,
            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{last}/{len}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            slice.len()
        )?;
        stream.write_all(slice)?;
        return stream.flush();
    }
    let (code, reason) = match config.force_status {
        Some(code) => (code, reason_phrase(code)),
        None => (200, "OK"),
    };
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        code,
        reason,
        config.body.len()
    )?;
    stream.write_all(&config.body)?;
    stream.flush()
}

/// The reason phrase the mock writes for a forced status (test-only
/// best effort; the clients under test never parse it further).
fn reason_phrase(code: u16) -> &'static str {
    match code {
        200 => "OK",
        404 => "Not Found",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Status",
    }
}

/// Parse a `Range:` value of the forms `bytes=a-b` / `bytes=a-`.
/// Malformed values return `None` (and are then ignored — the RFC 9110
/// fallback-to-200 shape).
fn parse_range_spec(value: &str) -> Option<(u64, Option<u64>)> {
    let spec = value.trim().strip_prefix("bytes=")?;
    let (start, end) = spec.split_once('-')?;
    let start = start.parse::<u64>().ok()?;
    if end.is_empty() {
        return Some((start, None));
    }
    let end = end.parse::<u64>().ok()?;
    if end < start {
        return None;
    }
    Some((start, Some(end)))
}

/// The mock adapter: a real (tiny) HTTP/1.1 client that implements
/// [`HttpFetch`] against a [`MockServer`] socket — tests are written
/// against the port, not against the mock's internals.
#[derive(Debug, Clone, Copy, Default)]
pub struct MockClient;

impl MockClient {
    /// A client for any mock server (stateless; the URL routes it).
    pub fn new() -> MockClient {
        MockClient
    }

    /// One request/response exchange on a fresh connection (the mock
    /// dialect is `Connection: close`).
    fn exchange(&self, url: &Url, request: &HttpRequest) -> Result<HttpResponse, NetworkError> {
        let mut stream = self.connect(url, request)?;
        self.send_request(&mut stream, url, request)?;
        let head = read_response_head(&mut stream)?;
        let content_length = content_length(&head.headers);
        let body = match content_length {
            Some(len) => {
                read_bounded_body(&mut stream, &head.body_prefix, len, request.response_limit)?
            }
            None => read_eof_body(&mut stream, &head.body_prefix, request.response_limit)?,
        };
        Ok(HttpResponse {
            status: head.status,
            reason: head.reason,
            headers: head.headers,
            body,
        })
    }

    fn connect(&self, url: &Url, request: &HttpRequest) -> Result<TcpStream, NetworkError> {
        let addrs = (url.host.as_str(), url.port)
            .to_socket_addrs()
            .map_err(|_| NetworkError::DnsFailure)?
            .collect::<Vec<_>>();
        if addrs.is_empty() {
            return Err(NetworkError::DnsFailure);
        }
        let mut last: Option<NetworkError> = None;
        for addr in addrs {
            match TcpStream::connect_timeout(&addr, request.timeouts.connect) {
                Ok(stream) => {
                    let _ = stream.set_read_timeout(Some(request.timeouts.read));
                    let _ = stream.set_write_timeout(Some(request.timeouts.write));
                    return Ok(stream);
                }
                Err(e) => {
                    last = Some(match e.kind() {
                        ErrorKind::TimedOut | ErrorKind::WouldBlock => NetworkError::ConnectTimeout,
                        ErrorKind::ConnectionRefused => NetworkError::ConnectRefused,
                        _ => NetworkError::Io { source: e },
                    });
                }
            }
        }
        Err(last.unwrap_or(NetworkError::ConnectRefused))
    }

    fn send_request(
        &self,
        stream: &mut TcpStream,
        url: &Url,
        request: &HttpRequest,
    ) -> Result<(), NetworkError> {
        let dirty = |text: &str| text.chars().any(|c| c.is_ascii_control());
        for (name, value) in &request.headers {
            if dirty(name) || dirty(value) {
                return Err(NetworkError::ProtocolViolation {
                    detail: "request header contains control bytes (header injection)",
                });
            }
        }
        let mut head = format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\n",
            request.method.as_str(),
            url.path_and_query,
            url.authority()
        );
        for (name, value) in &request.headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        if let Some(range) = request.range {
            head.push_str(&format!("Range: {}\r\n", range.to_header_value()));
        }
        head.push_str("Connection: close\r\n\r\n");
        stream.write_all(head.as_bytes()).map_err(|e| {
            if e.kind() == ErrorKind::TimedOut || e.kind() == ErrorKind::WouldBlock {
                NetworkError::WriteTimeout
            } else {
                NetworkError::Io { source: e }
            }
        })
    }
}

impl HttpFetch for MockClient {
    fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
        let mut url = Url::parse(&request.url)?;
        let mut visited: Vec<String> = vec![url.to_string()];
        let mut followed: u32 = 0;
        loop {
            let response = self.exchange(&url, &request)?;
            if is_redirect(response.status) {
                let Some(location) = response.header("location").map(str::to_string) else {
                    return Err(NetworkError::ProtocolViolation {
                        detail: "redirect status without a Location header",
                    });
                };
                if followed >= request.redirects.max_redirects {
                    return Err(NetworkError::TooManyRedirects {
                        limit: request.redirects.max_redirects,
                        followed,
                    });
                }
                let next = url.join(&location)?;
                let canonical = next.to_string();
                if visited.contains(&canonical) {
                    return Err(NetworkError::RedirectLoop {
                        hops: visited.len() as u32,
                    });
                }
                visited.push(canonical);
                url = next;
                followed += 1;
                continue;
            }
            return classify(response, &request);
        }
    }
}

/// The mock's simplified status policy (documented in the module docs;
/// the real policy is 6.A1c's).
fn classify(response: HttpResponse, request: &HttpRequest) -> Result<HttpResponse, NetworkError> {
    if (200..=299).contains(&response.status) {
        return Ok(response);
    }
    if response.status == 416 && request.range.is_some() {
        return Err(NetworkError::RangeNotSatisfiable);
    }
    if (400..=599).contains(&response.status) {
        return Err(NetworkError::http_status(response.status));
    }
    // 1xx (no upgrade support in v0) and non-redirect 3xx without
    // Location: surfaced as-is; the caller decides.
    Ok(response)
}

fn is_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

/// A parsed response head plus the body bytes that arrived with it.
struct ResponseHead {
    status: u16,
    reason: String,
    headers: Vec<(String, String)>,
    body_prefix: Vec<u8>,
}

/// Read and parse a response head, keeping any body bytes that arrived
/// in the same segments. Every malformation is a typed
/// [`NetworkError::ProtocolViolation`]; a head-phase deadline expiry
/// is [`NetworkError::ReadTimeout`].
fn read_response_head(stream: &mut TcpStream) -> Result<ResponseHead, NetworkError> {
    let head = match read_head(stream) {
        HeadOutcome::Head(head, rest) => (head, rest),
        HeadOutcome::Timeout => return Err(NetworkError::ReadTimeout),
        HeadOutcome::Eof | HeadOutcome::OverLimit => {
            return Err(NetworkError::ProtocolViolation {
                detail: "response head never terminated",
            });
        }
    };
    let (head_text, rest) = head;
    let mut lines = head_text.split("\r\n");
    let status_line = lines.next().unwrap_or_default();
    let mut parts = status_line.splitn(3, ' ');
    let version = parts.next().unwrap_or_default();
    if !version.starts_with("HTTP/1.") {
        return Err(NetworkError::ProtocolViolation {
            detail: "malformed status line",
        });
    }
    let code = parts
        .next()
        .unwrap_or_default()
        .parse::<u16>()
        .map_err(|_| NetworkError::ProtocolViolation {
            detail: "malformed status line",
        })?;
    let reason = parts.next().unwrap_or_default().to_string();
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_string(), value.trim().to_string()))
        .collect();
    Ok(ResponseHead {
        status: code,
        reason,
        headers,
        body_prefix: rest,
    })
}

fn content_length(headers: &[(String, String)]) -> Option<u64> {
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<u64>().ok())
}

/// Read exactly `expected` body bytes under the memory budget; a
/// premature end (EOF or reset) is [`NetworkError::BodyTruncated`],
/// never a silent short body. `rest` holds body bytes that arrived
/// with the head.
fn read_bounded_body(
    stream: &mut TcpStream,
    rest: &[u8],
    expected: u64,
    limit: u64,
) -> Result<Vec<u8>, NetworkError> {
    if expected > limit {
        return Err(NetworkError::ResponseTooLarge {
            limit,
            actual: expected,
        });
    }
    let mut body = vec![0u8; expected as usize];
    let initial = rest.len().min(body.len());
    body[..initial].copy_from_slice(&rest[..initial]);
    let mut received = initial;
    while received < body.len() {
        match stream.read(&mut body[received..]) {
            Ok(0) => {
                return Err(NetworkError::BodyTruncated {
                    expected,
                    received: received as u64,
                });
            }
            Ok(n) => received += n,
            Err(e) => {
                return Err(map_body_read_error(e, expected, received as u64));
            }
        }
    }
    Ok(body)
}

/// Read an unbounded (no `Content-Length`) body to EOF, still bounded
/// by the memory budget.
fn read_eof_body(stream: &mut TcpStream, rest: &[u8], limit: u64) -> Result<Vec<u8>, NetworkError> {
    let mut body = rest.to_vec();
    if body.len() as u64 > limit {
        return Err(NetworkError::ResponseTooLarge {
            limit,
            actual: body.len() as u64,
        });
    }
    let mut chunk = [0u8; 8192];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => return Ok(body),
            Ok(n) => {
                if body.len() as u64 + n as u64 > limit {
                    return Err(NetworkError::ResponseTooLarge {
                        limit,
                        actual: body.len() as u64 + n as u64,
                    });
                }
                body.extend_from_slice(&chunk[..n]);
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                return Err(NetworkError::ReadTimeout);
            }
            Err(e) => return Err(NetworkError::Io { source: e }),
        }
    }
}

/// Deadline → `ReadTimeout`; mid-body reset → `BodyTruncated`; the
/// rest stay `Io` with the OS error in the chain.
fn map_body_read_error(e: std::io::Error, expected: u64, received: u64) -> NetworkError {
    match e.kind() {
        ErrorKind::WouldBlock | ErrorKind::TimedOut => NetworkError::ReadTimeout,
        ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted => {
            NetworkError::BodyTruncated { expected, received }
        }
        _ => NetworkError::Io { source: e },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::ByteRange;
    use testkit::Must;

    /// 1000 bytes of non-trivial content.
    fn body_1000() -> Vec<u8> {
        (0..1000u32).map(|i| (i % 251) as u8).collect()
    }

    fn spawn(config: MockConfig) -> MockServer {
        MockServer::spawn(config).must()
    }

    /// The port is exercised as a trait object, the way the engine
    /// will hold the real client.
    fn fetcher() -> std::sync::Arc<dyn HttpFetch> {
        std::sync::Arc::new(MockClient)
    }

    #[test]
    fn roundtrip_200_body_equality() {
        let body = body_1000();
        let server = spawn(MockConfig::body(body.clone()));
        let fetch = fetcher();
        let response = fetch
            .fetch(HttpRequest::get(server.url("media/file.bin")))
            .must();
        assert_eq!(response.status, 200);
        assert_eq!(response.reason, "OK");
        assert!(response.is_success());
        assert_eq!(response.body, body);
        assert_eq!(response.header("content-length"), Some("1000"));
        let logged = server.requests();
        assert_eq!(logged.len(), 1);
        assert_eq!(logged[0].method, "GET");
        assert_eq!(logged[0].path, "/media/file.bin");
    }

    #[test]
    fn partial_content_206_with_content_range_echo() {
        let body = body_1000();
        let server = spawn(MockConfig::body(body.clone()));
        let request =
            HttpRequest::get(server.url("media/file.bin")).with_range(ByteRange::Bounded {
                start: 100,
                end_exclusive: 200,
            });
        let response = fetcher().fetch(request).must();
        assert_eq!(response.status, 206);
        assert_eq!(response.header("content-range"), Some("bytes 100-199/1000"));
        assert_eq!(response.body, body[100..200].to_vec());
    }

    #[test]
    fn open_ended_range_reads_to_end() {
        let body = body_1000();
        let server = spawn(MockConfig::body(body.clone()));
        let request = HttpRequest::get(server.url("f")).with_range(ByteRange::From { start: 990 });
        let response = fetcher().fetch(request).must();
        assert_eq!(response.status, 206);
        assert_eq!(response.header("content-range"), Some("bytes 990-999/1000"));
        assert_eq!(response.body, body[990..].to_vec());
    }

    #[test]
    fn range_ignored_config_serves_200_full_body() {
        let body = body_1000();
        let config = MockConfig {
            honor_range: false,
            ..MockConfig::body(body.clone())
        };
        let server = spawn(config);
        let request = HttpRequest::get(server.url("f")).with_range(ByteRange::Bounded {
            start: 0,
            end_exclusive: 10,
        });
        let response = fetcher().fetch(request).must();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, body);
    }

    #[test]
    fn status_416_is_range_not_satisfiable() {
        let server = spawn(MockConfig::body(body_1000()));
        let request = HttpRequest::get(server.url("f")).with_range(ByteRange::Bounded {
            start: 5000,
            end_exclusive: 6000,
        });
        let err = match fetcher().fetch(request) {
            Err(e) => e,
            Ok(response) => unreachable!("416 must be typed, got {}", response.status),
        };
        assert!(matches!(err, NetworkError::RangeNotSatisfiable));
        assert_eq!(err.code(), "streaming.net.range-not-satisfiable");
    }

    #[test]
    fn redirect_chain_of_three_lands_on_final_body() {
        let body = body_1000();
        let config = MockConfig {
            redirects: 3,
            ..MockConfig::body(body.clone())
        };
        let server = spawn(config);
        let response = fetcher()
            .fetch(HttpRequest::get(server.url("start/here")))
            .must();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, body);
        // The card's hop-log assertion: every hop URL was requested.
        let logged = server.requests();
        let paths: Vec<&str> = logged.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(paths, ["/start/here", "/r1", "/r2", "/r3"]);
    }

    #[test]
    fn too_many_redirects_is_bounded_by_policy() {
        let config = MockConfig {
            redirects: 5,
            ..MockConfig::body(body_1000())
        };
        let server = spawn(config);
        let request =
            HttpRequest::get(server.url("start")).with_redirects(crate::fetch::RedirectPolicy {
                max_redirects: 2,
                cross_host: true,
                mode: crate::fetch::RedirectMode::Follow,
            });
        let err = match fetcher().fetch(request) {
            Err(e) => e,
            Ok(response) => unreachable!("bounded policy must fire, got {}", response.status),
        };
        assert!(matches!(
            err,
            NetworkError::TooManyRedirects {
                limit: 2,
                followed: 2,
            }
        ));
    }

    #[test]
    fn self_loop_is_detected_as_redirect_loop() {
        let config = MockConfig {
            self_loop: true,
            ..MockConfig::body(body_1000())
        };
        let server = spawn(config);
        let err = match fetcher().fetch(HttpRequest::get(server.url("loop"))) {
            Err(e) => e,
            Ok(response) => unreachable!("loop must be typed, got {}", response.status),
        };
        // One request issued; the revisit is detected before a second.
        assert!(matches!(err, NetworkError::RedirectLoop { hops: 1 }));
        assert_eq!(err.code(), "streaming.net.redirect-loop");
    }

    #[test]
    fn read_timeout_fires_on_delayed_response() {
        let started = std::time::Instant::now();
        let config = MockConfig {
            delay: Some(Duration::from_millis(400)),
            ..MockConfig::body(body_1000())
        };
        let server = spawn(config);
        let timeouts = crate::fetch::TimeoutConfig {
            connect: Duration::from_secs(2),
            read: Duration::from_millis(120),
            write: Duration::from_secs(2),
        };
        let request = HttpRequest::get(server.url("slow")).with_timeouts(timeouts);
        let err = match fetcher().fetch(request) {
            Err(e) => e,
            Ok(response) => unreachable!("delay must time out, got {}", response.status),
        };
        assert!(matches!(err, NetworkError::ReadTimeout), "got {err:?}");
        assert!(err.is_timeout());
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_secs(3),
            "timeout must fire fast, took {elapsed:?}"
        );
    }

    #[test]
    fn hard_reset_mid_body_is_body_truncated() {
        let body = body_1000();
        let config = MockConfig {
            truncate_body_at: Some(400),
            ..MockConfig::body(body)
        };
        let server = spawn(config);
        let err = match fetcher().fetch(HttpRequest::get(server.url("cut"))) {
            Err(e) => e,
            Ok(response) => unreachable!("truncation must be typed, got {}", response.status),
        };
        match err {
            NetworkError::BodyTruncated { expected, received } => {
                assert_eq!(expected, 1000);
                assert_eq!(received, 400);
            }
            other => unreachable!("wrong error: {other:?}"),
        }
    }

    #[test]
    fn garbage_first_line_is_a_framing_error() {
        let config = MockConfig {
            garbage_first_line: true,
            ..MockConfig::body(body_1000())
        };
        let server = spawn(config);
        let err = match fetcher().fetch(HttpRequest::get(server.url("x"))) {
            Err(e) => e,
            Ok(response) => unreachable!("garbage must be typed, got {}", response.status),
        };
        match err {
            NetworkError::ProtocolViolation { detail } => {
                assert_eq!(detail, "malformed status line");
            }
            other => unreachable!("wrong error: {other:?}"),
        }
        assert_eq!(err.code(), "streaming.net.protocol-violation");
    }

    #[test]
    fn request_log_captures_the_range_header() {
        let server = spawn(MockConfig::body(body_1000()));
        let request = HttpRequest::get(server.url("ranged")).with_range(ByteRange::Bounded {
            start: 250,
            end_exclusive: 750,
        });
        let _ = fetcher().fetch(request).must();
        let logged = server.requests();
        assert_eq!(logged.len(), 1);
        assert_eq!(logged[0].header("range"), Some("bytes=250-749"));
        assert!(logged[0].header("host").is_some());
    }

    #[test]
    fn refused_port_is_connect_refused() {
        // Bind, note the port, close: nothing is listening afterwards.
        let listener = TcpListener::bind(("127.0.0.1", 0)).must();
        let addr = listener.local_addr().must();
        drop(listener);
        let err = match fetcher().fetch(HttpRequest::get(format!("http://{addr}/x"))) {
            Err(e) => e,
            Ok(response) => unreachable!("closed port must refuse, got {}", response.status),
        };
        assert!(matches!(err, NetworkError::ConnectRefused), "got {err:?}");
    }

    #[test]
    fn response_over_budget_is_response_too_large() {
        let server = spawn(MockConfig::body(body_1000()));
        let request = HttpRequest::get(server.url("big")).with_response_limit(64);
        let err = match fetcher().fetch(request) {
            Err(e) => e,
            Ok(response) => unreachable!("over budget must be typed, got {}", response.status),
        };
        assert!(matches!(
            err,
            NetworkError::ResponseTooLarge {
                limit: 64,
                actual: 1000,
            }
        ));
    }

    #[test]
    fn header_injection_is_rejected_typed() {
        let server = spawn(MockConfig::body(body_1000()));
        let request =
            HttpRequest::get(server.url("x")).with_header("X-Evil", "value\r\nHost: evil.example");
        let err = match fetcher().fetch(request) {
            Err(e) => e,
            Ok(response) => unreachable!("injection must be typed, got {}", response.status),
        };
        assert!(matches!(err, NetworkError::ProtocolViolation { .. }));
    }

    #[test]
    fn range_spec_parser_accepts_and_rejects() {
        assert_eq!(parse_range_spec("bytes=0-499"), Some((0, Some(499))));
        assert_eq!(parse_range_spec("bytes=100-"), Some((100, None)));
        assert_eq!(parse_range_spec("bytes=5-2"), None);
        assert_eq!(parse_range_spec("chunks=0-1"), None);
        assert_eq!(parse_range_spec("bytes=a-b"), None);
    }
}
