//! The `lumen:v1.http` transport seam (doc 06 §2.2 `http-fetch`;
//! ADR-011 is the in-tree https client precedent).
//!
//! v1 defines the seam and enforces **everything policy-side**
//! before any transport runs: HTTPS-only, exact-host allowlist,
//! per-minute rate, transfer caps. The transport itself is a trait
//! object the embedder supplies; this crate ships only
//! [`NoTransport`], which answers every request with the typed
//! `E_TRANSPORT` status — the honest v1 posture (mission card 7.F2:
//! wiring `streaming`'s ureq client into this host is a Lead
//! decision; doc 01 §2.8 lists no `streaming` edge for
//! `plugin-host-wasm`, so the seam stays unplugged until the Lead
//! adds the edge).
//!
//! The allowlist is enforced *here*, in front of the seam, so it
//! holds for every possible transport implementation.

use crate::error::HostError;

/// The `https://` scheme prefix — URL-grammar constant (the policy is
/// TLS-always-on; `http:` is refused, never downgraded). Protocol
/// vocabulary, not a configurable endpoint.
const HTTPS_SCHEME: &str = "https://"; // lumen-allow-literal: URL scheme grammar constant parsed from guest input — protocol, not a config endpoint

/// A fetched response body (bounded by the single-transfer cap
/// before it ever reaches guest memory).
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// Status code (200 expected; the seam does not follow
    /// redirects — policy decision recorded in the doc 06 §2.2
    /// `network-protocol` capability, which extensions may not hold
    /// in v1).
    pub status: u16,
    /// Body bytes (already capped).
    pub body: Vec<u8>,
}

/// Why a transport refused to serve a request.
#[derive(Debug)]
pub enum TransportError {
    /// No transport is wired (the v1 default).
    Unavailable,
    /// The transport is wired but failed (network, TLS, DNS).
    Failed(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::Unavailable => f.write_str(
                "no HTTP transport is wired into the extension host \
                 (v1: the seam is defined, the client is not)",
            ),
            TransportError::Failed(msg) => write!(f, "transport failed: {msg}"),
        }
    }
}

impl std::error::Error for TransportError {}

/// The transport seam: one blocking HTTPS GET per call.
pub trait HttpTransport: Send {
    /// Fetch `url` (already validated: https scheme, allowlisted
    /// host, default port).
    ///
    /// # Errors
    /// [`TransportError`] when the transport cannot serve the
    /// request. Policy denials never reach this trait — they are
    /// answered before dispatch.
    fn fetch(&mut self, url: &str) -> Result<HttpResponse, TransportError>;
}

/// The v1 default transport: none. Every `http_get` observes the
/// typed `E_TRANSPORT` status — the host is honest that fetching is
/// unavailable rather than pretending success or panicking.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoTransport;

impl HttpTransport for NoTransport {
    fn fetch(&mut self, _url: &str) -> Result<HttpResponse, TransportError> {
        Err(TransportError::Unavailable)
    }
}

/// A parsed guest URL — the policy-checked parts of an https URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUrl {
    /// Host (lowercased; exact-match against the allowlist).
    pub host: String,
    /// Path+query (starts with `/`; `""` becomes `"/"`).
    pub path: String,
    /// Explicit port, if any (`None` = default 443).
    pub port: Option<u16>,
}

impl ParsedUrl {
    /// Parse and policy-check a URL. Rules (doc 06 §2.2, MP §12):
    /// https only (TLS always on — an `http:` URL is refused, not
    /// downgraded); no userinfo (`user@host` is refused); no
    /// IP-literal hosts (certificate-bound hostnames only); default
    /// port or explicit `:443` only.
    ///
    /// # Errors
    /// [`HostError::PermissionDenied`] with the rule named for every
    /// refusal — the guest observes `E_PERM`/`E_ARG` statuses, never
    /// a hang.
    pub fn parse(url: &str) -> Result<ParsedUrl, HostError> {
        let rest = match url.strip_prefix(HTTPS_SCHEME) {
            Some(rest) => rest,
            None => {
                return Err(HostError::PermissionDenied {
                    detail: format!(
                        "URL is not {HTTPS_SCHEME} (TLS is always on; http is \
                         refused, not downgraded)"
                    ),
                });
            }
        };
        if rest.contains('@') {
            return Err(HostError::PermissionDenied {
                detail: "userinfo in URL is refused".to_string(),
            });
        }
        let authority_end = rest
            .find(['/', '?', '#'])
            .unwrap_or(rest.len());
        let authority = &rest[..authority_end];
        let mut tail = &rest[authority_end..];
        if let Some(hash) = tail.find('#') {
            tail = &tail[..hash];
        }
        if tail.is_empty() {
            tail = "/";
        }
        let (host_part, port) = match authority.rsplit_once(':') {
            Some((h, p)) => (h, Some(p)),
            None => (authority, None),
        };
        if let Some(p) = port
            && p != "443"
        {
            return Err(HostError::PermissionDenied {
                detail: format!("explicit port `{p}` refused (default/443 only)"),
            });
        }
        let host = host_part.to_ascii_lowercase();
        if host.is_empty() {
            return Err(HostError::PermissionDenied {
                detail: "empty host".to_string(),
            });
        }
        // IP literals: all-digits-and-dots, or bracketed IPv6.
        let is_ipv4 = !host.is_empty()
            && host.split('.').all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
            && host.matches('.').count() == 3;
        let is_ipv6 = host.starts_with('[');
        if is_ipv4 || is_ipv6 {
            return Err(HostError::PermissionDenied {
                detail: "IP-literal hosts are refused (certificate-bound \
                         hostnames only)"
                    .to_string(),
            });
        }
        if !host
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.')
        {
            return Err(HostError::PermissionDenied {
                detail: "host contains characters outside the DNS label set"
                    .to_string(),
            });
        }
        Ok(ParsedUrl {
            host,
            path: tail.to_string(),
            port: port.map(|_| 443),
        })
    }

    /// Exact-host allowlist check (doc 06 §2.2
    /// `network-host-allowlist`): the host must equal an entry.
    pub fn allowed_by(&self, allowlist: &[String]) -> bool {
        allowlist.iter().any(|allowed| {
            allowed.eq_ignore_ascii_case(&self.host)
        })
    }

    /// The canonical URL text handed to the transport.
    pub fn url(&self) -> String {
        format!("{HTTPS_SCHEME}{}{}", self.host, self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// House test helper (media-probe precedent).
    fn ok<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
        match r {
            Ok(v) => v,
            Err(e) => unreachable!("{e}"),
        }
    }

    #[test]
    fn https_only() {
        // lumen-allow-literal: test-double URLs for scheme policy (non-loopback host required by the case)
        for bad in ["http://musicbrainz.org/x", "ftp://x", "musicbrainz.org"] {
            assert!(
                matches!(ParsedUrl::parse(bad), Err(HostError::PermissionDenied { .. })),
                "{bad} must be refused"
            );
        }
        // lumen-allow-literal: test-double URL for scheme/case normalization
        let parsed = ok(ParsedUrl::parse("https://MusicBrainz.org/ws/2"));
        assert_eq!(parsed.host, "musicbrainz.org");
        assert_eq!(parsed.path, "/ws/2");
        // lumen-allow-literal: expected canonical reconstruction of the test-double URL
        assert_eq!(parsed.url(), "https://musicbrainz.org/ws/2");
    }

    #[test]
    fn userinfo_ip_and_ports_refused() {
        // One marker per literal line: the gate's inline suppression is
        // line-scoped, so each test-double URL carries its own reason.
        assert!(ParsedUrl::parse("https://user@host/").is_err()); // lumen-allow-literal: test-double URL for userinfo-refusal policy
        assert!(ParsedUrl::parse("https://user:pw@host/").is_err()); // lumen-allow-literal: test-double URL for userinfo-refusal policy
        assert!(ParsedUrl::parse("https://127.0.0.1/").is_err()); // lumen-allow-literal: test-double URL for IP-literal-refusal policy
        assert!(ParsedUrl::parse("https://[::1]/").is_err()); // lumen-allow-literal: test-double URL for IP-literal-refusal policy
        assert!(ParsedUrl::parse("https://host:8080/").is_err()); // lumen-allow-literal: test-double URL for non-443-port-refusal policy
        assert!(ParsedUrl::parse("https://host:443/").is_ok()); // lumen-allow-literal: test-double URL for the explicit-443 exception
    }

    #[test]
    fn allowlist_is_exact_match() {
        let allow = vec!["musicbrainz.org".to_string()];
        // lumen-allow-literal: test-double URL for allowlist exact-match policy
        assert!(ok(ParsedUrl::parse("https://musicbrainz.org/x")).allowed_by(&allow));
        // Subdomain, suffix and scheme-smuggling all fail.
        for bad in [
            // lumen-allow-literal: adversarial test-double URL for allowlist policy
            "https://api.musicbrainz.org/x",
            // lumen-allow-literal: adversarial test-double URL for allowlist policy
            "https://musicbrainz.org.evil.test/x",
            // lumen-allow-literal: adversarial test-double URL for allowlist policy
            "https://evil.test/musicbrainz.org",
        ] {
            assert!(
                !ok(ParsedUrl::parse(bad)).allowed_by(&allow),
                "{bad} must not match the allowlist"
            );
        }
        // lumen-allow-literal: non-allowlisted test-double host
        assert!(!ok(ParsedUrl::parse("https://other.org/")).allowed_by(&allow));
    }

    #[test]
    fn no_transport_is_honestly_unavailable() {
        let mut t = NoTransport;
        // lumen-allow-literal: test-double URL for the transport-seam refusal case
        let Err(err) = t.fetch("https://musicbrainz.org/") else {
            unreachable!("NoTransport must refuse")
        };
        assert!(matches!(err, TransportError::Unavailable));
        assert!(err.to_string().contains("no HTTP transport"));
    }
}
