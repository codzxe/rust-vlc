//! Fixture-based acceptance proof for `cargo xtask check-architecture`
//! (card 1.E1 AC: seeded violations in scratch crates are all caught).
//!
//! Each test builds a THROWAWAY scratch workspace under the system temp
//! dir — a root manifest, a `boundaries.toml`, and tiny member crates —
//! then runs the real xtask binary with `--root <scratch>` (the flag
//! exists for exactly this proof; the real workspace needs no flag).
//! Nothing here touches the checked-in tree, so the seeded violations
//! prove the checker's teeth without any mutation-and-revert dance.
//!
//! The scratch workspaces deliberately do NOT need to be buildable by
//! cargo (the checker reads manifests directly): the dev-edge cycle
//! fixture, for one, is a graph cargo would resolve but never reject.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct RunOutcome {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run_checker(root: &Path, extra_args: &[&str]) -> RunOutcome {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_xtask"));
    cmd.arg("check-architecture").arg("--root").arg(root);
    cmd.args(extra_args);
    match cmd.output() {
        Ok(out) => RunOutcome {
            code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        },
        Err(err) => RunOutcome {
            code: None,
            stdout: String::new(),
            stderr: format!("failed to spawn xtask binary: {err}"),
        },
    }
}

/// A scratch workspace that removes itself when dropped.
struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Fixture {
        let root =
            std::env::temp_dir().join(format!("lumen-arch-fixture-{}-{name}", std::process::id()));
        match fs::remove_dir_all(&root) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => unreachable!("cleaning stale fixture {}: {e}", root.display()),
        }
        match fs::create_dir_all(&root) {
            Ok(()) => {}
            Err(e) => unreachable!("creating fixture {}: {e}", root.display()),
        }
        Fixture { root }
    }

    fn write(&self, rel: &str, contents: &str) {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            match fs::create_dir_all(parent) {
                Ok(()) => {}
                Err(e) => unreachable!("creating {}: {e}", parent.display()),
            }
        }
        match fs::write(&path, contents) {
            Ok(()) => {}
            Err(e) => unreachable!("writing {}: {e}", path.display()),
        }
    }

    fn run(&self) -> RunOutcome {
        run_checker(&self.root, &[])
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Best-effort cleanup; a failure never fails the test.
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A root manifest with the workspace lint table (the passing shape).
const ROOT_MANIFEST: &str = "\
[workspace]
members = [\"crates/*\"]

[workspace.lints.rust]
unsafe_code = \"deny\"
";

/// A crate manifest that inherits the workspace lints and has no deps.
fn clean_crate(name: &str) -> String {
    format!(
        "\
[package]
name = \"{name}\"
version = \"0.1.0\"
edition = \"2024\"

[lints]
workspace = true
"
    )
}

/// The passing boundaries manifest for the standard three-crate fixture
/// (alpha L1 -> beta L0, testkit dev-only).
const CLEAN_BOUNDARIES: &str = "\
[manifest]
version = 1

[rules]
same_layer_pairs = []
dev_only = [\"testkit\"]
static_mut_allow = []

[crates.alpha]
layer = 1
internal_deps = [\"beta\"]
dev_deps = [\"testkit\"]
external_allowlist = [\"serde\"]
external_dev_allowlist = []

[crates.beta]
layer = 0
internal_deps = []
dev_deps = []
external_allowlist = []
external_dev_allowlist = []

[crates.testkit]
internal_deps = []
dev_deps = []
external_allowlist = []
external_dev_allowlist = []
";

/// Standard three-crate scratch workspace: alpha -> beta (allowed, L1->L0),
/// alpha dev-> testkit, alpha external serde, plus scanner decoys (a
/// comment and a string literal mentioning the forbidden words).
fn build_clean_fixture(fx: &Fixture) {
    fx.write("Cargo.toml", ROOT_MANIFEST);
    fx.write("boundaries.toml", CLEAN_BOUNDARIES);
    fx.write(
        "crates/alpha/Cargo.toml",
        "\
[package]
name = \"alpha\"
version = \"0.1.0\"
edition = \"2024\"

[lints]
workspace = true

[dependencies]
beta = { path = \"../beta\" }
serde = \"1\"

[dev-dependencies]
testkit = { path = \"../testkit\" }
",
    );
    fx.write(
        "crates/alpha/src/lib.rs",
        "// decoy: the words static mut in a comment must not trip the scanner.\n\
         /// doc decoy: static mut here too.\n\
         pub const DECOY: &str = \"static mut inside a string literal\";\n\
         pub fn alpha() {}\n",
    );
    fx.write("crates/beta/Cargo.toml", &clean_crate("beta"));
    fx.write("crates/beta/src/lib.rs", "pub fn beta() {}\n");
    fx.write("crates/testkit/Cargo.toml", &clean_crate("testkit"));
    fx.write("crates/testkit/src/lib.rs", "pub fn toolkit() {}\n");
}

#[test]
fn clean_scratch_workspace_passes_all_five_checks() {
    let fx = Fixture::new("clean");
    build_clean_fixture(&fx);
    let out = fx.run();
    assert_eq!(
        out.code,
        Some(0),
        "stdout:\n{}\nstderr:\n{}",
        out.stdout,
        out.stderr
    );
    assert!(
        out.stdout.contains("result: PASS (5 of 5 checks)"),
        "stdout:\n{}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains("architecture: result=pass edges=pass cycles=pass unsafe-lints=pass static-mut=pass testkit-direction=pass"),
        "machine line wrong:\n{}",
        out.stdout
    );
    // The decoys prove no false positives: the static-mut check passed
    // although the source mentions the forbidden words twice.
    assert!(
        out.stdout.contains("static-mut") && out.stdout.contains("PASS"),
        "stdout:\n{}",
        out.stdout
    );
}

#[test]
fn clean_run_writes_the_json_report_artifact() {
    let fx = Fixture::new("json-artifact");
    build_clean_fixture(&fx);
    let out = run_checker(&fx.root, &["--json"]);
    assert_eq!(out.code, Some(0), "stdout:\n{}", out.stdout);
    let report_path = fx.root.join("target/architecture-report.json");
    match fs::read_to_string(&report_path) {
        Ok(json) => {
            assert!(
                json.contains("\"schema\": \"architecture-report/1\""),
                "{json}"
            );
            assert!(json.contains("\"result\": \"pass\""), "{json}");
        }
        Err(e) => unreachable!("report artifact missing ({}): {e}", report_path.display()),
    }
    assert!(
        out.stdout.contains("\"schema\": \"architecture-report/1\""),
        "--json must echo the report:\n{}",
        out.stdout
    );
}

/// Seeded violation (a): forbidden internal edge — alpha -> beta exists but
/// `[crates.alpha].internal_deps` does not allow it.
#[test]
fn forbidden_internal_edge_is_caught() {
    let fx = Fixture::new("forbidden-edge");
    build_clean_fixture(&fx);
    fx.write(
        "boundaries.toml",
        &CLEAN_BOUNDARIES.replace(
            "[crates.alpha]\nlayer = 1\ninternal_deps = [\"beta\"]",
            "[crates.alpha]\nlayer = 1\ninternal_deps = []",
        ),
    );
    let out = fx.run();
    assert_eq!(
        out.code,
        Some(1),
        "stdout:\n{}\nstderr:\n{}",
        out.stdout,
        out.stderr
    );
    assert!(
        out.stdout
            .contains("`alpha` depends on `beta`, which is not in [crates.alpha].internal_deps"),
        "stdout:\n{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("[EDGE]")
            && out
                .stdout
                .contains("result: FAILURE (1 of 5 checks failed)"),
        "stdout:\n{}",
        out.stdout
    );
}

/// Seeded violation (a'): the edge IS allowlisted but points UP the layer
/// stack (L0 -> L1) — doc 01 §1.4 invariant 1.
#[test]
fn upward_layer_edge_is_caught() {
    let fx = Fixture::new("upward-edge");
    build_clean_fixture(&fx);
    fx.write(
        "boundaries.toml",
        &CLEAN_BOUNDARIES
            .replace("[crates.alpha]\nlayer = 1", "[crates.alpha]\nlayer = 0")
            .replace("[crates.beta]\nlayer = 0", "[crates.beta]\nlayer = 1"),
    );
    let out = fx.run();
    assert_eq!(out.code, Some(1), "stdout:\n{}", out.stdout);
    assert!(
        out.stdout.contains("points upward (layer 0 -> 1)"),
        "stdout:\n{}",
        out.stdout
    );
    assert!(out.stdout.contains("[LAYER]"), "stdout:\n{}", out.stdout);
}

/// Seeded violation (a''): a same-layer edge whose pair is not whitelisted
/// in `[rules].same_layer_pairs`.
#[test]
fn unlisted_same_layer_edge_is_caught() {
    let fx = Fixture::new("same-layer");
    build_clean_fixture(&fx);
    // alpha -> beta same-layer 1 -> 1, no whitelist entry.
    fx.write(
        "boundaries.toml",
        &CLEAN_BOUNDARIES.replace("[crates.beta]\nlayer = 0", "[crates.beta]\nlayer = 1"),
    );
    let out = fx.run();
    assert_eq!(out.code, Some(1), "stdout:\n{}", out.stdout);
    assert!(
        out.stdout
            .contains("same-layer (1 -> 1) but the pair is not whitelisted"),
        "stdout:\n{}",
        out.stdout
    );
}

/// Seeded violation (b): a dev-edge dependency cycle (alpha dev-> beta,
/// beta -> alpha) — the class cargo itself will not reject.
#[test]
fn dev_edge_cycle_is_caught() {
    let fx = Fixture::new("cycle");
    build_clean_fixture(&fx);
    // beta now depends on alpha; alpha dev-depends on beta. Both edges are
    // allowlisted, and no layers are assigned, so ONLY the cycle fires.
    fx.write(
        "crates/beta/Cargo.toml",
        "\
[package]
name = \"beta\"
version = \"0.1.0\"
edition = \"2024\"

[lints]
workspace = true

[dependencies]
alpha = { path = \"../alpha\" }
",
    );
    fx.write(
        "crates/alpha/Cargo.toml",
        "\
[package]
name = \"alpha\"
version = \"0.1.0\"
edition = \"2024\"

[lints]
workspace = true

[dependencies]
beta = { path = \"../beta\" }

[dev-dependencies]
testkit = { path = \"../testkit\" }
beta2 = \"1\"
",
    );
    fx.write(
        "boundaries.toml",
        "\
[manifest]
version = 1

[rules]
same_layer_pairs = []
dev_only = [\"testkit\"]
static_mut_allow = []

[crates.alpha]
internal_deps = [\"beta\"]
dev_deps = [\"beta\", \"testkit\", \"beta2\"]
external_allowlist = []
external_dev_allowlist = [\"beta2\"]

[crates.beta]
internal_deps = [\"alpha\"]
dev_deps = []
external_allowlist = []
external_dev_allowlist = []

[crates.testkit]
internal_deps = []
dev_deps = []
external_allowlist = []
external_dev_allowlist = []
",
    );
    let out = fx.run();
    assert_eq!(out.code, Some(1), "stdout:\n{}", out.stdout);
    assert!(out.stdout.contains("[CYCLE]"), "stdout:\n{}", out.stdout);
    assert!(
        out.stdout.contains("alpha -> beta -> alpha")
            || out.stdout.contains("beta -> alpha -> beta"),
        "cycle path must be rendered:\n{}",
        out.stdout
    );
    // Only the cycle check failed.
    assert!(
        out.stdout
            .contains("result: FAILURE (1 of 5 checks failed)"),
        "stdout:\n{}",
        out.stdout
    );
}

/// Seeded violation (c): a member without `[lints] workspace = true` AND a
/// root that does not deny unsafe_code.
#[test]
fn missing_lints_inheritance_is_caught() {
    let fx = Fixture::new("lints");
    build_clean_fixture(&fx);
    fx.write(
        "crates/beta/Cargo.toml",
        &clean_crate("beta").replace("\n[lints]\nworkspace = true\n", "\n"),
    );
    fx.write(
        "Cargo.toml",
        "\
[workspace]
members = [\"crates/*\"]
",
    );
    let out = fx.run();
    assert_eq!(out.code, Some(1), "stdout:\n{}", out.stdout);
    assert!(
        out.stdout
            .contains("crates/beta/Cargo.toml does not inherit the workspace lint table"),
        "stdout:\n{}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains("root manifest does not deny unsafe_code"),
        "stdout:\n{}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains("result: FAILURE (1 of 5 checks failed)"),
        "only the unsafe-lints check should fail:\n{}",
        out.stdout
    );
}

/// Seeded violation (d): a real `static mut` declaration in a member src
/// tree (the clean fixture's decoys prove the scanner is not just
/// matching the words anywhere).
#[test]
fn static_mut_declaration_is_caught() {
    let fx = Fixture::new("static-mut");
    build_clean_fixture(&fx);
    fx.write(
        "crates/beta/src/lib.rs",
        "pub fn beta() {}\n\npub static mut COUNTER: u32 = 0;\n",
    );
    let out = fx.run();
    assert_eq!(out.code, Some(1), "stdout:\n{}", out.stdout);
    assert!(
        out.stdout.contains("[STATIC-MUT]"),
        "stdout:\n{}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains("crates/beta/src/lib.rs:3: `static mut` global mutable state"),
        "stdout:\n{}",
        out.stdout
    );
}

/// Seeded violation (e): `testkit` in a `[dependencies]` table — reported by
/// the dedicated ARCH-7 check even though the allowlist would permit the
/// edge (the dev-only rule is absolute).
#[test]
fn testkit_in_dependencies_is_caught() {
    let fx = Fixture::new("testkit-direction");
    build_clean_fixture(&fx);
    fx.write(
        "crates/alpha/Cargo.toml",
        "\
[package]
name = \"alpha\"
version = \"0.1.0\"
edition = \"2024\"

[lints]
workspace = true

[dependencies]
beta = { path = \"../beta\" }
testkit = { path = \"../testkit\" }
",
    );
    // The allowlist PERMITS the edge — only the dev-only direction rule
    // can catch it.
    fx.write(
        "boundaries.toml",
        &CLEAN_BOUNDARIES.replace(
            "internal_deps = [\"beta\"]\ndev_deps = [\"testkit\"]",
            "internal_deps = [\"beta\", \"testkit\"]\ndev_deps = [\"testkit\"]",
        ),
    );
    let out = fx.run();
    assert_eq!(out.code, Some(1), "stdout:\n{}", out.stdout);
    assert!(
        out.stdout
            .contains("dev-only crate `testkit` must not be a normal dependency"),
        "stdout:\n{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("[TESTKIT-DIRECTION]"),
        "stdout:\n{}",
        out.stdout
    );
}

/// Manifest integrity both ways: an undeclared workspace crate and a stale
/// `[crates.<name>]` entry for a crate that does not exist.
#[test]
fn undeclared_and_stale_manifest_entries_are_caught() {
    let fx = Fixture::new("manifest-integrity");
    build_clean_fixture(&fx);
    // beta exists on disk but is dropped from the manifest; ghost does not
    // exist but has an entry.
    fx.write(
        "boundaries.toml",
        "\
[manifest]
version = 1

[rules]
same_layer_pairs = []
dev_only = [\"testkit\"]
static_mut_allow = []

[crates.alpha]
layer = 1
internal_deps = [\"beta\"]
dev_deps = [\"testkit\"]
external_allowlist = [\"serde\"]
external_dev_allowlist = []

[crates.ghost]
internal_deps = []
dev_deps = []
external_allowlist = []
external_dev_allowlist = []

[crates.testkit]
internal_deps = []
dev_deps = []
external_allowlist = []
external_dev_allowlist = []
",
    );
    let out = fx.run();
    assert_eq!(out.code, Some(1), "stdout:\n{}", out.stdout);
    assert!(
        out.stdout
            .contains("workspace crate `beta` (crates/beta/Cargo.toml) is not declared"),
        "stdout:\n{}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains("declares [crates.ghost] but no workspace crate named `ghost` exists"),
        "stdout:\n{}",
        out.stdout
    );
}

/// Seeded violation: an external dependency not in the per-crate allowlist.
#[test]
fn unapproved_external_dependency_is_caught() {
    let fx = Fixture::new("external-dep");
    build_clean_fixture(&fx);
    fx.write(
        "boundaries.toml",
        &CLEAN_BOUNDARIES.replace(
            "external_allowlist = [\"serde\"]",
            "external_allowlist = []",
        ),
    );
    let out = fx.run();
    assert_eq!(out.code, Some(1), "stdout:\n{}", out.stdout);
    assert!(
        out.stdout
            .contains("external crate `serde` is not in [crates.alpha].external_allowlist"),
        "stdout:\n{}",
        out.stdout
    );
    assert!(out.stdout.contains("[EXTERNAL]"), "stdout:\n{}", out.stdout);
}

/// Usage errors are tool errors (exit 2), distinct from violations.
#[test]
fn usage_errors_exit_two() {
    let fx = Fixture::new("usage");
    build_clean_fixture(&fx);
    let out = run_checker(&fx.root, &["--root"]);
    assert_eq!(out.code, Some(2), "stderr:\n{}", out.stderr);
    assert!(
        out.stderr.contains("--root requires a directory argument"),
        "stderr:\n{}",
        out.stderr
    );
    let out = run_checker(&fx.root, &["--no-such-flag"]);
    assert_eq!(out.code, Some(2), "stderr:\n{}", out.stderr);
    assert!(
        out.stderr.contains("unexpected argument `--no-such-flag`"),
        "stderr:\n{}",
        out.stderr
    );
}
