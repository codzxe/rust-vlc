//! Schema-first codegen: the JSON Schema, the TypeScript types and the
//! checksum (doc 05 §5.1 "Shared types" — card 1.D4).
//!
//! (The module is `codegen`, not `gen`: `gen` is a reserved keyword in
//! edition 2024 — the same constraint that named the envelope's
//! `generation` field.)
//!
//! Doc 05 §5.1: "`crates/commands` derives `schemars::JsonSchema` for
//! every wire type; `cargo xtask schema-gen` emits (a)
//! `schemas/commands.schema.json` — the single machine description of
//! the catalog; (b) the TS types the UI imports … No TS type is ever
//! hand-written." Card 1.D4 owns this generation (the xtask subcommand
//! is `codegen`, shelling out to the `catalog-gen` bin, which calls the
//! functions here — the same code the golden tests call, so the
//! committed artifacts and the tests can never disagree about what
//! "regenerated" means).
//!
//! Determinism: `schemars` 1.x emits maps as `BTreeMap`s (sorted keys)
//! and `serde_json` (without `preserve_order`, as configured here)
//! keeps that order, so the same types always produce byte-identical
//! output. The TypeScript emitter walks the parsed schema the same way
//! every time. Regeneration is idempotent; the outputs are committed
//! and `--check` fails on drift.

use std::path::Path;
use std::path::PathBuf;

use schemars::JsonSchema;

use crate::catalog::command::Command;
use crate::catalog::command::CommandResult;
use crate::catalog::event::EventKind;
use crate::envelope::CommandRequest;
use crate::envelope::CommandResponse;
use crate::envelope::Event;

/// The concrete wire shapes (§1.6 envelope instantiated with the
/// catalog payloads, §5.1 examples).
pub type WireRequest = CommandRequest<Command>;
/// Response side of [`WireRequest`].
pub type WireResponse = CommandResponse<CommandResult>;
/// Event side of the wire.
pub type WireEvent = Event<EventKind>;

/// The schema-emission root: one object that references the three wire
/// shapes, so a single `schema_for!` call produces one self-contained
/// document with every `$defs` entry the TS emitter needs (§5.1(a) —
/// "the single machine description of the catalog").
#[derive(JsonSchema)]
pub struct CatalogDocument {
    /// Host → engine request envelope.
    pub request: WireRequest,
    /// Engine → host response envelope.
    pub response: WireResponse,
    /// Engine → host event envelope.
    pub event: WireEvent,
}

/// The three generated artifacts, in memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outputs {
    /// `schemas/commands.schema.json` — pretty-printed, stable-sorted.
    pub schema: String,
    /// `ui/src/generated/commands.ts` — the UI's type surface.
    pub typescript: String,
    /// `schemas/generated/commands.checksum.txt` — catalog + output digests.
    pub checksum: String,
}

/// Default output paths relative to the workspace root (the bin and
/// the golden tests pass the root explicitly).
#[derive(Debug, Clone, Copy)]
pub struct OutputPaths<'a> {
    /// Where `schemas/commands.schema.json` lives.
    pub schema: &'a Path,
    /// Where `ui/src/generated/commands.ts` lives.
    pub typescript: &'a Path,
    /// Where `schemas/generated/commands.checksum.txt` lives.
    pub checksum: &'a Path,
}

/// Resolve the committed output paths under a workspace `root`.
///
/// The three relative locations live here so the bin, the golden
/// tests and this module cannot disagree about where the committed
/// artifacts belong. The joined `PathBuf`s are leaked to keep the
/// returned struct `Copy` (call sites are short-lived bin/test
/// processes that resolve paths once).
pub fn default_paths(root: &Path) -> OutputPaths<'static> {
    OutputPaths {
        schema: &PathBuf::from("schemas/commands.schema.json"),
        typescript: &PathBuf::from("ui/src/generated/commands.ts"),
        checksum: &PathBuf::from("schemas/generated/commands.checksum.txt"),
    }
    .under(root)
}

impl<'a> OutputPaths<'a> {
    /// Reinterpret the (relative) paths under `root`, leaking the
    /// two joined `PathBuf`s into `'static` (call sites are short-lived
    /// bin/test processes; this keeps the struct `Copy`).
    fn under(self, root: &Path) -> OutputPaths<'static> {
        let schema = root.join(self.schema);
        let typescript = root.join(self.typescript);
        let checksum = root.join(self.checksum);
        OutputPaths {
            schema: Box::leak(schema.into_boxed_path()),
            typescript: Box::leak(typescript.into_boxed_path()),
            checksum: Box::leak(checksum.into_boxed_path()),
        }
    }
}

/// Generate all three artifacts (deterministic; idempotent).
///
/// # Errors
/// Returns `Err` when schema generation or serialization fails (a bug
/// in this crate, not in the caller's inputs).
pub fn generate_outputs() -> Result<Outputs, String> {
    let schema_model = schemars::schema_for!(CatalogDocument);
    let schema = serde_json::to_string_pretty(&schema_model)
        .map_err(|err| format!("serializing the catalog schema: {err}"))?;
    let schema_value = serde_json::to_value(&schema_model)
        .map_err(|err| format!("re-parsing the catalog schema: {err}"))?;
    let typescript = emit_typescript(&schema_value);
    let checksum = checksum_text(&schema, &typescript);
    Ok(Outputs {
        schema,
        typescript,
        checksum,
    })
}

/// The checksum file body: the catalog-data digest plus digests of the
/// two other outputs (belt-and-braces: `--check` compares the files
/// byte-for-byte anyway; these lines make drift eyeballable in a diff).
pub fn checksum_text(schema: &str, typescript: &str) -> String {
    let commands = crate::catalog::spec::catalog_data().len();
    let events = crate::catalog::spec::event_catalog_data().len();
    format!(
        "# Lumen command-catalog checksums (card 1.D4, doc 05 §2-§3) — @generated, never hand-edit.\n\
         # Regenerate with `cargo xtask codegen`. fnv1a64 over the canonical catalog text\n\
         # and over the generated output bytes.\n\
         catalog-fnv1a64: {:016x}\n\
         schema-fnv1a64: {:016x}\n\
         typescript-fnv1a64: {:016x}\n\
         counts: {commands} commands / {events} events\n",
        crate::catalog::spec::catalog_checksum(),
        crate::catalog::spec::fnv1a64(schema.as_bytes()),
        crate::catalog::spec::fnv1a64(typescript.as_bytes()),
    )
}

/// Compare the committed artifacts against regeneration; one actionable
/// message per drifted (or missing) file. Empty = clean.
///
/// # Errors
/// I/O errors other than "file not found" (which counts as drift).
pub fn check_against(paths: &OutputPaths) -> Result<Vec<String>, std::io::Error> {
    let generated = generate_outputs().map_err(std::io::Error::other)?;
    let mut drift = Vec::new();
    for (label, path, expected) in [
        ("schema", paths.schema, &generated.schema),
        ("typescript", paths.typescript, &generated.typescript),
        ("checksum", paths.checksum, &generated.checksum),
    ] {
        match std::fs::read_to_string(path) {
            Ok(committed) => {
                if &committed != expected {
                    drift.push(format!(
                        "{}: {} differs from the regenerated output — run `cargo xtask codegen` and commit the refreshed file",
                        label,
                        path.display()
                    ));
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => drift.push(format!(
                "{}: {} is missing — run `cargo xtask codegen` and commit it",
                label,
                path.display()
            )),
            Err(err) => return Err(err),
        }
    }
    Ok(drift)
}

// ---------------------------------------------------------------------------
// The TypeScript emitter — a deterministic shape-mapping over the JSON
// Schema `schemars` produces (deliberately NOT an npm dependency; doc
// 05 §5.1 wants the TS surface generated, and the schema already is the
// complete type description).
// ---------------------------------------------------------------------------

/// Render `ui/src/generated/commands.ts` from the generated schema.
fn emit_typescript(schema: &serde_json::Value) -> String {
    let mut out = String::new();
    out.push_str(
        "// @generated — Lumen command-catalog TypeScript types (card 1.D4, doc 05 §2-§3, §5.1).\n",
    );
    out.push_str("// Regenerate with `cargo xtask codegen` — NEVER hand-edit; the catalog\n");
    out.push_str("// checksum gate fails on drift. Source of truth: crates/commands/src/catalog\n");
    out.push_str("// (Rust wire types) -> schemas/commands.schema.json -> this file.\n\n");
    let defs = schema
        .get("$defs")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut names: Vec<&String> = defs.keys().collect();
    names.sort_unstable();
    for name in names {
        out.push_str("export type ");
        out.push_str(name);
        out.push_str(" = ");
        out.push_str(&ts_type(&defs[name], &defs));
        out.push_str(";\n\n");
    }
    out.push_str("// The schema root: one object referencing the three wire envelopes.\n");
    out.push_str("export type CatalogDocument = ");
    out.push_str(&ts_type(schema, &defs));
    out.push_str(";\n");
    out
}

/// Map one JSON-Schema (sub)schema to a TS type expression.
fn ts_type(
    schema: &serde_json::Value,
    defs: &serde_json::Map<String, serde_json::Value>,
) -> String {
    if let Some(reference) = schema.get("$ref").and_then(serde_json::Value::as_str) {
        let name = reference.rsplit('/').next().unwrap_or(reference);
        return name.to_owned();
    }
    if let Some(value) = schema.get("const") {
        return ts_literal(value);
    }
    if let Some(values) = schema.get("enum").and_then(serde_json::Value::as_array) {
        let parts: Vec<String> = values.iter().map(ts_literal).collect();
        if parts.len() == 1 {
            return parts
                .into_iter()
                .next()
                .unwrap_or_else(|| "never".to_owned());
        }
        return format!("({})", parts.join(" | "));
    }
    if let Some(variants) = schema.get("oneOf").and_then(serde_json::Value::as_array) {
        let parts: Vec<String> = variants
            .iter()
            .map(|variant| ts_type(variant, defs))
            .collect();
        return format!("({})", parts.join(" | "));
    }
    if let Some(variants) = schema.get("anyOf").and_then(serde_json::Value::as_array) {
        let parts: Vec<String> = variants
            .iter()
            .map(|variant| ts_type(variant, defs))
            .collect();
        return format!("({})", parts.join(" | "));
    }
    if let Some(parts) = schema.get("allOf").and_then(serde_json::Value::as_array) {
        let joined: Vec<String> = parts.iter().map(|part| ts_type(part, defs)).collect();
        return joined.join(" & ");
    }
    match schema.get("type").and_then(serde_json::Value::as_str) {
        Some("object") => ts_object(schema, defs),
        Some("array") => {
            let items = schema
                .get("items")
                .map(|items| ts_type(items, defs))
                .unwrap_or_else(|| "unknown".to_owned());
            format!("{items}[]")
        }
        Some("string") => "string".to_owned(),
        Some("integer") | Some("number") => "number".to_owned(),
        Some("boolean") => "boolean".to_owned(),
        Some("null") => "null".to_owned(),
        Some(_) => "unknown".to_owned(),
        None => {
            // A type given as a list, e.g. Option<String> -> ["string","null"].
            if let Some(types) = schema.get("type").and_then(serde_json::Value::as_array) {
                let parts: Vec<String> = types
                    .iter()
                    .filter_map(|t| t.as_str())
                    .map(|t| match t {
                        "string" => "string".to_owned(),
                        "integer" | "number" => "number".to_owned(),
                        "boolean" => "boolean".to_owned(),
                        "null" => "null".to_owned(),
                        _ => "unknown".to_owned(),
                    })
                    .collect();
                return format!("({})", parts.join(" | "));
            }
            "unknown".to_owned()
        }
    }
}

/// Object schemas: `Record<string, T>` for maps, inline object literals
/// otherwise, `& (…)` when a flattened enum merged a `oneOf` beside the
/// plain properties.
fn ts_object(
    schema: &serde_json::Value,
    defs: &serde_json::Map<String, serde_json::Value>,
) -> String {
    let properties = schema
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    if properties.is_empty() {
        if let Some(additional) = schema.get("additionalProperties") {
            return format!("Record<string, {}>", ts_type(additional, defs));
        }
        return "Record<string, unknown>".to_owned();
    }
    let required: Vec<&str> = schema
        .get("required")
        .and_then(serde_json::Value::as_array)
        .map(|list| list.iter().filter_map(serde_json::Value::as_str).collect())
        .unwrap_or_default();
    let mut fields: Vec<String> = Vec::new();
    for (name, value) in &properties {
        let optional = if required.contains(&name.as_str()) {
            ""
        } else {
            "?"
        };
        let key = if is_ts_identifier(name) {
            name.clone()
        } else {
            format!("{name:?}")
        };
        fields.push(format!("{key}{optional}: {}", ts_type(value, defs)));
    }
    let object = format!("{{ {} }}", fields.join("; "));
    if let Some(variants) = schema.get("oneOf") {
        let union = ts_type(
            &serde_json::Value::Object(
                [("oneOf".to_owned(), variants.clone())]
                    .into_iter()
                    .collect(),
            ),
            defs,
        );
        return format!("({object} & {union})");
    }
    object
}

/// A JSON literal as a TS literal (strings quoted and escaped, numbers
/// verbatim, booleans/null verbatim).
fn ts_literal(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => format!("{text:?}"),
        serde_json::Value::Null => "null".to_owned(),
        other => other.to_string(),
    }
}

/// Whether a property name is a bare TS identifier.
fn is_ts_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_alphabetic() || first == '_' || first == '$' => {}
        _ => return false,
    }
    chars.all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_deterministic_and_idempotent() {
        let first = match generate_outputs() {
            Ok(outputs) => outputs,
            Err(err) => unreachable!("generation failed: {err}"),
        };
        let second = match generate_outputs() {
            Ok(outputs) => outputs,
            Err(err) => unreachable!("second generation failed: {err}"),
        };
        assert_eq!(first, second, "regeneration must be byte-identical");
    }

    #[test]
    fn schema_document_references_all_three_envelopes() {
        let outputs = match generate_outputs() {
            Ok(outputs) => outputs,
            Err(err) => unreachable!("generation failed: {err}"),
        };
        for needle in [
            "\"CommandRequest\"",
            "\"CommandResponse\"",
            "\"Event\"",
            "\"session.open\"",
            "\"playback.toggle-play-pause\"",
            "\"system.list-commands\"",
            "\"timing.seek-completed\"",
            "\"media.info-ready\"",
        ] {
            assert!(outputs.schema.contains(needle), "schema missing {needle}");
        }
    }

    #[test]
    fn typescript_exports_the_wire_surface() {
        let outputs = match generate_outputs() {
            Ok(outputs) => outputs,
            Err(err) => unreachable!("generation failed: {err}"),
        };
        for needle in [
            "export type CommandRequest",
            "export type CommandResponse",
            "export type Event",
            "\"playback.seek\"",
            "\"timing.seek-completed\"",
            "export type SeekTarget",
            "export type TrackInfo",
            "export type CatalogDocument",
        ] {
            assert!(
                outputs.typescript.contains(needle),
                "typescript missing {needle}"
            );
        }
        assert!(
            !ts_leaks_unknown_type(&outputs.typescript),
            "unmapped schema shape leaked into the TS output"
        );
    }

    /// Whether the TS output uses the `unknown` TYPE (as opposed to the
    /// quoted `Duration::Unknown` variant literal `"unknown"`): strip
    /// double-quoted string literals, then no bare `unknown` may remain.
    fn ts_leaks_unknown_type(typescript: &str) -> bool {
        let mut outside_quotes = String::new();
        let mut in_quotes = false;
        for ch in typescript.chars() {
            match ch {
                '"' => in_quotes = !in_quotes,
                _ if !in_quotes => outside_quotes.push(ch),
                _ => {}
            }
        }
        outside_quotes.contains("unknown")
    }

    #[test]
    fn checksum_text_carries_the_counts_and_digests() {
        let outputs = match generate_outputs() {
            Ok(outputs) => outputs,
            Err(err) => unreachable!("generation failed: {err}"),
        };
        assert!(
            outputs.checksum.contains("catalog-fnv1a64: "),
            "checksum must lead with the catalog digest"
        );
        assert!(outputs.checksum.contains("61 commands / 23 events"));
    }

    /// The drift negative control at the file level: a doctored
    /// schema/typescript/checksum file (in a temp dir) must be reported,
    /// and a clean copy must not.
    #[test]
    fn check_reports_doctored_artifacts() {
        let outputs = match generate_outputs() {
            Ok(outputs) => outputs,
            Err(err) => unreachable!("generation failed: {err}"),
        };
        let dir = std::env::temp_dir().join(format!("lumen-1d4-check-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        if let Err(err) = std::fs::create_dir_all(&dir) {
            unreachable!("tempdir failed: {err}");
        }
        let paths = OutputPaths {
            schema: &dir.join("commands.schema.json"),
            typescript: &dir.join("commands.ts"),
            checksum: &dir.join("commands.checksum.txt"),
        };
        for (path, content) in [
            (paths.schema, &outputs.schema),
            (paths.typescript, &outputs.typescript),
            (paths.checksum, &outputs.checksum),
        ] {
            if let Err(err) = std::fs::write(path, content) {
                unreachable!("writing {} failed: {err}", path.display());
            }
        }
        match check_against(&paths) {
            Ok(drift) => assert!(drift.is_empty(), "clean copy reported drift: {drift:?}"),
            Err(err) => unreachable!("clean check failed: {err}"),
        }
        // Seed the drift: one doctored schema byte.
        let doctored = outputs.schema.replace("session.open", "session.opens");
        assert_ne!(doctored, outputs.schema);
        if let Err(err) = std::fs::write(paths.schema, doctored) {
            unreachable!("rewriting schema failed: {err}");
        }
        match check_against(&paths) {
            Ok(drift) => {
                assert!(!drift.is_empty(), "doctored schema not reported");
                assert!(
                    drift[0].contains("schema"),
                    "wrong drift message: {}",
                    drift[0]
                );
            }
            Err(err) => unreachable!("doctored check failed: {err}"),
        }
        // A missing file is drift too.
        if let Err(err) = std::fs::remove_file(paths.typescript) {
            unreachable!("removing typescript failed: {err}");
        }
        match check_against(&paths) {
            Ok(drift) => {
                assert_eq!(
                    drift.len(),
                    2,
                    "expected schema+typescript drift: {drift:?}"
                );
                assert!(drift[1].contains("missing"), "wrong message: {}", drift[1]);
            }
            Err(err) => unreachable!("missing-file check failed: {err}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ts_identifier_check_is_conservative() {
        assert!(is_ts_identifier("requestId"));
        assert!(is_ts_identifier("seekKind"));
        assert!(!is_ts_identifier("")); // empty is not an identifier
    }
}
