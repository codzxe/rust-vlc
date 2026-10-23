//! Scenario 10 — **missing file** (doc 10 §6.4 row 10): absent and
//! unreadable paths, driven through the command layer.
//!
//! Pass criteria: typed `Inaccessible` with a redacted message. The
//! four shapes: a nonexistent path (the canonicalize refusal), a
//! directory (`not a regular file`), a dangling symlink (resolve
//! failure), and a chmod-000 file (the access-layer's own permission
//! refusal). Every refusal leaves the host `Idle`, no session, and
//! later snapshots answer the honest `session.not-ready`.

use std::os::unix::fs::PermissionsExt;

use commands::envelope::CommandOutcome;
use commands::{Command, ErrorCode, Stage};
use testkit::Must;
use testkit::alloc_count::serialize_tests;

use crate::host::{PARENT, ScenarioHost, TempDir, dispatch};

/// Dispatch one `session.open` for `path` and hand back the typed
/// error (or fail the test — success is not an outcome here).
fn refuse_open(host: &mut ScenarioHost, path: std::path::PathBuf) -> commands::CommandError {
    let response = dispatch(
        host,
        Command::SessionOpen {
            source: commands::catalog::types::SourceSpec::File { path },
            options: Default::default(),
        },
    );
    match response.result {
        Err(error) => error,
        Ok(_) => unreachable!("an inaccessible path must refuse, not open"),
    }
}

/// The common refusal shape: input.inaccessible, Idle, no session,
/// redacted message, later snapshots not-ready.
fn assert_inaccessible(host: &mut ScenarioHost, error: &commands::CommandError, basename: &str) {
    assert_eq!(error.stage, Stage::Input, "{error}");
    assert_eq!(error.code, ErrorCode::Inaccessible, "{error}");
    // V8 redaction: the basename survives, machine paths do not.
    assert!(
        error.message.contains(basename),
        "the message names the file: {error}"
    );
    assert!(
        !error.message.contains("/home/"),
        "the message carries no machine path (V8): {error}"
    );
    assert_eq!(host.state(), commands::PlaybackState::Idle, "stays Idle");
    assert!(host.session_id().is_none(), "no session exists");
    let snapshot = dispatch(host, Command::MediaGetCurrentTime);
    match snapshot.result {
        Err(later) => {
            assert_eq!(later.stage, Stage::Session, "{later}");
            assert_eq!(later.code, ErrorCode::NotReady, "{later}");
        }
        Ok(_) => unreachable!("a snapshot without a session must refuse"),
    }
}

/// The four inaccessible shapes.
#[test]
fn missing_and_unreadable_paths_refuse_typed_and_redacted() {
    let _serial = serialize_tests();
    let scratch = TempDir::new("s10");

    // 1. A nonexistent path.
    let absent = scratch.path().join("no-such-file.avi");
    let mut host = ScenarioHost::new();
    let error = refuse_open(&mut host, absent);
    assert_inaccessible(&mut host, &error, "no-such-file.avi");

    // 2. A directory: nothing to read here (66 EX_NOINPUT, the
    //    unsupported-path suite's own contract).
    let directory = scratch.path().join("a-directory");
    std::fs::create_dir_all(&directory).must();
    let error = refuse_open(&mut host, directory);
    assert_inaccessible(&mut host, &error, "a-directory");

    // 3. A dangling symlink: the resolve failure is the truth.
    let dangling = scratch.path().join("dangling.avi");
    std::os::unix::fs::symlink(scratch.path().join("gone.avi"), &dangling).must();
    let error = refuse_open(&mut host, dangling);
    assert_inaccessible(&mut host, &error, "dangling.avi");

    // 4. A chmod-000 file: canonicalization succeeds; the ACCESS
    //    layer owns the permission refusal.
    let secret = scratch.copy_parent("secret.avi");
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o000)).must();
    let error = refuse_open(&mut host, secret);
    assert_inaccessible(&mut host, &error, "secret.avi");

    // The host remains reusable after every refusal: the next open
    // of a good file succeeds (no poisoned state).
    let good = testkit::fixture_path(PARENT).must();
    let response = dispatch(
        &mut host,
        Command::SessionOpen {
            source: commands::catalog::types::SourceSpec::File { path: good },
            options: Default::default(),
        },
    );
    assert!(
        matches!(&response.result, Ok(CommandOutcome::Accepted { .. })),
        "the host is reusable after refusals: {:?}",
        response.result
    );
    let close = dispatch(&mut host, Command::SessionClose);
    assert!(close.result.is_ok(), "closes: {:?}", close.result);
}
