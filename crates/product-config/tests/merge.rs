//! Precedence merge v1 (doc 07 §4.4 + §4.5 matrix rows in v1 scope:
//! product defaults + user) and the extension-point behaviors later cards
//! rely on (layer ordering, deep table merge, array replace, determinism).

mod common;

use std::collections::BTreeMap;

use common::{load_root_product, root_schema};
use product_config::{
    Diagnostic, Layer, LayerSource, PartialProduct, ProductConfig, Stage, layer, load_partial_str,
    render_all, resolve_layers,
};

fn base() -> ProductConfig {
    match load_root_product() {
        Ok(c) => c,
        Err(diags) => unreachable!("{}", render_all(&diags)),
    }
}

fn user_layer(src: &str) -> PartialProduct {
    match load_partial_str("user.toml", src, &root_schema()) {
        Ok(p) => p,
        Err(diags) => unreachable!("user layer must load\n{}", render_all(&diags)),
    }
}

fn resolve_user(
    base: &ProductConfig,
    user: &PartialProduct,
) -> Result<ProductConfig, Vec<Diagnostic>> {
    resolve_layers(
        base,
        &[LayerSource::new(Layer::User, "user.toml", user.clone())],
        &root_schema(),
    )
}

/// T1 — only the product layer: the product default resolves.
#[test]
fn t1_only_product_layer_resolves_product_defaults() {
    let base = base();
    let resolved = match resolve_layers(&base, &[], &root_schema()) {
        Ok(r) => r,
        Err(diags) => unreachable!("{}", render_all(&diags)),
    };
    assert_eq!(resolved, base);
    assert_eq!(resolved.window.width, 1280);
    assert_eq!(resolved.defaults.theme, "dark");
}

/// T4 — user overrides product (and lower layers, absent in v1).
#[test]
fn t4_user_overrides_product() {
    let base = base();
    let user = user_layer(
        "[window]\nwidth = 1920\n\n[telemetry]\nenabled = true\n\n[playback]\nstart_paused = true\n",
    );
    let resolved = match resolve_user(&base, &user) {
        Ok(r) => r,
        Err(diags) => unreachable!("{}", render_all(&diags)),
    };
    assert_eq!(resolved.window.width, 1920);
    assert!(resolved.telemetry.enabled);
    assert!(resolved.playback.start_paused);
    // Untouched keys fall through to the product layer.
    assert_eq!(resolved.window.height, 720);
    assert_eq!(resolved.defaults.locale, "en-US");
    assert_eq!(resolved.identity.name, "Lumen");
}

/// T10 — unknown key in the user layer: startup error with did-you-mean.
#[test]
fn t10_unknown_key_in_user_layer_is_an_error_with_hint() {
    let schema = root_schema();
    let diags = match load_partial_str("user.toml", "[window]\nwidht = 100\n", &schema) {
        Ok(_) => unreachable!("typo must be rejected"),
        Err(d) => d,
    };
    let hit = diags.iter().find(|d| d.key_path == "window.widht");
    let d = match hit {
        Some(d) => d,
        None => unreachable!("no window.widht diagnostic\n{}", render_all(&diags)),
    };
    assert!(d.rule.contains("unknown key"), "{}", d);
    assert!(d.hint.contains("did you mean `width`"), "{}", d);
    assert_eq!(d.file, "user.toml");
    assert_eq!(d.line, Some(2), "{}", d);
}

/// T12 — out-of-range value in the user layer: error with file + line.
#[test]
fn t12_out_of_range_in_user_layer_is_an_error() {
    let schema = root_schema();
    let diags = match load_partial_str("user.toml", "[window]\nwidth = 100\n", &schema) {
        Ok(_) => unreachable!("out-of-range user value must be rejected"),
        Err(d) => d,
    };
    let hit = diags.iter().find(|d| d.key_path == "window.width");
    let d = match hit {
        Some(d) => d,
        None => unreachable!("no window.width diagnostic\n{}", render_all(&diags)),
    };
    assert!(d.rule.contains("must be >= 480"), "{}", d);
    assert_eq!(d.file, "user.toml");
    assert_eq!(d.line, Some(2), "{}", d);
}

/// T13 — array policy `replace`: the higher layer's list fully replaces.
#[test]
fn t13_array_policy_replace() {
    let base = base();
    let user = user_layer("[playback]\npreferred_audio_languages = [\"fr\", \"de\"]\n");
    let resolved = match resolve_user(&base, &user) {
        Ok(r) => r,
        Err(diags) => unreachable!("{}", render_all(&diags)),
    };
    assert_eq!(
        resolved.playback.preferred_audio_languages,
        vec!["fr", "de"]
    );
}

/// T15 — two layers agree: the value resolves once, no double-apply artifacts.
#[test]
fn t15_layers_agree_resolve_once() {
    let base = base();
    let user = user_layer("[window]\nwidth = 1280\n");
    let resolved = match resolve_user(&base, &user) {
        Ok(r) => r,
        Err(diags) => unreachable!("{}", render_all(&diags)),
    };
    assert_eq!(resolved, base);
}

/// T16 — reset (removing the key from the user layer) exposes the
/// next-lower value again.
#[test]
fn t16_reset_returns_the_next_lower_value() {
    let base = base();
    let with_override = user_layer("[window]\nwidth = 1920\n");
    let resolved = match resolve_user(&base, &with_override) {
        Ok(r) => r,
        Err(diags) => unreachable!("{}", render_all(&diags)),
    };
    assert_eq!(resolved.window.width, 1920);
    // `config.reset-setting` removes the key from the user layer (doc 07
    // §4.4 rule 5): resolve without it.
    let after_reset = match resolve_user(&base, &user_layer("[playback]\nstart_paused = true\n")) {
        Ok(r) => r,
        Err(diags) => unreachable!("{}", render_all(&diags)),
    };
    assert_eq!(after_reset.window.width, base.window.width);
}

/// §4.4 rule 1 — deep merge for tables: a nested leaf override leaves its
/// sibling keys alone.
#[test]
fn deep_merge_replaces_only_the_named_leaf() {
    let base = base();
    let user = user_layer(
        "[identity.ids.windows]\nmsi_upgrade_code = \"00000000-1111-2222-3333-444444444444\"\n",
    );
    let resolved = match resolve_user(&base, &user) {
        Ok(r) => r,
        Err(diags) => unreachable!("{}", render_all(&diags)),
    };
    assert_eq!(
        resolved.identity.ids.windows.msi_upgrade_code,
        "00000000-1111-2222-3333-444444444444"
    );
    assert_eq!(
        resolved.identity.ids.windows.app_user_model_id,
        "dev.lumen.Lumen"
    );
    assert_eq!(resolved.identity.ids.reverse_dns, "dev.lumen");
    assert_eq!(resolved.identity.name, "Lumen");
}

/// The features table merges per key (deep merge), not wholesale.
#[test]
fn features_table_merges_per_key() {
    let base = base();
    let mut features = BTreeMap::new();
    features.insert("library".to_string(), true);
    let user = PartialProduct {
        features: Some(features),
        ..PartialProduct::default()
    };
    let resolved = match resolve_user(&base, &user) {
        Ok(r) => r,
        Err(diags) => unreachable!("{}", render_all(&diags)),
    };
    assert_eq!(resolved.features.get("library"), Some(&true));
    assert_eq!(resolved.features.get("core-playback"), Some(&true));
    assert_eq!(resolved.features.get("streaming-http"), Some(&false));
}

/// §4.4 rule 7 — determinism: the merge is a pure function of the ordered
/// layer list; the resolved snapshot is byte-identical across runs
/// (doc 07 §10.2, 100 iterations).
#[test]
fn merge_is_deterministic_across_runs() {
    let base = base();
    let user = user_layer("[window]\nwidth = 1920\n[playback]\nstart_paused = true\n");
    let mut snapshots: Vec<String> = Vec::new();
    for _ in 0..100 {
        let resolved = match resolve_user(&base, &user) {
            Ok(r) => r,
            Err(diags) => unreachable!("{}", render_all(&diags)),
        };
        match serde_json::to_string(&resolved) {
            Ok(json) => snapshots.push(json),
            Err(err) => unreachable!("resolved config must serialize: {err}"),
        }
    }
    let first = match snapshots.first() {
        Some(s) => s,
        None => unreachable!("no snapshots"),
    };
    assert!(snapshots.iter().all(|s| s == first), "snapshots diverged");
}

/// Programmatic partials are validated too (§4.4 rule 4 applies to every
/// layer source, not only file-loaded ones).
#[test]
fn programmatic_partial_out_of_range_is_rejected() {
    let base = base();
    let window = product_config::partial::PartialWindow {
        width: Some(100),
        ..Default::default()
    };
    let user = PartialProduct {
        window: Some(window),
        ..PartialProduct::default()
    };
    let diags = match resolve_user(&base, &user) {
        Ok(_) => unreachable!("must be rejected"),
        Err(d) => d,
    };
    let hit = diags.iter().find(|d| d.key_path == "window.width");
    let d = match hit {
        Some(d) => d,
        None => unreachable!("no window.width diagnostic\n{}", render_all(&diags)),
    };
    assert!(d.rule.contains("must be >= 480"), "{}", d);
    assert_eq!(d.file, "user.toml");
}

/// A cross-field violation that only appears after layering is blamed on
/// the layer file that set the offending key.
#[test]
fn cross_field_violation_after_merge_is_blamed_on_the_layer() {
    let base = base();
    // min_width = 2000 is structurally legal (minimum 320, no maximum) but
    // breaks min_width < width once layered over the product's width = 1280.
    let user = user_layer("[window]\nmin_width = 2000\n");
    let diags = match resolve_user(&base, &user) {
        Ok(_) => unreachable!("must be rejected"),
        Err(d) => d,
    };
    let hit = diags.iter().find(|d| d.key_path == "window.min_width");
    let d = match hit {
        Some(d) => d,
        None => unreachable!("no window.min_width diagnostic\n{}", render_all(&diags)),
    };
    assert!(d.rule.contains("must be < window.width"), "{}", d);
    assert_eq!(d.stage, Stage::Merge);
    assert_eq!(d.file, "user.toml", "{}", d);
    assert!(d.hint.contains("layering"), "{}", d);
}

/// The layer list must be ordered lowest → highest (doc 07 §4.1).
#[test]
fn layer_list_must_be_ordered() {
    let base = base();
    let empty = PartialProduct::default();
    let layers = vec![
        LayerSource::new(Layer::User, "user.toml", empty.clone()),
        LayerSource::new(Layer::Profile, "profile.toml", empty),
    ];
    let diags = match resolve_layers(&base, &layers, &root_schema()) {
        Ok(_) => unreachable!("out-of-order layers must be rejected"),
        Err(d) => d,
    };
    let hit = diags.iter().find(|d| d.key_path == "<layer-list>");
    let d = match hit {
        Some(d) => d,
        None => unreachable!("no layer-list diagnostic\n{}", render_all(&diags)),
    };
    assert!(d.rule.contains("lowest"), "{}", d);
    assert!(
        d.found.contains("user") && d.found.contains("profile"),
        "{}",
        d
    );
}

/// The product layer is the base, not a list entry.
#[test]
fn product_layer_is_not_a_list_entry() {
    let base = base();
    let layers = vec![LayerSource::new(
        Layer::Product,
        "product.toml",
        PartialProduct::default(),
    )];
    let diags = match resolve_layers(&base, &layers, &root_schema()) {
        Ok(_) => unreachable!("product-as-entry must be rejected"),
        Err(d) => d,
    };
    assert!(
        diags.iter().any(|d| d.rule.contains("resolution base")),
        "{}",
        render_all(&diags)
    );
}

/// The v1 convenience `layer()` = product defaults + one user layer.
#[test]
fn layer_convenience_is_product_plus_user() {
    let base = base();
    let user = user_layer("[window]\nwidth = 1600\n");
    let resolved = match layer(&base, &user, &root_schema()) {
        Ok(r) => r,
        Err(diags) => unreachable!("{}", render_all(&diags)),
    };
    assert_eq!(resolved.window.width, 1600);
    assert_eq!(resolved.window.height, base.window.height);
}
