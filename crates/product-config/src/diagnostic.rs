//! Actionable diagnostics — the doc 07 §3.5 error-message contract.
//!
//! Every validation failure carries: file, line (1-based, when locatable),
//! fully-qualified key path (`file:line:key.path`), the violated rule stated
//! as a constraint with units expanded for humans, expected vs found, and a
//! fix hint (or the document that governs the rule). Near-miss keys get a
//! "did you mean" hint (Levenshtein over the known key set).

use std::fmt;

/// Which pipeline stage produced the diagnostic (doc 07 §3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// TOML syntax / parse failure (§3.4 stage 1).
    Parse,
    /// JSON-Schema-structural violation (§3.4 stage 2).
    Structural,
    /// Cross-field / filesystem / cross-file rule (§3.4 stage 3).
    Semantic,
    /// Layered-resolution violation (§4.4).
    Merge,
}

impl Stage {
    fn as_str(self) -> &'static str {
        match self {
            Stage::Parse => "parse",
            Stage::Structural => "structural",
            Stage::Semantic => "semantic",
            Stage::Merge => "merge",
        }
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

/// One actionable configuration diagnostic (doc 07 §3.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub stage: Stage,
    /// File name as the user wrote it (e.g. `product.toml`).
    pub file: String,
    /// 1-based line of the offending key, when locatable in the source.
    pub line: Option<u32>,
    /// Fully-qualified dotted key path from the file root.
    pub key_path: String,
    /// The violated rule, stated as a constraint (units expanded).
    pub rule: String,
    /// What the value was expected to be.
    pub expected: String,
    /// What was actually found.
    pub found: String,
    /// Fix hint, or the document that governs the rule.
    pub hint: String,
}

impl Diagnostic {
    /// Location line: `error[config]: <file>:<line>:<key.path>` — the doc 07
    /// §3.5 `file:key.path` form extended with the line number (task 1.B1
    /// contract: every failure carries file + line + field path).
    pub fn location(&self) -> String {
        match self.line {
            Some(line) => format!("{}:{}:{}", self.file, line, self.key_path),
            None => format!("{}:{}", self.file, self.key_path),
        }
    }
}

/// Wrap `text` at `width` visible columns on word boundaries; continuation
/// lines are indented to `indent` so wrapped prose stays under its label.
fn wrap(text: &str, width: usize, indent: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split(' ') {
        let candidate_len = if current.is_empty() {
            word.len()
        } else {
            current.len() + 1 + word.len()
        };
        if !current.is_empty() && candidate_len > width {
            lines.push(current.clone());
            current.clear();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    let padding = " ".repeat(indent);
    for line in lines.iter_mut().skip(1) {
        let mut wrapped = padding.clone();
        wrapped.push_str(line);
        *line = wrapped;
    }
    lines
}

impl fmt::Display for Diagnostic {
    /// The doc 07 §3.5 mandatory format (key path, value, rule, hint) plus the
    /// 1.B1 extensions: line number in the location and an `expected:` line.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `error[config]:` is the doc 07 §3.5 mandatory prefix, kept verbatim
        // (severity swaps it to `warning[config]:`; the pipeline stage rides
        // along as a structured field, not rendered prose).
        writeln!(f, "{}[config]: {}", self.severity.as_str(), self.location())?;
        writeln!(f, "  value: {}", self.found)?;
        writeln!(f, "  expected: {}", self.expected)?;
        writeln!(f, "  rule:  {}", self.rule)?;
        let hint_lines = wrap(&self.hint, 68, 9);
        for (i, line) in hint_lines.iter().enumerate() {
            if i == 0 {
                writeln!(f, "  hint:  {line}")?;
            } else {
                writeln!(f, "{line}")?;
            }
        }
        Ok(())
    }
}

/// Render a list of diagnostics, each followed by a blank separator line.
pub fn render_all(diagnostics: &[Diagnostic]) -> String {
    let mut out = String::new();
    for d in diagnostics {
        out.push_str(&d.to_string());
    }
    if !diagnostics.is_empty() {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Diagnostic {
        Diagnostic {
            severity: Severity::Error,
            stage: Stage::Structural,
            file: "product.toml".to_string(),
            line: Some(57),
            key_path: "limits.probe_max_bytes".to_string(),
            rule: "must be >= 16777216 (16 MiB) and <= 67108864 (64 MiB)".to_string(),
            expected: "integer in 16777216 (16 MiB)..=67108864 (64 MiB)".to_string(),
            found: "1048576 (1.0 MiB)".to_string(),
            hint: "probing below 16 MiB rejects valid multi-track files; if you need a lower bound for an embedded target, change the schema range via review (docs/07-config-driven-system.md \u{a7}3.3), not this value.".to_string(),
        }
    }

    #[test]
    fn location_includes_file_line_and_key_path() {
        let d = sample();
        assert_eq!(d.location(), "product.toml:57:limits.probe_max_bytes");
    }

    #[test]
    fn location_without_line_is_doc_07_s35_form() {
        let mut d = sample();
        d.line = None;
        assert_eq!(d.location(), "product.toml:limits.probe_max_bytes");
    }

    #[test]
    fn display_has_all_mandatory_s35_fields() {
        let text = sample().to_string();
        let first = text.lines().next().unwrap_or_default();
        assert_eq!(
            first,
            "error[config]: product.toml:57:limits.probe_max_bytes"
        );
        assert!(text.contains("  value: 1048576 (1.0 MiB)\n"));
        assert!(
            text.contains("  rule:  must be >= 16777216 (16 MiB) and <= 67108864 (64 MiB)\n"),
            "{text}"
        );
        assert!(text.contains("  hint:  probing below 16 MiB"));
        // expected vs found are both present (1.B1 contract)
        assert!(text.contains("  expected: integer in 16777216"));
    }

    #[test]
    fn wrapped_hint_keeps_label_alignment() {
        let text = sample().to_string();
        let lines: Vec<&str> = text.lines().collect();
        let hint_idx = match lines.iter().position(|l| l.starts_with("  hint:  ")) {
            Some(i) => i,
            None => unreachable!("no hint line in {text}"),
        };
        match lines.get(hint_idx + 1) {
            Some(cont) => {
                assert!(cont.starts_with("         "), "continuation indent: {cont}");
                assert!(!cont.contains(':'), "continuation is prose: {cont}");
            }
            None => unreachable!("hint should wrap in this sample: {text}"),
        }
    }

    #[test]
    fn wrap_never_exceeds_width_plus_indent_for_long_words() {
        let lines = wrap(
            "aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk",
            10,
            9,
        );
        for line in &lines {
            let visible = line.trim_start_matches(' ').len();
            // a single over-long word may exceed; multi-word lines must fit
            if line.trim_start_matches(' ').contains(' ') {
                assert!(visible <= 10 + 9, "line too long: {line}");
            }
        }
    }
}
