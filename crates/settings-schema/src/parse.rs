//! Parsing `schemas/settings.toml` into the typed model with actionable
//! diagnostics (doc 07 §5.2 entry format).
//!
//! Two-stage: `toml` + serde parse into raw optional-field structs (so a
//! missing mandated field becomes *our* diagnostic with the entry's line,
//! not a bare serde error), then a conversion pass that type-checks the
//! value domain and the literal default. Cross-entry and cross-file rules
//! (D1's `product.toml` join, D2–D6) live in [`crate::validate`].
//!
//! Line locations come from a small read-only scanner (the same approach as
//! `product-config`'s `lines.rs`): the `toml` crate exposes spans only on
//! parse errors, so header/assignment lines are indexed by walking the
//! source once. Paths use `setting.<index>.<field>`.

use serde::Deserialize;

use crate::diagnostic::SchemaDiagnostic;
use crate::model::ArrayPolicy;
use crate::model::CliSpec;
use crate::model::DefaultValue;
use crate::model::Domain;
use crate::model::ListElement;
use crate::model::Scope;
use crate::model::SettingDef;
use crate::model::SettingType;
use crate::model::SettingsSchema;
use crate::model::UiPlacement;
use crate::model::ValueRange;
use crate::model::check_value;

/// Parse and structurally validate schema source text. On any violation the
/// FULL diagnostic list is returned (every problem, one pass).
pub fn parse_str(src: &str) -> Result<SettingsSchema, Vec<SchemaDiagnostic>> {
    let raw = match toml::from_str::<RawFile>(src) {
        Ok(raw) => raw,
        Err(err) => {
            let line = err.span().map(|span| line_of_offset(src, span.start));
            return Err(vec![SchemaDiagnostic::error(
                "E-PARSE",
                line,
                "<file>",
                format!("TOML syntax error: {err}"),
            )]);
        }
    };
    let index = LineIndex::new(src);
    let mut diags = Vec::new();

    let areas = parse_areas(&raw, &index, &mut diags);
    let mut settings = Vec::new();
    for (i, entry) in raw.setting.iter().enumerate() {
        let ctx = EntryCtx {
            index: &index,
            header_line: index.line_of(&format!("setting.{i}")).unwrap_or(1),
            i,
        };
        if let Some(def) = convert_entry(&ctx, entry, &mut diags) {
            settings.push(def);
        }
    }
    if diags.is_empty() {
        Ok(SettingsSchema { areas, settings })
    } else {
        Err(diags)
    }
}

// --- raw (serde) shapes -----------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    areas: Option<RawAreas>,
    #[serde(default)]
    setting: Vec<RawSetting>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAreas {
    known: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSetting {
    key: Option<String>,
    r#type: Option<String>,
    range: Option<RawRange>,
    enumeration: Option<Vec<String>>,
    default: Option<toml::Value>,
    default_ref: Option<String>,
    description: Option<String>,
    restart_required: Option<bool>,
    placement: Option<RawPlacement>,
    cli: Option<RawCli>,
    feature_gate: Option<String>,
    since: Option<String>,
    scope: Option<String>,
    array_policy: Option<String>,
    advanced: Option<bool>,
    deprecated_in: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRange {
    min: Option<toml::Value>,
    max: Option<toml::Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPlacement {
    screen: Option<String>,
    section: Option<String>,
    order: Option<u32>,
    ui: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCli {
    flag: Option<String>,
    short: Option<String>,
}

// --- per-entry context (line lookup + diagnostic addressing) ----------------

struct EntryCtx<'a> {
    index: &'a LineIndex,
    header_line: u32,
    i: usize,
}

impl EntryCtx<'_> {
    fn field_line(&self, field: &str) -> u32 {
        self.index
            .line_of(&format!("setting.{}.{}", self.i, field))
            .unwrap_or(self.header_line)
    }

    fn field_path(&self, field: &str) -> String {
        format!("setting[{}].{}", self.i, field)
    }

    fn err(&self, diags: &mut Vec<SchemaDiagnostic>, rule: &str, field: &str, message: String) {
        diags.push(SchemaDiagnostic::error(
            rule,
            Some(self.field_line(field)),
            &self.field_path(field),
            message,
        ));
    }
}

// --- areas ------------------------------------------------------------------

fn parse_areas(raw: &RawFile, index: &LineIndex, diags: &mut Vec<SchemaDiagnostic>) -> Vec<String> {
    let Some(areas) = raw.areas.as_ref() else {
        diags.push(SchemaDiagnostic::error(
            "E-FIELD",
            Some(1),
            "areas",
            "missing mandated `[areas]` table — declare `known = [...]` with the settings \
             screens (doc 07 §5.4); it seeds layouts/settings-areas.toml"
                .to_owned(),
        ));
        return Vec::new();
    };
    let Some(known) = areas.known.as_ref() else {
        diags.push(SchemaDiagnostic::error(
            "E-FIELD",
            index.line_of("areas"),
            "areas.known",
            "missing mandated `areas.known` — the list of settings screen ids (doc 07 §5.4)"
                .to_owned(),
        ));
        return Vec::new();
    };
    if known.is_empty() {
        diags.push(SchemaDiagnostic::error(
            "E-AREAS",
            index.line_of("areas.known"),
            "areas.known",
            "the known-areas list is empty — D5 needs at least one screen id".to_owned(),
        ));
    }
    let mut seen: Vec<&str> = Vec::new();
    for area in known {
        if !is_kebab(area) {
            diags.push(SchemaDiagnostic::error(
                "E-AREAS",
                index.line_of("areas.known"),
                "areas.known",
                format!("area id `{area}` is not kebab-case (`^[a-z0-9]+(-[a-z0-9]+)*$`)"),
            ));
        }
        if seen.contains(&area.as_str()) {
            diags.push(SchemaDiagnostic::error(
                "E-AREAS",
                index.line_of("areas.known"),
                "areas.known",
                format!("duplicate area id `{area}` — screen ids must be unique"),
            ));
        }
        seen.push(area);
    }
    known.clone()
}

// --- one entry --------------------------------------------------------------

/// Convert raw entry `i`; returns the typed def when every mandated field is
/// present (otherwise the caller only sees the diagnostics — parse fails).
fn convert_entry(
    ctx: &EntryCtx<'_>,
    raw: &RawSetting,
    diags: &mut Vec<SchemaDiagnostic>,
) -> Option<SettingDef> {
    // Mandated presence (doc 07 §5.2: every entry carries all ten fields —
    // empty string / false where not applicable — so generators never
    // special-case).
    for (field, present) in [
        ("key", raw.key.is_some()),
        ("type", raw.r#type.is_some()),
        ("description", raw.description.is_some()),
        ("restart_required", raw.restart_required.is_some()),
        ("placement", raw.placement.is_some()),
        ("cli", raw.cli.is_some()),
        ("feature_gate", raw.feature_gate.is_some()),
        ("since", raw.since.is_some()),
    ] {
        if !present {
            ctx.err(
                diags,
                "E-FIELD",
                field,
                format!(
                    "missing mandated field `{field}` — every entry carries all ten doc 07 \
                     §5.2 fields; use an empty string / false where not applicable"
                ),
            );
        }
    }
    match (raw.default.is_some(), raw.default_ref.is_some()) {
        (false, false) => ctx.err(
            diags,
            "D1",
            "default",
            "entry carries neither `default` nor `default_ref` — exactly one is required \
             (rule D1, doc 07 §5.2)"
                .to_owned(),
        ),
        (true, true) => ctx.err(
            diags,
            "D1",
            "default",
            "entry carries both `default` and `default_ref` — exactly one is allowed; \
             product-level defaults are stated once, in product.toml, and referenced via \
             `default_ref` (rule D1, doc 07 §5.2)"
                .to_owned(),
        ),
        _ => {}
    }

    let key = raw.key.clone()?;
    let type_tag = raw.r#type.clone()?;
    let description = raw.description.clone()?;
    let restart_required = raw.restart_required?;
    let raw_placement = raw.placement.as_ref()?;
    let raw_cli = raw.cli.as_ref()?;
    let feature_gate = raw.feature_gate.clone()?;
    let since = raw.since.clone()?;

    // --- simple formats ---
    if !is_setting_key(&key) {
        ctx.err(
            diags,
            "E-FORMAT",
            "key",
            format!(
                "setting key `{key}` must be a dotted kebab-case id `area.key` \
                 (`^[a-z0-9]+(-[a-z0-9]+)*\\.[a-z0-9]+(-[a-z0-9]+)*$`, doc 07 §5.2)"
            ),
        );
    }
    let ty = match SettingType::parse(&type_tag) {
        Some(ty) => ty,
        None => {
            ctx.err(
                diags,
                "E-TYPE",
                "type",
                format!(
                    "unknown type `{type_tag}` — valid: bool, int, float, string, enum, path, \
                     url, list<string>, list<path>, list<url>, list<enum>, duration-ms, \
                     percent"
                ),
            );
            return None;
        }
    };
    if description.is_empty() {
        ctx.err(
            diags,
            "E-FORMAT",
            "description",
            "description must be a localization msgid (e.g. `settings.playback.subtitle-mode`) \
             — never literal prose and never empty (doc 07 §5.2)"
                .to_owned(),
        );
    }
    if !feature_gate.is_empty() && !is_kebab(&feature_gate) {
        ctx.err(
            diags,
            "E-FORMAT",
            "feature_gate",
            format!("feature_gate `{feature_gate}` must be a kebab-case feature id or empty"),
        );
    }
    if !is_version(&since) {
        ctx.err(
            diags,
            "E-FORMAT",
            "since",
            format!("since `{since}` must be a product version like `0.1.0`"),
        );
    }

    // --- domain (range / enumeration) ---
    let domain = convert_domain(ctx, ty, raw, diags);

    // --- default (literal type check; ref existence is D1 in validate) ---
    let default = match (&raw.default, &raw.default_ref) {
        (Some(value), _) => {
            if let Err(v) = check_value(ty, &domain, value) {
                ctx.err(
                    diags,
                    "E-DEFAULT",
                    "default",
                    format!(
                        "literal default {} violates the entry's type/domain ({}): expected \
                         {}, found {} — {}",
                        render_value(value),
                        v.rule,
                        v.expected,
                        v.found,
                        v.hint
                    ),
                );
            }
            DefaultValue::Literal(value.clone())
        }
        (None, Some(reference)) => {
            if !is_snake_path(reference) {
                ctx.err(
                    diags,
                    "E-FORMAT",
                    "default_ref",
                    format!(
                        "default_ref `{reference}` must be a dotted snake_case product.toml \
                         path like `defaults.theme` (rule D1 joins the two spellings, doc 07 \
                         §5.2)"
                    ),
                );
            }
            DefaultValue::Ref(reference.clone())
        }
        (None, None) => DefaultValue::Literal(toml::Value::Boolean(false)),
    };

    // --- placement ---
    let placement = UiPlacement {
        screen: raw_placement.screen.clone().unwrap_or_default(),
        section: raw_placement.section.clone().unwrap_or_default(),
        order: raw_placement.order.unwrap_or(0),
        ui: raw_placement.ui.unwrap_or(true),
    };
    if placement.screen.is_empty() {
        ctx.err(
            diags,
            "E-FIELD",
            "placement.screen",
            "placement.screen is mandated — one of the `[areas].known` screen ids (doc 07 \
             §5.2)"
                .to_owned(),
        );
    }
    if placement.section.is_empty() {
        ctx.err(
            diags,
            "E-FIELD",
            "placement.section",
            "placement.section is mandated — the section within the screen".to_owned(),
        );
    }

    // --- cli ---
    let cli = CliSpec {
        flag: raw_cli.flag.clone().unwrap_or_default(),
        short: raw_cli.short.clone().unwrap_or_default(),
    };
    if !is_long_flag(&cli.flag) {
        ctx.err(
            diags,
            "E-FORMAT",
            "cli.flag",
            format!(
                "cli.flag `{}` must be a long flag `--kebab-case` (short flags come from the \
                 fixed reserved table, doc 07 §5.2)",
                cli.flag
            ),
        );
    }
    if !cli.short.is_empty() && !is_short_flag(&cli.short) {
        ctx.err(
            diags,
            "E-FORMAT",
            "cli.short",
            format!(
                "cli.short `{}` must be a single-letter short flag like `-v` or empty",
                cli.short
            ),
        );
    }

    // --- [PROPOSAL] extensions with defaults ---
    let scope = match raw.scope.as_deref() {
        None | Some("user") => Scope::User,
        Some("session") => Scope::Session,
        Some(other) => {
            ctx.err(
                diags,
                "E-FORMAT",
                "scope",
                format!("scope `{other}` must be `user` or `session` (doc 07 §4.1)"),
            );
            Scope::User
        }
    };
    let array_policy = match raw.array_policy.as_deref() {
        None | Some("replace") => ArrayPolicy::Replace,
        Some("append-unique") => ArrayPolicy::AppendUnique,
        Some(other) => {
            ctx.err(
                diags,
                "E-FORMAT",
                "array_policy",
                format!("array_policy `{other}` must be `replace` or `append-unique`"),
            );
            ArrayPolicy::Replace
        }
    };
    if !matches!(ty, SettingType::List(_)) && array_policy == ArrayPolicy::AppendUnique {
        ctx.err(
            diags,
            "E-FIELD",
            "array_policy",
            "array_policy `append-unique` is only meaningful for list<T> entries (doc 07 \
             §4.4 rule 3)"
                .to_owned(),
        );
    }

    Some(SettingDef {
        key,
        ty,
        domain,
        default,
        description,
        restart_required,
        placement,
        cli,
        feature_gate,
        since,
        scope,
        array_policy,
        advanced: raw.advanced.unwrap_or(false),
        deprecated_in: raw.deprecated_in.clone().unwrap_or_default(),
        line: ctx.header_line,
        index: ctx.i,
    })
}

/// Domain conversion: which of `range`/`enumeration` the type requires, and
/// their well-formedness.
fn convert_domain(
    ctx: &EntryCtx<'_>,
    ty: SettingType,
    raw: &RawSetting,
    diags: &mut Vec<SchemaDiagnostic>,
) -> Domain {
    let has_range = raw.range.is_some();
    let has_enum = raw.enumeration.is_some();
    if has_range && has_enum {
        ctx.err(
            diags,
            "E-DOMAIN",
            "range",
            "range and enumeration are mutually exclusive — pick the domain that matches the \
             entry type (doc 07 §5.2)"
                .to_owned(),
        );
    }
    let needs_range = ty.is_ranged();
    let needs_enum = matches!(ty, SettingType::Enum | SettingType::List(ListElement::Enum));
    if needs_range && !has_range {
        ctx.err(
            diags,
            "E-DOMAIN",
            "range",
            format!(
                "type `{}` requires `range = {{ min = .., max = .. }}` — numeric settings \
                 are always bounded (doc 07 §5.2)",
                ty.as_str()
            ),
        );
    }
    if needs_enum && !has_enum {
        ctx.err(
            diags,
            "E-DOMAIN",
            "enumeration",
            format!(
                "type `{}` requires `enumeration = [..]` listing every member",
                ty.as_str()
            ),
        );
    }
    if !needs_range && has_range {
        ctx.err(
            diags,
            "E-DOMAIN",
            "range",
            format!(
                "type `{}` cannot carry a range — only int/float/duration-ms/percent are \
                 ranged (doc 07 §5.2)",
                ty.as_str()
            ),
        );
    }
    if !needs_enum && has_enum {
        ctx.err(
            diags,
            "E-DOMAIN",
            "enumeration",
            format!(
                "type `{}` cannot carry an enumeration — only enum and list<enum> (doc 07 §5.2)",
                ty.as_str()
            ),
        );
    }

    if let Some(range) = raw.range.as_ref() {
        let min = range.min.as_ref().and_then(value_as_number).or_else(|| {
            ctx.err(
                diags,
                "E-TYPE",
                "range.min",
                "range.min must be a number (an integer for int/duration-ms)".to_owned(),
            );
            None
        });
        let max = range.max.as_ref().and_then(value_as_number).or_else(|| {
            ctx.err(
                diags,
                "E-TYPE",
                "range.max",
                "range.max must be a number (an integer for int/duration-ms)".to_owned(),
            );
            None
        });
        if ty.is_integer_valued() {
            for (side, value) in [("min", range.min.as_ref()), ("max", range.max.as_ref())] {
                if let Some(v) = value
                    && v.as_integer().is_none()
                {
                    ctx.err(
                        diags,
                        "E-TYPE",
                        "range",
                        format!("range.{side} must be an integer for type `{}`", ty.as_str()),
                    );
                }
            }
        }
        match (min, max) {
            (Some(min), Some(max)) => {
                if min > max {
                    ctx.err(
                        diags,
                        "E-DOMAIN",
                        "range",
                        format!("range is inverted: min {min} > max {max}"),
                    );
                }
                Domain::Range(ValueRange { min, max })
            }
            _ => Domain::Unconstrained,
        }
    } else if let Some(members) = raw.enumeration.as_ref() {
        if members.is_empty() {
            ctx.err(
                diags,
                "E-DOMAIN",
                "enumeration",
                "enumeration must list at least one member".to_owned(),
            );
        }
        let mut seen: Vec<&str> = Vec::new();
        for member in members {
            if member.is_empty() || member.contains(char::is_whitespace) {
                ctx.err(
                    diags,
                    "E-DOMAIN",
                    "enumeration",
                    format!("enumeration member `{member}` must be non-empty and single-word"),
                );
            }
            if seen.contains(&member.as_str()) {
                ctx.err(
                    diags,
                    "E-DOMAIN",
                    "enumeration",
                    format!("duplicate enumeration member `{member}`"),
                );
            }
            seen.push(member);
        }
        Domain::Enumeration(members.clone())
    } else {
        Domain::Unconstrained
    }
}

/// A numeric TOML value as `f64` (integers widen — representation, not
/// coercion, for float-typed settings).
fn value_as_number(value: &toml::Value) -> Option<f64> {
    match value {
        toml::Value::Integer(n) => Some(*n as f64),
        toml::Value::Float(f) => Some(*f),
        _ => None,
    }
}

/// Render a literal default for diagnostics (TOML syntax).
fn render_value(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => format!("`\"{s}\"`"),
        other => format!("`{other}`"),
    }
}

// --- format predicates ------------------------------------------------------

fn is_kebab(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

fn is_setting_key(s: &str) -> bool {
    let mut parts = s.split('.');
    let first = parts.next().unwrap_or_default();
    let second = match parts.next() {
        Some(second) => second,
        None => return false,
    };
    if parts.next().is_some() {
        return false;
    }
    is_kebab(first) && is_kebab(second)
}

fn is_snake_path(s: &str) -> bool {
    !s.is_empty()
        && s.split('.').count() >= 2
        && s.split('.').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
}

fn is_long_flag(s: &str) -> bool {
    let body = s.strip_prefix("--").unwrap_or("");
    is_kebab(body)
}

fn is_short_flag(s: &str) -> bool {
    let body = s.strip_prefix('-').unwrap_or("");
    body.len() == 1 && body.chars().all(|c| c.is_ascii_lowercase())
}

fn is_version(s: &str) -> bool {
    let mut parts = s.splitn(3, '.');
    let major = parts.next().unwrap_or_default();
    let minor = parts.next().unwrap_or_default();
    let patch = parts.next().unwrap_or_default();
    let numeric = |p: &str| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit());
    let patch_clean = patch.split(['-', '+']).next().unwrap_or_default();
    numeric(major) && numeric(minor) && numeric(patch_clean)
}

/// 1-based line of a byte offset.
fn line_of_offset(src: &str, offset: usize) -> u32 {
    let bound = offset.min(src.len());
    1 + src.as_bytes()[..bound]
        .iter()
        .filter(|b| **b == b'\n')
        .count() as u32
}

// --- line index (same read-only scanner approach as product-config lines.rs)

/// Maps dotted paths (`areas.known`, `setting.<i>`, `setting.<i>.<field>`)
/// to the 1-based source line that defines them.
struct LineIndex {
    lines: Vec<(String, u32)>,
}

impl LineIndex {
    fn new(src: &str) -> Self {
        let mut lines: Vec<(String, u32)> = Vec::new();
        let mut table: Vec<String> = Vec::new();
        let mut setting_count = 0usize;
        for (idx, raw) in src.lines().enumerate() {
            let lineno = (idx + 1) as u32;
            let stripped = strip_comment(raw);
            let trimmed = stripped.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(header) = table_header(trimmed) {
                let mut path = header.segments;
                if header.is_array {
                    path.push(setting_count.to_string());
                    setting_count += 1;
                }
                lines.push((path.join("."), lineno));
                table = path;
                continue;
            }
            if let Some(key) = assignment_key(trimmed) {
                let mut path = table.clone();
                path.extend(key);
                lines.push((path.join("."), lineno));
            }
        }
        LineIndex { lines }
    }

    fn line_of(&self, path: &str) -> Option<u32> {
        self.lines
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, line)| *line)
    }
}

struct Header {
    segments: Vec<String>,
    is_array: bool,
}

fn table_header(line: &str) -> Option<Header> {
    let is_array = line.starts_with("[[");
    let open = if is_array { "[[" } else { "[" };
    let close = if is_array { "]]" } else { "]" };
    if !line.starts_with(open) || !line.ends_with(close) {
        return None;
    }
    let inner = &line[open.len()..line.len() - close.len()];
    if inner.is_empty() {
        return None;
    }
    let mut segments = Vec::new();
    for seg in inner.split('.') {
        let s = seg.trim();
        if s.is_empty() {
            return None;
        }
        segments.push(unquote(s));
    }
    Some(Header { segments, is_array })
}

fn assignment_key(line: &str) -> Option<Vec<String>> {
    let eq = line.find('=')?;
    let key = line[..eq].trim();
    if key.is_empty() || key.starts_with('[') {
        return None;
    }
    let mut segments = Vec::new();
    for seg in key.split('.') {
        let s = seg.trim();
        if s.is_empty() {
            return None;
        }
        segments.push(unquote(s));
    }
    Some(segments)
}

fn strip_comment(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_basic = false;
    let mut in_literal = false;
    for c in line.chars() {
        match c {
            '"' if !in_literal => in_basic = !in_basic,
            '\'' if !in_basic => in_literal = !in_literal,
            '#' if !in_basic && !in_literal => break,
            _ => {}
        }
        out.push(c);
    }
    out
}

fn unquote(seg: &str) -> String {
    if seg.len() >= 2 {
        let bytes = seg.as_bytes();
        let first = bytes[0];
        let last = bytes[seg.len() - 1];
        if first == b'"' && last == b'"' || first == b'\'' && last == b'\'' {
            return seg[1..seg.len() - 1].to_string();
        }
    }
    seg.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
[areas]
known = ["playback"]

[[setting]]
key = "playback.subtitle-mode"
type = "enum"
enumeration = ["auto", "off"]
default = "auto"
description = "settings.playback.subtitle-mode"
restart_required = false
placement = { screen = "playback", section = "tracks", order = 10, ui = true }
cli = { flag = "--playback-subtitle-mode", short = "" }
feature_gate = ""
since = "0.1.0"
"#;

    fn diags_of(src: &str) -> Vec<SchemaDiagnostic> {
        match parse_str(src) {
            Ok(_) => Vec::new(),
            Err(diags) => diags,
        }
    }

    fn rendered(diags: &[SchemaDiagnostic]) -> String {
        crate::diagnostic::render_all(diags)
    }

    #[test]
    fn minimal_entry_parses() {
        let parsed = parse_str(MINIMAL);
        let failure = match &parsed {
            Err(diags) => rendered(diags),
            Ok(_) => String::new(),
        };
        assert_eq!(failure, "", "parse unexpectedly failed");
        let schema = match parsed {
            Ok(schema) => schema,
            Err(_) => return,
        };
        assert_eq!(schema.areas, vec!["playback".to_string()]);
        assert_eq!(schema.len(), 1);
        assert!(schema.entry("playback.subtitle-mode").is_some());
    }

    #[test]
    fn missing_mandated_field_is_actionable_with_line() {
        let src =
            "\n[areas]\nknown = [\"playback\"]\n\n[[setting]]\nkey = \"a.b\"\ntype = \"bool\"\n";
        let diags = diags_of(src);
        let missing: Vec<&SchemaDiagnostic> = diags
            .iter()
            .filter(|d| d.rule == "E-FIELD" && d.message.contains("description"))
            .collect();
        assert_eq!(
            missing.len(),
            1,
            "expected one E-FIELD for description, got: {}",
            rendered(&diags)
        );
        assert_eq!(
            missing[0].line,
            Some(5),
            "falls back to the [[setting]] header line"
        );
        assert_eq!(missing[0].path, "setting[0].description");
    }

    #[test]
    fn unknown_field_is_rejected() {
        let src = MINIMAL.replace(
            "since = \"0.1.0\"",
            "since = \"0.1.0\"\nrestart_requierd = false",
        );
        let diags = diags_of(&src);
        assert!(
            diags.iter().any(|d| d.rule == "E-PARSE"),
            "expected E-PARSE for unknown field, got: {}",
            rendered(&diags)
        );
    }

    #[test]
    fn default_and_default_ref_both_is_d1() {
        let src = MINIMAL.replace(
            "default = \"auto\"",
            "default = \"auto\"\ndefault_ref = \"defaults.subtitle_mode\"",
        );
        let diags = diags_of(&src);
        assert!(
            diags
                .iter()
                .any(|d| d.rule == "D1" && d.message.contains("both")),
            "expected D1 both-defaults error, got: {}",
            rendered(&diags)
        );
    }

    #[test]
    fn int_without_range_is_e_domain() {
        let src = MINIMAL
            .replace("type = \"enum\"", "type = \"int\"")
            .replace("enumeration = [\"auto\", \"off\"]", "")
            .replace("default = \"auto\"", "default = 5");
        let diags = diags_of(&src);
        assert!(
            diags
                .iter()
                .any(|d| d.rule == "E-DOMAIN" && d.message.contains("numeric settings")),
            "expected E-DOMAIN range-required, got: {}",
            rendered(&diags)
        );
    }

    #[test]
    fn literal_default_out_of_range_is_e_default() {
        let src = MINIMAL
            .replace("type = \"enum\"", "type = \"int\"")
            .replace(
                "enumeration = [\"auto\", \"off\"]",
                "range = { min = 1, max = 10 }",
            )
            .replace("default = \"auto\"", "default = 42");
        let diags = diags_of(&src);
        assert!(
            diags
                .iter()
                .any(|d| d.rule == "E-DEFAULT" && d.message.contains("V-RANGE")),
            "expected E-DEFAULT out-of-range, got: {}",
            rendered(&diags)
        );
    }

    #[test]
    fn line_index_maps_setting_fields() {
        let index = LineIndex::new(MINIMAL);
        assert_eq!(index.line_of("areas.known"), Some(3));
        assert_eq!(index.line_of("setting.0"), Some(5));
        assert_eq!(index.line_of("setting.0.key"), Some(6));
        assert_eq!(index.line_of("setting.0.cli"), Some(13));
    }
}
