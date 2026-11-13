//! App-level end-to-end conformance — card 7.J1 (doc 05 §5.4; doc 10
//! D-5: real files, real loopback sockets, the in-tree WS test
//! client).
//!
//! Drives the FULL app stack exactly as `main` assembles it —
//! `product.toml` (schema-validated, feature flag flipped in a temp
//! copy — the repo's own flag is honestly `false` until the Lead
//! ratifies) → `control-server.toml` → [`resolve_server_config`] →
//! [`ControlServer::bind`] → [`serve_loop`] with the app's dispatch
//! and a capturing log sink — then speaks `lumen-control/1` over the
//! wire with [`control_protocol::client::TestClient`].

use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc;

use commands::CommandResult;
use commands::envelope::CommandOutcome;
use control_protocol::client::TestClient;
use control_protocol::protocol::ClientFrame;
use control_protocol::protocol::ProtocolErrorCode;
use control_protocol::protocol::ServerFrame;
use control_protocol::server::ControlServer;

use control_server::CatalogDispatch;
use control_server::load_config_file;
use control_server::logging::MemoryLog;
use control_server::resolve_server_config;
use control_server::serve_loop;

/// The workspace root (this file lives in apps/control-server/tests).
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The committed root schema.
fn root_schema() -> product_config::Schema {
    let path = repo_root().join("product.schema.json");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => unreachable!("cannot read {}: {err}", path.display()),
    };
    match text.parse::<product_config::Schema>() {
        Ok(schema) => schema,
        Err(err) => unreachable!("root schema failed to load: {err}"),
    }
}

/// A temp copy of the REAL product.toml with the control-server flag
/// flipped to `true` (exactly once — never silently test a wrong
/// file). The repo's own flag stays `false` until the Lead ratifies
/// (rule F2); this copy is what ratification looks like.
///
/// The temp path carries a per-CALL counter, not just the pid: parallel
/// tests in this binary share the pid, and two of them writing the SAME
/// pid-anchored path raced write/truncate/read on CI (run 43409155006 —
/// one test's loader read the other's half-written file and died
/// "required key is missing" for every top-level key, i.e. an empty
/// moment of the sibling's write). Same flake class as the
/// platform-linux ETXTBSY stub (wave-L round 4): per-call uniqueness is
/// the contract for shared-/tmp test artifacts.
fn flipped_product() -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static CALL: AtomicU32 = AtomicU32::new(0);
    let call = CALL.fetch_add(1, Ordering::Relaxed);
    let source = repo_root().join("product.toml");
    let text = match std::fs::read_to_string(&source) {
        Ok(text) => text,
        Err(err) => unreachable!("cannot read {}: {err}", source.display()),
    };
    // lumen-allow-literal: the feature flag line being flipped (doc 07 §2.7 [features]) — config key, not product identity
    let flipped = text.replace("control-server = false", "control-server = true");
    assert_eq!(
        flipped.matches("control-server = true").count(),
        1,
        "the flip must land exactly once"
    );
    let path = std::env::temp_dir().join(format!(
        "lumen-control-server-e2e-product-{}-{call}.toml",
        std::process::id()
    ));
    if let Err(err) = std::fs::write(&path, flipped) {
        unreachable!("cannot write {}: {err}", path.display());
    }
    path
}

/// Load a product.toml through the schema pipeline.
fn load_product(path: &Path) -> product_config::ProductConfig {
    let schema = root_schema();
    match product_config::load_file(path, &schema) {
        Ok(product) => product,
        Err(diagnostics) => unreachable!("{} failed to load: {:?}", path.display(), diagnostics),
    }
}

/// Write a temp operator config; returns its path.
///
/// The name carries the per-CALL counter (same contract as
/// [`flipped_product`]): parallel tests in this binary share the pid, and
/// the two `strict_config()` call sites (the strict-shape test and the
/// e2e session test) raced write/truncate/read on the ONE fixed path —
/// run 43451838615 (Rust lane, 28fb38fa) caught the e2e side reading the
/// sibling's truncate window: the load went through empty, the resolve
/// came out token-less, and hello was refused `Unauthenticated
/// "token rejected"`. Per-call uniqueness makes the collision impossible.
fn write_config(name: &str, body: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static CALL: AtomicU32 = AtomicU32::new(0);
    let call = CALL.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "lumen-control-server-e2e-{name}-{}-{call}.toml",
        std::process::id()
    ));
    if let Err(err) = std::fs::write(&path, body) {
        unreachable!("cannot write {}: {err}", path.display());
    }
    path
}

/// The strict-but-fixed-token operator config used by the session
/// tests: loopback, ephemeral port, `config.set-setting` granted.
fn strict_config() -> PathBuf {
    write_config(
        "strict",
        "[server]\nbind = \"127.0.0.1\"\nport = 0\n\n[auth]\ntoken = \"e2e-fixed-token\"\n\n[policy]\nsensitive_allow = [\"config.set-setting\"]\n",
    )
}

/// Send one JSON text frame.
fn send(client: &mut TestClient, text: &str) {
    if let Err(err) = client.send_text(text) {
        unreachable!("send failed: {err}");
    }
}

/// Read the next server frame.
fn next_frame(client: &mut TestClient) -> ServerFrame {
    match client.read_server_frame() {
        Ok(Some(frame)) => frame,
        other => unreachable!("expected a server frame, got {other:?}"),
    }
}

/// The hello payload with the given token.
fn hello(token: &str) -> String {
    let frame = ClientFrame::Hello {
        // lumen-allow-literal: wire protocol name (doc 05 §5.4), not product identity
        protocol: "lumen-control".to_owned(),
        versions: vec![1],
        token: Some(token.to_owned()),
    };
    match serde_json::to_string(&frame) {
        Ok(json) => json,
        Err(err) => unreachable!("{err}"),
    }
}

/// A request envelope with the given id and `cmd` JSON fragment.
fn request(id: u64, command_json: &str) -> String {
    format!(r#"{{"type":"request","requestId":{id},{command_json}}}"#)
}

// ---------------------------------------------------------------------------
// Config-file conformance (the app's own seam)
// ---------------------------------------------------------------------------

#[test]
fn operator_toml_parses_into_the_strict_shape() {
    let path = strict_config();
    match load_config_file(&path) {
        Ok(file) => {
            assert_eq!(file.server.bind, "127.0.0.1");
            assert_eq!(file.server.port, 0);
            assert!(!file.server.read_only);
            assert_eq!(file.auth.token.as_deref(), Some("e2e-fixed-token"));
            assert_eq!(file.policy.sensitive_allow, ["config.set-setting"]);
        }
        Err(err) => unreachable!("valid config rejected: {err}"),
    }
    let _ = std::fs::remove_file(path);
}

#[test]
fn dirty_operator_toml_is_a_typed_config_error() {
    // (a) not TOML at all.
    let path = write_config("broken", "[server\nbind = oops");
    match load_config_file(&path) {
        Err(control_server::AppError::ConfigParse { .. }) => {}
        other => unreachable!("broken toml: {other:?}"),
    }
    // (b) unknown key (deny_unknown_fields — typos surface at startup).
    let path = write_config("unknown-key", "[server]\nbinder = \"127.0.0.1\"\n");
    match load_config_file(&path) {
        Err(control_server::AppError::ConfigParse { detail, .. }) => {
            assert!(detail.contains("binder"), "{detail}");
        }
        other => unreachable!("unknown key accepted: {other:?}"),
    }
    let _ = std::fs::remove_file(path);
}

#[test]
fn a_typoed_sensitive_grant_is_a_typed_startup_error() {
    let path = write_config(
        "typo-grant",
        "[policy]\nsensitive_allow = [\"config.set-stuff\"]\n",
    );
    let file = match load_config_file(&path) {
        Ok(file) => file,
        Err(err) => unreachable!("valid toml rejected: {err}"),
    };
    let product = load_product(&flipped_product());
    match resolve_server_config(&product, &file) {
        Err(control_server::AppError::PolicyResolve { source }) => {
            assert!(source.to_string().contains("config.set-stuff"));
        }
        other => unreachable!("typo grant accepted: {other:?}"),
    }
    let _ = std::fs::remove_file(path);
}

#[test]
fn a_non_loopback_bind_in_the_operator_config_is_refused_at_bind() {
    let path = write_config("open-bind", "[server]\nbind = \"0.0.0.0\"\n");
    let file = match load_config_file(&path) {
        Ok(file) => file,
        Err(err) => unreachable!("valid toml rejected: {err}"),
    };
    let product = load_product(&flipped_product());
    let config = match resolve_server_config(&product, &file) {
        Ok(config) => config,
        Err(err) => unreachable!("resolve refused a loopback-safe config: {err}"),
    };
    match ControlServer::bind(config) {
        Err(control_protocol::ProtocolError::NonLoopbackBind { requested }) => {
            assert_eq!(requested, "0.0.0.0");
        }
        other => unreachable!("open bind accepted: {other:?}"),
    }
    let _ = std::fs::remove_file(path);
}

// ---------------------------------------------------------------------------
// The full session (feature-gated product + config + wire + dispatch + log)
// ---------------------------------------------------------------------------

#[test]
fn e2e_full_session_through_the_app_stack() {
    let product = load_product(&flipped_product());
    let config_path = strict_config();
    let file = match load_config_file(&config_path) {
        Ok(file) => file,
        Err(err) => unreachable!("valid config rejected: {err}"),
    };
    let mut config = match resolve_server_config(&product, &file) {
        Ok(config) => config,
        Err(err) => unreachable!("feature-flipped product refused: {err}"),
    };
    config.bind = "127.0.0.1".to_owned();
    config.port = 0;
    let engine_info = config.engine_info.clone();
    let mut server = match ControlServer::bind(config) {
        Ok(server) => server,
        Err(err) => unreachable!("loopback bind failed: {err}"),
    };
    let address = match server.local_addr() {
        Ok(address) => address.to_string(),
        Err(err) => unreachable!("local_addr failed: {err}"),
    };
    // The event loop exactly as main runs it, capped at one session,
    // with a capturing sink instead of the stderr one. The listener is
    // already bound, so the client cannot race it.
    let (tx_ready, rx_ready) = mpsc::channel::<String>();
    let done = std::thread::spawn(move || {
        if let Err(err) = tx_ready.send("ready".to_owned()) {
            unreachable!("test thread went away: {err}");
        }
        let mut dispatch = CatalogDispatch::new(engine_info);
        let mut sink = MemoryLog::new();
        let outcome = match serve_loop(&mut server, &mut dispatch, &mut sink, Some(1)) {
            Ok(outcome) => outcome,
            Err(err) => unreachable!("serve loop failed: {err}"),
        };
        (outcome, sink)
    });
    match rx_ready.recv() {
        Ok(_) => {}
        Err(_) => unreachable!("server thread died before accepting"),
    }

    let mut client = match TestClient::connect(&address) {
        Ok(client) => client,
        Err(err) => unreachable!("connect failed: {err}"),
    };
    send(&mut client, &hello("e2e-fixed-token"));
    match next_frame(&mut client) {
        ServerFrame::Welcome(welcome) => {
            assert_eq!(welcome.version, 1);
            assert_eq!(welcome.engine_info.host, "control-server");
            assert_eq!(
                welcome.engine_info.engine_version,
                product.identity.display_version
            );
            assert!(welcome.capabilities.token_auth);
            assert!(!welcome.capabilities.read_only);
        }
        other => unreachable!("expected welcome, got {other:?}"),
    }

    // (1) Catalog introspection: all 64 commands, from the live table.
    send(&mut client, &request(1, r#""cmd":"system.list-commands""#));
    match next_frame(&mut client) {
        ServerFrame::Response { response } => match response.result {
            Ok(CommandOutcome::Completed(CommandResult::CommandListings { commands })) => {
                assert_eq!(commands.len(), 64);
                assert!(commands.iter().any(|row| row.id == "config.set-setting"));
            }
            other => unreachable!("list-commands result: {other:?}"),
        },
        other => unreachable!("list-commands response: {other:?}"),
    }

    // (2) A GRANTED sensitive op passes the policy and reaches the
    // dispatch — which answers honestly that no engine is embedded.
    send(
        &mut client,
        &request(
            2,
            r#""cmd":"config.set-setting","args":{"area":"playback","key":"preferred-audio-languages","value":["en"]}"#,
        ),
    );
    match next_frame(&mut client) {
        ServerFrame::Response { response } => match response.result {
            Err(err) => {
                assert_eq!(err.code, commands::error::ErrorCode::NotReady);
                assert_eq!(err.stage, commands::error::Stage::Control);
            }
            other => unreachable!("granted op answered {other:?}"),
        },
        other => unreachable!("granted op response: {other:?}"),
    }

    // (3) An UNGRANTED sensitive op is refused by the policy; the
    // connection survives.
    send(
        &mut client,
        &request(
            3,
            r#""cmd":"session.open","args":{"source":{"kind":"url","url":"http://127.0.0.1/x"}}"#,
        ),
    );
    match next_frame(&mut client) {
        ServerFrame::ProtocolError { code, .. } => {
            assert_eq!(code, ProtocolErrorCode::Denied);
        }
        other => unreachable!("unganted op allowed: {other:?}"),
    }

    // (4) Still alive: engine-info answers with the product identity.
    send(
        &mut client,
        &request(4, r#""cmd":"system.get-engine-info""#),
    );
    match next_frame(&mut client) {
        ServerFrame::Response { response } => match response.result {
            Ok(CommandOutcome::Completed(CommandResult::EngineInfo { info })) => {
                assert_eq!(info.engine_version, product.identity.display_version);
                assert_eq!(info.host, "control-server");
            }
            other => unreachable!("engine-info result: {other:?}"),
        },
        other => unreachable!("engine-info response: {other:?}"),
    }

    // Clean close.
    if let Err(err) = client.close(1000) {
        unreachable!("{err}");
    }
    match client.expect_close() {
        Ok(code) => assert_eq!(code, Some(1000)),
        Err(err) => unreachable!("close echo: {err}"),
    }

    let (outcome, sink) = match done.join() {
        Ok(pair) => pair,
        Err(_) => unreachable!("server thread panicked"),
    };
    assert_eq!(outcome.sessions, 1);
    assert!(outcome.capped);
    // The structured log carries the session record (no token, V8).
    let session_records: Vec<_> = sink
        .records
        .iter()
        .filter(|record| record.event == "session-end")
        .collect();
    assert_eq!(session_records.len(), 1);
    let fields: Vec<(&str, &str)> = session_records[0]
        .fields
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect();
    assert!(fields.contains(&("requests_served", "3")));
    assert!(fields.contains(&("ops_denied", "1")));
    assert!(fields.contains(&("authenticated", "true")));
    for (key, _) in &fields {
        assert_ne!(*key, "token", "the log stream must never carry the token");
    }
    let _ = std::fs::remove_file(config_path);
}

// ---------------------------------------------------------------------------
// The stderr sink shape (one JSON line, stable keys)
// ---------------------------------------------------------------------------

#[test]
fn the_production_sink_renders_one_parseable_json_line_per_record() {
    let record = control_server::LogRecord::new(control_server::logging::Level::Info, "startup")
        .with("addr", "127.0.0.1:0");
    let line = control_server::logging::render_line(&record, 1234.5);
    assert!(line.ends_with('\n'), "line discipline: {line:?}");
    let parsed = match line.trim().parse::<serde_json::Value>() {
        Ok(value) => value,
        Err(err) => unreachable!("not JSON: {err}: {line}"),
    };
    assert_eq!(parsed["ts"], serde_json::json!(1234.5));
    assert_eq!(parsed["level"], serde_json::json!("info"));
    assert_eq!(parsed["event"], serde_json::json!("startup"));
    assert_eq!(parsed["addr"], serde_json::json!("127.0.0.1:0"));
}
