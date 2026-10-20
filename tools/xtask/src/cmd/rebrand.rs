//! `xtask rebrand` — build-time rebrand tooling, v0 (doc 07 §8, card 1.E4).
//!
//! Reads a **target** product definition (`--config <path>`, e.g. the Nova
//! acceptance fixture `tests/rebrand/nova.toml`) and rewrites the Lumen
//! identity fields into a **copy** of the source tree's identity-bearing
//! surface set, plus the target's brand assets. The working tree is NEVER
//! modified in place: all output goes to `--dest <dir>` (default
//! `target/rebrand-<short-name>/`); an existing dest is an error.
//!
//! # v0 surface set (what exists today — inspected for card 1.E4)
//!
//! | Surface | Identity carried | Rewrite |
//! |---|---|---|
//! | `product.toml` | the 19 identity key-values in their tables | key-value rebuild (below) |
//! | `README.md` | H1 title + working-name sentence | token rewrite |
//! | `schemas/generated/settings-docs.md` | H1 | token rewrite |
//! | `schemas/generated/settings-help.txt` | header + `lumen` CLI-name tokens | token rewrite |
//! | `locales/en-US.toml` | **none** — inspected: no identity literal outside the allowlisted header | not a surface |
//!
//! Every touched file and every token replacement is reported on stdout
//! (`touched:` / `copied-asset:` / `missing-asset:` lines) so the
//! acceptance skeleton (`tests/rebrand/rebrand_skeleton.rs`) can assert
//! the exact report.
//!
//! # Honest v0 limitations (card 1.E4 + doc 13 §4 row)
//!
//! * **Std-only TOML-subset reader** `[PROPOSAL]`: xtask is std-only by
//!   design (C5 — see the Cargo.toml header), so this file carries a
//!   minimal reader for the fixture's flat string-key tables. Full
//!   fidelity loading via a product-config query bin is a later card.
//! * **No runtime plane**: window title / CLI `--version` / About are
//!   data at RUN time (doc 07 §8.1); those assertions are parked as
//!   `#[ignore]`d placeholders in the skeleton until a runtime exists.
//! * **Token counting**: free-text surfaces count token REPLACEMENTS, not
//!   lines — a line carrying two `lumen` tokens counts 2.
//! * **Missing assets are reported, not fatal** — existence checks belong
//!   to the config validator (card 1.E2 owns that wiring).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::XtaskError;

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

/// What the command line asked for.
#[derive(Debug)]
enum Invocation {
    /// `--help` / `-h`.
    Help,
    /// `--config <path>` (required) with optional `--dest <dir>`.
    Rebrand {
        config: PathBuf,
        dest: Option<PathBuf>,
    },
}

const USAGE: &str = "usage: cargo xtask rebrand --config <product.toml> [--dest <dir>]

Rewrites the Lumen identity into a COPY of the identity-bearing surface
set (doc 07 §8). The working tree is never modified. The default dest is
target/rebrand-<short-name>/.
";

fn parse_args(args: &[String]) -> Result<Invocation, XtaskError> {
    let mut config: Option<String> = None;
    let mut dest: Option<String> = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(Invocation::Help),
            "--config" => {
                if config.is_some() {
                    return Err(XtaskError::Usage(
                        "`--config` given more than once".to_string(),
                    ));
                }
                config = Some(
                    rest.next()
                        .ok_or_else(|| XtaskError::Usage("`--config` requires a path".to_string()))?
                        .clone(),
                );
            }
            "--dest" => {
                if dest.is_some() {
                    return Err(XtaskError::Usage(
                        "`--dest` given more than once".to_string(),
                    ));
                }
                dest = Some(
                    rest.next()
                        .ok_or_else(|| XtaskError::Usage("`--dest` requires a path".to_string()))?
                        .clone(),
                );
            }
            other => {
                return Err(XtaskError::Usage(format!("unexpected argument `{other}`")));
            }
        }
    }
    match config {
        Some(config) => Ok(Invocation::Rebrand {
            config: PathBuf::from(config),
            dest: dest.map(PathBuf::from),
        }),
        None => Err(XtaskError::Usage(
            "`--config <path>` is required".to_string(),
        )),
    }
}

/// Entry point (frozen-dispatch shape, card 1.A3).
pub fn run(args: &[String]) -> Result<ExitCode, XtaskError> {
    match parse_args(args)? {
        Invocation::Help => {
            print!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        Invocation::Rebrand { config, dest } => {
            let code = rebrand(&config, dest.as_deref());
            Ok(ExitCode::from(code))
        }
    }
}

// ---------------------------------------------------------------------------
// Identity model (the 19 rewritten fields)
// ---------------------------------------------------------------------------

/// The identity extracted from a product definition — exactly the fields
/// doc 07 §8.2's propagation matrix marks as rewrite-bearing in v0.
struct Identity {
    name: String,
    short_name: String,
    publisher: String,
    display_version: String,
    reverse_dns: String,
    app_user_model_id: String,
    msi_upgrade_code: String,
    bundle_id: String,
    appstream_id: String,
    web_app_id: String,
    accent_color: String,
    icon_ico: String,
    icon_icns: String,
    icon_png_512: String,
    logo_svg: String,
    urls_help: String,
    urls_support: String,
    urls_privacy: String,
    urls_update_feed: String,
}

impl Identity {
    /// The 19 `(dotted.key.path, value)` pairs, in stable order.
    fn fields(&self) -> [(&'static str, &str); 19] {
        [
            ("identity.name", &self.name),
            ("identity.short_name", &self.short_name),
            ("identity.publisher", &self.publisher),
            ("identity.display_version", &self.display_version),
            ("identity.ids.reverse_dns", &self.reverse_dns),
            (
                "identity.ids.windows.app_user_model_id",
                &self.app_user_model_id,
            ),
            (
                "identity.ids.windows.msi_upgrade_code",
                &self.msi_upgrade_code,
            ),
            ("identity.ids.macos.bundle_id", &self.bundle_id),
            ("identity.ids.linux.appstream_id", &self.appstream_id),
            ("identity.ids.web.app_id", &self.web_app_id),
            ("brand.accent_color", &self.accent_color),
            ("brand.icon_ico", &self.icon_ico),
            ("brand.icon_icns", &self.icon_icns),
            ("brand.icon_png_512", &self.icon_png_512),
            ("brand.logo_svg", &self.logo_svg),
            ("urls.help", &self.urls_help),
            ("urls.support", &self.urls_support),
            ("urls.privacy", &self.urls_privacy),
            ("urls.update_feed", &self.urls_update_feed),
        ]
    }
}

// ---------------------------------------------------------------------------
// Std-only TOML-subset reader (flat string-key tables)
// ---------------------------------------------------------------------------

/// A parse failure with a 1-based line number and an actionable message.
struct ParseErr {
    line: usize,
    message: String,
}

/// One `key = "value"` entry resolved against its `[table]`.
struct Entry {
    table: String,
    key: String,
    value: String,
}

/// Parse the TOML subset the rebrand config needs: `[table]` /
/// `[a.b.c]` headers and `key = "value"` lines (comments `#`, blank
/// lines, integers/booleans are skipped as non-string values).
fn parse_toml_subset(src: &str) -> Result<Vec<Entry>, ParseErr> {
    let mut entries = Vec::new();
    let mut table = String::new();
    for (idx, raw) in src.lines().enumerate() {
        let line = raw.trim();
        let lineno = idx + 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = table_header_name(line) {
            table = name;
            continue;
        }
        if line.starts_with('[') {
            return Err(ParseErr {
                line: lineno,
                message: "table header missing `]`".to_string(),
            });
        }
        match string_assignment(line) {
            Some((key, _eq, span)) => {
                let value = line[span].to_string();
                if bad_escape(&value) {
                    return Err(ParseErr {
                        line: lineno,
                        message: "unsupported escape".to_string(),
                    });
                }
                if entries
                    .iter()
                    .any(|e: &Entry| e.table == table && e.key == key)
                {
                    return Err(ParseErr {
                        line: lineno,
                        message: format!("duplicate key `{key}`"),
                    });
                }
                entries.push(Entry {
                    table: table.clone(),
                    key,
                    value,
                });
            }
            None => {
                // Non-string values (integers, booleans, arrays) are
                // valid TOML this reader does not model — SKIP them;
                // only malformed lines are errors.
                if let Some(message) = reject_or_skip(line) {
                    return Err(ParseErr {
                        line: lineno,
                        message,
                    });
                }
            }
        }
    }
    Ok(entries)
}

/// For a line that is neither header nor a well-formed `key = "value"`:
/// `Some(message)` rejects it, `None` skips it (a non-string assignment —
/// integer, boolean or array value the reader does not model).
fn reject_or_skip(line: &str) -> Option<String> {
    if line.contains("'''") || line.contains("\"\"\"") {
        return Some("multi-line strings unsupported".to_string());
    }
    let Some((_, rest)) = line.split_once('=') else {
        // not an assignment at all — reject
        return Some("expected `key = value`".to_string());
    };
    let rest = rest.trim_start();
    if rest.is_empty() {
        return Some("missing value".to_string());
    }
    if !rest.starts_with('"') {
        // integer / boolean / array value — skip, not an error
        return None;
    }
    // A string value that failed `string_assignment`: either it never
    // closes, or non-comment text follows the closing quote.
    let mut escaped = false;
    let mut closed = false;
    for c in rest[1..].chars() {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            closed = true;
            break;
        }
    }
    if !closed {
        Some("unterminated string".to_string())
    } else {
        Some("unexpected text after string".to_string())
    }
}

/// The escape sequences this subset accepts: `\"` and `\\` only —
/// identity values carry no control-character escapes (any `\x`/`\u`
/// and friends are rejected as unsupported).
fn bad_escape(value: &str) -> bool {
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') | Some('\\') => {}
                _ => return true,
            }
        }
    }
    false
}

/// `[table.name]` (plain or `[[array]]`) → the dotted name.
fn table_header_name(line: &str) -> Option<String> {
    let header = if let Some(rest) = line.strip_prefix("[[") {
        rest.strip_suffix("]]")?
    } else {
        let rest = line.strip_prefix('[')?;
        rest.strip_suffix(']')?
    };
    Some(header.trim().to_string())
}

/// A `key = "value"` line: returns the bare key, the byte index of `=`,
/// and the value's byte span *within the (trimmed) line* — excluding the
/// quotes, escape sequences kept verbatim.
fn string_assignment(line: &str) -> Option<(String, usize, std::ops::Range<usize>)> {
    let eq = line.find('=')?;
    let key = line[..eq].trim_end();
    if key.is_empty() || !key.chars().all(is_bare_key_char) {
        return None;
    }
    let rest = line[eq + 1..].trim_start();
    let mut chars = rest.char_indices();
    let (_, first) = chars.next()?;
    if first != '"' {
        return None;
    }
    let mut close = None;
    let mut escaped = false;
    for (idx, c) in chars {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            close = Some(idx);
            break;
        }
    }
    let close = close?;
    let tail = rest[close + 1..].trim();
    if !tail.is_empty() && !tail.starts_with('#') {
        return None;
    }
    let rest_offset = line.len() - rest.len();
    Some((key.to_string(), eq, rest_offset + 1..rest_offset + close))
}

fn is_bare_key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// Extract the full 19-field identity, reporting ALL missing fields
/// together (never one at a time — doc 07 §3's actionable-diagnostics
/// posture).
fn extract_identity(entries: &[Entry]) -> Result<Identity, Vec<String>> {
    let get = |table: &str, key: &str| -> Option<&str> {
        entries
            .iter()
            .find(|e| e.table == table && e.key == key)
            .map(|e| e.value.as_str())
    };
    let mut missing = Vec::new();
    let mut need = |table: &str, key: &str, slot: &mut String| match get(table, key) {
        Some(v) => *slot = v.to_string(),
        None => missing.push(format!("{table}.{key}")),
    };
    let mut id = blank_identity();
    need("identity", "name", &mut id.name);
    need("identity", "short_name", &mut id.short_name);
    need("identity", "publisher", &mut id.publisher);
    need("identity", "display_version", &mut id.display_version);
    need("identity.ids", "reverse_dns", &mut id.reverse_dns);
    need(
        "identity.ids.windows",
        "app_user_model_id",
        &mut id.app_user_model_id,
    );
    need(
        "identity.ids.windows",
        "msi_upgrade_code",
        &mut id.msi_upgrade_code,
    );
    need("identity.ids.macos", "bundle_id", &mut id.bundle_id);
    need("identity.ids.linux", "appstream_id", &mut id.appstream_id);
    need("identity.ids.web", "app_id", &mut id.web_app_id);
    need("brand", "accent_color", &mut id.accent_color);
    need("brand", "icon_ico", &mut id.icon_ico);
    need("brand", "icon_icns", &mut id.icon_icns);
    need("brand", "icon_png_512", &mut id.icon_png_512);
    need("brand", "logo_svg", &mut id.logo_svg);
    need("urls", "help", &mut id.urls_help);
    need("urls", "support", &mut id.urls_support);
    need("urls", "privacy", &mut id.urls_privacy);
    need("urls", "update_feed", &mut id.urls_update_feed);
    if missing.is_empty() {
        Ok(id)
    } else {
        Err(missing)
    }
}

fn blank_identity() -> Identity {
    let s = || String::new();
    Identity {
        name: s(),
        short_name: s(),
        publisher: s(),
        display_version: s(),
        reverse_dns: s(),
        app_user_model_id: s(),
        msi_upgrade_code: s(),
        bundle_id: s(),
        appstream_id: s(),
        web_app_id: s(),
        accent_color: s(),
        icon_ico: s(),
        icon_icns: s(),
        icon_png_512: s(),
        logo_svg: s(),
        urls_help: s(),
        urls_support: s(),
        urls_privacy: s(),
        urls_update_feed: s(),
    }
}

// ---------------------------------------------------------------------------
// product.toml rewriter
// ---------------------------------------------------------------------------

/// The result of one surface rewrite.
struct Rewrite {
    text: String,
    count: usize,
}

/// Rewrite `product.toml`: for every identity key, replace the value
/// inside the matching `[table]` with the target's. Trailing comments,
/// spacing and all non-identity content are preserved byte-exactly.
/// Replacements where the value already equals the target's are elided,
/// so an identity-equal run produces a byte-identical copy.
fn rewrite_product_toml(src: &str, target: &Identity) -> Rewrite {
    let rewrite_map: std::collections::BTreeMap<(&str, &str), &str> = target
        .fields()
        .iter()
        .filter_map(|(key_path, value)| {
            key_path
                .rsplit_once('.')
                .map(|(table, key)| ((table, key), *value))
        })
        .collect();
    let mut count = 0usize;
    let mut table = String::new();
    let mut out: Vec<String> = Vec::new();
    for raw in src.split('\n') {
        let line = raw.trim();
        if let Some(name) = table_header_name(line) {
            table = name;
            out.push(raw.to_string());
            continue;
        }
        if let Some((key, _eq, span)) = string_assignment(line)
            && let Some(new_value) = rewrite_map.get(&(table.as_str(), key.as_str()))
            && &line[span.clone()] != *new_value
        {
            // Rebuild the line: everything through the opening quote
            // (`key = "` with the ORIGINAL spacing — span.start is the
            // first content byte) + the new value + everything from the
            // closing quote onward (quote + trailing comment). The span
            // covers only the bytes BETWEEN the quotes, so no quote is
            // added or duplicated here.
            let indent_len = raw.len() - line.len();
            let mut new_raw = String::with_capacity(raw.len() + new_value.len());
            new_raw.push_str(&raw[..indent_len]);
            new_raw.push_str(&line[..span.start]);
            new_raw.push_str(new_value);
            new_raw.push_str(&line[span.end..]);
            count += 1;
            out.push(new_raw);
            continue;
        }
        out.push(raw.to_string());
    }
    // `split('\n')` + `join("\n")` round-trips the trailing newline.
    Rewrite {
        text: out.join("\n"),
        count,
    }
}

// ---------------------------------------------------------------------------
// Free-text token rewriter (README, generated docs)
// ---------------------------------------------------------------------------

/// Build the source→target token table, LONGEST FIRST so hosts and ids
/// rewrite before their substrings (`updates.lumen-player…` before
/// `lumen-player…` before `Lumen`).
fn token_table(source: &Identity, target: &Identity) -> Vec<(String, String)> {
    let mut tokens: Vec<(String, String)> = vec![
        (
            source.urls_update_feed.clone(),
            target.urls_update_feed.clone(),
        ),
        (source.urls_help.clone(), target.urls_help.clone()),
        (source.urls_support.clone(), target.urls_support.clone()),
        (source.urls_privacy.clone(), target.urls_privacy.clone()),
        (
            source.app_user_model_id.clone(),
            target.app_user_model_id.clone(),
        ),
        (source.bundle_id.clone(), target.bundle_id.clone()),
        (source.appstream_id.clone(), target.appstream_id.clone()),
        (source.web_app_id.clone(), target.web_app_id.clone()),
        (
            source.msi_upgrade_code.clone(),
            target.msi_upgrade_code.clone(),
        ),
        (source.reverse_dns.clone(), target.reverse_dns.clone()),
        (source.publisher.clone(), target.publisher.clone()),
        (source.accent_color.clone(), target.accent_color.clone()),
        (source.name.clone(), target.name.clone()),
        (source.short_name.clone(), target.short_name.clone()),
        (source.name.to_lowercase(), target.name.to_lowercase()),
        (
            source.short_name.to_lowercase(),
            target.short_name.to_lowercase(),
        ),
    ];
    tokens.retain(|(from, to)| !from.is_empty() && from != to);
    tokens.sort_by_key(|(from, _)| std::cmp::Reverse(from.len()));
    tokens.dedup_by(|a, b| a.0 == b.0);
    tokens
}

/// Replace every token occurrence (longest-first, single left-to-right
/// pass per position — earlier tokens cannot be corrupted by later ones
/// because each position is consumed exactly once). Returns the rewritten
/// text and the number of replacements.
fn rewrite_tokens(src: &str, tokens: &[(String, String)]) -> Rewrite {
    let mut out = String::with_capacity(src.len());
    let mut count = 0usize;
    let mut rest = src;
    'outer: while !rest.is_empty() {
        for (from, to) in tokens {
            if rest.starts_with(from.as_str()) {
                out.push_str(to);
                rest = &rest[from.len()..];
                count += 1;
                continue 'outer;
            }
        }
        let cut = rest.char_indices().nth(1).map_or(rest.len(), |(i, _)| i);
        out.push_str(&rest[..cut]);
        rest = &rest[cut..];
    }
    Rewrite { text: out, count }
}

// ---------------------------------------------------------------------------
// Allowlist
// ---------------------------------------------------------------------------

/// One `[[exempt]]` entry (integrity: path + reason + owner, doc 07
/// §10.4 posture).
struct Exempt {
    path: String,
    #[allow(dead_code)]
    reason: String,
    #[allow(dead_code)]
    owner: String,
}

/// Parse the allowlist; every entry must carry path+reason+owner.
fn parse_allowlist(src: &str) -> Result<Vec<Exempt>, String> {
    // Collect each [[exempt]] block's key/value pairs first, then map to
    // Exempt with integrity validation — a block that never saw `path`
    // must still be caught (reason/owner without path is an integrity
    // error, not a silently-dropped block).
    let mut blocks: Vec<Vec<(String, String)>> = Vec::new();
    let mut table = String::new();
    for raw in src.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = table_header_name(line) {
            table = name;
            if table == "exempt" {
                blocks.push(Vec::new());
            }
            continue;
        }
        if table == "exempt"
            && let Some((key, _eq, span)) = string_assignment(line)
            && let Some(block) = blocks.last_mut()
        {
            block.push((key, line[span].to_string()));
        } else if table == "exempt" {
            return Err(format!("bad allowlist line: `{line}`"));
        }
    }
    let mut out = Vec::new();
    for block in &blocks {
        let get = |key: &str| {
            block
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        let (path, reason, owner) = (get("path"), get("reason"), get("owner"));
        if path.is_empty() || reason.is_empty() || owner.is_empty() {
            return Err("every [[exempt]] block needs path + reason + owner".to_string());
        }
        out.push(Exempt {
            path,
            reason,
            owner,
        });
    }
    Ok(out)
}

/// Exact match or recursive directory prefix (trailing `/`).
fn is_exempt(entries: &[Exempt], path: &str) -> bool {
    entries.iter().any(|e| {
        if e.path.ends_with('/') {
            path.starts_with(e.path.as_str())
        } else {
            path == e.path
        }
    })
}

// ---------------------------------------------------------------------------
// Orchestration
// ---------------------------------------------------------------------------

/// The v0 surface set (see the module docs).
const SURFACES: [&str; 4] = [
    "product.toml",
    "README.md",
    "schemas/generated/settings-docs.md",
    "schemas/generated/settings-help.txt",
];

/// Find the workspace root (same discovery rule as `gate`, card 1.A3 —
/// duplicated here because gate.rs's helper is private to that module).
fn find_workspace_root(start: &Path) -> Result<PathBuf, XtaskError> {
    for dir in start.ancestors() {
        let manifest = dir.join("Cargo.toml");
        if let Ok(text) = fs::read_to_string(&manifest)
            && text
                .lines()
                .any(|line| line.trim_start().starts_with("[workspace]"))
        {
            return Ok(dir.to_path_buf());
        }
    }
    Err(XtaskError::internal(
        "no [workspace] Cargo.toml above the current directory",
    ))
}

/// Lowercase, filesystem-safe slug of a product name.
fn slug(name: &str) -> String {
    let mut s = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c.to_ascii_lowercase());
        } else if (c == ' ' || c == '_' || c == '-') && !s.ends_with('-') && !s.is_empty() {
            s.push('-');
        }
    }
    s.trim_end_matches('-').to_string()
}

/// Run the rebrand; returns the process exit code (0 success, 2 error).
/// Every failure prints `rebrand: <message>` on stderr.
fn rebrand(config: &Path, dest: Option<&Path>) -> u8 {
    match rebrand_inner(config, dest) {
        Ok(()) => 0,
        Err(message) => {
            eprintln!("rebrand: {message}");
            2
        }
    }
}

fn rebrand_inner(config: &Path, dest: Option<&Path>) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let root = find_workspace_root(&cwd).map_err(|e| e.to_string())?;

    // --- load target identity ------------------------------------------------
    let target_text = fs::read_to_string(config)
        .map_err(|e| format!("cannot read config {}: {e}", config.display()))?;
    let entries = parse_toml_subset(&target_text)
        .map_err(|err| format!("config is malformed at line {}: {}", err.line, err.message))?;
    let target = extract_identity(&entries)
        .map_err(|missing| format!("missing identity field(s): {}", missing.join(", ")))?;

    // --- load source identity (from the workspace product.toml) --------------
    let product_path = root.join("product.toml");
    let source_text = fs::read_to_string(&product_path)
        .map_err(|e| format!("cannot read {}: {e}", product_path.display()))?;
    let source_entries = parse_toml_subset(&source_text).map_err(|err| {
        format!(
            "product.toml is malformed at line {}: {}",
            err.line, err.message
        )
    })?;
    let source = extract_identity(&source_entries).map_err(|missing| {
        format!(
            "product.toml is missing identity field(s): {}",
            missing.join(", ")
        )
    })?;

    // --- dest resolution ------------------------------------------------------
    let dest = match dest {
        Some(d) => d.to_path_buf(),
        None => root
            .join("target")
            .join(format!("rebrand-{}", slug(&target.short_name))),
    };
    if dest.exists() {
        return Err(format!(
            "dest {} already exists — refusing to overwrite (delete it first)",
            dest.display()
        ));
    }

    // --- allowlist --------------------------------------------------------------
    let allowlist_path = root.join("tools/xtask/data/rebrand-allowlist.toml");
    let allowlist_text = fs::read_to_string(&allowlist_path)
        .map_err(|e| format!("cannot read {}: {e}", allowlist_path.display()))?;
    let allowlist = parse_allowlist(&allowlist_text)?;
    let exempt_surfaces = SURFACES.iter().filter(|s| is_exempt(&allowlist, s)).count();

    // --- report header ----------------------------------------------------------
    println!(
        "rebrand: source `{}` ({}) -> target `{}` ({})",
        source.name, source.reverse_dns, target.name, target.reverse_dns
    );
    println!("dest: {}", dest.display());
    println!(
        "allowlist: {} exempt path(s), {} in the v0 surface set",
        allowlist.len(),
        exempt_surfaces
    );

    // --- surface rewrites ---------------------------------------------------------
    let tokens = token_table(&source, &target);
    let mut total_rewrites = 0usize;
    let mut touched = 0usize;
    for surface in SURFACES {
        let text = fs::read_to_string(root.join(surface))
            .map_err(|e| format!("cannot read surface {surface}: {e}"))?;
        let rewrite = if surface == "product.toml" {
            rewrite_product_toml(&text, &target)
        } else {
            rewrite_tokens(&text, &tokens)
        };
        let dest_path = dest.join(surface);
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&dest_path, &rewrite.text).map_err(|e| e.to_string())?;
        println!(
            "touched: {surface} ({} {})",
            rewrite.count,
            if rewrite.count == 1 {
                "rewrite"
            } else {
                "rewrites"
            }
        );
        total_rewrites += rewrite.count;
        touched += usize::from(rewrite.count > 0);
    }

    // --- brand assets ----------------------------------------------------------------
    let mut copied = 0usize;
    for asset in [
        &target.icon_ico,
        &target.icon_icns,
        &target.icon_png_512,
        &target.logo_svg,
    ] {
        let src_path = root.join(asset);
        match fs::read(&src_path) {
            Ok(bytes) => {
                let dest_path = dest.join(asset);
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                fs::write(&dest_path, &bytes).map_err(|e| e.to_string())?;
                println!("copied-asset: {asset} ({} bytes)", bytes.len());
                copied += 1;
            }
            Err(_) => {
                // Reported, not fatal: existence checks belong to the
                // config validator (card 1.E2 owns that wiring).
                println!("missing-asset: {asset}");
            }
        }
    }

    // --- result line --------------------------------------------------------------------
    if total_rewrites == 0 {
        println!("result: no-op");
    } else {
        println!(
            "result: {touched} surface(s) touched, {total_rewrites} rewrites, {copied} asset(s) copied"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // The committed product definition, trimmed to the identity-bearing
    // tables (kept in sync by test below against the real file).
    const SAMPLE_PRODUCT: &str = r##"schema_version = 1

[identity]
name = "Lumen"
short_name = "Lumen"
publisher = "Lumen Contributors"   # [PROPOSAL] placeholder entity
display_version = "0.1.0-dev"
version_code = 1
description_key = "product.description"

[identity.ids]
reverse_dns = "dev.lumen"

[identity.ids.windows]
app_user_model_id = "dev.lumen.Lumen"
msi_upgrade_code = "7c1f6a52-9d3e-4b8a-a001-3f9c2d64e701"

[identity.ids.macos]
bundle_id = "dev.lumen.Lumen"

[identity.ids.linux]
appstream_id = "dev.lumen.Lumen"

[identity.ids.web]
app_id = "dev.lumen.Lumen"

[window]
title_template = "{media_title} — {product_name}"

[brand]
accent_color = "#4f8cff"
icon_ico = "assets/brand/icon.ico"
icon_icns = "assets/brand/icon.icns"
icon_png_512 = "assets/brand/icon-512.png"
logo_svg = "assets/brand/logo.svg"

[urls]
help = "https://lumen-player.example.org/docs"
support = "https://lumen-player.example.org/support"
privacy = "https://lumen-player.example.org/privacy"
update_feed = "https://updates.lumen-player.example.org/feed.json"

[migrations]
# entries = []
"##;

    const SAMPLE_README: &str = "# Lumen Media Player (working name)

A clean-room, from-scratch reimplementation of VLC media player.

Working name \"Lumen\" is a placeholder only — the product is fully rebrandable via
`product.toml`.
";

    /// The Lumen-shaped identity extracted from SAMPLE_PRODUCT.
    fn sample_identity() -> Identity {
        match identity_of(SAMPLE_PRODUCT) {
            Ok(identity) => identity,
            Err(message) => unreachable!("SAMPLE_PRODUCT is broken: {message}"),
        }
    }

    fn identity_of(text: &str) -> Result<Identity, String> {
        let entries =
            parse_toml_subset(text).map_err(|err| format!("line {}: {}", err.line, err.message))?;
        extract_identity(&entries).map_err(|missing| missing.join(", "))
    }

    fn nova_identity() -> Identity {
        Identity {
            name: "Nova Player".to_string(),
            short_name: "Nova".to_string(),
            publisher: "Nova Player Contributors".to_string(),
            display_version: "0.2.0-nova".to_string(),
            reverse_dns: "org.novaplayer".to_string(),
            app_user_model_id: "org.novaplayer.NovaPlayer".to_string(),
            msi_upgrade_code: "9a6f2c48-1e5b-4c3a-8d7f-2b4e6a8c0d1f".to_string(),
            bundle_id: "org.novaplayer.NovaPlayer".to_string(),
            appstream_id: "org.novaplayer.NovaPlayer".to_string(),
            web_app_id: "org.novaplayer.NovaPlayer".to_string(),
            accent_color: "#ff5500".to_string(),
            icon_ico: "tests/rebrand/assets/icon.ico".to_string(),
            icon_icns: "tests/rebrand/assets/icon.icns".to_string(),
            icon_png_512: "tests/rebrand/assets/icon-512.png".to_string(),
            logo_svg: "tests/rebrand/assets/logo.svg".to_string(),
            urls_help: "https://nova-player.example.org/docs".to_string(),
            urls_support: "https://nova-player.example.org/support".to_string(),
            urls_privacy: "https://nova-player.example.org/privacy".to_string(),
            urls_update_feed: "https://updates.nova-player.example.org/feed.json".to_string(),
        }
    }

    // ---- reader: happy paths --------------------------------------------------

    #[test]
    fn reader_parses_nested_tables_and_skips_non_string_values() {
        let lines = match parse_toml_subset(SAMPLE_PRODUCT) {
            Ok(lines) => lines,
            Err(err) => unreachable!("parse failed at line {}: {}", err.line, err.message),
        };
        let get = |table: &str, key: &str| {
            lines
                .iter()
                .find(|e| e.table == table && e.key == key)
                .map(|e| e.value.as_str())
        };
        assert_eq!(get("identity", "name"), Some("Lumen"));
        assert_eq!(
            get("identity.ids.windows", "app_user_model_id"),
            Some("dev.lumen.Lumen")
        );
        assert_eq!(
            get("urls", "help"),
            Some("https://lumen-player.example.org/docs")
        );
        // integers/booleans are skipped as non-string values
        assert_eq!(get("identity", "version_code"), None);
    }

    #[test]
    fn reader_rejects_malformed_lines_with_line_numbers() {
        for (text, needle) in [
            (
                "[identity]\nname = \"Lumen\"\nbad line\n",
                "expected `key = value`",
            ),
            ("[identity]\nname = \"unterminated\n", "unterminated string"),
            ("[identity.\nname = \"x\"\n", "table header missing"),
            (
                "[identity]\nname = \"a\"\nname = \"b\"\n",
                "duplicate key `name`",
            ),
            ("[identity]\nname =\n", "missing value"),
            (
                "[identity]\nname = \"a\" trailing\n",
                "unexpected text after string",
            ),
            ("[identity]\nname = '''x'''\n", "multi-line strings"),
            (
                "[identity]\nname = \"bad \\x escape\"\n",
                "unsupported escape",
            ),
        ] {
            let err = match parse_toml_subset(text) {
                Err(err) => err,
                Ok(_) => unreachable!("expected an error for {text:?}"),
            };
            assert!(
                err.message.contains(needle),
                "line {}: `{}` does not mention {needle:?}",
                err.line,
                err.message
            );
            assert!(err.line >= 1, "error carries a 1-based line number");
        }
    }

    #[test]
    fn missing_identity_fields_are_all_reported_together() {
        let lines = match parse_toml_subset("[identity]\nname = \"X\"\n") {
            Ok(lines) => lines,
            Err(err) => unreachable!("parse failed at line {}: {}", err.line, err.message),
        };
        let missing = match extract_identity(&lines) {
            Err(missing) => missing,
            Ok(_) => unreachable!("expected a missing-field error"),
        };
        assert!(missing.contains(&"identity.short_name".to_string()));
        assert!(missing.contains(&"identity.ids.reverse_dns".to_string()));
        assert!(missing.contains(&"brand.accent_color".to_string()));
        assert!(missing.contains(&"urls.update_feed".to_string()));
        assert_eq!(missing.len(), 18, "1 of 19 fields present -> 18 missing");
    }

    // ---- product.toml rewriter ---------------------------------------------------

    #[test]
    fn product_toml_rewrite_replaces_identity_values_in_table_context() {
        let target = nova_identity();
        let rewrite = rewrite_product_toml(SAMPLE_PRODUCT, &target);
        assert!(rewrite.text.contains("name = \"Nova Player\""));
        assert!(rewrite.text.contains(
            "publisher = \"Nova Player Contributors\"   # [PROPOSAL] placeholder entity"
        ));
        assert!(rewrite.text.contains("reverse_dns = \"org.novaplayer\""));
        assert!(
            rewrite
                .text
                .contains("app_user_model_id = \"org.novaplayer.NovaPlayer\"")
        );
        assert!(rewrite.text.contains("accent_color = \"#ff5500\""));
        assert!(
            rewrite
                .text
                .contains("help = \"https://nova-player.example.org/docs\"")
        );
        // Non-identity content is preserved.
        assert!(rewrite.text.contains("# entries = []"));
        assert!(
            rewrite
                .text
                .contains("title_template = \"{media_title} — {product_name}\"")
        );
        assert!(
            rewrite
                .text
                .contains("description_key = \"product.description\"")
        );
        assert!(rewrite.text.contains("version_code = 1"));
        // No source identity values remain.
        assert!(!rewrite.text.contains("\"Lumen\""));
        assert!(!rewrite.text.contains("dev.lumen"));
        assert!(!rewrite.text.contains("#4f8cff"));
        // All 19 identity keys are rewritten.
        assert_eq!(rewrite.count, 19);
    }

    #[test]
    fn product_toml_rewrite_elides_unchanged_values() {
        let source = sample_identity();
        let rewrite = rewrite_product_toml(SAMPLE_PRODUCT, &source);
        assert_eq!(rewrite.count, 0, "identity-equal run rewrites nothing");
        assert_eq!(rewrite.text, SAMPLE_PRODUCT, "byte-identical no-op");
    }

    #[test]
    fn rebuild_preserves_spacing_and_comments_exactly() {
        // The known production-bug class (original card 1.E4): the value
        // span covers only the bytes between the quotes.
        let src = "[identity]\nname    =  \"Lumen\"   # the working name\n";
        let rewrite = rewrite_product_toml(src, &nova_identity());
        assert_eq!(
            rewrite.text,
            "[identity]\nname    =  \"Nova Player\"   # the working name\n"
        );
        // Tight form is preserved too.
        let tight = "[identity]\nname=\"Lumen\"\n";
        let rewrite = rewrite_product_toml(tight, &nova_identity());
        assert_eq!(rewrite.text, "[identity]\nname=\"Nova Player\"\n");
    }

    // ---- free-text token rewriter ----------------------------------------------------

    #[test]
    fn readme_title_rule_rewrites_name_tokens() {
        let tokens = token_table(&sample_identity(), &nova_identity());
        let rewrite = rewrite_tokens(SAMPLE_README, &tokens);
        assert!(
            rewrite
                .text
                .contains("# Nova Player Media Player (working name)")
        );
        assert!(
            rewrite
                .text
                .contains("Working name \"Nova Player\" is a placeholder")
        );
        assert!(!rewrite.text.contains("Lumen"));
        assert_eq!(rewrite.count, 2, "exactly the two name tokens");
    }

    #[test]
    fn token_table_orders_longest_first_and_drops_identity_equals() {
        let tokens = token_table(&sample_identity(), &nova_identity());
        let lens: Vec<usize> = tokens.iter().map(|(from, _)| from.len()).collect();
        let mut sorted = lens.clone();
        sorted.sort_by(|a, b| b.cmp(a));
        assert_eq!(lens, sorted, "longest first");
        // identity-equal pairs are dropped
        let same = token_table(&sample_identity(), &sample_identity());
        assert!(same.is_empty());
    }

    // ---- allowlist ------------------------------------------------------------------------

    const SAMPLE_ALLOWLIST: &str = "[[exempt]]
path = \"worklog.md\"
reason = \"append-only history\"
owner = \"Lead\"

[[exempt]]
path = \"docs/adr/\"
reason = \"immutable decisions\"
owner = \"Lead\"
";

    #[test]
    fn allowlist_parses_entries_and_matches_files_and_directories() {
        let entries = match parse_allowlist(SAMPLE_ALLOWLIST) {
            Ok(entries) => entries,
            Err(err) => unreachable!("allowlist parse failed: {err}"),
        };
        assert_eq!(entries.len(), 2);
        assert!(is_exempt(&entries, "worklog.md"));
        assert!(is_exempt(&entries, "docs/adr/ADR-001-hardware-decode.md"));
        assert!(!is_exempt(&entries, "README.md"));
        assert!(!is_exempt(&entries, "docs/inventory/formats.md"));
        assert!(
            !is_exempt(&entries, "docs/adr-something-else.md"),
            "directory prefix must not match siblings"
        );
    }

    #[test]
    fn allowlist_integrity_requires_path_reason_and_owner() {
        for broken in [
            "[[exempt]]\npath = \"x\"\nreason = \"y\"\n",
            "[[exempt]]\npath = \"x\"\nowner = \"z\"\n",
            "[[exempt]]\nreason = \"y\"\nowner = \"z\"\n",
        ] {
            match parse_allowlist(broken) {
                Err(err) => assert!(err.contains("path + reason + owner"), "message: {err}"),
                Ok(_) => unreachable!("expected integrity error for {broken:?}"),
            }
        }
    }

    // ---- args -----------------------------------------------------------------------------

    #[test]
    fn args_require_config() {
        let err = match parse_args(&[]) {
            Err(err) => err,
            Ok(_) => unreachable!("expected an error"),
        };
        assert!(err.to_string().contains("`--config <path>` is required"));
    }

    #[test]
    fn args_reject_unknown_flags() {
        let err = match parse_args(&[
            "--config".to_string(),
            "x.toml".to_string(),
            "--no-such-flag".to_string(),
        ]) {
            Err(err) => err,
            Ok(_) => unreachable!("expected an error"),
        };
        assert!(
            err.to_string()
                .contains("unexpected argument `--no-such-flag`")
        );
    }

    #[test]
    fn args_reject_missing_and_duplicate_values() {
        let err = match parse_args(&["--config".to_string()]) {
            Err(err) => err,
            Ok(_) => unreachable!("expected an error"),
        };
        assert!(err.to_string().contains("`--config` requires a path"));
        let err = match parse_args(&[
            "--config".to_string(),
            "a".to_string(),
            "--config".to_string(),
            "b".to_string(),
        ]) {
            Err(err) => err,
            Ok(_) => unreachable!("expected an error"),
        };
        assert!(err.to_string().contains("`--config` given more than once"));
        let err = match parse_args(&[
            "--config".to_string(),
            "a".to_string(),
            "--dest".to_string(),
        ]) {
            Err(err) => err,
            Ok(_) => unreachable!("expected an error"),
        };
        assert!(err.to_string().contains("`--dest` requires a path"));
    }

    #[test]
    fn args_accept_help_config_dest() {
        match parse_args(&["--help".to_string()]) {
            Ok(Invocation::Help) => {}
            other => unreachable!("expected Help, got {other:?}"),
        }
        match parse_args(&[
            "--config".to_string(),
            "nova.toml".to_string(),
            "--dest".to_string(),
            "/tmp/x".to_string(),
        ]) {
            Ok(Invocation::Rebrand { config, dest }) => {
                assert_eq!(config, PathBuf::from("nova.toml"));
                assert_eq!(dest.as_deref(), Some(Path::new("/tmp/x")));
            }
            other => unreachable!("expected Rebrand, got {other:?}"),
        }
    }

    // ---- helpers ---------------------------------------------------------------------------

    #[test]
    fn slug_is_deterministic() {
        assert_eq!(slug("Nova Player"), "nova-player");
        assert_eq!(slug("Lumen"), "lumen");
        assert_eq!(slug("A  B--C"), "a-b-c");
    }
}
