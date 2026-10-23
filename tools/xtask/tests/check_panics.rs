//! Fixture-based acceptance proof for `cargo xtask check-panics`
//! (card 7-e, audit F-5): seeded panic-family sites in a scratch
//! workspace are caught, the sanctioned zones pass clean, and the
//! cited-invariant allowance is path+message scoped, never a blanket
//! file pass.
//!
//! Each test builds a throwaway scratch workspace under the system
//! temp dir — the nine scanned crates' `src/` trees (the checker only
//! reads sources, no manifest needed) — then runs the real xtask
//! binary with `--root <scratch>` (the flag exists for exactly this
//! proof; the real workspace needs no flag). Nothing here touches the
//! checked-in tree, so the seeded violations prove the gate's teeth
//! without any mutation-and-revert dance on real engine files.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// testkit's `Must`, locally bound: the workspace denies `unwrap`/`expect`
/// everywhere, tests included (`unreachable!` is the sanctioned test-only
/// escape — precisely the asymmetry audit F-5 is about).
trait Must {
    type Value;
    fn must(self) -> Self::Value;
}
impl<T, E: std::fmt::Display> Must for Result<T, E> {
    type Value = T;
    fn must(self) -> T {
        match self {
            Ok(v) => v,
            Err(e) => unreachable!("fixture setup failed: {e}"),
        }
    }
}
impl<T> Must for Option<T> {
    type Value = T;
    fn must(self) -> T {
        match self {
            Some(v) => v,
            None => unreachable!("fixture setup failed: None"),
        }
    }
}

struct RunOutcome {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run_checker(root: &Path) -> RunOutcome {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_xtask"));
    cmd.arg("check-panics").arg("--root").arg(root);
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
        let root = std::env::temp_dir().join(format!(
            "lumen-panics-fixture-{}-{name}",
            std::process::id()
        ));
        match fs::remove_dir_all(&root) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => unreachable!("cleaning stale fixture {}: {e}", root.display()),
        }
        fs::create_dir_all(&root).must();
        Fixture { root }
    }

    /// Create `crates/<name>/src` for all nine scanned crates.
    fn all_nine(&self) {
        for name in [
            "playback-engine",
            "media-clock",
            "packet-queue",
            "demux",
            "codecs",
            "audio-engine",
            "video-engine",
            "input",
            "media-probe",
        ] {
            fs::create_dir_all(self.root.join("crates").join(name).join("src")).must();
        }
    }

    /// Write `crates/<crate>/src/<file>` with `body`.
    fn file(&self, krate: &str, file: &str, body: &str) {
        let path = self.root.join("crates").join(krate).join("src").join(file);
        fs::create_dir_all(path.parent().must()).must();
        fs::write(path, body).must();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn seeded_non_test_site_fails() {
    let fx = Fixture::new("seeded");
    fx.all_nine();
    fx.file(
        "media-clock",
        "lib.rs",
        "fn fine() -> u8 { 1 }\nfn seeded() -> u8 { unreachable!(\"seeded 7-e demo\") }\n",
    );
    let out = run_checker(&fx.root);
    assert_eq!(out.code, Some(1), "exit 1 on a violation\n{}", out.stdout);
    assert!(out.stdout.contains("FAIL — 1 violation"), "{}", out.stdout);
    assert!(
        out.stdout.contains("media-clock/src/lib.rs") && out.stdout.contains(":2:"),
        "the report names the exact site: {}",
        out.stdout
    );
}

#[test]
fn sanctioned_zones_pass_clean() {
    let fx = Fixture::new("zones");
    fx.all_nine();
    // Doc examples and string contents never fire (stripped).
    fx.file(
        "demux",
        "lib.rs",
        "//! doc example: `.unwrap()` and unreachable!(\"in prose\") are fine\n\
         //! ```text\n\
         //! let v = Some(1).unwrap();\n\
         //! ```\n\
         fn production() -> u8 { 1 }\n\
         #[cfg(test)]\n\
         mod tests {\n\
             fn helper() -> u8 { let v = Some(1).unwrap(); v }\n\
             #[test]\n\
             fn t() { assert_eq!(helper(), 1); }\n\
         }\n",
    );
    // The F-2 executable-contract module zone.
    fx.file(
        "input",
        "contract.rs",
        "pub fn contract_only() { unreachable!(\"contract vocabulary\") }\n",
    );
    // A cited invariant: path AND message fragment both match.
    fx.file(
        "audio-engine",
        "pipeline.rs",
        "fn cold_path() {\n    let _ = unreachable!(\"fresh SharedBytes has refcount 1\");\n}\n",
    );
    let out = run_checker(&fx.root);
    assert_eq!(out.code, Some(0), "clean tree passes\n{}", out.stdout);
    assert!(out.stdout.contains("PASS — 0 violations"), "{}", out.stdout);
}

#[test]
fn cited_invariant_allowance_is_scoped_to_the_message() {
    let fx = Fixture::new("scoped");
    fx.all_nine();
    // Right path, WRONG message: the citation must not become a
    // blanket file pass.
    fx.file(
        "audio-engine",
        "pipeline.rs",
        "fn sneaky() { unreachable!(\"some other message\") }\n",
    );
    let out = run_checker(&fx.root);
    assert_eq!(
        out.code,
        Some(1),
        "path alone does not pass\n{}",
        out.stdout
    );
    assert!(out.stdout.contains("FAIL"), "{}", out.stdout);
}

#[test]
fn a_missing_scanned_crate_is_an_error_not_a_scope_shrink() {
    let fx = Fixture::new("shrink");
    // Only one of the nine crates exists.
    fx.file("media-clock", "lib.rs", "fn fine() -> u8 { 1 }\n");
    let out = run_checker(&fx.root);
    assert_eq!(out.code, Some(2), "exit 2 on scope shrink\n{}", out.stdout);
    assert!(
        out.stdout.contains("no src/ tree") || out.stderr.contains("no src/ tree"),
        "the error names the missing crate: {}{}",
        out.stdout,
        out.stderr
    );
}
