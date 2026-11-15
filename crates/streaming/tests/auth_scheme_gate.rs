//! The auth wrapper's scheme gate (AUD-07 — the 6.H1 F1 residual),
//! pinned over the mock testkit's real sockets.
//!
//! [`AuthFetch::follow`](streaming::auth::AuthFetch) resolves
//! `Location` targets **manually** (challenge-first: the wrapper must
//! know the URL a challenge actually came from), so the main client's
//! unconditional https→http refusal never engages for it — these
//! tests pin that the wrapper refuses the downgrade itself, with the
//! transport's own typed error
//! ([`NetworkError::CrossSchemeRedirect`](streaming::error::NetworkError)),
//! so Basic/Digest credentials cannot ride a plaintext hop.
//!
//! The mock testkit is a plaintext socket regardless of the URL
//! scheme — exactly the shape that lets an https-labeled origin
//! express the downgrade `Location` over the wire (same pinning
//! strategy as `client.rs::https_to_http_redirect_downgrade_is_refused_not_followed`
//! and `tests/https_activation.rs`).
//!
//! The same-origin/cross-host trust boundary (credentials re-attach
//! only on allowed-host hops) is pinned in `src/auth.rs`'s in-file
//! tests (`downgrade_hop_never_receives_credentials`,
//! `cross_host_hop_never_receives_credentials`) — the one-shot
//! `MockConfig` cannot express challenge-then-redirect, so the
//! challenge-shaped fixtures live there.

use streaming::auth::{AuthFetch, Credentials};
use streaming::error::NetworkError;
use streaming::fetch::{HttpFetch, HttpRequest};
use streaming::mockhttp::{MockConfig, MockServer};
use testkit::Must;

/// AUD-07 regression: an https→http redirect inside the auth
/// wrapper's **manual** follow loop is refused with the transport's
/// own typed error — never followed, never contacted — so Basic/Digest
/// credentials cannot ride a plaintext hop. The wrapper's inner
/// requests use `RedirectMode::Manual`, so without this gate the main
/// client's unconditional refusal never engages for it.
#[test]
fn scheme_downgrade_redirect_is_refused_in_auth_follow() {
    // The "downgraded" target: plaintext http, ready to serve.
    let target = MockServer::spawn(MockConfig::body(b"leaked-over-http".to_vec())).must();
    let mut origin_config = MockConfig::body(Vec::new());
    origin_config.absolute_redirect = Some(target.url("/media/leak.flac"));
    let origin = MockServer::spawn(origin_config).must();
    // The origin is fetched over an https-scheme URL (the testkit
    // transports plaintext regardless — only the *scheme decision*
    // under test is real).
    let https_start = origin.url("/start").replacen("http://", "https://", 1); // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint

    let auth = AuthFetch::new(origin.client()).with_credentials(Credentials::new("user", "pass"));
    let error = auth.fetch(HttpRequest::get(https_start)).err().must();

    assert!(
        matches!(error, NetworkError::CrossSchemeRedirect { hops: 1 }),
        "the downgrade must be the transport's typed refusal, got {error}"
    );
    assert_eq!(error.code(), "streaming.net.cross-scheme-redirect");
    assert!(!error.is_retryable(), "a policy refusal is not weather");
    // The probe reached only the https origin, credential-less …
    let logged = origin.requests();
    assert_eq!(logged.len(), 1, "the https origin answered exactly once");
    assert!(
        logged[0].header("authorization").is_none(),
        "credentials are withheld until challenged"
    );
    // … and the downgraded http target was never contacted, so no
    // Authorization header could have ridden a plaintext hop.
    assert!(
        target.requests().is_empty(),
        "the downgraded http target must never be contacted"
    );
}

/// AUD-07 (the gate's negative space): only the downgrade is refused.
/// An https→https hop (a same-host port change to a second testkit
/// socket) and an http→https upgrade both still follow through the
/// auth wrapper's manual loop, credentials and all.
#[test]
fn same_scheme_and_upgrade_redirects_still_follow_in_auth() {
    // https→https: the target is labeled https, the origin hop is
    // https — a plain cross-port hop on the same host.
    let target = MockServer::spawn(MockConfig::body(b"final bytes".to_vec())).must();
    let https_target = target
        .url("/media/final.flac")
        .replacen("http://", "https://", 1); // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
    let mut origin_config = MockConfig::body(Vec::new());
    origin_config.absolute_redirect = Some(https_target);
    let origin = MockServer::spawn(origin_config).must();
    let https_start = origin.url("/start").replacen("http://", "https://", 1); // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint

    let auth = AuthFetch::new(origin.client()).with_credentials(Credentials::new("user", "pass"));
    let response = auth.fetch(HttpRequest::get(https_start)).must();
    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"final bytes");
    assert_eq!(origin.requests().len(), 1);
    assert_eq!(target.requests().len(), 1, "the https target was followed");

    // http→https: the upgrade is always followable.
    let upgrade_target = MockServer::spawn(MockConfig::body(b"upgraded bytes".to_vec())).must();
    let https_upgrade = upgrade_target
        .url("/media/up.flac")
        .replacen("http://", "https://", 1); // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
    let mut origin_config = MockConfig::body(Vec::new());
    origin_config.absolute_redirect = Some(https_upgrade);
    let origin = MockServer::spawn(origin_config).must();

    let auth = AuthFetch::new(origin.client()).with_credentials(Credentials::new("user", "pass"));
    let response = auth.fetch(HttpRequest::get(origin.url("/start"))).must();
    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"upgraded bytes");
    assert_eq!(origin.requests().len(), 1);
    assert_eq!(
        upgrade_target.requests().len(),
        1,
        "the upgrade hop was followed"
    );
}
