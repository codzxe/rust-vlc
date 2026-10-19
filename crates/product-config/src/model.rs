//! The typed product model (doc 07 §2). Every field of `product.toml` has a
//! home here; `serde(deny_unknown_fields)` keeps the set exact.
//!
//! Optionality mirrors `product.schema.json`: fields the schema marks
//! required are plain; fields carrying JSON-Schema `default` annotations
//! (window geometry persistence flags, telemetry — doc 07 §3.2) fall back to
//! those defaults; `urls.*` are optional (the telemetry/privacy cross-field
//! rule makes absence meaningful — doc 07 §3.3).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Complete, validated `product.toml` (doc 07 §2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductConfig {
    pub schema_version: u32,
    pub identity: Identity,
    pub window: Window,
    pub brand: Brand,
    pub defaults: Defaults,
    pub features: BTreeMap<String, bool>,
    pub playback: Playback,
    pub limits: Limits,
    #[serde(default)]
    pub telemetry: Telemetry,
    pub urls: Urls,
    #[serde(default)]
    pub migrations: Migrations,
}

/// Rebrandable identity (doc 07 §2.1, §8.2 propagation matrix).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub name: String,
    pub short_name: String,
    pub publisher: String,
    pub display_version: String,
    pub version_code: u64,
    pub description_key: String,
    pub ids: IdentityIds,
}

/// Reverse-DNS root and per-OS identifiers (doc 07 §2.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityIds {
    pub reverse_dns: String,
    pub windows: WindowsIds,
    pub macos: MacosIds,
    pub linux: LinuxIds,
    pub web: WebIds,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowsIds {
    pub app_user_model_id: String,
    pub msi_upgrade_code: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MacosIds {
    pub bundle_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinuxIds {
    pub appstream_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebIds {
    pub app_id: String,
}

/// Window defaults (doc 07 §2.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Window {
    pub width: u32,
    pub height: u32,
    pub min_width: u32,
    pub min_height: u32,
    #[serde(default)]
    pub start_maximized: bool,
    #[serde(default = "default_true")]
    pub remember_geometry: bool,
    pub title_template: String,
    pub title_idle_key: String,
}

fn default_true() -> bool {
    true
}

/// Brand assets (doc 07 §2.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Brand {
    pub accent_color: String,
    pub icon_ico: String,
    pub icon_icns: String,
    pub icon_png_512: String,
    pub logo_svg: String,
}

/// Product defaults (doc 07 §2.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Defaults {
    pub theme: String,
    pub locale: String,
    pub profile: String,
}

/// Playback & track-selection rules (doc 07 §2.8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Playback {
    pub preferred_audio_languages: Vec<String>,
    pub preferred_subtitle_languages: Vec<String>,
    pub subtitle_mode: SubtitleMode,
    pub hw_decode_policy: HwDecodePolicy,
    pub start_paused: bool,
    pub default_rate: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SubtitleMode {
    Auto,
    ForcedOnly,
    On,
    Off,
}

/// Hardware-decode policy vocabulary (doc 07 §2.8). Grown post-ADR-001
/// (accepted 2026-10-19, `docs/adr/ADR-001-hardware-decode.md`): hw-first
/// H.264/HEVC via OS decoders is the accepted policy, so the two
/// pre-registered hardware values are legal. The shipped `product.toml`
/// stays `software-only` until the Phase 7 audited per-OS integration
/// lands — rule F2 keeps `features.hw-decode = false` until then.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HwDecodePolicy {
    /// All decoding in Rust software (MP §2 default; also the honest shipped
    /// value while the ADR-001 hardware integration is not yet implemented).
    SoftwareOnly,
    /// Prefer OS-decoder hardware paths, fall back to software (ADR-001 §1).
    PreferHardware,
    /// Hardware only; without OS-decoder support the open fails honestly
    /// with `unsupported-software-decode-pending` (ADR-001 §1).
    HardwareOnly,
}

/// Cache & resource limits — reviewed safety bounds (doc 07 §2.9).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub cache_demux_mib: u64,
    pub cache_network_mib: u64,
    pub queue_packets_audio: u64,
    pub queue_packets_video: u64,
    pub probe_max_bytes: u64,
    pub max_playlist_items: u64,
    pub max_open_sessions: u64,
    pub max_wasm_extension_memory_mib: u64,
}

/// Telemetry & privacy — opt-in only, shipped defaults off (doc 07 §2.6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Telemetry {
    pub enabled: bool,
    pub crash_reports: CrashReports,
    pub usage_stats: bool,
    pub update_checks: bool,
}

impl Default for Telemetry {
    fn default() -> Self {
        Telemetry {
            enabled: false,
            crash_reports: CrashReports::Off,
            usage_stats: false,
            update_checks: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CrashReports {
    Off,
    Ask,
    Send,
}

/// Product URLs (doc 07 §2.10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct Urls {
    pub help: Option<String>,
    pub support: Option<String>,
    pub privacy: Option<String>,
    pub update_feed: Option<String>,
}

/// Schema-version migration ledger (doc 07 §2.11).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct Migrations {
    #[serde(default)]
    pub entries: Vec<MigrationEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationEntry {
    pub from: u32,
    pub to: u32,
    pub summary: String,
    pub migrator: String,
}
