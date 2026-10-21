//! The std-only JSON value and its deterministic emitter (2.E1).
//!
//! The doc 10 §7 evidence artifacts are JSON (`transcripts/<id>.json`
//! per §6.4, the `evidence/1` ledger per §7.2). This crate is
//! external-dependency-free by C5, so the writer is in-house — the
//! 2.A3/2.B5/2.D1 std-only-tool precedents. Scope is exactly what the
//! artifacts need: a value tree, one pretty emitter, RFC 8259 string
//! escaping. Determinism is by construction: object fields keep
//! insertion order (the writer's, stable per code version), integers
//! stay integers (no float formatting anywhere in the transcripts —
//! the wire's float scalars are recorded through their fixed-`{:.N}`
//! Display forms instead, see [`crate::harness`]).

use std::fmt::Write as _;

/// One JSON value (the subset the evidence artifacts need).
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    /// `null`.
    Null,
    /// `true` / `false`.
    Bool(bool),
    /// A signed integer (µs positions, counters).
    Int(i64),
    /// An unsigned integer (seq numbers, byte counts).
    UInt(u64),
    /// A string (already the final text; escaped at emit time).
    Str(String),
    /// An array.
    Arr(Vec<Json>),
    /// An object — insertion-ordered (deterministic per code version).
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// Build an object from field pairs (order-preserving sugar).
    pub fn obj(fields: Vec<(&str, Json)>) -> Json {
        Json::Obj(
            fields
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        )
    }

    /// A string value.
    pub fn str(text: impl Into<String>) -> Json {
        Json::Str(text.into())
    }

    /// The pretty (2-space) deterministic serialization, with a final
    /// newline — the exact bytes the committed artifacts carry.
    ///
    /// # Errors
    /// Never (string formatting is infallible for these values); the
    /// `Result` shape only satisfies `write!`.
    pub fn render(&self) -> String {
        let mut out = String::new();
        let _ = self.write_pretty(&mut out, 0);
        out.push('\n');
        out
    }

    fn write_pretty(&self, out: &mut String, depth: usize) -> std::fmt::Result {
        let pad = "  ".repeat(depth);
        let pad_inner = "  ".repeat(depth + 1);
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(true) => out.push_str("true"),
            Json::Bool(false) => out.push_str("false"),
            Json::Int(value) => write!(out, "{value}")?,
            Json::UInt(value) => write!(out, "{value}")?,
            Json::Str(text) => write_json_string(out, text)?,
            Json::Arr(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return Ok(());
                }
                out.push_str("[\n");
                for (index, item) in items.iter().enumerate() {
                    out.push_str(&pad_inner);
                    item.write_pretty(out, depth + 1)?;
                    if index + 1 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&pad);
                out.push(']');
            }
            Json::Obj(fields) => {
                if fields.is_empty() {
                    out.push_str("{}");
                    return Ok(());
                }
                out.push_str("{\n");
                for (index, (key, value)) in fields.iter().enumerate() {
                    out.push_str(&pad_inner);
                    write_json_string(out, key)?;
                    out.push_str(": ");
                    value.write_pretty(out, depth + 1)?;
                    if index + 1 < fields.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&pad);
                out.push('}');
            }
        }
        Ok(())
    }
}

/// RFC 8259 string escaping: `"` `\` and the control set; all other
/// bytes pass through (the transcripts are ASCII by construction —
/// the one non-ASCII source, event `Debug` forms, never carries
/// non-ASCII for this vertical's payloads; if one ever does, the
/// bytes pass through as UTF-8, which is legal JSON).
fn write_json_string(out: &mut String, text: &str) -> std::fmt::Result {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                write!(out, "\\u{:04x}", c as u32)?;
            }
            c => out.push(c),
        }
    }
    out.push('"');
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact emit shape: 2-space indent, trailing newline.
    #[test]
    fn renders_the_pretty_shape() {
        let value = Json::obj(vec![
            ("schema", Json::str("transcript/1")),
            ("count", Json::UInt(2)),
            ("items", Json::Arr(vec![Json::Int(-1), Json::Bool(true)])),
            ("empty", Json::Obj(vec![])),
        ]);
        assert_eq!(
            value.render(),
            "{\n  \
             \"schema\": \"transcript/1\",\n  \
             \"count\": 2,\n  \
             \"items\": [\n    -1,\n    true\n  ],\n  \
             \"empty\": {}\n\
             }\n"
        );
    }

    /// Escaping: quotes, backslash, newlines, control bytes.
    #[test]
    fn escapes_strings() {
        let value = Json::Str("a\"b\\c\nd\te\u{01}".to_string());
        assert_eq!(value.render(), "\"a\\\"b\\\\c\\nd\\te\\u0001\"\n");
    }

    /// Determinism: the same tree renders byte-identically twice, and
    /// field order is insertion order (never re-sorted).
    #[test]
    fn deterministic_and_order_preserving() {
        use testkit::Must;
        let value = Json::obj(vec![("zebra", Json::UInt(1)), ("alpha", Json::UInt(2))]);
        assert_eq!(value.render(), value.render());
        let text = value.render();
        assert!(text.find("\"zebra\"").must() < text.find("\"alpha\"").must());
    }

    /// The unit types render as their literals.
    #[test]
    fn scalars_render() {
        assert_eq!(Json::Null.render(), "null\n");
        assert_eq!(Json::Bool(false).render(), "false\n");
        assert_eq!(Json::Int(i64::MIN).render(), "-9223372036854775808\n");
        assert_eq!(Json::UInt(u64::MAX).render(), "18446744073709551615\n");
        assert_eq!(Json::Arr(vec![]).render(), "[]\n");
    }
}
