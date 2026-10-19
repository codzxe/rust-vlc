//! Generators G1–G3 (doc 07 §5.3): TS types, CLI `--help` text and the
//! docs page. All three are pure functions of the parsed schema — running
//! twice is byte-identical (idempotence is the card's acceptance criterion
//! and CI's drift check via `--check`).
//!
//! G1 writes the TYPE surface only (`settings.d.ts` is an ambient
//! declaration file — no values): the value-bearing table for the React
//! screens lands with the settings framework (card 4.E1). G4 is not a file
//! at all: runtime validation is this crate's [`crate::runtime`] module.

use std::io::Write;

use crate::model::DefaultValue;
use crate::model::Domain;
use crate::model::SettingDef;
use crate::model::SettingType;
use crate::model::SettingsSchema;
use crate::model::fmt_bound;

/// Which artifact to generate (doc 01 §2.3 sketch; `Validator` from the
/// sketch is the crate's own `validate` module — G4 is a library, not a
/// generated file).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenTarget {
    /// G1 — `schemas/generated/settings.d.ts` (TS type surface).
    TsTypes,
    /// G2 — `schemas/generated/settings-help.txt` (CLI `--help` golden).
    CliHelp,
    /// G3 — `schemas/generated/settings-docs.md` (settings reference).
    Docs,
}

/// A generator failure (I/O only — the emitters themselves are infallible
/// once the schema parses and validates).
#[derive(Debug, thiserror::Error)]
pub enum GenError {
    /// Writing the artifact failed.
    #[error("writing generated artifact: {0}")]
    Io(#[from] std::io::Error),
}

/// Generate one artifact into `out`.
pub fn generate(
    schema: &SettingsSchema,
    target: GenTarget,
    out: &mut impl Write,
) -> Result<(), GenError> {
    let text = generate_str(schema, target);
    out.write_all(text.as_bytes())?;
    Ok(())
}

/// Generate one artifact to a string (deterministic: same schema in, same
/// bytes out).
pub fn generate_str(schema: &SettingsSchema, target: GenTarget) -> String {
    match target {
        GenTarget::TsTypes => emit_ts(schema),
        GenTarget::CliHelp => emit_help(schema),
        GenTarget::Docs => emit_docs(schema),
    }
}

// --- shared helpers ---------------------------------------------------------

/// The CLI value placeholder for an entry (G2): `<on|off>`, `<a|b|c>`,
/// `<min..max>` (+ ` ms` for durations), `<string>`, `<path>`, `<url>`,
/// `<list>`.
fn value_placeholder(def: &SettingDef) -> String {
    match def.ty {
        SettingType::Bool => "<on|off>".to_owned(),
        SettingType::Enum => format!("<{}>", enum_members(def).join("|")),
        SettingType::Int | SettingType::Float | SettingType::Percent => match &def.domain {
            Domain::Range(r) => format!("<{}..{}>", fmt_bound(r.min), fmt_bound(r.max)),
            Domain::Unconstrained | Domain::Enumeration(_) => "<number>".to_owned(),
        },
        SettingType::DurationMs => match &def.domain {
            Domain::Range(r) => format!("<{}..{} ms>", fmt_bound(r.min), fmt_bound(r.max)),
            Domain::Unconstrained | Domain::Enumeration(_) => "<ms>".to_owned(),
        },
        SettingType::String => "<string>".to_owned(),
        SettingType::Path => "<path>".to_owned(),
        SettingType::Url => "<url>".to_owned(),
        SettingType::List(_) => "<list>".to_owned(),
    }
}

fn enum_members(def: &SettingDef) -> &[String] {
    match &def.domain {
        Domain::Enumeration(members) => members.as_slice(),
        Domain::Range(_) | Domain::Unconstrained => &[],
    }
}

/// The default cell/line: literal TOML value, or the product.toml path.
fn default_display(def: &SettingDef) -> String {
    match &def.default {
        DefaultValue::Literal(value) => format!("{value}"),
        DefaultValue::Ref(path) => format!("-> product.toml {path}"),
    }
}

/// The docs Domain column: `4..512`, `4..512 ms`, member list, or `—`.
fn domain_display(def: &SettingDef) -> String {
    match (&def.ty, &def.domain) {
        (_, Domain::Range(r)) => {
            if matches!(def.ty, SettingType::DurationMs) {
                format!("{}..{} ms", fmt_bound(r.min), fmt_bound(r.max))
            } else {
                format!("{}..{}", fmt_bound(r.min), fmt_bound(r.max))
            }
        }
        (SettingType::Enum | SettingType::List(_), Domain::Enumeration(members)) => {
            members.join(", ")
        }
        _ => "—".to_owned(),
    }
}

fn marker_list(def: &SettingDef) -> Vec<String> {
    let mut markers = Vec::new();
    if !def.feature_gate.is_empty() {
        markers.push(format!("[gate: {}]", def.feature_gate));
    }
    if def.advanced {
        markers.push("[advanced]".to_owned());
    }
    if def.restart_required {
        markers.push("[restart required]".to_owned());
    }
    if !def.deprecated_in.is_empty() {
        markers.push(format!("[deprecated in {}]", def.deprecated_in));
    }
    markers
}

/// Whether every entry on this screen is engine-internal (`ui = false`).
fn screen_is_internal(schema: &SettingsSchema, screen: &str) -> bool {
    let entries = schema.screen_entries(screen);
    !entries.is_empty() && entries.iter().all(|d| !d.placement.ui)
}

// --- G1: settings.d.ts --------------------------------------------------------

fn emit_ts(schema: &SettingsSchema) -> String {
    let mut out = String::new();
    out.push_str(
        "// Generated by `cargo xtask schema-gen` from schemas/settings.toml — DO NOT EDIT.\n",
    );
    out.push_str("// Generator G1 (doc 07 §5.3): the typed settings surface for the UI.\n");
    out.push_str("// Regenerate: `cargo xtask schema-gen` (idempotent; CI diffs with --check).\n");
    out.push_str("//\n");
    out.push_str(&format!(
        "// {} entries across {} screen(s) ({} known areas). The value-bearing table\n",
        schema.len(),
        schema.screens_in_use().len(),
        schema.areas.len()
    ));
    out.push_str("// (ui/src/generated/settings-schema.ts) lands with the settings framework\n");
    out.push_str("// (card 4.E1); this ambient file fixes the type surface only.\n\n");

    out.push_str("export type SettingType =\n");
    let tags = [
        "bool",
        "int",
        "float",
        "string",
        "enum",
        "path",
        "url",
        "duration-ms",
        "percent",
        "list<string>",
        "list<path>",
        "list<url>",
        "list<enum>",
    ];
    for tag in tags {
        out.push_str(&format!("  | '{tag}'\n"));
    }
    out.push('\n');

    out.push_str(
        "/** UI control hint derived from the type (doc 07 §5.3 G1 control selection). */\n",
    );
    out.push_str("export type SettingControl =\n");
    for control in [
        "toggle",
        "select",
        "slider",
        "input",
        "list",
        "path-picker",
        "duration",
    ] {
        out.push_str(&format!("  | '{control}'\n"));
    }
    out.push('\n');

    out.push_str("/** Screen ids from schemas/settings.toml `[areas]` (doc 07 §5.4). */\n");
    out.push_str("export type SettingScreen =\n");
    for screen in schema.screens_in_use() {
        out.push_str(&format!("  | '{screen}'\n"));
    }
    out.push('\n');

    out.push_str("/** Canonical setting keys — globally unique (rule D2). */\n");
    out.push_str("export type SettingKey =\n");
    let mut keys: Vec<&str> = schema.settings.iter().map(|d| d.key.as_str()).collect();
    keys.sort_unstable();
    for key in keys {
        out.push_str(&format!("  | '{key}'\n"));
    }
    out.push('\n');

    out.push_str("/** The keys of one screen: `Extract<SettingKey, `${S}.${string}`>`. */\n");
    out.push_str(
        "export type SettingKeysOfScreen<S extends SettingScreen> = Extract<SettingKey, `${S}.${string}`>;\n\n",
    );

    out.push_str("/** One settings entry — the doc 07 §5.2 field set. */\n");
    out.push_str("export interface SettingEntry {\n");
    out.push_str("  /** Canonical dotted kebab-case id. */\n");
    out.push_str("  key: SettingKey;\n");
    out.push_str("  type: SettingType;\n");
    out.push_str("  /** UI control hint (G1 control selection). */\n");
    out.push_str("  control: SettingControl;\n");
    out.push_str("  /** Numeric domain `[min, max]`; `null` when unconstrained. */\n");
    out.push_str("  range: readonly [number, number] | null;\n");
    out.push_str("  /** Enum members; `null` for non-enum types. */\n");
    out.push_str("  enumeration: readonly string[] | null;\n");
    out.push_str(
        "  /** Literal default; `null` when the default is referenced from product.toml. */\n",
    );
    out.push_str("  default: boolean | number | string | readonly string[] | null;\n");
    out.push_str(
        "  /** product.toml path referenced by `default_ref` (rule D1); `null` for literals. */\n",
    );
    out.push_str("  defaultRef: string | null;\n");
    out.push_str("  /** Description msgid (localization key — never literal prose). */\n");
    out.push_str("  description: string;\n");
    out.push_str("  restartRequired: boolean;\n");
    out.push_str("  /** UI placement (doc 07 §5.2). */\n");
    out.push_str("  screen: SettingScreen;\n");
    out.push_str("  section: string;\n");
    out.push_str("  order: number;\n");
    out.push_str(
        "  /** `false` for engine-internal entries (never rendered; CLI + docs only). */\n",
    );
    out.push_str("  ui: boolean;\n");
    out.push_str("  /** Long flag — unique and disjoint from reserved CLI names (rule D3). */\n");
    out.push_str("  cliFlag: string;\n");
    out.push_str("  cliShort: string;\n");
    out.push_str("  /** Feature gate id; `null` = ungated (doc 05 §6.4). */\n");
    out.push_str("  featureGate: string | null;\n");
    out.push_str("  /** First product version shipping this entry. */\n");
    out.push_str("  since: string;\n");
    out.push_str("  /** `user` (persisted, layer 4) or `session` (layer 6) — doc 07 §4.1. */\n");
    out.push_str("  scope: 'user' | 'session';\n");
    out.push_str("  /** Array merge policy (doc 07 §4.4 rule 3). */\n");
    out.push_str("  arrayPolicy: 'replace' | 'append-unique';\n");
    out.push_str("  /** VLC-style \"show advanced\" folding. */\n");
    out.push_str("  advanced: boolean;\n");
    out.push_str("  /** Version that deprecated this entry; `null` = active. */\n");
    out.push_str("  deprecatedIn: string | null;\n");
    out.push_str("}\n\n");

    out.push_str(
        "/** Entries per screen in `(section, order)` order (doc 07 §5.4 navigation). */\n",
    );
    out.push_str("export interface SettingsByScreen {\n");
    for screen in schema.screens_in_use() {
        out.push_str(&format!("  {screen}: readonly SettingEntry[];\n"));
    }
    out.push_str("}\n");
    out
}

// --- G2: settings-help.txt ----------------------------------------------------

fn emit_help(schema: &SettingsSchema) -> String {
    let mut out = String::new();
    out.push_str("Lumen settings-derived command-line options\n");
    out.push_str("Generated by `cargo xtask schema-gen` from schemas/settings.toml — DO NOT EDIT (doc 07 §5.3 G2).\n\n");
    out.push_str("Usage:\n");
    out.push_str("  lumen [OPTIONS] [COMMAND]\n\n");
    out.push_str("  Settings surface as generated long flags (--<area>-<key> <value>).\n");
    out.push_str("  `lumen --help` lists areas with counts; `lumen --help <area>` lists that\n");
    out.push_str("  area's flags (the VLC --longhelp pattern, generated).\n");
    out.push_str("  Reserved (non-settings) flags: -h/--help, -v/--version, --json,\n");
    out.push_str("  --control-stdio, --generate-manifest.\n\n");
    out.push_str("Areas:\n");
    for screen in schema.screens_in_use() {
        let entries = schema.screen_entries(&screen);
        let count = entries.len();
        let plural = if count == 1 { "option" } else { "options" };
        let annotation = if screen_is_internal(schema, &screen) {
            "  (engine-internal: ui = false)"
        } else {
            ""
        };
        out.push_str(&format!("  {screen:<14} {count} {plural}{annotation}\n"));
    }
    out.push('\n');
    for screen in schema.screens_in_use() {
        let annotation = if screen_is_internal(schema, &screen) {
            " (engine-internal: ui = false)"
        } else {
            ""
        };
        out.push_str(&format!("{screen}{annotation}\n"));
        let mut last_section = String::new();
        for def in schema.screen_entries(&screen) {
            if def.placement.section != last_section {
                out.push_str(&format!("  {}\n", def.placement.section));
                last_section = def.placement.section.clone();
            }
            let mut line = format!("  {} {}", def.cli.flag, value_placeholder(def));
            let markers = marker_list(def).join(" ");
            if !markers.is_empty() {
                line.push_str("  ");
                line.push_str(&markers);
            }
            out.push_str(&line);
            out.push('\n');
            out.push_str(&format!("      {}\n", def.description));
            out.push_str(&format!("      default: {}\n", default_display(def)));
        }
        out.push('\n');
    }
    out
}

// --- G3: settings-docs.md ------------------------------------------------------

fn emit_docs(schema: &SettingsSchema) -> String {
    let mut out = String::new();
    out.push_str("# Lumen settings reference\n\n");
    out.push_str(
        "Generated by `cargo xtask schema-gen` from `schemas/settings.toml` — DO NOT EDIT\n",
    );
    out.push_str(
        "(doc 07 §5.3 G3). Regenerate: `cargo xtask schema-gen`; PR diffs show doc drift\n",
    );
    out.push_str("for free.\n\n");
    out.push_str(&format!(
        "{} entries across {} screen(s) of {} known areas. Defaults marked `->` are\n",
        schema.len(),
        schema.screens_in_use().len(),
        schema.areas.len()
    ));
    out.push_str("referenced from `product.toml` (rule D1 — product-level defaults are stated\n");
    out.push_str("once, there). Engine-internal entries are never rendered in the UI (CLI,\n");
    out.push_str("docs and validation only).\n\n");
    for screen in schema.screens_in_use() {
        let annotation = if screen_is_internal(schema, &screen) {
            " (engine-internal — ui = false)"
        } else {
            ""
        };
        out.push_str(&format!("## {screen}{annotation}\n\n"));
        let mut last_section = String::new();
        for def in schema.screen_entries(&screen) {
            if def.placement.section != last_section {
                if !last_section.is_empty() {
                    out.push('\n');
                }
                out.push_str(&format!("### {screen} — {}\n\n", def.placement.section));
                out.push_str(
                    "| Key | Type | Default | Domain | Since | Restart | CLI flag | Gate |\n",
                );
                out.push_str("|---|---|---|---|---|---|---|---|\n");
                last_section = def.placement.section.clone();
            }
            let mut key_cell = format!("`{}`", def.key);
            if def.advanced {
                key_cell.push_str(" (advanced)");
            }
            if !def.deprecated_in.is_empty() {
                key_cell.push_str(&format!(" (deprecated in {})", def.deprecated_in));
            }
            let restart = if def.restart_required { "yes" } else { "—" };
            let gate = if def.feature_gate.is_empty() {
                "—".to_owned()
            } else {
                format!("`{}`", def.feature_gate)
            };
            out.push_str(&format!(
                "| {} | `{}` | `{}` | {} | {} | {} | `{}` | {} |\n",
                key_cell,
                def.ty.as_str(),
                default_display(def),
                domain_display(def),
                def.since,
                restart,
                def.cli.flag,
                gate
            ));
        }
        out.push('\n');
    }
    out.push_str("## Single-definition rules\n\n");
    out.push_str(
        "The schema enforces D1–D6 (doc 07 §5.2): defaults stated once (D1), unique keys\n",
    );
    out.push_str(
        "(D2), unique CLI flags disjoint from reserved names (D3), 1:1 description msgids\n",
    );
    out.push_str(
        "(D4), unique placement orders per screen section with known screens (D5), and one\n",
    );
    out.push_str("entry per product-decision slot (D6).\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GenTarget;

    fn fixture_schema() -> SettingsSchema {
        let src = "\
[areas]
known = [\"playback\", \"engine\"]

[[setting]]
key = \"playback.subtitle-mode\"
type = \"enum\"
enumeration = [\"auto\", \"off\"]
default_ref = \"playback.subtitle_mode\"
description = \"settings.playback.subtitle-mode\"
restart_required = false
placement = { screen = \"playback\", section = \"tracks\", order = 10, ui = true }
cli = { flag = \"--playback-subtitle-mode\", short = \"\" }
feature_gate = \"\"
since = \"0.1.0\"

[[setting]]
key = \"engine.tick-interval-ms\"
type = \"duration-ms\"
range = { min = 50, max = 2000 }
default = 250
description = \"settings.engine.tick-interval-ms\"
restart_required = true
placement = { screen = \"engine\", section = \"internal\", order = 10, ui = false }
cli = { flag = \"--engine-tick-interval-ms\", short = \"\" }
feature_gate = \"\"
since = \"0.1.0\"
advanced = true
";
        match crate::parse::parse_str(src) {
            Ok(schema) => schema,
            Err(diags) => {
                assert_eq!(
                    "",
                    crate::diagnostic::render_all(&diags),
                    "fixture must parse"
                );
                SettingsSchema {
                    areas: Vec::new(),
                    settings: Vec::new(),
                }
            }
        }
    }

    #[test]
    fn generation_is_deterministic() {
        let schema = fixture_schema();
        for target in [GenTarget::TsTypes, GenTarget::CliHelp, GenTarget::Docs] {
            assert_eq!(
                generate_str(&schema, target),
                generate_str(&schema, target),
                "{target:?} must be byte-identical across runs"
            );
        }
    }

    #[test]
    fn ts_types_list_every_key_and_screen() {
        let ts = generate_str(&fixture_schema(), GenTarget::TsTypes);
        assert!(ts.contains("| 'playback.subtitle-mode'"), "{ts}");
        assert!(ts.contains("| 'engine.tick-interval-ms'"), "{ts}");
        assert!(
            ts.contains("  | 'engine'\n  | 'playback'\n"),
            "screens sorted: {ts}"
        );
        assert!(ts.contains("engine: readonly SettingEntry[];"), "{ts}");
        assert!(ts.contains("playback: readonly SettingEntry[];"), "{ts}");
        assert!(ts.contains("export interface SettingEntry {"), "{ts}");
    }

    #[test]
    fn help_lists_areas_flags_and_defaults() {
        let help = generate_str(&fixture_schema(), GenTarget::CliHelp);
        assert!(help.contains("playback"), "{help}");
        assert!(help.contains("1 option"), "{help}");
        assert!(help.contains("(engine-internal: ui = false)"), "{help}");
        assert!(
            help.contains("--playback-subtitle-mode <auto|off>"),
            "{help}"
        );
        assert!(
            help.contains("--engine-tick-interval-ms <50..2000 ms>  [advanced] [restart required]"),
            "{help}"
        );
        assert!(
            help.contains("default: -> product.toml playback.subtitle_mode"),
            "{help}"
        );
        assert!(help.contains("default: 250"), "{help}");
        assert!(help.contains("settings.playback.subtitle-mode"), "{help}");
    }

    #[test]
    fn docs_tables_carry_every_column() {
        let docs = generate_str(&fixture_schema(), GenTarget::Docs);
        assert!(
            docs.contains("## engine (engine-internal — ui = false)"),
            "{docs}"
        );
        assert!(
            docs.contains("| `playback.subtitle-mode` | `enum` | `-> product.toml playback.subtitle_mode` | auto, off | 0.1.0 | — | `--playback-subtitle-mode` | — |"),
            "{docs}"
        );
        assert!(
            docs.contains("| `engine.tick-interval-ms` (advanced) | `duration-ms` | `250` | 50..2000 ms | 0.1.0 | yes | `--engine-tick-interval-ms` | — |"),
            "{docs}"
        );
        assert!(docs.contains("## Single-definition rules"), "{docs}");
    }

    #[test]
    fn generate_into_writer_matches_string_form() {
        let schema = fixture_schema();
        let mut buf = Vec::new();
        let result = generate(&schema, GenTarget::Docs, &mut buf);
        assert!(result.is_ok());
        assert_eq!(
            String::from_utf8(buf).ok(),
            Some(generate_str(&schema, GenTarget::Docs))
        );
    }
}
