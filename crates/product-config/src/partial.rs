//! The user-layer shape: a *partial* product configuration (doc 07 §4).
//!
//! Layers 2–6 (profile, admin, user preferences, flags, session — §4.1) are
//! expressed as sparse overrides over the product defaults: only keys the
//! layer sets are present; everything else falls through to the next-lower
//! layer. Unknown keys are rejected at parse time (`deny_unknown_fields`) so
//! typos surface as startup errors with did-you-mean hints (§4.4 rule 4,
//! §4.5 T10) — never as silent no-ops.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A sparse override document for any precedence layer (doc 07 §4.1).
///
/// v1 (card 1.B1) loads this from user preferences; profile/admin/flag/
/// session loaders are Phase-2+ cards that reuse this shape.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PartialProduct {
    pub schema_version: Option<u32>,
    pub identity: Option<PartialIdentity>,
    pub window: Option<PartialWindow>,
    pub brand: Option<PartialBrand>,
    pub defaults: Option<PartialDefaults>,
    pub features: Option<BTreeMap<String, bool>>,
    pub playback: Option<PartialPlayback>,
    pub limits: Option<PartialLimits>,
    pub telemetry: Option<PartialTelemetry>,
    pub urls: Option<PartialUrls>,
    pub migrations: Option<PartialMigrations>,
}

impl PartialProduct {
    /// True when no key is set (an empty layer is a no-op, §4.5 T15).
    pub fn is_empty(&self) -> bool {
        *self == PartialProduct::default()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PartialIdentity {
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub publisher: Option<String>,
    pub display_version: Option<String>,
    pub version_code: Option<u64>,
    pub description_key: Option<String>,
    pub ids: Option<PartialIdentityIds>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PartialIdentityIds {
    pub reverse_dns: Option<String>,
    pub windows: Option<PartialWindowsIds>,
    pub macos: Option<PartialMacosIds>,
    pub linux: Option<PartialLinuxIds>,
    pub web: Option<PartialWebIds>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PartialWindowsIds {
    pub app_user_model_id: Option<String>,
    pub msi_upgrade_code: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PartialMacosIds {
    pub bundle_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PartialLinuxIds {
    pub appstream_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PartialWebIds {
    pub app_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PartialWindow {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub start_maximized: Option<bool>,
    pub remember_geometry: Option<bool>,
    pub title_template: Option<String>,
    pub title_idle_key: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PartialBrand {
    pub accent_color: Option<String>,
    pub icon_ico: Option<String>,
    pub icon_icns: Option<String>,
    pub icon_png_512: Option<String>,
    pub logo_svg: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PartialDefaults {
    pub theme: Option<String>,
    pub locale: Option<String>,
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PartialPlayback {
    pub preferred_audio_languages: Option<Vec<String>>,
    pub preferred_subtitle_languages: Option<Vec<String>>,
    pub subtitle_mode: Option<crate::model::SubtitleMode>,
    pub hw_decode_policy: Option<crate::model::HwDecodePolicy>,
    pub start_paused: Option<bool>,
    pub default_rate: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PartialLimits {
    pub cache_demux_mib: Option<u64>,
    pub cache_network_mib: Option<u64>,
    pub queue_packets_audio: Option<u64>,
    pub queue_packets_video: Option<u64>,
    pub probe_max_bytes: Option<u64>,
    pub max_playlist_items: Option<u64>,
    pub max_open_sessions: Option<u64>,
    pub max_wasm_extension_memory_mib: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PartialTelemetry {
    pub enabled: Option<bool>,
    pub crash_reports: Option<crate::model::CrashReports>,
    pub usage_stats: Option<bool>,
    pub update_checks: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PartialUrls {
    pub help: Option<String>,
    pub support: Option<String>,
    pub privacy: Option<String>,
    pub update_feed: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PartialMigrations {
    pub entries: Option<Vec<crate::model::MigrationEntry>>,
}
