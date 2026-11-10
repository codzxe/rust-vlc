//! Binary-surface integration tests for `schema-gen` (card 1.B2).
//!
//! These run the compiled generator binary against the REAL committed
//! files (workspace root discovery walks up from this crate) and against
//! on-disk fixtures in a temp dir — proving the acceptance criteria at the
//! CLI level:
//!
//! * `--check` on the committed outputs exits 0 (no drift);
//! * regeneration is idempotent (two runs, byte-identical outputs);
//! * a duplicate-option fixture FAILS with an actionable diagnostic naming
//!   the rule (D2);
//! * drift (a stale committed output) exits 1.

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

struct RunOutcome {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run_bin(args: &[&str]) -> RunOutcome {
    match Command::new(env!("CARGO_BIN_EXE_schema-gen"))
        .args(args)
        .output()
    {
        Ok(out) => RunOutcome {
            code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        },
        Err(err) => RunOutcome {
            code: None,
            stdout: String::new(),
            stderr: format!("failed to spawn schema-gen: {err}"),
        },
    }
}

/// The Lumen workspace root (this file lives in crates/settings-schema/tests).
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A unique-enough scratch dir per test-binary run.
fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lumen-schema-gen-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).ok();
    dir
}

/// Two clean entries; the duplicate test mutates key #2 into key #1.
const CLEAN_FIXTURE: &str = r#"
[areas]
known = ["playback"]

[[setting]]
key = "playback.subtitle-mode"
type = "enum"
enumeration = ["auto", "off"]
default_ref = "playback.subtitle_mode"
description = "settings.playback.subtitle-mode"
restart_required = false
placement = { screen = "playback", section = "tracks", order = 10, ui = true }
cli = { flag = "--playback-subtitle-mode", short = "" }
feature_gate = ""
since = "0.1.0"

[[setting]]
key = "playback.start-paused"
type = "bool"
default = false
description = "settings.playback.start-paused"
restart_required = false
placement = { screen = "playback", section = "playback", order = 20, ui = true }
cli = { flag = "--playback-start-paused", short = "" }
feature_gate = ""
since = "0.1.0"
"#;

const PRODUCT_FIXTURE: &str = r#"
[playback]
subtitle_mode = "auto"
"#;

fn write_fixture(dir: &Path, schema: &str, product: &str) -> (PathBuf, PathBuf) {
    let schema_path = dir.join("settings.toml");
    let product_path = dir.join("product.toml");
    let schema_ok = fs::write(&schema_path, schema).is_ok();
    let product_ok = fs::write(&product_path, product).is_ok();
    assert!(schema_ok && product_ok, "writing fixture files");
    (schema_path, product_path)
}

fn read(path: &Path) -> String {
    match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => format!("<unreadable: {err}>"),
    }
}

#[test]
fn committed_outputs_have_no_drift() {
    let out = run_bin(&["--check"]);
    assert_eq!(
        out.code,
        Some(0),
        "stdout: {}\nstderr: {}",
        out.stdout,
        out.stderr
    );
    assert!(out.stdout.contains("no drift"), "stdout: {}", out.stdout);
    // The entry count is DERIVED from the seed (its `[[setting]]` rows),
    // never pinned — the seed is the single source of truth, so this
    // cannot go stale when the seed legitimately grows (phase-7 exit
    // gate OF-2: c71cd5a's 27→32 growth stranded a pinned literal here).
    assert!(
        out.stdout.contains(&format!("{} entries", committed_seed_entry_count())),
        "stdout: {}",
        out.stdout
    );
}

/// The committed seed's `[[setting]]` row count, read straight from
/// `schemas/settings.toml` with an independent raw-TOML parse (the crate
/// under test must not be asked what its own input contains).
fn committed_seed_entry_count() -> usize {
    let text = fs::read_to_string(repo_root().join("schemas/settings.toml"))
        .unwrap_or_else(|err| unreachable!("reading the committed seed: {err}"));
    let table = toml::from_str::<toml::Table>(&text)
        .unwrap_or_else(|err| unreachable!("parsing the committed seed: {err}"));
    table
        .get("setting")
        .and_then(toml::Value::as_array)
        .map_or(0, Vec::len)
}

/// Phase-7 exit gate OF-2 guard: the seed's own shape must be exactly what
/// every committed generated artifact self-reports. If the seed grows (or
/// changes shape) without regeneration, THIS fails first, naming the two
/// regeneration commands — the drift class `c71cd5a` introduced (seed grew
/// 27→32; the Rust artifacts were regenerated but three test pins and the
/// UI artifact were left stale, and nothing tied the artifacts' own
/// self-reported counts back to the seed at test time).
#[test]
fn seed_shape_is_what_every_committed_artifact_self_reports() {
    let root = repo_root();
    let text = fs::read_to_string(root.join("schemas/settings.toml"))
        .unwrap_or_else(|err| unreachable!("reading the committed seed: {err}"));
    let table = toml::from_str::<toml::Table>(&text)
        .unwrap_or_else(|err| unreachable!("parsing the committed seed: {err}"));
    let entries = table
        .get("setting")
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| unreachable!("the seed carries no [[setting]] rows"));
    let known_areas = table
        .get("areas")
        .and_then(toml::Value::as_table)
        .and_then(|areas| areas.get("known"))
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| unreachable!("the seed carries no [areas] known registry"));
    fn screen_of(entry: &toml::Value) -> &str {
        entry
            .get("placement")
            .and_then(toml::Value::as_table)
            .and_then(|placement| placement.get("screen"))
            .and_then(toml::Value::as_str)
            .unwrap_or_default()
    }
    let mut screens: Vec<&str> = entries.iter().map(|entry| screen_of(entry)).collect();
    assert_eq!(
        screens.len(),
        entries.len(),
        "every [[setting]] row must name a placement.screen"
    );
    screens.sort_unstable();
    screens.dedup();

    // G1 (`schemas/generated/settings.d.ts`) self-reports all three counts
    // in its header comment.
    let types = read(&root.join("schemas/generated/settings.d.ts"));
    let header = format!(
        "// {} entries across {} screen(s) ({} known areas).",
        entries.len(),
        screens.len(),
        known_areas.len()
    );
    assert!(
        types.contains(&header),
        "settings.d.ts must self-report the seed's shape ({header}) — regenerate with \
         `cargo xtask schema-gen`, and the UI side with `npm run settings:generate` in ui/"
    );

    // G2 (`schemas/generated/settings-help.txt`): the Areas index lists
    // exactly the screens in use, each with its true per-screen count.
    let help = read(&root.join("schemas/generated/settings-help.txt"));
    for screen in &screens {
        let count = entries
            .iter()
            .filter(|entry| screen_of(entry) == *screen)
            .count();
        let plural = if count == 1 { "option" } else { "options" };
        let row = format!("  {screen:<14} {count} {plural}");
        assert!(
            help.contains(&row),
            "settings-help.txt must list `{row}` — regenerate with `cargo xtask schema-gen`"
        );
    }
}

#[test]
fn regeneration_is_idempotent_byte_for_byte() {
    let dir = temp_dir("idem");
    let (schema, product) = write_fixture(&dir, CLEAN_FIXTURE, PRODUCT_FIXTURE);
    let out1 = dir.join("run1");
    let out2 = dir.join("run2");
    let generate_into = |out_dir: &Path| {
        run_bin(&[
            "--source",
            schema.to_str().unwrap_or_default(),
            "--product",
            product.to_str().unwrap_or_default(),
            "--out-dir",
            out_dir.to_str().unwrap_or_default(),
        ])
    };
    let first = generate_into(&out1);
    assert_eq!(first.code, Some(0), "stderr: {}", first.stderr);
    let second = generate_into(&out2);
    assert_eq!(second.code, Some(0), "stderr: {}", second.stderr);

    for name in ["settings.d.ts", "settings-help.txt", "settings-docs.md"] {
        let a = read(&out1.join(name));
        let b = read(&out2.join(name));
        assert_eq!(a, b, "{name} must be byte-identical across runs");
        assert!(!a.is_empty(), "{name} must not be empty");
    }

    // And the freshly generated pair matches a --check against itself.
    let check = run_bin(&[
        "--source",
        schema.to_str().unwrap_or_default(),
        "--product",
        product.to_str().unwrap_or_default(),
        "--ts-out",
        out1.join("settings.d.ts").to_str().unwrap_or_default(),
        "--help-out",
        out1.join("settings-help.txt").to_str().unwrap_or_default(),
        "--docs-out",
        out1.join("settings-docs.md").to_str().unwrap_or_default(),
        "--check",
    ]);
    assert_eq!(check.code, Some(0), "stderr: {}", check.stderr);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn duplicate_option_fixture_fails_with_rule_d2() {
    let dir = temp_dir("dup");
    let duplicated = CLEAN_FIXTURE.replace(
        "key = \"playback.start-paused\"",
        "key = \"playback.subtitle-mode\"",
    );
    let (schema, product) = write_fixture(&dir, &duplicated, PRODUCT_FIXTURE);
    let out = run_bin(&[
        "--source",
        schema.to_str().unwrap_or_default(),
        "--product",
        product.to_str().unwrap_or_default(),
        "--out-dir",
        dir.join("out").to_str().unwrap_or_default(),
    ]);
    assert_eq!(
        out.code,
        Some(2),
        "duplicate fixture must fail (exit 2), stderr:\n{}",
        out.stderr
    );
    assert!(
        out.stderr.contains("error[D2]"),
        "diagnostic must name the rule:\n{}",
        out.stderr
    );
    assert!(
        out.stderr
            .contains("duplicate setting key `playback.subtitle-mode`"),
        "diagnostic must be actionable:\n{}",
        out.stderr
    );
    assert!(
        out.stderr.contains("first defined at line"),
        "diagnostic must point at the first definition:\n{}",
        out.stderr
    );
    // Nothing is written when validation fails.
    assert!(
        !dir.join("out").join("settings.d.ts").is_file(),
        "no outputs may be written on a failed validation"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn stale_output_drift_fails_check_with_exit_one() {
    let dir = temp_dir("drift");
    let (schema, product) = write_fixture(&dir, CLEAN_FIXTURE, PRODUCT_FIXTURE);
    let out_dir = dir.join("out");
    let generate = run_bin(&[
        "--source",
        schema.to_str().unwrap_or_default(),
        "--product",
        product.to_str().unwrap_or_default(),
        "--out-dir",
        out_dir.to_str().unwrap_or_default(),
    ]);
    assert_eq!(generate.code, Some(0), "stderr: {}", generate.stderr);
    // Corrupt one committed-looking output, then --check must flag drift.
    let help_path = out_dir.join("settings-help.txt");
    let stale = read(&help_path).replace("27 entries", "999 entries");
    // The fixture only has 2 entries; any hand edit is drift:
    let stale = stale.replace("playback", "playbackx");
    fs::write(&help_path, stale).ok();
    let check = run_bin(&[
        "--source",
        schema.to_str().unwrap_or_default(),
        "--product",
        product.to_str().unwrap_or_default(),
        "--ts-out",
        out_dir.join("settings.d.ts").to_str().unwrap_or_default(),
        "--help-out",
        help_path.to_str().unwrap_or_default(),
        "--docs-out",
        out_dir
            .join("settings-docs.md")
            .to_str()
            .unwrap_or_default(),
        "--check",
    ]);
    assert_eq!(
        check.code,
        Some(1),
        "drift must exit 1, stderr:\n{}",
        check.stderr
    );
    assert!(
        check
            .stderr
            .contains("differs from the regenerated output at line"),
        "drift must be actionable:\n{}",
        check.stderr
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn missing_schema_file_is_an_io_error_not_a_panic() {
    let out = run_bin(&[
        "--source",
        "/nonexistent/path/settings.toml",
        "--product",
        "/nonexistent/path/product.toml",
        "--out-dir",
        "/nonexistent/out",
    ]);
    assert_eq!(out.code, Some(2), "stderr: {}", out.stderr);
    assert!(
        out.stderr.contains("reading"),
        "I/O failure must be reported:\n{}",
        out.stderr
    );
}

#[test]
fn help_flag_prints_usage_and_exits_zero() {
    let out = run_bin(&["--help"]);
    assert_eq!(out.code, Some(0), "stderr: {}", out.stderr);
    assert!(
        out.stdout.contains("Usage: schema-gen [OPTIONS]"),
        "{}",
        out.stdout
    );
    assert!(out.stdout.contains("--check"), "{}", out.stdout);
}

/// The generated artifacts of the committed seed must exist in the repo
/// (this test pins the G1/G2/G3 file contract of card 1.B2).
#[test]
fn committed_generated_files_exist() {
    let root = repo_root();
    for name in [
        "schemas/generated/settings.d.ts",
        "schemas/generated/settings-help.txt",
        "schemas/generated/settings-docs.md",
    ] {
        let path = root.join(name);
        assert!(path.is_file(), "missing committed artifact: {name}");
        assert!(!read(&path).is_empty(), "empty committed artifact: {name}");
    }
}
