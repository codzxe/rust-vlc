//! Fixture-based acceptance proof for `cargo xtask lint-literals` and
//! `cargo xtask check-identity` (card 7.w21a, doc 07 §9 C1/C2):
//! seeded config-plane literals in a scratch workspace are caught with
//! `file:line: kind` reports, the audited exemptions (config-layer
//! crates, tests/fixtures dirs, cfg(test) loopback URLs, the inline
//! `// lumen-allow-literal: <reason>` marker) pass clean AND are
//! printed, and the two scopes differ exactly as specified (C1 brand
//! trio; C2 adds the technical ids and the accent hex).
//!
//! Each test builds a throwaway scratch workspace under the system
//! temp dir — `crates/*/src` + `apps/*/src` (the scanner only reads
//! sources; no manifest needed) plus a fixture `product.toml` — then
//! runs the real xtask binary with `--root <scratch>` (the flag
//! exists for exactly this proof). Nothing here touches the
//! checked-in tree, so the seeded violations prove the gate's teeth
//! without any mutation-and-revert dance on real engine files.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// testkit's `Must`, locally bound: the workspace denies `unwrap`/`expect`
/// everywhere, tests included (`unreachable!` is the sanctioned test-only
/// escape).
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

/// Run the real xtask binary with `<subcommand> --root <root>`.
fn run_tool(subcommand: &str, root: &Path) -> RunOutcome {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_xtask"));
    cmd.arg(subcommand).arg("--root").arg(root);
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
            "lumen-literals-fixture-{}-{name}",
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

    /// Create the minimal scan domain: one crate + one app, both with
    /// `src/` trees, plus the identity-complete `product.toml`.
    fn minimal(&self) {
        fs::create_dir_all(self.root.join("crates").join("engine").join("src")).must();
        fs::create_dir_all(self.root.join("apps").join("host").join("src")).must();
        self.product_toml_full();
    }

    /// The identity-complete `product.toml` the gates read.
    fn product_toml_full(&self) {
        fs::write(
            self.root.join("product.toml"),
            "[identity]\n\
             name = \"Lumen\"\n\
             short_name = \"Lumen\"\n\
             publisher = \"Lumen Contributors\"\n\
             \n\
             [identity.ids]\n\
             reverse_dns = \"dev.lumen\"\n\
             \n\
             [identity.ids.windows]\n\
             app_user_model_id = \"dev.lumen.Lumen\"\n\
             msi_upgrade_code = \"7c1f6a52-9d3e-4b8a-a001-3f9c2d64e701\"\n\
             \n\
             [identity.ids.macos]\n\
             bundle_id = \"dev.lumen.Lumen\"\n\
             \n\
             [identity.ids.linux]\n\
             appstream_id = \"dev.lumen.Lumen\"\n\
             \n\
             [identity.ids.web]\n\
             app_id = \"dev.lumen.Lumen\"\n\
             \n\
             [brand]\n\
             accent_color = \"#4f8cff\"\n",
        )
        .must();
    }

    /// Write `crates/<crate>/src/<file>` (or `apps/<app>/src/<file>`)
    /// with `body`, creating directories as needed.
    fn file(&self, base: &str, krate: &str, file: &str, body: &str) {
        let path = self.root.join(base).join(krate).join("src").join(file);
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
fn seeded_production_literals_fail_with_file_line_kind() {
    let fx = Fixture::new("seeded");
    fx.minimal();
    fx.file(
        "crates",
        "engine",
        "lib.rs",
        "pub fn surfaces() -> [&'static str; 4] {\n    [\n        \"Welcome to Lumen\",\n        \"https://cdn.example.org/manifest.m3u8\",\n        \"/home/alice/video.mkv\",\n        \"#4f8cff\",\n    ]\n}\n",
    );
    let out = run_tool("lint-literals", &fx.root);
    assert_eq!(out.code, Some(1), "exit 1 on violations\n{}", out.stdout);
    assert!(
        out.stdout.contains("FAIL — 4 violation(s)"),
        "all four kinds fire: {}",
        out.stdout
    );
    for (needle, kind) in [
        ("crates/engine/src/lib.rs:3:", "identity"),
        ("crates/engine/src/lib.rs:4:", "url"),
        ("crates/engine/src/lib.rs:5:", "abs-path"),
        ("crates/engine/src/lib.rs:6:", "hex-color"),
    ] {
        assert!(
            out.stdout.contains(&format!("{needle} {kind}")),
            "the report names {} as {} kind: {}",
            needle,
            kind,
            out.stdout
        );
    }
}

#[test]
fn audited_exemptions_pass_clean_and_are_printed() {
    let fx = Fixture::new("exemptions");
    fx.minimal();
    // Production crate: one marked identity site, one inline cfg(test)
    // loopback URL, and a file-level `#[cfg(test)] mod helper;` pull.
    fx.file(
        "crates",
        "engine",
        "lib.rs",
        "pub fn argv0() -> &'static str {\n    \"lumen\" // lumen-allow-literal: argv0 token; identity wiring is the open proposal\n}\n\n#[cfg(test)]\nmod helper;\n\n#[cfg(test)]\nmod tests {\n    fn local_feed() -> &'static str {\n        \"ws://localhost:9/feed\"\n    }\n}\n",
    );
    // Pulled in wholly as test context by the decl above.
    fx.file(
        "crates",
        "engine",
        "helper.rs",
        "fn remote_feed() -> &'static str {\n    \"http://127.0.0.1:8080/feed\"\n}\n",
    );
    // A `tests` path component: exempt by category.
    fx.file(
        "crates",
        "engine",
        "tests/vectors.rs",
        "fn scratch() -> &'static str {\n    \"/tmp/lumen-scratch.bin\"\n}\n",
    );
    // The config layer itself: its literals ARE the source.
    fx.file(
        "crates",
        "product-config",
        "lib.rs",
        "pub const NAME: &str = \"Lumen\";\npub const ACCENT: &str = \"#4f8cff\";\n",
    );
    // A clean app.
    fx.file("apps", "host", "main.rs", "fn main() {}\n");
    let out = run_tool("lint-literals", &fx.root);
    assert_eq!(out.code, Some(0), "audited exemptions pass\n{}", out.stdout);
    assert!(
        out.stdout.contains("PASS — 0 violations, 3 allowlisted"),
        "the marker, the inline cfg(test) loopback and the pulled-in \
         helper's loopback are all counted: {}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains("allowed: crates/engine/src/lib.rs:2: identity — argv0 token"),
        "the inline marker's reason is printed verbatim: {}",
        out.stdout
    );
    assert!(
        out.stdout
            .matches("loopback URL inside #[cfg(test)] context")
            .count()
            == 2,
        "both loopback sites carry the category reason (inline mod + pulled file): {}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains("(1 config-layer exempt, 1 tests/fixtures-dir exempt)"),
        "the category totals are reported: {}",
        out.stdout
    );
    // check-identity shares the marker verdict: the same tree passes C2.
    let out2 = run_tool("check-identity", &fx.root);
    assert_eq!(
        out2.code,
        Some(0),
        "the marker covers C2 too\n{}",
        out2.stdout
    );
}

#[test]
fn c2_adds_technical_ids_and_accent_that_c1_leaves_alone() {
    let fx = Fixture::new("scopes");
    fx.minimal();
    // Production literal: the MSI GUID, no brand-trio word anywhere.
    fx.file(
        "crates",
        "engine",
        "lib.rs",
        "pub fn upgrade_code() -> &'static str {\n    \"upgrade:7c1f6a52-9d3e-4b8a-a001-3f9c2d64e701\"\n}\n",
    );
    let c1 = run_tool("lint-literals", &fx.root);
    assert_eq!(
        c1.code,
        Some(0),
        "C1 fires only on the brand trio — the GUID is C2's business:\n{}",
        c1.stdout
    );
    let c2 = run_tool("check-identity", &fx.root);
    assert_eq!(
        c2.code,
        Some(1),
        "C2 catches what C1 deliberately leaves alone\n{}",
        c2.stdout
    );
    assert!(
        c2.stdout.contains("crates/engine/src/lib.rs:2: identity")
            && c2.stdout.contains("identity.ids.windows.msi_upgrade_code"),
        "the report names the id it matched: {}",
        c2.stdout
    );

    // The accent hex: a hex-color finding under C1, an identity
    // finding (still a hex-color) under C2 — the same site GAINS a
    // kind, it never loses one.
    let fx2 = Fixture::new("accent");
    fx2.minimal();
    fx2.file(
        "crates",
        "engine",
        "lib.rs",
        "pub fn accent() -> &'static str {\n    \"#4F8CFF\"\n}\n",
    );
    let accent_c1 = run_tool("lint-literals", &fx2.root);
    assert_eq!(
        accent_c1.code,
        Some(1),
        "C1: a bare hex color fires on its own kind\n{}",
        accent_c1.stdout
    );
    assert!(
        accent_c1
            .stdout
            .contains("crates/engine/src/lib.rs:2: hex-color"),
        "C1 reports the plain hex-color kind: {}",
        accent_c1.stdout
    );
    let accent_c2 = run_tool("check-identity", &fx2.root);
    assert_eq!(
        accent_c2.code,
        Some(1),
        "C2 stays red\n{}",
        accent_c2.stdout
    );
    assert!(
        accent_c2
            .stdout
            .contains("crates/engine/src/lib.rs:2: identity — identity(brand.accent_color)"),
        "under C2 the accent is identity: {}",
        accent_c2.stdout
    );
}

#[test]
fn url_rules_are_strict_about_test_context_and_hosts() {
    let fx = Fixture::new("urls");
    fx.minimal();
    fx.file(
        "crates",
        "engine",
        "lib.rs",
        "pub fn production_endpoint() -> &'static str {\n    \"http://127.0.0.1:8080/control\"\n}\n\n#[cfg(test)]\nmod tests {\n    fn double() -> &'static str {\n        \"https://unit.test/feed\"\n    }\n}\n",
    );
    let out = run_tool("lint-literals", &fx.root);
    assert_eq!(
        out.code,
        Some(1),
        "a loopback URL outside test context AND a non-loopback URL \
         inside one both fire\n{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("FAIL — 2 violation(s)"),
        "exactly the two URL findings: {}",
        out.stdout
    );
    assert!(
        out.stdout.contains("crates/engine/src/lib.rs:2: url")
            && out.stdout.contains("crates/engine/src/lib.rs:8: url"),
        "both sites are named: {}",
        out.stdout
    );
}

#[test]
fn a_marker_without_a_reason_suppresses_nothing() {
    let fx = Fixture::new("reasonless");
    fx.minimal();
    fx.file(
        "crates",
        "engine",
        "lib.rs",
        "// lumen-allow-literal:\npub fn name() -> &'static str {\n    \"Lumen\"\n}\n",
    );
    let out = run_tool("lint-literals", &fx.root);
    assert_eq!(
        out.code,
        Some(1),
        "a reasonless marker is a comment, not an exemption\n{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("crates/engine/src/lib.rs:3: identity"),
        "the site still fires: {}",
        out.stdout
    );
}

#[test]
fn a_broken_identity_source_is_an_xtask_error() {
    // Missing product.toml entirely.
    let fx = Fixture::new("no-toml");
    fs::create_dir_all(fx.root.join("crates").join("engine").join("src")).must();
    fs::create_dir_all(fx.root.join("apps").join("host").join("src")).must();
    fx.file("crates", "engine", "lib.rs", "fn clean() -> u8 { 1 }\n");
    let out = run_tool("lint-literals", &fx.root);
    assert_eq!(out.code, Some(2), "exit 2, not a fake pass\n{}", out.stdout);
    assert!(
        out.stdout.contains("reading") || out.stderr.contains("reading"),
        "the error names the missing file: {}{}",
        out.stdout,
        out.stderr
    );
    // Identity-incomplete product.toml.
    let fx2 = Fixture::new("half-toml");
    fs::create_dir_all(fx2.root.join("crates").join("engine").join("src")).must();
    fs::create_dir_all(fx2.root.join("apps").join("host").join("src")).must();
    fx2.file("crates", "engine", "lib.rs", "fn clean() -> u8 { 1 }\n");
    fs::write(
        fx2.root.join("product.toml"),
        "[identity]\nname = \"Lumen\"\n",
    )
    .must();
    let out2 = run_tool("check-identity", &fx2.root);
    assert_eq!(
        out2.code,
        Some(2),
        "a gate whose premise is product.toml cannot guess it\n{}",
        out2.stdout
    );
    assert!(
        out2.stdout.contains("identity.short_name") || out2.stderr.contains("identity.short_name"),
        "every missing field is named at once: {}{}",
        out2.stdout,
        out2.stderr
    );
}

#[test]
fn a_shrinking_scan_domain_is_an_error_not_a_smaller_pass() {
    let fx = Fixture::new("shrink");
    // crates/ exists but apps/ does not: the domain must not silently
    // shrink to whatever is present.
    fs::create_dir_all(fx.root.join("crates").join("engine").join("src")).must();
    fx.product_toml_full();
    fx.file("crates", "engine", "lib.rs", "fn clean() -> u8 { 1 }\n");
    let out = run_tool("lint-literals", &fx.root);
    assert_eq!(out.code, Some(2), "exit 2 on scope shrink\n{}", out.stdout);
    assert!(
        out.stdout.contains("apps/") || out.stderr.contains("apps/"),
        "the error names the missing base: {}{}",
        out.stdout,
        out.stderr
    );
}
