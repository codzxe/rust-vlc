//! Catalog auditing — the five doc 07 §6.4 CI rules, each as a library
//! function plus the combined directory audit (`audit_catalogs`, the
//! interface registered in doc 01 §2.3).
//!
//! | # | Rule | Function | Fixture proving violation detection |
//! |---|------|----------|-------------------------------------|
//! | 1 | Missing-key detection | [`audit_missing_keys`] | `tests/fixtures/locales-missing/de.toml` |
//! | 2 | Duplicate-key detection (in-file parse error) + orphan keys | [`Catalog::from_toml_str`] parse error / [`audit_orphan_keys`] | `locales-duplicate/de.toml` / `locales-orphan/de.toml` |
//! | 3 | Placeholder validation | [`audit_placeholders`] | `locales-placeholder/de.toml` |
//! | 4 | Reference-completeness | [`audit_references`] (+ [`crate::refs`]) | `fixtures/refs/unresolved.toml` |
//! | 5 | Plural-rule conformance | [`audit_plural_conformance`] | `locales-plural-ru/ru.toml`, `locales-plural-ja/ja.toml`, `locales-plural-unknown/xx.toml` |
//!
//! The full proof matrix (each rule's positive fixture → exact expected
//! issue, plus negative controls) lives in `tests/audit_proof_matrix.rs`.
//! CI wiring (`cargo xtask l10n-audit`, doc 07 §6.4) is card 1.E2; until
//! then this crate's test suite runs the same audits on the shipped tree.

use std::fmt;
use std::path::Path;

use crate::catalog::{Catalog, CatalogError, REFERENCE_LOCALE, load_locales_dir};
use crate::plural::{PluralCategory, categories_for_language};
use crate::refs::MsgidRef;

/// One catalog-audit finding. `Display` follows the actionable-diagnostic
/// shape of doc 07 §3.5 (what, where, rule, hint).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogIssue {
    /// The locales directory (or the reference catalog) is missing.
    ReferenceMissing { dir: String },
    /// A catalog file failed to load (syntax, non-string value, bad key,
    /// bad message format, invalid tag, …).
    CatalogUnreadable { file: String, error: String },
    /// Rule 1: a reference key is missing from a catalog.
    MissingKey { file: String, key: String },
    /// Rule 2: a catalog key absent from the reference (orphan).
    OrphanKey { file: String, key: String },
    /// Rule 3: the placeholder set (names + types) differs from the
    /// reference for a key.
    PlaceholderMismatch {
        file: String,
        key: String,
        expected: String,
        actual: String,
    },
    /// Rule 4: a msgid referenced from a config file does not resolve to
    /// a reference-catalog key.
    UnresolvedReference {
        file: String,
        field: String,
        key: String,
    },
    /// Rule 5: a plural message's arms do not conform to the locale's
    /// CLDR category set.
    PluralNonConformance {
        file: String,
        key: String,
        detail: String,
    },
}

impl fmt::Display for CatalogIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatalogIssue::ReferenceMissing { dir } => write!(
                f,
                "error[l10n]: {dir} — reference catalog {REFERENCE_LOCALE}.toml is missing; \
                 it is the source of truth for the key set (doc 07 §6.3)"
            ),
            CatalogIssue::CatalogUnreadable { file, error } => {
                write!(f, "error[l10n]: {file} — catalog failed to load: {error}")
            }
            CatalogIssue::MissingKey { file, key } => write!(
                f,
                "error[l10n]: {file}:{key} — key is missing from this catalog \
                 (rule 1: every catalog must cover the reference key set, doc 07 §6.4)"
            ),
            CatalogIssue::OrphanKey { file, key } => write!(
                f,
                "error[l10n]: {file}:{key} — key is absent from the reference catalog \
                 (rule 2: orphan key; add it to {REFERENCE_LOCALE}.toml first, doc 07 §6.4)"
            ),
            CatalogIssue::PlaceholderMismatch {
                file,
                key,
                expected,
                actual,
            } => write!(
                f,
                "error[l10n]: {file}:{key} — placeholder set mismatch \
                 (rule 3: names and types must equal the reference, doc 07 §6.4); \
                 reference: {expected}; catalog: {actual}"
            ),
            CatalogIssue::UnresolvedReference { file, field, key } => write!(
                f,
                "error[l10n]: {file}:{field} — msgid {key} does not resolve \
                 (rule 4: every referenced msgid must exist in {REFERENCE_LOCALE}.toml, doc 07 §6.4)"
            ),
            CatalogIssue::PluralNonConformance { file, key, detail } => write!(
                f,
                "error[l10n]: {file}:{key} — plural arms do not conform to the locale \
                 (rule 5: CLDR cardinal categories, doc 07 §6.4); {detail}"
            ),
        }
    }
}

/// Key coverage of a catalog against the reference (rule 1's maintainer
/// metric, doc 07 §6.4 `[PROPOSAL]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coverage {
    /// Reference keys present in the catalog.
    pub covered: usize,
    /// Total reference keys.
    pub reference_total: usize,
}

impl Coverage {
    /// Coverage as a percentage (0–100). A missing reference yields 0.
    pub fn percent(&self) -> f32 {
        if self.reference_total == 0 {
            return 0.0;
        }
        (self.covered as f32 / self.reference_total as f32) * 100.0
    }
}

/// Rule 1 — Missing-key detection: every catalog ⊇ the reference key set
/// (doc 07 §6.4). Returns one issue per missing key.
pub fn audit_missing_keys(reference: &Catalog, catalog: &Catalog) -> Vec<CatalogIssue> {
    reference
        .keys()
        .filter(|key| !catalog.contains_key(key))
        .map(|key| CatalogIssue::MissingKey {
            file: catalog.source_name().to_string(),
            key: key.as_str().to_string(),
        })
        .collect()
}

/// Rule 1 companion — the coverage percentage for maintainers.
pub fn catalog_coverage(reference: &Catalog, catalog: &Catalog) -> Coverage {
    let covered = reference
        .keys()
        .filter(|key| catalog.contains_key(key))
        .count();
    Coverage {
        covered,
        reference_total: reference.len(),
    }
}

/// Rule 2 (cross-file half) — Orphan keys: a catalog key absent from the
/// reference is an error. The in-file half (duplicate keys) is a TOML
/// parse error surfaced by [`Catalog::from_toml_str`] and reported as
/// [`CatalogIssue::CatalogUnreadable`] by [`audit_catalogs`].
pub fn audit_orphan_keys(reference: &Catalog, catalog: &Catalog) -> Vec<CatalogIssue> {
    catalog
        .keys()
        .filter(|key| !reference.contains_key(key))
        .map(|key| CatalogIssue::OrphanKey {
            file: catalog.source_name().to_string(),
            key: key.as_str().to_string(),
        })
        .collect()
}

/// Rule 3 — Placeholder validation: for each shared key, the placeholder
/// set (names + types) must equal the reference's exactly (doc 07 §6.4:
/// `{percent, number, percent}` in en-US and `{percent}` in `de` is a CI
/// failure).
pub fn audit_placeholders(reference: &Catalog, catalog: &Catalog) -> Vec<CatalogIssue> {
    let mut issues = Vec::new();
    for key in catalog.keys() {
        let (Some(reference_message), Some(catalog_message)) =
            (reference.get(key), catalog.get(key))
        else {
            // Missing keys are rule 1's job; orphans are rule 2's.
            continue;
        };
        let expected = reference_message.placeholder_set();
        let actual = catalog_message.placeholder_set();
        if expected != actual {
            issues.push(CatalogIssue::PlaceholderMismatch {
                file: catalog.source_name().to_string(),
                key: key.as_str().to_string(),
                expected: format_placeholder_set(&expected),
                actual: format_placeholder_set(&actual),
            });
        }
    }
    issues
}

fn format_placeholder_set(set: &std::collections::BTreeMap<String, &'static str>) -> String {
    if set.is_empty() {
        return "(no placeholders)".to_string();
    }
    let parts: Vec<String> = set
        .iter()
        .map(|(name, tag)| format!("{{{name}}} as {tag}"))
        .collect();
    parts.join(", ")
}

/// Rule 4 — Reference-completeness: every msgid referenced from the
/// declarative configs must resolve to a reference-catalog key
/// (doc 07 §6.4). References come from [`crate::refs`].
pub fn audit_references(reference: &Catalog, refs: &[MsgidRef]) -> Vec<CatalogIssue> {
    refs.iter()
        .filter(|r| !reference.contains_key(&r.key))
        .map(|r| CatalogIssue::UnresolvedReference {
            file: r.file.clone(),
            field: r.field.clone(),
            key: r.key.as_str().to_string(),
        })
        .collect()
}

/// Rule 5 — Plural-rule conformance: each `{n, plural, …}` message must
/// carry exactly the locale's CLDR cardinal categories (doc 07 §6.4).
///
/// For languages outside the Phase-1 category table only the universal
/// `other` category is required — extra arms are accepted, not flagged
/// (see [`crate::plural`] for the honest data-scope note).
pub fn audit_plural_conformance(catalog: &Catalog) -> Vec<CatalogIssue> {
    const OTHER_ONLY: &[PluralCategory] = &[PluralCategory::Other];
    let language = catalog
        .locale()
        .as_str()
        .split('-')
        .next()
        .unwrap_or_default()
        .to_string();
    let known = categories_for_language(&language);
    let required: &[PluralCategory] = known.unwrap_or(OTHER_ONLY);
    // Unknown languages: require `other`, tolerate extra arms. Known
    // languages: the arm set must equal the CLDR set exactly.
    let strict = known.is_some();
    let mut issues = Vec::new();
    for key in catalog.keys() {
        // get() returns Option<&Message>; missing keys are impossible here
        // (catalogs are fully populated maps) — skip defensively.
        let Some(message) = catalog.get(key) else {
            continue;
        };
        for (name, arms) in message.plural_arms_by_name() {
            let detail = plural_conformance_detail(&arms, required, &language, strict);
            if let Some(detail) = detail {
                issues.push(CatalogIssue::PluralNonConformance {
                    file: catalog.source_name().to_string(),
                    key: key.as_str().to_string(),
                    detail: format!("plural placeholder {name:?}: {detail}"),
                });
            }
        }
    }
    issues
}

fn plural_conformance_detail(
    arms: &[PluralCategory],
    required: &[PluralCategory],
    language: &str,
    strict: bool,
) -> Option<String> {
    let missing: Vec<&str> = required
        .iter()
        .filter(|category| !arms.contains(category))
        .map(|category| category.as_str())
        .collect();
    if !missing.is_empty() {
        return Some(format!(
            "missing required categor{} {} for language {language:?}",
            if missing.len() == 1 { "y" } else { "ies" },
            missing.join("/")
        ));
    }
    if strict {
        let unexpected: Vec<&str> = arms
            .iter()
            .filter(|category| !required.contains(category))
            .map(|category| category.as_str())
            .collect();
        if !unexpected.is_empty() {
            return Some(format!(
                "categor{} {} not used by language {language:?}",
                if unexpected.len() == 1 { "y" } else { "ies" },
                unexpected.join("/")
            ));
        }
    }
    None
}

/// Run every catalog-audit rule over a `locales/` directory (doc 01 §2.3
/// interface sketch): loads the catalogs, finds the reference, and checks
/// rules 1, 2 (orphans + load errors, which include duplicate keys), 3,
/// and 5 for every catalog. Rule 4 needs references from outside the
/// directory — use [`audit_catalogs_with_refs`].
pub fn audit_catalogs(dir: &Path) -> Vec<CatalogIssue> {
    audit_catalogs_with_refs(dir, &[])
}

/// [`audit_catalogs`] plus rule 4: `refs` are the msgid references
/// extracted from the declarative configs (see [`crate::refs`]).
pub fn audit_catalogs_with_refs(dir: &Path, refs: &[MsgidRef]) -> Vec<CatalogIssue> {
    let mut issues = Vec::new();
    let loaded = load_locales_dir(dir);
    for (file, error) in &loaded.failures {
        issues.push(CatalogIssue::CatalogUnreadable {
            file: file.clone(),
            error: error.to_string(),
        });
    }
    let Some(reference) = loaded
        .catalogs
        .iter()
        .find(|catalog| catalog.locale().as_str() == REFERENCE_LOCALE)
    else {
        issues.push(CatalogIssue::ReferenceMissing {
            dir: dir.display().to_string(),
        });
        return issues;
    };
    // The reference itself must conform (rule 5) and resolve every
    // external msgid reference (rule 4); rules 1–3 against itself are
    // trivially empty.
    issues.extend(audit_plural_conformance(reference));
    issues.extend(audit_references(reference, refs));
    for catalog in &loaded.catalogs {
        if catalog.locale().as_str() == REFERENCE_LOCALE {
            continue;
        }
        issues.extend(audit_missing_keys(reference, catalog));
        issues.extend(audit_orphan_keys(reference, catalog));
        issues.extend(audit_placeholders(reference, catalog));
        issues.extend(audit_plural_conformance(catalog));
    }
    issues
}

/// Map a load failure to its structured issue (exposed for callers that
/// load catalogs themselves, e.g. the future `xtask l10n-audit`).
pub fn catalog_error_issue(file: &str, error: &CatalogError) -> CatalogIssue {
    CatalogIssue::CatalogUnreadable {
        file: file.to_string(),
        error: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Coverage, audit_missing_keys, audit_orphan_keys, audit_placeholders, audit_references,
        catalog_coverage,
    };
    use crate::catalog::Catalog;
    use crate::key::MessageKey;
    use crate::locale::Locale;
    use crate::refs::MsgidRef;

    const REFERENCE: &str = "\
\"fixture.simple\" = \"Plain text\"
\"fixture.greeting\" = \"Hello, {name}!\"
\"fixture.progress\" = \"Buffering… {percent, number, percent}\"
\"fixture.source\" = \"Opening {source}\"
\"fixture.items\" = \"{count, plural, one {# item} other {# items}}\"
\"fixture.two\" = \"Second key\"
";

    fn en_us() -> Result<Locale, Box<dyn std::error::Error>> {
        Ok(Locale::parse("en-US")?)
    }

    fn reference() -> Result<Catalog, Box<dyn std::error::Error>> {
        Ok(Catalog::from_toml_str(en_us()?, "en-US.toml", REFERENCE)?)
    }

    #[test]
    fn rule1_reports_every_missing_key() -> Result<(), Box<dyn std::error::Error>> {
        let reference = reference()?;
        let catalog = Catalog::from_toml_str(
            Locale::parse("de")?,
            "de.toml",
            "\"fixture.simple\" = \"Einfacher Text\"\n\"fixture.two\" = \"Zweiter Schlüssel\"\n",
        )?;
        let issues = audit_missing_keys(&reference, &catalog);
        let shown: Vec<String> = issues
            .iter()
            .map(|issue| match issue {
                crate::audit::CatalogIssue::MissingKey { key, .. } => key.clone(),
                other => format!("unexpected issue: {other}"),
            })
            .collect();
        assert_eq!(
            shown,
            [
                "fixture.greeting",
                "fixture.items",
                "fixture.progress",
                "fixture.source",
            ]
        );
        Ok(())
    }

    #[test]
    fn rule1_counts_missing_keys_correctly() -> Result<(), Box<dyn std::error::Error>> {
        let reference = reference()?;
        let catalog = Catalog::from_toml_str(
            Locale::parse("de")?,
            "de.toml",
            "\"fixture.simple\" = \"Einfacher Text\"\n\"fixture.two\" = \"Zweiter Schlüssel\"\n",
        )?;
        let issues = audit_missing_keys(&reference, &catalog);
        assert_eq!(issues.len(), 4); // greeting, progress, source, items
        let coverage = catalog_coverage(&reference, &catalog);
        assert_eq!(
            coverage,
            Coverage {
                covered: 2,
                reference_total: 6
            }
        );
        assert!((coverage.percent() - 100.0 * 2.0 / 6.0).abs() < 1e-4);
        Ok(())
    }

    #[test]
    fn rule2_reports_orphan_keys() -> Result<(), Box<dyn std::error::Error>> {
        let reference = reference()?;
        let mut src = String::from(REFERENCE);
        src.push_str("\"fixture.orphan\" = \"Verwaist\"\n");
        let catalog = Catalog::from_toml_str(Locale::parse("de")?, "de.toml", &src)?;
        let issues = audit_orphan_keys(&reference, &catalog);
        assert_eq!(issues.len(), 1);
        assert!(
            matches!(&issues[0], crate::audit::CatalogIssue::OrphanKey { file, key }
            if file == "de.toml" && key == "fixture.orphan")
        );
        assert!(issues[0].to_string().contains("orphan key"));
        Ok(())
    }

    #[test]
    fn rule3_reports_placeholder_mismatches() -> Result<(), Box<dyn std::error::Error>> {
        let reference = reference()?;
        // The doc 07 §6.4 example: {percent, number, percent} vs {percent}.
        let catalog = Catalog::from_toml_str(
            Locale::parse("de")?,
            "de.toml",
            "\
\"fixture.simple\" = \"Einfacher Text\"
\"fixture.greeting\" = \"Hallo, {name}!\"
\"fixture.progress\" = \"Puffer… {percent}\"
\"fixture.source\" = \"Öffne {source}\"
\"fixture.items\" = \"{count, plural, one {# Element} other {# Elemente}}\"
\"fixture.two\" = \"Zweiter Schlüssel\"
",
        )?;
        let issues = audit_placeholders(&reference, &catalog);
        assert_eq!(issues.len(), 1, "issues: {issues:?}");
        assert!(matches!(
            &issues[0],
            crate::audit::CatalogIssue::PlaceholderMismatch { file, key, expected, actual }
                if file == "de.toml"
                    && key == "fixture.progress"
                    && expected.contains("number,percent")
                    && actual.contains("simple")
        ));
        assert!(issues[0].to_string().contains("placeholder set mismatch"));
        Ok(())
    }

    #[test]
    fn rule4_reports_unresolved_references() -> Result<(), Box<dyn std::error::Error>> {
        let reference = reference()?;
        let key = MessageKey::new("fixture.no-such-key")?;
        let refs = vec![
            MsgidRef {
                file: "product.toml".to_string(),
                field: "description_key".to_string(),
                key: MessageKey::new("fixture.simple")?,
            },
            MsgidRef {
                file: "layouts/menus.toml".to_string(),
                field: "label_key".to_string(),
                key: key.clone(),
            },
        ];
        let issues = audit_references(&reference, &refs);
        assert_eq!(issues.len(), 1);
        assert!(matches!(
            &issues[0],
            crate::audit::CatalogIssue::UnresolvedReference { file, field, key }
                if file == "layouts/menus.toml" && field == "label_key" && key == "fixture.no-such-key"
        ));
        assert!(issues[0].to_string().contains("does not resolve"));
        Ok(())
    }

    #[test]
    fn a_perfect_catalog_produces_no_issues() -> Result<(), Box<dyn std::error::Error>> {
        let reference = reference()?;
        let catalog = Catalog::from_toml_str(Locale::parse("de")?, "de.toml", REFERENCE)?;
        assert!(audit_missing_keys(&reference, &catalog).is_empty());
        assert!(audit_orphan_keys(&reference, &catalog).is_empty());
        assert!(audit_placeholders(&reference, &catalog).is_empty());
        let coverage = catalog_coverage(&reference, &catalog);
        assert_eq!(coverage.percent(), 100.0);
        Ok(())
    }
}
