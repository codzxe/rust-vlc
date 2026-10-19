//! Msgid reference extraction — the input side of audit rule 4 (doc 07
//! §6.4): every msgid referenced from the declarative config files must
//! resolve to a reference-catalog key.
//!
//! The reference sources named by doc 07 §6.4 are `product.toml`,
//! `schemas/settings.toml`, `layouts/*`, `keymaps/*`, `profiles/*`,
//! `themes/*` (and doc 05's menu/UI-action tables, which materialize as
//! `layouts/menus.toml`). They reference message keys through a small set
//! of field names:
//!
//! * `description_key` — product.toml identity (doc 07 §3.2)
//! * `title_idle_key` — product.toml window (doc 07 §3.2)
//! * `description` — settings-schema entries (doc 07 §5.2: "msgid; never
//!   literal text")
//! * `label_key` — layouts: menus, settings areas, controls (doc 07 §7.2)
//!
//! Extraction is conservative: only values that syntactically ARE message
//! keys (doc 07 §6.1 grammar) are treated as msgid references, so prose
//! `description` fields in sibling configs (e.g. theme notes) do not
//! produce false references.

use std::fmt;
use std::path::Path;

use crate::key::MessageKey;
use crate::line_col;

/// Field names whose string values are msgid references (doc 07 §3.2,
/// §5.2, §7.2). Values that do not match the key grammar are ignored.
pub const MSGID_FIELDS: &[&str] = &[
    "label_key",
    "description_key",
    "title_idle_key",
    "description",
];

/// A message-key reference found in a config file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsgidRef {
    /// File the reference came from (as passed to the extractor).
    pub file: String,
    /// The field carrying the reference (e.g. `label_key`).
    pub field: String,
    /// The referenced key.
    pub key: MessageKey,
}

impl fmt::Display for MsgidRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{} -> {}", self.file, self.field, self.key)
    }
}

/// A config source that could not be scanned for references.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RefError {
    /// The file is not valid TOML.
    #[error("parse error: {message} (line {line}, column {column})")]
    Parse {
        message: String,
        line: usize,
        column: usize,
    },
    /// The file could not be read.
    #[error("{context}: {detail}")]
    Io { context: String, detail: String },
}

/// Map a TOML parse error to a positioned [`RefError::Parse`] (line and
/// column from the error span; 1:1 when the parser gives none).
fn refs_parse_error(src: &str, error: &toml::de::Error) -> RefError {
    let message = error.message().to_string();
    let (line, column) = error
        .span()
        .map(|span| line_col(src, span.start))
        .unwrap_or((1, 1));
    RefError::Parse {
        message,
        line,
        column,
    }
}

/// Extract msgid references from one config file's contents.
pub fn extract_msgid_refs(file: &str, src: &str) -> Result<Vec<MsgidRef>, RefError> {
    let table: toml::Table = src.parse().map_err(|error| refs_parse_error(src, &error))?;
    let mut refs = Vec::new();
    walk_table(file, &table, &mut refs);
    Ok(refs)
}

/// Read `path` (if it exists) and extract its msgid references. A missing
/// file is `Ok(None)` — Phase 1 ships before most config files exist, and
/// rule 4 trivially passes for absent sources (documented honesty, not a
/// skip).
pub fn extract_msgid_refs_from_file(path: &Path) -> Result<Option<Vec<MsgidRef>>, RefError> {
    let src = match std::fs::read_to_string(path) {
        Ok(src) => src,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(RefError::Io {
                context: format!("reading {}", path.display()),
                detail: source.to_string(),
            });
        }
    };
    let file = path.display().to_string();
    Ok(Some(extract_msgid_refs(&file, &src)?))
}

fn walk_table(file: &str, table: &toml::Table, refs: &mut Vec<MsgidRef>) {
    for (field, value) in table {
        walk_value(file, field, value, refs);
    }
}

fn walk_value(file: &str, field: &str, value: &toml::Value, refs: &mut Vec<MsgidRef>) {
    match value {
        toml::Value::Table(table) => walk_table(file, table, refs),
        toml::Value::Array(items) => {
            for item in items {
                // Arrays of tables ([[setting]], …) keep the field context.
                walk_value(file, field, item, refs);
            }
        }
        toml::Value::String(text) => {
            if MSGID_FIELDS.contains(&field)
                && let Ok(key) = MessageKey::new(text)
            {
                refs.push(MsgidRef {
                    file: file.to_string(),
                    field: field.to_string(),
                    key,
                });
            }
        }
        toml::Value::Integer(_)
        | toml::Value::Float(_)
        | toml::Value::Boolean(_)
        | toml::Value::Datetime(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{MSGID_FIELDS, RefError, extract_msgid_refs};

    #[test]
    fn extracts_known_msgid_fields() -> Result<(), RefError> {
        let src = "\
[identity]
name = \"Lumen\"
description_key = \"product.description\"

[window]
title_idle_key = \"window.title.idle\"

[[setting]]
key = \"playback.seek-step-short\"
description = \"settings.playback.seek-step-short\"

[[menu.leaf]]
label_key = \"menu.media.open-file\"
command = \"media.open-file\"
";
        let refs = extract_msgid_refs("product-like.toml", src)?;
        // `toml::Table` iterates in sorted key order (BTreeMap; the
        // `preserve_order` feature would add a dependency — a Lead
        // decision), so refs appear as identity → menu → setting → window
        // regardless of document order. Order is not part of the contract;
        // rule 4 only needs the set.
        let shown: Vec<String> = refs.iter().map(|r| r.to_string()).collect();
        assert_eq!(
            shown,
            [
                "product-like.toml:description_key -> product.description",
                "product-like.toml:label_key -> menu.media.open-file",
                "product-like.toml:description -> settings.playback.seek-step-short",
                "product-like.toml:title_idle_key -> window.title.idle",
            ]
        );
        Ok(())
    }

    #[test]
    fn ignores_non_msgid_values_and_prose() -> Result<(), RefError> {
        let src = "\
name = \"Lumen\"
description = \"A media player.\"      # prose, not a msgid
technical = \"player.controls.play\"   # not a msgid field
flag = true
nested = { label_key = \"not a key\" } # field matches, value doesn't
";
        let refs = extract_msgid_refs("x.toml", src)?;
        assert!(refs.is_empty(), "refs: {refs:?}");
        Ok(())
    }

    #[test]
    fn parse_failures_are_typed_errors() {
        let src = "not [valid toml\n";
        let error = extract_msgid_refs("bad.toml", src).err();
        assert!(
            matches!(&error, Some(RefError::Parse { .. })),
            "expected parse error, got {error:?}"
        );
    }

    #[test]
    fn msgid_field_table_matches_doc07() {
        assert_eq!(
            MSGID_FIELDS,
            &[
                "label_key",
                "description_key",
                "title_idle_key",
                "description"
            ]
        );
    }
}
