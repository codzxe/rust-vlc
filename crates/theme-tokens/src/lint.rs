//! Semantic-only consumption lint (card 1.B4; doc 08 §3.2, §3.4).
//!
//! Doc 08 §3.2: *"Components reference semantic tokens only"*. The real
//! `ui/` enforcement is the ESLint/Stylelint rule set (doc 08 §3.4, Phase
//! 4); [`lint_source`] is the crate-side checker used by this crate's
//! tests and available to CI. Given any source text (CSS, TSX, Rust, …)
//! it flags:
//!
//! * **name-form references** to the primitive/alias layers — the rooted
//!   dotted path (`primitive.color.gray.900`, `alias.color.surface`) and
//!   the CSS custom-property form (`var(--color-gray-900)`). An alias
//!   CSS form that coincides with a semantic name (e.g. `--color-surface`
//!   exists as a semantic token) is a legitimate semantic reference and
//!   is *not* flagged; primitive CSS forms never coincide and always are.
//! * **raw color literals** (`#8f8f8f`, case-insensitive) whose value
//!   duplicates a primitive/alias color — the message names the semantic
//!   token(s) that carry the same value, if any.
//!
//! Line/column positions are 1-based (characters, not bytes).

use std::collections::HashMap;
use std::collections::HashSet;

use crate::model::{ModeValues, ResolvedValue, TokenSet};

/// How the semantic-only rule was violated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViolationKind {
    /// A rooted dotted path (`primitive.…` / `alias.…`).
    DottedPath,
    /// A CSS custom-property form of a non-semantic name
    /// (`--color-gray-900`) that is not a semantic token.
    CssName,
    /// A raw color literal duplicating a primitive/alias value.
    RawValue,
}

/// One lint violation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    /// 1-based line of the occurrence.
    pub line: usize,
    /// 1-based column (in characters) of the occurrence.
    pub column: usize,
    /// The offending text as it appears in the source (hex keeps its
    /// original case).
    pub snippet: String,
    /// How the rule was violated.
    pub kind: ViolationKind,
    /// Which non-semantic layer was referenced (`"primitive"`/`"alias"`).
    pub layer: &'static str,
    /// The layer-root-stripped name of the referenced token.
    pub token: String,
    /// Actionable message (includes the doc 08 §3.2 rule).
    pub message: String,
}

/// Check `source` against `tokens`: every hit is a place where a component
/// would consume a non-semantic token (or a raw value) instead of a
/// semantic one. Results are sorted by (line, column, snippet).
pub fn lint_source(tokens: &TokenSet, source: &str) -> Vec<Violation> {
    let mut violations: Vec<Violation> = Vec::new();
    let semantic_names: HashSet<&str> = tokens
        .semantic()
        .iter()
        .map(|t| t.css_name.as_str())
        .collect();
    let line_starts = line_start_offsets(source);

    // Name forms: rooted dotted path + non-shadowed CSS form.
    for entry in tokens.non_semantic() {
        let rooted = format!("{}.{}", entry.layer, entry.name);
        for (offset, matched) in occurrences(source, &rooted, Boundary::Dotted) {
            let (line, column) = position(source, &line_starts, offset);
            violations.push(Violation {
                line,
                column,
                snippet: matched,
                kind: ViolationKind::DottedPath,
                layer: entry.layer,
                token: entry.name.clone(),
                message: format!(
                    "{layer} token `{name}` referenced directly — components consume \
                     semantic tokens only (doc 08 §3.2)",
                    layer = entry.layer,
                    name = entry.name,
                ),
            });
        }
        let css_form = format!("--{}", entry.name.replace('.', "-"));
        if !semantic_names.contains(css_form.as_str()) {
            for (offset, matched) in occurrences(source, &css_form, Boundary::Css) {
                let (line, column) = position(source, &line_starts, offset);
                violations.push(Violation {
                    line,
                    column,
                    snippet: matched,
                    kind: ViolationKind::CssName,
                    layer: entry.layer,
                    token: entry.name.clone(),
                    message: format!(
                        "`{css_form}` is the {layer} token `{name}`, not a semantic \
                         token — components consume semantic tokens only (doc 08 §3.2)",
                        layer = entry.layer,
                        name = entry.name,
                    ),
                });
            }
        }
    }

    // Raw color literals: map each distinct non-semantic color to the
    // semantic CSS names carrying the same value (for the suggestion).
    let mut semantic_by_hex: HashMap<String, Vec<&str>> = HashMap::new();
    for token in tokens.semantic() {
        for_each_color(&token.values, &mut |color| {
            let names = semantic_by_hex.entry(color.to_hex()).or_default();
            names.push(token.css_name.as_str());
        });
    }
    let mut raw_seen: HashSet<String> = HashSet::new();
    for entry in tokens.non_semantic() {
        for_each_color(&entry.values, &mut |color| {
            let hex = color.to_hex();
            if !raw_seen.insert(hex.clone()) {
                return; // report each distinct value under its first owner
            }
            for (offset, matched) in occurrences(source, &hex, Boundary::Hex) {
                let (line, column) = position(source, &line_starts, offset);
                let suggestion = semantic_by_hex.get(&hex).map(Vec::as_slice).map_or_else(
                    String::new,
                    |names| {
                        format!(
                            " — consume {} instead",
                            names
                                .iter()
                                .map(|n| format!("var({n})"))
                                .collect::<Vec<_>>()
                                .join(" or ")
                        )
                    },
                );
                let message = format!(
                    "raw color `{matched}` duplicates {layer} token `{name}`{suggestion} \
                     (doc 08 §3.2)",
                    layer = entry.layer,
                    name = entry.name,
                );
                violations.push(Violation {
                    line,
                    column,
                    snippet: matched,
                    kind: ViolationKind::RawValue,
                    layer: entry.layer,
                    token: entry.name.clone(),
                    message,
                });
            }
        });
    }

    violations.sort_by(|a, b| {
        (a.line, a.column, a.snippet.as_str()).cmp(&(b.line, b.column, b.snippet.as_str()))
    });
    violations
}

/// Byte offsets where each line starts (line 1 = offset 0).
fn line_start_offsets(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (offset, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(offset + 1);
        }
    }
    starts
}

/// 1-based (line, column) for a byte offset, via the precomputed line
/// starts.
fn position(source: &str, line_starts: &[usize], offset: usize) -> (usize, usize) {
    let line = line_starts.partition_point(|&start| start <= offset);
    let line_start = line_starts[line - 1];
    let column = source[line_start..offset].chars().count() + 1;
    (line, column)
}

/// Boundary flavor for [`occurrences`].
#[derive(Clone, Copy)]
enum Boundary {
    /// Dotted path: neighbors must not extend the path.
    Dotted,
    /// CSS custom property: neighbors must not extend the name.
    Css,
    /// Hex literal: case-insensitive, neighbors must not extend the literal.
    Hex,
}

impl Boundary {
    fn may_precede(self, c: char) -> bool {
        match self {
            Boundary::Dotted => c.is_ascii_alphanumeric() || c == '_' || c == '.',
            Boundary::Css | Boundary::Hex => {
                c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'
            }
        }
    }

    fn may_follow(self, c: char) -> bool {
        match self {
            Boundary::Dotted => c.is_ascii_alphanumeric() || c == '_' || c == '.',
            Boundary::Css | Boundary::Hex => c.is_ascii_alphanumeric() || c == '_' || c == '-',
        }
    }
}

/// All occurrences of `needle` in `source` honoring word boundaries;
/// yields (byte offset, matched text). `Hex` matches case-insensitively
/// (byte-preserving ASCII lowercasing, so offsets stay valid).
fn occurrences(source: &str, needle: &str, boundary: Boundary) -> Vec<(usize, String)> {
    let mut hits = Vec::new();
    if needle.is_empty() {
        return hits;
    }
    let (hay, needle) = if matches!(boundary, Boundary::Hex) {
        (source.to_ascii_lowercase(), needle.to_ascii_lowercase())
    } else {
        (source.to_owned(), needle.to_owned())
    };
    let mut from = 0;
    while let Some(found) = hay[from..].find(needle.as_str()) {
        let offset = from + found;
        let end = offset + needle.len();
        let extends_before = hay[..offset]
            .chars()
            .next_back()
            .map(|c| boundary.may_precede(c))
            .unwrap_or(false);
        let extends_after = hay[end..]
            .chars()
            .next()
            .map(|c| boundary.may_follow(c))
            .unwrap_or(false);
        if !extends_before && !extends_after {
            hits.push((offset, source[offset..end].to_owned()));
        }
        from = offset + needle.len();
    }
    hits
}

/// Visit every color carried by a (possibly per-mode) value.
fn for_each_color(values: &ModeValues, visit: &mut impl FnMut(crate::color::Rgb8)) {
    match values {
        ModeValues::Single(value) => {
            if let ResolvedValue::Color(color) = value {
                visit(*color);
            }
        }
        ModeValues::PerMode(pairs) => {
            for (_, value) in pairs {
                if let ResolvedValue::Color(color) = value {
                    visit(*color);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal-but-valid token source (one accent, two grays) for
    /// boundary tests that must not touch the real token file. The JSON
    /// contains `"#…` sequences, so the raw string needs `##` delimiters.
    const MINI_SOURCE: &str = r##"{
  "$extensions": { "lumen.tokens": {
      "modes": ["dark", "light"],
      "default-mode": "dark",
      "accents": ["orange"],
      "default-accent": "orange",
      "accent-primaries": { "orange": "{primitive.color.orange.500}" },
      "on-accent-candidates": ["{primitive.color.gray.800}"]
  } },
  "primitive": { "color": { "$type": "color",
      "orange": { "500": { "$value": "#D37918" } },
      "gray": { "800": { "$value": "#161616" }, "25": { "$value": "#EAEAEA" } }
  } },
  "alias": { "color": { "$type": "color",
      "accent": { "$value": "{primitive.color.orange.500}" },
      "accent.bright": { "$value": { "dark": "{primitive.color.orange.500}", "light": "{primitive.color.gray.25}" } },
      "accent.icon": { "$value": "{primitive.color.orange.500}" },
      "accent.strong": { "$value": "{primitive.color.orange.500}" },
      "accent.tinted": { "$value": "{primitive.color.gray.800}" },
      "content.on-accent": { "$value": "{primitive.color.gray.800}" },
      "surface": { "$value": { "dark": "{primitive.color.gray.800}", "light": "{primitive.color.gray.25}" } }
  } },
  "semantic": { "color": { "$type": "color",
      "accent": { "$value": "{alias.color.accent}" },
      "accent.bright": { "$value": "{alias.color.accent.bright}" },
      "accent.icon": { "$value": "{alias.color.accent.icon}" },
      "accent.strong": { "$value": "{alias.color.accent.strong}" },
      "accent.tinted": { "$value": "{alias.color.accent.tinted}" },
      "content.on-accent": { "$value": "{alias.color.content.on-accent}" },
      "surface": { "$value": "{alias.color.surface}" }
  } }
}
"##;

    fn mini() -> Result<TokenSet, crate::TokenError> {
        crate::parse(MINI_SOURCE)
    }

    #[test]
    fn dotted_paths_are_flagged_with_positions() -> Result<(), crate::TokenError> {
        let set = mini()?;
        let source = ".a { x: 1; }\n.b { y: var(--color-surface); }\n/* alias.color.surface */\n";
        let violations = lint_source(&set, source);
        let dotted: Vec<&Violation> = violations
            .iter()
            .filter(|v| v.kind == ViolationKind::DottedPath)
            .collect();
        assert_eq!(dotted.len(), 1, "violations: {violations:?}");
        assert_eq!(dotted[0].token, "color.surface");
        assert_eq!(dotted[0].layer, "alias");
        assert_eq!(dotted[0].line, 3);
        assert_eq!(dotted[0].column, 4);
        Ok(())
    }

    #[test]
    fn primitive_css_names_are_flagged_but_shadowed_alias_forms_are_not()
    -> Result<(), crate::TokenError> {
        let set = mini()?;
        let source =
            ".a { fill: var(--color-gray-800); }\n.b { background: var(--color-surface); }";
        let violations = lint_source(&set, source);
        let css: Vec<&Violation> = violations
            .iter()
            .filter(|v| v.kind == ViolationKind::CssName)
            .collect();
        assert_eq!(css.len(), 1, "violations: {violations:?}");
        assert_eq!(css[0].token, "color.gray.800");
        assert_eq!(css[0].layer, "primitive");
        assert_eq!(css[0].snippet, "--color-gray-800");
        // --color-surface coincides with the semantic token and is fine.
        assert!(
            violations
                .iter()
                .all(|v| !v.snippet.contains("--color-surface"))
        );
        Ok(())
    }

    #[test]
    fn raw_hex_is_flagged_case_insensitively_with_a_semantic_suggestion()
    -> Result<(), crate::TokenError> {
        let set = mini()?;
        let source = ".play { fill: #D37918; }";
        let violations = lint_source(&set, source);
        assert_eq!(violations.len(), 1, "violations: {violations:?}");
        assert_eq!(violations[0].kind, ViolationKind::RawValue);
        assert_eq!(violations[0].snippet, "#D37918");
        assert_eq!(violations[0].token, "color.orange.500");
        assert!(violations[0].message.contains("var(--color-accent)"));
        Ok(())
    }

    #[test]
    fn boundaries_reject_extensions_of_the_needle() -> Result<(), crate::TokenError> {
        let set = mini()?;
        let source = "xprimitive.color.gray.800 primitive.color.gray.8000 --color-gray-8001";
        let violations = lint_source(&set, source);
        assert!(
            violations.is_empty(),
            "extended occurrences must not match: {violations:?}"
        );
        Ok(())
    }

    #[test]
    fn clean_semantic_usage_produces_no_violations() -> Result<(), crate::TokenError> {
        let set = mini()?;
        let source =
            ".play {\n  fill: var(--color-accent);\n  color: var(--color-content-on-accent);\n}\n";
        assert!(lint_source(&set, source).is_empty());
        Ok(())
    }
}
