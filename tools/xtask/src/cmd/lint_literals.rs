//! `xtask lint-literals` — the C1 config-propagation gate (doc 07 §9,
//! card 7.w21a / issue #61 "lint-literals").
//!
//! # What this gate is (and deliberately is not)
//!
//! Doc 07 §6.5's full C1 sketch (heuristic prose sweep: "any string
//! literal containing a space and an uppercase letter") is a Phase-4
//! localization-plane linter. This v0 implements the **config-plane**
//! half — the four literal classes that must come from configuration,
//! not code, per MP §5 ("every user-visible or build-visible value is
//! config, not code") and doc 07 §1's "hidden fork" failure mode:
//!
//! | Kind | Fires on | Where the value must live |
//! |---|---|---|
//! | `identity` | the product/brand strings from `product.toml` `[identity]` (`name`, `short_name`, `publisher`) appearing as string literals, matched case-insensitively as a whole word | `crates/product-config` (the runtime plane, card 4.H2) |
//! | `url` | `http://`, `https://`, `ws://`, `wss://` literals | `product.toml` `[urls]` (user-facing) — engine-networking test doubles excepted, below |
//! | `abs-path` | `/home/`, `/tmp/`, `/Users/`, `C:\`, `~/` literals | user/session paths resolved at runtime, never compiled in |
//! | `hex-color` | `#RRGGBB` / `#RGB` literals | `product.toml` `[brand]` / the theme-token layer (`crates/theme-tokens`) |
//!
//! Lowercase matching is intentional and matches the repo's own rebrand
//! token model (`rebrand::token_table` rewrites `name.to_lowercase()`
//! alongside `name`): the lowercase form is the CLI/log-prefix identity,
//! so `"lumen"` in a literal is as much a hard-coded identity as
//! `"Lumen"`.
//!
//! # The allowlist (audited, never blanket)
//!
//! 1. **Global, by category** (spec-fixed, not per-site): the config
//!    layer itself (`crates/product-config`, `crates/theme-tokens`,
//!    `crates/settings-schema`, `crates/localization` — these crates
//!    ARE the config); any `tests/` or `fixtures/` directory component
//!    in the path; and comments — the scanner extracts **string
//!    literals only**, so doc comments and `//` prose never fire (the
//!    spec's "doc comments" allowance, honored a fortiori: a comment is
//!    not compiled into the product).
//! 2. **Inline, per site**: a `// lumen-allow-literal: <reason>` line
//!    comment on or directly above the offending line suppresses it and
//!    is PRINTED in the report — every suppression is a reviewable act
//!    with a stated reason. A marker without a reason suppresses
//!    nothing.
//! 3. **Test-context URLs**: `127.0.0.1`/`localhost` hosts are allowed
//!    only inside `#[cfg(test)]` regions (the spec's "test-only
//!    contexts" exception). In non-test code even a loopback URL is a
//!    violation (an endpoint is a product decision); in test code a
//!    non-loopback URL is still a violation (a real host leaked into a
//!    test double).
//!
//! `#[cfg(test)]` modules inside `src/` are **scanned** (the spec
//! explicitly does not exempt them); their legitimate literals carry
//! inline markers like everything else. This is the difference between
//! an exemption and a blind spot: the markers make the debt countable,
//! and a rebrand that renames those sites re-reviews every one.
//!
//! # Relationship to `tools/xtask/data/rebrand-allowlist.toml`
//!
//! The rebrand allowlist's header sketches "the future check-identity
//! subcommand consumes this same file". This card deliberately does
//! NOT: the file's `crates/` entry exempts the entire crates tree for
//! *rebrand* purposes (the rebrand tool only rewrites its v0 surface
//! set, so everything else must not block it), and consuming it here
//! would vacate the C1/C2 scan of exactly the trees it must cover. The
//! rebrand-scale exemptions that matter to this gate (config layer,
//! tests/fixtures, comments) are implemented as the finer-grained
//! category + inline mechanisms above. Reconciling the two registers
//! is a Lead decision recorded as an open question in the card report.
//!
//! # Honest limitations (v0)
//!
//! * One finding per literal per kind (a literal holding two URLs
//!   reports the first); the report shows the literal's start line,
//!   not a column; multi-line literals report their opening line.
//! * The scanner is a lexer-shaped state machine, not rustc: macro
//!   bodies, `include_str!` contents and token streams are seen as
//!   plain source (which is the desired posture — a literal inside a
//!   macro IS a literal in the binary).
//! * `b"…"` byte strings and raw strings are scanned; char literals
//!   are not (a single character cannot carry any of the four kinds).
//! * `~` is matched as `~/` (or a literal that is exactly `~`); a
//!   bare `~` embedded mid-literal is not (no anchor to a real path).
//! * Only `127.0.0.1` and `localhost` count as loopback (`::1`,
//!   `0.0.0.0` do not — the spec names exactly the two).
//!
//! Usage: `cargo xtask lint-literals [--root <dir>]`
//!
//! Exit code: 0 no violations | 1 violations found | 2 xtask error.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::XtaskError;

use super::toml_lite;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// The plain-text usage (mirrors the module docs).
const USAGE: &str = "\
Usage: cargo xtask lint-literals [--root <dir>]

  Scans string literals in crates/*/src/** and apps/*/src/** for the four
  config-plane literal classes (doc 07 §9 C1): identity strings from
  product.toml, URLs, absolute filesystem paths and hex colors.
  Exemptions: the config-layer crates, tests//fixtures/ dirs, comments,
  loopback URLs inside #[cfg(test)] regions, and the inline
  `// lumen-allow-literal: <reason>` marker (printed in the report).

  --root <dir>  check the workspace rooted at <dir> instead of walking up
                from the current directory (fixture-test seam)

Exit code: 0 no violations | 1 violations found | 2 xtask error.";

/// Entry point for `cargo xtask lint-literals`.
pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
    let mut root_override: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(ExitCode::SUCCESS);
            }
            "--root" => {
                i += 1;
                let Some(dir) = args.get(i) else {
                    return Err(XtaskError::Usage(format!(
                        "--root requires a directory argument\n{USAGE}"
                    )));
                };
                root_override = Some(PathBuf::from(dir));
            }
            other => {
                return Err(XtaskError::Usage(format!(
                    "unexpected argument `{other}`\n{USAGE}"
                )));
            }
        }
        i += 1;
    }
    let root = match root_override {
        Some(dir) => dir,
        None => {
            let cwd = std::env::current_dir()
                .map_err(|e| XtaskError::io("determining current directory", e))?;
            find_root(&cwd)?
        }
    };
    let root = root
        .canonicalize()
        .map_err(|e| XtaskError::io(format!("resolving root {}", root.display()), e))?;

    let identity = Identity::load(&root)?;
    let outcome = scan_workspace(&root, &identity, Scope::C1)?;
    outcome.print(&root, "lint-literals");
    if outcome.violations.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(1))
    }
}

// ---------------------------------------------------------------------------
// Identity model (loaded from product.toml — the single source of truth)
// ---------------------------------------------------------------------------

/// The identity strings this gate enforces, loaded from the workspace
/// `product.toml`. The brand trio drives [`Scope::C1`]; the technical
/// ids and the accent hex join for [`Scope::C2`] (`check-identity`).
pub(crate) struct Identity {
    /// `(label, value)` brand strings — matched case-insensitively as
    /// whole words (`identity.name`, `identity.short_name`,
    /// `identity.publisher`), deduplicated by value.
    brand: Vec<(&'static str, String)>,
    /// `(label, value)` technical identifiers — matched as exact
    /// case-sensitive substrings (reverse DNS root, per-OS ids, the
    /// MSI upgrade GUID).
    technical: Vec<(&'static str, String)>,
    /// `brand.accent_color` — matched case-insensitively (hex is
    /// case-free in practice).
    accent_color: String,
}

impl Identity {
    /// Load and validate the identity from `<root>/product.toml`.
    /// Every field this gate needs is reported together when missing
    /// (the doc 07 §3 all-at-once diagnostic posture).
    pub(crate) fn load(root: &Path) -> Result<Identity, XtaskError> {
        let path = root.join("product.toml");
        let text = std::fs::read_to_string(&path)
            .map_err(|e| XtaskError::io(format!("reading {}", path.display()), e))?;
        let doc = toml_lite::parse(&text)
            .map_err(|e| XtaskError::internal(format!("product.toml is malformed: {e}")))?;
        let mut missing: Vec<&'static str> = Vec::new();
        let mut get = |dotted: &'static str| -> Option<String> {
            doc.get(dotted)
                .and_then(toml_lite::Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    missing.push(dotted);
                    None
                })
        };
        let name = get("identity.name");
        let short_name = get("identity.short_name");
        let publisher = get("identity.publisher");
        let reverse_dns = get("identity.ids.reverse_dns");
        let app_user_model_id = get("identity.ids.windows.app_user_model_id");
        let msi_upgrade_code = get("identity.ids.windows.msi_upgrade_code");
        let bundle_id = get("identity.ids.macos.bundle_id");
        let appstream_id = get("identity.ids.linux.appstream_id");
        let web_app_id = get("identity.ids.web.app_id");
        let accent_color = get("brand.accent_color");
        if !missing.is_empty() {
            return Err(XtaskError::internal(format!(
                "product.toml is missing the identity field(s) {} — the gate \
                 cannot guess the product identity (doc 07 §3)",
                missing.join(", ")
            )));
        }
        // The unpack dance keeps the borrow checker happy while the
        // `get` closure holds `doc`; every Option is Some here (the
        // missing-list guard above returned otherwise).
        let (name, short_name, publisher) = match (name, short_name, publisher) {
            (Some(a), Some(b), Some(c)) => (a, b, c),
            _ => unreachable!("guarded by the missing-field error above"),
        };
        let mut brand: Vec<(&'static str, String)> = Vec::new();
        for (label, value) in [
            ("identity.name", name),
            ("identity.short_name", short_name),
            ("identity.publisher", publisher),
        ] {
            if !value.is_empty() && !brand.iter().any(|(_, v)| *v == value) {
                brand.push((label, value));
            }
        }
        let mut technical = Vec::new();
        for (label, value) in [
            ("identity.ids.reverse_dns", reverse_dns),
            ("identity.ids.windows.app_user_model_id", app_user_model_id),
            ("identity.ids.windows.msi_upgrade_code", msi_upgrade_code),
            ("identity.ids.macos.bundle_id", bundle_id),
            ("identity.ids.linux.appstream_id", appstream_id),
            ("identity.ids.web.app_id", web_app_id),
        ] {
            let value = value.unwrap_or_default();
            if !value.is_empty() {
                technical.push((label, value));
            }
        }
        Ok(Identity {
            brand,
            technical,
            accent_color: accent_color.unwrap_or_default(),
        })
    }
}

// ---------------------------------------------------------------------------
// Classification
// ---------------------------------------------------------------------------

/// Which identity set a scan enforces: C1 (`lint-literals`) uses the
/// brand trio; C2 (`check-identity`, doc 07 §9) adds the technical ids
/// and the accent hex.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    /// `lint-literals`: brand strings only.
    C1,
    /// `check-identity`: brand strings + ids + accent hex.
    C2,
}

/// One classified literal. The `kind` token is the report's
/// `file:line: kind` field; the detail names the matched value.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    /// 1-based line of the literal's opening quote.
    pub(crate) line: usize,
    /// Machine kind token (`identity`, `url`, `abs-path`, `hex-color`).
    pub(crate) kind: &'static str,
    /// What matched (e.g. `identity("Lumen")`, `url("https://x.org")`).
    pub(crate) detail: String,
}

/// Classify one literal's text under `scope`. Returns at most one
/// finding per kind, in kind order (identity, url, abs-path, hex-color).
pub(crate) fn classify(text: &str, identity: &Identity, scope: Scope) -> Vec<Finding> {
    let mut out = Vec::new();
    if let Some(label) = brand_match(text, &identity.brand) {
        out.push(Finding {
            line: 0,
            kind: "identity",
            detail: format!("identity({label})"),
        });
    }
    if scope == Scope::C2
        && let Some(label) = technical_match(text, &identity.technical, &identity.accent_color)
    {
        out.push(Finding {
            line: 0,
            kind: "identity",
            detail: format!("identity({label})"),
        });
    }
    if let Some((host, _is_local)) = url_match(text) {
        out.push(Finding {
            line: 0,
            kind: "url",
            detail: format!("url({host})"),
        });
    }
    if let Some(marker) = abs_path_match(text) {
        out.push(Finding {
            line: 0,
            kind: "abs-path",
            detail: format!("abs-path({marker})"),
        });
    }
    if let Some(color) = hex_color_match(text) {
        out.push(Finding {
            line: 0,
            kind: "hex-color",
            detail: format!("hex-color({color})"),
        });
    }
    out
}

/// The first brand string (`label`) contained in `text` as a whole,
/// case-insensitive word.
fn brand_match(text: &str, brand: &[(&'static str, String)]) -> Option<String> {
    for (label, value) in brand {
        if contains_word(text, value) {
            return Some(label.to_string());
        }
    }
    None
}

/// The first technical id or the accent hex contained in `text` (exact
/// substring; hex compared case-insensitively).
fn technical_match(
    text: &str,
    technical: &[(&'static str, String)],
    accent: &str,
) -> Option<String> {
    for (label, value) in technical {
        if !value.is_empty() && text.contains(value.as_str()) {
            return Some(label.to_string());
        }
    }
    if !accent.is_empty()
        && text
            .to_ascii_lowercase()
            .contains(&accent.to_ascii_lowercase())
    {
        return Some("brand.accent_color".to_string());
    }
    None
}

/// Whether `needle` appears in `haystack` as a whole word, ignoring
/// ASCII case: both edges (when present) must be non-alphanumeric.
pub(crate) fn contains_word(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let hay: Vec<char> = haystack.chars().flat_map(|c| c.to_lowercase()).collect();
    let ned: Vec<char> = needle.chars().flat_map(|c| c.to_lowercase()).collect();
    if hay.len() < ned.len() {
        return false;
    }
    (0..=hay.len() - ned.len()).any(|at| {
        hay[at..at + ned.len()] == ned[..]
            && at
                .checked_sub(1)
                .and_then(|p| hay.get(p))
                .is_none_or(|p| !p.is_ascii_alphanumeric())
            && hay
                .get(at + ned.len())
                .is_none_or(|c| !c.is_ascii_alphanumeric())
    })
}

/// The URL schemes this gate scans (case-insensitive match on the
/// literal, lowercase compare).
const SCHEMES: [&str; 4] = ["https://", "http://", "wss://", "ws://"];

/// Find the first URL in `text`: returns `(host, is_loopback)`. The
/// host runs from after the scheme to the first delimiter (`/ ? # :`
/// or whitespace); userinfo before an `@` is dropped.
pub(crate) fn url_match(text: &str) -> Option<(String, bool)> {
    let lower = text.to_ascii_lowercase();
    let mut best: Option<usize> = None;
    let mut best_scheme_len = 0;
    for scheme in SCHEMES {
        if let Some(pos) = lower.find(scheme)
            && best.is_none_or(|b| pos < b)
        {
            best = Some(pos);
            best_scheme_len = scheme.len();
        }
    }
    let start = best? + best_scheme_len;
    let rest = &text[start..];
    let host_end = rest
        .find(['/', '?', '#', ':', ' ', '\t', '\n', '\r', '"', '<', '>'])
        .unwrap_or(rest.len());
    let mut host = &rest[..host_end];
    if let Some(at) = host.rfind('@') {
        host = &host[at + 1..];
    }
    let is_loopback =
        host.eq_ignore_ascii_case("127.0.0.1") || host.eq_ignore_ascii_case("localhost");
    Some((host.to_string(), is_loopback))
}

/// The absolute-path markers this gate scans. Matched case-sensitively
/// for the Unix prefixes (they are literal path shapes), both cases
/// for the Windows drive (both occur in the wild), `~/` anywhere and a
/// literal that is exactly `~`.
pub(crate) fn abs_path_match(text: &str) -> Option<&'static str> {
    for marker in ["/home/", "/tmp/", "/Users/"] {
        if text.contains(marker) {
            return Some(marker);
        }
    }
    for marker in ["C:\\", "c:\\", "C:/", "c:/"] {
        if text.contains(marker) {
            return Some(marker);
        }
    }
    if text.contains("~/") || text == "~" {
        return Some("~/");
    }
    None
}

/// Find a `#RRGGBB` / `#RGB` color in `text`: a `#` followed by a run
/// of exactly 3 or 6 ASCII hex digits, bounded by a non-alphanumeric
/// char (or the string edge) on BOTH sides. The trailing boundary
/// keeps format strings (`{:#010x}` — `x` is alphanumeric) and prose
/// anchors (`#decoding`) out of the net while still catching
/// minified-CSS shapes (`"color:#fff"` — the `:` before `#` is a
/// legitimate color context, not a format flag).
pub(crate) fn hex_color_match(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] != '#' {
            i += 1;
            continue;
        }
        // Leading boundary: the char before `#` (if any) must not be
        // alphanumeric (rejects `abc#fff`-shaped noise, keeps
        // `color:#fff`, `(#fff)`, `"#fff"`).
        let lead_ok = i
            .checked_sub(1)
            .and_then(|p| chars.get(p))
            .is_none_or(|p| !p.is_ascii_alphanumeric());
        let mut run = 0usize;
        while i + 1 + run < chars.len() && chars[i + 1 + run].is_ascii_hexdigit() {
            run += 1;
        }
        let trail_ok = i
            .checked_add(run + 1)
            .and_then(|t| chars.get(t))
            .is_none_or(|c| !c.is_ascii_alphanumeric());
        if lead_ok && trail_ok && (run == 3 || run == 6) {
            let color: String = chars[i..i + run + 1].iter().collect();
            return Some(color);
        }
        i += 1 + run;
    }
    None
}

// ---------------------------------------------------------------------------
// Workspace scan
// ---------------------------------------------------------------------------

/// The config-layer crates whose `src/` trees ARE the configuration:
/// literals there are the source, not a leak (spec-fixed category).
const CONFIG_LAYER_CRATES: [&str; 4] = [
    "product-config",
    "theme-tokens",
    "settings-schema",
    "localization",
];

/// The inline marker (spec-fixed shape). The reason after the colon
/// must be non-empty for the marker to suppress.
const MARKER: &str = "// lumen-allow-literal:";

/// A violation or allowlisted finding with its file (repo-relative).
pub(crate) struct Site {
    /// Repo-relative path with forward slashes.
    pub(crate) path: String,
    pub(crate) finding: Finding,
}

/// The whole-workspace outcome, shared by both subcommands.
pub(crate) struct Outcome {
    /// Files whose literals were lexed and classified.
    pub(crate) files_scanned: usize,
    /// Files skipped by the global config-layer category.
    pub(crate) config_layer_files: usize,
    /// Files skipped by the global tests/fixtures-dir category
    /// (a `tests` or `fixtures` path component — applied per FILE, so
    /// `src/tests/helper.rs` is exempt while `src/asf/tests.rs` is a
    /// scanned src module).
    pub(crate) test_dir_files: usize,
    /// Total literal sites classified.
    pub(crate) literals_scanned: usize,
    /// Allowlisted sites, with the reason each printed in the report.
    pub(crate) allowed: Vec<(Site, String)>,
    /// Violations.
    pub(crate) violations: Vec<Site>,
}

impl Outcome {
    /// Print the report with the given tool label. The violation list
    /// lines are exactly `path:line: kind` prefixed (spec contract).
    pub(crate) fn print(&self, root: &Path, label: &str) {
        println!(
            "{label}: {} files scanned across crates/ + apps/ \
             ({} config-layer exempt, {} tests/fixtures-dir exempt)",
            self.files_scanned, self.config_layer_files, self.test_dir_files
        );
        println!(
            "{label}: {} string-literal sites classified",
            self.literals_scanned
        );
        for (site, reason) in &self.allowed {
            println!(
                "  allowed: {}:{}: {} — {}",
                site.path, site.finding.line, site.finding.kind, reason
            );
        }
        if self.violations.is_empty() {
            println!(
                "{label}: PASS — 0 violations, {} allowlisted",
                self.allowed.len()
            );
        } else {
            println!("{label}: FAIL — {} violation(s):", self.violations.len());
            for site in &self.violations {
                println!(
                    "  {}:{}: {} — {}",
                    rel(root, &site.path),
                    site.finding.line,
                    site.finding.kind,
                    site.finding.detail
                );
            }
            println!(
                "  fix: read the value from the config layer (crates/product-config at \
                 runtime; product.toml / theme tokens at build time), or mark a \
                 legitimate site with `// lumen-allow-literal: <reason>` on or \
                 directly above the line — never delete the check"
            );
        }
    }
}

/// `path` relative to `root` when possible (report formatting).
fn rel(root: &Path, path: &str) -> String {
    Path::new(path)
        .strip_prefix(root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.to_string())
}

/// Scan `crates/*/src/**` and `apps/*/src/**` under `root`.
pub(crate) fn scan_workspace(
    root: &Path,
    identity: &Identity,
    scope: Scope,
) -> Result<Outcome, XtaskError> {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut config_layer_files = 0usize;
    let mut test_dir_files = 0usize;
    for base in ["crates", "apps"] {
        let dir = root.join(base);
        if !dir.is_dir() {
            return Err(XtaskError::internal(format!(
                "{}/ is missing under {} — the scan domain must not silently shrink",
                base,
                root.display()
            )));
        }
        let mut members: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map_err(|e| XtaskError::io(format!("listing {}", dir.display()), e))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_dir() && p.join("src").is_dir())
            .collect();
        members.sort();
        if members.is_empty() {
            return Err(XtaskError::internal(format!(
                "no {}/*/<src> member under {} — the scan domain must not silently shrink",
                base,
                root.display()
            )));
        }
        for member in members {
            let rel_member = rel(root, &member.display().to_string());
            if base == "crates"
                && CONFIG_LAYER_CRATES
                    .iter()
                    .any(|name| rel_member == format!("crates/{name}"))
            {
                config_layer_files += count_rs_files(&member.join("src"));
                continue;
            }
            let mut member_files: Vec<PathBuf> = Vec::new();
            collect_rs_files(&member.join("src"), &mut member_files)
                .map_err(|e| XtaskError::io(format!("walking {}", member.display()), e))?;
            for file in member_files {
                let rel_file = rel(root, &file.display().to_string());
                if rel_file.split('/').any(|c| c == "tests" || c == "fixtures") {
                    test_dir_files += 1;
                } else {
                    files.push(file);
                }
            }
        }
    }
    files.sort();

    // Pass 1: lex every file; collect file-level test-mod declarations.
    let mut scans: Vec<(PathBuf, SourceScan)> = Vec::with_capacity(files.len());
    for file in &files {
        let src = std::fs::read_to_string(file)
            .map_err(|e| XtaskError::io(format!("reading {}", file.display()), e))?;
        scans.push((file.clone(), scan_source(&src, file)));
    }
    let test_mod_files: Vec<PathBuf> = scans
        .iter()
        .flat_map(|(_, scan)| scan.mod_files.iter().cloned())
        .collect();

    // Pass 2: classify.
    let mut outcome = Outcome {
        files_scanned: scans.len(),
        config_layer_files,
        test_dir_files,
        literals_scanned: 0,
        allowed: Vec::new(),
        violations: Vec::new(),
    };
    for (file, scan) in &scans {
        let file_str = rel(root, &file.display().to_string());
        let is_test_mod_file = test_mod_files.contains(file);
        for site in &scan.literals {
            outcome.literals_scanned += 1;
            let findings = classify(&site.text, identity, scope);
            if findings.is_empty() {
                continue;
            }
            let is_test_context = scan.whole_file_test
                || is_test_mod_file
                || scan
                    .test_regions
                    .iter()
                    .any(|(start, end)| site.line >= *start && site.line <= *end);
            for finding in findings {
                let mut finding = finding;
                finding.line = site.line;
                // URL exception: a loopback host inside test context is
                // allowlisted by category (printed, reason fixed).
                if finding.kind == "url"
                    && is_test_context
                    && url_match(&site.text).is_some_and(|(_, local)| local)
                {
                    outcome.allowed.push((
                        Site {
                            path: file_str.clone(),
                            finding,
                        },
                        "loopback URL inside #[cfg(test)] context (spec exception)".to_string(),
                    ));
                    continue;
                }
                let site_entry = Site {
                    path: file_str.clone(),
                    finding,
                };
                match inline_marker_reason(&scan.raw_lines, site.line) {
                    Some(reason) => outcome.allowed.push((site_entry, reason)),
                    None => outcome.violations.push(site_entry),
                }
            }
        }
    }
    Ok(outcome)
}

/// Count `.rs` files under `dir` (for the exempt-category totals).
fn count_rs_files(dir: &Path) -> usize {
    let mut out = Vec::new();
    if collect_rs_files(dir, &mut out).is_ok() {
        out.len()
    } else {
        0
    }
}

/// The inline marker's reason for a finding on `line` (1-based): the
/// marker on the SAME line (a trailing comment after the last string
/// quote — a marker inside a string suppresses nothing) or directly
/// ABOVE it (a line whose trimmed form starts with the marker). A
/// marker without a non-empty reason suppresses nothing.
fn inline_marker_reason(raw_lines: &[String], line: usize) -> Option<String> {
    let same = raw_lines.get(line.checked_sub(1)?);
    let above = line.checked_sub(2).and_then(|i| raw_lines.get(i));
    let from_marker = |text: &str, must_start: bool| -> Option<String> {
        let pos = text.find(MARKER)?;
        if must_start && !text.trim_start().starts_with(MARKER) {
            return None;
        }
        if !must_start && text.rfind('"').is_some_and(|quote| quote > pos) {
            // The marker sits inside a string literal on this line.
            return None;
        }
        let reason = text[pos + MARKER.len()..].trim();
        if reason.is_empty() || reason.starts_with('/') {
            // `//` continuing the comment is not a reason.
            return None;
        }
        Some(reason.to_string())
    };
    from_marker(same?, false).or_else(|| from_marker(above?, true))
}

/// Collect `.rs` files under `dir` recursively (sorted for stable
/// reports; the check_panics precedent).
fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
        .collect::<Result<Vec<std::fs::DirEntry>, std::io::Error>>()?
        .into_iter()
        .map(|e| e.path())
        .collect();
    entries.sort();
    for entry in entries {
        if entry.is_dir() {
            collect_rs_files(&entry, out)?;
        } else if entry.extension().is_some_and(|e| e == "rs") {
            out.push(entry);
        }
    }
    Ok(())
}

/// Walk up from `start` to the directory whose Cargo.toml declares a
/// `[workspace]` (the check_architecture precedent). Shared with
/// `check_identity`.
pub(crate) fn find_root(start: &Path) -> Result<PathBuf, XtaskError> {
    let mut dir: Option<&Path> = Some(start);
    while let Some(d) = dir {
        let manifest = d.join("Cargo.toml");
        if manifest.is_file() {
            match std::fs::read_to_string(&manifest) {
                Ok(contents) => {
                    if contents.lines().any(|line| line.starts_with("[workspace]")) {
                        return Ok(d.to_path_buf());
                    }
                }
                Err(e) => {
                    return Err(XtaskError::io(format!("reading {}", manifest.display()), e));
                }
            }
        }
        dir = d.parent();
    }
    Err(XtaskError::internal(format!(
        "no Cargo.toml with [workspace] found in {} or any parent — \
         run `cargo xtask lint-literals` from inside the Lumen workspace",
        start.display()
    )))
}

// ---------------------------------------------------------------------------
// Source scanner: string literals + cfg(test) context
// ---------------------------------------------------------------------------

/// One extracted string literal (normal, byte or raw; escapes decoded
/// for the normal/byte forms, verbatim for raw).
pub(crate) struct LiteralSite {
    /// 1-based line of the opening quote.
    pub(crate) line: usize,
    /// Decoded literal contents.
    pub(crate) text: String,
}

/// One file's scan: the literals, the test-context regions (the
/// check_panics brace-tracking model), and the raw lines for the
/// inline-marker lookup.
pub(crate) struct SourceScan {
    pub(crate) literals: Vec<LiteralSite>,
    /// 1-based inclusive line ranges covered by a test-context item.
    pub(crate) test_regions: Vec<(usize, usize)>,
    /// Files pulled in by file-level `#[cfg(test)] mod name;` decls.
    pub(crate) mod_files: Vec<PathBuf>,
    /// The file's own `#![cfg(test)]` makes it wholly test context.
    pub(crate) whole_file_test: bool,
    /// Raw (unlexed) source lines.
    pub(crate) raw_lines: Vec<String>,
}

/// The lexer modes.
#[derive(PartialEq)]
enum Mode {
    Normal,
    LineComment,
    /// Block comment (Rust nests them) at depth `n`.
    BlockComment(usize),
    /// Inside a `"…"` literal; escapes decoded into the buffer.
    Str,
    /// Inside a `'…'` char literal.
    Char,
    /// Inside a raw string with `n` leading `#`s.
    RawStr(usize),
}

/// Scan one source file: extract literals and test context.
pub(crate) fn scan_source(src: &str, file: &Path) -> SourceScan {
    let chars: Vec<char> = src.chars().collect();
    let mut literals: Vec<LiteralSite> = Vec::new();
    let mut test_regions: Vec<(usize, usize)> = Vec::new();
    let mut mod_files: Vec<PathBuf> = Vec::new();
    let mut whole_file_test = false;

    let mut mode = Mode::Normal;
    let mut i = 0usize;
    // Current 1-based line.
    let mut line = 1usize;
    // Literal capture state (Str / RawStr only).
    let mut buf = String::new();
    let mut lit_start_line = 0usize;
    // Brace depth in Normal mode.
    let mut depth: i64 = 0;
    // Open test regions: (depth before the region's `{`, start line).
    let mut open_regions: Vec<(i64, usize)> = Vec::new();
    // A test-context attribute was just seen; the next `{` (region) or
    // `;` (file-level mod decl) resolves it.
    let mut test_attr_pending = false;
    // Normal-mode text since the pending attribute closed.
    let mut since_attr = String::new();

    while i < chars.len() {
        let c = chars[i];
        match mode {
            Mode::LineComment => {
                if c == '\n' {
                    line += 1;
                    mode = Mode::Normal;
                }
                i += 1;
            }
            Mode::BlockComment(nested) => {
                if c == '/' && chars.get(i + 1) == Some(&'*') {
                    mode = Mode::BlockComment(nested + 1);
                    i += 2;
                } else if c == '*' && chars.get(i + 1) == Some(&'/') {
                    mode = if nested == 1 {
                        Mode::Normal
                    } else {
                        Mode::BlockComment(nested - 1)
                    };
                    i += 2;
                } else {
                    if c == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
            Mode::Char => {
                if c == '\\' && i + 1 < chars.len() {
                    if chars[i + 1] == '\n' {
                        line += 1;
                    }
                    i += 2;
                } else if c == '\'' {
                    mode = Mode::Normal;
                    i += 1;
                } else {
                    if c == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
            Mode::Str => {
                if c == '\\' && i + 1 < chars.len() {
                    decode_escape(&chars, i, &mut buf);
                    if chars[i + 1] == '\n' {
                        line += 1;
                    }
                    i += escape_len(&chars, i);
                    continue;
                }
                if c == '"' {
                    literals.push(LiteralSite {
                        line: lit_start_line,
                        text: std::mem::take(&mut buf),
                    });
                    mode = Mode::Normal;
                    i += 1;
                    continue;
                }
                if c == '\n' {
                    line += 1;
                }
                buf.push(c);
                i += 1;
            }
            Mode::RawStr(hashes) => {
                if c == '"' {
                    let mut following = 0usize;
                    while following < hashes && chars.get(i + 1 + following) == Some(&'#') {
                        following += 1;
                    }
                    if following == hashes {
                        literals.push(LiteralSite {
                            line: lit_start_line,
                            text: std::mem::take(&mut buf),
                        });
                        i += 1 + hashes;
                        mode = Mode::Normal;
                        continue;
                    }
                }
                if c == '\n' {
                    line += 1;
                }
                buf.push(c);
                i += 1;
            }
            Mode::Normal => {
                // Comments.
                if c == '/' && chars.get(i + 1) == Some(&'/') {
                    mode = Mode::LineComment;
                    i += 2;
                    continue;
                }
                if c == '/' && chars.get(i + 1) == Some(&'*') {
                    mode = Mode::BlockComment(1);
                    i += 2;
                    continue;
                }
                // String literals (a `b"`/`b` prefix is plain Normal
                // text before the quote — the capture is identical).
                if c == '"' {
                    mode = Mode::Str;
                    buf.clear();
                    lit_start_line = line;
                    i += 1;
                    continue;
                }
                if c == 'r' && looks_like_raw_string(&chars, i) {
                    let mut j = i + 1;
                    let mut hashes = 0usize;
                    while chars.get(j) == Some(&'#') {
                        hashes += 1;
                        j += 1;
                    }
                    // chars[j] == '"'
                    mode = Mode::RawStr(hashes);
                    buf.clear();
                    lit_start_line = line;
                    i = j + 1;
                    continue;
                }
                // Char literals vs lifetimes.
                if c == '\'' && char_literal_len(&chars, i).is_some() {
                    mode = Mode::Char;
                    i += 1;
                    continue;
                }
                // Attributes: `#[…]` / `#![…]` (test-context tracking,
                // the check_panics model).
                if c == '#' {
                    let mut j = i + 1;
                    let mut inner = false;
                    if chars.get(j) == Some(&'!') {
                        inner = true;
                        j += 1;
                    }
                    if chars.get(j) == Some(&'[')
                        && let Some((text, span)) = capture_attribute(&chars, j)
                    {
                        if is_test_attribute(&text) {
                            if inner {
                                whole_file_test = true;
                            } else {
                                test_attr_pending = true;
                                since_attr.clear();
                            }
                        }
                        let total = 1 + usize::from(inner) + span;
                        line += count_newlines(&chars, i, (i + total).min(chars.len()));
                        i += total;
                        continue;
                    }
                    i += 1;
                    continue;
                }
                // Braces and file-level mod decls. (A newline in
                // Normal mode still advances the line counter.)
                if c == '\n' {
                    line += 1;
                } else if c == '{' {
                    if test_attr_pending {
                        open_regions.push((depth, line));
                        test_attr_pending = false;
                        since_attr.clear();
                    }
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                    if open_regions
                        .last()
                        .is_some_and(|(open_depth, _)| *open_depth == depth)
                        && let Some((_, start)) = open_regions.pop()
                    {
                        test_regions.push((start, line));
                    }
                } else if c == ';' && test_attr_pending {
                    // A pending test attribute resolved without a
                    // body: a file-level `mod name;` pulls a whole
                    // file into test context.
                    let tokens: Vec<&str> = since_attr.split_whitespace().collect();
                    if tokens.first() == Some(&"mod") && tokens.len() >= 2 {
                        resolve_mod_file(file, tokens[1], &mut mod_files);
                    }
                    test_attr_pending = false;
                    since_attr.clear();
                } else if test_attr_pending {
                    if c.is_whitespace() {
                        if !since_attr.is_empty() && !since_attr.ends_with(' ') {
                            since_attr.push(' ');
                        }
                    } else if since_attr.len() < 96 {
                        since_attr.push(c);
                    }
                }
                i += 1;
            }
        }
    }
    SourceScan {
        literals,
        test_regions,
        mod_files,
        whole_file_test,
        raw_lines: src.lines().map(str::to_string).collect(),
    }
}

/// Decode the escape at `chars[at] == '\\'` into `buf`.
fn decode_escape(chars: &[char], at: usize, buf: &mut String) {
    let Some(next) = chars.get(at + 1) else {
        return;
    };
    match next {
        'n' => buf.push('\n'),
        't' => buf.push('\t'),
        'r' => buf.push('\r'),
        '0' => buf.push('\0'),
        '\\' => buf.push('\\'),
        '"' => buf.push('"'),
        '\'' => buf.push('\''),
        '\n' => {} // line continuation
        'x' => {
            let hi = chars.get(at + 2).and_then(|c| c.to_digit(16));
            let lo = chars.get(at + 3).and_then(|c| c.to_digit(16));
            if let (Some(hi), Some(lo)) = (hi, lo)
                && let Some(byte) = u8::try_from(hi * 16 + lo).ok()
                && let Some(decoded) = char::from_u32(u32::from(byte))
            {
                buf.push(decoded);
            }
        }
        'u' => {
            // `\u{…}` — bounded scan for the closing brace.
            if chars.get(at + 2) == Some(&'{') {
                let mut value = 0u32;
                let mut j = at + 3;
                let mut digits = 0usize;
                while j < chars.len() && digits < 8 {
                    match chars[j].to_digit(16) {
                        Some(d) => {
                            value = value.saturating_mul(16).saturating_add(d);
                            digits += 1;
                            j += 1;
                        }
                        None => break,
                    }
                }
                if chars.get(j) == Some(&'}')
                    && let Some(decoded) = char::from_u32(value)
                {
                    buf.push(decoded);
                }
            }
        }
        other => buf.push(*other), // unknown escape: keep the char
    }
}

/// How many chars the escape at `chars[at]` occupies (backslash +
/// payload; `\xNN` is 4, `\u{…}` runs to the closing brace).
fn escape_len(chars: &[char], at: usize) -> usize {
    let Some(next) = chars.get(at + 1) else {
        return 1;
    };
    match next {
        'x' => 4,
        'u' if chars.get(at + 2) == Some(&'{') => {
            let mut j = at + 3;
            while j < chars.len() && j < at + 16 && chars[j] != '}' {
                j += 1;
            }
            (j - at + 1).min(chars.len() - at)
        }
        _ => 2,
    }
}

/// Count `\n` in `chars[from..to]` (multi-char span consumption keeps
/// the line counter honest).
fn count_newlines(chars: &[char], from: usize, to: usize) -> usize {
    chars[from.min(chars.len())..to.min(chars.len())]
        .iter()
        .filter(|c| **c == '\n')
        .count()
}

/// Whether `chars[at] == 'r'` begins a raw string (`r"` or `r#"`).
/// A preceding identifier character means the `r` is a name's tail —
/// except `b`/`c` (the `br#"…"`/`cr#"…"` byte/C-string prefixes).
fn looks_like_raw_string(chars: &[char], at: usize) -> bool {
    let mut j = at + 1;
    let mut hashes = 0;
    while chars.get(j) == Some(&'#') {
        hashes += 1;
        j += 1;
    }
    hashes <= 255
        && chars.get(j) == Some(&'"')
        && at
            .checked_sub(1)
            .and_then(|prev| chars.get(prev))
            .is_none_or(|prev| {
                !prev.is_ascii_alphanumeric() || *prev == 'b' || *prev == 'c' || *prev == '_'
            })
}

/// The length (in chars, including both quotes) of a char literal
/// starting at `chars[at] == '\''`, or `None` when it is a lifetime.
fn char_literal_len(chars: &[char], at: usize) -> Option<usize> {
    let next = chars.get(at + 1)?;
    if *next == '\\' {
        let mut j = at + 2;
        while j < chars.len() && j <= at + 12 {
            if chars[j] == '\'' {
                return Some(j - at + 1);
            }
            j += 1;
        }
        None
    } else {
        match chars.get(at + 2) {
            Some('\'') if *next != '\'' => Some(3),
            _ => None,
        }
    }
}

/// Capture the `[…]` attribute starting at `chars[at] == '['`,
/// returning its inner text and the span's char length (both brackets
/// included). Strings inside the attribute are kept verbatim (the
/// check_panics precedent — attribute strings never carry brackets).
fn capture_attribute(chars: &[char], at: usize) -> Option<(String, usize)> {
    let mut bracket_depth = 0i64;
    let mut text = String::new();
    let mut span = 0usize;
    let mut j = at;
    while j < chars.len() {
        let c = chars[j];
        span += 1;
        if c == '"' {
            text.push(c);
            j += 1;
            while j < chars.len() {
                let s = chars[j];
                text.push(s);
                span += 1;
                if s == '\\' && j + 1 < chars.len() {
                    text.push(chars[j + 1]);
                    span += 1;
                    j += 2;
                    continue;
                }
                j += 1;
                if s == '"' {
                    break;
                }
            }
            continue;
        }
        if c == '[' {
            bracket_depth += 1;
            if bracket_depth == 1 {
                j += 1;
                continue;
            }
        } else if c == ']' {
            bracket_depth -= 1;
            if bracket_depth == 0 {
                return Some((text, span));
            }
        }
        text.push(c);
        j += 1;
    }
    None
}

/// Whether an attribute's inner text denotes test context (the
/// check_panics predicate, verbatim): `test`, `cfg(test)`,
/// `cfg(any(test, …))`, `cfg(all(…, test, …))` — and never
/// `cfg(not(test))` or a `test` inside a string.
fn is_test_attribute(text: &str) -> bool {
    let text = text.trim();
    if text == "test" {
        return true;
    }
    let Some(rest) = text.strip_prefix("cfg") else {
        return false;
    };
    let rest = rest.trim();
    let Some(inner) = rest.strip_prefix('(').and_then(|r| r.strip_suffix(')')) else {
        return false;
    };
    let inner = inner.trim();
    if inner == "test" {
        return true;
    }
    for combinator in ["any", "all"] {
        let Some(body) = inner
            .strip_prefix(combinator)
            .and_then(|r| r.trim().strip_prefix('('))
            .and_then(|r| r.trim().strip_suffix(')'))
        else {
            continue;
        };
        let mut child_depth = 0i64;
        let mut token = String::new();
        let mut found = false;
        for c in body.chars() {
            match c {
                '(' => {
                    child_depth += 1;
                    token.push(c);
                }
                ')' => {
                    child_depth -= 1;
                    token.push(c);
                }
                ',' if child_depth == 0 => {
                    if token.trim() == "test" {
                        found = true;
                        break;
                    }
                    token.clear();
                }
                _ => token.push(c),
            }
        }
        if token.trim() == "test" {
            found = true;
        }
        if found {
            return true;
        }
    }
    false
}

/// Resolve the file(s) a `mod name;` declaration in `file` pulls in
/// (Rust 2018 layout; the check_panics precedent).
fn resolve_mod_file(file: &Path, name: &str, out: &mut Vec<PathBuf>) {
    let dir = match file.file_name().and_then(|n| n.to_str()) {
        Some("lib.rs") | Some("main.rs") | Some("mod.rs") => file
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".")),
        _ => {
            let stem = file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            file.parent()
                .map(|p| p.join(stem))
                .unwrap_or_else(|| PathBuf::from("."))
        }
    };
    for candidate in [
        dir.join(format!("{name}.rs")),
        dir.join(name).join("mod.rs"),
    ] {
        if candidate.is_file() && !out.contains(&candidate) {
            out.push(candidate);
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests (in-crate; the fixture-based end-to-end proof lives in
// tools/xtask/tests/lint_literals.rs)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// testkit's Must, locally bound so the assertions stay one line.
    trait Must {
        type Value;
        fn must(self) -> Self::Value;
    }
    impl<T, E: std::fmt::Display> Must for Result<T, E> {
        type Value = T;
        fn must(self) -> T {
            match self {
                Ok(v) => v,
                Err(e) => unreachable!("{e}"),
            }
        }
    }
    impl<T> Must for Option<T> {
        type Value = T;
        fn must(self) -> T {
            match self {
                Some(v) => v,
                None => unreachable!("must(): None"),
            }
        }
    }

    #[test]
    fn word_matching_is_case_insensitive_and_bounded() {
        assert!(contains_word("Lumen", "Lumen"));
        assert!(contains_word("the lumen player", "Lumen"));
        assert!(contains_word("LUMEN-CLI", "lumen"));
        assert!(contains_word("usage: lumen open", "Lumen"));
        assert!(contains_word("(lumen)", "lumen"));
        assert!(!contains_word("illumenx", "lumen"), "bounded by alnum");
        assert!(!contains_word("volume", "lumen"), "not a substring hit");
        assert!(!contains_word("Lumen Contributors", "Lumen Contributor"));
        assert!(contains_word(
            "by Lumen Contributors, 2026",
            "Lumen Contributors"
        ));
        assert!(!contains_word("", "Lumen"));
        assert!(!contains_word("anything", ""));
    }

    #[test]
    fn url_matching_extracts_hosts() {
        assert_eq!(
            url_match("see https://example.com/docs now"),
            Some(("example.com".to_string(), false))
        );
        assert_eq!(
            url_match("http://127.0.0.1:8080/x"),
            Some(("127.0.0.1".to_string(), true))
        );
        assert_eq!(
            url_match("ws://localhost"),
            Some(("localhost".to_string(), true))
        );
        assert_eq!(
            url_match("wss://user@cdn.example.org/stream"),
            Some(("cdn.example.org".to_string(), false))
        );
        assert_eq!(url_match("no scheme here"), None);
        assert_eq!(url_match("memory://lumen-x"), None, "not a scanned scheme");
    }

    #[test]
    fn absolute_path_shapes() {
        assert_eq!(abs_path_match("/home/z/clip.avi"), Some("/home/"));
        assert_eq!(abs_path_match("cache at /tmp/lumen"), Some("/tmp/"));
        assert_eq!(abs_path_match("/Users/z/Movies"), Some("/Users/"));
        assert_eq!(abs_path_match(r"C:\Users\z"), Some("C:\\"));
        assert_eq!(abs_path_match("~/notes.txt"), Some("~/"));
        assert_eq!(abs_path_match("~"), Some("~/"));
        assert_eq!(abs_path_match("relative/path.rs"), None);
        assert_eq!(
            abs_path_match("/etc/passwd"),
            None,
            "outside the marker set"
        );
    }

    #[test]
    fn hex_color_shapes() {
        assert_eq!(hex_color_match("#4f8cff"), Some("#4f8cff".to_string()));
        assert_eq!(hex_color_match("color:#fff;"), Some("#fff".to_string()));
        assert_eq!(
            hex_color_match("accent #FFF200."),
            Some("#FFF200".to_string())
        );
        assert_eq!(hex_color_match("#ABCDEF"), Some("#ABCDEF".to_string()));
        // Format strings and prose anchors stay out of the net.
        assert_eq!(hex_color_match("{:#010x}"), None);
        assert_eq!(hex_color_match("#decoding header"), None);
        assert_eq!(hex_color_match("#c18f85b069732afa"), None, "7-digit run");
        assert_eq!(hex_color_match("#ff"), None);
        assert_eq!(hex_color_match("plain text"), None);
        assert_eq!(hex_color_match("abc#fff"), None, "alnum before #");
    }

    #[test]
    fn markers_need_a_reason() {
        let raw = vec![
            "let a = \"Lumen\"; // lumen-allow-literal: argv0 token".to_string(),
            "// lumen-allow-literal:".to_string(),
            "let b = \"Lumen\";".to_string(),
            "// lumen-allow-literal: real reason".to_string(),
            "let c = \"Lumen\";".to_string(),
            "let d = \"Lumen\";".to_string(),
        ];
        assert_eq!(
            inline_marker_reason(&raw, 1),
            Some("argv0 token".to_string()),
            "same-line marker"
        );
        assert_eq!(inline_marker_reason(&raw, 3), None, "reasonless marker");
        assert_eq!(
            inline_marker_reason(&raw, 5),
            Some("real reason".to_string()),
            "marker directly above"
        );
        assert_eq!(inline_marker_reason(&raw, 6), None, "no marker");
    }

    #[test]
    fn scanner_extracts_literals_and_skips_comments() {
        let src = "\
//! doc mention of Lumen and https://example.com — never a finding
// line comment: Lumen /tmp/ #fff
let a = \"Lumen player\";
let b = b\"lumen bytes\";
let raw = r#\"raw https://example.com body\"#;
let escaped = \"call it \\u{4c}umen\";
fn f<'a>(x: &'a u8) -> u8 { *x }
let fmt = format!(\"{:#010x}\", 7);
";
        let scan = scan_source(src, Path::new("crates/x/src/lib.rs"));
        let texts: Vec<&str> = scan.literals.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(
            texts,
            vec![
                "Lumen player",
                "lumen bytes",
                "raw https://example.com body",
                "call it Lumen", // \\u{4c} decoded — no smuggling
                "{:#010x}",
            ],
            "comments produce no literals; escapes decode"
        );
        assert_eq!(scan.literals[0].line, 3);
        assert_eq!(scan.literals[3].line, 6);
        assert!(scan.test_regions.is_empty());
    }

    #[test]
    fn scanner_tracks_test_regions_and_mod_files() {
        let src = "\
fn production() -> u8 { 1 }
#[cfg(test)]
mod tests {
    fn helper() -> &'static str { \"Lumen\" }
}
";
        let scan = scan_source(src, Path::new("crates/x/src/lib.rs"));
        assert_eq!(scan.test_regions.len(), 1);
        assert_eq!(scan.test_regions[0], (3, 5));
        assert!(scan.test_regions.iter().any(|(s, e)| *s <= 4 && 4 <= *e));
        assert!(!scan.test_regions.iter().any(|(s, e)| *s <= 1 && 1 <= *e));
    }

    #[test]
    fn file_level_test_mod_resolves() {
        let dir = std::env::temp_dir().join(format!("lint-lit-unit-{}", std::process::id()));
        let lib = dir.join("src").join("lib.rs");
        let tests = dir.join("src").join("tests.rs");
        std::fs::create_dir_all(dir.join("src")).must();
        std::fs::write(&lib, "#[cfg(test)]\nmod tests;\n").must();
        std::fs::write(&tests, "fn f() -> u8 { 1 }\n").must();
        let scan = scan_source("#[cfg(test)]\nmod tests;\n", &lib);
        assert_eq!(scan.mod_files, vec![tests.clone()]);
        std::fs::remove_dir_all(&dir).must();
    }

    #[test]
    fn test_attribute_shapes() {
        assert!(is_test_attribute("test"));
        assert!(is_test_attribute("cfg(test)"));
        assert!(is_test_attribute("cfg(any(test, feature = \"contract\"))"));
        assert!(is_test_attribute("cfg(all(target_os = \"linux\", test))"));
        assert!(!is_test_attribute("cfg(not(test))"));
        assert!(!is_test_attribute("cfg(feature = \"integration-tests\")"));
    }

    #[test]
    fn classify_c1_vs_c2_scopes() {
        let identity = Identity {
            brand: vec![
                ("identity.name", "Lumen".to_string()),
                ("identity.publisher", "Lumen Contributors".to_string()),
            ],
            technical: vec![(
                "identity.ids.windows.msi_upgrade_code",
                "7c1f6a52-9d3e-4b8a-a001-3f9c2d64e701".to_string(),
            )],
            accent_color: "#4f8cff".to_string(),
        };
        // C1: brand trio only — the GUID is C2's business.
        assert!(!classify("Lumen", &identity, Scope::C1).is_empty());
        assert!(
            classify(
                "upgrade code 7c1f6a52-9d3e-4b8a-a001-3f9c2d64e701",
                &identity,
                Scope::C1
            )
            .is_empty()
        );
        let c2 = classify(
            "upgrade code 7c1f6a52-9d3e-4b8a-a001-3f9c2d64e701",
            &identity,
            Scope::C2,
        );
        assert_eq!(c2.len(), 1);
        assert_eq!(c2[0].kind, "identity");
        // The accent hex fires as a hex-color in C1 and as identity in C2.
        let c1_accent = classify("#4f8cff", &identity, Scope::C1);
        assert_eq!(c1_accent.len(), 1);
        assert_eq!(c1_accent[0].kind, "hex-color");
        let c2_accent = classify("#4F8CFF", &identity, Scope::C2);
        assert_eq!(c2_accent.len(), 2);
        assert!(c2_accent.iter().any(|f| f.kind == "identity"));
        assert!(c2_accent.iter().any(|f| f.kind == "hex-color"));
    }
}
