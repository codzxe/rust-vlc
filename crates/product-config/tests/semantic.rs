//! Semantic-pass fixtures (doc 07 §3.3 cross-field, injected enums, catalog
//! keys, F1/F2, path existence + magic bytes) and the clean pass over the
//! committed root `product.toml`.

mod common;

use std::path::PathBuf;

use common::{for_key, load_fixture, load_root_product, marker_line, read_fixture};
use product_config::{FeatureRegistry, SemanticInputs, Stage, render_all, validate_semantics};

fn semantic_diagnostics(name: &str, inputs: &SemanticInputs) -> Vec<product_config::Diagnostic> {
    let config = match load_fixture(name) {
        Ok(c) => c,
        Err(diags) => unreachable!(
            "{name} must pass structural validation\n{}",
            render_all(&diags)
        ),
    };
    let src = read_fixture(name);
    let lines = product_config::lines::LineIndex::new(&src);
    validate_semantics(name, &config, inputs, &lines)
}

fn one<'a>(
    diags: &'a [product_config::Diagnostic],
    key: &str,
    name: &str,
) -> &'a product_config::Diagnostic {
    let hits = for_key(diags, key);
    assert_eq!(
        hits.len(),
        1,
        "{name}: expected exactly one {key}\n{}",
        render_all(diags)
    );
    match hits.first() {
        Some(d) => d,
        None => unreachable!("{name}: no {key} diagnostic"),
    }
}

// --- cross-field rules ----------------------------------------------------------

#[test]
fn cross_field_min_width_must_be_below_width() {
    let name = "cross-field-min-width.toml";
    let src = read_fixture(name);
    let diags = semantic_diagnostics(name, &SemanticInputs::none());
    let d = one(&diags, "window.min_width", name);
    assert_eq!(d.stage, Stage::Semantic);
    assert!(d.rule.contains("must be < window.width"), "{}", d);
    assert_eq!(d.found, "1600");
    assert_eq!(d.line, marker_line(&src, "min_width = 1600"), "{}", d);
}

#[test]
fn cross_field_schema_version_must_match_migrations() {
    let name = "cross-field-schema-version.toml";
    let diags = semantic_diagnostics(name, &SemanticInputs::none());
    let d = one(&diags, "schema_version", name);
    assert!(d.rule.contains("highest migration target"), "{}", d);
    assert_eq!(d.found, "2");
    assert_eq!(d.expected, "1");
}

#[test]
fn cross_field_telemetry_requires_privacy_url() {
    let name = "cross-field-telemetry-privacy.toml";
    let diags = semantic_diagnostics(name, &SemanticInputs::none());
    let d = one(&diags, "telemetry.enabled", name);
    assert!(d.rule.contains("urls.privacy"), "{}", d);
    assert!(d.hint.contains("privacy policy"), "{}", d);
}

#[test]
fn id_consistency_rule_3_3_b() {
    let name = "id-consistency.toml";
    let src = read_fixture(name);
    let diags = semantic_diagnostics(name, &SemanticInputs::none());
    let d = one(&diags, "identity.ids.windows.app_user_model_id", name);
    assert!(d.rule.contains("PascalCase"), "{}", d);
    assert_eq!(d.expected, "dev.lumen.Lumen");
    assert_eq!(d.found, "\"wrong.id.Here\"");
    assert_eq!(
        d.line,
        marker_line(&src, "app_user_model_id = \"wrong.id.Here\""),
        "{}",
        d
    );
    // The other three platform ids are still consistent — no extra hits.
    assert!(for_key(&diags, "identity.ids.macos.bundle_id").is_empty());
}

#[test]
fn reserved_short_name_is_semantic() {
    let name = "reserved-short-name.toml";
    let diags = semantic_diagnostics(name, &SemanticInputs::none());
    let d = one(&diags, "identity.short_name", name);
    assert!(d.rule.contains("reserved"), "{}", d);
    assert_eq!(d.found, "\"vlc\"");
}

#[test]
fn migrations_inverted_entry_is_rejected() {
    let name = "migrations-inverted.toml";
    let diags = semantic_diagnostics(name, &SemanticInputs::none());
    let d = one(&diags, "migrations.entries.0.from", name);
    assert!(d.rule.contains("from must be < to"), "{}", d);
    // The chain also desyncs schema_version (max to = 2 != 1).
    let version = one(&diags, "schema_version", name);
    assert!(
        version.rule.contains("highest migration target"),
        "{}",
        version
    );
}

// --- injected enums (§3.3, listings supplied like config-validator would) --------

#[test]
fn injected_enum_theme_must_exist_in_themes_dir() {
    let name = "injected-enum-theme.toml";
    let inputs = SemanticInputs {
        themes: Some(vec!["dark".to_string(), "light".to_string()]),
        ..SemanticInputs::none()
    };
    let diags = semantic_diagnostics(name, &inputs);
    let d = one(&diags, "defaults.theme", name);
    assert!(d.rule.contains("themes/<id>.toml"), "{}", d);
    assert!(d.expected.contains("dark, light"), "{}", d);
    assert_eq!(d.found, "\"oled-dark\"");
}

#[test]
fn injected_enum_locale_must_exist_in_locales_dir() {
    let name = "injected-enum-locale.toml";
    let inputs = SemanticInputs {
        locales: Some(vec!["en-US".to_string()]),
        ..SemanticInputs::none()
    };
    let diags = semantic_diagnostics(name, &inputs);
    let d = one(&diags, "defaults.locale", name);
    assert!(d.rule.contains("locales/<tag>.toml"), "{}", d);
    assert_eq!(d.found, "\"xx-XX\"");
}

#[test]
fn injected_enum_profile_must_exist_in_profiles_dir() {
    let name = "injected-enum-profile.toml";
    let inputs = SemanticInputs {
        profiles: Some(vec!["default".to_string()]),
        ..SemanticInputs::none()
    };
    let diags = semantic_diagnostics(name, &inputs);
    let d = one(&diags, "defaults.profile", name);
    assert!(d.rule.contains("profiles/<id>.toml"), "{}", d);
    assert_eq!(d.found, "\"cinema\"");
}

#[test]
fn injected_enums_are_deferred_when_no_listing_is_supplied() {
    // The fixture is invalid for the listing above, but with no listing the
    // check honestly reports nothing (the tool supplies listings, doc 07
    // §3.4 stage 3).
    let name = "injected-enum-theme.toml";
    let diags = semantic_diagnostics(name, &SemanticInputs::none());
    assert!(
        for_key(&diags, "defaults.theme").is_empty(),
        "{}",
        render_all(&diags)
    );
}

// --- catalog-key existence --------------------------------------------------------

#[test]
fn title_idle_key_must_exist_in_the_reference_catalog() {
    let name = "catalog-missing-key.toml";
    let inputs = SemanticInputs {
        reference_catalog_keys: Some(vec![
            "product.description".to_string(),
            "window.title.idle".to_string(),
        ]),
        ..SemanticInputs::none()
    };
    let diags = semantic_diagnostics(name, &inputs);
    let d = one(&diags, "window.title_idle_key", name);
    assert!(d.rule.contains("reference catalog"), "{}", d);
    assert_eq!(d.found, "\"window.title.gone\"");
    // description_key is in the catalog: no diagnostic for it.
    assert!(for_key(&diags, "identity.description_key").is_empty());
}

// --- F1 / F2 feature-registry intersection ------------------------------------------

#[test]
fn f1_unknown_feature_id_is_rejected() {
    let name = "feature-unknown-f1.toml";
    let inputs = SemanticInputs {
        feature_registry: Some(FeatureRegistry::from_status([
            ("core-playback", true),
            ("local-file", true),
        ])),
        ..SemanticInputs::none()
    };
    let diags = semantic_diagnostics(name, &inputs);
    let d = one(&diags, "features.telepathy", name);
    assert!(d.rule.contains("rule F1"), "{}", d);
    assert!(d.hint.contains("config alone enables nothing"), "{}", d);
    assert_eq!(d.found, "\"telepathy\"");
}

#[test]
fn f2_exposed_but_unimplemented_feature_is_rejected() {
    let name = "feature-unimplemented-f2.toml";
    let inputs = SemanticInputs {
        feature_registry: Some(FeatureRegistry::from_status([
            ("core-playback", true),
            ("streaming-http", false),
        ])),
        ..SemanticInputs::none()
    };
    let diags = semantic_diagnostics(name, &inputs);
    let d = one(&diags, "features.streaming-http", name);
    assert!(d.rule.contains("rule F2"), "{}", d);
    assert!(d.expected.contains("= false"), "{}", d);
}

#[test]
fn f1_f2_are_deferred_when_no_registry_is_supplied() {
    let name = "feature-unknown-f1.toml";
    let diags = semantic_diagnostics(name, &SemanticInputs::none());
    assert!(
        for_key(&diags, "features.telepathy").is_empty(),
        "{}",
        render_all(&diags)
    );
}

// --- path existence + magic bytes ------------------------------------------------------

fn asset_base() -> PathBuf {
    common::repo_root().join("crates/product-config/tests/fixtures/assets")
}

#[test]
fn brand_icon_with_wrong_magic_bytes_is_rejected() {
    let name = "assets-wrong-magic.toml";
    let inputs = SemanticInputs {
        base_dir: Some(asset_base()),
        ..SemanticInputs::none()
    };
    let diags = semantic_diagnostics(name, &inputs);
    let d = one(&diags, "brand.icon_ico", name);
    assert!(d.rule.contains("ICO"), "{}", d);
    assert!(d.rule.contains("magic"), "{}", d);
    assert!(d.found.starts_with("file starts with"), "{}", d);
    // The other three assets are valid: exactly one diagnostic total.
    assert_eq!(diags.len(), 1, "{}", render_all(&diags));
}

#[test]
fn missing_brand_asset_is_rejected() {
    let name = "assets-missing.toml";
    let inputs = SemanticInputs {
        base_dir: Some(asset_base()),
        ..SemanticInputs::none()
    };
    let diags = semantic_diagnostics(name, &inputs);
    let d = one(&diags, "brand.logo_svg", name);
    assert!(d.rule.contains("must exist"), "{}", d);
    assert!(d.found.contains("missing.svg"), "{}", d);
}

// --- the committed root product.toml passes the in-document semantic rules -------------

#[test]
fn root_product_toml_passes_semantic_none() {
    let config = match load_root_product() {
        Ok(c) => c,
        Err(diags) => unreachable!("{}", render_all(&diags)),
    };
    let src = match std::fs::read_to_string(common::repo_root().join("product.toml")) {
        Ok(s) => s,
        Err(err) => unreachable!("cannot read root product.toml: {err}"),
    };
    let lines = product_config::lines::LineIndex::new(&src);
    let diags = validate_semantics("product.toml", &config, &SemanticInputs::none(), &lines);
    assert!(diags.is_empty(), "{}", render_all(&diags));
}
