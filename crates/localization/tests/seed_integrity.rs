//! Seed integrity — the *shipped* tree must satisfy the localization
//! contract (doc 07 §6), not just the violation fixtures: `locales/`
//! (the reference catalog `en-US.toml` plus any future translations)
//! passes every catalog-audit rule, and the msgids referenced from the
//! real `product.toml` resolve in the reference catalog (rule 4).
//!
//! The violation fixtures proving that the rules *fire* live in
//! `tests/audit_proof_matrix.rs`; this file is the negative control over
//! the repository itself. CI wiring (`cargo xtask l10n-audit`, doc 07
//! §6.4) is card 1.E2 — until then this suite is the enforcement.

use std::path::{Path, PathBuf};

use localization::audit::{audit_catalogs, audit_catalogs_with_refs};
use localization::catalog::{REFERENCE_LOCALE, load_catalog_file};
use localization::key::MessageKey;
use localization::refs::extract_msgid_refs_from_file;

fn repo(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

#[test]
fn shipped_locales_tree_passes_every_catalog_audit_rule() {
    let issues = audit_catalogs(&repo("locales"));
    assert!(issues.is_empty(), "issues: {issues:?}");
}

#[test]
fn product_toml_msgids_resolve_in_the_reference_catalog() -> Result<(), Box<dyn std::error::Error>>
{
    // Rule 4 over the shipped tree: every msgid referenced by the real
    // product.toml (identity.description_key, window.title_idle_key —
    // doc 07 §2.1/§2.2) must exist in locales/en-US.toml.
    let refs = extract_msgid_refs_from_file(&repo("product.toml"))?.unwrap_or_default();
    assert_eq!(refs.len(), 2, "product.toml msgid refs: {refs:?}");
    let issues = audit_catalogs_with_refs(&repo("locales"), &refs);
    assert!(issues.is_empty(), "issues: {issues:?}");
    Ok(())
}

#[test]
fn phase1_seed_keys_are_present() -> Result<(), Box<dyn std::error::Error>> {
    let catalog = load_catalog_file(&repo("locales/en-US.toml"))?;
    assert_eq!(catalog.locale().as_str(), REFERENCE_LOCALE);
    // Representative keys from each seeded namespace (product, window,
    // xtask, ci — the Lead-approved Phase-1 scope, see locales/en-US.toml).
    for raw in [
        "product.description",
        "window.title.idle",
        "xtask.gate.status.pass",
        "ci.job.stub-suffix",
    ] {
        let key = MessageKey::new(raw)?;
        assert!(catalog.contains_key(&key), "missing seed key {raw}");
    }
    Ok(())
}
