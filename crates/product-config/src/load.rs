//! Load pipeline (doc 07 §3.4): parse → structural → typed.
//!
//! Stage 1 parses the TOML (syntax failures carry the `toml` crate's span,
//! converted to a 1-based line). Stage 2 runs the data-driven schema pass and
//! reports *every* violation. Stage 3 deserializes the typed model — after a
//! clean stage 2 this cannot fail for schema-shaped input; if it does, the
//! residual error is reported honestly instead of being swallowed.
//! Semantic-class rules (fs, cross-file) are a separate pass ([`crate::
//! semantic`]) because they need inputs JSON Schema cannot express.

use std::path::Path;

use crate::diagnostic::{Diagnostic, Severity, Stage};
use crate::lines::LineIndex;
use crate::model::ProductConfig;
use crate::partial::PartialProduct;
use crate::schema::{RequiredMode, Schema};

/// Line (1-based) of a byte offset in `src`, if the offset is in range.
fn line_of_offset(src: &str, offset: usize) -> Option<u32> {
    if offset > src.len() {
        return None;
    }
    Some(1 + src[..offset].bytes().filter(|b| *b == b'\n').count() as u32)
}

/// Stage-1 diagnostic from a `toml` parse error (span → line).
fn parse_diagnostic(file: &str, src: &str, err: &toml::de::Error) -> Diagnostic {
    let line = err
        .span()
        .and_then(|span| line_of_offset(src, span.start))
        .or(Some(1));
    Diagnostic {
        severity: Severity::Error,
        stage: Stage::Parse,
        file: file.to_string(),
        line,
        key_path: "<document>".to_string(),
        rule: "the file must be well-formed TOML".to_string(),
        expected: "a syntactically valid TOML document".to_string(),
        found: err.message().to_string(),
        hint: "fix the syntax error at the indicated position (TOML v1.0 grammar); the message above comes from the TOML parser.".to_string(),
    }
}

/// Convert a parsed TOML document to the JSON value the schema pass speaks
/// (doc 07 §3.4 stage 1: `product.toml` → TOML → `serde_json::Value`).
/// The error is boxed to keep the `Result` small (`clippy::result_large_err`);
/// callers unbox it into the aggregate `Vec<Diagnostic>`.
fn toml_to_json(file: &str, value: &toml::Value) -> Result<serde_json::Value, Box<Diagnostic>> {
    match serde_json::to_value(value) {
        Ok(json) => Ok(json),
        Err(err) => Err(Box::new(Diagnostic {
            severity: Severity::Error,
            stage: Stage::Parse,
            file: file.to_string(),
            line: Some(1),
            key_path: "<document>".to_string(),
            rule: "the TOML document must be representable as JSON for schema validation".to_string(),
            expected: "TOML values convertible to JSON (no non-string datetimes in product.toml)".to_string(),
            found: err.to_string(),
            hint: "replace the offending value with a plain TOML scalar, array or table (doc 07 §3.4 stage 1).".to_string(),
        })),
    }
}

/// Stages 1–3 for a full `product.toml` document.
pub fn load_str(file: &str, src: &str, schema: &Schema) -> Result<ProductConfig, Vec<Diagnostic>> {
    let toml_value = match toml::from_str::<toml::Value>(src) {
        Ok(v) => v,
        Err(err) => return Err(vec![parse_diagnostic(file, src, &err)]),
    };
    let json = match toml_to_json(file, &toml_value) {
        Ok(j) => j,
        Err(d) => return Err(vec![*d]),
    };
    let lines = LineIndex::new(src);
    let structural = schema.validate(file, &json, &lines, RequiredMode::Full);
    if !structural.is_empty() {
        return Err(structural);
    }
    match serde_json::from_value::<ProductConfig>(json) {
        Ok(config) => Ok(config),
        Err(err) => Err(vec![Diagnostic {
            severity: Severity::Error,
            stage: Stage::Structural,
            file: file.to_string(),
            line: None,
            key_path: "<document>".to_string(),
            rule: "the document must deserialize into the typed product model".to_string(),
            expected: "a value shape accepted by the typed model (crates/product-config/src/model.rs)".to_string(),
            found: err.to_string(),
            hint: "the schema pass and the typed model disagree — this is a product-config crate bug; please report it with the offending file.".to_string(),
        }]),
    }
}

/// Stages 1–2 for a sparse layer document (user preferences, §4.2 layer 4):
/// every present key is validated against the same schema entries; missing
/// keys fall through to the next-lower layer (§4.4 rule 4).
pub fn load_partial_str(
    file: &str,
    src: &str,
    schema: &Schema,
) -> Result<PartialProduct, Vec<Diagnostic>> {
    let toml_value = match toml::from_str::<toml::Value>(src) {
        Ok(v) => v,
        Err(err) => return Err(vec![parse_diagnostic(file, src, &err)]),
    };
    let json = match toml_to_json(file, &toml_value) {
        Ok(j) => j,
        Err(d) => return Err(vec![*d]),
    };
    let lines = LineIndex::new(src);
    let structural = schema.validate(file, &json, &lines, RequiredMode::Partial);
    if !structural.is_empty() {
        return Err(structural);
    }
    match serde_json::from_value::<PartialProduct>(json) {
        Ok(partial) => Ok(partial),
        Err(err) => Err(vec![Diagnostic {
            severity: Severity::Error,
            stage: Stage::Structural,
            file: file.to_string(),
            line: None,
            key_path: "<document>".to_string(),
            rule: "the layer document must deserialize into the partial product model".to_string(),
            expected: "a sparse subset of the product.toml shape (crates/product-config/src/partial.rs)".to_string(),
            found: err.to_string(),
            hint: "the schema pass and the partial model disagree — this is a product-config crate bug; please report it with the offending file.".to_string(),
        }]),
    }
}

fn read_file(path: &Path, label: &str) -> Result<String, Box<Diagnostic>> {
    match std::fs::read_to_string(path) {
        Ok(src) => Ok(src),
        Err(err) => Err(Box::new(Diagnostic {
            severity: Severity::Error,
            stage: Stage::Parse,
            file: label.to_string(),
            line: Some(1),
            key_path: "<document>".to_string(),
            rule: "the configuration file must be readable".to_string(),
            expected: "a readable UTF-8 TOML file".to_string(),
            found: format!("{path:?}: {err}"),
            hint: "check the path and permissions; the platform adapter supplies per-layer locations (doc 07 §4.2).".to_string(),
        })),
    }
}

/// [`load_str`] from a path; `file` in diagnostics is the path as given.
pub fn load_file(path: &Path, schema: &Schema) -> Result<ProductConfig, Vec<Diagnostic>> {
    let file = path.display().to_string();
    let src = match read_file(path, &file) {
        Ok(s) => s,
        Err(d) => return Err(vec![*d]),
    };
    load_str(&file, &src, schema)
}

/// [`load_partial_str`] from a path.
pub fn load_partial_file(path: &Path, schema: &Schema) -> Result<PartialProduct, Vec<Diagnostic>> {
    let file = path.display().to_string();
    let src = match read_file(path, &file) {
        Ok(s) => s,
        Err(d) => return Err(vec![*d]),
    };
    load_partial_str(&file, &src, schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_of_offset_counts_newlines() {
        let src = "a = 1\nb = 2\nc = 3";
        assert_eq!(line_of_offset(src, 0), Some(1));
        assert_eq!(line_of_offset(src, 5), Some(1));
        assert_eq!(line_of_offset(src, 6), Some(2));
        assert_eq!(line_of_offset(src, 11), Some(2));
        assert_eq!(line_of_offset(src, 12), Some(3));
        assert_eq!(line_of_offset(src, 999), None);
    }

    #[test]
    fn parse_error_carries_the_parser_line() {
        let schema = Schema::empty_for_tests();
        let diags = match load_str("bad.toml", "a = \n[unclosed", &schema) {
            Ok(_) => Vec::new(),
            Err(d) => d,
        };
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].stage, Stage::Parse);
        assert_eq!(diags[0].file, "bad.toml");
        assert!(
            diags[0].line.is_some(),
            "parse errors are locatable: {diags:?}"
        );
    }
}
