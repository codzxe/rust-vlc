//! Streaming authentication (card 6.E1) — HTTP Basic + Digest over
//! the [`HttpFetch`](crate::fetch::HttpFetch) port, a redaction-safe
//! credential vocabulary, and the credential-store policy seam.
//!
//! # Flow: challenge-first
//!
//! Credentials are **withheld until the server challenges**: the
//! wrapper's first fetch carries no `Authorization` header, so a
//! server that never asks never receives a secret. On a `401` whose
//! `WWW-Authenticate` names a supported scheme, the wrapper answers
//! the challenge once — Basic (RFC 7617) or Digest MD5 (RFC 7616) —
//! and re-issues. A second `401` is the typed wrong-password shape,
//! [`NetworkError::AuthFailed`].
//!
//! # Credential sources (most specific wins)
//!
//! 1. URL userinfo (`http://user:pass@host/…`), accepted and stripped
//!    by [`Url::parse_with_userinfo`] — the password never reaches a
//!    logged URL string.
//! 2. Explicit credentials ([`AuthFetch::with_credentials`]).
//! 3. A [`CredentialStore`] lookup keyed by the URL's
//!    `scheme://host:port`.
//!
//! # Digest scope `[PROPOSAL]`
//!
//! RFC 7616 MD5 only (`algorithm=MD5`; SHA-256 challenges are a
//! typed refusal). `qop=auth`; `auth-int` needs entity hashing that
//! v0 has no method for. The `cnonce` is derived from the system
//! clock and a per-wrapper counter — std has no RNG and no new
//! dependencies are allowed, so entropy adequacy is `[VERIFY]`. MD5
//! digest itself is legacy-weak by modern standards and is offered
//! for server compatibility, marked `[PROPOSAL]` per the wave card.
//!
//! # Redaction (MP §12)
//!
//! [`Credentials`] and [`crate::fetch::Userinfo`] hand-write `Debug`
//! (and `Display`) so a password can never reach a log line through
//! them — pinned by tests below, per the 6.E1 acceptance criteria.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{AuthSchemeKind, NetworkError};
use crate::fetch::{
    HttpFetch, HttpRequest, HttpResponse, RedirectMode, RedirectPolicy, Url, Userinfo,
};

// ---------------------------------------------------------------- credentials

/// A username/password pair whose formatting never exposes the
/// password (card 6.E1 redaction contract).
#[derive(Clone, PartialEq, Eq)]
pub struct Credentials {
    username: String,
    password: String,
}

impl Credentials {
    /// A credential pair.
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Credentials {
        Credentials {
            username: username.into(),
            password: password.into(),
        }
    }

    /// The username (not a secret).
    pub fn username(&self) -> &str {
        &self.username
    }

    /// The password (a secret — handle with care; never format it).
    pub fn password(&self) -> &str {
        &self.password
    }
}

impl From<Userinfo> for Credentials {
    fn from(info: Userinfo) -> Credentials {
        Credentials::new(info.username, info.password)
    }
}

/// Redacted by construction: `Debug` shows the username (useful in
/// logs) and a placeholder for the password.
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("username", &self.username)
            .field("password", &"[redacted]")
            .finish()
    }
}

/// Redacted by construction: `Display` names the user only.
impl std::fmt::Display for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "credentials for {:?} (password redacted)", self.username)
    }
}

// ------------------------------------------------------------- credential store

/// The credential-store policy seam (card 6.E1): URL-keyed lookup of
/// network credentials, shared across worker threads.
///
/// **Policy decision (recorded):** persistence — OS keychain
/// (keyring/secret-service) vs an encrypted config file vs
/// in-memory-only — is `[PROPOSAL]`, owned by the platform phase; the
/// OS-keychain route is `[VERIFY]` (no pure-Rust keychain binding
/// without new dependencies). This trait is the stable seam either
/// way: a keychain-backed impl and the in-memory impl below are
/// interchangeable at the [`AuthFetch`] composition point.
pub trait CredentialStore: Send + Sync {
    /// Credentials stored for `url`'s origin (`scheme://host:port`),
    /// if any. Path and query never participate in the key.
    fn lookup(&self, url: &Url) -> Option<Credentials>;

    /// Store `credentials` under `url`'s origin key.
    fn put(&self, url: &Url, credentials: Credentials);
}

/// The origin key a [`CredentialStore`] uses: scheme + authority
/// (path/query excluded — credentials are per-server, not per-file).
fn origin_key(url: &Url) -> String {
    format!("{}://{}", url.scheme, url.authority())
}

/// In-memory [`CredentialStore`] (card 6.E1): process-lifetime
/// storage with interior mutability, redacted `Debug`. Persistence is
/// `[PROPOSAL]` (see the trait docs).
#[derive(Default)]
pub struct InMemoryCredentialStore {
    entries: Mutex<HashMap<String, Credentials>>,
}

impl InMemoryCredentialStore {
    /// An empty store.
    pub fn new() -> InMemoryCredentialStore {
        InMemoryCredentialStore::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Credentials>> {
        self.entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl CredentialStore for InMemoryCredentialStore {
    fn lookup(&self, url: &Url) -> Option<Credentials> {
        self.lock().get(&origin_key(url)).cloned()
    }

    fn put(&self, url: &Url, credentials: Credentials) {
        self.lock().insert(origin_key(url), credentials);
    }
}

/// Redacted: shows the entry count only — usernames and passwords
/// never reach a log line through the store.
impl std::fmt::Debug for InMemoryCredentialStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self.lock().len();
        f.debug_struct("InMemoryCredentialStore")
            .field("entries", &count)
            .finish()
    }
}

// ------------------------------------------------------------------- schemes

/// The `Authorization: Basic …` header value for a credential pair
/// (RFC 7617; the user/pass separator string is UTF-8 encoded as the
/// RFC's default charset).
pub fn basic_authorization(username: &str, password: &str) -> String {
    let raw = format!("{username}:{password}");
    format!("Basic {}", base64_encode(raw.as_bytes()))
}

/// A parsed `WWW-Authenticate` challenge (scheme + parameters).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Challenge {
    /// `Basic realm="…"` (RFC 7617).
    Basic {
        /// The offered realm (informational for v0 — credentials are
        /// chosen before the challenge is seen).
        realm: Option<String>,
    },
    /// `Digest realm="…" nonce="…" …` (RFC 7616).
    Digest(DigestChallenge),
}

/// The RFC 7616 digest challenge parameters this layer understands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DigestChallenge {
    /// The protection space (enters HA1).
    pub realm: String,
    /// The server's nonce (enters the response hash).
    pub nonce: String,
    /// The server's opaque token, echoed when present.
    pub opaque: Option<String>,
    /// The offered quality-of-protection tokens (may be empty — the
    /// RFC 2617 legacy shape).
    pub qop: Vec<String>,
    /// The offered algorithm (default MD5 when absent).
    pub algorithm: Option<String>,
}

/// Parse a `WWW-Authenticate` value into a [`Challenge`].
///
/// Unknown schemes and digest challenges missing `realm` or `nonce`
/// return `None` (the caller maps that to a typed protocol
/// violation) — malformed server input is never guessed at (MP §8).
pub(crate) fn parse_www_authenticate(value: &str) -> Option<Challenge> {
    let (scheme, rest) = value.trim().split_once(' ')?;
    if scheme.eq_ignore_ascii_case("basic") {
        let realm = parse_auth_params(rest)
            .into_iter()
            .find(|(name, _)| name == "realm")
            .map(|(_, value)| value);
        return Some(Challenge::Basic { realm });
    }
    if scheme.eq_ignore_ascii_case("digest") {
        let params = parse_auth_params(rest);
        let get = |name: &str| {
            params
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.clone())
        };
        let qop = get("qop")
            .map(|q| {
                q.split(',')
                    .map(str::trim)
                    .filter(|token| !token.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        return Some(Challenge::Digest(DigestChallenge {
            realm: get("realm")?,
            nonce: get("nonce")?,
            opaque: get("opaque"),
            qop,
            algorithm: get("algorithm"),
        }));
    }
    None
}

/// Parse comma-separated `name=value` auth parameters; values may be
/// quoted strings (with backslash escapes). Malformed tails are
/// skipped, never fatal — this is server-controlled text.
pub(crate) fn parse_auth_params(text: &str) -> Vec<(String, String)> {
    let mut params = Vec::new();
    let mut chars = text.chars().peekable();
    loop {
        while matches!(chars.peek(), Some(',' | ' ' | '\t')) {
            chars.next();
        }
        let mut name = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' {
                break;
            }
            name.push(c);
            chars.next();
        }
        if name.is_empty() || chars.peek() != Some(&'=') {
            break;
        }
        chars.next(); // consume '='
        let mut value = String::new();
        if chars.peek() == Some(&'"') {
            chars.next();
            while let Some(c) = chars.next() {
                match c {
                    '\\' => {
                        if let Some(escaped) = chars.next() {
                            value.push(escaped);
                        }
                    }
                    '"' => break,
                    _ => value.push(c),
                }
            }
        } else {
            while let Some(&c) = chars.peek() {
                if c == ',' {
                    break;
                }
                value.push(c);
                chars.next();
            }
            value = value.trim().to_string();
        }
        params.push((name.to_ascii_lowercase(), value));
    }
    params
}

/// The inputs of one RFC 7616 `response` computation (the many-arg
/// shape collapsed — the digest is a function of exactly these).
pub(crate) struct DigestInputs<'a> {
    /// The offered username.
    pub username: &'a str,
    /// The secret (never formatted anywhere).
    pub password: &'a str,
    /// The challenge realm (enters HA1).
    pub realm: &'a str,
    /// The request method (`GET` in v0).
    pub method: &'a str,
    /// The request URI (path + query) as sent.
    pub uri: &'a str,
    /// The server's nonce.
    pub nonce: &'a str,
    /// The nonce count (8 hex digits).
    pub nc: &'a str,
    /// The client nonce.
    pub cnonce: &'a str,
    /// The quality of protection (`"auth"`), or `None` for the
    /// RFC 2617 legacy shape.
    pub qop: Option<&'a str>,
}

/// The RFC 7616 `response` digest (MD5, `qop=auth` or the RFC 2617
/// legacy no-qop shape).
pub(crate) fn digest_response(inputs: &DigestInputs<'_>) -> String {
    let DigestInputs {
        username,
        password,
        realm,
        method,
        uri,
        nonce,
        nc,
        cnonce,
        qop,
    } = inputs;
    let ha1 = md5_hex(format!("{username}:{realm}:{password}").as_bytes());
    let ha2 = md5_hex(format!("{method}:{uri}").as_bytes());
    match qop {
        Some(qop) => md5_hex(format!("{ha1}:{nonce}:{nc}:{cnonce}:{qop}:{ha2}").as_bytes()),
        None => md5_hex(format!("{ha1}:{nonce}:{ha2}").as_bytes()),
    }
}

/// Build the `Authorization: Digest …` header value for a challenge.
///
/// `None` means the challenge names an unsupported algorithm (only
/// MD5 — SHA-256 is a deliberate typed refusal, no new deps).
pub(crate) fn digest_authorization(
    credentials: &Credentials,
    challenge: &DigestChallenge,
    method: &str,
    uri: &str,
    nc: &str,
    cnonce: &str,
) -> Option<String> {
    if let Some(algorithm) = &challenge.algorithm
        && !algorithm.eq_ignore_ascii_case("MD5")
    {
        return None;
    }
    let qop = if challenge
        .qop
        .iter()
        .any(|token| token.eq_ignore_ascii_case("auth"))
    {
        Some("auth")
    } else {
        None
    };
    let response = digest_response(&DigestInputs {
        username: credentials.username(),
        password: credentials.password(),
        realm: &challenge.realm,
        method,
        uri,
        nonce: &challenge.nonce,
        nc,
        cnonce,
        qop,
    });
    let mut header = format!(
        "Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", uri=\"{uri}\"",
        credentials.username(),
        challenge.realm,
        challenge.nonce
    );
    if let Some(qop) = qop {
        header.push_str(&format!(", qop={qop}, nc={nc}, cnonce=\"{cnonce}\""));
    }
    if let Some(opaque) = &challenge.opaque {
        header.push_str(&format!(", opaque=\"{opaque}\""));
    }
    header.push_str(&format!(", response=\"{response}\""));
    Some(header)
}

// ------------------------------------------------------------------ the wrapper

/// An [`HttpFetch`] wrapper that answers HTTP authentication
/// challenges (card 6.E1): Basic and Digest MD5, credentials from
/// URL userinfo, explicit configuration, or a [`CredentialStore`].
///
/// See the [module docs](self) for the challenge-first flow and the
/// redaction contract.
pub struct AuthFetch {
    inner: Arc<dyn HttpFetch>,
    credentials: Option<Credentials>,
    store: Option<Arc<dyn CredentialStore>>,
    /// Monotonic counter for digest `nc` values (one challenge is
    /// answered per fetch; the counter keeps them distinct).
    digest_counter: AtomicU32,
}

impl AuthFetch {
    /// Wrap `inner` with no credentials (a plain pass-through until
    /// credentials are configured).
    pub fn new(inner: impl HttpFetch + 'static) -> AuthFetch {
        AuthFetch {
            inner: Arc::new(inner),
            credentials: None,
            store: None,
            digest_counter: AtomicU32::new(1),
        }
    }

    /// Attach explicit credentials (source 2 in the module's
    /// precedence order).
    pub fn with_credentials(mut self, credentials: Credentials) -> AuthFetch {
        self.credentials = Some(credentials);
        self
    }

    /// Attach a credential store (source 3 in the module's
    /// precedence order).
    pub fn with_store(mut self, store: impl CredentialStore + 'static) -> AuthFetch {
        self.store = Some(Arc::new(store));
        self
    }

    /// The next digest `nc` value (8 hex digits, RFC 7616 §3.4).
    fn next_nc(&self) -> String {
        let n = self.digest_counter.fetch_add(1, Ordering::Relaxed);
        format!("{n:08x}")
    }

    /// A client nonce: MD5 over the server nonce, the URI, the wall
    /// clock, and the counter. std has no RNG and no new
    /// dependencies are allowed — entropy adequacy `[VERIFY]`.
    fn next_cnonce(&self, nonce: &str, uri: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let counter = self.digest_counter.load(Ordering::Relaxed);
        md5_hex(format!("{nonce}:{uri}:{nanos}:{counter}").as_bytes())
    }

    /// The authenticated retry: one re-issue with the computed
    /// `Authorization` header; a second 401 is the typed
    /// wrong-password answer.
    fn answer(
        &self,
        request: &HttpRequest,
        authorization: String,
        scheme: AuthSchemeKind,
    ) -> Result<HttpResponse, NetworkError> {
        let mut authorized = request.clone();
        authorized
            .headers
            .retain(|(name, _)| !name.eq_ignore_ascii_case("authorization"));
        authorized
            .headers
            .push(("Authorization".to_string(), authorization));
        match self.inner.fetch(authorized) {
            Ok(response) => Ok(response),
            Err(NetworkError::HttpStatus { code: 401, .. }) => {
                Err(NetworkError::AuthFailed { scheme })
            }
            Err(err) => Err(err),
        }
    }

    /// Follow a redirect chain **manually** (bounded by the
    /// template's redirect budget, loop- and cross-host-checked),
    /// returning the final URL alongside the final response.
    ///
    /// `authorization` is `(header value, allowed host)`: the
    /// `Authorization` header is attached **only on hops whose host
    /// matches `allowed host`** (case-insensitive) — a cross-origin
    /// hop never receives these credentials (the 6.H1
    /// credential-leak posture; same-origin hops are the curl/VLC
    /// default trust boundary).
    ///
    /// Manual following is what lets the auth layer know the URL the
    /// challenge actually came from (the digest `uri` and the
    /// same-origin check both need it).
    fn follow(
        &self,
        template: &HttpRequest,
        start: Url,
        authorization: Option<(String, String)>,
    ) -> Result<(Url, HttpResponse), NetworkError> {
        let mut url = start;
        let mut visited: Vec<String> = vec![url.to_string()];
        let mut followed: u32 = 0;
        loop {
            let mut hop_request = HttpRequest {
                method: template.method,
                url: url.to_string(),
                headers: template
                    .headers
                    .iter()
                    .filter(|(name, _)| !name.eq_ignore_ascii_case("authorization"))
                    .cloned()
                    .collect(),
                range: template.range,
                timeouts: template.timeouts,
                redirects: RedirectPolicy {
                    max_redirects: template.redirects.max_redirects,
                    cross_host: template.redirects.cross_host,
                    mode: RedirectMode::Manual,
                },
                response_limit: template.response_limit,
            };
            if let Some((value, allowed_host)) = &authorization
                && url.host.eq_ignore_ascii_case(allowed_host)
            {
                hop_request
                    .headers
                    .push(("Authorization".to_string(), value.clone()));
            }
            let response = self.inner.fetch_with_challenge(hop_request)?;
            if matches!(response.status, 301 | 302 | 303 | 307 | 308)
                && response.header("location").is_some()
            {
                let location = response.header("location").unwrap_or_default().to_string();
                if followed >= template.redirects.max_redirects {
                    return Err(NetworkError::TooManyRedirects {
                        limit: template.redirects.max_redirects,
                        followed,
                    });
                }
                let next = url.join(&location)?;
                let cross_host = !next.host.eq_ignore_ascii_case(&url.host);
                if cross_host && !template.redirects.cross_host {
                    return Err(NetworkError::CrossHostRedirect { hops: followed + 1 });
                }
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
            return Ok((url, response));
        }
    }
}

impl HttpFetch for AuthFetch {
    fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
        // Source 1: URL userinfo — accepted, stripped, and used.
        let (original_url, userinfo) = Url::parse_with_userinfo(&request.url)?;
        let mut request = request;
        if userinfo.is_some() {
            request.url = original_url.to_string();
        }

        // Most specific source wins: userinfo > explicit > store.
        let credentials = userinfo
            .map(Credentials::from)
            .or_else(|| self.credentials.clone())
            .or_else(|| {
                self.store
                    .as_ref()
                    .and_then(|store| store.lookup(&original_url))
            });

        // Challenge-first probe: credentials withheld until the
        // server asks for them, redirects followed manually so the
        // challenge's true URL is known.
        match self.follow(&request, original_url.clone(), None) {
            Ok((final_url, response)) => {
                if response.status != 401 {
                    return classify_response(response, &request);
                }
                let Some(credentials) = credentials else {
                    // Nothing to answer with: the 401 is the answer.
                    return Err(NetworkError::http_status(401));
                };
                // A cross-origin challenge target never receives
                // these credentials (same-origin trust boundary).
                if !final_url.host.eq_ignore_ascii_case(&original_url.host) {
                    return Err(NetworkError::http_status(401));
                }
                let value = response
                    .header("www-authenticate")
                    .ok_or(NetworkError::ProtocolViolation {
                        detail: "401 without a WWW-Authenticate challenge",
                    })?
                    .to_string();
                let (authorization, scheme) = match parse_www_authenticate(&value) {
                    Some(Challenge::Basic { .. }) => (
                        basic_authorization(credentials.username(), credentials.password()),
                        AuthSchemeKind::Basic,
                    ),
                    Some(Challenge::Digest(challenge)) => {
                        let nc = self.next_nc();
                        let cnonce = self.next_cnonce(&challenge.nonce, &final_url.path_and_query);
                        match digest_authorization(
                            &credentials,
                            &challenge,
                            "GET",
                            &final_url.path_and_query,
                            &nc,
                            &cnonce,
                        ) {
                            Some(authorization) => (authorization, AuthSchemeKind::Digest),
                            None => {
                                return Err(NetworkError::ProtocolViolation {
                                    detail: "digest challenge names an unsupported algorithm",
                                });
                            }
                        }
                    }
                    None => {
                        return Err(NetworkError::ProtocolViolation {
                            detail: "unsupported or unparseable WWW-Authenticate challenge",
                        });
                    }
                };
                // The authenticated retry: the chain is re-followed
                // with the header attached on same-origin hops; a
                // second 401 is the typed wrong-password answer.
                let (_, response) = self.follow(
                    &request,
                    original_url,
                    Some((authorization, final_url.host.clone())),
                )?;
                if response.status == 401 {
                    return Err(NetworkError::AuthFailed { scheme });
                }
                classify_response(response, &request)
            }
            // Conservative transport (no challenge surfacing — the
            // trait default): a 401 error with no readable
            // challenge. Offer Basic once, then fail typed (the
            // VLC-style fallback).
            Err(NetworkError::HttpStatus { code: 401, .. }) => match credentials {
                Some(credentials) => self.answer(
                    &request,
                    basic_authorization(credentials.username(), credentials.password()),
                    AuthSchemeKind::Basic,
                ),
                None => Err(NetworkError::http_status(401)),
            },
            Err(err) => Err(err),
        }
    }
}

/// Redacted: the wrapper's configuration shape only — credentials
/// (whose own `Debug` is redacted anyway) are summarized, never
/// formatted.
impl std::fmt::Debug for AuthFetch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthFetch")
            .field("credentials", &self.credentials.is_some())
            .field("store", &self.store.is_some())
            .finish()
    }
}

/// The status classification the transports apply (mirrored here for
/// the challenge-mode responses that bypassed it): 2xx `Ok`, 416 the
/// typed range rejection, 4xx/5xx the typed status error, everything
/// else surfaced for the caller to judge.
fn classify_response(
    response: HttpResponse,
    request: &HttpRequest,
) -> Result<HttpResponse, NetworkError> {
    if response.is_success() {
        return Ok(response);
    }
    if response.status == 416 && request.range.is_some() {
        return Err(NetworkError::RangeNotSatisfiable);
    }
    if (400..=599).contains(&response.status) {
        return Err(NetworkError::http_status(response.status));
    }
    Ok(response)
}

// ---------------------------------------------------------------- base64 (RFC 4648)

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 with padding (RFC 4648) — no deps, testkit +
/// Basic auth use.
pub(crate) fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let bytes = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(bytes[0]) << 16) | (u32::from(bytes[1]) << 8) | u32::from(bytes[2]);
        out.push(BASE64_ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(BASE64_ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            BASE64_ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            BASE64_ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Base64 decode (padding and ASCII whitespace tolerated); `None` on
/// any non-alphabet byte — malformed input is never guessed at.
pub(crate) fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let value_of = |byte: u8| {
        BASE64_ALPHABET
            .iter()
            .position(|&alphabet| alphabet == byte)
            .map(|position| position as u32)
    };
    let mut out = Vec::new();
    let mut accumulator = 0u32;
    let mut bits = 0u32;
    for &byte in text.as_bytes() {
        if matches!(byte, b'=' | b'\r' | b'\n' | b' ' | b'\t') {
            continue;
        }
        accumulator = (accumulator << 6) | value_of(byte)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((accumulator >> bits) as u8);
        }
    }
    Some(out)
}

// ------------------------------------------------------------------ MD5 (RFC 1321)

/// Per-round shift amounts (RFC 1321 §3.4).
const MD5_SHIFTS: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

/// Sine-derived constants (RFC 1321 §3.4): `floor(abs(sin(i+1)) · 2^32)`.
const MD5_K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

/// MD5 (RFC 1321) — clean-room, no deps. Used only inside the Digest
/// auth computation; MD5's collision weakness is why Digest itself is
/// `[PROPOSAL]`.
pub(crate) fn md5(input: &[u8]) -> [u8; 16] {
    let mut a0: u32 = 0x6745_2301;
    let mut b0: u32 = 0xefcd_ab89;
    let mut c0: u32 = 0x98ba_dcfe;
    let mut d0: u32 = 0x1032_5476;

    // Padding: 0x80, zeros to 56 mod 64, then the LE bit length.
    let bit_length = u64::try_from(input.len())
        .unwrap_or(u64::MAX)
        .wrapping_mul(8);
    let mut message = input.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_le_bytes());

    for block in message.chunks(64) {
        let mut words = [0u32; 16];
        for (i, word) in words.iter_mut().enumerate() {
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&block[i * 4..i * 4 + 4]);
            *word = u32::from_le_bytes(bytes);
        }
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64usize {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let sum = f
                .wrapping_add(a)
                .wrapping_add(MD5_K[i])
                .wrapping_add(words[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(sum.rotate_left(MD5_SHIFTS[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    let mut digest = [0u8; 16];
    digest[0..4].copy_from_slice(&a0.to_le_bytes());
    digest[4..8].copy_from_slice(&b0.to_le_bytes());
    digest[8..12].copy_from_slice(&c0.to_le_bytes());
    digest[12..16].copy_from_slice(&d0.to_le_bytes());
    digest
}

/// MD5 as lowercase hex.
pub(crate) fn md5_hex(input: &[u8]) -> String {
    md5(input)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::HttpRequest;
    use testkit::Must;

    /// RFC 1321 §A.5 test vectors — the MD5 core is pinned to the
    /// standard before anything auth-shaped trusts it.
    #[test]
    fn md5_rfc1321_vectors() {
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_hex(b"a"), "0cc175b9c0f1b6a831c399e269772661");
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            md5_hex(b"message digest"),
            "f96b697d7cb7938d525a2f31aaf161d0"
        );
        assert_eq!(
            md5_hex(b"abcdefghijklmnopqrstuvwxyz"),
            "c3fcd3d76192e4007dfb496cca67e13b"
        );
        assert_eq!(
            md5_hex(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"),
            "d174ab98d277d9f5a5611c2c9f419d9f"
        );
    }

    /// RFC 4648 §10 test vectors + round-trip.
    #[test]
    fn base64_rfc4648_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        for raw in [
            &b""[..],
            b"f",
            b"fo",
            b"foo",
            b"auth vector longer than twenty",
        ] {
            let encoded = base64_encode(raw);
            assert_eq!(base64_decode(&encoded).as_deref(), Some(raw));
        }
        assert_eq!(base64_decode("!!!not-base64!!!"), None);
    }

    /// RFC 2617 §3.5's worked digest example (the classic known
    /// vector, unchanged in RFC 7616's MD5 path): Mufasa's response
    /// must be `6629fae49393a05397450978507c4ef1`.
    #[test]
    fn digest_rfc2617_known_vector() {
        let response = digest_response(&DigestInputs {
            username: "Mufasa",
            password: "Circle Of Life",
            realm: "testrealm@host.com",
            method: "GET",
            uri: "/dir/index.html",
            nonce: "dcd98b7102dd2f0e8b11d0f600bfb0c093",
            nc: "00000001",
            cnonce: "0a4f113b",
            qop: Some("auth"),
        });
        assert_eq!(response, "6629fae49393a05397450978507c4ef1");
        // The legacy no-qop shape (RFC 2617 §3.2.2.1): HA1:nonce:HA2.
        let legacy = digest_response(&DigestInputs {
            username: "Mufasa",
            password: "Circle Of Life",
            realm: "testrealm@host.com",
            method: "GET",
            uri: "/dir/index.html",
            nonce: "dcd98b7102dd2f0e8b11d0f600bfb0c093",
            nc: "",
            cnonce: "",
            qop: None,
        });
        let ha1 = md5_hex(b"Mufasa:testrealm@host.com:Circle Of Life");
        let ha2 = md5_hex(b"GET:/dir/index.html");
        assert_eq!(
            legacy,
            md5_hex(format!("{ha1}:dcd98b7102dd2f0e8b11d0f600bfb0c093:{ha2}").as_bytes())
        );
    }

    /// `WWW-Authenticate` parsing: the RFC 2617 §3.5 challenge, verbatim.
    #[test]
    fn parse_rfc2617_challenge() {
        let challenge = parse_www_authenticate(
            "Digest realm=\"testrealm@host.com\", qop=\"auth,auth-int\", \
             nonce=\"dcd98b7102dd2f0e8b11d0f600bfb0c093\", \
             opaque=\"5ccc069c403ebaf9f0171e9517f40e41\"",
        )
        .must();
        match challenge {
            Challenge::Digest(digest) => {
                assert_eq!(digest.realm, "testrealm@host.com");
                assert_eq!(digest.nonce, "dcd98b7102dd2f0e8b11d0f600bfb0c093");
                assert_eq!(
                    digest.opaque.as_deref(),
                    Some("5ccc069c403ebaf9f0171e9517f40e41")
                );
                assert_eq!(digest.qop, vec!["auth".to_string(), "auth-int".to_string()]);
                assert_eq!(digest.algorithm, None);
            }
            other => unreachable!("wrong challenge: {other:?}"),
        }
        let basic = parse_www_authenticate("Basic realm=\"Lumen\"").must();
        assert_eq!(
            basic,
            Challenge::Basic {
                realm: Some("Lumen".to_string())
            }
        );
        assert_eq!(parse_www_authenticate("Negotiate"), None);
        // Digest without a nonce: typed refusal, not a guess.
        assert_eq!(parse_www_authenticate("Digest realm=\"x\""), None);
    }

    /// The digest header built for the RFC 2617 vector carries the
    /// exact response and echoes realm/nonce/opaque.
    #[test]
    fn digest_authorization_carries_the_vector_response() {
        let challenge = DigestChallenge {
            realm: "testrealm@host.com".to_string(),
            nonce: "dcd98b7102dd2f0e8b11d0f600bfb0c093".to_string(),
            opaque: Some("5ccc069c403ebaf9f0171e9517f40e41".to_string()),
            qop: vec!["auth".to_string(), "auth-int".to_string()],
            algorithm: None,
        };
        let credentials = Credentials::new("Mufasa", "Circle Of Life");
        let header = digest_authorization(
            &credentials,
            &challenge,
            "GET",
            "/dir/index.html",
            "00000001",
            "0a4f113b",
        )
        .must();
        assert!(header.contains("username=\"Mufasa\""));
        assert!(header.contains("realm=\"testrealm@host.com\""));
        assert!(header.contains("nonce=\"dcd98b7102dd2f0e8b11d0f600bfb0c093\""));
        assert!(header.contains("uri=\"/dir/index.html\""));
        assert!(header.contains("qop=auth, nc=00000001, cnonce=\"0a4f113b\""));
        assert!(header.contains("response=\"6629fae49393a05397450978507c4ef1\""));
        // A SHA-256 challenge is a typed refusal, never a downgrade.
        let sha256 = DigestChallenge {
            algorithm: Some("SHA-256".to_string()),
            ..challenge
        };
        assert_eq!(
            digest_authorization(&credentials, &sha256, "GET", "/", "1", "c"),
            None
        );
    }

    /// Redaction contract (6.E1 acceptance): `Debug`/`Display` of
    /// credentials — and `Debug` of the store and wrapper — never
    /// contain the password, while the username stays useful.
    #[test]
    fn redaction_never_exposes_passwords() {
        let credentials = Credentials::new("alice", "hunter2-super-secret");
        let debug = format!("{credentials:?}");
        let display = format!("{credentials}");
        assert!(!debug.contains("hunter2-super-secret"));
        assert!(!display.contains("hunter2-super-secret"));
        assert!(debug.contains("alice"));
        assert!(display.contains("alice"));

        let url = Url::parse("http://media.example.com:8080/stream.flac").must();
        let store = InMemoryCredentialStore::new();
        store.put(&url, credentials);
        let store_debug = format!("{store:?}");
        assert!(!store_debug.contains("hunter2-super-secret"));
        assert!(!store_debug.contains("alice"), "usernames stay out too");

        let wrapper =
            AuthFetch::new(NoopFetch).with_credentials(Credentials::new("bob", "another-secret"));
        let wrapper_debug = format!("{wrapper:?}");
        assert!(!wrapper_debug.contains("another-secret"));

        // Userinfo redaction (the URL-carried secret).
        let info = crate::fetch::Url::parse_with_userinfo("http://u:p@h/")
            .must()
            .1
            .must();
        assert!(!format!("{info:?}").contains(":p"));
        assert_eq!(
            format!("{info:?}"),
            "Userinfo { username: \"u\", password: \"[redacted]\" }"
        );
    }

    /// The store is URL-origin-keyed: same origin, different path —
    /// found; different port or host — not found (never a cross-site
    /// credential leak).
    #[test]
    fn store_lookup_is_origin_keyed() {
        let store = InMemoryCredentialStore::new();
        let key_url = Url::parse("https://cdn.example.org:8443/a/list.m3u").must();
        store.put(&key_url, Credentials::new("user", "pass"));
        let same_origin = Url::parse("https://cdn.example.org:8443/other/path.flac").must();
        assert_eq!(
            store.lookup(&same_origin).map(|c| c.username().to_string()),
            Some("user".to_string())
        );
        let other_port = Url::parse("https://cdn.example.org/track.flac").must();
        let other_host = Url::parse("https://elsewhere.example.org:8443/track.flac").must();
        assert_eq!(store.lookup(&other_port), None);
        assert_eq!(store.lookup(&other_host), None);
    }

    /// The conservative-transport fallback: an inner fetch that does
    /// NOT override `fetch_with_challenge` (the trait default) still
    /// gets Basic auth after its typed 401 — and its requests prove
    /// the header only appears on the retry.
    #[derive(Clone)]
    struct BasicOnlyServer {
        requests: Arc<Mutex<Vec<HttpRequest>>>,
    }

    impl HttpFetch for BasicOnlyServer {
        fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {
            match self.requests.lock() {
                Ok(mut guard) => guard.push(request.clone()),
                Err(poisoned) => poisoned.into_inner().push(request.clone()),
            }
            let authorized = request.headers.iter().any(|(name, value)| {
                name.eq_ignore_ascii_case("authorization") && value.as_str() == "Basic dXNlcjpwYXNz"
            });
            if authorized {
                Ok(HttpResponse {
                    status: 200,
                    reason: "OK".to_string(),
                    headers: Vec::new(),
                    body: b"payload".to_vec(),
                })
            } else {
                Err(NetworkError::http_status(401))
            }
        }
    }

    #[test]
    fn conservative_transport_gets_basic_fallback() {
        let server = BasicOnlyServer {
            requests: Arc::new(Mutex::new(Vec::new())),
        };
        let auth =
            AuthFetch::new(server.clone()).with_credentials(Credentials::new("user", "pass"));
        let response = auth.fetch(HttpRequest::get("http://example.com/x")).must();
        assert_eq!(response.body, b"payload");
        let logged = match server.requests.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };
        assert_eq!(logged.len(), 2, "probe (401) then Basic retry");
        assert!(
            logged[0]
                .headers
                .iter()
                .all(|(name, _)| !name.eq_ignore_ascii_case("authorization"))
        );
        assert!(
            logged[1]
                .headers
                .iter()
                .any(|(name, value)| name.eq_ignore_ascii_case("authorization")
                    && value == "Basic dXNlcjpwYXNz")
        );
    }

    /// A no-op inner fetch (fixture for the Debug redaction test).
    struct NoopFetch;

    impl HttpFetch for NoopFetch {
        fn fetch(&self, _request: HttpRequest) -> Result<HttpResponse, NetworkError> {
            Err(NetworkError::ConnectRefused)
        }
    }
}
