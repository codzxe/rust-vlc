//! Semantic-only consumption lint, fixture-driven (card 1.B4; doc 08 §3.2).
//!
//! `tests/fixtures/semantic_lint/violating.css` is a stand-in for component
//! code OUTSIDE this crate that references primitive/alias tokens directly
//! (dotted paths, CSS-form names, raw hex) — every such reference must FAIL
//! the check. `clean.css` consumes semantic tokens only and must pass.

use std::fs;
use std::path::PathBuf;

use theme_tokens::lint::ViolationKind;
use theme_tokens::lint::lint_source;
use theme_tokens::parse;

/// The Lumen workspace root (this file lives in crates/theme-tokens/tests).
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/semantic_lint")
        .join(name);
    Ok(fs::read_to_string(path)?)
}

fn parse_committed_source() -> Result<theme_tokens::TokenSet, Box<dyn std::error::Error>> {
    let path = workspace_root().join("themes/tokens.json");
    let source = fs::read_to_string(&path)?;
    Ok(parse(&source)?)
}

#[test]
fn violating_fixture_fails_the_check() -> Result<(), Box<dyn std::error::Error>> {
    let set = parse_committed_source()?;
    let text = fixture("violating.css")?;
    let violations = lint_source(&set, &text);

    // Raw hex duplicating primitive color.gray.300 (#8F8F8F).
    assert!(
        violations.iter().any(|v| v.kind == ViolationKind::RawValue
            && v.token == "color.gray.300"
            && v.snippet == "#8F8F8F"),
        "expected a raw-value violation for #8F8F8F, got: {violations:?}"
    );
    // CSS-form primitive name: var(--color-gray-900) → primitive color.gray.900.
    assert!(
        violations.iter().any(|v| v.kind == ViolationKind::CssName
            && v.layer == "primitive"
            && v.token == "color.gray.900"
            && v.snippet == "--color-gray-900"),
        "expected a primitive CSS-name violation, got: {violations:?}"
    );
    // CSS-form alias name: var(--stroke-width-icon) → alias stroke-width.icon.
    assert!(
        violations.iter().any(|v| v.kind == ViolationKind::CssName
            && v.layer == "alias"
            && v.token == "stroke-width.icon"),
        "expected an alias CSS-name violation, got: {violations:?}"
    );
    // Dotted primitive path: primitive.color.orange.500.
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::DottedPath
                && v.layer == "primitive"
                && v.token == "color.orange.500"),
        "expected a dotted primitive-path violation, got: {violations:?}"
    );
    // Dotted alias path: alias.color.surface.
    assert!(
        violations
            .iter()
            .any(|v| v.kind == ViolationKind::DottedPath
                && v.layer == "alias"
                && v.token == "color.surface"),
        "expected a dotted alias-path violation, got: {violations:?}"
    );
    // Every message cites the rule.
    assert!(
        violations.iter().all(|v| v.message.contains("doc 08 §3.2")),
        "every violation message must cite the rule: {violations:?}"
    );
    // Sorted output for determinism.
    let mut ordered = violations.clone();
    ordered.sort_by(|a, b| {
        (a.line, a.column, a.snippet.as_str()).cmp(&(b.line, b.column, b.snippet.as_str()))
    });
    assert_eq!(violations, ordered, "violations must be sorted");
    Ok(())
}

#[test]
fn clean_fixture_passes_the_check() -> Result<(), Box<dyn std::error::Error>> {
    let set = parse_committed_source()?;
    let text = fixture("clean.css")?;
    let violations = lint_source(&set, &text);
    assert!(
        violations.is_empty(),
        "semantic-only source must lint clean, got: {violations:?}"
    );
    Ok(())
}
