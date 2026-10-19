//! Source line indexing: maps a dotted key path to the 1-based line of the
//! TOML source that defines it, for diagnostic locations.
//!
//! The `toml` crate exposes spans only on parse errors, not on parsed values,
//! and no spanned-TOML crate is pre-audited (docs/dependencies/
//! preaudit-foundation.md), so this is a small read-only scanner. It tracks
//! table headers (`[a.b]`, `[[a.b]]`) and key assignments (`k = v`, `k.v = w`)
//! and records, for every full dotted path, the line it starts on. Lookups
//! fall back to the deepest known ancestor (a missing `identity.name`
//! points at the `[identity]` header; a missing table points at line 1).

use std::collections::HashMap;

/// Line map from dotted key path (array-of-table elements indexed as
/// `path.to.array.0.field`) to the 1-based source line.
#[derive(Debug, Clone)]
pub struct LineIndex {
    lines: HashMap<String, u32>,
}

impl LineIndex {
    /// Scan `src` (UTF-8 TOML text) and index every assignable path.
    pub fn new(src: &str) -> Self {
        let mut lines: HashMap<String, u32> = HashMap::new();
        let mut table: Vec<String> = Vec::new();
        let mut array_counts: HashMap<Vec<String>, usize> = HashMap::new();
        for (idx, raw) in src.lines().enumerate() {
            let lineno = (idx + 1) as u32;
            let stripped = strip_comment(raw);
            let trimmed = stripped.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(header) = table_header(trimmed) {
                // [a.b] -> table path [a, b]; [[a.b]] -> array element index.
                let is_array = header.is_array;
                let mut path = header.segments;
                if is_array {
                    let count = array_counts.entry(path.clone()).or_insert(0);
                    let index = *count;
                    *count += 1;
                    let dotted = join_path(&path);
                    lines.entry(dotted).or_insert(lineno);
                    path.push(index.to_string());
                }
                let dotted = join_path(&path);
                lines.entry(dotted).or_insert(lineno);
                table = path;
                continue;
            }
            if let Some(key) = assignment_key(trimmed) {
                let mut path = table.clone();
                path.extend(key);
                let dotted = join_path(&path);
                lines.entry(dotted).or_insert(lineno);
            }
        }
        LineIndex { lines }
    }

    /// The 1-based line of the deepest known ancestor of `path` (including
    /// `path` itself). Dotted segments; integer segments index into
    /// arrays-of-tables.
    pub fn line_of(&self, path: &str) -> Option<u32> {
        if let Some(line) = self.lines.get(path) {
            return Some(*line);
        }
        let segments: Vec<&str> = path.split('.').collect();
        for len in (1..=segments.len()).rev() {
            let candidate = segments[..len].join(".");
            if let Some(line) = self.lines.get(&candidate) {
                return Some(*line);
            }
        }
        None
    }
}

fn join_path(path: &[String]) -> String {
    path.join(".")
}

struct Header {
    segments: Vec<String>,
    is_array: bool,
}

/// Recognize `[table]` / `[[array]]` headers (bare keys, dotted allowed).
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

/// The key part of a `key = value` line, split into dotted segments.
fn assignment_key(line: &str) -> Option<Vec<String>> {
    let eq = line.find('=')?;
    let key = line[..eq].trim();
    if key.is_empty() {
        return None;
    }
    // Reject table headers mis-detected (e.g. "[x] =" is invalid TOML anyway).
    if key.starts_with('[') {
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

/// Strip a trailing comment, respecting quoted strings (`#` inside quotes is
/// literal — e.g. `accent_color = "#4f8cff"`).
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

/// Remove basic/literal quoting from a single key segment.
fn unquote(seg: &str) -> String {
    if seg.len() >= 2 {
        let bytes = seg.as_bytes();
        let first = bytes[0];
        let last = match bytes.last() {
            Some(b) => *b,
            None => return seg.to_string(),
        };
        if first == b'"' && last == b'"' {
            return seg[1..seg.len() - 1].to_string();
        }
        if first == b'\'' && last == b'\'' {
            return seg[1..seg.len() - 1].to_string();
        }
    }
    seg.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r##"schema_version = 1

[identity]
name = "Lumen"          # display name
accent = "#4f8cff"

[identity.ids]
reverse_dns = "dev.lumen"

[identity.ids.windows]
app_user_model_id = "dev.lumen.Lumen"

[[migrations.entries]]
from = 1
to = 2

[[migrations.entries]]
from = 2
to = 3
"##;

    #[test]
    fn finds_top_level_keys() {
        let idx = LineIndex::new(SRC);
        assert_eq!(idx.line_of("schema_version"), Some(1));
    }

    #[test]
    fn finds_table_keys() {
        let idx = LineIndex::new(SRC);
        assert_eq!(idx.line_of("identity.name"), Some(4));
        assert_eq!(idx.line_of("identity.ids.reverse_dns"), Some(8));
        assert_eq!(
            idx.line_of("identity.ids.windows.app_user_model_id"),
            Some(11)
        );
    }

    #[test]
    fn hash_inside_string_is_not_a_comment() {
        let idx = LineIndex::new("accent = \"#4f8cff\" # trailing\n");
        assert_eq!(idx.line_of("accent"), Some(1));
    }

    #[test]
    fn array_of_tables_elements_are_indexed() {
        let idx = LineIndex::new(SRC);
        assert_eq!(idx.line_of("migrations.entries.0.from"), Some(14));
        assert_eq!(idx.line_of("migrations.entries.1.to"), Some(19));
    }

    #[test]
    fn missing_key_falls_back_to_nearest_ancestor() {
        let idx = LineIndex::new(SRC);
        // identity.description_key does not exist; [identity] is on line 3.
        assert_eq!(idx.line_of("identity.description_key"), Some(3));
        // A wholly missing table has no ancestor below the root.
        assert_eq!(idx.line_of("nonexistent.table.key"), None);
    }

    #[test]
    fn table_header_line_is_the_fallback_for_missing_members() {
        let idx = LineIndex::new("[window]\nwidth = 1280\n");
        assert_eq!(idx.line_of("window.height"), Some(1));
        assert_eq!(idx.line_of("window.width"), Some(2));
    }
}
