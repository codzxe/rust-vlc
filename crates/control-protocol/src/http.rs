//! RFC 6455 §4.2 HTTP upgrade handshake — card 7.J1, in-tree.
//!
//! Server side: parse the client's opening HTTP request from a byte
//! stream (bounded read, dirty-world tolerant: bare-LF lines accepted,
//! request bytes never echoed into errors), validate the required
//! headers, compute `Sec-WebSocket-Accept`, write the 101.
//!
//! Client side (the conformance driver, [`client`]): build a request
//! with a given key and validate the server's accept value.
//!
//! The `Sec-WebSocket-Accept` computation is `base64(SHA-1(key +
//! GUID))` over the RFC 6455 GUID — the two in-tree primitives of
//! [`crate::sha1`] and [`crate::base64`].

use std::io::Read;
use std::io::Write;

use crate::base64;
use crate::error::ProtocolError;
use crate::sha1::Sha1;

/// RFC 6458? No — RFC 6455 §1.3: the WebSocket GUID.
pub const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// The only WebSocket version this server speaks (RFC 6455).
pub const WEBSOCKET_VERSION: u8 = 13;

/// Cap on the opening HTTP request: any request head larger than this
/// is rejected without buffering it (D-1 bounded memory).
pub const MAX_REQUEST_HEAD: usize = 8 * 1024;

/// The validated opening handshake (server view).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeRequest {
    /// The request target (path). v1 accepts any path — routing is a
    /// future concern (documented; not a security decision).
    pub path: String,
    /// The decoded 16-byte `Sec-WebSocket-Key` nonce.
    pub key: [u8; 16],
}

/// `Sec-WebSocket-Accept` for a key (RFC 6455 §4.2.2 step 4).
///
/// NOTE the exact input: the RFC concatenates the base64 TEXT of
/// `Sec-WebSocket-Key` (as it appeared on the wire) with the GUID —
/// NOT the decoded 16 nonce bytes. [`UpgradeRequest::key`] carries
/// the decoded nonce (shape-validated); this function re-encodes it,
/// so the two representations can never drift.
pub fn accept_value(key: &[u8; 16]) -> String {
    let key_text = base64::encode(key);
    let mut hasher = Sha1::new();
    hasher.update(key_text.as_bytes());
    hasher.update(WEBSOCKET_GUID.as_bytes());
    base64::encode(&hasher.finalize())
}

/// The 101 response bytes for an accept value.
pub fn response_101(accept: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept}\r\n\
         \r\n"
    )
    .into_bytes()
}

/// An HTTP error response (used to refuse upgrades; the reason is a
/// static string, never request-derived).
pub fn error_response(status: u16, reason: &str) -> Vec<u8> {
    format!("HTTP/1.1 {status} {reason}\r\nConnection: close\r\n\r\n").into_bytes()
}

/// Read and validate the client's opening handshake from `stream`.
///
/// Dirty-world posture (D-3): the head is read with a hard 8 KiB cap
/// and no partial-buffer retention on failure; header names are
/// case-insensitive; the `Connection` header is a token list; values
/// are trimmed. Failure returns the HTTP error response to write, plus
/// the typed error.
///
/// # Errors
/// [`ProtocolError::HttpUpgrade`] with a stable reason token, or
/// [`ProtocolError::Io`]/[`ProtocolError::Closed`] on transport
/// trouble. The caller writes the returned error response bytes.
pub fn read_upgrade_request<R: Read>(stream: &mut R) -> Result<UpgradeRequest, ProtocolError> {
    let head = match read_head(stream) {
        Ok(Some(head)) => head,
        Ok(None) => return Err(ProtocolError::Closed),
        Err(err) => return Err(err),
    };
    match validate_head(&head) {
        Ok(request) => Ok(request),
        Err(reason) => Err(ProtocolError::HttpUpgrade { reason }),
    }
}

/// Read up to `\r\n\r\n` (or bare `\n\n`) within [`MAX_REQUEST_HEAD`].
/// `Ok(None)` = clean EOF before any byte.
fn read_head<R: Read>(stream: &mut R) -> Result<Option<String>, ProtocolError> {
    let mut buf: Vec<u8> = Vec::with_capacity(512);
    let mut byte = [0u8; 1];
    loop {
        match stream.read(&mut byte) {
            Ok(0) => {
                return if buf.is_empty() {
                    Ok(None)
                } else {
                    Err(ProtocolError::HttpUpgrade {
                        reason: "eof-mid-request",
                    })
                };
            }
            Ok(_) => {
                buf.push(byte[0]);
                if buf.len() > MAX_REQUEST_HEAD {
                    return Err(ProtocolError::HttpUpgrade {
                        reason: "request-head-too-large",
                    });
                }
                if ends_with_head_terminator(&buf) {
                    return Ok(Some(String::from_utf8_lossy(&buf).into_owned()));
                }
            }
            Err(err) => {
                return if is_disconnect(&err) && buf.is_empty() {
                    Ok(None)
                } else {
                    Err(ProtocolError::Io {
                        what: "reading upgrade request",
                        source: err,
                    })
                };
            }
        }
    }
}

/// `\r\n\r\n` or (dirty-world) a bare `\n\n` terminator.
fn ends_with_head_terminator(buf: &[u8]) -> bool {
    buf.ends_with(b"\r\n\r\n") || buf.ends_with(b"\n\n")
}

/// Whether an I/O error is a peer disconnect (read side).
fn is_disconnect(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::BrokenPipe
    )
}

/// Validate a request head per RFC 6455 §4.2.1. Stable reason tokens.
fn validate_head(head: &str) -> Result<UpgradeRequest, &'static str> {
    let mut lines = head.split('\n').map(|line| line.trim_end_matches('\r'));
    let request_line = lines.next().ok_or("empty-request")?;
    let mut parts = request_line.split_ascii_whitespace();
    let method = parts.next().ok_or("empty-request-line")?;
    let path = parts.next().ok_or("missing-request-target")?;
    let version = parts.next().ok_or("missing-http-version")?;
    if method != "GET" {
        return Err("method-not-get");
    }
    if !version.eq_ignore_ascii_case("HTTP/1.1") {
        return Err("not-http-1-1");
    }
    if path.is_empty() || path.len() > 2048 {
        return Err("bad-request-target");
    }

    let mut host = false;
    let mut upgrade = false;
    let mut connection_upgrade = false;
    let mut key_text: Option<&str> = None;
    let mut version_header: Option<&str> = None;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue; // malformed line tolerated; required headers are checked below
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        match name.as_str() {
            "host" => host = !value.is_empty(),
            "upgrade" => {
                upgrade = value
                    .split(',')
                    .any(|token| token.trim().eq_ignore_ascii_case("websocket"));
            }
            "connection" => {
                connection_upgrade = value
                    .split(',')
                    .any(|token| token.trim().eq_ignore_ascii_case("upgrade"));
            }
            "sec-websocket-key" => key_text = Some(value),
            "sec-websocket-version" => version_header = Some(value),
            _ => {}
        }
    }
    if !host {
        return Err("missing-host");
    }
    if !upgrade {
        return Err("missing-upgrade-websocket");
    }
    if !connection_upgrade {
        return Err("missing-connection-upgrade");
    }
    match version_header {
        Some(value) if value.trim() == "13" => {}
        Some(_) => return Err("unsupported-websocket-version"),
        None => return Err("missing-websocket-version"),
    }
    let key_text = key_text.ok_or("missing-sec-websocket-key")?;
    let decoded = base64::decode(key_text).map_err(|_| "bad-sec-websocket-key")?;
    let key: [u8; 16] = match decoded.as_slice() {
        slice if slice.len() == 16 => {
            let mut key = [0u8; 16];
            key.copy_from_slice(slice);
            key
        }
        _ => return Err("bad-sec-websocket-key"),
    };
    Ok(UpgradeRequest {
        path: path.to_owned(),
        key,
    })
}

/// Build a client-side opening request for `key` (test client).
pub fn client_request(path: &str, key: &[u8; 16], host: &str) -> Vec<u8> {
    let encoded = base64::encode(key);
    format!(
        "GET {path} HTTP/1.1\r\n\
         Host: {host}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: {encoded}\r\n\
         Sec-WebSocket-Version: 13\r\n\
         \r\n"
    )
    .into_bytes()
}

/// Validate a server's 101 response against our key (test client).
///
/// # Errors
/// [`ProtocolError::HttpUpgrade`] when the response is not the exact
/// 101 + accept the RFC demands.
pub fn validate_server_response(response: &str, key: &[u8; 16]) -> Result<(), ProtocolError> {
    let expected_accept = accept_value(key);
    let mut lines = response.split('\n').map(|line| line.trim_end_matches('\r'));
    let status = lines.next().ok_or(ProtocolError::HttpUpgrade {
        reason: "empty-response",
    })?;
    if !status.starts_with("HTTP/1.1 101") {
        return Err(ProtocolError::HttpUpgrade {
            reason: "not-switching-protocols",
        });
    }
    let mut accept: Option<String> = None;
    for line in lines {
        if let Some((_, value)) = line
            .split_once(':')
            .filter(|(name, _)| name.trim().eq_ignore_ascii_case("sec-websocket-accept"))
        {
            accept = Some(value.trim().to_owned());
        }
    }
    match accept {
        Some(value) if value == expected_accept => Ok(()),
        Some(_) => Err(ProtocolError::HttpUpgrade {
            reason: "accept-mismatch",
        }),
        None => Err(ProtocolError::HttpUpgrade {
            reason: "missing-accept",
        }),
    }
}

/// Read a full server response head from `stream` (test client; the
/// same bounded read as the server side, minus validation).
pub fn read_response_head<R: Read>(stream: &mut R) -> Result<String, ProtocolError> {
    match read_head(stream) {
        Ok(Some(head)) => Ok(head),
        Ok(None) => Err(ProtocolError::Closed),
        Err(err) => Err(err),
    }
}

/// Write bytes, mapping I/O errors into the typed taxonomy.
pub fn write_all<W: Write>(stream: &mut W, bytes: &[u8]) -> Result<(), ProtocolError> {
    stream.write_all(bytes).map_err(|source| ProtocolError::Io {
        what: "writing handshake",
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accept_value_matches_rfc6455_example() {
        let key = base64::decode("dGhlIHNhbXBsZSBub25jZQ==").unwrap_or_default();
        let mut nonce = [0u8; 16];
        if key.len() == 16 {
            nonce.copy_from_slice(&key);
        }
        assert_eq!(accept_value(&nonce), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    #[test]
    fn valid_request_parses() {
        // lumen-allow-literal: protocol-name request path — wire vocabulary, not product identity
        let request = "GET /lumen-control HTTP/1.1\r\n\
                       Host: 127.0.0.1:4355\r\n\
                       Upgrade: WebSocket\r\n\
                       Connection: keep-alive, Upgrade\r\n\
                       Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                       Sec-WebSocket-Version: 13\r\n\r\n";
        match validate_head(request) {
            Ok(upgrade) => {
                // lumen-allow-literal: protocol-name request path — wire vocabulary, not product identity
                assert_eq!(upgrade.path, "/lumen-control");
                assert_eq!(base64::encode(&upgrade.key), "dGhlIHNhbXBsZSBub25jZQ==");
            }
            Err(reason) => unreachable!("valid request rejected: {reason}"),
        }
    }

    #[test]
    fn dirty_requests_are_stable_reason_tokens() {
        let good = |extra: &str| {
            format!(
                "GET / HTTP/1.1\r\nHost: h\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
                 Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n{extra}\r\n"
            )
        };
        let cases: &[(&str, &str)] = &[
            ("POST / HTTP/1.1\r\nHost: h\r\n\r\n", "method-not-get"),
            ("GET / HTTP/1.0\r\nHost: h\r\n\r\n", "not-http-1-1"),
            (
                "GET / HTTP/1.1\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n",
                "missing-host",
            ),
            (
                "GET / HTTP/1.1\r\nHost: h\r\nConnection: Upgrade\r\n\r\n",
                "missing-upgrade-websocket",
            ),
            (
                "GET / HTTP/1.1\r\nHost: h\r\nUpgrade: websocket\r\n\r\n",
                "missing-connection-upgrade",
            ),
            (
                "GET / HTTP/1.1\r\nHost: h\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
                 Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n",
                "missing-websocket-version",
            ),
            (
                "GET / HTTP/1.1\r\nHost: h\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
                 Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 8\r\n\r\n",
                "unsupported-websocket-version",
            ),
            (
                "GET / HTTP/1.1\r\nHost: h\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
                 Sec-WebSocket-Key: c2hvcnQ=\r\nSec-WebSocket-Version: 13\r\n\r\n",
                "bad-sec-websocket-key",
            ),
        ];
        for (request, expected_reason) in cases {
            let reason = validate_head(request).err().unwrap_or_default();
            assert_eq!(reason, *expected_reason, "request {request:?}");
        }
        // Sanity: the empty good case parses.
        assert!(validate_head(&good("")).is_ok());
    }

    #[test]
    fn client_roundtrip_against_server_validation() {
        let key = [7u8; 16];
        let request = client_request("/control", &key, "127.0.0.1:1");
        let text = String::from_utf8_lossy(&request).into_owned();
        match validate_head(&text) {
            Ok(upgrade) => {
                let accept = accept_value(&upgrade.key);
                let response = String::from_utf8_lossy(&response_101(&accept)).into_owned();
                assert!(validate_server_response(&response, &key).is_ok());
                // A doctored accept must fail.
                let doctored =
                    response.replace(&accept, "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
                assert_ne!(doctored, response);
                assert!(validate_server_response(&doctored, &key).is_err());
            }
            Err(reason) => unreachable!("self-request rejected: {reason}"),
        }
    }
}
