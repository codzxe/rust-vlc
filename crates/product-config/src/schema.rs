//! Data-driven structural validation against `product.schema.json`
//! (doc 07 §3.2, §3.4 stage 2).
//!
//! The schema file is the single source of truth for structural rules: the
//! engine below reads `type`, `required`, `additionalProperties`, `enum`,
//! `const`, `minimum`, `maximum`, `minLength`, `maxLength`, `minItems`,
//! `maxItems`, `items`, `pattern` (via the hand-written matcher registry),
//! `not.enum`, and the annotations `x-lumen-unit` / `x-lumen-hint` straight
//! from the parsed schema, so a range widened in the schema is enforced
//! without touching Rust code. An empty `enum: []` marks an *injected* enum
//! (doc 07 §3.2 `defaults` comment): the structural pass defers it to the
//! semantic stage, which receives the directory listing.
//!
//! Semantic-class rules (path existence, cross-file, cross-field, F1/F2)
//! cannot be expressed in JSON Schema and live in [`crate::semantic`].

use serde_json::Value;

use crate::diagnostic::{Diagnostic, Severity, Stage};
use crate::lines::LineIndex;
use crate::patterns::matcher_for;
use crate::suggest::did_you_mean;

/// Errors that mean the *schema itself* is broken (not the config).
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("product.schema.json is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("schema pattern has no hand-written matcher: {pattern}")]
    UnmatchedPattern { pattern: String },
}

/// Whether `required` is enforced (sparse layer documents are validated with
/// it off — doc 07 §4.4 rule 4: every present value is checked, missing keys
/// fall through to the next layer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequiredMode {
    Full,
    Partial,
}

/// A parsed `product.schema.json`.
#[derive(Debug, Clone)]
pub struct Schema {
    root: Value,
}

// `FromStr` (not an inherent `from_str`) per clippy::should_implement_trait;
// callers use `"…".parse::<Schema>()`.
impl std::str::FromStr for Schema {
    type Err = SchemaError;

    /// Parse the schema and verify every `pattern` it uses has a matcher
    /// (schema drift fails loudly instead of validating nothing).
    fn from_str(json: &str) -> Result<Schema, SchemaError> {
        let root: Value = serde_json::from_str(json)?;
        Schema::from_json(root)
    }
}

impl Schema {
    /// Same, from an already-parsed JSON value.
    pub fn from_json(root: Value) -> Result<Schema, SchemaError> {
        let schema = Schema { root };
        schema.check_pattern_coverage()?;
        Ok(schema)
    }

    /// The parsed schema document (for tools and tests).
    pub fn root(&self) -> &Value {
        &self.root
    }

    fn check_pattern_coverage(&self) -> Result<(), SchemaError> {
        fn walk(node: &Value) -> Option<String> {
            match node {
                Value::Object(map) => {
                    if let Some(Value::String(pattern)) = map.get("pattern")
                        && matcher_for(pattern).is_none()
                    {
                        return Some(pattern.clone());
                    }
                    map.values().find_map(walk)
                }
                Value::Array(items) => items.iter().find_map(walk),
                _ => None,
            }
        }
        match walk(&self.root) {
            Some(pattern) => Err(SchemaError::UnmatchedPattern { pattern }),
            None => Ok(()),
        }
    }

    /// An empty schema that validates nothing — test scaffolding for paths
    /// that are unreachable after a failed assertion.
    #[cfg(test)]
    pub(crate) fn empty_for_tests() -> Schema {
        Schema {
            root: Value::Object(serde_json::Map::new()),
        }
    }

    /// Resolve a local `$ref` (`#/$defs/name`) to its target node.
    fn resolve<'a>(&'a self, node: &'a Value) -> &'a Value {
        match node.get("$ref").and_then(Value::as_str) {
            Some(reference) if reference.starts_with("#/") => {
                let pointer = &reference[1..];
                match self.root.pointer(pointer) {
                    Some(target) => target,
                    None => node,
                }
            }
            _ => node,
        }
    }

    /// Validate `value` structurally; returns every violation (doc 07 §3.4
    /// stage 2: report every violation, not just the first).
    pub fn validate(
        &self,
        file: &str,
        value: &Value,
        lines: &LineIndex,
        mode: RequiredMode,
    ) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        self.check_node(&self.root, value, "", file, lines, mode, &mut out);
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn check_node(
        &self,
        schema_node: &Value,
        value: &Value,
        path: &str,
        file: &str,
        lines: &LineIndex,
        mode: RequiredMode,
        out: &mut Vec<Diagnostic>,
    ) {
        let node = self.resolve(schema_node);

        // type
        if let Some(expected_type) = node.get("type").and_then(Value::as_str)
            && !type_matches(expected_type, value)
        {
            out.push(self.diagnostic(
                file,
                lines,
                path,
                format!("must be of type {expected_type}"),
                format!("a {expected_type} value"),
                describe_value(value),
                generic_hint(node),
            ));
            // A wrong-typed value cannot satisfy the value constraints;
            // do not pile on (one actionable diagnostic per defect).
            return;
        }

        // const
        if let Some(const_value) = node.get("const")
            && value != const_value
        {
            out.push(self.diagnostic(
                file,
                lines,
                path,
                format!("must be exactly {}", describe_value(const_value)),
                describe_value(const_value),
                describe_value(value),
                generic_hint(node),
            ));
        }

        // enum (empty enum = injected at validation time; deferred to semantic)
        if let Some(Value::Array(variants)) = node.get("enum")
            && !variants.is_empty()
            && !variants.contains(value)
        {
            // Legal values are vocabulary tokens and render bare; the
            // *found* value stays a quoted literal (`describe_value`) so
            // `value: "always"` reads as a literal against `rule: must be
            // one of: auto, ...` — same convention as the semantic pass
            // (injected enums) and the §3.5 golden prose.
            let bare = |v: &Value| match v {
                Value::String(s) => s.clone(),
                other => describe_value(other),
            };
            let list = variants.iter().map(bare).collect::<Vec<_>>().join(", ");
            out.push(self.diagnostic(
                file,
                lines,
                path,
                format!("must be one of: {list}"),
                format!("one of: {list}"),
                describe_value(value),
                generic_hint(node),
            ));
        }

        // not: { enum: [ ... ] } (reserved names, doc 07 §3.3(a))
        if let Some(not_node) = node.get("not")
            && let Some(Value::Array(reserved)) = not_node.get("enum")
            && reserved.contains(value)
        {
            let list = reserved
                .iter()
                .map(describe_value)
                .collect::<Vec<_>>()
                .join(", ");
            out.push(self.diagnostic(
                file,
                lines,
                path,
                format!("must not be a reserved name (clean-room rule, MP \u{a7}1); reserved: {list}"),
                format!("any value except: {list}"),
                describe_value(value),
                "choose a name that is not on the reserved list; this project is a clean-room reimplementation (docs/MASTER-PROMPT.md \u{a7}1).".to_string(),
            ));
        }

        // numeric ranges (units expanded for humans, doc 07 §3.5 rule 2)
        if value.is_number() {
            let unit = node
                .get("x-lumen-unit")
                .and_then(Value::as_str)
                .filter(|u| *u == "MiB");
            let min = node.get("minimum").and_then(Value::as_f64);
            let max = node.get("maximum").and_then(Value::as_f64);
            let found = value.as_f64().unwrap_or(f64::NAN);
            let below = min.is_some_and(|m| found < m);
            let above = max.is_some_and(|m| found > m);
            if below || above {
                out.push(self.range_diagnostic(file, lines, path, node, unit, min, max, found));
            }
        }

        // string length
        if let Some(text) = value.as_str() {
            let chars = text.chars().count();
            if let Some(min) = node.get("minLength").and_then(Value::as_u64)
                && (chars as u64) < min
            {
                out.push(self.diagnostic(
                    file,
                    lines,
                    path,
                    format!("must be at least {min} character(s) long"),
                    format!("a string of at least {min} characters"),
                    format!("{chars} character(s): {}", describe_value(value)),
                    generic_hint(node),
                ));
            }
            if let Some(max) = node.get("maxLength").and_then(Value::as_u64)
                && (chars as u64) > max
            {
                out.push(self.diagnostic(
                    file,
                    lines,
                    path,
                    format!("must be at most {max} character(s) long"),
                    format!("a string of at most {max} characters"),
                    format!("{chars} character(s): {}", describe_value(value)),
                    generic_hint(node),
                ));
            }
            // pattern (matcher guaranteed by Schema::load coverage check)
            if let Some(pattern) = node.get("pattern").and_then(Value::as_str) {
                let ok = match matcher_for(pattern) {
                    Some(matcher) => matcher(text),
                    None => false,
                };
                if !ok {
                    out.push(self.diagnostic(
                        file,
                        lines,
                        path,
                        format!("must match the pattern {pattern}"),
                        format!("a string matching {pattern}"),
                        describe_value(value),
                        generic_hint(node),
                    ));
                }
            }
        }

        // arrays
        if let Some(items) = value.as_array() {
            if let Some(min) = node.get("minItems").and_then(Value::as_u64)
                && (items.len() as u64) < min
            {
                out.push(self.diagnostic(
                    file,
                    lines,
                    path,
                    format!("must contain at least {min} item(s)"),
                    format!("an array of at least {min} items"),
                    format!("an array of {} items", items.len()),
                    generic_hint(node),
                ));
            }
            if let Some(max) = node.get("maxItems").and_then(Value::as_u64)
                && (items.len() as u64) > max
            {
                out.push(self.diagnostic(
                    file,
                    lines,
                    path,
                    format!("must contain at most {max} item(s)"),
                    format!("an array of at most {max} items"),
                    format!("an array of {} items", items.len()),
                    generic_hint(node),
                ));
            }
            if let Some(item_schema) = node.get("items") {
                for (i, item) in items.iter().enumerate() {
                    let item_path = child_path(path, &i.to_string());
                    self.check_node(item_schema, item, &item_path, file, lines, mode, out);
                }
            }
        }

        // objects
        if let Some(map) = value.as_object() {
            let properties = node.get("properties").and_then(Value::as_object);
            if mode == RequiredMode::Full
                && let Some(Value::Array(required)) = node.get("required")
            {
                for key in required.iter().filter_map(Value::as_str) {
                    if !map.contains_key(key) {
                        let key_path = child_path(path, key);
                        out.push(self.diagnostic(
                            file,
                            lines,
                            &key_path,
                            "required key is missing".to_string(),
                            "a present value for this key".to_string(),
                            "absent".to_string(),
                            "add the key with a legal value; the full shape is defined in product.schema.json (docs/07-config-driven-system.md \u{a7}3.2).".to_string(),
                        ));
                    }
                }
            }
            for (key, child) in map {
                let child_path = child_path(path, key);
                let child_schema = match properties.and_then(|p| p.get(key)) {
                    Some(s) => s,
                    None => match node.get("additionalProperties") {
                        // additionalProperties: false — unknown keys are
                        // errors: typo detection (doc 07 §3.2).
                        Some(Value::Bool(false)) => {
                            let known: Vec<&str> = properties
                                .map(|p| p.keys().map(String::as_str).collect())
                                .unwrap_or_default();
                            let mut hint = format!(
                                "remove the unknown key or rename it to a legal one; the legal keys of this table are: {}",
                                known.join(", ")
                            );
                            if let Some(suggestion) = did_you_mean(key, &known) {
                                hint = format!("did you mean `{suggestion}`? {hint}");
                            }
                            out.push(self.diagnostic(
                                file,
                                lines,
                                &child_path,
                                "unknown key (this table forbids additional properties: typo detection)".to_string(),
                                format!("one of: {}", known.join(", ")),
                                key.to_string(),
                                hint,
                            ));
                            continue;
                        }
                        // additionalProperties: { schema } (features.*: bool)
                        Some(schema) => schema,
                        None => continue,
                    },
                };
                self.check_node(child_schema, child, &child_path, file, lines, mode, out);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn range_diagnostic(
        &self,
        file: &str,
        lines: &LineIndex,
        path: &str,
        node: &Value,
        unit: Option<&str>,
        min: Option<f64>,
        max: Option<f64>,
        found: f64,
    ) -> Diagnostic {
        let format_bound = |v: f64| -> String {
            match unit {
                Some(_) if v.fract() == 0.0 && (v / MIB as f64).fract() == 0.0 => {
                    format!("{v} ({} MiB)", (v as u64) / MIB)
                }
                Some(_) => format!("{v} ({:.1} MiB)", v / MIB as f64),
                None => format!("{v}"),
            }
        };
        let format_value = |v: f64| -> String {
            match unit {
                Some(_) => format!("{v} ({:.1} MiB)", v / MIB as f64),
                None => format!("{v}"),
            }
        };
        let ty = node.get("type").and_then(Value::as_str).unwrap_or("number");
        let (rule, expected) = match (min, max) {
            (Some(min), Some(max)) => (
                format!(
                    "must be >= {} and <= {}",
                    format_bound(min),
                    format_bound(max)
                ),
                format!("{} in {}..={}", ty, format_bound(min), format_bound(max)),
            ),
            (Some(min), None) => (
                format!("must be >= {}", format_bound(min)),
                format!("{} >= {}", ty, format_bound(min)),
            ),
            (None, Some(max)) => (
                format!("must be <= {}", format_bound(max)),
                format!("{} <= {}", ty, format_bound(max)),
            ),
            (None, None) => (
                "must be a finite number".to_string(),
                format!("a finite {ty}"),
            ),
        };
        self.diagnostic(
            file,
            lines,
            path,
            rule,
            expected,
            format_value(found),
            generic_hint(node),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn diagnostic(
        &self,
        file: &str,
        lines: &LineIndex,
        path: &str,
        rule: String,
        expected: String,
        found: String,
        hint: String,
    ) -> Diagnostic {
        Diagnostic {
            severity: Severity::Error,
            stage: Stage::Structural,
            file: file.to_string(),
            line: if path.is_empty() {
                Some(1)
            } else {
                lines.line_of(path)
            },
            key_path: if path.is_empty() {
                "<root>".to_string()
            } else {
                path.to_string()
            },
            rule,
            expected,
            found,
            hint,
        }
    }
}

const MIB: u64 = 1024 * 1024;

fn child_path(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        key.to_string()
    } else {
        format!("{parent}.{key}")
    }
}

fn type_matches(expected: &str, value: &Value) -> bool {
    match expected {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        _ => true,
    }
}

fn describe_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("{s:?}"),
        Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(describe_value).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Object(map) => {
            let inner: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{k}={}", describe_value(v)))
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
    }
}

fn generic_hint(node: &Value) -> String {
    if let Some(hint) = node.get("x-lumen-hint").and_then(Value::as_str) {
        return hint.to_string();
    }
    if let Some(desc) = node.get("description").and_then(Value::as_str) {
        return desc.to_string();
    }
    "the governing rule is documented in product.schema.json and docs/07-config-driven-system.md \u{a7}3.".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> Schema {
        let json = r#"{
            "type": "object",
            "required": ["a"],
            "additionalProperties": false,
            "properties": {
                "a": { "type": "integer", "minimum": 1, "maximum": 10 },
                "b": { "type": "string", "pattern": "^[a-z]+$" }
            }
        }"#;
        match json.parse::<Schema>() {
            Ok(s) => s,
            Err(e) => unreachable!("fixture schema must load: {e}"),
        }
    }

    fn validate(value: &Value) -> Vec<Diagnostic> {
        schema().validate(
            "t.toml",
            value,
            &LineIndex::new("a = 1\n"),
            RequiredMode::Full,
        )
    }

    #[test]
    fn valid_value_produces_no_diagnostics() {
        let mut obj = serde_json::Map::new();
        obj.insert("a".to_string(), Value::from(5));
        assert!(validate(&Value::Object(obj)).is_empty());
    }

    #[test]
    fn missing_required_is_reported_with_ancestor_line() {
        let diags = validate(&Value::Object(serde_json::Map::new()));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].key_path, "a");
        assert!(diags[0].rule.contains("required"));
    }

    #[test]
    fn below_minimum_reports_range_with_expected_and_found() {
        let mut obj = serde_json::Map::new();
        obj.insert("a".to_string(), Value::from(0));
        let diags = validate(&Value::Object(obj));
        assert_eq!(diags.len(), 1);
        assert!(diags[0].rule.contains("must be >= 1"));
        assert_eq!(diags[0].found, "0");
        assert!(diags[0].expected.contains("1..=10"));
    }

    #[test]
    fn unknown_key_is_reported_even_in_partial_mode() {
        let mut obj = serde_json::Map::new();
        obj.insert("ab".to_string(), Value::from(1));
        let diags = schema().validate(
            "t.toml",
            &Value::Object(obj),
            &LineIndex::new("ab = 1\n"),
            RequiredMode::Partial,
        );
        assert_eq!(diags.len(), 1);
        assert!(
            diags[0].hint.contains("did you mean `a`"),
            "{}",
            diags[0].hint
        );
    }

    #[test]
    fn partial_mode_skips_required_but_keeps_value_rules() {
        let mut obj = serde_json::Map::new();
        obj.insert("a".to_string(), Value::from(99));
        let diags = schema().validate(
            "t.toml",
            &Value::Object(obj),
            &LineIndex::new("a = 99\n"),
            RequiredMode::Partial,
        );
        assert_eq!(diags.len(), 1);
        assert!(diags[0].rule.contains("<= 10"), "{}", diags[0].rule);
    }

    #[test]
    fn unmatched_pattern_fails_schema_load_loudly() {
        let json = r#"{ "type": "object", "properties": { "x": { "type": "string", "pattern": "^nope$" } } }"#;
        match json.parse::<Schema>() {
            Ok(_) => unreachable!("unmatched pattern must fail load"),
            Err(SchemaError::UnmatchedPattern { pattern }) => {
                assert_eq!(pattern, "^nope$")
            }
            Err(e) => unreachable!("wrong error: {e}"),
        }
    }

    #[test]
    fn wrong_type_short_circuits_value_checks() {
        let mut obj = serde_json::Map::new();
        obj.insert("a".to_string(), Value::from("wide"));
        let diags = validate(&Value::Object(obj));
        assert_eq!(diags.len(), 1);
        assert!(diags[0].rule.contains("must be of type integer"));
    }
}
