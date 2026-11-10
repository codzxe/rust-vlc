//! The manifest model (doc 06 §2): one `<id>.toml` per plugin,
//! validated field-by-field, unknown fields **rejected** (§2.1 —
//! forward-compat rides `api_version`, never silent fields).
//!
//! A manifest describes **capability, never implementation** (§2.1): no
//! code paths, no engine feature flags, no config *values* — the
//! `[config_schema]` table *references* a settings fragment
//! (repo path for builtins, inline TOML for extensions); the values
//! live in the settings system (doc 07), not here.
//!
//! Two entry points:
//!
//! - [`PluginManifest::from_toml`] — parse + validate a manifest
//!   document (builtin `include_str!` text or the extension package's
//!   copy). Returns the first violation as an actionable
//!   [`ManifestError`] (MP §8).
//! - [`PluginManifest::to_toml`] — serialize back. Round-trip is
//!   value-exact by construction (see the module tests and the four
//!   doc 06 §2.3 example fixtures, which must parse AND round-trip
//!   per the card's acceptance criteria).
//!
//! # Serialization shape (stable; CI and tooling diff against it)
//!
//! Field order mirrors doc 06 §2.2: `id`, `kind`, `version`,
//! `api_version`, `priority`, `origin`, then the `[capabilities]`,
//! `[permissions]`, `[config_schema]`, `[dependencies]`,
//! `[provenance]` tables in §2.2 order. The `toml` serializer forces
//! bare keys before any table header, so `[dependencies]` (an array of
//! bare-key strings) always serializes immediately after the last
//! scalar — position in the document is a serialization artifact, not
//! part of the value.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::kind::PluginKind;
use crate::semver::{Semver, SemverError};

/// serde bridge for [`PluginKind`] (keeps the enum itself free of the
/// serde derive, like every other vocabulary crate type).
impl Serialize for PluginKind {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PluginKind {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<PluginKind, D::Error> {
        let raw = String::deserialize(d)?;
        for kind in PluginKind::ALL {
            if kind.as_str() == raw {
                return Ok(kind);
            }
        }
        Err(serde::de::Error::custom(format!(
            "unknown plugin kind `{raw}` (one of the sixteen doc 06 §1.2 kinds)"
        )))
    }
}

/// The `ext.` id prefix for third-party extensions (doc 06 §2.2).
pub const EXT_PREFIX: &str = "ext.";

/// Maximum id length in bytes (doc 06 §2.2).
pub const ID_MAX_BYTES: usize = 64;

/// Maximum capability entries per manifest, summed across all lists
/// (doc 06 §2.2 — oversized claims are rejected as a DoS guard).
pub const CAPABILITY_ENTRY_LIMIT: usize = 64;

/// Maximum bytes per capability entry (doc 06 §2.2).
pub const CAPABILITY_ENTRY_MAX_BYTES: usize = 32;

/// Default priority for builtins (doc 06 §2.2 [PROPOSAL]).
pub const DEFAULT_PRIORITY_BUILTIN: u32 = 100;

/// Default priority for extensions — builtins win contentional ties
/// (doc 06 §2.2 [PROPOSAL]).
pub const DEFAULT_PRIORITY_EXTENSION: u32 = 50;

// ---------------------------------------------------------------------------
// id
// ---------------------------------------------------------------------------

/// A validated plugin id (doc 06 §2.2): globally unique, stable, never
/// reused; grammar `^[a-z0-9][a-z0-9-]*(\.[a-z0-9][a-z0-9-]*){1,2}$`,
/// ≤ 64 bytes, 2–3 dot-separated segments.
///
/// Namespacing: `lumen.<kind-area>.<name>` = first-party builtin
/// (prefix reserved, the registry rejects it for extensions);
/// `ext.<publisher-slug>.<name>` = third-party extension;
/// no other prefixes in v1 `[PROPOSAL]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PluginId(String);

impl PluginId {
    /// Validate a plugin id against the doc 06 §2.2 grammar.
    ///
    /// # Errors
    /// [`ManifestError::Id`] naming the violated rule.
    pub fn parse(s: &str) -> Result<PluginId, ManifestError> {
        if s.len() > ID_MAX_BYTES {
            return Err(ManifestError::Id(format!(
                "id exceeds {ID_MAX_BYTES} bytes ({} bytes)",
                s.len()
            )));
        }
        if s.is_empty() {
            return Err(ManifestError::Id("id is empty".to_string()));
        }
        let segments: Vec<&str> = s.split('.').collect();
        // {1,2} repetitions after the first ⇒ 2 or 3 segments total.
        if segments.len() < 2 || segments.len() > 3 {
            return Err(ManifestError::Id(format!(
                "id `{s}` must have 2 or 3 dot-separated segments (grammar \
                 ^[a-z0-9][a-z0-9-]*(\\.[a-z0-9][a-z0-9-]*){{1,2}}$)"
            )));
        }
        for seg in segments {
            let mut chars = seg.chars();
            match chars.next() {
                Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
                _ => {
                    return Err(ManifestError::Id(format!(
                        "segment `{seg}` must start with [a-z0-9]"
                    )));
                }
            }
            if !chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                return Err(ManifestError::Id(format!(
                    "segment `{seg}` contains characters outside [a-z0-9-]"
                )));
            }
        }
        Ok(PluginId(s.to_string()))
    }

    /// The id string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The namespacing prefix (`lumen.` or `ext.`), if any (doc 06
    /// §2.2).
    pub fn prefix(&self) -> Option<&'static str> {
        if self.0.starts_with(crate::kind::LUMEN_PREFIX) {
            Some(crate::kind::LUMEN_PREFIX)
        } else if self.0.starts_with(EXT_PREFIX) {
            Some(EXT_PREFIX)
        } else {
            None
        }
    }

    /// The `publisher-slug` of an `ext.` id — segment 2 (doc 06 §2.2:
    /// it must match the `provenance` publisher once a registry entry
    /// is created; the slug is claimed on first install).
    pub fn publisher_slug(&self) -> Option<&str> {
        match self.segments().get(1) {
            // Only meaningful for `ext.` ids; `lumen.` segment 2 is a
            // kind-area, and unprefixed ids have no publisher.
            _ if self.prefix() == Some(EXT_PREFIX) => self.segments().get(1).copied(),
            _ => None,
        }
    }

    /// Dot-separated segments.
    fn segments(&self) -> Vec<&str> {
        self.0.split('.').collect()
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for PluginId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for PluginId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<PluginId, D::Error> {
        let raw = String::deserialize(d)?;
        PluginId::parse(&raw).map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

/// A parsed, validated plugin manifest (doc 06 §2.2 — every field).
///
/// Invariants enforced at construction:
///
/// - `origin = builtin` ⇒ no `[permissions]`, no `[provenance]`;
///   their provenance is the signed commit + reproducible build.
/// - `origin = extension` ⇒ `[provenance]` **required**;
///   `[permissions]` optional but deny-by-default.
/// - A `lumen.` id must be builtin; an `ext.` id must be an extension
///   (disk manifests claiming builtin are a provenance-forgery class,
///   §2.2 `origin`).
/// - `priority` in 0–1000; `api_version` ≥ 1.
/// - Capabilities match the kind's key set (§2.2 table) and obey the
///   list rules (non-empty, deduplicated, lowercase, ≤ 32 B / entry,
///   ≤ 64 entries total).
/// - `network-host-allowlist` implies `http-fetch` (it is the companion
///   list to that permission, §2.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    /// Globally unique stable id (doc 06 §2.2).
    pub id: PluginId,
    /// One of the sixteen kinds (§1.2).
    pub kind: PluginKind,
    /// The plugin's own version, semver 2.0.
    pub version: SemverField,
    /// Engine plugin-API major the plugin was built against (§5.5).
    pub api_version: u32,
    /// Selection weight, 0–1000. `None` = kind/origin default
    /// (`effective_priority` resolves it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u32>,
    /// builtin | extension (§2.2).
    pub origin: Origin,
    /// Kind-specific capability claims (§2.2 table).
    pub capabilities: Capabilities,
    /// Extension-only permissions, deny by default (§2.2).
    #[serde(default, skip_serializing_if = "Permissions::is_absent")]
    pub permissions: Permissions,
    /// Reference to the plugin's settings fragment (§2.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<ConfigSchema>,
    /// Plugin-to-plugin dependencies (§2.2).
    #[serde(default, skip_serializing_if = "Dependencies::is_absent")]
    pub dependencies: Dependencies,
    /// Required for extensions (§2.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

/// `version` in the manifest: a semver 2.0 string, carried as a
/// validated newtype through serde.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SemverField(pub Semver);

impl Serialize for SemverField {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for SemverField {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<SemverField, D::Error> {
        let raw = String::deserialize(d)?;
        match Semver::parse(&raw) {
            Ok(v) => Ok(SemverField(v)),
            Err(e) => Err(serde::de::Error::custom(e)),
        }
    }
}

impl fmt::Display for SemverField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Where a plugin lives (doc 06 §2.2 `origin`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
    /// Compiled into the engine, registered at startup (§1.1).
    Builtin,
    /// A sandboxed WASM module installed on disk (§5).
    Extension,
}

impl Origin {
    /// Manifest spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Origin::Builtin => "builtin",
            Origin::Extension => "extension",
        }
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PluginManifest {
    /// Parse + validate a manifest document (`<id>.toml`, doc 06 §2.1).
    ///
    /// # Errors
    /// [`ManifestError`] — TOML syntax, unknown fields, or the first
    /// §2.2 rule violated; every variant's message is actionable.
    pub fn from_toml(src: &str) -> Result<PluginManifest, ManifestError> {
        let manifest: PluginManifest = toml::from_str(src)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Serialize back to a manifest document (stable field order, see
    /// the module docs).
    ///
    /// # Errors
    /// [`ManifestError::Serialize`] when the value cannot be expressed
    /// (cannot happen for a `from_toml`-parsed manifest; kept honest
    /// for programmatically built ones).
    pub fn to_toml(&self) -> Result<String, ManifestError> {
        match toml::to_string(self) {
            Ok(s) => Ok(s),
            Err(e) => Err(ManifestError::Serialize(e.to_string())),
        }
    }

    /// The §2.2 field-by-field cross-field rules (everything a single
    /// field cannot check on its own).
    ///
    /// # Errors
    /// [`ManifestError`] naming the violated rule.
    pub fn validate(&self) -> Result<(), ManifestError> {
        // api_version: positive integer.
        if self.api_version == 0 {
            return Err(ManifestError::ApiVersion(
                "api_version must be a positive integer".to_string(),
            ));
        }
        // priority: 0..=1000.
        if let Some(p) = self.priority
            && p > 1000
        {
            return Err(ManifestError::Priority(p));
        }
        // Origin ↔ prefix consistency (§2.2 id + origin).
        match (self.id.prefix(), self.origin) {
            (Some(crate::kind::LUMEN_PREFIX), Origin::Extension) => {
                return Err(ManifestError::Origin(format!(
                    "id `{}` carries the reserved `lumen.` prefix but origin is `extension`", // lumen-allow-literal: diagnostic quoting the reserved `lumen.` plugin-id prefix — protocol vocabulary
                    self.id
                )));
            }
            (Some(EXT_PREFIX), Origin::Builtin) => {
                return Err(ManifestError::Origin(format!(
                    "id `{}` carries the `ext.` prefix but origin is `builtin`",
                    self.id
                )));
            }
            (None, _) => {
                return Err(ManifestError::Id(format!(
                    "id `{}` has no v1 namespace prefix (`lumen.` or `ext.`)", // lumen-allow-literal: diagnostic quoting the reserved `lumen.` plugin-id prefix — protocol vocabulary
                    self.id
                )));
            }
            _ => {}
        }
        // Extension ⇒ provenance required; builtin ⇒ none (§2.2).
        match (self.origin, &self.provenance) {
            (Origin::Extension, None) => {
                return Err(ManifestError::Provenance(
                    "extensions must carry a [provenance] table".to_string(),
                ));
            }
            (Origin::Builtin, Some(_)) => {
                return Err(ManifestError::Provenance(
                    "builtin manifests carry no [provenance] — provenance is the signed \
                     commit + reproducible build (doc 06 §2.2)"
                        .to_string(),
                ));
            }
            _ => {}
        }
        // Permissions are extension-only (§2.2).
        if self.origin == Origin::Builtin && !self.permissions.is_absent() {
            return Err(ManifestError::Permissions(
                "[permissions] is extension-only and must be absent for builtin (deny by \
                 default — builtins need no grants)"
                    .to_string(),
            ));
        }
        // Capabilities: kind key set + list rules (§2.2 table).
        self.capabilities.validate(self.kind)?;
        // network-host-allowlist implies http-fetch (§2.2).
        if self.permissions.network_host_allowlist.is_some() && !self.permissions.http_fetch {
            return Err(ManifestError::Permissions(
                "network-host-allowlist is the companion list to http-fetch: declare \
                 `http-fetch = true` too"
                    .to_string(),
            ));
        }
        Ok(())
    }

    /// Effective priority: the declared value or the origin default
    /// (doc 06 §2.2 — builtin 100, extension 50 [PROPOSAL]). Users
    /// may override per-id in settings, clamped to 0–1000 (§3.4, the
    /// registry's job — this is the manifest-side default).
    pub const fn effective_priority(&self) -> u32 {
        match self.priority {
            Some(p) => p,
            None => match self.origin {
                Origin::Builtin => DEFAULT_PRIORITY_BUILTIN,
                Origin::Extension => DEFAULT_PRIORITY_EXTENSION,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// [capabilities]
// ---------------------------------------------------------------------------

/// Kind-specific capability claims (doc 06 §2.2 table). Every claim is
/// verified by the conformance kit (§7.2) — a claim the plugin cannot
/// demonstrate is a manifest lie that blocks shipping.
///
/// Exactly the key set for the manifest's `kind` may appear
/// (`first_listed_key` pins the primary key); unknown keys are rejected
/// by `deny_unknown_fields` and misplaced keys by [`validate`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    /// `access`: `schemes`, `extensions` hint, `supports_seek`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schemes: Option<Vec<String>>,
    /// `access`/`network-protocol`/`demuxer`/`audio-filter`/`video-filter`
    /// (hint lists; for network-protocol see `features`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
    /// `access`: whether the source can seek.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_seek: Option<bool>,
    /// `network-protocol`/`subtitle-parser`: capability features.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,
    /// `demuxer`: container format names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formats: Option<Vec<String>>,
    /// `demuxer`: MIME types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_types: Option<Vec<String>>,
    /// `decoder`/`encoder`: codec names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codecs: Option<Vec<String>>,
    /// `decoder`/`encoder`: profiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profiles: Option<Vec<String>>,
    /// `decoder`/`encoder`: channel ceiling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_channels: Option<u32>,
    /// `decoder`/`encoder`: sample rates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_rates: Option<Vec<u32>>,
    /// `decoder`/`encoder`: bit depths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bit_depths: Option<Vec<u32>>,
    /// `decoder`/`encoder`: truthful hardware claim (MP §2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardware: Option<bool>,
    /// `audio-filter`/`video-filter`: effect names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<Vec<String>>,
    /// `audio-filter`/`video-filter`: input formats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formats_in: Option<Vec<String>>,
    /// `audio-filter`/`video-filter`: output formats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formats_out: Option<Vec<String>>,
    /// `audio-output`: device ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub devices: Option<Vec<String>>,
    /// `audio-output`: latency class.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_class: Option<String>,
    /// `video-output`/`ui-panel`: render/presentation surfaces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surfaces: Option<Vec<String>>,
    /// `subtitle-renderer`: payload classes rendered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renders: Option<Vec<String>>,
    /// `metadata-provider`: data kinds provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provides: Option<Vec<String>>,
    /// `metadata-provider`: lookup sources.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<String>>,
    /// `service-discovery`: discovery protocols.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discovers: Option<Vec<String>>,
}

impl Capabilities {
    /// The primary key of each kind — the one the §2.2 table lists
    /// first and the one a manifest must carry for its kind (the
    /// remaining keys of that kind are optional complements).
    const fn first_listed_key(kind: PluginKind) -> &'static str {
        match kind {
            PluginKind::Access => "schemes",
            PluginKind::NetworkProtocol => "schemes",
            PluginKind::Demuxer => "formats",
            PluginKind::Decoder | PluginKind::Encoder => "codecs",
            PluginKind::AudioFilter | PluginKind::VideoFilter => "effects",
            PluginKind::AudioOutput => "devices",
            PluginKind::VideoOutput => "surfaces",
            PluginKind::SubtitleParser => "formats",
            PluginKind::SubtitleRenderer => "renders",
            PluginKind::PlaylistParser | PluginKind::PlaylistExporter => "formats",
            PluginKind::MetadataProvider => "provides",
            PluginKind::UiPanel => "surfaces",
            PluginKind::ServiceDiscovery => "discovers",
        }
    }

    /// The full legal key set for a kind (§2.2 table row).
    const fn legal_keys(kind: PluginKind) -> &'static [&'static str] {
        match kind {
            PluginKind::Access => &["schemes", "extensions", "supports_seek"],
            PluginKind::NetworkProtocol => &["schemes", "features"],
            PluginKind::Demuxer => &["formats", "extensions", "mime_types"],
            PluginKind::Decoder | PluginKind::Encoder => &[
                "codecs",
                "profiles",
                "max_channels",
                "sample_rates",
                "bit_depths",
                "hardware",
            ],
            PluginKind::AudioFilter | PluginKind::VideoFilter => {
                &["effects", "formats_in", "formats_out"]
            }
            PluginKind::AudioOutput => &["devices", "latency_class"],
            PluginKind::VideoOutput => &["surfaces"],
            PluginKind::SubtitleParser => &["formats", "features"],
            PluginKind::SubtitleRenderer => &["renders"],
            PluginKind::PlaylistParser | PluginKind::PlaylistExporter => &["formats"],
            PluginKind::MetadataProvider => &["provides", "sources"],
            PluginKind::UiPanel => &["surfaces"],
            PluginKind::ServiceDiscovery => &["discovers"],
        }
    }

    /// The set string-list capability keys present on this value
    /// (whichever are `Some`) — drives the §2.2 list rules.
    fn present_string_lists(&self) -> Vec<(&'static str, &Vec<String>)> {
        let mut out = Vec::new();
        for (name, slot) in [
            ("schemes", &self.schemes),
            ("extensions", &self.extensions),
            ("features", &self.features),
            ("formats", &self.formats),
            ("mime_types", &self.mime_types),
            ("codecs", &self.codecs),
            ("profiles", &self.profiles),
            ("effects", &self.effects),
            ("formats_in", &self.formats_in),
            ("formats_out", &self.formats_out),
            ("devices", &self.devices),
            ("surfaces", &self.surfaces),
            ("renders", &self.renders),
            ("provides", &self.provides),
            ("sources", &self.sources),
            ("discovers", &self.discovers),
        ] {
            if let Some(list) = slot {
                out.push((name, list));
            }
        }
        out
    }

    /// Validate against a kind: primary key present, no key outside the
    /// kind's §2.2 row, and the list rules (non-empty, deduplicated,
    /// lowercase, ≤ 32 B / entry, ≤ 64 entries total).
    ///
    /// # Errors
    /// [`ManifestError::Capabilities`] naming the violation.
    pub fn validate(&self, kind: PluginKind) -> Result<(), ManifestError> {
        // Key-set membership: a Some(..) key outside the kind's row.
        for (name, _) in self.present_string_lists() {
            if !Self::legal_keys(kind).contains(&name) {
                return Err(ManifestError::Capabilities(format!(
                    "capability key `{name}` is not legal for kind `{kind}` (legal keys: {})",
                    Self::legal_keys(kind).join(", ")
                )));
            }
        }
        // (membership for the scalar keys is checked below)
        for (_name, _value) in [
            ("max_channels", self.max_channels),
            (
                "sample_rates",
                self.sample_rates.as_deref().map(|v| v.len() as u32),
            ),
            (
                "bit_depths",
                self.bit_depths.as_deref().map(|v| v.len() as u32),
            ),
        ] {}
        for name in ["max_channels", "latency_class", "hardware", "supports_seek"] {
            let present = match name {
                "max_channels" => self.max_channels.is_some(),
                "latency_class" => self.latency_class.is_some(),
                "hardware" => self.hardware.is_some(),
                _ => self.supports_seek.is_some(),
            };
            if present && !Self::legal_keys(kind).contains(&name) {
                return Err(ManifestError::Capabilities(format!(
                    "capability key `{name}` is not legal for kind `{kind}` (legal keys: {})",
                    Self::legal_keys(kind).join(", ")
                )));
            }
        }
        // Primary key present and non-empty.
        let primary = Self::first_listed_key(kind);
        let empty = match primary {
            "schemes" => self.schemes.as_ref().is_none_or(Vec::is_empty),
            "formats" => self.formats.as_ref().is_none_or(Vec::is_empty),
            "codecs" => self.codecs.as_ref().is_none_or(Vec::is_empty),
            "effects" => self.effects.as_ref().is_none_or(Vec::is_empty),
            "devices" => self.devices.as_ref().is_none_or(Vec::is_empty),
            "surfaces" => self.surfaces.as_ref().is_none_or(Vec::is_empty),
            "renders" => self.renders.as_ref().is_none_or(Vec::is_empty),
            "provides" => self.provides.as_ref().is_none_or(Vec::is_empty),
            "discovers" => self.discovers.as_ref().is_none_or(Vec::is_empty),
            _ => true,
        };
        if empty {
            return Err(ManifestError::Capabilities(format!(
                "kind `{kind}` requires its primary capability key `{primary}` (non-empty)"
            )));
        }
        // List rules (§2.2): non-empty, deduplicated, lowercase,
        // ≤ 32 B per entry, ≤ 64 entries total.
        let mut total = 0usize;
        for (name, list) in self.present_string_lists() {
            if list.is_empty() {
                return Err(ManifestError::Capabilities(format!(
                    "capability list `{name}` must be non-empty"
                )));
            }
            let mut seen = std::collections::HashSet::new();
            for entry in list {
                if entry.is_empty() {
                    return Err(ManifestError::Capabilities(format!(
                        "capability list `{name}` contains an empty entry"
                    )));
                }
                if !seen.insert(entry.clone()) {
                    return Err(ManifestError::Capabilities(format!(
                        "capability list `{name}` contains duplicate `{entry}`"
                    )));
                }
                if entry.len() > CAPABILITY_ENTRY_MAX_BYTES {
                    return Err(ManifestError::Capabilities(format!(
                        "capability entry `{entry}` exceeds {CAPABILITY_ENTRY_MAX_BYTES} bytes"
                    )));
                }
                if !entry.chars().all(|c| {
                    c.is_ascii_lowercase()
                        || c.is_ascii_digit()
                        || c == '-'
                        || c == '.'
                        || c == '+'
                        || c == '/'
                        || c == '_'
                }) {
                    return Err(ManifestError::Capabilities(format!(
                        "capability entry `{entry}` must be lowercase"
                    )));
                }
                total += 1;
            }
        }
        if total > CAPABILITY_ENTRY_LIMIT {
            return Err(ManifestError::Capabilities(format!(
                "manifest claims {total} capability entries; the limit is {CAPABILITY_ENTRY_LIMIT}"
            )));
        }
        // Numeric lists obey the same non-empty / dedup rules.
        for (name, list) in [
            ("sample_rates", &self.sample_rates),
            ("bit_depths", &self.bit_depths),
        ] {
            if let Some(list) = list {
                if list.is_empty() {
                    return Err(ManifestError::Capabilities(format!(
                        "capability list `{name}` must be non-empty"
                    )));
                }
                let mut seen = std::collections::HashSet::new();
                for v in list {
                    if !seen.insert(*v) {
                        return Err(ManifestError::Capabilities(format!(
                            "capability list `{name}` contains duplicate {v}"
                        )));
                    }
                }
                if list.len() > CAPABILITY_ENTRY_LIMIT {
                    return Err(ManifestError::Capabilities(format!(
                        "capability list `{name}` exceeds {CAPABILITY_ENTRY_LIMIT} entries"
                    )));
                }
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// [permissions]
// ---------------------------------------------------------------------------

/// Extension permissions (doc 06 §2.2): deny by default; must be
/// absent for builtins. The v1 recognized set `[PROPOSAL — additive
/// across api_version minors]`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Permissions {
    /// Read-only access to files under user-granted sandbox roots
    /// (enforced by the `lumen:v1.fs` import).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fs_sandboxed_read: bool,
    /// Outbound HTTPS GET via the host fetcher (`lumen:v1.http`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub http_fetch: bool,
    /// Companion list to `http-fetch`: exact hostnames the plugin may
    /// contact (implies `http-fetch`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_host_allowlist: Option<Vec<String>>,
    /// Register panels/menus through the layout system (`lumen:v1.ui`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ui_surface: bool,
    /// Read device/output inventory, read-only (`lumen:v1.devices`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub device_query: bool,
}

impl Permissions {
    /// No permission granted at all (deny-by-default ground state).
    pub fn is_absent(&self) -> bool {
        *self == Permissions::default()
    }

    /// Grants as a sorted list of permission names (for events and
    /// install UI; doc 06 §5.3 logging).
    pub fn granted(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.fs_sandboxed_read {
            out.push("fs-sandboxed-read");
        }
        if self.http_fetch {
            out.push("http-fetch");
        }
        if self.network_host_allowlist.is_some() {
            out.push("network-host-allowlist");
        }
        if self.ui_surface {
            out.push("ui-surface");
        }
        if self.device_query {
            out.push("device-query");
        }
        out
    }
}

// ---------------------------------------------------------------------------
// [config_schema]
// ---------------------------------------------------------------------------

/// Reference to the plugin's settings fragment (doc 06 §2.2, shape
/// `[PROPOSAL]`): a repo path for builtins or an inline fragment for
/// extensions. Keys are namespaced under the plugin id
/// (`plugins.<id>.*`); duplicates across plugins fail registry build.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigSchema {
    /// Builtin: `settings/<area>/<id>.toml` checked into the repo.
    /// Extension: the full inline TOML fragment.
    pub settings: String,
}

// ---------------------------------------------------------------------------
// [dependencies]
// ---------------------------------------------------------------------------

/// Plugin-to-plugin dependencies (doc 06 §2.2 — plugin ids, *not*
/// crate dependencies). All listed ids must be present and enabled or
/// the depending plugin is not loaded with a `plugins.failed` event;
/// cycles are rejected at registry build (the registry's job, not
/// this type's).
///
/// Doc 06 §2.3 examples (c)/(d) serialize an *empty* `[dependencies]`
/// table; §2.2 says "array of plugin ids". Both shapes are accepted on
/// the wire; an empty table and an absent table are the same value
/// (empty list) — `is_absent` normalizes them for output.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "DependenciesWire", into = "DependenciesWire")]
pub struct Dependencies(pub Vec<PluginId>);

impl Dependencies {
    /// Empty / not declared.
    pub fn is_absent(&self) -> bool {
        self.0.is_empty()
    }
}

/// Lenient wire shape: array of ids, or a (possibly empty) table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum DependenciesWire {
    /// §2.2's shape: `dependencies = ["lumen.demux.matroska"]`.
    Array(Vec<PluginId>),
    /// §2.3 examples (c)/(d): a bare empty `[dependencies]` table.
    Table(Depth0),
}

/// A table that accepts only nothing (any key is an unknown field).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Depth0 {}

impl From<DependenciesWire> for Dependencies {
    fn from(wire: DependenciesWire) -> Dependencies {
        match wire {
            DependenciesWire::Array(v) => Dependencies(v),
            DependenciesWire::Table(_) => Dependencies(Vec::new()),
        }
    }
}

impl From<Dependencies> for DependenciesWire {
    fn from(deps: Dependencies) -> DependenciesWire {
        DependenciesWire::Array(deps.0)
    }
}

// ---------------------------------------------------------------------------
// [provenance]
// ---------------------------------------------------------------------------

/// Extension provenance (doc 06 §2.2): required for extensions,
/// absent for builtins (their provenance is the signed git commit +
/// reproducible build).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    /// Human-readable publisher name (install UI).
    pub publisher: String,
    /// Canonical source URL of the package.
    pub source: String,
    /// `sha256:<hex>` of the package's `plugin.wasm` — verified at
    /// install and every load.
    pub hash: PackageHash,
    /// Detached signature over the package `[PROPOSAL: ed25519; scheme
    /// + key distribution decided by ADR]` — optional in v1, required
    /// if `product.toml` demands signed extensions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<Signature>,
    /// SPDX identifier.
    pub license: String,
}

/// A `sha256:<hex>` package hash (doc 06 §2.2 `provenance.hash`).
///
/// The §2.3 *example* manifests shorten the hex (`"sha256:9f2c…(64
/// hex)"`) because they are documentation, not installable packages;
/// this type therefore accepts either exactly-64 lowercase/uppercase
/// hex digits or an ellipsis placeholder (kept verbatim so the doc
/// fixtures round-trip), and exposes [`PackageHash::is_full`] for the
/// install-time verifier (§5.7), which requires the full form.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageHash {
    raw: String,
    full: bool,
}

impl PackageHash {
    /// Parse a `sha256:<hex>` value.
    ///
    /// # Errors
    /// [`ManifestError::Hash`] when the prefix or the digest shape is
    /// wrong.
    pub fn parse(s: &str) -> Result<PackageHash, ManifestError> {
        let hex = match s.strip_prefix("sha256:") {
            Some(h) => h,
            None => {
                return Err(ManifestError::Hash(format!(
                    "`{s}` must start with `sha256:`"
                )));
            }
        };
        if hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Ok(PackageHash {
                raw: s.to_string(),
                full: true,
            });
        }
        // Documentation placeholder (doc 06 §2.3): `9f2c…(64 hex)`.
        if hex.contains('…') && hex.contains("(64 hex)") {
            return Ok(PackageHash {
                raw: s.to_string(),
                full: false,
            });
        }
        Err(ManifestError::Hash(format!(
            "`{s}` is neither 64 hex digits nor the doc's placeholder form"
        )))
    }

    /// Whether this is a full 64-hex digest (install-time requirement,
    /// doc 06 §5.7) rather than a doc placeholder.
    pub const fn is_full(&self) -> bool {
        self.full
    }

    /// The raw string as written.
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl fmt::Display for PackageHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

impl Serialize for PackageHash {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.raw)
    }
}

impl<'de> Deserialize<'de> for PackageHash {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<PackageHash, D::Error> {
        let raw = String::deserialize(d)?;
        PackageHash::parse(&raw).map_err(serde::de::Error::custom)
    }
}

/// A detached package signature (doc 06 §2.2 `provenance.signature`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signature {
    /// Signature scheme (v1: ed25519 `[PROPOSAL]`, doc 06 §10 Q5).
    pub scheme: String,
    /// The verification key, in the scheme's canonical text form.
    pub key: Pubkey,
}

/// A public key value (opaque text; the scheme defines the grammar).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pubkey(String);

impl Pubkey {
    /// Accept a public-key string (non-empty; grammar is the signature
    /// scheme's, decided by the pending ADR).
    ///
    /// # Errors
    /// [`ManifestError::Signature`] when empty.
    pub fn parse(s: &str) -> Result<Pubkey, ManifestError> {
        if s.is_empty() {
            return Err(ManifestError::Signature(
                "public key must be non-empty".to_string(),
            ));
        }
        Ok(Pubkey(s.to_string()))
    }

    /// The key text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for Pubkey {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Pubkey {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Pubkey, D::Error> {
        let raw = String::deserialize(d)?;
        Pubkey::parse(&raw).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Manifest validation failures (doc 06 §2.2; MP §8 actionable).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ManifestError {
    /// TOML syntax or shape failure (including unknown fields — §2.1).
    #[error("manifest TOML: {0}")]
    Toml(String),
    /// `id` grammar / namespace violation.
    #[error("manifest id: {0}")]
    Id(String),
    /// `version` is not semver 2.0.
    #[error("manifest version: {0}")]
    Version(#[from] SemverError),
    /// `api_version` is not a positive integer.
    #[error("manifest api_version: {0}")]
    ApiVersion(String),
    /// `priority` outside 0–1000.
    #[error("manifest priority {0} outside the 0–1000 range")]
    Priority(u32),
    /// `origin` inconsistent with the id prefix or the
    /// permissions/provenance tables.
    #[error("manifest origin: {0}")]
    Origin(String),
    /// `[capabilities]` violates the §2.2 table or list rules.
    #[error("manifest capabilities: {0}")]
    Capabilities(String),
    /// `[permissions]` violates §2.2 (builtin present, or companion
    /// rule broken).
    #[error("manifest permissions: {0}")]
    Permissions(String),
    /// `[provenance]` missing for an extension / present for a
    /// builtin, or malformed.
    #[error("manifest provenance: {0}")]
    Provenance(String),
    /// `provenance.hash` malformed.
    #[error("manifest provenance hash: {0}")]
    Hash(String),
    /// `provenance.signature` malformed.
    #[error("manifest signature: {0}")]
    Signature(String),
    /// Serialization failed (cannot happen for parsed manifests).
    #[error("manifest serialize: {0}")]
    Serialize(String),
}

impl From<toml::de::Error> for ManifestError {
    fn from(e: toml::de::Error) -> ManifestError {
        ManifestError::Toml(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kind::LUMEN_PREFIX;

    fn id(s: &str) -> PluginId {
        match PluginId::parse(s) {
            Ok(i) => i,
            Err(e) => unreachable!("{s}: {e}"),
        }
    }

    #[test]
    fn id_grammar() {
        for good in [
            "lumen.demux.matroska", // lumen-allow-literal: test fixture value — round-trip data, not display identity
            "lumen.codec.flac", // lumen-allow-literal: test fixture value — round-trip data, not display identity
            "ext.acme.lyrics-panel",
            "a.b",
            "ext.a-b.c-d",
        ] {
            assert!(PluginId::parse(good).is_ok(), "{good}");
        }
        for bad in [
            "",                                                                              // empty
            "nodots",  // 1 segment
            "a.b.c.d", // 4 segments
            ".a.b",    // empty segment
            "a..b",    // empty segment
            "A.b",     // uppercase
            "a.B",     // uppercase
            "-a.b",    // leading dash
            "a b.c",   // space
            "lumen.codec.a-very-long-kind-area-name-that-exceeds-the-sixty-four-byte-cap.x", // > 64 B // lumen-allow-literal: test fixture value — round-trip data, not display identity
        ] {
            assert!(PluginId::parse(bad).is_err(), "{bad:?}");
        }
        // Exactly 64 bytes is legal.
        let exact = "a.".to_string() + &"b".repeat(62);
        assert_eq!(exact.len(), 64);
        assert!(PluginId::parse(&exact).is_ok());
    }

    #[test]
    fn id_prefixes_and_slugs() {
        assert_eq!(id("lumen.demux.matroska").prefix(), Some(LUMEN_PREFIX)); // lumen-allow-literal: test fixture value — round-trip data, not display identity
        assert_eq!(id("ext.acme.lyrics-panel").prefix(), Some(EXT_PREFIX));
        assert_eq!(id("ext.acme.lyrics-panel").publisher_slug(), Some("acme"));
        assert_eq!(id("lumen.demux.matroska").publisher_slug(), None); // lumen-allow-literal: test fixture value — round-trip data, not display identity
    }

    #[test]
    fn hash_accepts_full_and_doc_placeholder() {
        let full = match PackageHash::parse(
            "sha256:9f2c3f1e0a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5",
        ) {
            Ok(h) => h,
            Err(e) => unreachable!("{e}"),
        };
        assert!(full.is_full());
        let doc = match PackageHash::parse("sha256:9f2c…(64 hex)") {
            Ok(h) => h,
            Err(e) => unreachable!("{e}"),
        };
        assert!(!doc.is_full());
        for bad in [
            "sha256:",
            "sha256:xyz",
            "sha256:9f2c…",
            "9f2c…(64 hex)",
            "sha512:9f2c…(64 hex)",
        ] {
            assert!(PackageHash::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn empty_dependencies_table_and_array_agree() {
        // Deserializing the Dependencies value directly: the document
        // root is a table, so a wrapper field hands the wire shape to
        // the untagged enum (mirrors how `dependencies` sits inside a
        // manifest).
        #[derive(serde::Deserialize)]
        struct Doc {
            dependencies: Dependencies,
        }
        let array = match toml::from_str::<Doc>("dependencies = []") {
            Ok(d) => d.dependencies,
            Err(e) => unreachable!("{e}"),
        };
        let table = match toml::from_str::<Doc>("[dependencies]") {
            Ok(d) => d.dependencies,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(array, table, "§2.3 empty table ≡ §2.2 empty array");
        assert!(array.is_absent());
        let one = match toml::from_str::<Doc>("dependencies = [\"ext.acme.tags\"]") {
            Ok(d) => d.dependencies,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(one.0.len(), 1);
        assert!(!one.is_absent());
    }

    #[test]
    fn capabilities_primary_key_enforced_per_kind() {
        let mut caps = Capabilities {
            formats: Some(["matroska"].iter().map(|s| s.to_string()).collect()),
            ..Capabilities::default()
        };
        assert!(caps.validate(PluginKind::Demuxer).is_ok());
        // formats is not legal for a decoder.
        assert!(caps.validate(PluginKind::Decoder).is_err());
        // A demuxer without formats fails the primary-key rule.
        caps.formats = None;
        assert!(caps.validate(PluginKind::Demuxer).is_err());
        // Duplicate entry.
        caps.formats = Some(["mkv", "mkv"].iter().map(|s| s.to_string()).collect());
        assert!(caps.validate(PluginKind::Demuxer).is_err());
        // Uppercase entry.
        caps.formats = Some(["MKV"].iter().map(|s| s.to_string()).collect());
        assert!(caps.validate(PluginKind::Demuxer).is_err());
        // Oversized entry (> 32 bytes).
        caps.formats = Some(vec!["a".repeat(33)]);
        assert!(caps.validate(PluginKind::Demuxer).is_err());
        // Over the 64-entry budget.
        caps.formats = Some((0..65).map(|i| format!("f{i}")).collect());
        assert!(caps.validate(PluginKind::Demuxer).is_err());
        // Exactly 64 entries is fine.
        caps.formats = Some((0..64).map(|i| format!("f{i}")).collect());
        assert!(caps.validate(PluginKind::Demuxer).is_ok());
    }

    #[test]
    fn cross_field_rules() {
        // Extension without provenance: `from_toml` runs validate() as
        // its last step, so the rule fires at parse already.
        assert!(matches!(
            PluginManifest::from_toml(
                "id = \"ext.acme.x\"\nkind = \"ui-panel\"\nversion = \"1.0.0\"\napi_version = 1\n\
                 origin = \"extension\"\n[capabilities]\nsurfaces = [\"side-panel\"]\n",
            ),
            Err(ManifestError::Provenance(_)),
        ));
        // Builtin with permissions: fires at parse (validate is
        // from_toml's last step).
        assert!(matches!(
            PluginManifest::from_toml(
                // lumen-allow-literal: test fixture value — round-trip data, not display identity
                "id = \"lumen.demux.x\"\nkind = \"demuxer\"\nversion = \"1.0.0\"\napi_version = 1\n\
                 origin = \"builtin\"\n[capabilities]\nformats = [\"x\"]\n\
                 [permissions]\nhttp-fetch = true\n",
            ),
            Err(ManifestError::Permissions(_)),
        ));
        // allowlist without http-fetch.
        assert!(matches!(
            PluginManifest::from_toml(
                // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
                "id = \"ext.acme.x\"\nkind = \"metadata-provider\"\nversion = \"1.0.0\"\n\
                 api_version = 1\norigin = \"extension\"\n[capabilities]\nprovides = [\"tags\"]\n\
                 [permissions]\nnetwork-host-allowlist = [\"musicbrainz.org\"]\n\
                 [provenance]\npublisher = \"A\"\nsource = \"https://a\"\nhash = \"sha256:9f2c…(64 hex)\"\n\
                 license = \"MIT\"\n",
            ),
            Err(ManifestError::Permissions(_)),
        ));
        // ext. id claiming builtin.
        assert!(matches!(
            PluginManifest::from_toml(
                "id = \"ext.acme.x\"\nkind = \"demuxer\"\nversion = \"1.0.0\"\napi_version = 1\n\
                 origin = \"builtin\"\n[capabilities]\nformats = [\"x\"]\n",
            ),
            Err(ManifestError::Origin(_)),
        ));
    }

    #[test]
    fn priority_defaults_and_range() {
        let builtin = match PluginManifest::from_toml(
            // lumen-allow-literal: test fixture value — round-trip data, not display identity
            "id = \"lumen.demux.x\"\nkind = \"demuxer\"\nversion = \"1.0.0\"\napi_version = 1\n\
             origin = \"builtin\"\n[capabilities]\nformats = [\"x\"]\n",
        ) {
            Ok(m) => m,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(builtin.effective_priority(), 100);
        let ext = match PluginManifest::from_toml(
            // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
            "id = \"ext.acme.x\"\nkind = \"metadata-provider\"\nversion = \"1.0.0\"\n\
             api_version = 1\norigin = \"extension\"\n[capabilities]\nprovides = [\"tags\"]\n\
             [provenance]\npublisher = \"A\"\nsource = \"https://a\"\nhash = \"sha256:9f2c…(64 hex)\"\n\
             license = \"MIT\"\n",
        ) {
            Ok(m) => m,
            Err(e) => unreachable!("{e}"),
        };
        assert_eq!(ext.effective_priority(), 50);
        assert!(
            matches!(
                PluginManifest::from_toml(
                    // lumen-allow-literal: test fixture value — round-trip data, not display identity
                    "id = \"lumen.demux.x\"\nkind = \"demuxer\"\nversion = \"1.0.0\"\n\
                     api_version = 1\npriority = 1001\norigin = \"builtin\"\n\
                     [capabilities]\nformats = [\"x\"]\n"
                ),
                Err(ManifestError::Priority(1001))
            ),
            "out of range"
        );
    }

    #[test]
    fn unknown_fields_rejected() {
        // §2.1: unknown fields are rejected, not ignored.
        // lumen-allow-literal: test fixture value — round-trip data, not display identity
        let src = "id = \"lumen.demux.x\"\nkind = \"demuxer\"\nversion = \"1.0.0\"\n\
                   api_version = 1\norigin = \"builtin\"\nfuture_field = true\n\
                   [capabilities]\nformats = [\"x\"]\n";
        assert!(PluginManifest::from_toml(src).is_err());
        // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
        let src = "id = \"ext.acme.x\"\nkind = \"metadata-provider\"\nversion = \"1.0.0\"\n\
                   api_version = 1\norigin = \"extension\"\n[capabilities]\nprovides = [\"tags\"]\n\
                   [capabilities.sneaky]\nx = 1\n[provenance]\npublisher = \"A\"\n\
                   source = \"https://a\"\nhash = \"sha256:9f2c…(64 hex)\"\nlicense = \"MIT\"\n";
        assert!(PluginManifest::from_toml(src).is_err());
        // lumen-allow-literal: test-double URL on a reserved/example host — never a shipped endpoint
        let src = "id = \"ext.acme.x\"\nkind = \"metadata-provider\"\nversion = \"1.0.0\"\n\
                   api_version = 1\norigin = \"extension\"\n[capabilities]\nprovides = [\"tags\"]\n\
                   [permissions]\nhtt-fetch = true\n[provenance]\npublisher = \"A\"\n\
                   source = \"https://a\"\nhash = \"sha256:9f2c…(64 hex)\"\nlicense = \"MIT\"\n";
        assert!(PluginManifest::from_toml(src).is_err(), "typo'd permission");
    }
}
