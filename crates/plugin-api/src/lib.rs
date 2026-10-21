//! The Lumen plugin contract (card 1.D2; doc 06 §1–2): the sixteen-kind
//! [`PluginKind`](kind::PluginKind) taxonomy, the validated
//! [manifest](manifest) model (`<id>.toml`, unknown fields rejected,
//! capability-not-implementation), and the **host traits** the L1-owned
//! plugin kinds implement ([`Access`](access::Access),
//! [`Demuxer`](demux::Demuxer), [`SubtitleParser`](subtitle::SubtitleParser),
//! [`PlaylistParser`](playlist::PlaylistParser), ...).
//!
//! Layer L1 (doc 01 §2.2 registry): depends only on [`media_types`] (L0
//! vocabulary) and [`codec_api`] (L1 codec contract — the decoder and
//! encoder kinds *reuse* its traits; they are re-exported here so a
//! plugin author needs one import surface, never redefined).
//!
//! # Scope fence (doc 06 §8, this card = §1–2 only)
//!
//! - **No registry, no selection** — doc 06 §3 is the Phase 2 engine
//!   card. This crate defines the *data* selection scores (`priority`,
//!   capability keys) and validates it; it does not rank plugins.
//! - **No WASM host** — doc 06 §5 is `crates/plugin-host-wasm`. The
//!   v1 extension policy is data on [`PluginKind`] (§1.4).
//! - **No enforcement** — doc 06 §6 forbidden-list tooling consumes
//!   these types.
//! - **No conformance kit** — doc 06 §7 lands with the first plugin
//!   implementations (the `contract` module pattern of `codec-api` is
//!   the precedent to copy).
//! - Domain-owned traits (`AudioBackend`, `Presenter`,
//!   `ServiceDiscovery`, ...) are **not** defined here: doc 06 §1.2
//!   places them in their domain crates and keeps L1 free of L2/L3/L4
//!   deps; the manifest's `kind` *names* them without importing them.
//!
//! # Threading and no-blocking contract
//!
//! Same binding as codec-api (doc 01 §4.1–4.2, restated per doc 06
//! §1.3): host traits are called from dedicated worker threads, every
//! method must return (bounded work per call; unbounded waits are
//! contract violations), cancellation is generation staleness — never a
//! token — and panics are contract violations (workspace lints deny
//! `panic!`/`unwrap`/`expect`). The `Send` supertrait marks the traits
//! the engine owns across thread boundaries; sinks are created and
//! consumed on the caller's thread and deliberately are not `Send`.
//!
//! # Module map
//!
//! - [`kind`]       — [`PluginKind`](kind::PluginKind) (16 kinds, §1.2)
//!   and the §1.4 extension policy.
//! - [`semver`]     — the minimal semver-2.0 parser `version` fields
//!   must satisfy (§2.2; no external semver crate — dependency
//!   discipline).
//! - [`manifest`]   — the manifest model: [`PluginManifest`],
//!   [`Capabilities`], [`Permissions`], [`ConfigSchema`],
//!   [`Provenance`], [`PluginId`] grammar, and TOML (de)serialization
//!   with the §2.2 field-by-field validation.
//! - [`access`]     — [`Access`] + [`SourceStream`]: bytes in (§1.2
//!   kinds 1–2).
//! - [`demux`]      — [`Demuxer`] + [`DemuxEvent`]: container →
//!   packets + seek tables (kind 3).
//! - [`codec`]      — decoder/encoder kinds: re-exports of
//!   [`codec_api`] traits (kinds 4–5, never redefined).
//! - [`filter`]     — [`AudioFilter`] / [`VideoFilter`] outcome +
//!   shape vocabulary (kinds 6–7; the domain traits live in
//!   `audio-engine`/`video-engine` per §1.2 [PROPOSAL: the outcome
//!   types are staged here]).
//! - [`subtitle`]   — [`SubtitleParser`] (kind 10).
//! - [`playlist`]   — [`PlaylistParser`], [`PlaylistExporter`] and the
//!   minimal [`Playlist`] vocabulary (kinds 12–13).
//! - [`metadata`]   — [`MetadataProvider`] (kind 14).
//! - [`ui`]         — [`UiPanel`] host contract (kind 15, §1.2
//!   [PROPOSAL] placement — extension-only in v1).
//! - [`discovery`]  — [`ServiceDiscovery`] host contract (kind 16;
//!   domain trait in `device-discovery` per §1.2 [PROPOSAL: staged
//!   here like the filter outcomes]).

pub mod access;
pub mod codec;
pub mod demux;
pub mod discovery;
pub mod filter;
pub mod kind;
pub mod manifest;
pub mod metadata;
pub mod playlist;
pub mod semver;
pub mod subtitle;
pub mod ui;

// Headline re-exports — flat access for the contract surface every
// plugin crate speaks. Keep in sync with `prelude` below.
pub use access::{Access, SeekOrigin, SourcePosition, SourceStream};
pub use codec::{Decoder, Encoder};
pub use demux::{DemuxEvent, Demuxer, PacketSink, SeekSupport};
pub use filter::{AudioFilter, FilterOutcome, FrameSource, VideoFilter};
pub use kind::{LUMEN_PREFIX, PluginKind};
pub use manifest::{
    Capabilities, ConfigSchema, Dependencies, Permissions, PluginId, PluginManifest, Provenance,
    Pubkey,
};
pub use metadata::{Metadata, MetadataProvider, Tag};
pub use playlist::{Playlist, PlaylistEntry, PlaylistExporter, PlaylistParser};
pub use subtitle::{SubtitleParser, SubtitleSink};
pub use ui::{PanelDescriptor, UiPanel};

/// Prelude-style alias: the whole plugin contract in one `use`.
pub mod prelude {
    pub use crate::access::{Access, SeekOrigin, SourcePosition, SourceStream};
    pub use crate::codec::{Decoder, Encoder};
    pub use crate::demux::{DemuxEvent, Demuxer, PacketSink};
    pub use crate::filter::{AudioFilter, FilterOutcome, FrameSource, VideoFilter};
    pub use crate::kind::{LUMEN_PREFIX, PluginKind};
    pub use crate::manifest::{
        Capabilities, ConfigSchema, Dependencies, Permissions, PluginId, PluginManifest,
        Provenance, Pubkey,
    };
    pub use crate::metadata::{Metadata, MetadataProvider, Tag};
    pub use crate::playlist::{Playlist, PlaylistEntry, PlaylistExporter, PlaylistParser};
    pub use crate::subtitle::{SubtitleParser, SubtitleSink};
    pub use crate::ui::{PanelDescriptor, UiPanel};
}

/// The engine plugin-API major this workspace builds (doc 06 §2.2
/// `api_version`, §5.5).
///
/// Builtins are compiled against it by construction; CI checks every
/// `plugins/builtin/*.toml` carries exactly this value (§2.2: a
/// mismatch is a build break, never a runtime surprise).
pub const ENGINE_PLUGIN_API_MAJOR: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prelude_reexports_the_whole_contract() {
        // One `use` compiles the whole v1 surface (doc 06 §1.2 L1 rows).
        let _: PluginKind = PluginKind::Demuxer;
        let api: u32 = ENGINE_PLUGIN_API_MAJOR;
        assert_eq!(api, 1, "v1 of the plugin API (doc 06 §5.5)");
    }
}
