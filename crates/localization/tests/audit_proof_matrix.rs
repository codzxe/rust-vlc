//! Audit proof matrix — the card 1.B3 acceptance criterion: **the five
//! doc 07 §6.4 CI audit rules enforced by fixtures.**
//!
//! Every rule is proven in both directions:
//!
//! | Rule | Violating fixture (must be flagged) | Exact expected issue | Negative control |
//! |------|------------------------------------|----------------------|------------------|
//! | 1 missing-key | `fixtures/locales-missing/de.toml` | 2 × `MissingKey` (fixture.greeting, fixture.items) | `locales-good` |
//! | 2 duplicate (in-file) | `fixtures/locales-duplicate/de.toml` | 1 × `CatalogUnreadable` containing "duplicate key" + line | — |
//! | 2 orphan (cross-file) | `fixtures/locales-orphan/de.toml` | 1 × `OrphanKey` (fixture.orphan) | `locales-good` |
//! | 3 placeholders | `fixtures/locales-placeholder/de.toml` | 1 × `PlaceholderMismatch` on fixture.progress (doc 07 §6.4 example: `{percent, number, percent}` vs `{percent}`) | `locales-good` |
//! | 4 reference-completeness | `fixtures/refs/unresolved.toml` | 1 × `UnresolvedReference` (fixture.no-such-key) | `refs/resolved.toml` |
//! | 5 plural conformance | `fixtures/locales-plural-ru/ru.toml` | 1 × `PluralNonConformance` (missing few/many) | `locales-good` |
//! | 5 plural (unused arm) | `fixtures/locales-plural-ja/ja.toml` | 1 × `PluralNonConformance` (ja has no `one`) | `locales-plural-unknown/xx.toml` |
//!
//! Each rule is exercised through its individual library function AND
//! through the combined `audit_catalogs`/`audit_catalogs_with_refs` entry
//! points that CI (card 1.E2 `cargo xtask l10n-audit`) will call.

use std::path::Path;

use localization::audit::{
    CatalogIssue, audit_catalogs, audit_catalogs_with_refs, audit_missing_keys, audit_orphan_keys,
    audit_placeholders, audit_plural_conformance, audit_references, catalog_coverage,
};
use localization::catalog::{Catalog, load_locales_dir};
use localization::locale::Locale;
use localization::refs::{extract_msgid_refs, extract_msgid_refs_from_file};

fn fixtures(relative: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(relative)
}

/// Load the reference (en-US) and the single other catalog of a fixture
/// `locales-*` directory. Fails the test (without panicking) if the
/// fixture is malformed — fixtures are test data, so a broken fixture is
/// itself a failure.
fn load_pair(dir: &str) -> Result<(Catalog, Catalog), Box<dyn std::error::Error>> {
    let loaded = load_locales_dir(&fixtures(dir));
    let reference = loaded
        .catalogs
        .iter()
        .find(|catalog| catalog.locale().as_str() == "en-US")
        .cloned();
    let other = loaded
        .catalogs
        .iter()
        .find(|catalog| catalog.locale().as_str() != "en-US")
        .cloned();
    let no_failures = loaded.failures.is_empty();
    match (reference, other, no_failures) {
        (Some(reference), Some(other), true) => Ok((reference, other)),
        _ => Err(format!(
            "fixture {dir} must contain en-US.toml and exactly one loadable other catalog \
             (failures: {:?})",
            loaded.failures
        )
        .into()),
    }
}

// ---------------------------------------------------------------------------
// Rule 1 — Missing-key detection
// ---------------------------------------------------------------------------

#[test]
fn rule1_missing_keys_are_flagged_exactly() -> Result<(), Box<dyn std::error::Error>> {
    let (reference, catalog) = load_pair("locales-missing")?;
    let issues = audit_missing_keys(&reference, &catalog);
    let keys: Vec<String> = issues
        .iter()
        .map(|issue| match issue {
            CatalogIssue::MissingKey { key, .. } => key.clone(),
            other => format!("unexpected issue: {other}"),
        })
        .collect();
    assert_eq!(
        keys,
        ["fixture.greeting".to_string(), "fixture.items".to_string()]
    );

    // Coverage metric (doc 07 §6.4 maintainer output).
    let coverage = catalog_coverage(&reference, &catalog);
    assert_eq!(coverage.covered, 4);
    assert_eq!(coverage.reference_total, 6);
    assert!((coverage.percent() - (400.0 / 6.0)).abs() < 1e-4);

    // The combined entry point reports the same two issues.
    let combined = audit_catalogs(&fixtures("locales-missing"));
    assert_eq!(combined.len(), 2);
    assert!(
        combined
            .iter()
            .all(|issue| matches!(issue, CatalogIssue::MissingKey { .. }))
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Rule 2 — Duplicate-key detection (in-file parse error) + orphan keys
// ---------------------------------------------------------------------------

#[test]
fn rule2_duplicate_keys_fail_at_parse_level() {
    let loaded = load_locales_dir(&fixtures("locales-duplicate"));
    // The duplicate is a load failure — TOML rejects it outright (doc 07
    // §6.2: "duplicate detection is parse-level for free").
    let duplicate_errors: Vec<String> = loaded
        .failures
        .iter()
        .filter(|(name, _)| name == "de.toml")
        .map(|(_, error)| error.to_string())
        .collect();
    assert_eq!(
        duplicate_errors.len(),
        1,
        "de.toml must fail to load exactly once (catalogs: {:?})",
        loaded
            .catalogs
            .iter()
            .map(|catalog| catalog.source_name().to_string())
            .collect::<Vec<_>>()
    );
    assert!(
        duplicate_errors
            .first()
            .map(|error| error.contains("duplicate key"))
            .unwrap_or(false),
        "expected duplicate-key error, got {duplicate_errors:?}"
    );
    // …while the reference still loads.
    assert!(
        loaded
            .catalogs
            .iter()
            .any(|catalog| catalog.locale().as_str() == "en-US")
    );
}

#[test]
fn rule2_duplicate_keys_surface_through_the_combined_audit() {
    let issues = audit_catalogs(&fixtures("locales-duplicate"));
    assert_eq!(issues.len(), 1, "issues: {issues:?}");
    assert!(
        matches!(&issues[0], CatalogIssue::CatalogUnreadable { file, error }
        if file == "de.toml" && error.contains("duplicate key"))
    );
    assert!(issues[0].to_string().contains("duplicate key"));
}

#[test]
fn rule2_orphan_keys_are_flagged_exactly() -> Result<(), Box<dyn std::error::Error>> {
    let (reference, catalog) = load_pair("locales-orphan")?;
    let issues = audit_orphan_keys(&reference, &catalog);
    assert_eq!(issues.len(), 1);
    assert!(matches!(&issues[0], CatalogIssue::OrphanKey { file, key }
        if file == "de.toml" && key == "fixture.orphan"));
    assert!(issues[0].to_string().contains("orphan key"));

    let combined = audit_catalogs(&fixtures("locales-orphan"));
    assert_eq!(combined.len(), 1);
    assert!(matches!(&combined[0], CatalogIssue::OrphanKey { .. }));
    Ok(())
}

// ---------------------------------------------------------------------------
// Rule 3 — Placeholder validation
// ---------------------------------------------------------------------------

#[test]
fn rule3_placeholder_mismatch_is_flagged_exactly() -> Result<(), Box<dyn std::error::Error>> {
    let (reference, catalog) = load_pair("locales-placeholder")?;
    let issues = audit_placeholders(&reference, &catalog);
    assert_eq!(issues.len(), 1, "issues: {issues:?}");
    match &issues[0] {
        CatalogIssue::PlaceholderMismatch {
            file,
            key,
            expected,
            actual,
        } => {
            assert_eq!(file, "de.toml");
            assert_eq!(key, "fixture.progress");
            assert!(expected.contains("number,percent"), "expected: {expected}");
            assert!(actual.contains("simple"), "actual: {actual}");
        }
        other => return Err(format!("wrong issue: {other}").into()),
    }
    assert!(issues[0].to_string().contains("placeholder set mismatch"));

    let combined = audit_catalogs(&fixtures("locales-placeholder"));
    assert_eq!(combined.len(), 1);
    assert!(matches!(
        &combined[0],
        CatalogIssue::PlaceholderMismatch { .. }
    ));
    Ok(())
}

// ---------------------------------------------------------------------------
// Rule 4 — Reference-completeness
// ---------------------------------------------------------------------------

#[test]
fn rule4_unresolved_references_are_flagged_exactly() -> Result<(), Box<dyn std::error::Error>> {
    let (reference, _) = load_pair("locales-good")?;
    let src = std::fs::read_to_string(fixtures("refs/unresolved.toml"))?;
    let refs = extract_msgid_refs("layouts/menus.toml", &src)?;
    assert_eq!(refs.len(), 2);

    let issues = audit_references(&reference, &refs);
    assert_eq!(issues.len(), 1);
    assert!(
        matches!(&issues[0], CatalogIssue::UnresolvedReference { file, field, key }
        if file == "layouts/menus.toml" && field == "label_key" && key == "fixture.no-such-key")
    );
    assert!(issues[0].to_string().contains("does not resolve"));

    // Through the combined entry point.
    let combined = audit_catalogs_with_refs(&fixtures("locales-good"), &refs);
    assert_eq!(combined.len(), 1);
    assert!(matches!(
        &combined[0],
        CatalogIssue::UnresolvedReference { .. }
    ));

    // Negative control: every reference resolves.
    let src = std::fs::read_to_string(fixtures("refs/resolved.toml"))?;
    let refs = extract_msgid_refs("product.toml", &src)?;
    assert!(audit_references(&reference, &refs).is_empty());
    assert!(audit_catalogs_with_refs(&fixtures("locales-good"), &refs).is_empty());
    Ok(())
}

#[test]
fn rule4_file_extraction_helper_reports_missing_files_as_none()
-> Result<(), Box<dyn std::error::Error>> {
    // Phase 1: most config sources do not exist yet; a missing file is
    // Ok(None), not an error and not a skipped check.
    let refs = extract_msgid_refs_from_file(&fixtures("refs/does-not-exist.toml"))?;
    assert!(refs.is_none());
    let refs = extract_msgid_refs_from_file(&fixtures("refs/unresolved.toml"))?;
    assert!(refs.is_some());
    Ok(())
}

// ---------------------------------------------------------------------------
// Rule 5 — Plural-rule conformance
// ---------------------------------------------------------------------------

#[test]
fn rule5_missing_plural_categories_are_flagged() -> Result<(), Box<dyn std::error::Error>> {
    let (_, ru) = load_pair("locales-plural-ru")?;
    assert_eq!(ru.locale().as_str(), "ru");
    let issues = audit_plural_conformance(&ru);
    assert_eq!(issues.len(), 1, "issues: {issues:?}");
    match &issues[0] {
        CatalogIssue::PluralNonConformance { file, key, detail } => {
            assert_eq!(file, "ru.toml");
            assert_eq!(key, "fixture.items");
            assert!(
                detail.contains("few") && detail.contains("many"),
                "detail: {detail}"
            );
        }
        other => return Err(format!("wrong issue: {other}").into()),
    }

    let combined = audit_catalogs(&fixtures("locales-plural-ru"));
    assert_eq!(combined.len(), 1);
    assert!(matches!(
        &combined[0],
        CatalogIssue::PluralNonConformance { .. }
    ));
    Ok(())
}

#[test]
fn rule5_unused_plural_categories_are_flagged() -> Result<(), Box<dyn std::error::Error>> {
    let (_, ja) = load_pair("locales-plural-ja")?;
    assert_eq!(ja.locale().as_str(), "ja");
    let issues = audit_plural_conformance(&ja);
    assert_eq!(issues.len(), 1, "issues: {issues:?}");
    match &issues[0] {
        CatalogIssue::PluralNonConformance { file, key, detail } => {
            assert_eq!(file, "ja.toml");
            assert_eq!(key, "fixture.items");
            assert!(detail.contains("one"), "detail: {detail}");
            assert!(detail.contains("ja"), "detail: {detail}");
        }
        other => return Err(format!("wrong issue: {other}").into()),
    }
    Ok(())
}

#[test]
fn rule5_unknown_language_enforces_only_the_parse_level_other()
-> Result<(), Box<dyn std::error::Error>> {
    let (_, xx) = load_pair("locales-plural-unknown")?;
    assert_eq!(xx.locale().as_str(), "xx");
    assert!(audit_plural_conformance(&xx).is_empty());

    let combined = audit_catalogs(&fixtures("locales-plural-unknown"));
    assert!(combined.is_empty(), "issues: {combined:?}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Negative control + reference-missing guard
// ---------------------------------------------------------------------------

#[test]
fn a_conformant_tree_produces_zero_issues() {
    let issues = audit_catalogs(&fixtures("locales-good"));
    assert!(issues.is_empty(), "issues: {issues:?}");
}

#[test]
fn missing_reference_catalog_is_reported() {
    // An empty directory has no en-US.toml: the audit must say so instead
    // of silently passing.
    let empty = std::env::temp_dir().join(format!("lumen-l10n-empty-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&empty);
    let _ = std::fs::create_dir(&empty);
    let issues = audit_catalogs(&empty);
    let _ = std::fs::remove_dir_all(&empty);
    assert_eq!(issues.len(), 1);
    assert!(matches!(&issues[0], CatalogIssue::ReferenceMissing { .. }));
}

#[test]
fn missing_locales_directory_is_reported() {
    let issues = audit_catalogs(&fixtures("does-not-exist"));
    assert!(!issues.is_empty());
    assert!(
        issues
            .iter()
            .any(|issue| matches!(issue, CatalogIssue::CatalogUnreadable { .. }))
    );
}

// ---------------------------------------------------------------------------
// Fixture data sanity (a broken fixture must fail loudly, not silently
// pass the matrix)
// ---------------------------------------------------------------------------

#[test]
fn fixture_reference_catalogs_are_identical_across_directories()
-> Result<(), Box<dyn std::error::Error>> {
    let canonical = std::fs::read_to_string(fixtures("locales-good/en-US.toml"))?;
    for dir in [
        "locales-missing",
        "locales-duplicate",
        "locales-orphan",
        "locales-placeholder",
        "locales-plural-ru",
        "locales-plural-ja",
        "locales-plural-unknown",
    ] {
        let reference = std::fs::read_to_string(fixtures(dir).join("en-US.toml"))?;
        assert_eq!(reference, canonical, "reference drifted in {dir}");
    }
    // …and the canonical reference parses cleanly.
    Catalog::from_toml_str(Locale::parse("en-US")?, "en-US.toml", &canonical)?;
    Ok(())
}
