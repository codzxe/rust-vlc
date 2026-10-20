//! The sixteen plugin kinds (doc 06 §1.2) and the §1.4 v1 extension
//! policy.

use std::fmt;

/// The `lumen.` id prefix reserved for first-party builtins (doc 06
/// §2.2 namespacing; the registry rejects it for extensions).
pub const LUMEN_PREFIX: &str = "lumen.";

/// What a plugin *is* — the sixteen kinds of doc 06 §1.2.
///
/// Doc 01 §2 defines fourteen variants; doc 06 §1.2 splits
/// `SubtitleFormat` into [`SubtitleParser`] + [`SubtitleRenderer`] and
/// `PlaylistFormat` into [`PlaylistParser`] + [`PlaylistExporter`]
/// (parsing and rendering/exporting have different traits, lifecycles
/// and sandbox postures) — the resulting sixteen-variant enum is the
/// `[PROPOSAL]` this crate implements, flagged for the doc 06 §10 Q1
/// Lead reconciliation.
///
/// Every variant carries its manifest string (`kind = "..."`, doc 06
/// §2.2), its instance scope (§1.3) and its v1 extension policy (§1.4).
///
/// [`SubtitleParser`]: PluginKind::SubtitleParser
/// [`PlaylistParser`]: PluginKind::PlaylistParser
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum PluginKind {
    /// 1 — file/disc byte source, first stage (§1.2 row 1; impls in
    /// `input`, L2).
    Access,
    /// 2 — network stream source (HTTP(S), HLS, DASH, RTSP; impls in
    /// `streaming`, L2).
    NetworkProtocol,
    /// 3 — container parse → timestamped packets + seek tables (impls
    /// in `demux`, L2).
    Demuxer,
    /// 4 — packet → frame; re-uses [`codec_api::Decoder`], never
    /// redefined (impls in `codecs`, L3).
    Decoder,
    /// 5 — frame → packet; re-uses [`codec_api::Encoder`] (Phase 7;
    /// `codecs`, L3).
    Encoder,
    /// 6 — post-decode audio chain (domain trait in `audio-engine`,
    /// L3).
    AudioFilter,
    /// 7 — post-decode video chain (domain trait in `video-engine`,
    /// L3).
    VideoFilter,
    /// 8 — final audio sink + master clock (domain trait
    /// `AudioBackend` in `audio-output`, L4).
    AudioOutput,
    /// 9 — presentation (domain trait `Presenter` in `video-engine`,
    /// L3).
    VideoOutput,
    /// 10 — subtitle format → timed cues (impls in `subtitles`, L2).
    SubtitleParser,
    /// 11 — cue + style → overlay draw list (default impl in
    /// `renderer-wgpu`, L4).
    SubtitleRenderer,
    /// 12 — URI/drop → playlist entries (domain trait in `playlists`,
    /// L2).
    PlaylistParser,
    /// 13 — playlist → file, `playlist.save` (domain trait in
    /// `playlists`, L2) [PROPOSAL kind].
    PlaylistExporter,
    /// 14 — tag/cover-art extraction + on-demand lookups (builtin home
    /// `media-probe`, L2).
    MetadataProvider,
    /// 15 — UI surface talking only to the command layer (guest
    /// protocol in `plugin-host-wasm`, L6) — **extension only in v1**
    /// (§1.2 row 15).
    UiPanel,
    /// 16 — background enumeration → device lists (domain trait in
    /// `device-discovery`, L2).
    ServiceDiscovery,
}

impl PluginKind {
    /// All sixteen kinds, in doc 06 §1.2 table order.
    pub const ALL: [PluginKind; 16] = [
        PluginKind::Access,
        PluginKind::NetworkProtocol,
        PluginKind::Demuxer,
        PluginKind::Decoder,
        PluginKind::Encoder,
        PluginKind::AudioFilter,
        PluginKind::VideoFilter,
        PluginKind::AudioOutput,
        PluginKind::VideoOutput,
        PluginKind::SubtitleParser,
        PluginKind::SubtitleRenderer,
        PluginKind::PlaylistParser,
        PluginKind::PlaylistExporter,
        PluginKind::MetadataProvider,
        PluginKind::UiPanel,
        PluginKind::ServiceDiscovery,
    ];

    /// The manifest string (doc 06 §2.2 `kind`; the §2.3 examples and
    /// the §2.2 capability table use these kebab-case spellings).
    pub const fn as_str(self) -> &'static str {
        match self {
            PluginKind::Access => "access",
            PluginKind::NetworkProtocol => "network-protocol",
            PluginKind::Demuxer => "demuxer",
            PluginKind::Decoder => "decoder",
            PluginKind::Encoder => "encoder",
            PluginKind::AudioFilter => "audio-filter",
            PluginKind::VideoFilter => "video-filter",
            PluginKind::AudioOutput => "audio-output",
            PluginKind::VideoOutput => "video-output",
            PluginKind::SubtitleParser => "subtitle-parser",
            PluginKind::SubtitleRenderer => "subtitle-renderer",
            PluginKind::PlaylistParser => "playlist-parser",
            PluginKind::PlaylistExporter => "playlist-exporter",
            PluginKind::MetadataProvider => "metadata-provider",
            PluginKind::UiPanel => "ui-panel",
            PluginKind::ServiceDiscovery => "service-discovery",
        }
    }

    /// Who owns construction/teardown (doc 06 §1.3 instance-scope
    /// table).
    #[allow(clippy::match_same_arms)]
    pub const fn instance_scope(self) -> InstanceScope {
        match self {
            PluginKind::Access | PluginKind::NetworkProtocol => InstanceScope::PerOpenSource,
            PluginKind::Demuxer
            | PluginKind::Decoder
            | PluginKind::SubtitleParser
            | PluginKind::SubtitleRenderer
            | PluginKind::AudioFilter
            | PluginKind::VideoFilter => InstanceScope::PerPlaybackSession,
            PluginKind::Encoder | PluginKind::PlaylistParser | PluginKind::PlaylistExporter => {
                InstanceScope::PerJob
            }
            PluginKind::AudioOutput
            | PluginKind::VideoOutput
            | PluginKind::MetadataProvider
            | PluginKind::ServiceDiscovery
            | PluginKind::UiPanel => InstanceScope::AppSingleton,
        }
    }

    /// May this kind load as a **WASM extension** in v1? (doc 06 §1.4 —
    /// the hard registry rule `[PROPOSAL]`; the host refuses the rest
    /// with a typed error.)
    ///
    /// The doc's two lists leave `subtitle-parser` and
    /// `subtitle-renderer` in **neither** column; this crate classifies
    /// them as builtin-only (they sit on the demux-adjacent parse path)
    /// — `[PROPOSAL]` for the Lead reconciliation.
    pub const fn extension_allowed_in_v1(self) -> bool {
        matches!(
            self,
            PluginKind::UiPanel
                | PluginKind::PlaylistParser
                | PluginKind::PlaylistExporter
                | PluginKind::MetadataProvider
                | PluginKind::ServiceDiscovery
                | PluginKind::AudioFilter
                | PluginKind::VideoFilter
        )
    }
}

impl fmt::Display for PluginKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Instance scope — who owns construction and teardown (doc 06 §1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstanceScope {
    /// One instance per open source: `access`, `network-protocol`.
    PerOpenSource,
    /// One per playback session (decoder: per track): `demuxer`,
    /// `decoder`, `subtitle-parser`, `subtitle-renderer`,
    /// `audio-filter`, `video-filter`.
    PerPlaybackSession,
    /// One per job / per call: `encoder`, `playlist-parser`,
    /// `playlist-exporter`.
    PerJob,
    /// App singletons: `audio-output`, `video-output`,
    /// `metadata-provider`, `service-discovery`, `ui-panel`.
    AppSingleton,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn sixteen_kinds_in_doc_order() {
        assert_eq!(PluginKind::ALL.len(), 16, "doc 06 §1.2: sixteen kinds");
        let spellings: Vec<&str> = PluginKind::ALL.iter().map(|k| k.as_str()).collect();
        assert_eq!(
            spellings,
            [
                "access",
                "network-protocol",
                "demuxer",
                "decoder",
                "encoder",
                "audio-filter",
                "video-filter",
                "audio-output",
                "video-output",
                "subtitle-parser",
                "subtitle-renderer",
                "playlist-parser",
                "playlist-exporter",
                "metadata-provider",
                "ui-panel",
                "service-discovery",
            ]
        );
    }

    #[test]
    fn spellings_are_unique_kebab_case() {
        let mut seen = HashSet::new();
        for kind in PluginKind::ALL {
            let s = kind.as_str();
            assert!(!seen.contains(s), "duplicate spelling {s}");
            seen.insert(s);
            assert!(
                s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{s} is kebab-case"
            );
            assert!(!s.contains("__"), "{s} no double underscores");
            assert!(!s.starts_with('-') && !s.ends_with('-'), "{s} edges");
            assert_eq!(kind.to_string(), s, "Display = manifest string");
        }
    }

    #[test]
    fn v1_extension_policy_matches_doc_lists() {
        // Allowed list (doc 06 §1.4): ui-panel, playlist-parser,
        // playlist-exporter, metadata-provider, service-discovery,
        // audio-filter, video-filter.
        let allowed: Vec<&str> = PluginKind::ALL
            .iter()
            .filter(|k| k.extension_allowed_in_v1())
            .map(|k| k.as_str())
            .collect();
        assert_eq!(
            allowed,
            [
                "audio-filter",
                "video-filter",
                "playlist-parser",
                "playlist-exporter",
                "metadata-provider",
                "ui-panel",
                "service-discovery",
            ]
        );
        // Refused list: access, network-protocol, demuxer, decoder,
        // encoder, audio-output, video-output — plus the two
        // subtitle kinds the doc leaves unlisted [PROPOSAL].
        let refused: Vec<&str> = PluginKind::ALL
            .iter()
            .filter(|k| !k.extension_allowed_in_v1())
            .map(|k| k.as_str())
            .collect();
        assert_eq!(
            refused,
            [
                "access",
                "network-protocol",
                "demuxer",
                "decoder",
                "encoder",
                "audio-output",
                "video-output",
                "subtitle-parser",
                "subtitle-renderer",
            ]
        );
    }

    #[test]
    fn instance_scopes_match_doc_table() {
        for (kind, want) in [
            (PluginKind::Access, InstanceScope::PerOpenSource),
            (PluginKind::NetworkProtocol, InstanceScope::PerOpenSource),
            (PluginKind::Demuxer, InstanceScope::PerPlaybackSession),
            (PluginKind::Decoder, InstanceScope::PerPlaybackSession),
            (
                PluginKind::SubtitleParser,
                InstanceScope::PerPlaybackSession,
            ),
            (
                PluginKind::SubtitleRenderer,
                InstanceScope::PerPlaybackSession,
            ),
            (PluginKind::AudioFilter, InstanceScope::PerPlaybackSession),
            (PluginKind::VideoFilter, InstanceScope::PerPlaybackSession),
            (PluginKind::Encoder, InstanceScope::PerJob),
            (PluginKind::PlaylistParser, InstanceScope::PerJob),
            (PluginKind::PlaylistExporter, InstanceScope::PerJob),
            (PluginKind::AudioOutput, InstanceScope::AppSingleton),
            (PluginKind::VideoOutput, InstanceScope::AppSingleton),
            (PluginKind::MetadataProvider, InstanceScope::AppSingleton),
            (PluginKind::ServiceDiscovery, InstanceScope::AppSingleton),
            (PluginKind::UiPanel, InstanceScope::AppSingleton),
        ] {
            assert_eq!(kind.instance_scope(), want, "{kind}");
        }
    }
}
