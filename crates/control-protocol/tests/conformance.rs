//! Conformance suite — card 7.J1 (doc 05 §5.4; doc 01 §2.3 (e)).
//!
//! Every case drives the REAL server over a REAL loopback socket with
//! the in-tree test client ([`control_protocol::client::TestClient`]):
//! no synthetic echo chambers (D-5). The harness binds an ephemeral
//! loopback port per case, runs [`ControlServer::serve_one`] on a
//! side thread, and joins it for the [`SessionSummary`] — the
//! assertion vocabulary is the server's OWN typed outcomes.
//!
//! Coverage (the card's acceptance list): handshake success/failure,
//! token wrong/right, version mismatch, malformed frames (truncated
//! headers, bad masks, oversized length claims), fragmentation
//! round-trip, ping/pong, close semantics, loopback refusal,
//! sensitive-command deny-by-default + allowed-when-enabled, request
//! validation, auth lockout, read-only unauthenticated mode.

use std::io::Read;
use std::io::Write;
use std::sync::mpsc;

use commands::Command;
use commands::WireRequest;
use commands::WireResponse;
use commands::catalog::spec::catalog_checksum;
use commands::catalog::types::EngineInfo;
use commands::envelope::CommandOutcome;
use commands::envelope::Generation;
use commands::envelope::RequestId;
use control_protocol::CloseCode;
use control_protocol::ProtocolError;
use control_protocol::ServerConfig;
use control_protocol::ServerPolicy;
use control_protocol::SessionSummary;
use control_protocol::TokenSource;
use control_protocol::client::TestClient;
use control_protocol::policy::PolicyFile;
use control_protocol::policy::PolicySection;
use control_protocol::protocol::ClientFrame;
use control_protocol::protocol::ProtocolErrorCode;
use control_protocol::protocol::ServerFrame;
use control_protocol::server::AuthLimits;
use control_protocol::server::ControlServer;
use control_protocol::server::Dispatch;
use control_protocol::server::SessionEnd;

/// The fixed token every default-harness server accepts.
const TOKEN: &str = "conformance-test-token";

// --- harness ---------------------------------------------------------------

/// A one-shot test server: binds ephemeral loopback, serves exactly
/// `sessions` connections, reports every summary on join.
struct Harness {
    ready: mpsc::Receiver<String>,
    done: std::thread::JoinHandle<Vec<SessionSummary>>,
}

impl Harness {
    /// Spawn with the given policy and a dispatch that answers
    /// `system.get-engine-info` (class S) and completes everything else.
    fn spawn(policy: ServerPolicy, sessions: usize) -> Harness {
        Self::spawn_full(policy, sessions, AuthLimits::documented())
    }

    fn spawn_full(policy: ServerPolicy, sessions: usize, auth: AuthLimits) -> Harness {
        let mut config = ServerConfig::strict(test_engine_info());
        config.token = TokenSource::Fixed(TOKEN.to_owned());
        config.policy = policy;
        config.auth_limits = auth;
        Self::spawn_config(config, sessions)
    }

    fn spawn_config(mut config: ServerConfig, sessions: usize) -> Harness {
        config.bind = "127.0.0.1".to_owned();
        config.port = 0;
        let (tx_ready, rx_ready) = mpsc::channel::<String>();
        let done = std::thread::spawn(move || {
            let mut server = match ControlServer::bind(config) {
                Ok(server) => server,
                Err(err) => unreachable!("bind failed: {err}"),
            };
            let addr = match server.local_addr() {
                Ok(addr) => addr.to_string(),
                Err(err) => unreachable!("local_addr failed: {err}"),
            };
            match tx_ready.send(addr) {
                Ok(()) => {}
                Err(_) => unreachable!("the test thread went away"),
            }
            let mut dispatch = EchoDispatch {
                info: test_engine_info(),
            };
            let mut summaries = Vec::new();
            for _ in 0..sessions {
                match server.serve_one(&mut dispatch) {
                    Ok(summary) => summaries.push(summary),
                    Err(err) => unreachable!("serve_one failed: {err}"),
                }
            }
            summaries
        });
        Harness {
            ready: rx_ready,
            done,
        }
    }

    /// The bound address (waits for the bind).
    fn addr(&self) -> String {
        match self.ready.recv() {
            Ok(addr) => addr,
            Err(_) => unreachable!("server thread died before binding"),
        }
    }

    /// Join and return the session summaries.
    fn join(self) -> Vec<SessionSummary> {
        match self.done.join() {
            Ok(summaries) => summaries,
            Err(_) => unreachable!("server thread panicked"),
        }
    }
}

/// The test dispatch: serves `system.get-engine-info` (class S
/// `Completed`) and answers everything else with a class-S
/// `Completed(Unit)` (the response shape is the layer's, not the
/// policy's — the policy gate runs BEFORE this).
struct EchoDispatch {
    info: EngineInfo,
}

impl Dispatch for EchoDispatch {
    fn dispatch(&mut self, request: WireRequest) -> WireResponse {
        let result = match request.command {
            Command::SystemGetEngineInfo => Ok(CommandOutcome::Completed(
                commands::CommandResult::EngineInfo {
                    info: self.info.clone(),
                },
            )),
            _ => Ok(CommandOutcome::Completed(commands::CommandResult::Unit)),
        };
        WireResponse {
            request_id: request.request_id,
            session: None,
            generation: Generation(0),
            result,
        }
    }
}

// --- helpers ----------------------------------------------------------------

fn test_engine_info() -> EngineInfo {
    EngineInfo {
        engine_version: "0.1.0-conformance".to_owned(),
        catalog_version: catalog_checksum().to_string(),
        protocol_version: "lumen-control/1".to_owned(),
        host: "conformance".to_owned(),
        features: Vec::new(),
        plugins: Vec::new(),
        decoders: Vec::new(),
        sessions: Vec::new(),
    }
}

/// Connect or fail the test with the typed error.
fn connect(addr: &str) -> TestClient {
    match TestClient::connect(addr) {
        Ok(client) => client,
        Err(err) => unreachable!("connect failed: {err}"),
    }
}

/// Send one text frame or fail the test with the typed error.
fn send(client: &mut TestClient, text: &str) {
    if let Err(err) = client.send_text(text) {
        unreachable!("send failed: {err}");
    }
}

/// Read the next server frame or fail the test.
fn next_frame(client: &mut TestClient) -> ServerFrame {
    match client.read_server_frame() {
        Ok(Some(frame)) => frame,
        other => unreachable!("expected a server frame, got {other:?}"),
    }
}

/// Read the close code or fail the test.
fn close_code(client: &mut TestClient) -> Option<u16> {
    match client.expect_close() {
        Ok(code) => code,
        Err(err) => unreachable!("expected a close frame: {err}"),
    }
}

/// The typed `protocol-error` frame every session-ending failure
/// sends BEFORE its close frame, then the close code itself.
fn error_then_close(client: &mut TestClient, code: ProtocolErrorCode) -> Option<u16> {
    match next_frame(client) {
        ServerFrame::ProtocolError { code: seen, .. } => assert_eq!(seen, code),
        other => unreachable!("expected a protocol-error frame, got {other:?}"),
    }
    close_code(client)
}

/// A ready-to-send `hello` text payload.
fn hello_text(token: Option<&str>, versions: &[u32]) -> String {
    let frame = ClientFrame::Hello {
        protocol: "lumen-control".to_owned(),
        versions: versions.to_vec(),
        token: token.map(str::to_owned),
    };
    match serde_json::to_string(&frame) {
        Ok(json) => json,
        Err(err) => unreachable!("{err}"),
    }
}

/// A ready-to-send request text payload.
fn request_text(id: u64, command_json: &str) -> String {
    format!(r#"{{"type":"request","requestId":{id},{command_json}}}"#)
}

/// The `system.get-engine-info` request (class S, non-sensitive).
fn engine_info_request(id: u64) -> String {
    request_text(id, r#""cmd":"system.get-engine-info""#)
}

// ---------------------------------------------------------------------------
// Handshake
// ---------------------------------------------------------------------------

#[test]
fn handshake_success_full_session_round_trip() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    send(&mut client, &hello_text(Some(TOKEN), &[1]));
    match next_frame(&mut client) {
        ServerFrame::Welcome(welcome) => {
            assert_eq!(welcome.version, 1, "the negotiated major");
            assert_eq!(welcome.engine_info.host, "conformance");
            assert!(welcome.capabilities.token_auth);
            assert!(!welcome.capabilities.read_only);
            assert_eq!(welcome.capabilities.max_frame_bytes, 1024 * 1024);
        }
        other => unreachable!("expected welcome, got {other:?}"),
    }
    // A request round-trips through the dispatch.
    send(&mut client, &engine_info_request(1));
    match next_frame(&mut client) {
        ServerFrame::Response { response } => {
            assert_eq!(response.request_id, RequestId(1));
            match response.result {
                Ok(CommandOutcome::Completed(commands::CommandResult::EngineInfo { .. })) => {}
                other => unreachable!("engine-info result: {other:?}"),
            }
        }
        other => unreachable!("expected response, got {other:?}"),
    }
    // Clean close (1000) — the server must echo it.
    if let Err(err) = client.close(1000) {
        unreachable!("{err}");
    }
    assert_eq!(close_code(&mut client), Some(1000));
    let summaries = harness.join();
    assert_eq!(summaries.len(), 1);
    assert!(summaries[0].authenticated);
    assert_eq!(summaries[0].version, Some(1));
    assert_eq!(summaries[0].requests_served, 1);
    assert_eq!(summaries[0].close, SessionEnd::PeerClosed(Some(1000)));
}

#[test]
fn handshake_failure_non_upgrade_http_gets_an_http_error() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let addr = harness.addr();
    // A raw TCP peer (not the WS test client): plain HTTP, no upgrade.
    let mut raw = match std::net::TcpStream::connect(&addr) {
        Ok(stream) => stream,
        Err(err) => unreachable!("{err}"),
    };
    if let Err(err) = raw.write_all(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n") {
        unreachable!("{err}");
    }
    let mut response = Vec::new();
    let _ = raw.read_to_end(&mut response);
    let head = String::from_utf8_lossy(&response).into_owned();
    assert!(
        head.starts_with("HTTP/1.1 400"),
        "an HTTP error, not a 101: {head:?}"
    );
    assert!(
        !head.contains("101"),
        "no protocol switch for a non-upgrade request"
    );
    let summaries = harness.join();
    assert_eq!(summaries[0].requests_served, 0, "no command executed");
}

// ---------------------------------------------------------------------------
// Token handshake
// ---------------------------------------------------------------------------

#[test]
fn wrong_token_closes_4401_and_counts() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    send(&mut client, &hello_text(Some("wrong-token"), &[1]));
    // Typed protocol-error first, then the close.
    match next_frame(&mut client) {
        ServerFrame::ProtocolError { code, message } => {
            assert_eq!(code, ProtocolErrorCode::Unauthenticated);
            assert!(
                !message.contains(TOKEN),
                "no token material in the error (V8)"
            );
        }
        other => unreachable!("expected a protocol-error, got {other:?}"),
    }
    assert_eq!(close_code(&mut client), Some(4401));
    let summaries = harness.join();
    assert!(!summaries[0].authenticated);
    assert_eq!(summaries[0].auth_failures, 1);
    assert_eq!(
        summaries[0].close,
        SessionEnd::ServerClosed(CloseCode::Unauthorized)
    );
}

#[test]
fn frames_before_authentication_are_refused() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    // A request BEFORE any hello: not-hello + close.
    send(&mut client, &engine_info_request(1));
    match next_frame(&mut client) {
        ServerFrame::ProtocolError { code, .. } => {
            assert_eq!(code, ProtocolErrorCode::NotHello);
        }
        other => unreachable!("expected not-hello, got {other:?}"),
    }
    assert_eq!(close_code(&mut client), Some(4400));
    let summaries = harness.join();
    assert_eq!(
        summaries[0].requests_served, 0,
        "no command executed pre-auth"
    );
}

#[test]
fn five_failures_lock_out_even_the_right_token() {
    // doc 05 §5.4: 5 failures impose a 60 s lockout. The failure book
    // is server-global (one attacker, many connections).
    let harness = Harness::spawn_full(
        ServerPolicy::deny_all_sensitive(),
        6,
        AuthLimits {
            max_failures: 5,
            lockout: std::time::Duration::from_secs(60),
        },
    );
    let addr = harness.addr();
    for _ in 0..5 {
        let mut client = connect(&addr);
        send(&mut client, &hello_text(Some("bad"), &[1]));
        assert_eq!(
            error_then_close(&mut client, ProtocolErrorCode::Unauthenticated),
            Some(4401),
            "failure close"
        );
    }
    // The 6th connection presents the RIGHT token — the lockout wins.
    let mut client = connect(&addr);
    send(&mut client, &hello_text(Some(TOKEN), &[1]));
    match next_frame(&mut client) {
        ServerFrame::ProtocolError { code, message } => {
            assert_eq!(code, ProtocolErrorCode::Unauthenticated);
            assert!(message.contains("locked out"), "{message}");
        }
        other => unreachable!("expected the lockout error, got {other:?}"),
    }
    assert_eq!(close_code(&mut client), Some(4401));
    let summaries = harness.join();
    assert_eq!(summaries.len(), 6);
    let last = match summaries.last() {
        Some(last) => last,
        None => unreachable!("no summaries"),
    };
    assert!(last.locked_out);
    assert_eq!(last.auth_failures, 5);
}

// ---------------------------------------------------------------------------
// Version negotiation
// ---------------------------------------------------------------------------

#[test]
fn version_mismatch_is_a_typed_error_and_close_4402() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    send(&mut client, &hello_text(Some(TOKEN), &[2, 3]));
    match next_frame(&mut client) {
        ServerFrame::ProtocolError { code, message } => {
            assert_eq!(code, ProtocolErrorCode::VersionMismatch);
            assert!(message.contains("mutually supported"), "{message}");
        }
        other => unreachable!("expected version-mismatch, got {other:?}"),
    }
    assert_eq!(close_code(&mut client), Some(4402));
    let summaries = harness.join();
    assert_eq!(
        summaries[0].close,
        SessionEnd::ServerClosed(CloseCode::VersionMismatch)
    );
}

#[test]
fn unknown_protocol_name_is_rejected() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    send(
        &mut client,
        r#"{"type":"hello","protocol":"some-other-rpc","versions":[1],"token":"x"}"#,
    );
    match next_frame(&mut client) {
        ServerFrame::ProtocolError { code, .. } => {
            assert_eq!(code, ProtocolErrorCode::UnknownProtocol);
        }
        other => unreachable!("expected unknown-protocol, got {other:?}"),
    }
    assert_eq!(close_code(&mut client), Some(4400));
}

#[test]
fn hello_twice_closes() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    send(&mut client, &hello_text(Some(TOKEN), &[1]));
    match next_frame(&mut client) {
        ServerFrame::Welcome(_) => {}
        other => unreachable!("first hello must succeed: {other:?}"),
    }
    send(&mut client, &hello_text(Some(TOKEN), &[1]));
    match next_frame(&mut client) {
        ServerFrame::ProtocolError { code, .. } => {
            assert_eq!(code, ProtocolErrorCode::HelloTwice);
        }
        other => unreachable!("expected hello-twice, got {other:?}"),
    }
    assert_eq!(close_code(&mut client), Some(4400));
}

// ---------------------------------------------------------------------------
// Malformed frames (raw wire bytes)
// ---------------------------------------------------------------------------

#[test]
fn truncated_frame_header_closes_1002() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    // A text-frame head that stops inside the 16-bit extended length,
    // then the peer ends its write side: the server must hit EOF
    // MID-FRAME (a typed 1002 close), never block on missing bytes.
    if let Err(err) = client.send_raw(&[0x81, 0x80 | 126, 0x01]) {
        unreachable!("{err}");
    }
    if let Err(err) = client.shutdown_write() {
        unreachable!("{err}");
    }
    assert_eq!(
        error_then_close(&mut client, ProtocolErrorCode::BadFrame),
        Some(1002)
    );
    let summaries = harness.join();
    assert_eq!(
        summaries[0].close,
        SessionEnd::ServerClosed(CloseCode::ProtocolError)
    );
}

#[test]
fn unmasked_client_frame_closes_1002() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    // FIN+text, 2-byte payload, NO mask bit, no mask key, raw bytes.
    if let Err(err) = client.send_raw(&[0x81, 0x02, b'h', b'i']) {
        unreachable!("{err}");
    }
    assert_eq!(
        error_then_close(&mut client, ProtocolErrorCode::BadFrame),
        Some(1002)
    );
}

#[test]
fn oversized_length_claim_closes_1009_before_allocation() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    // 64-bit length claiming 4 GiB, then a truncated body: the claim
    // itself must be the typed rejection (never an allocation, never a
    // wait for 4 GiB of payload).
    let mut bytes = vec![0x81, 0x80 | 127];
    bytes.extend_from_slice(&(4u64 << 30).to_be_bytes());
    bytes.extend_from_slice(&[0, 0, 0, 0]);
    if let Err(err) = client.send_raw(&bytes) {
        unreachable!("{err}");
    }
    assert_eq!(
        error_then_close(&mut client, ProtocolErrorCode::BadFrame),
        Some(1009)
    );
    let summaries = harness.join();
    assert_eq!(
        summaries[0].close,
        SessionEnd::ServerClosed(CloseCode::TooBig)
    );
}

#[test]
fn garbage_masked_payload_is_a_recoverable_json_error() {
    // "Wrong mask" from the client's own perspective: the payload
    // arrives masked with a key the receiver cannot know is wrong —
    // the observable contract is that unmasking yields garbage, which
    // the application layer must answer with a typed error while the
    // connection survives.
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    // Valid masked TEXT frame whose payload is not JSON.
    let garbage: [u8; 8] = [0x00, 0xff, 0x10, 0x01, 0xde, 0xad, 0xbe, 0xef];
    send(&mut client, &String::from_utf8_lossy(&garbage).into_owned());
    match next_frame(&mut client) {
        ServerFrame::ProtocolError { code, .. } => {
            assert_eq!(code, ProtocolErrorCode::BadJson);
        }
        other => unreachable!("expected bad-json, got {other:?}"),
    }
    // The connection SURVIVES: a well-formed hello still works.
    send(&mut client, &hello_text(Some(TOKEN), &[1]));
    match next_frame(&mut client) {
        ServerFrame::Welcome(_) => {}
        other => unreachable!("connection did not survive the json error: {other:?}"),
    }
    if let Err(err) = client.close(1000) {
        unreachable!("{err}");
    }
    let _ = close_code(&mut client);
    let summaries = harness.join();
    assert_eq!(summaries[0].protocol_errors, 1);
    assert!(summaries[0].authenticated, "the session recovered");
}

#[test]
fn invalid_utf8_text_closes_1007() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    use control_protocol::frame::Opcode;
    if let Err(err) = client.send_text_masked(true, Opcode::Text, &[0xff, 0xfe, 0xfd], [1, 2, 3, 4])
    {
        unreachable!("{err}");
    }
    assert_eq!(
        error_then_close(&mut client, ProtocolErrorCode::BadFrame),
        Some(1007)
    );
    let summaries = harness.join();
    assert_eq!(
        summaries[0].close,
        SessionEnd::ServerClosed(CloseCode::InvalidUtf8)
    );
}

// ---------------------------------------------------------------------------
// Fragmentation
// ---------------------------------------------------------------------------

#[test]
fn fragmented_hello_round_trips_with_interleaved_ping() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    use control_protocol::frame::Opcode;
    let payload = hello_text(Some(TOKEN), &[1]).into_bytes();
    // Three fragments: start / middle / final, with a ping in between.
    if let Err(err) = client.send_text_masked(false, Opcode::Text, &payload[..5], [1, 1, 1, 1]) {
        unreachable!("{err}");
    }
    if let Err(err) = client.ping(b"mid-fragment") {
        unreachable!("{err}");
    }
    if let Err(err) =
        client.send_text_masked(false, Opcode::Continuation, &payload[5..40], [2, 2, 2, 2])
    {
        unreachable!("{err}");
    }
    if let Err(err) =
        client.send_text_masked(true, Opcode::Continuation, &payload[40..], [3, 3, 3, 3])
    {
        unreachable!("{err}");
    }
    // The interleaved ping is answered BEFORE the message completes.
    if let Err(err) = client.expect_pong(b"mid-fragment") {
        unreachable!("{err}");
    }
    match next_frame(&mut client) {
        ServerFrame::Welcome(welcome) => assert_eq!(welcome.version, 1),
        other => unreachable!("fragmented hello did not reassemble: {other:?}"),
    }
    if let Err(err) = client.close(1000) {
        unreachable!("{err}");
    }
    let _ = close_code(&mut client);
    let summaries = harness.join();
    assert!(summaries[0].authenticated);
}

// ---------------------------------------------------------------------------
// Ping/pong + close semantics
// ---------------------------------------------------------------------------

#[test]
fn ping_is_answered_by_a_matching_pong() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    send(&mut client, &hello_text(Some(TOKEN), &[1]));
    let _ = next_frame(&mut client);
    if let Err(err) = client.ping(b"conformance-ping") {
        unreachable!("{err}");
    }
    if let Err(err) = client.expect_pong(b"conformance-ping") {
        unreachable!("{err}");
    }
    // An unsolicited pong is tolerated (RFC 6455 §5.5.3).
    if let Err(err) = client.send_text_masked(
        true,
        control_protocol::frame::Opcode::Pong,
        b"late",
        [9, 9, 9, 9],
    ) {
        unreachable!("{err}");
    }
    // The session still serves requests afterwards.
    send(&mut client, &engine_info_request(1));
    match next_frame(&mut client) {
        ServerFrame::Response { .. } => {}
        other => unreachable!("post-pong response: {other:?}"),
    }
    if let Err(err) = client.close(1000) {
        unreachable!("{err}");
    }
    let _ = close_code(&mut client);
}

#[test]
fn close_without_code_is_echoed_empty_then_eof() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    send(&mut client, &hello_text(Some(TOKEN), &[1]));
    let _ = next_frame(&mut client);
    // A close frame with NO payload (RFC 6455 §5.5.1).
    if let Err(err) = client.send_text_masked(
        true,
        control_protocol::frame::Opcode::Close,
        &[],
        [4, 4, 4, 4],
    ) {
        unreachable!("{err}");
    }
    assert_eq!(
        close_code(&mut client),
        None,
        "the echo carries no code either"
    );
    match client.read_frame() {
        Ok(None) => {}
        other => unreachable!("EOF after close, got {other:?}"),
    }
    let summaries = harness.join();
    assert_eq!(summaries[0].close, SessionEnd::PeerClosed(None));
}

// ---------------------------------------------------------------------------
// Loopback refusal
// ---------------------------------------------------------------------------

#[test]
fn non_loopback_bind_is_refused_at_construction_conformance() {
    for bad in ["0.0.0.0", "10.0.0.1", "example.com"] {
        let mut config = ServerConfig::strict(test_engine_info());
        config.bind = bad.to_owned();
        match ControlServer::bind(config) {
            Err(ProtocolError::NonLoopbackBind { requested }) => assert_eq!(requested, bad),
            other => unreachable!("bind {bad:?} accepted: {other:?}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Deny-by-default sensitive ops
// ---------------------------------------------------------------------------

#[test]
fn sensitive_command_denied_by_default_and_connection_survives() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    send(&mut client, &hello_text(Some(TOKEN), &[1]));
    let _ = next_frame(&mut client);
    // A sensitive op (settings mutation): refused even though the
    // session is fully token-authenticated.
    send(
        &mut client,
        &request_text(
            1,
            r#""cmd":"config.set-setting","args":{"area":"playback","key":"autoplay","value":true}"#,
        ),
    );
    match next_frame(&mut client) {
        ServerFrame::ProtocolError { code, message } => {
            assert_eq!(code, ProtocolErrorCode::Denied);
            assert!(message.contains("sensitive"), "{message}");
        }
        other => unreachable!("expected denied, got {other:?}"),
    }
    // The connection SURVIVES: an ordinary command still dispatches.
    send(&mut client, &engine_info_request(2));
    match next_frame(&mut client) {
        ServerFrame::Response { response } => assert_eq!(response.request_id, RequestId(2)),
        other => unreachable!("connection died after a denial: {other:?}"),
    }
    if let Err(err) = client.close(1000) {
        unreachable!("{err}");
    }
    let _ = close_code(&mut client);
    let summaries = harness.join();
    assert_eq!(summaries[0].ops_denied, 1);
    assert_eq!(
        summaries[0].requests_served, 1,
        "only the ordinary op reached the dispatch"
    );
}

#[test]
fn sensitive_command_allowed_when_config_grants_it() {
    // The grant is CONFIG-DRIVEN (PolicyFile → ServerPolicy), never a
    // hard-coded literal.
    let file = PolicyFile {
        policy: PolicySection {
            sensitive_allow: vec!["config.set-setting".to_owned()],
        },
        ..PolicyFile::default()
    };
    let policy = match ServerPolicy::from_file(&file) {
        Ok(policy) => policy,
        Err(err) => unreachable!("grant rejected: {err}"),
    };
    let harness = Harness::spawn(policy, 1);
    let mut client = connect(&harness.addr());
    send(&mut client, &hello_text(Some(TOKEN), &[1]));
    let _ = next_frame(&mut client);
    send(
        &mut client,
        &request_text(
            1,
            r#""cmd":"config.set-setting","args":{"area":"playback","key":"autoplay","value":true}"#,
        ),
    );
    match next_frame(&mut client) {
        ServerFrame::Response { .. } => {}
        other => unreachable!("granted op did not dispatch: {other:?}"),
    }
    // A DIFFERENT sensitive op is still denied (per-id grants).
    send(
        &mut client,
        &request_text(2, r#""cmd":"system.plugin-enable","args":{"id":"x"}"#),
    );
    match next_frame(&mut client) {
        ServerFrame::ProtocolError { code, .. } => {
            assert_eq!(code, ProtocolErrorCode::Denied);
        }
        other => unreachable!("ungranted sibling op allowed: {other:?}"),
    }
    if let Err(err) = client.close(1000) {
        unreachable!("{err}");
    }
    let _ = close_code(&mut client);
    let summaries = harness.join();
    assert_eq!(summaries[0].ops_denied, 1);
    assert_eq!(summaries[0].requests_served, 1);
}

#[test]
fn unauthenticated_read_only_mode_serves_the_safe_subset_only() {
    let file = PolicyFile {
        server: control_protocol::policy::ServerSection {
            read_only: true,
            ..Default::default()
        },
        ..PolicyFile::default()
    };
    let policy = match ServerPolicy::from_file(&file) {
        Ok(policy) => policy,
        Err(err) => unreachable!("{err}"),
    };
    let harness = Harness::spawn(policy, 1);
    let mut client = connect(&harness.addr());
    // NO token: read-only mode admits the fixed safe subset.
    send(&mut client, &hello_text(None, &[1]));
    match next_frame(&mut client) {
        ServerFrame::Welcome(welcome) => {
            assert!(
                welcome.capabilities.read_only,
                "the welcome advertises the mode"
            );
        }
        other => unreachable!("read-only hello refused: {other:?}"),
    }
    // Safe subset: media.get-info passes the POLICY (the response
    // proves it reached the dispatch).
    send(&mut client, &request_text(1, r#""cmd":"media.get-info""#));
    match next_frame(&mut client) {
        ServerFrame::Response { .. } => {}
        other => unreachable!("safe-subset op refused: {other:?}"),
    }
    // Outside the subset: playback.play is denied.
    send(&mut client, &request_text(2, r#""cmd":"playback.play""#));
    match next_frame(&mut client) {
        ServerFrame::ProtocolError { code, .. } => {
            assert_eq!(code, ProtocolErrorCode::Denied);
        }
        other => unreachable!("read-only overreach allowed: {other:?}"),
    }
    if let Err(err) = client.close(1000) {
        unreachable!("{err}");
    }
    let _ = close_code(&mut client);
    let summaries = harness.join();
    assert!(!summaries[0].authenticated);
    assert_eq!(summaries[0].requests_served, 1);
}

// ---------------------------------------------------------------------------
// Request validation (connection survives)
// ---------------------------------------------------------------------------

#[test]
fn request_validation_rejects_dirty_payloads_and_survives() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    send(&mut client, &hello_text(Some(TOKEN), &[1]));
    let _ = next_frame(&mut client);

    // (a) unknown envelope field.
    send(
        &mut client,
        &request_text(1, r#""cmd":"playback.play","extra":true"#),
    );
    match next_frame(&mut client) {
        ServerFrame::ProtocolError { code, message } => {
            assert_eq!(code, ProtocolErrorCode::InvalidRequest);
            assert!(message.contains("extra"), "{message}");
        }
        other => unreachable!("unknown field accepted: {other:?}"),
    }
    // (b) unknown command id (the catalog's adjacent tag rejects it).
    send(
        &mut client,
        &request_text(2, r#""cmd":"playback.does-not-exist""#),
    );
    match next_frame(&mut client) {
        ServerFrame::ProtocolError { code, .. } => {
            assert_eq!(code, ProtocolErrorCode::UnknownType);
        }
        other => unreachable!("unknown cmd accepted: {other:?}"),
    }
    // (c) bad argument type (volume is a number, not a string).
    send(
        &mut client,
        &request_text(3, r#""cmd":"audio.set-volume","args":{"volume":"loud"}"#),
    );
    match next_frame(&mut client) {
        ServerFrame::ProtocolError { code, .. } => {
            assert_eq!(code, ProtocolErrorCode::UnknownType);
        }
        other => unreachable!("bad arg type accepted: {other:?}"),
    }
    // (d) valid JSON that is not an envelope OBJECT (an array):
    // `bad-json` is reserved for payloads that do not parse at all
    // (covered by garbage_masked_payload…); a JSON array parses, it
    // just has no `type` — the typed envelope parse answers it.
    send(&mut client, "[1,2,3]");
    match next_frame(&mut client) {
        ServerFrame::ProtocolError { code, .. } => {
            assert_eq!(code, ProtocolErrorCode::UnknownType);
        }
        other => unreachable!("array payload accepted: {other:?}"),
    }
    // The connection SURVIVES all of the above.
    send(&mut client, &engine_info_request(4));
    match next_frame(&mut client) {
        ServerFrame::Response { response } => assert_eq!(response.request_id, RequestId(4)),
        other => unreachable!("connection died during validation: {other:?}"),
    }
    if let Err(err) = client.close(1000) {
        unreachable!("{err}");
    }
    let _ = close_code(&mut client);
    let summaries = harness.join();
    assert_eq!(summaries[0].protocol_errors, 4);
    assert_eq!(summaries[0].requests_served, 1);
}

#[test]
fn non_monotonic_request_id_closes_the_connection() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    send(&mut client, &hello_text(Some(TOKEN), &[1]));
    let _ = next_frame(&mut client);
    send(&mut client, &engine_info_request(5));
    let _ = next_frame(&mut client);
    // A REPLAY of id 5 (doc 05 §4.1/§5.4 anti-replay): close 4400.
    send(&mut client, &engine_info_request(5));
    match next_frame(&mut client) {
        ServerFrame::ProtocolError { code, .. } => {
            assert_eq!(code, ProtocolErrorCode::NonMonotonicRequestId);
        }
        other => unreachable!("replay accepted: {other:?}"),
    }
    assert_eq!(close_code(&mut client), Some(4400));
    let summaries = harness.join();
    assert_eq!(summaries[0].requests_served, 1);
}

// ---------------------------------------------------------------------------
// Binary frames + subscribe
// ---------------------------------------------------------------------------

#[test]
fn binary_application_frames_are_rejected_but_survive() {
    let harness = Harness::spawn(ServerPolicy::deny_all_sensitive(), 1);
    let mut client = connect(&harness.addr());
    send(&mut client, &hello_text(Some(TOKEN), &[1]));
    let _ = next_frame(&mut client);
    // lumen-control/1 speaks JSON TEXT frames only.
    if let Err(err) = client.send_binary(&[0x00, 0x01, 0x02]) {
        unreachable!("{err}");
    }
    match next_frame(&mut client) {
        ServerFrame::ProtocolError { code, .. } => {
            assert_eq!(code, ProtocolErrorCode::BadFrame);
        }
        other => unreachable!("binary frame accepted: {other:?}"),
    }
    // Subscribe is accepted (recorded; the event push is the host
    // engine's business — no ack frame in the doc 05 envelope).
    send(
        &mut client,
        r#"{"type":"subscribe","events":["timing.seek-completed"]}"#,
    );
    send(&mut client, &engine_info_request(1));
    match next_frame(&mut client) {
        ServerFrame::Response { .. } => {}
        other => unreachable!("post-subscribe response: {other:?}"),
    }
    if let Err(err) = client.close(1000) {
        unreachable!("{err}");
    }
    let _ = close_code(&mut client);
    let summaries = harness.join();
    assert_eq!(summaries[0].requests_served, 1);
    assert_eq!(summaries[0].protocol_errors, 1);
}
