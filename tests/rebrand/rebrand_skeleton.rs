//! Rebrand acceptance skeleton — card 1.E4 (doc 07 §8.3, fast tier).
//!
//! # Wiring (read this before moving the file)
//!
//! This file lives at the repo root (`tests/rebrand/`) because doc 07 §8.3
//! parks the rebrand regression suite in the repo-root tests/ tree, next
//! to its fixture (`nova.toml` + `assets/`). The root `Cargo.toml` is a
//! virtual workspace manifest, so cargo compiles no root-level test
//! target on its own; the file is pulled into the xtask package's `cli`
//! integration test via `#[path]` in `tools/xtask/tests/cli.rs` (card
//! 1.E4's owned test file). Consequence: `env!("CARGO_BIN_EXE_xtask")`
//! below resolves because the *xtask package* compiles this code — when
//! the root tests/ tree becomes a cargo target of its own (later phase),
//! give this file its own spawn helper and drop the `#[path]` include.
//!
//! # What the fast tier asserts (doc 07 §8.3 steps 1–2 + 5–6, v0 surface)
//!
//! 1. Run the rebrand tool on the Nova fixture into a temp dest (no full
//!    build, no working-tree mutation).
//! 2. (a) dest `product.toml` carries the Nova identity and matches the
//!    product.schema.json shapes for the identity fields (patterns +
//!    the derived-id cross-field rule of doc 07 §3.3(b), re-implemented
//!    std-only here — the config-validator wiring is card 1.E2).
//!    (b) dest `README.md` shows the Nova title and zero "Lumen".
//!    (c) the touched-surface report lists exactly the v0 surface set and
//!    the fixture's four brand assets.
//!    (d) reverse control: `--config product.toml` (the real one) is a
//!    byte-identical no-op.
//! 3. Error paths: missing config file, malformed TOML, missing identity
//!    fields — each an honest exit-2 with an actionable message.
//!
//! Card 4.H2 extensions (the §8.3 acceptance surfaces the build-time plane
//! can prove without a bundler): dest `apps/desktop/tauri/tauri.conf.json`
//! carries the Nova installer identity (productName / identifier / window
//! title — the installer-identity config-file surface) and dest
//! `schemas/generated/settings-docs.md` carries the Nova header (the
//! generated-docs surface). The runtime surfaces (window title, About,
//! CLI, logs) are proven from the same fixture in
//! `tests/rebrand/rebrand_acceptance.rs` (hosted in crates/product-config).
//!
//! Steps 3–4 of doc 07 §8.3 (boot the rebranded CLI; window title, About,
//! `engineInfo`) need the runtime plane, which does not exist in Phase 1:
//! they are parked as `#[ignore]`d placeholder tests at the bottom with
//! honest doc comments — [PROPOSAL] until a runtime lands (Phase 2+).
//!
//! Tier note (doc 07 §8.3): the full-build tier (rebrand + build + boot
//! the rewritten tree) is a release/nightly job, not this file.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---- fixture constants (tests/rebrand/nova.toml, verified by (a)) ----------

const NOVA_NAME: &str = "Nova Player";
const NOVA_SHORT_NAME: &str = "Nova";
const NOVA_PUBLISHER: &str = "Nova Player Contributors";
const NOVA_DISPLAY_VERSION: &str = "0.2.0-nova";
const NOVA_REVERSE_DNS: &str = "org.novaplayer";
const NOVA_DERIVED_ID: &str = "org.novaplayer.NovaPlayer";
const NOVA_MSI_GUID: &str = "9a6f2c48-1e5b-4c3a-8d7f-2b4e6a8c0d1f";
const NOVA_ACCENT: &str = "#ff5500";
const NOVA_URL_BASE: &str = "https://nova-player.example.org/";

const NOVA_ASSETS: [&str; 4] = [
    "tests/rebrand/assets/icon.ico",
    "tests/rebrand/assets/icon.icns",
    "tests/rebrand/assets/icon-512.png",
    "tests/rebrand/assets/logo.svg",
];

// ---- spawn helper ------------------------------------------------------------

struct Outcome {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run_rebrand(config: &Path, dest: &Path) -> Outcome {
    // Hermetic rooting (phase-7 gate OF-4): the tool discovers the
    // workspace by walking up from ITS cwd, so pin the cwd to the
    // worktree root this test file was compiled in — never the test
    // harness's incidental cwd, never the shared target dir. Tool and
    // test then provably operate on the same tree.
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(repo_root())
        .args([
            "rebrand",
            "--config",
            &config.to_string_lossy(),
            "--dest",
            &dest.to_string_lossy(),
        ])
        .output();
    match out {
        Ok(out) => Outcome {
            code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        },
        Err(err) => Outcome {
            code: None,
            stdout: String::new(),
            stderr: format!("failed to spawn the xtask binary: {err}"),
        },
    }
}

/// A fresh temp dest dir, unique per test+pid (cleaned up on drop).
struct TempDest(PathBuf);
impl TempDest {
    fn new(tag: &str) -> TempDest {
        let dir =
            std::env::temp_dir().join(format!("lumen-1e4-rebrand-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        TempDest(dir)
    }
}
impl Drop for TempDest {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn repo_root() -> PathBuf {
    // Phase-7 exit gate OF-4 fix: derive the root from the compile-time
    // manifest dir, NOT by walking up from CARGO_BIN_EXE_xtask. This file
    // is compiled into the xtask package's `cli` integration test (the
    // wiring note at the top), so CARGO_MANIFEST_DIR is
    // <worktree>/tools/xtask and the worktree root is two levels up —
    // baked in at compile time, always THIS worktree. Under a shared
    // CARGO_TARGET_DIR (six parallel agent worktrees, one target dir)
    // the binary lives in the orchestrator's target tree and the old
    // walk-up resolved a DIFFERENT checkout, so the reverse-control
    // proof compared its own worktree against a stale tree and failed
    // spuriously ("README.md differs in the no-op dest copy"). The tool
    // is a true no-op; only the test's root discovery was wrong.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|e| unreachable!("worktree root: {e}"))
}

fn read(dest: &Path, rel: &str) -> String {
    match fs::read_to_string(dest.join(rel)) {
        Ok(text) => text,
        Err(err) => unreachable!("dest file `{rel}` unreadable: {err}"),
    }
}

/// Lines of `stdout` beginning with `prefix` (report parsing for (c)).
fn report_lines<'a>(stdout: &'a str, prefix: &str) -> Vec<&'a str> {
    stdout
        .lines()
        .filter_map(|line| line.strip_prefix(prefix))
        .collect()
}

/// PascalCase derivation, mirroring the fixture's own derivation note:
/// split on spaces/hyphens/underscores, uppercase each word's first letter.
fn pascal_case(name: &str) -> String {
    let mut out = String::new();
    for word in name.split([' ', '-', '_']) {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.extend(chars.flat_map(|c| c.to_lowercase()));
        }
    }
    out
}

// ---- the acceptance test (fast tier) -----------------------------------------

#[test]
fn nova_fixture_rebrand_passes_skeleton_assertions() {
    let root = repo_root();
    let dest = TempDest::new("nova");
    let out = run_rebrand(&root.join("tests/rebrand/nova.toml"), &dest.0);

    assert_eq!(
        out.code,
        Some(0),
        "stdout:\n{}\nstderr:\n{}",
        out.stdout,
        out.stderr
    );
    assert!(
        out.stdout.contains(
            "rebrand: source `Lumen` (dev.lumen) -> target `Nova Player` (org.novaplayer)"
        ),
        "stdout:\n{}",
        out.stdout
    );

    // --- (a) dest product.toml carries the Nova identity --------------------

    let product = read(&dest.0, "product.toml");
    for (key_path, line) in [
        ("identity.name", format!("name = \"{NOVA_NAME}\"").as_str()),
        (
            "identity.short_name",
            format!("short_name = \"{NOVA_SHORT_NAME}\"").as_str(),
        ),
        (
            "identity.publisher",
            format!("publisher = \"{NOVA_PUBLISHER}\"").as_str(),
        ),
        (
            "identity.display_version",
            format!("display_version = \"{NOVA_DISPLAY_VERSION}\"").as_str(),
        ),
        (
            "identity.ids.reverse_dns",
            format!("reverse_dns = \"{NOVA_REVERSE_DNS}\"").as_str(),
        ),
        (
            "identity.ids.windows.app_user_model_id",
            format!("app_user_model_id = \"{NOVA_DERIVED_ID}\"").as_str(),
        ),
        (
            "identity.ids.windows.msi_upgrade_code",
            format!("msi_upgrade_code = \"{NOVA_MSI_GUID}\"").as_str(),
        ),
        (
            "identity.ids.macos.bundle_id",
            format!("bundle_id = \"{NOVA_DERIVED_ID}\"").as_str(),
        ),
        (
            "identity.ids.linux.appstream_id",
            format!("appstream_id = \"{NOVA_DERIVED_ID}\"").as_str(),
        ),
        (
            "identity.ids.web.app_id",
            format!("app_id = \"{NOVA_DERIVED_ID}\"").as_str(),
        ),
        (
            "brand.accent_color",
            format!("accent_color = \"{NOVA_ACCENT}\"").as_str(),
        ),
        (
            "brand.logo_svg",
            "logo_svg = \"tests/rebrand/assets/logo.svg\"",
        ),
        (
            "urls.help",
            format!("help = \"{NOVA_URL_BASE}docs\"").as_str(),
        ),
        (
            "urls.support",
            format!("support = \"{NOVA_URL_BASE}support\"").as_str(),
        ),
        (
            "urls.privacy",
            format!("privacy = \"{NOVA_URL_BASE}privacy\"").as_str(),
        ),
        (
            "urls.update_feed",
            "update_feed = \"https://updates.nova-player.example.org/feed.json\"",
        ),
    ] {
        assert!(
            product.contains(line),
            "dest product.toml: {key_path} missing `{line}`"
        );
    }
    // No source identity values survive.
    for stale in [
        "= \"Lumen\"",
        "dev.lumen",
        "#4f8cff",
        "lumen-player.example.org",
    ] {
        assert!(
            !product.contains(stale),
            "source identity token {stale:?} survived in dest product.toml"
        );
    }
    // Non-identity content survives byte-level.
    assert!(product.contains("title_template = \"{media_title} — {product_name}\""));
    assert!(product.contains("version_code = 1"));
    assert!(product.contains("schema_version = 1"));
    assert!(product.contains("# entries = []"));

    // Schema shapes (product.schema.json, re-implemented std-only — the
    // config-validator wiring is card 1.E2):
    // name 1..=40 chars; short_name [A-Za-z0-9-]{1,12}; display_version
    // ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.]+)?$; accent ^#[0-9a-f]{6}$;
    // https URLs; MSI GUID 8-4-4-4-12 hex; derived ids consistent
    // (doc 07 §3.3(b)).
    assert!(
        (1..=40).contains(&NOVA_NAME.chars().count()),
        "identity.name length is out of the 1..=40 schema range"
    );
    assert!(
        !NOVA_SHORT_NAME.is_empty()
            && NOVA_SHORT_NAME.len() <= 12
            && NOVA_SHORT_NAME
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-'),
        "identity.short_name violates ^[A-Za-z0-9-]{{1,12}}$"
    );
    {
        // schema: ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.]+)?$
        let (core, pre) = match NOVA_DISPLAY_VERSION.split_once('-') {
            Some((c, p)) => (c, Some(p)),
            None => (NOVA_DISPLAY_VERSION, None),
        };
        let segs: Vec<&str> = core.split('.').collect();
        let core_ok = segs.len() == 3
            && segs
                .iter()
                .all(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()));
        let pre_ok = match pre {
            Some(p) => !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '.'),
            None => true,
        };
        assert!(
            core_ok && pre_ok,
            "identity.display_version violates the schema pattern"
        );
    }
    assert!(
        NOVA_ACCENT.len() == 7
            && NOVA_ACCENT.starts_with('#')
            && NOVA_ACCENT[1..]
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "brand.accent_color violates ^#[0-9a-f]{{6}}$"
    );
    for url in [
        format!("{NOVA_URL_BASE}docs"),
        format!("{NOVA_URL_BASE}support"),
        format!("{NOVA_URL_BASE}privacy"),
        "https://updates.nova-player.example.org/feed.json".to_string(),
    ] {
        assert!(url.starts_with("https://"), "urls must be https://: {url}");
    }
    assert_eq!(
        NOVA_MSI_GUID.len(),
        36,
        "msi_upgrade_code must be a 36-char GUID"
    );
    assert!(
        NOVA_MSI_GUID
            .split('-')
            .map(str::len)
            .zip([8usize, 4, 4, 4, 12])
            .all(|(got, want)| got == want)
            && NOVA_MSI_GUID
                .chars()
                .all(|c| c.is_ascii_hexdigit() || c == '-'),
        "msi_upgrade_code violates the GUID pattern"
    );
    assert_eq!(
        NOVA_DERIVED_ID,
        format!("{NOVA_REVERSE_DNS}.{}", pascal_case(NOVA_NAME)),
        "derived ids must equal reverse_dns + \".\" + PascalCase(name) (doc 07 §3.3(b))"
    );
    assert!(
        NOVA_REVERSE_DNS
            .split('.')
            .all(|segment| !segment.is_empty()
                && segment
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')),
        "reverse_dns must be lowercase dotted segments"
    );

    // --- (b) dest README shows the Nova title, zero Lumen --------------------

    let readme = read(&dest.0, "README.md");
    assert!(
        readme.contains("# Nova Player Media Player"),
        "README title: {readme:?}"
    );
    assert!(
        !readme.contains("Lumen"),
        "README must carry no Lumen identity"
    );

    // --- (4.H2) installer identity: the tauri config file carries Nova -----
    // The scaffold is infra-blocked for *builds* (no bundler here), but the
    // config content is the generation/wiring proof (doc 07 §8.3 step 2):
    // productName + window title + identifier all flow from the fixture.

    let tauri = read(&dest.0, "apps/desktop/tauri/tauri.conf.json");
    assert!(
        tauri.contains("\"productName\": \"Nova Player\""),
        "dest tauri.conf.json productName: {tauri}"
    );
    assert!(
        tauri.contains("\"identifier\": \"org.novaplayer.desktop\""),
        "dest tauri.conf.json identifier (reverse_dns + .desktop): {tauri}"
    );
    assert!(
        tauri.contains("\"title\": \"Nova Player\""),
        "dest tauri.conf.json window title: {tauri}"
    );
    for stale in ["Lumen", "dev.lumen"] {
        assert!(
            !tauri.contains(stale),
            "source identity {stale:?} survived in dest tauri.conf.json"
        );
    }

    // --- (4.H2) generated docs: the settings reference header is Nova ------

    let docs = read(&dest.0, "schemas/generated/settings-docs.md");
    assert!(
        docs.contains("# Nova Player settings reference"),
        "dest settings-docs.md header: first line is {docs:?}"
    );
    assert!(
        !docs.contains("Lumen"),
        "dest settings-docs.md must carry no Lumen identity"
    );

    // --- (c) the report lists exactly the expected files ----------------------

    // The README rewrite count is DERIVED from the source README (count the
    // source-identity occurrences the rebrander will rewrite), never pinned:
    // wave 24's docs truth pass (lane E, issue #58) legitimately edited the
    // README and a stale pin (7 -> 6) failed the suite — the OF-2 derivation
    // principle applies to every count that tracks living content.
    let source_readme = fs::read_to_string(repo_root().join("README.md"))
        .unwrap_or_else(|e| unreachable!("source README unreadable: {e}"));
    // Case-insensitive: the rebrander rewrites every identity occurrence
    // (Lumen, lumen, dev.lumen) — 6 in the current README.
    let lowered = source_readme.to_lowercase();
    let readme_rewrites = lowered.matches("lumen").count();
    let touched = report_lines(&out.stdout, "touched: ");
    assert_eq!(
        touched,
        [
            "product.toml (19 rewrites)",
            "apps/desktop/tauri/tauri.conf.json (3 rewrites)",
            &format!("README.md ({readme_rewrites} rewrites)"),
            "schemas/generated/settings-docs.md (1 rewrite)",
            "schemas/generated/settings-help.txt (4 rewrites)",
        ],
        "touched-surface report mismatch (stdout:\n{})",
        out.stdout
    );
    let copied = report_lines(&out.stdout, "copied-asset: ");
    assert_eq!(copied.len(), NOVA_ASSETS.len(), "stdout:\n{}", out.stdout);
    for asset in NOVA_ASSETS {
        assert!(
            copied.iter().any(|line| line.starts_with(asset)),
            "asset {asset} missing from the report (stdout:\n{})",
            out.stdout
        );
        assert!(
            dest.0.join(asset).is_file(),
            "asset {asset} was not copied into the dest tree"
        );
    }
    // The totals line is derived from the same per-file counts the report
    // carried above (never a pinned sum — the README count follows content).
    let total_rewrites = 19 + 3 + readme_rewrites + 1 + 4;
    assert!(
        out.stdout.contains(&format!(
            "result: 5 surface(s) touched, {total_rewrites} rewrites, 4 asset(s) copied"
        )),
        "result line mismatch (stdout:\n{})",
        out.stdout
    );
    assert!(
        out.stdout.contains("0 in the v0 surface set"),
        "no v0 surface may be allowlisted (stdout:\n{})",
        out.stdout
    );

    // The generated help text: identity tokens rewritten, count is TOKENS.
    let help = read(&dest.0, "schemas/generated/settings-help.txt");
    assert!(
        !help.contains("lumen") && !help.contains("Lumen"),
        "dest settings-help.txt still says lumen"
    );
}

// ---- reverse control: the real product.toml is a no-op ------------------------

#[test]
fn reverse_control_real_product_toml_is_a_noop() {
    let root = repo_root();
    let dest = TempDest::new("noop");
    let out = run_rebrand(&root.join("product.toml"), &dest.0);
    assert_eq!(
        out.code,
        Some(0),
        "stdout:\n{}\nstderr:\n{}",
        out.stdout,
        out.stderr
    );
    assert!(
        out.stdout.contains("result: no-op"),
        "the identity-equal run must report a no-op (stdout:\n{})",
        out.stdout
    );
    // Zero diffs: every surface in the dest copy is byte-identical to the
    // repository file (doc 07 §8.3 step 1 reverse control).
    for surface in [
        "product.toml",
        "apps/desktop/tauri/tauri.conf.json",
        "README.md",
        "schemas/generated/settings-docs.md",
        "schemas/generated/settings-help.txt",
    ] {
        let repo_bytes = fs::read(root.join(surface));
        let dest_bytes = fs::read(dest.0.join(surface));
        match (repo_bytes, dest_bytes) {
            (Ok(repo), Ok(copy)) => {
                assert_eq!(repo, copy, "`{surface}` differs in the no-op dest copy");
            }
            (repo, dest_bytes) => {
                unreachable!("`{surface}` unreadable (repo: {repo:?}, dest: {dest_bytes:?})");
            }
        }
    }
    // Honest [VERIFY] record: the shipped product.toml points at
    // assets/brand/* which do not exist in the repo yet; the tool reports
    // them as missing assets instead of failing (config-validator, card
    // 1.E2, owns existence checks). If the assets land, this conditional
    // simply stops asserting.
    if !root.join("assets/brand/icon.ico").is_file() {
        assert!(
            out.stdout.contains("missing-asset: assets/brand/icon.ico"),
            "missing brand assets must be reported, not silently ignored (stdout:\n{})",
            out.stdout
        );
    }
}

// ---- error paths ------------------------------------------------------------------

#[test]
fn rebrand_missing_config_file_is_an_error() {
    let dest = TempDest::new("missing");
    let out = run_rebrand(Path::new("/nonexistent/rebrand-config.toml"), &dest.0);
    assert_eq!(out.code, Some(2), "stderr: {}", out.stderr);
    assert!(
        out.stderr.starts_with("rebrand: "),
        "errors carry the tool prefix (stderr: {})",
        out.stderr
    );
    assert!(
        out.stderr.contains("cannot read config"),
        "actionable message (stderr: {})",
        out.stderr
    );
}

#[test]
fn rebrand_malformed_config_is_an_error() {
    let dest = temp_dest("malformed");
    let config =
        std::env::temp_dir().join(format!("lumen-1e4-malformed-{}.toml", std::process::id()));
    if let Err(err) = fs::write(&config, "[identity\nname = \"unterminated\n") {
        unreachable!("fixture write failed: {err}");
    }
    let out = run_rebrand(&config, &dest);
    let _ = fs::remove_file(&config);
    let _ = fs::remove_dir_all(&dest);
    assert_eq!(out.code, Some(2), "stderr: {}", out.stderr);
    assert!(
        out.stderr.contains("malformed at line 1"),
        "parse errors carry the 1-based line (stderr: {})",
        out.stderr
    );
}

#[test]
fn rebrand_missing_identity_fields_is_an_error() {
    let dest = temp_dest("missing-fields");
    let config = std::env::temp_dir().join(format!(
        "lumen-1e4-missing-fields-{}.toml",
        std::process::id()
    ));
    if let Err(err) = fs::write(&config, "[window]\nwidth = 1280\n") {
        unreachable!("fixture write failed: {err}");
    }
    let out = run_rebrand(&config, &dest);
    let _ = fs::remove_file(&config);
    let _ = fs::remove_dir_all(&dest);
    assert_eq!(out.code, Some(2), "stderr: {}", out.stderr);
    assert!(
        out.stderr.contains("missing identity field(s)"),
        "all missing fields reported together (stderr: {})",
        out.stderr
    );
}

/// (error-path helper: an owned dest path the caller cleans up.)
fn temp_dest(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lumen-1e4-rebrand-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

// ---- runtime-plane placeholders (doc 07 §8.3 steps 3–4) ---------------------------

#[test]
#[ignore = "boot tier: the unit-tier proof lives in tests/rebrand/rebrand_acceptance.rs::surface_window_title_resolves_nova (card 4.H2); this placeholder keeps the full-desktop boot assertion for the nightly tier — doc 07 §8.3 step 4"]
fn window_title_renders_nova() {
    // FUTURE ASSERTION (doc 07 §8.3 step 4): unit-assert the window-title
    // template with the fixture config loaded — "{media_title} — Nova
    // Player" with a fake {media_title}, and the idle title "Nova Player"
    // via the window.title.idle key.
}

#[test]
#[ignore = "boot tier: the unit-tier proof lives in tests/rebrand/rebrand_acceptance.rs::surface_cli_identity_resolves_nova (card 4.H2) + the BLOCKED(wiring) register for the CLI host; this placeholder keeps the full-build boot assertion for the nightly/release job — doc 07 §8.3 step 3"]
fn cli_identity_is_nova() {
    // FUTURE ASSERTION (doc 07 §8.3 step 3): build the rebranded tree
    // (full-build tier, nightly/release job), boot the CLI headlessly and
    // assert `--version` prints "Nova Player 0.2.0-nova" and a log line
    // prefix carries `nova`.
}

#[test]
#[ignore = "boot tier: the unit-tier proof lives in tests/rebrand/rebrand_acceptance.rs::surface_about_facts_render_nova + ui/src/screens/settings/areas/updates-about.nova.test.tsx (card 4.H2); this placeholder keeps the booted-About/engineInfo assertion for the nightly tier — doc 07 §8.3 step 4"]
fn about_model_renders_nova_logo() {
    // FUTURE ASSERTION (doc 07 §8.3 step 4): unit-assert the About model
    // with the fixture config loaded — Nova name + publisher copyright
    // line + the copied logo path (tests/rebrand/assets/logo.svg) — and
    // the control-server engineInfo.name mock reporting Nova.
}
