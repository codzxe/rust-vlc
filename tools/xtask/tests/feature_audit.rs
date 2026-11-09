//! Fixture-based acceptance proof for `cargo xtask feature-audit`
//! (issue #20, card 8.F1): a good matrix passes, and each seeded failure
//! mode — implemented-without-evidence, dangling evidence path, invalid
//! status value, missing platform column, unparseable TOML — fails with
//! exit 1, while an unreadable matrix is an xtask error (exit 2).
//!
//! Same pattern as `check_architecture.rs`: scratch workspaces under the
//! system temp dir, the real binary via `CARGO_BIN_EXE_xtask`, nothing
//! touches the checked-in tree.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct RunOutcome {
    code: Option<i32>,
    stdout: String,
}

fn run_audit(root: &Path) -> RunOutcome {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_xtask"));
    cmd.arg("feature-audit").arg("--root").arg(root);
    match cmd.output() {
        Ok(out) => RunOutcome {
            code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        },
        Err(err) => RunOutcome {
            code: None,
            stdout: format!("failed to spawn xtask binary: {err}"),
        },
    }
}

/// A scratch workspace that removes itself when dropped.
struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Fixture {
        let root = std::env::temp_dir().join(format!(
            "xtask-feature-audit-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs")).expect("create fixture docs dir");
        Fixture { root }
    }

    fn write_matrix(&self, body: &str) {
        fs::write(self.root.join("docs/feature-matrix.toml"), body).expect("write matrix");
    }

    fn add_evidence_file(&self) {
        fs::create_dir_all(self.root.join("crates/demux/src")).expect("mkdir crates");
        fs::write(self.root.join("crates/demux/src/x.rs"), "").expect("write evidence file");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

const GOOD_MATRIX: &str = r#"
[meta]
schema_version = 1

[[feature]]
id = "container-x"
category = "container"
priority = "P0"
phase_target = 5
status = { windows = "implemented", linux = "implemented", macos = "implemented", web = "not_started" }
evidence = "crates/demux/src/x.rs (card 5.D1)"
notes = "fine"

[[feature]]
id = "ui-screen-y"
category = "ui-screen"
priority = "P1"
phase_target = 4
status = { windows = "not_started", linux = "not_started", macos = "not_started", web = "not_started" }
evidence = ""
"#;

#[test]
fn good_matrix_passes() {
    let fx = Fixture::new("good");
    fx.write_matrix(GOOD_MATRIX);
    fx.add_evidence_file();
    let out = run_audit(&fx.root);
    assert_eq!(out.code, Some(0), "clean fixture must pass:\n{}", out.stdout);
    assert!(out.stdout.contains("result: PASS"), "{}", out.stdout);
}

#[test]
fn good_matrix_reports_derived_counts() {
    let fx = Fixture::new("counts");
    fx.write_matrix(GOOD_MATRIX);
    fx.add_evidence_file();
    let out = run_audit(&fx.root);
    assert!(out.stdout.contains("rows: 2"), "{}", out.stdout);
    assert!(out.stdout.contains("container=1"), "{}", out.stdout);
    assert!(out.stdout.contains("ui-screen=1"), "{}", out.stdout);
    assert!(out.stdout.contains("5=1"), "per-phase count missing:\n{}", out.stdout);
}

#[test]
fn implemented_without_evidence_fails() {
    let fx = Fixture::new("no-evidence");
    fx.write_matrix(&GOOD_MATRIX.replace(
        r#"evidence = "crates/demux/src/x.rs (card 5.D1)""#,
        "evidence = \"\"",
    ));
    let out = run_audit(&fx.root);
    assert_eq!(out.code, Some(1), "implemented row without evidence must fail:\n{}", out.stdout);
    assert!(out.stdout.contains("without evidence"), "{}", out.stdout);
}

#[test]
fn evidence_citing_missing_file_fails() {
    let fx = Fixture::new("missing-file");
    fx.write_matrix(GOOD_MATRIX);
    // No crates/ tree in this fixture: the evidence path cannot resolve.
    let out = run_audit(&fx.root);
    assert_eq!(out.code, Some(1), "dangling evidence path must fail:\n{}", out.stdout);
    assert!(
        out.stdout.contains("missing path `crates/demux/src/x.rs`"),
        "violation should name the path:\n{}",
        out.stdout
    );
}

#[test]
fn invalid_status_value_fails() {
    let fx = Fixture::new("bad-status");
    fx.write_matrix(&GOOD_MATRIX.replace(r#"macos = "implemented""#, r#"macos = "done""#));
    fx.add_evidence_file();
    let out = run_audit(&fx.root);
    assert_eq!(out.code, Some(1), "status outside the vocabulary must fail:\n{}", out.stdout);
    assert!(out.stdout.contains("not in the vocabulary"), "{}", out.stdout);
}

#[test]
fn missing_platform_column_fails() {
    let fx = Fixture::new("bad-columns");
    fx.write_matrix(&GOOD_MATRIX.replace(
        r#", web = "not_started" }"#,
        r#" }"#,
    ));
    fx.add_evidence_file();
    let out = run_audit(&fx.root);
    assert_eq!(out.code, Some(1), "missing platform column must fail:\n{}", out.stdout);
    assert!(out.stdout.contains("!= exactly"), "{}", out.stdout);
}

#[test]
fn unparseable_matrix_fails_with_exit_one() {
    let fx = Fixture::new("bad-toml");
    fx.write_matrix("[meta\nschema_version = 1\n");
    let out = run_audit(&fx.root);
    assert_eq!(out.code, Some(1), "parse failure is a failed audit:\n{}", out.stdout);
    assert!(out.stdout.contains("parse:"), "{}", out.stdout);
}

#[test]
fn missing_matrix_is_xtask_error() {
    let fx = Fixture::new("no-matrix");
    let out = run_audit(&fx.root);
    assert_eq!(out.code, Some(2), "unreadable matrix is an xtask error:\n{}", out.stdout);
}
