//! Card 6.E1 end-to-end: HTTP authentication over the mock testkit.
//!
//! Every scenario runs a real [`AuthFetch`] → [`MockClient`] →
//! [`MockServer`] round trip over a localhost socket — challenge,
//! answer, and the typed wrong-password failure — and asserts on the
//! recorded request log (what actually went over the wire) plus the
//! redaction contract on the way out.

use streaming::auth::{AuthFetch, CredentialStore, Credentials, InMemoryCredentialStore};
use streaming::error::NetworkError;
use streaming::fetch::{HttpRequest, HttpFetch, Url};
use streaming::mockhttp::{MockAuth, MockConfig, MockServer};
use testkit::Must;

/// The fixed digest nonce of the RFC 2617 §3.5 example — the server
/// and the client meet on the classic known vector's challenge
/// parameters, and the mock *recomputes* the response digest (no
/// fixture string match).
const RFC2617_NONCE: &str = "dcd98b7102dd2f0e8b11d0f600bfb0c093";

/// Basic auth: the challenge → authorized fetch round trip. The probe
/// carries no credentials; the retry carries the exact `Authorization:
/// Basic` header; the body comes back.
#[test]
fn basic_challenge_round_trip() {
    let server = MockServer::spawn(MockConfig {
        auth: Some(MockAuth::basic("user", "pass", "Lumen Test")),
        routes: vec![(
            "/media/song.flac".to_string(),
            b"fake-flac-bytes".to_vec(),
        )],
        ..MockConfig::default()
    })
    .must();

    let auth = AuthFetch::new(server.client())
        .with_credentials(Credentials::new("user", "pass"));
    let response = auth
        .fetch(HttpRequest::get(server.url("/media/song.flac")))
        .must();

    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"fake-flac-bytes");

    let logged = server.requests();
    assert_eq!(logged.len(), 2, "probe, then authorized retry");
    assert!(
        logged[0].header("authorization").is_none(),
        "credentials are withheld until challenged"
    );
    assert_eq!(
        logged[1].header("authorization"),
        // base64("user:pass") — RFC 4648 vector territory.
        Some("Basic dXNlcjpwYXNz")
    );
}

/// Basic auth with the wrong password: the retry is rejected and the
/// failure is the *typed* wrong-password error, not a raw status.
#[test]
fn basic_wrong_password_is_typed_auth_failure() {
    let server = MockServer::spawn(MockConfig {
        auth: Some(MockAuth::basic("user", "pass", "Lumen Test")),
        routes: vec![("/x".to_string(), b"body".to_vec())],
        ..MockConfig::default()
    })
    .must();

    let auth = AuthFetch::new(server.client())
        .with_credentials(Credentials::new("user", "wrong-password"));
    let error = auth
        .fetch(HttpRequest::get(server.url("/x")))
        .err()
        .must();

    assert!(
        matches!(error, NetworkError::AuthFailed { scheme } if scheme.as_str() == "basic"),
        "wrong password must be the typed auth failure, got {error}"
    );
    assert_eq!(error.code(), "streaming.net.auth-failed");
    // The error text never carries the rejected credential.
    assert!(!error.to_string().contains("wrong-password"));
    // Probe + rejected retry, then give up (no third attempt).
    assert_eq!(server.requests().len(), 2);
}

/// A server that never challenges never sees a credential — the
/// challenge-first flow's core safety property.
#[test]
fn unchallenged_server_never_receives_credentials() {
    let server = MockServer::spawn(MockConfig {
        routes: vec![("/open".to_string(), b"public".to_vec())],
        ..MockConfig::default()
    })
    .must();

    let auth = AuthFetch::new(server.client())
        .with_credentials(Credentials::new("user", "pass"));
    let response = auth.fetch(HttpRequest::get(server.url("/open"))).must();
    assert_eq!(response.body, b"public");

    let logged = server.requests();
    assert_eq!(logged.len(), 1);
    assert!(logged[0].header("authorization").is_none());
}

/// URL userinfo (`http://user:pass@host/…`): accepted, stripped from
/// the request URL, and used as the credential source.
#[test]
fn url_userinfo_is_accepted_stripped_and_used() {
    let server = MockServer::spawn(MockConfig {
        auth: Some(MockAuth::basic("alice", "s3cret", "Lumen Test")),
        routes: vec![("/media/a.mp3".to_string(), b"bytes".to_vec())],
        ..MockConfig::default()
    })
    .must();

    let plain = Url::parse(&server.url("/media/a.mp3")).must();
    let userinfo_url = format!("http://alice:s3cret@{}/media/a.mp3", plain.authority());
    // No explicit credentials and no store — the userinfo is the source.
    let auth = AuthFetch::new(server.client());
    let response = auth.fetch(HttpRequest::get(userinfo_url)).must();
    assert_eq!(response.body, b"bytes");

    let logged = server.requests();
    assert_eq!(logged.len(), 2);
    // The wire never saw the userinfo: the probe went to the clean
    // path, the retry carried the Basic header.
    assert_eq!(logged[0].path, "/media/a.mp3");
    assert!(logged[0].header("authorization").is_none());
    assert!(
        logged[1]
            .header("authorization")
            .is_some_and(|value| value.starts_with("Basic "))
    );
}

/// The credential store: a URL-keyed lookup feeds the challenge
/// answer for any fetch to that origin.
#[test]
fn credential_store_feeds_the_challenge_answer() {
    let server = MockServer::spawn(MockConfig {
        auth: Some(MockAuth::basic("user", "pass", "Lumen Test")),
        routes: vec![("/a/x".to_string(), b"one".to_vec())],
        ..MockConfig::default()
    })
    .must();

    let store = InMemoryCredentialStore::new();
    let key = Url::parse(&server.url("/any/path/for/the/key")).must();
    store.put(&key, Credentials::new("user", "pass"));

    let auth = AuthFetch::new(server.client()).with_store(store);
    let response = auth.fetch(HttpRequest::get(server.url("/a/x"))).must();
    assert_eq!(response.body, b"one");
    assert_eq!(
        server.requests()[1].header("authorization"),
        Some("Basic dXNlcjpwYXNz")
    );
}

/// Digest auth (RFC 7616 MD5): the client answers the fixed-nonce
/// challenge with a server-verified response digest — the mock
/// recomputes it from the secret, so a wrong computation cannot pass.
#[test]
fn digest_challenge_round_trip() {
    let server = MockServer::spawn(MockConfig {
        auth: Some(MockAuth::digest(
            "Mufasa",
            "Circle Of Life",
            "testrealm@host.com",
            RFC2617_NONCE,
        )),
        routes: vec![("/dir/index.html".to_string(), b"<html/>".to_vec())],
        ..MockConfig::default()
    })
    .must();

    let auth = AuthFetch::new(server.client()).with_credentials(Credentials::new(
        "Mufasa",
        "Circle Of Life",
    ));
    let response = auth
        .fetch(HttpRequest::get(server.url("/dir/index.html")))
        .must();
    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"<html/>");

    let logged = server.requests();
    assert_eq!(logged.len(), 2);
    assert!(logged[0].header("authorization").is_none());
    let sent = logged[1].header("authorization").must();
    assert!(sent.starts_with("Digest "), "the retry is a Digest answer");
    assert!(sent.contains("username=\"Mufasa\""));
    assert!(sent.contains("realm=\"testrealm@host.com\""));
    assert!(sent.contains(&format!("nonce=\"{RFC2617_NONCE}\"")));
    assert!(sent.contains("uri=\"/dir/index.html\""));
    assert!(sent.contains("qop=auth"));
    assert!(sent.contains("response=\""));
}

/// Digest with the wrong password: the server recomputes with its own
/// secret, the digests disagree, and the client surfaces the typed
/// digest auth failure.
#[test]
fn digest_wrong_password_is_typed_auth_failure() {
    let server = MockServer::spawn(MockConfig {
        auth: Some(MockAuth::digest(
            "Mufasa",
            "Circle Of Life",
            "testrealm@host.com",
            RFC2617_NONCE,
        )),
        routes: vec![("/dir/index.html".to_string(), b"<html/>".to_vec())],
        ..MockConfig::default()
    })
    .must();

    let auth = AuthFetch::new(server.client())
        .with_credentials(Credentials::new("Mufasa", "wrong-password"));
    let error = auth
        .fetch(HttpRequest::get(server.url("/dir/index.html")))
        .err()
        .must();
    assert!(
        matches!(error, NetworkError::AuthFailed { scheme } if scheme.as_str() == "digest"),
        "wrong digest password must be the typed auth failure, got {error}"
    );
    // The secret never appears in the error text.
    assert!(!error.to_string().contains("wrong-password"));
}

/// Auth composes with redirect chains: the challenge comes from the
/// final hop (the auth layer follows manually so it knows the true
/// challenge URL), the retry carries the header on every same-origin
/// hop, and a cross-origin hop would never see it.
#[test]
fn auth_across_redirect_chain_answers_at_final_hop() {
    let server = MockServer::spawn(MockConfig {
        auth: Some(MockAuth::basic("user", "pass", "Lumen Test")),
        redirects: 2,
        routes: vec![("/r2".to_string(), b"protected".to_vec())],
        ..MockConfig::default()
    })
    .must();

    let auth = AuthFetch::new(server.client())
        .with_credentials(Credentials::new("user", "pass"));
    let response = auth.fetch(HttpRequest::get(server.url("/entry"))).must();
    assert_eq!(response.body, b"protected");

    let logged = server.requests();
    // Probe (manual following): entry → r1 → r2, no credentials
    // anywhere; r2 answers with the 401 challenge.
    assert_eq!(logged[0].path, "/entry");
    assert_eq!(logged[1].path, "/r1");
    assert_eq!(logged[2].path, "/r2");
    assert!(
        logged[0].header("authorization").is_none()
            && logged[1].header("authorization").is_none()
            && logged[2].header("authorization").is_none(),
        "the probe never carries credentials"
    );
    // Retry: every same-origin hop carries the Basic header; r2
    // (the challenger) accepts it.
    assert_eq!(logged[3].path, "/entry");
    assert_eq!(logged[3].header("authorization"), Some("Basic dXNlcjpwYXNz"));
    assert_eq!(logged[4].path, "/r1");
    assert_eq!(logged[4].header("authorization"), Some("Basic dXNlcjpwYXNz"));
    assert_eq!(logged[5].path, "/r2");
    assert_eq!(
        logged[5].header("authorization"),
        Some("Basic dXNlcjpwYXNz"),
        "same-origin hops are inside the credential trust boundary"
    );
}
