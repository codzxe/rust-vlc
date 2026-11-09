//! `toml_lite` — shared std-only TOML-subset reader for the issue-#20 tools
//! (feature-audit / config-validator / parity-report).
//!
//! xtask is std-only by C5 (no `toml`/`serde_json` crates), same posture as
//! `check_architecture`'s hand-rolled manifest reader. This module is the
//! shared version covering the value shapes the audited files actually use:
//!
//! * `[table]` and `[a.b.c]` headers, `[[array-of-tables]]` rows;
//! * `key = value` with basic strings, integers, floats, booleans,
//!   single/multi-line arrays of scalars, and inline tables
//!   (`{ k = "v", ... }`, also allowed to span lines);
//! * `#` comments (never inside strings) and blank lines.
//!
//! Deliberately NOT supported (parse error, loudly, with the line number —
//! never a silent skip): multi-line basic strings (`"""`), literal strings
//! (`'...'`), dotted keys, dates, mixed-type arrays. None of the audited
//! files use them; a future file that does should fail the tool, not lie.
//!
//! Ownership: issue #20 (card 8.F1). Registered under the same
//! granted-registry-ownership precedent as check-panics (7-e) / visual (4.G1).

use std::fmt;

/// A parse failure with the 1-based line where it was detected.
#[derive(Debug)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

/// A parsed value in the subset.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    /// Array of scalar values (strings/ints/bools — the files never mix).
    Array(Vec<Value>),
    /// Inline table: ordered `key = value` pairs.
    Table(Vec<(String, Value)>),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_table(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Table(t) => Some(t),
            _ => None,
        }
    }

    /// Human name of the value's type, for mismatch diagnostics.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Str(_) => "string",
            Value::Int(_) => "integer",
            Value::Float(_) => "float",
            Value::Bool(_) => "boolean",
            Value::Array(_) => "array",
            Value::Table(_) => "table",
        }
    }
}

/// One `[table.header]` and its entries, in source order.
#[derive(Debug)]
pub struct Table {
    /// Dotted path segments (`[a.b]` → `["a", "b"]`).
    pub path: Vec<String>,
    pub entries: Vec<(String, Value)>,
}

/// One `[[name]]` row.
#[derive(Debug)]
pub struct ArrayRow {
    pub name: String,
    pub entries: Vec<(String, Value)>,
}

/// A parsed document.
#[derive(Debug, Default)]
pub struct Document {
    pub tables: Vec<Table>,
    pub arrays: Vec<ArrayRow>,
    /// Bare top-level `key = value` entries before any `[table]` header
    /// (the locale catalogs use quoted dotted keys this way).
    pub root_entries: Vec<(String, Value)>,
}

impl Document {
    /// Look up a table by exact dotted path.
    pub fn table(&self, path: &str) -> Option<&Table> {
        let want: Vec<&str> = path.split('.').collect();
        self.tables.iter().find(|t| {
            t.path.len() == want.len() && t.path.iter().zip(want.iter()).all(|(a, b)| a == b)
        })
    }

    /// All rows of `[[name]]`, in source order.
    pub fn array_rows(&self, name: &str) -> Vec<&ArrayRow> {
        self.arrays.iter().filter(|r| r.name == name).collect()
    }

    /// Scalar lookup by dotted path: `key` (bare root entry) or
    /// `table.sub.key` (any table depth).
    pub fn get(&self, dotted: &str) -> Option<&Value> {
        let segments: Vec<&str> = dotted.split('.').collect();
        if segments.len() == 1 {
            return self
                .root_entries
                .iter()
                .find(|(k, _)| k == segments[0])
                .map(|(_, v)| v);
        }
        let (key, table_path) = segments.split_last()?;
        self.tables
            .iter()
            .find(|t| {
                t.path.len() == table_path.len()
                    && t.path.iter().zip(table_path.iter()).all(|(a, b)| a == b)
            })?
            .entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// Whether a bare top-level key exists (quoted catalog keys like
    /// `"product.description"`).
    pub fn has_root_key(&self, key: &str) -> bool {
        self.root_entries.iter().any(|(k, _)| k == key)
    }
}

/// Parse a TOML-subset document.
pub fn parse(text: &str) -> Result<Document, ParseError> {
    let mut doc = Document::default();
    let mut current: Option<(Vec<String>, usize)> = None; // table path + index

    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0usize;
    while i < lines.len() {
        let lineno = i + 1;
        let line = strip_comment(lines[i]);
        let trimmed = line.trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }
        if trimmed.starts_with("[[") {
            let Some(close) = trimmed.find("]]") else {
                return Err(ParseError {
                    line: lineno,
                    message: "unterminated [[array-of-tables]] header".into(),
                });
            };
            let name = trimmed[2..close].trim().to_string();
            if name.is_empty() || name.contains('.') {
                return Err(ParseError {
                    line: lineno,
                    message: format!("unsupported array-of-tables header `{name}`"),
                });
            }
            doc.arrays.push(ArrayRow {
                name,
                entries: Vec::new(),
            });
            current = None; // entries attach to the array row, not a table
            i += 1;
            continue;
        }
        if trimmed.starts_with('[') {
            let Some(close) = trimmed.rfind(']') else {
                return Err(ParseError {
                    line: lineno,
                    message: "unterminated [table] header".into(),
                });
            };
            let path: Vec<String> = trimmed[1..close]
                .split('.')
                .map(|s| s.trim().to_string())
                .collect();
            if path.iter().any(|s| s.is_empty()) {
                return Err(ParseError {
                    line: lineno,
                    message: format!("bad table header `{trimmed}`"),
                });
            }
            doc.tables.push(Table {
                path: path.clone(),
                entries: Vec::new(),
            });
            current = Some((path, doc.tables.len() - 1));
            i += 1;
            continue;
        }
        // key = value (may continue over lines for arrays/inline tables)
        let Some(eq) = find_top_level_eq(trimmed) else {
            return Err(ParseError {
                line: lineno,
                message: format!("expected `key = value` or a header, got `{trimmed}`"),
            });
        };
        let key = if trimmed.starts_with('"') {
            // Quoted key (e.g. `"product.description" = "..."` in the
            // locale catalogs): one literal segment, dots included.
            let end = trimmed[1..]
                .find('"')
                .map(|p| p + 1)
                .ok_or_else(|| ParseError {
                    line: lineno,
                    message: format!("unterminated quoted key in `{trimmed}`"),
                })?;
            parse_string(&trimmed[..=end], lineno)?
        } else {
            trimmed[..eq].trim().to_string()
        };
        if key.is_empty() || (!trimmed.starts_with('"') && key.contains('.')) {
            return Err(ParseError {
                line: lineno,
                message: format!("unsupported key `{key}` (dotted keys are not in the subset)"),
            });
        }
        let mut value_text = trimmed[eq + 1..].trim().to_string();
        let opener = value_text.chars().next().unwrap_or(' ');
        // Multi-line continuation for arrays `[` and inline tables `{`.
        if (opener == '[' && !brackets_closed(&value_text, '[', ']'))
            || (opener == '{' && !brackets_closed(&value_text, '{', '}'))
        {
            let (joined, consumed) = join_until_closed(&lines, i, &value_text, opener)?;
            value_text = joined;
            let value = parse_value(&value_text, lineno)?;
            attach(&mut doc, &mut current, key, value);
            i += consumed + 1;
            continue;
        }
        let value = parse_value(&value_text, lineno)?;
        attach(&mut doc, &mut current, key, value);
        i += 1;
    }
    Ok(doc)
}

/// Attach a parsed entry to the current table, the last array row, or the
/// document's bare root entries.
fn attach(doc: &mut Document, current: &mut Option<(Vec<String>, usize)>, key: String, value: Value) {
    if let Some((_, idx)) = *current {
        doc.tables[idx].entries.push((key, value));
    } else if let Some(row) = doc.arrays.last_mut() {
        row.entries.push((key, value));
    } else {
        doc.root_entries.push((key, value));
    }
}

/// Strip a `#` comment, respecting double-quoted strings on the line.
fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut chars = line.char_indices();
    while let Some((idx, c)) = chars.next() {
        match c {
            '"' => in_string = !in_string,
            '\\' if in_string => {
                chars.next(); // skip escaped char
            }
            '#' if !in_string => return &line[..idx],
            _ => {}
        }
    }
    line
}

/// Find the `=` separating key from value, outside strings/brackets.
fn find_top_level_eq(line: &str) -> Option<usize> {
    let mut in_string = false;
    for (idx, c) in line.char_indices() {
        match c {
            '"' => in_string = !in_string,
            '=' if !in_string => return Some(idx),
            _ => {}
        }
    }
    None
}

/// Whether brackets of the given kind are balanced in the text (strings
/// respected). Used to detect multi-line arrays / inline tables.
fn brackets_closed(text: &str, open: char, close: char) -> bool {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => in_string = !in_string,
            '\\' if in_string => {
                chars.next();
            }
            ch if !in_string && ch == open => depth += 1,
            ch if !in_string && ch == close => depth -= 1,
            _ => {}
        }
    }
    depth <= 0
}

/// Join continuation lines until the brackets balance; returns the joined
/// value text and the number of extra lines consumed.
fn join_until_closed(
    lines: &[&str],
    start: usize,
    first: &str,
    opener: char,
) -> Result<(String, usize), ParseError> {
    let close = if opener == '[' { ']' } else { '}' };
    let mut joined = first.to_string();
    let mut consumed = 0usize;
    let mut i = start + 1;
    while i < lines.len() {
        let piece = strip_comment(lines[i]).trim();
        if !piece.is_empty() {
            joined.push(' ');
            joined.push_str(piece);
        }
        consumed += 1;
        if brackets_closed(&joined, opener, close) {
            return Ok((joined, consumed));
        }
        i += 1;
    }
    Err(ParseError {
        line: start + 1,
        message: "unterminated multi-line array/inline table".into(),
    })
}

/// Parse a single value (already trimmed, comments stripped).
fn parse_value(text: &str, line: usize) -> Result<Value, ParseError> {
    let t = text.trim();
    if t.is_empty() {
        return Err(ParseError { line, message: "empty value".into() });
    }
    if t.starts_with('"') {
        return parse_string(t, line).map(Value::Str);
    }
    if t.starts_with('[') {
        let inner = t
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .ok_or_else(|| ParseError {
                line,
                message: "bad array literal".into(),
            })?;
        let mut items = Vec::new();
        for piece in split_top_level(inner, ',') {
            let piece = piece.trim();
            if piece.is_empty() {
                continue;
            }
            items.push(parse_value(piece, line)?);
        }
        return Ok(Value::Array(items));
    }
    if t.starts_with('{') {
        let inner = t
            .strip_prefix('{')
            .and_then(|s| s.strip_suffix('}'))
            .ok_or_else(|| ParseError {
                line,
                message: "bad inline table".into(),
            })?;
        let mut entries = Vec::new();
        for piece in split_top_level(inner, ',') {
            let piece = piece.trim();
            if piece.is_empty() {
                continue;
            }
            let Some(eq) = find_top_level_eq(piece) else {
                return Err(ParseError {
                    line,
                    message: format!("bad inline-table entry `{piece}`"),
                });
            };
            let key = piece[..eq].trim().to_string();
            let value = parse_value(piece[eq + 1..].trim(), line)?;
            entries.push((key, value));
        }
        return Ok(Value::Table(entries));
    }
    if t == "true" {
        return Ok(Value::Bool(true));
    }
    if t == "false" {
        return Ok(Value::Bool(false));
    }
    // Integer (with optional sign; no underscores/dates in the audited files).
    if let Ok(int) = t.parse::<i64>() {
        return Ok(Value::Int(int));
    }
    if let Ok(float) = t.parse::<f64>() {
        return Ok(Value::Float(float));
    }
    Err(ParseError { line, message: format!("unsupported value `{t}`") })
}

/// Parse a double-quoted string with the standard escapes.
fn parse_string(t: &str, line: usize) -> Result<String, ParseError> {
    let body = t.strip_prefix('"').and_then(|s| s.strip_suffix('"')).ok_or_else(|| {
        ParseError { line, message: format!("bad string literal `{t}`") }
    })?;
    let mut out = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('b') => out.push('\u{0008}'),
            Some('f') => out.push('\u{000C}'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            other => {
                return Err(ParseError {
                    line,
                    message: format!("unsupported escape `\\{other:?}`"),
                })
            }
        }
    }
    Ok(out)
}

/// Split on `sep` at bracket depth 0, string-aware.
fn split_top_level(text: &str, sep: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut start = 0usize;
    let mut chars = text.char_indices();
    while let Some((idx, c)) = chars.next() {
        match c {
            '"' => in_string = !in_string,
            '\\' if in_string => {
                chars.next();
            }
            ch if !in_string && (ch == '[' || ch == '{') => depth += 1,
            ch if !in_string && (ch == ']' || ch == '}') => depth -= 1,
            ch if !in_string && depth == 0 && ch == sep => {
                parts.push(&text[start..idx]);
                start = idx + 1;
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_matrix_shaped_document() {
        let text = r#"
# leading comment
[meta]
schema_version = 1
last_updated = "2026-10-22"

[[feature]]
id = "container-x"
category = "container"
phase_target = 5
vlc_modules = ["mkv", "mp4"]
status = { windows = "implemented", linux = "not_started", macos = "web-placeholder", web = "not_started" }
evidence = "crates/demux/src/x.rs (card 5.D1); docs/formats/x.md # see also"
notes = "trailing comment # inside string stays"
"#;
        let doc = parse(text).expect("parses");
        assert_eq!(doc.get("meta.schema_version"), Some(&Value::Int(1)));
        let rows = doc.array_rows("feature");
        assert_eq!(rows.len(), 1);
        let id = rows[0].entries.iter().find(|(k, _)| k == "id").unwrap();
        assert_eq!(id.1, Value::Str("container-x".into()));
        let status = rows[0].entries.iter().find(|(k, _)| k == "status").unwrap();
        let Value::Table(t) = &status.1 else { panic!("status table") };
        assert_eq!(t.len(), 4);
        assert_eq!(t[0].1.as_str(), Some("implemented"));
        let evidence = rows[0].entries.iter().find(|(k, _)| k == "evidence").unwrap();
        assert_eq!(
            evidence.1.as_str(),
            Some("crates/demux/src/x.rs (card 5.D1); docs/formats/x.md # see also")
        );
    }

    #[test]
    fn parses_multiline_array_and_nested_tables() {
        let text = "[a.b]\nknown = [\n    \"x\", \"y\",\n    \"z\",\n]\nflag = true\n";
        let doc = parse(text).expect("parses");
        let known = doc.get("a.b.known").expect("known");
        let Value::Array(items) = known else { panic!("array") };
        assert_eq!(items.len(), 3);
        assert_eq!(doc.get("a.b.flag"), Some(&Value::Bool(true)));
    }

    #[test]
    fn comment_after_hash_outside_string_is_stripped() {
        let text = "[t]\nv = 42 # the answer\n";
        let doc = parse(text).expect("parses");
        assert_eq!(doc.get("t.v"), Some(&Value::Int(42)));
    }

    #[test]
    fn rejects_unsupported_shapes_loudly() {
        assert!(parse("[t]\nv = '''raw'''\n").is_err());
        assert!(parse("[t]\nv = 2026-10-22\n").is_err());
        assert!(parse("[t]\nv\n").is_err());
        assert!(parse("no header\n").is_err());
    }

    #[test]
    fn inline_table_multiline_continuation() {
        let text = "[t]\nv = {\n  a = \"1\",\n  b = \"2\",\n}\n";
        let doc = parse(text).expect("parses");
        let Value::Table(t) = doc.get("t.v").unwrap() else { panic!() };
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn array_rows_keep_source_order() {
        let text = "[[r]]\nk = 1\n[[r]]\nk = 2\n[[r]]\nk = 3\n";
        let doc = parse(text).expect("parses");
        let ks: Vec<i64> = doc
            .array_rows("r")
            .iter()
            .map(|r| r.entries[0].1.as_int().unwrap())
            .collect();
        assert_eq!(ks, vec![1, 2, 3]);
    }
}
