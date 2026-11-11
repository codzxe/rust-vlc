//! SSDP/UPnP — the honest feasibility verdict + the provable subset
//! (card 7.D1, issue #42). `[VERIFY]` recorded 2026-11-10.
//!
//! # The verdict (five findings)
//!
//! **1. SSDP is UDP multicast; the in-tree https client can never
//! serve it.** UPnP Device Architecture 1.1's discovery step is
//! `M-SEARCH` multicast to `239.255.255.250:1900` with **unicast UDP
//! responses**. The admitted HTTP client (ADR-011: ureq 3.5.0 +
//! rustls over **TCP**) has no UDP path by design — doc 01 §2.4 keeps
//! network protocols behind ports, and the https client serves the
//! *post-discovery* steps only: the device DESCRIPTION fetch and the
//! UPnP AV ContentDirectory SOAP calls are plain HTTP/1.1 over TCP
//! unicast. Using it for discovery is not blocked by effort but by
//! transport semantics; no adapter can bridge it.
//!
//! **2. Native feasibility: PROVEN FEASIBLE, std-only.**
//! `std::net::UdpSocket` carries the whole SSDP surface — multicast
//! send, group join (`join_multicast_v4`), read timeouts. Live
//! evidence on THIS sandbox (2026-11-10, recorded in the worklog):
//! ephemeral-socket send to `239.255.255.250:1900` accepted by the
//! kernel; bind + group join succeed; **zero SSDP responders exist in
//! this sandbox** (1.5 s M-SEARCH window, 0 responses), so an
//! end-to-end discovery against real hardware is `[VERIFY]` — not
//! observable here, and not claimed.
//!
//! **3. What is implemented is exactly what is provable.** The
//! M-SEARCH codec (byte-exact, UDA 1.1 §1.2.3 header set), the
//! dirty-world response parser, and the socket searcher — whose
//! transport logic is proven with **real UDP round-trips** against an
//! in-process loopback responder (`tests/ssdp.rs`: real sockets, real
//! wire bytes, real M-SEARCH conformance checks). The multicast-LAN
//! behavior (group joins across routers, MX random-delay response
//! storms, NAT) stays `[VERIFY]`.
//!
//! **4. Web: BLOCKED, typed.** Browsers expose no UDP API; the wasm
//! build compiles this lane into
//! [`UnavailableReason::BlockedOnWeb`](crate::error::UnavailableReason)
//! per doc 09 §10's blocked-capability policy. No gateway workaround
//! exists in tree; if one is ever wanted it is a `[PROPOSAL]` for the
//! Lead (a sidecar service relaying SSDP over the control protocol).
//!
//! **5. DLNA: same verdict by construction.** DLNA media
//! servers/renderers are discovered through the SAME SSDP exchange
//! (DLNA is a profile over UPnP AV); DLNA-specific browsing rides the
//! description/SOAP follow-up lane (finding 1). Nothing here claims
//! DLNA profile conformance.
//!
//! # Scope line for the description/SOAP lane `[PROPOSAL]`
//!
//! This crate is L2 and `streaming` (the ureq adapter) is L2 — a
//! `device-discovery → streaming` edge needs a Lead-ratified
//! `same_layer_pairs` entry (like `streaming->playlists`) or the fetch
//! must be composed at the host layer. v0 therefore stops at SSDP
//! responses (location URLs as data) — nothing fetches descriptions,
//! and nothing pretends to.

use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use crate::capability::Capability;
use crate::error::DiscoveryError;

/// The SSDP multicast group (UDA 1.1 §1: `239.255.255.250:1900`).
pub const SSDP_MULTICAST_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::new(239, 255, 255, 250));
/// The SSDP port (UDA 1.1 §1).
pub const SSDP_PORT: u16 = 1900;
/// The multicast TTL UDA 1.1 §1.2.2 mandates by default (site-local
/// scope, no off-site propagation).
pub const SSDP_TTL: u32 = 4;
/// The receive slice (keeps the wait loop responsive to the deadline
/// without busy-spinning).
const READ_SLICE: Duration = Duration::from_millis(200);
/// One datagram's read bound (D-3): larger SSDP responses are
/// kernel-truncated to this and fail the parse — counted, never a
/// crash, never unbounded growth.
const DATAGRAM_CAP: usize = 4096;
/// The header-count bound of one response (D-3).
const HEADER_CAP: usize = 64;
/// Field length bounds (D-3; spec-plausible generous ceilings).
const LOCATION_CAP: usize = 2048;
const FIELD_CAP: usize = 256;
/// The response-count ceiling (searches clamp to it — D-3).
const MAX_RESPONSES_CEILING: usize = 256;

/// Build the M-SEARCH request exactly as UDA 1.1 §1.2.3 specifies:
/// the five mandatory headers, CRLF lines, empty line terminator. The
/// `HOST` value is always the canonical group (`239.255.255.250:1900`)
/// — the spec fixes it there; the *destination* socket address is a
/// transport concern (multicast in production, any address in tests).
#[must_use = "pure message builder"]
pub fn build_msearch(mx: u32, search_target: &str) -> String {
    format!(
        "M-SEARCH * HTTP/1.1\r\n\
         HOST: 239.255.255.250:1900\r\n\
         MAN: \"ssdp:discover\"\r\n\
         MX: {mx}\r\n\
         ST: {search_target}\r\n\
         \r\n"
    )
}

/// One parsed `M-SEARCH` response (`HTTP/1.1 200 OK` + headers), as
/// data — nothing is fetched from `location`; it is a string the
/// host's open path can hand to the access layer later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsdpResponse {
    /// The device DESCRIPTION URL (UDA 1.1 §1.2.2 `LOCATION`). Non-empty,
    /// control-char-free, ≤ [`LOCATION_CAP`] bytes.
    pub location: String,
    /// The search target this response answers (`ST`).
    pub search_target: String,
    /// The unique service name (`USN`) — the dedup key.
    pub usn: String,
    /// The advertiser's lifetime in seconds (`CACHE-CONTROL:
    /// max-age=…`) when present and well-formed.
    pub max_age: Option<u64>,
    /// The advertising stack's signature (`SERVER`) when present.
    pub server: Option<String>,
}

/// Parse one M-SEARCH response datagram (dirty-world tolerant: header
/// names are case-insensitive, optional headers may be absent or
/// malformed — the malformed *optional* value degrades to `None`, the
/// malformed *required* value is the typed
/// [`DiscoveryError::MalformedResponse`]).
///
/// # Errors
/// [`DiscoveryError::MalformedResponse`] naming the exact defect.
pub fn parse_search_response(bytes: &[u8]) -> Result<SsdpResponse, DiscoveryError> {
    let malformed = |detail: String| DiscoveryError::MalformedResponse {
        protocol: "ssdp",
        detail,
    };
    let text =
        std::str::from_utf8(bytes).map_err(|_| malformed("response is not UTF-8".to_string()))?;
    let mut lines = text.split('\n').map(|line| line.trim_end_matches('\r'));
    let status = lines.next().unwrap_or("");
    // Status line: `HTTP/1.1 200 OK` — tolerate 1.0 stacks and any
    // reason phrase, require the 200.
    let status_ok = status.starts_with("HTTP/1.")
        && status
            .split_whitespace()
            .nth(1)
            .is_some_and(|code| code == "200");
    if !status_ok {
        return Err(malformed(format!("bad status line: {status:?}")));
    }

    let mut location: Option<String> = None;
    let mut search_target: Option<String> = None;
    let mut usn: Option<String> = None;
    let mut max_age: Option<u64> = None;
    let mut server: Option<String> = None;
    let mut headers_seen = 0usize;
    for line in lines {
        if line.is_empty() {
            continue; // trailing terminator / blank padding
        }
        headers_seen += 1;
        if headers_seen > HEADER_CAP {
            return Err(malformed(format!("more than {HEADER_CAP} headers")));
        }
        let Some((name, value)) = line.split_once(':') else {
            // A colon-less line is dirt; skip it, don't fail the whole
            // response (the required-header check below still applies).
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        if value.len() > LOCATION_CAP {
            return Err(malformed(format!(
                "header {name} exceeds {LOCATION_CAP} bytes"
            )));
        }
        match name.as_str() {
            "location" => {
                if value.is_empty() || value.chars().any(|c| c.is_ascii_control() || c == ' ') {
                    return Err(malformed("LOCATION is empty or unprintable".to_string()));
                }
                location = Some(value.to_string());
            }
            "st" => {
                if value.is_empty() {
                    return Err(malformed("ST is empty".to_string()));
                }
                search_target = Some(value.to_string());
            }
            "usn" => {
                if value.is_empty() {
                    return Err(malformed("USN is empty".to_string()));
                }
                usn = Some(value.to_string());
            }
            // Optional: degrade to None on a malformed value.
            "cache-control" => {
                max_age = value
                    .split('=')
                    .nth(1)
                    .and_then(|n| n.trim().parse::<u64>().ok());
            }
            "server" if !value.is_empty() => {
                server = Some(value.to_string());
            }
            _ => {} // EXT, DATE, and vendor headers are not our vocabulary
        }
    }

    let required = |field: &str, value: Option<String>| match value {
        Some(v) if v.len() > FIELD_CAP => {
            Err(malformed(format!("{field} exceeds {FIELD_CAP} bytes")))
        }
        Some(v) => Ok(v),
        None => Err(malformed(format!("{field} is required (UDA 1.1 §1.2.2)"))),
    };
    Ok(SsdpResponse {
        location: required("LOCATION", location)?,
        search_target: required("ST", search_target)?,
        usn: required("USN", usn)?,
        max_age,
        server,
    })
}

/// Search tuning (all bounds clamped).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsdpSearchConfig {
    /// Where the M-SEARCH is sent — the canonical multicast group in
    /// production, any address in tests (the loopback responder).
    pub destination: SocketAddr,
    /// The MX header (devices answer within 0..=MX seconds, randomly —
    /// UDA 1.1 §1.2.3). Clamped to 1..=5.
    pub mx: u32,
    /// How many distinct devices (unique USNs) to collect before
    /// stopping. Clamped to 1..=[`MAX_RESPONSES_CEILING`].
    pub max_responses: usize,
    /// The receive window. Should cover `mx` seconds for
    /// spec-conformant responders (plus margin); clamped to
    /// 200 ms..=60 s.
    pub wait: Duration,
}

impl SsdpSearchConfig {
    /// The production default: the multicast group, MX 1, up to 32
    /// devices, a 1.5 s window (MX + 500 ms margin).
    #[must_use = "constructs the config"]
    pub fn new() -> SsdpSearchConfig {
        SsdpSearchConfig {
            destination: SocketAddr::new(SSDP_MULTICAST_ADDR, SSDP_PORT),
            mx: 1,
            max_responses: 32,
            wait: Duration::from_millis(1500),
        }
    }

    /// A config with explicit bounds, clamped (see the field docs).
    #[must_use = "constructs the config"]
    pub fn with_limits(
        destination: SocketAddr,
        mx: u32,
        max_responses: usize,
        wait: Duration,
    ) -> SsdpSearchConfig {
        SsdpSearchConfig {
            destination,
            mx: mx.clamp(1, 5),
            max_responses: max_responses.clamp(1, MAX_RESPONSES_CEILING),
            wait: wait.clamp(Duration::from_millis(200), Duration::from_secs(60)),
        }
    }
}

impl Default for SsdpSearchConfig {
    fn default() -> Self {
        SsdpSearchConfig::new()
    }
}

/// One search's outcome: the collected responses plus the honest
/// count of datagrams that failed the parse (skipped, never fatal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsdpSearchOutcome {
    /// Distinct devices (unique USNs), in arrival order.
    pub responses: Vec<SsdpResponse>,
    /// Datagrams that failed [`parse_search_response`].
    pub skipped_malformed: u32,
}

/// The SSDP searcher (native lane; the wasm build answers every call
/// with the typed blocked verdict).
#[derive(Debug, Clone)]
pub struct SsdpSearcher {
    config: SsdpSearchConfig,
}

impl SsdpSearcher {
    /// A searcher with explicit config.
    #[must_use = "constructs the searcher"]
    pub fn new(config: SsdpSearchConfig) -> SsdpSearcher {
        SsdpSearcher { config }
    }

    /// The config in effect.
    #[must_use = "pure accessor"]
    pub fn config(&self) -> &SsdpSearchConfig {
        &self.config
    }

    /// The capability probe: does this target have a usable UDP
    /// socket API at all?
    ///
    /// Native: one ephemeral socket bind (nothing is sent — the probe
    /// proves the API, not the network). Wasm: the typed
    /// [`UnavailableReason::BlockedOnWeb`] verdict (doc 09 §10).
    #[must_use = "probe outcome for the capability report"]
    pub fn probe() -> (Option<SsdpSearcher>, Capability) {
        #[cfg(target_family = "wasm")]
        {
            (
                None,
                Capability::Unavailable(crate::error::UnavailableReason::BlockedOnWeb {
                    detail: "SSDP needs UDP multicast; browsers expose no UDP API \
                             (doc 09 §10 blocked-capability policy)",
                }),
            )
        }
        #[cfg(not(target_family = "wasm"))]
        {
            match UdpSocket::bind("0.0.0.0:0") {
                Ok(_socket) => (
                    Some(SsdpSearcher::new(SsdpSearchConfig::new())),
                    Capability::Available,
                ),
                Err(err) => (
                    None,
                    Capability::from_probe_error(&DiscoveryError::Io {
                        context: "ssdp probe bind",
                        kind: err.kind(),
                        detail: err.to_string(),
                    }),
                ),
            }
        }
    }

    /// Run one bounded M-SEARCH: send, then collect unicast responses
    /// until the window closes or `max_responses` distinct USNs
    /// arrived. Malformed datagrams are skipped and counted (dirty
    /// world: one bad advertiser never poisons the search).
    ///
    /// # Errors
    /// Typed [`DiscoveryError`] — socket bind/send/option failures
    /// ([`DiscoveryError::Io`]) or the typed web-blocked verdict on
    /// wasm. An empty result is `Ok` with zero responses: "no
    /// devices" is data, not an error (MP §8).
    pub fn search(&self, search_target: &str) -> Result<SsdpSearchOutcome, DiscoveryError> {
        #[cfg(target_family = "wasm")]
        {
            let _ = search_target;
            Err(DiscoveryError::Unavailable {
                reason: crate::error::UnavailableReason::BlockedOnWeb {
                    detail: "SSDP needs UDP multicast; browsers expose no UDP API \
                             (doc 09 §10 blocked-capability policy)",
                },
            })
        }
        #[cfg(not(target_family = "wasm"))]
        {
            self.search_native(search_target)
        }
    }

    #[cfg(not(target_family = "wasm"))]
    fn search_native(&self, search_target: &str) -> Result<SsdpSearchOutcome, DiscoveryError> {
        let socket = UdpSocket::bind("0.0.0.0:0").map_err(|err| DiscoveryError::Io {
            context: "ssdp search bind",
            kind: err.kind(),
            detail: err.to_string(),
        })?;
        // Site-local multicast scope (UDA 1.1 §1.2.2). Harmless for
        // unicast test destinations.
        socket
            .set_multicast_ttl_v4(SSDP_TTL)
            .map_err(|err| DiscoveryError::Io {
                context: "ssdp multicast ttl",
                kind: err.kind(),
                detail: err.to_string(),
            })?;
        let request = build_msearch(self.config.mx, search_target);
        socket
            .send_to(request.as_bytes(), self.config.destination)
            .map_err(|err| DiscoveryError::Io {
                context: "ssdp m-search send",
                kind: err.kind(),
                detail: err.to_string(),
            })?;
        socket
            .set_read_timeout(Some(READ_SLICE))
            .map_err(|err| DiscoveryError::Io {
                context: "ssdp read timeout",
                kind: err.kind(),
                detail: err.to_string(),
            })?;

        let deadline = Instant::now() + self.config.wait;
        let mut seen_usns: HashSet<String> = HashSet::new();
        let mut responses = Vec::new();
        let mut skipped_malformed = 0_u32;
        while responses.len() < self.config.max_responses && Instant::now() < deadline {
            let mut buf = [0_u8; DATAGRAM_CAP];
            match socket.recv_from(&mut buf) {
                Ok((len, _src)) => match parse_search_response(&buf[..len]) {
                    Ok(response) => {
                        if seen_usns.insert(response.usn.clone()) {
                            responses.push(response);
                        }
                    }
                    Err(_typed) => {
                        skipped_malformed += 1;
                    }
                },
                Err(err)
                    if err.kind() == std::io::ErrorKind::WouldBlock
                        || err.kind() == std::io::ErrorKind::TimedOut =>
                {
                    // Slice elapsed: re-check the deadline, no busy spin.
                }
                Err(err) => {
                    return Err(DiscoveryError::Io {
                        context: "ssdp response recv",
                        kind: err.kind(),
                        detail: err.to_string(),
                    });
                }
            }
        }
        Ok(SsdpSearchOutcome {
            responses,
            skipped_malformed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixture URLs live on the RFC 5737 TEST-NET-1 range (and a bare
    // single-label host): protocol fixtures, never shipped endpoints.
    // lumen-allow-literal: spec-fixture URL on the RFC 5737 TEST-NET-1 reserved range — never a shipped endpoint.
    const SPEC_LOCATION: &str = "http://192.0.2.7:8200/desc.xml";
    // lumen-allow-literal: dirty-world fixture URL on the RFC 5737 TEST-NET-1 reserved range — never a shipped endpoint.
    const DIRTY_LOCATION: &str = "http://192.0.2.9/d.xml";
    // lumen-allow-literal: minimal fake-host fixture URL — never a shipped endpoint.
    const FAKE_LOCATION: &str = "http://h/d";

    fn spec_response() -> String {
        format!(
            "HTTP/1.1 200 OK\r\n\
         CACHE-CONTROL: max-age=1800\r\n\
         DATE: Tue, 10 Nov 2026 12:00:00 GMT\r\n\
         EXT:\r\n\
         LOCATION: {SPEC_LOCATION}\r\n\
         SERVER: Linux/6.1 UPnP/1.1 MiniDLNA/1.3.2\r\n\
         ST: upnp:rootdevice\r\n\
         USN: uuid:2fac1234-31f8-11b4-a222-08002b34c003::upnp:rootdevice\r\n\
         CONTENT-LENGTH: 0\r\n\
         \r\n"
        )
    }

    #[test]
    fn msearch_bytes_match_the_uda_shape() {
        let msg = build_msearch(1, "ssdp:all");
        let expected = "M-SEARCH * HTTP/1.1\r\n\
                        HOST: 239.255.255.250:1900\r\n\
                        MAN: \"ssdp:discover\"\r\n\
                        MX: 1\r\n\
                        ST: ssdp:all\r\n\
                        \r\n";
        assert_eq!(msg, expected);
        // The HOST header is the canonical group even when the
        // destination is a test address (spec fixes the value).
        assert!(msg.contains("HOST: 239.255.255.250:1900"));
    }

    #[test]
    fn spec_shaped_response_parses_every_field() {
        let parsed = match parse_search_response(spec_response().as_bytes()) {
            Ok(r) => r,
            Err(e) => unreachable!("spec fixture must parse: {e}"),
        };
        assert_eq!(parsed.location, SPEC_LOCATION);
        assert_eq!(parsed.search_target, "upnp:rootdevice");
        assert!(parsed.usn.starts_with("uuid:"));
        assert_eq!(parsed.max_age, Some(1800));
        assert_eq!(
            parsed.server.as_deref(),
            Some("Linux/6.1 UPnP/1.1 MiniDLNA/1.3.2")
        );
    }

    #[test]
    fn parse_is_dirty_world_tolerant() {
        // Lower-case headers + LF-only endings + unknown headers.
        let dirty = format!(
            "HTTP/1.1 200 OK\nlocation: {DIRTY_LOCATION}\nst: MediaServer\n\
                     usn: uuid:dirty\nx-vendor: anything\n\n"
        );
        let parsed = match parse_search_response(dirty.as_bytes()) {
            Ok(r) => r,
            Err(e) => unreachable!("dirty-but-valid must parse: {e}"),
        };
        assert_eq!(parsed.location, DIRTY_LOCATION);
        assert_eq!(parsed.max_age, None);

        // Malformed optional CACHE-CONTROL degrades to None.
        let bad_age = format!(
            "HTTP/1.1 200 OK\r\nCACHE-CONTROL: no-cache\r\n\
                       LOCATION: {FAKE_LOCATION}.xml\r\nST: s\r\nUSN: u\r\n\r\n"
        );
        match parse_search_response(bad_age.as_bytes()) {
            Ok(r) => assert_eq!(r.max_age, None),
            Err(e) => unreachable!("optional malformation degrades: {e}"),
        }
    }

    #[test]
    fn missing_required_headers_are_typed_errors() {
        let no_location = b"HTTP/1.1 200 OK\r\nST: s\r\nUSN: u\r\n\r\n" as &[u8];
        let no_st = format!("HTTP/1.1 200 OK\r\nLOCATION: {FAKE_LOCATION}\r\nUSN: u\r\n\r\n");
        let no_usn = format!("HTTP/1.1 200 OK\r\nLOCATION: {FAKE_LOCATION}\r\nST: s\r\n\r\n");
        for (name, bytes) in [
            ("no location", no_location),
            ("no st", no_st.as_bytes()),
            ("no usn", no_usn.as_bytes()),
        ] {
            match parse_search_response(bytes) {
                Err(DiscoveryError::MalformedResponse { detail, .. }) => {
                    assert!(detail.contains("required"), "{name}: {detail}");
                }
                other => unreachable!("{name} must be a typed error, got {other:?}"),
            }
        }
    }

    #[test]
    fn bad_status_lines_and_encodings_are_typed_errors() {
        assert!(parse_search_response(b"NOTIFY * HTTP/1.1\r\n\r\n").is_err());
        assert!(parse_search_response(b"HTTP/1.1 500 Oops\r\n\r\n").is_err());
        assert!(parse_search_response(&[0xff, 0xfe, 0x00]).is_err());
    }

    #[test]
    fn config_clamps_to_spec_plausible_bounds() {
        let cfg = SsdpSearchConfig::with_limits(
            SocketAddr::new(SSDP_MULTICAST_ADDR, SSDP_PORT),
            99,
            100_000,
            Duration::from_secs(999),
        );
        assert_eq!(cfg.mx, 5);
        assert_eq!(cfg.max_responses, MAX_RESPONSES_CEILING);
        assert_eq!(cfg.wait, Duration::from_secs(60));
        assert_eq!(
            SsdpSearchConfig::with_limits(
                SocketAddr::new(SSDP_MULTICAST_ADDR, SSDP_PORT),
                0,
                0,
                Duration::ZERO
            )
            .mx,
            1
        );
    }

    #[test]
    fn constants_are_the_spec_values() {
        assert_eq!(SSDP_MULTICAST_ADDR.to_string(), "239.255.255.250");
        assert_eq!(SSDP_PORT, 1900);
        assert_eq!(SSDP_TTL, 4, "UDA 1.1 site-local default");
    }
}
