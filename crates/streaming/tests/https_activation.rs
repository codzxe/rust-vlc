//! The ADR-011 activation proof (issue #39; Lead ratification
//! 2026-11-09): https is admitted, classifies to the same access
//! strategy as http, and rides the identical `open_http` composition.
//!
//! What this suite proves, honestly:
//!
//! 1. **Routing** — [`SchemeRegistry::classify`] maps `https://`
//!    sources to [`AccessKind::Http`] (the ureq transport's
//!    strategy): the pre-ratification `HttpsNotAdmitted` refusal
//!    vocabulary no longer exists in the dispatch surface at all, so
//!    any scheme-specific https refusal is now a compile error, not a
//!    runtime state.
//! 2. **Composition** — an `https://`-shaped source opened through
//!    [`SchemeRegistry::open_http`] (mock transport injected via
//!    [`SchemeRegistry::with_fetch`]) runs the same probe →
//!    ranged-resource → `SourceStream` pipeline an http source runs,
//!    serving byte-identical media.
//!
//! What it cannot prove in-tree: a real TLS handshake. The mock
//! testkit is a plaintext socket by design and this sandbox has no
//! network egress, so the rustls (ring 0.17.14) round-trip is
//! [VERIFY: needs a networked CI lane]. The transport's TLS *config*
//! is pinned separately (always-on verification, webpki roots,
//! rustls provider) by the in-crate test
//! `client.rs::agent_config_pins_transport_safety`, and a real HTTPS
//! round-trip was measured out-of-band when the dependency was
//! admitted (record `docs/dependencies/records/ureq-3.5.0.md` R10).

use std::sync::Arc;

use plugin_api::SourceStream;
use streaming::fetch::HttpFetch;
use streaming::mockhttp::{MockConfig, MockServer};
use streaming::ranged::{NoRangePolicy, RangedConfig};
use streaming::schemes::{AccessKind, SchemeError, SchemeRegistry};

/// https classifies to the http access strategy — the ADR-011
/// admission is unconditional post-ratification, and a malformed
/// https URL refuses through the *shared* port parse exactly as http
/// does (no scheme-specific refusal arm remains anywhere).
#[test]
fn https_routes_to_the_http_access_strategy() {
    for source in [
        "https://cdn.example.org/live/master.m3u8",
        "HTTPS://Example.com",
        "https://cdn.example.org:8443/seg/x.mp4?range=1",
    ] {
        match SchemeRegistry::classify(source) {
            Ok(AccessKind::Http { url }) => {
                assert_eq!(url.scheme, "https", "{source:?}");
            }
            other => unreachable!("admitted https must classify: {other:?}"),
        }
    }
    match SchemeRegistry::classify("https://cdn.example.org/live/master.m3u8") {
        Ok(AccessKind::Http { url }) => {
            assert_eq!(url.port, 443, "the scheme default port is explicit");
            assert_eq!(url.path_and_query, "/live/master.m3u8");
        }
        other => unreachable!("admitted https must classify: {other:?}"),
    }
    match SchemeRegistry::classify("https://") {
        Err(SchemeError::Malformed { .. }) => {}
        other => unreachable!("an empty authority must be malformed: {other:?}"),
    }
}

/// The full access-plugin composition over an https-shaped source:
/// classify → `open_http` → probe → ranged windows → `SourceStream`
/// reads, byte-identical to the served body. The mock transport is
/// plaintext, so this proves the *routing and composition*; the TLS
/// handshake itself is the [VERIFY: needs a networked CI lane] lane
/// (see the module docs).
#[test]
fn https_source_rides_the_open_http_composition() {
    let body: Vec<u8> = (0..u16::MAX).map(|i| (i % 251) as u8).collect();
    let server = MockServer::spawn(MockConfig::body(body.clone()))
        .unwrap_or_else(|e| unreachable!("spawn: {e}"));
    let registry = SchemeRegistry::with_fetch(Arc::new(server.client()) as Arc<dyn HttpFetch>);

    // The mock's URL is plaintext http; the *source reference* under
    // test is https-shaped — exactly the dispatch difference this
    // activation owns.
    let https_source = server.url("media.bin").replacen("http://", "https://", 1);
    let url = match SchemeRegistry::classify(&https_source) {
        Ok(AccessKind::Http { url }) => url,
        other => unreachable!("https must classify: {other:?}"),
    };

    // 1 KiB chunks over a ~65 KiB body: the ranged path genuinely
    // fetches multiple windows (the request log below pins that).
    let config = RangedConfig {
        window_bytes: 4096,
        chunk_bytes: 1024,
        no_range: NoRangePolicy::RequireRange,
    };
    let mut stream = registry
        .open_http(&url, config)
        .unwrap_or_else(|e| unreachable!("open: {e}"));
    assert_eq!(stream.size(), Some(body.len() as u64));

    let mut got = Vec::new();
    loop {
        let chunk = stream.read(1024).unwrap_or_else(|e| unreachable!("{e}"));
        if chunk.is_empty() {
            break; // clean, sticky EOF at the known length
        }
        got.extend_from_slice(&chunk);
    }
    assert_eq!(got, body, "the https-sourced stream serves the body");

    let requests = server.requests();
    assert!(
        requests.len() >= 30,
        "dozens of range requests over a {}-byte body (got {})",
        body.len(),
        requests.len()
    );
}
