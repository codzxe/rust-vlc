//! Layered precedence resolution (doc 07 §4) — v1 scope: product defaults +
//! user overrides.
//!
//! The six layers of §4.1 are modeled by [`Layer`]; resolution takes an
//! ordered list of sparse overrides (lowest → highest) so the Phase-2+
//! profile/admin/flags/session cards *extend* this API with loaders and
//! layer-specific semantics (locks §4.3, session-scope routing §4.1) instead
//! of rewriting it. Merge rules per §4.4: tables deep-merge, scalars replace,
//! arrays replace (v1 policy), every layer's values are validated against the
//! same schema entries *before* merging, unknown keys are errors, and the
//! merge is a pure function of the ordered layer list.

use std::collections::BTreeMap;

use crate::diagnostic::{Diagnostic, Severity, Stage};
use crate::lines::LineIndex;
use crate::model::ProductConfig;
use crate::partial::PartialProduct;
use crate::schema::{RequiredMode, Schema};
use crate::semantic;

/// The six precedence layers (doc 07 §4.1, lowest → highest; doc 01 §2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    /// `product.toml` + settings defaults — the `base` of every resolution.
    Product,
    /// `profiles/<id>.toml` (Phase 2+ loader).
    Profile,
    /// Managed config + lock list (Phase 2+ loader, §4.3).
    Admin,
    /// User preferences (§4.2 layer 4) — the v1 override layer.
    User,
    /// Command-line flags (Phase 2+ loader).
    Flags,
    /// In-memory per-session values (Phase 2+ loader).
    Session,
}

impl Layer {
    /// Canonical precedence rank, 1 (product) … 6 (session), doc 07 §4.1.
    pub fn precedence(self) -> u8 {
        match self {
            Layer::Product => 1,
            Layer::Profile => 2,
            Layer::Admin => 3,
            Layer::User => 4,
            Layer::Flags => 5,
            Layer::Session => 6,
        }
    }
}

/// One layer contribution to a resolution.
#[derive(Debug, Clone)]
pub struct LayerSource {
    /// Which precedence layer this document belongs to.
    pub layer: Layer,
    /// File name used in diagnostics for this layer's values.
    pub file: String,
    /// The sparse overrides (only set keys take effect).
    pub partial: PartialProduct,
}

impl LayerSource {
    pub fn new(layer: Layer, file: impl Into<String>, partial: PartialProduct) -> Self {
        LayerSource {
            layer,
            file: file.into(),
            partial,
        }
    }
}

/// Remove `null` members from objects (TOML has no null; these are the
/// `None` fields of a serialized [`PartialProduct`] — absent keys, not null
/// values).
fn strip_nulls(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.into_iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k, strip_nulls(v)))
                .collect(),
        ),
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(strip_nulls).collect())
        }
        other => other,
    }
}

/// v1 convenience: product defaults + one user layer (card 1.B1 scope).
pub fn layer(
    base: &ProductConfig,
    user: &PartialProduct,
    schema: &Schema,
) -> Result<ProductConfig, Vec<Diagnostic>> {
    resolve_layers(
        base,
        &[LayerSource::new(Layer::User, "user", user.clone())],
        schema,
    )
}

/// Resolve `layers` (ordered lowest → highest) over `base` (doc 07 §4.4).
///
/// Stage 1 validates every layer's present values against the same schema
/// entries (rule 4); stage 2 merges (tables deep, scalars/arrays replace);
/// stage 3 re-runs the pure cross-field rules over the resolved config so a
/// combination that is legal per-key but illegal as a whole (e.g. a user
/// `min_width` above the product `width`) fails with an actionable
/// diagnostic attributed to the highest layer that set an involved key.
pub fn resolve_layers(
    base: &ProductConfig,
    layers: &[LayerSource],
    schema: &Schema,
) -> Result<ProductConfig, Vec<Diagnostic>> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    // Layer-list sanity: strictly increasing canonical precedence (§4.1).
    for (i, source) in layers.iter().enumerate() {
        if source.layer == Layer::Product {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                stage: Stage::Merge,
                file: source.file.clone(),
                line: None,
                key_path: "<layer-list>".to_string(),
                rule: "the product layer is the resolution base, not a list entry".to_string(),
                expected: "layers starting at Profile or above".to_string(),
                found: format!("layers[{i}] = Layer::Product"),
                hint: "pass product.toml as `base` and only overriding layers in the list (doc 07 \u{a7}4.4).".to_string(),
            });
        }
        if i > 0 {
            let prev = match layers.get(i - 1) {
                Some(p) => p,
                None => continue,
            };
            if source.layer.precedence() <= prev.layer.precedence() {
                diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    stage: Stage::Merge,
                    file: source.file.clone(),
                    line: None,
                    key_path: "<layer-list>".to_string(),
                    rule: "layers must be ordered lowest \u{2192} highest precedence (doc 07 \u{a7}4.1)".to_string(),
                    expected: format!(
                        "precedence > {} (the previous layer's {})",
                        prev.layer.precedence(),
                        layer_name(prev.layer)
                    ),
                    found: format!(
                        "{} ({}) after {} ({})",
                        layer_name(source.layer),
                        source.layer.precedence(),
                        layer_name(prev.layer),
                        prev.layer.precedence()
                    ),
                    hint: "order the list product \u{2192} profile \u{2192} admin \u{2192} user \u{2192} flags \u{2192} session; the merge is a pure function of that order (doc 07 \u{a7}4.4 rule 7).".to_string(),
                });
            }
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    // Stage 1: per-layer validation of every present value (§4.4 rule 4).
    let empty_lines = LineIndex::new("");
    let mut provenance: BTreeMap<String, String> = BTreeMap::new();
    for source in layers {
        let json = match serde_json::to_value(&source.partial) {
            Ok(j) => strip_nulls(j),
            Err(err) => {
                diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    stage: Stage::Merge,
                    file: source.file.clone(),
                    line: None,
                    key_path: "<document>".to_string(),
                    rule: "the layer document must be serializable for validation".to_string(),
                    expected: "a partial product.toml shape".to_string(),
                    found: err.to_string(),
                    hint: "this is a product-config crate bug; please report it with the offending layer.".to_string(),
                });
                continue;
            }
        };
        record_provenance(&json, "", &source.file, &mut provenance);
        diagnostics.extend(schema.validate(
            &source.file,
            &json,
            &empty_lines,
            RequiredMode::Partial,
        ));
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    // Stage 2: merge, lowest → highest.
    let mut resolved = base.clone();
    for source in layers {
        merge_over(&mut resolved, &source.partial);
    }

    // Stage 3: pure cross-field rules over the resolved config (§3.3),
    // attributed to the highest layer that set an involved key.
    let resolved_lines = LineIndex::new("");
    let mut ctx = semantic::Ctx {
        file: "product.toml",
        lines: &resolved_lines,
        out: Vec::new(),
    };
    let pre = ctx.out.len();
    semantic::cross_field_checks(&mut ctx, &resolved);
    for diagnostic in ctx.out.iter_mut().skip(pre) {
        if let Some(owner) = blame(&provenance, &diagnostic.key_path) {
            diagnostic.file = owner;
        }
        diagnostic.stage = Stage::Merge;
        diagnostic.line = None;
        let note = " this violation appears after layering; the file shown is the layer that set the key (doc 07 \u{a7}4.4).".to_string();
        diagnostic.hint.push_str(&note);
    }
    if !ctx.out.is_empty() {
        return Err(ctx.out);
    }
    Ok(resolved)
}

fn layer_name(layer: Layer) -> &'static str {
    match layer {
        Layer::Product => "product",
        Layer::Profile => "profile",
        Layer::Admin => "admin",
        Layer::User => "user",
        Layer::Flags => "flags",
        Layer::Session => "session",
    }
}

/// Record `path -> file` for every leaf in a layer's JSON form.
fn record_provenance(
    value: &serde_json::Value,
    path: &str,
    file: &str,
    out: &mut BTreeMap<String, String>,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                record_provenance(child, &child_path, file, out);
            }
        }
        serde_json::Value::Array(items) => {
            for (i, child) in items.iter().enumerate() {
                let child_path = format!("{path}.{i}");
                record_provenance(child, &child_path, file, out);
            }
        }
        leaf => {
            let _ = leaf;
            if !path.is_empty() {
                out.insert(path.to_string(), file.to_string());
            }
        }
    }
}

/// Which layer file owns a cross-field violation: the highest layer that set
/// the offending key, else its parent-section setter, else the base file.
fn blame(provenance: &BTreeMap<String, String>, key_path: &str) -> Option<String> {
    // Exact key first.
    if let Some(file) = provenance.get(key_path) {
        return Some(file.clone());
    }
    // Longest prefix (e.g. `window.min_width` set, rule reports `window`).
    let segments: Vec<&str> = key_path.split('.').collect();
    for len in (1..=segments.len()).rev() {
        let candidate = segments[..len].join(".");
        if let Some(file) = provenance.get(&candidate) {
            return Some(file.clone());
        }
    }
    None
}

/// Deep-merge one sparse layer over `config` (§4.4 rules 1–3: tables
/// recursively, scalars and arrays replace).
fn merge_over(config: &mut ProductConfig, partial: &PartialProduct) {
    if let Some(v) = partial.schema_version {
        config.schema_version = v;
    }
    if let Some(p) = &partial.identity {
        if let Some(v) = &p.name {
            config.identity.name = v.clone();
        }
        if let Some(v) = &p.short_name {
            config.identity.short_name = v.clone();
        }
        if let Some(v) = &p.publisher {
            config.identity.publisher = v.clone();
        }
        if let Some(v) = &p.display_version {
            config.identity.display_version = v.clone();
        }
        if let Some(v) = p.version_code {
            config.identity.version_code = v;
        }
        if let Some(v) = &p.description_key {
            config.identity.description_key = v.clone();
        }
        if let Some(p) = &p.ids {
            if let Some(v) = &p.reverse_dns {
                config.identity.ids.reverse_dns = v.clone();
            }
            if let Some(p) = &p.windows {
                if let Some(v) = &p.app_user_model_id {
                    config.identity.ids.windows.app_user_model_id = v.clone();
                }
                if let Some(v) = &p.msi_upgrade_code {
                    config.identity.ids.windows.msi_upgrade_code = v.clone();
                }
            }
            if let Some(p) = &p.macos
                && let Some(v) = &p.bundle_id
            {
                config.identity.ids.macos.bundle_id = v.clone();
            }
            if let Some(p) = &p.linux
                && let Some(v) = &p.appstream_id
            {
                config.identity.ids.linux.appstream_id = v.clone();
            }
            if let Some(p) = &p.web
                && let Some(v) = &p.app_id
            {
                config.identity.ids.web.app_id = v.clone();
            }
        }
    }
    if let Some(p) = &partial.window {
        if let Some(v) = p.width {
            config.window.width = v;
        }
        if let Some(v) = p.height {
            config.window.height = v;
        }
        if let Some(v) = p.min_width {
            config.window.min_width = v;
        }
        if let Some(v) = p.min_height {
            config.window.min_height = v;
        }
        if let Some(v) = p.start_maximized {
            config.window.start_maximized = v;
        }
        if let Some(v) = p.remember_geometry {
            config.window.remember_geometry = v;
        }
        if let Some(v) = &p.title_template {
            config.window.title_template = v.clone();
        }
        if let Some(v) = &p.title_idle_key {
            config.window.title_idle_key = v.clone();
        }
    }
    if let Some(p) = &partial.brand {
        if let Some(v) = &p.accent_color {
            config.brand.accent_color = v.clone();
        }
        if let Some(v) = &p.icon_ico {
            config.brand.icon_ico = v.clone();
        }
        if let Some(v) = &p.icon_icns {
            config.brand.icon_icns = v.clone();
        }
        if let Some(v) = &p.icon_png_512 {
            config.brand.icon_png_512 = v.clone();
        }
        if let Some(v) = &p.logo_svg {
            config.brand.logo_svg = v.clone();
        }
    }
    if let Some(p) = &partial.defaults {
        if let Some(v) = &p.theme {
            config.defaults.theme = v.clone();
        }
        if let Some(v) = &p.locale {
            config.defaults.locale = v.clone();
        }
        if let Some(v) = &p.profile {
            config.defaults.profile = v.clone();
        }
    }
    if let Some(features) = &partial.features {
        for (id, exposed) in features {
            config.features.insert(id.clone(), *exposed);
        }
    }
    if let Some(p) = &partial.playback {
        if let Some(v) = &p.preferred_audio_languages {
            config.playback.preferred_audio_languages = v.clone();
        }
        if let Some(v) = &p.preferred_subtitle_languages {
            config.playback.preferred_subtitle_languages = v.clone();
        }
        if let Some(v) = p.subtitle_mode {
            config.playback.subtitle_mode = v;
        }
        if let Some(v) = p.hw_decode_policy {
            config.playback.hw_decode_policy = v;
        }
        if let Some(v) = p.start_paused {
            config.playback.start_paused = v;
        }
        if let Some(v) = p.default_rate {
            config.playback.default_rate = v;
        }
    }
    if let Some(p) = &partial.limits {
        if let Some(v) = p.cache_demux_mib {
            config.limits.cache_demux_mib = v;
        }
        if let Some(v) = p.cache_network_mib {
            config.limits.cache_network_mib = v;
        }
        if let Some(v) = p.queue_packets_audio {
            config.limits.queue_packets_audio = v;
        }
        if let Some(v) = p.queue_packets_video {
            config.limits.queue_packets_video = v;
        }
        if let Some(v) = p.probe_max_bytes {
            config.limits.probe_max_bytes = v;
        }
        if let Some(v) = p.max_playlist_items {
            config.limits.max_playlist_items = v;
        }
        if let Some(v) = p.max_open_sessions {
            config.limits.max_open_sessions = v;
        }
        if let Some(v) = p.max_wasm_extension_memory_mib {
            config.limits.max_wasm_extension_memory_mib = v;
        }
    }
    if let Some(p) = &partial.telemetry {
        if let Some(v) = p.enabled {
            config.telemetry.enabled = v;
        }
        if let Some(v) = p.crash_reports {
            config.telemetry.crash_reports = v;
        }
        if let Some(v) = p.usage_stats {
            config.telemetry.usage_stats = v;
        }
        if let Some(v) = p.update_checks {
            config.telemetry.update_checks = v;
        }
    }
    if let Some(p) = &partial.urls {
        if let Some(v) = &p.help {
            config.urls.help = Some(v.clone());
        }
        if let Some(v) = &p.support {
            config.urls.support = Some(v.clone());
        }
        if let Some(v) = &p.privacy {
            config.urls.privacy = Some(v.clone());
        }
        if let Some(v) = &p.update_feed {
            config.urls.update_feed = Some(v.clone());
        }
    }
    if let Some(p) = &partial.migrations
        && let Some(v) = &p.entries
    {
        config.migrations.entries = v.clone();
    }
}
