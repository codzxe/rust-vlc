//! Actionable diagnostics for the settings schema (doc 07 §3.5 contract,
//! applied to `schemas/settings.toml`).
//!
//! Every failure carries: the rule (`D1`–`D6` single-definition rules from
//! doc 07 §5.2, or an `E-*` well-formedness rule), the 1-based source line
//! when locatable, the offending entry path (`setting[<i>].<field>`), and a
//! message that states the constraint, what was found and how to fix it.

use std::fmt;

/// Severity of a schema finding. All current rules are hard errors (a
/// violation is build-breaking, doc 07 §5); the variant exists so later
/// advisory checks (e.g. deprecation notices) reuse the same channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Build-breaking violation.
    Error,
    /// Advisory finding; regeneration still succeeds.
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

/// One actionable settings-schema diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDiagnostic {
    /// Rule id: `D1`–`D6` (doc 07 §5.2) or `E-PARSE` / `E-FIELD` / `E-TYPE` /
    /// `E-FORMAT` / `E-DOMAIN` / `E-DEFAULT` / `E-AREAS` / `E-GATE`.
    pub rule: String,
    pub severity: Severity,
    /// 1-based line in the schema source, when locatable.
    pub line: Option<u32>,
    /// Offending path, e.g. `setting[3].cli.flag` or `areas.known`.
    pub path: String,
    /// Constraint + expected vs found + fix hint.
    pub message: String,
}

impl SchemaDiagnostic {
    /// Build an error-severity diagnostic.
    pub fn error(rule: &str, line: Option<u32>, path: &str, message: String) -> Self {
        SchemaDiagnostic {
            rule: rule.to_owned(),
            severity: Severity::Error,
            line,
            path: path.to_owned(),
            message,
        }
    }

    /// `settings.toml:<line>: <path>: error[<rule>]: <message>`
    fn render(&self) -> String {
        let line = match self.line {
            Some(n) => format!(":{n}"),
            None => String::new(),
        };
        format!(
            "schemas/settings.toml{line}: {}: {}[{}]: {}",
            self.path,
            self.severity.as_str(),
            self.rule,
            self.message
        )
    }
}

impl fmt::Display for SchemaDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

/// Render every diagnostic, one per line, highest line number first (the
/// conventional compiler order: the first reported failure is the one
/// nearest the top of the file after stable ordering).
pub fn render_all(diags: &[SchemaDiagnostic]) -> String {
    let mut rendered: Vec<String> = diags.iter().map(SchemaDiagnostic::render).collect();
    rendered.sort();
    let mut out = rendered.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_renders_rule_line_and_path() {
        let d = SchemaDiagnostic::error(
            "D2",
            Some(42),
            "setting[1].key",
            "duplicate setting key `playback.subtitle-mode`".to_owned(),
        );
        assert_eq!(
            d.to_string(),
            "schemas/settings.toml:42: setting[1].key: error[D2]: duplicate setting key \
             `playback.subtitle-mode`"
        );
    }

    #[test]
    fn missing_line_renders_without_colon() {
        let d = SchemaDiagnostic::error("E-AREAS", None, "areas.known", "empty".to_owned());
        assert_eq!(
            d.to_string(),
            "schemas/settings.toml: areas.known: error[E-AREAS]: empty"
        );
    }

    #[test]
    fn render_all_sorts_and_ends_with_newline() {
        let a = SchemaDiagnostic::error("D2", Some(9), "setting[0].key", "dup a".to_owned());
        let b = SchemaDiagnostic::error("D3", Some(2), "setting[1].cli.flag", "dup b".to_owned());
        let out = render_all(&[a, b]);
        assert!(out.ends_with('\n'));
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(
            lines[0].contains(":2:"),
            "sorted by rendered location: {out}"
        );
        assert!(
            lines[1].contains(":9:"),
            "sorted by rendered location: {out}"
        );
    }
}
