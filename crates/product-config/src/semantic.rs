//! Semantic validation pass (doc 07 §3.4 stage 3): the rules JSON Schema
//! cannot express — filesystem existence, cross-file membership, cross-field
//! consistency, feature-registry intersection (F1/F2).
//!
//! Every input that depends on files that do not exist yet (themes/,
//! locales/, profiles/ listings, the reference catalog, the engine feature
//! registry) is an explicit [`SemanticInputs`] field: the crate is the
//! *mechanism*, the `cargo xtask config-validator` tool (card 1.E2 wiring)
//! supplies the real listings. `SemanticInputs::none()` runs only the pure
//! in-document checks.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::diagnostic::{Diagnostic, Severity, Stage};
use crate::lines::LineIndex;
use crate::model::ProductConfig;

/// Inputs the semantic pass needs from the surrounding repository/tooling.
#[derive(Debug, Clone, Default)]
pub struct SemanticInputs {
    /// Repository root for brand-asset existence checks; `None` skips the
    /// filesystem entirely (pure in-document checks only).
    pub base_dir: Option<PathBuf>,
    /// `themes/<id>.toml` ids (injected enum, doc 07 §3.3).
    pub themes: Option<Vec<String>>,
    /// `locales/<tag>.toml` tags (injected enum, doc 07 §3.3).
    pub locales: Option<Vec<String>>,
    /// `profiles/<id>.toml` ids (injected enum, doc 07 §3.3).
    pub profiles: Option<Vec<String>>,
    /// Message keys present in the reference catalog (doc 07 §3.3 cross-field:
    /// `description_key` / `title_idle_key` must exist).
    pub reference_catalog_keys: Option<Vec<String>>,
    /// The engine's static feature registry (rules F1/F2, doc 07 §2.7).
    pub feature_registry: Option<FeatureRegistry>,
}

impl SemanticInputs {
    /// No external inputs: only pure in-document checks run.
    pub fn none() -> Self {
        SemanticInputs::default()
    }
}

/// The engine feature registry surface the validator needs (doc 07 §2.7):
/// known ids and whether each is implemented.
#[derive(Debug, Clone, Default)]
pub struct FeatureRegistry {
    entries: BTreeMap<String, FeatureStatus>,
}

/// Status of one feature id in the engine's static registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureStatus {
    pub implemented: bool,
}

impl FeatureRegistry {
    /// Build from `(id, implemented)` pairs.
    pub fn from_status<I, S>(entries: I) -> Self
    where
        I: IntoIterator<Item = (S, bool)>,
        S: Into<String>,
    {
        FeatureRegistry {
            entries: entries
                .into_iter()
                .map(|(id, implemented)| (id.into(), FeatureStatus { implemented }))
                .collect(),
        }
    }

    fn status(&self, id: &str) -> Option<FeatureStatus> {
        self.entries.get(id).copied()
    }

    fn known_ids(&self) -> Vec<&str> {
        self.entries.keys().map(String::as_str).collect()
    }
}

/// Reserved product names (clean-room rule, MP §1; doc 07 §3.3(a)). The
/// `name` check is structural (schema `not.enum`); `short_name` is checked
/// here.
pub const RESERVED_NAMES: &[&str] = &["VLC", "VideoLAN", "vlc"];

/// Render a found string value as a quoted literal — the same convention
/// as the structural pass's `describe_value`, so a semantic diagnostic's
/// `value:` field reads `"vlc"`, not `vlc` (uniform error-message
/// contract, doc 07 §3.5).
fn quoted(value: &str) -> String {
    format!("{value:?}")
}

pub(crate) struct Ctx<'a> {
    pub(crate) file: &'a str,
    pub(crate) lines: &'a LineIndex,
    pub(crate) out: Vec<Diagnostic>,
}

impl Ctx<'_> {
    fn push(
        &mut self,
        key_path: &str,
        rule: String,
        expected: String,
        found: String,
        hint: String,
    ) {
        self.out.push(Diagnostic {
            severity: Severity::Error,
            stage: Stage::Semantic,
            file: self.file.to_string(),
            line: self.lines.line_of(key_path),
            key_path: key_path.to_string(),
            rule,
            expected,
            found,
            hint,
        });
    }
}

/// PascalCase of a product name for identifier derivation (doc 07 §3.3(b)):
/// split on spaces/hyphens/underscores, capitalize each word's first letter.
pub fn pascal_case(name: &str) -> String {
    let mut out = String::new();
    for word in name.split([' ', '-', '_']) {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            for c in first.to_uppercase() {
                out.push(c);
            }
            out.push_str(chars.as_str());
        }
    }
    out
}

/// Run the semantic pass over a loaded product config.
pub fn validate_semantics(
    file: &str,
    config: &ProductConfig,
    inputs: &SemanticInputs,
    lines: &LineIndex,
) -> Vec<Diagnostic> {
    let mut ctx = Ctx {
        file,
        lines,
        out: Vec::new(),
    };
    cross_field_checks(&mut ctx, config);
    injected_enums(&mut ctx, config, inputs);
    catalog_checks(&mut ctx, config, inputs);
    feature_checks(&mut ctx, config, inputs);
    asset_checks(&mut ctx, config, inputs);
    ctx.out
}

/// Pure in-document cross-field rules (doc 07 §3.3 cross-field row). Shared
/// with the merge pass, which re-runs them on the *resolved* config.
pub(crate) fn cross_field_checks(ctx: &mut Ctx<'_>, config: &ProductConfig) {
    // §3.3(a): short_name must differ from the reserved list.
    if RESERVED_NAMES.contains(&config.identity.short_name.as_str()) {
        ctx.push(
            "identity.short_name",
            format!(
                "must not be a reserved name (clean-room rule, MP \u{a7}1); reserved: {}",
                RESERVED_NAMES.join(", ")
            ),
            format!("any value except: {}", RESERVED_NAMES.join(", ")),
            quoted(&config.identity.short_name),
            "choose a compact name that is not on the reserved list (docs/MASTER-PROMPT.md \u{a7}1).".to_string(),
        );
    }

    // §3.3(b): derived identifiers must be mutually consistent.
    let derived = format!(
        "{}.{}",
        config.identity.ids.reverse_dns,
        pascal_case(&config.identity.name)
    );
    for (key_path, value) in [
        (
            "identity.ids.windows.app_user_model_id",
            &config.identity.ids.windows.app_user_model_id,
        ),
        (
            "identity.ids.macos.bundle_id",
            &config.identity.ids.macos.bundle_id,
        ),
        (
            "identity.ids.linux.appstream_id",
            &config.identity.ids.linux.appstream_id,
        ),
        ("identity.ids.web.app_id", &config.identity.ids.web.app_id),
    ] {
        if *value != derived {
            ctx.push(
                key_path,
                format!("must equal reverse_dns + \".\" + PascalCase(identity.name), i.e. `{derived}` (doc 07 \u{a7}3.3(b))"),
                derived.clone(),
                quoted(value),
                "derive the identifier from identity.ids.reverse_dns and identity.name so one literal serves every surface (doc 07 \u{a7}2.1, \u{a7}8.2).".to_string(),
            );
        }
    }

    // Cross-field geometry: min_* < the default size (doc 07 §2.2 comments).
    if config.window.min_width >= config.window.width {
        ctx.push(
            "window.min_width",
            "must be < window.width".to_string(),
            format!("an integer < window.width ({})", config.window.width),
            config.window.min_width.to_string(),
            "raise window.width or lower window.min_width; the minimum must leave room for the default layout (doc 07 \u{a7}2.2).".to_string(),
        );
    }
    if config.window.min_height >= config.window.height {
        ctx.push(
            "window.min_height",
            "must be < window.height".to_string(),
            format!("an integer < window.height ({})", config.window.height),
            config.window.min_height.to_string(),
            "raise window.height or lower window.min_height; the minimum must leave room for the default layout (doc 07 \u{a7}2.2).".to_string(),
        );
    }

    // Migration chain: from < to, ascending, no gaps; schema_version == max(to).
    let entries = &config.migrations.entries;
    for (i, entry) in entries.iter().enumerate() {
        let key_path = format!("migrations.entries.{i}.from");
        if entry.from >= entry.to {
            ctx.push(
                &key_path,
                format!("from must be < to ({} >= {})", entry.from, entry.to),
                format!("from < to, i.e. an integer < {}", entry.to),
                entry.from.to_string(),
                "each migration moves the schema forward exactly one step (doc 07 \u{a7}2.11)."
                    .to_string(),
            );
        }
        if i > 0 {
            let prev = match entries.get(i - 1) {
                Some(p) => p,
                None => continue,
            };
            if entry.from != prev.to {
                ctx.push(
                    &format!("migrations.entries.{i}.from"),
                    format!("must continue the chain without gaps: previous entry ends at {}", prev.to),
                    format!("from == {} (the previous entry's `to`)", prev.to),
                    entry.from.to_string(),
                    "migration entries must form one unbroken ascending chain from schema version 1 (doc 07 \u{a7}2.11, \u{a7}3.3).".to_string(),
                );
            }
        }
    }
    let max_to = entries.last().map(|e| e.to);
    let expected_version = max_to.unwrap_or(1);
    if config.schema_version != expected_version {
        ctx.push(
            "schema_version",
            format!(
                "must equal the highest migration target ({expected_version}), or 1 when there are no entries"
            ),
            expected_version.to_string(),
            config.schema_version.to_string(),
            "bump schema_version together with the migration that reaches it — never ahead of it (doc 07 \u{a7}2.5, \u{a7}3.3).".to_string(),
        );
    }

    // Telemetry requires a privacy policy before any data may flow.
    if config.telemetry.enabled {
        match &config.urls.privacy {
            Some(url) if !url.is_empty() => {}
            _ => ctx.push(
                "telemetry.enabled",
                "requires urls.privacy to be set (a privacy policy must exist before data flows)".to_string(),
                "telemetry.enabled = false, or a non-empty https:// urls.privacy".to_string(),
                "telemetry.enabled = true with urls.privacy absent".to_string(),
                "set urls.privacy to the product's privacy policy, or keep telemetry off (doc 07 \u{a7}3.3 cross-field, MP \u{a7}12).".to_string(),
            ),
        }
    }

    // Brand paths must be safe repo-relative strings (static part of the
    // path-existence rule — runs even without a base dir).
    for (key_path, value) in [
        ("brand.icon_ico", &config.brand.icon_ico),
        ("brand.icon_icns", &config.brand.icon_icns),
        ("brand.icon_png_512", &config.brand.icon_png_512),
        ("brand.logo_svg", &config.brand.logo_svg),
    ] {
        if let Some(problem) = unsafe_path_problem(value) {
            ctx.push(
                key_path,
                format!("must be a repo-relative POSIX path ({problem})"),
                "a repo-relative POSIX path: not absolute, no `..`, no backslashes, no drive letters".to_string(),
                (*value).clone(),
                "point at a file inside the repository; existence and magic bytes are then checked against the repo root (doc 07 \u{a7}2.3, \u{a7}3.3).".to_string(),
            );
        }
    }
}

fn unsafe_path_problem(path: &str) -> Option<&'static str> {
    if path.is_empty() {
        Some("empty path")
    } else if path.starts_with('/') {
        Some("absolute path")
    } else if path.contains('\\') {
        Some("backslash separator")
    } else if path
        .as_bytes()
        .first()
        .is_some_and(|b| b.is_ascii_alphabetic())
        && path.as_bytes().get(1) == Some(&b':')
    {
        Some("drive-letter path")
    } else if path.split('/').any(|segment| segment == "..") {
        Some("`..` component")
    } else {
        None
    }
}

fn injected_enums(ctx: &mut Ctx<'_>, config: &ProductConfig, inputs: &SemanticInputs) {
    // One row per injected enum. Placeholder vocabulary per doc 07 §2.4:
    // themes/<id>.toml, locales/<tag>.toml (BCP-47 tag), profiles/<id>.toml.
    struct InjectedEnum<'a> {
        value: String,
        key_path: &'a str,
        listing: &'a Option<Vec<String>>,
        dir: &'a str,
        token: &'a str,
    }
    let checks = [
        InjectedEnum {
            value: config.defaults.theme.clone(),
            key_path: "defaults.theme",
            listing: &inputs.themes,
            dir: "themes/",
            token: "<id>",
        },
        InjectedEnum {
            value: config.defaults.locale.clone(),
            key_path: "defaults.locale",
            listing: &inputs.locales,
            dir: "locales/",
            token: "<tag>",
        },
        InjectedEnum {
            value: config.defaults.profile.clone(),
            key_path: "defaults.profile",
            listing: &inputs.profiles,
            dir: "profiles/",
            token: "<id>",
        },
    ];
    for InjectedEnum {
        value,
        key_path,
        listing,
        dir,
        token,
    } in checks
    {
        if let Some(available) = listing
            && !available.contains(&value)
        {
            ctx.push(
                key_path,
                format!(
                    "must match a `{dir}{token}.toml` file (injected enum from the {dir} listing)"
                ),
                format!("one of: {}", available.join(", ")),
                quoted(&value),
                format!("add `{dir}{value}.toml` or point the default at an existing file (doc 07 \u{a7}3.3 injected enums)."),
            );
        }
    }
}

fn catalog_checks(ctx: &mut Ctx<'_>, config: &ProductConfig, inputs: &SemanticInputs) {
    if let Some(keys) = &inputs.reference_catalog_keys {
        for (value, key_path) in [
            (&config.identity.description_key, "identity.description_key"),
            (&config.window.title_idle_key, "window.title_idle_key"),
        ] {
            if !keys.contains(value) {
                ctx.push(
                    key_path,
                    "must exist in the reference catalog (unresolved msgid reference)".to_string(),
                    "a message key present in locales/en-US.toml".to_string(),
                    quoted(value),
                    "add the key to the reference catalog or reference an existing one (doc 07 \u{a7}3.3 cross-field, \u{a7}6).".to_string(),
                );
            }
        }
    }
}

fn feature_checks(ctx: &mut Ctx<'_>, config: &ProductConfig, inputs: &SemanticInputs) {
    if let Some(registry) = &inputs.feature_registry {
        for (id, exposed) in &config.features {
            let key_path = format!("features.{id}");
            match registry.status(id) {
                None => ctx.push(
                    &key_path,
                    "unknown feature id (rule F1: config cannot even name an unimplemented feature)".to_string(),
                    format!(
                        "one of the engine registry ids: {}",
                        registry.known_ids().join(", ")
                    ),
                    quoted(id),
                    "remove the key or land the feature in the engine's static registry first — config alone enables nothing (doc 07 \u{a7}2.7, \u{a7}3.3).".to_string(),
                ),
                Some(status) if *exposed && !status.implemented => ctx.push(
                    &key_path,
                    "feature is exposed as enabled but its implementation does not exist (rule F2)".to_string(),
                    "features.<id> = false until the implementing crates ship and pass their gate".to_string(),
                    format!("features.{id} = true"),
                    "flip the entry to false, or land the implementation (feature-matrix row + tests) first (doc 07 \u{a7}2.7, \u{a7}3.3, doc 05 \u{a7}6.4).".to_string(),
                ),
                Some(_) => {}
            }
        }
    }
}

fn asset_checks(ctx: &mut Ctx<'_>, config: &ProductConfig, inputs: &SemanticInputs) {
    let base_dir = match &inputs.base_dir {
        Some(dir) => dir,
        None => return,
    };
    let assets: [(&str, &str, AssetKind); 4] = [
        ("brand.icon_ico", &config.brand.icon_ico, AssetKind::Ico),
        ("brand.icon_icns", &config.brand.icon_icns, AssetKind::Icns),
        (
            "brand.icon_png_512",
            &config.brand.icon_png_512,
            AssetKind::Png,
        ),
        ("brand.logo_svg", &config.brand.logo_svg, AssetKind::Svg),
    ];
    for (key_path, rel, kind) in assets {
        let path = Path::new(base_dir).join(rel);
        match std::fs::read(&path) {
            Err(err) => ctx.push(
                key_path,
                format!(
                    "must exist in the repository (checked against {}): {err}",
                    base_dir.display()
                ),
                format!("an existing {} file at `{rel}`", kind.name()),
                format!("missing file at `{}`", path.display()),
                "add the asset, or point the key at the existing file — the rebrand system swaps exactly these files (doc 07 \u{a7}2.3, \u{a7}3.3, \u{a7}8).".to_string(),
            ),
            Ok(bytes) => match kind.magic_ok(&bytes) {
                Ok(()) => {}
                Err(found) => ctx.push(
                    key_path,
                    format!("must be a {} file (magic-byte check)", kind.name()),
                    format!("a {} file ({})", kind.name(), kind.magic()),
                    found,
                    "replace the file with one of the right format; per-platform icons are validated by magic bytes because extensions lie (doc 07 \u{a7}3.3 path existence).".to_string(),
                ),
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum AssetKind {
    Ico,
    Icns,
    Png,
    Svg,
}

impl AssetKind {
    fn name(self) -> &'static str {
        match self {
            AssetKind::Ico => "ICO",
            AssetKind::Icns => "ICNS",
            AssetKind::Png => "PNG",
            AssetKind::Svg => "SVG",
        }
    }

    fn magic(self) -> &'static str {
        match self {
            AssetKind::Ico => "starts with 00 00 01 00",
            AssetKind::Icns => "starts with the ASCII bytes `icns`",
            AssetKind::Png => "starts with 89 50 4E 47 0D 0A 1A 0A",
            AssetKind::Svg => "starts (after whitespace/BOM) with `<`",
        }
    }

    /// Ok(()) when the magic bytes fit; Err(human-readable found).
    fn magic_ok(self, bytes: &[u8]) -> Result<(), String> {
        let starts_with = |prefix: &[u8]| bytes.starts_with(prefix);
        let ok = match self {
            AssetKind::Ico => starts_with(&[0x00, 0x00, 0x01, 0x00]),
            AssetKind::Icns => starts_with(b"icns"),
            AssetKind::Png => starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
            AssetKind::Svg => {
                let mut rest = bytes;
                if rest.starts_with(&[0xEF, 0xBB, 0xBF]) {
                    rest = &rest[3..];
                }
                rest.iter()
                    .find(|b| !b.is_ascii_whitespace())
                    .is_some_and(|b| *b == b'<')
            }
        };
        if ok {
            Ok(())
        } else {
            let head: Vec<String> = bytes.iter().take(8).map(|b| format!("{b:02X}")).collect();
            Err(format!("file starts with {}", head.join(" ")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal_case_derives_identifiers() {
        assert_eq!(pascal_case("Lumen"), "Lumen");
        assert_eq!(pascal_case("Nova Player"), "NovaPlayer");
        assert_eq!(pascal_case("my-cool_player"), "MyCoolPlayer");
    }

    #[test]
    fn path_problems_are_detected() {
        assert!(unsafe_path_problem("/etc/passwd").is_some());
        assert!(unsafe_path_problem("assets/../../etc/passwd").is_some());
        assert!(unsafe_path_problem("assets\\brand\\icon.ico").is_some());
        assert!(unsafe_path_problem("C:\\icon.ico").is_some());
        assert!(unsafe_path_problem("").is_some());
        assert!(unsafe_path_problem("assets/brand/icon.ico").is_none());
    }

    #[test]
    fn magic_bytes_detect_formats() {
        assert!(
            AssetKind::Ico
                .magic_ok(&[0x00, 0x00, 0x01, 0x00, 0x00])
                .is_ok()
        );
        assert!(AssetKind::Ico.magic_ok(b"not-ico").is_err());
        assert!(AssetKind::Icns.magic_ok(b"icns").is_ok());
        assert!(
            AssetKind::Png
                .magic_ok(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
                .is_ok()
        );
        assert!(AssetKind::Svg.magic_ok(b"\n  <?xml").is_ok());
        assert!(AssetKind::Svg.magic_ok(b"plain text").is_err());
    }
}
