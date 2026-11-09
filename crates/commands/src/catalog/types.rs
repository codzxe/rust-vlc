//! The catalog's common parameter and result types (doc 05 §2.2, plus
//! the payload shapes the §2.3–2.12 tables and §3.2 event table define
//! — card 1.D4).
//!
//! These are **wire types**: the versioned, host-facing contract of the
//! one command layer (§1.1). They are deliberately distinct from
//! `media-types`' engine-internal vocabulary — §2.2's `Timestamp` is
//! integer **microseconds**, `Rate`/`Volume` are `f32` wire scalars,
//! [`Duration`] has an `Infinite` variant, and the id newtypes expose
//! their raw fields. The engine converts at its boundary (Phase 2);
//! the wire never changes shape under it (§6).
//!
//! Wire forms (§5.1): `rename_all = "camelCase"` on structs, internally
//! tagged enums (`kind`) with kebab-case variant names. V1 closed
//! shapes: plain structs carry `deny_unknown_fields`; enum payloads
//! reject unknown ids via their tags. `Auto`/`None` selectors and
//! `Duration::Unknown`/`Infinite` are explicit variants, never sentinels
//! (V3).

use std::collections::BTreeMap;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::envelope::Generation;
use crate::envelope::SessionId;

/// Microseconds since media start; can be negative only in
/// [`SeekTarget::Relative`] (§2.2). A bare integer on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Timestamp(pub i64);

/// A duration that may not be known (or may be unbounded, for streams) —
/// never a sentinel number (§2.2, V3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Duration {
    /// A known duration, in microseconds.
    Known {
        /// Media duration (µs).
        time: Timestamp,
    },
    /// Not (yet) known — may later become known (`timing.duration-known`).
    Unknown,
    /// Unbounded stream.
    Infinite,
}

/// Linear volume, `0.0..=audio.max-volume` (config, default 1.25)
/// (§2.2, `[PROPOSAL]` range per doc 05).
pub type Volume = f32;

/// Playback rate, `(0.0, playback.max-rate]` (config, default 16.0)
/// (§2.2, `[PROPOSAL]` range per doc 05).
pub type Rate = f32;

/// Engine track handle, stable within one session and never reused
/// inside it (§2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct TrackId(pub u32);

/// Playlist item handle, stable within the playlist lifetime (§2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct PlaylistItemId(pub u64);

/// What a track carries (§2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TrackKind {
    /// Video pictures.
    Video,
    /// Audio samples.
    Audio,
    /// Subtitle cues.
    Subtitle,
    /// Chapters, timed metadata and other non-AV streams.
    Data,
}

/// Track selection (§2.2): a specific track, no track, or (meaningful
/// for subtitles only) the track-selection rules from `product.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TrackSelector {
    /// Select the track with this id.
    Id {
        /// Track handle.
        id: TrackId,
    },
    /// Select no track.
    None,
    /// Apply the product's track-selection rules (subtitles only).
    Auto,
}

/// Where a track came from (§2.2 `TrackInfo`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TrackOrigin {
    /// A stream inside the container.
    Stream,
    /// An external subtitle file added after open.
    ExternalSubtitle,
}

/// Clean-room codec descriptor (§2.2: `{ fourcc, description, profile,
/// level }`). All fields optional — containers differ wildly in what
/// they report. `[VERIFY]` — field granularity to be audited against
/// Phase-0 format inventory.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CodecDescriptor {
    /// FourCC when the container carries one.
    pub fourcc: Option<String>,
    /// Human-readable codec name, when known.
    pub description: Option<String>,
    /// Codec profile, when known.
    pub profile: Option<String>,
    /// Codec level, when known.
    pub level: Option<String>,
}

/// Video-specific track facts (§2.2 `TrackInfo.video`). Not `Eq`/`Hash`:
/// `frame_rate` is an `f32` (§2.2's wire scalars are floats by
/// contract; [`SeekTarget`] likewise).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct VideoTrackInfo {
    /// Coded width in pixels.
    pub width: u32,
    /// Coded height in pixels.
    pub height: u32,
    /// Nominal frame rate (frames/s), when known.
    pub frame_rate: Option<f32>,
    /// Bits per component, when known.
    pub bit_depth: Option<u32>,
    /// Whether the stream carries HDR metadata.
    pub hdr: bool,
}

/// Audio-specific track facts (§2.2 `TrackInfo.audio`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AudioTrackInfo {
    /// Channel count.
    pub channels: u16,
    /// Sample rate (Hz), when known.
    pub sample_rate: Option<u32>,
    /// Bits per sample, when known.
    pub bit_depth: Option<u8>,
    /// Opaque codec parameters, when the container carries them.
    pub codec_params: Option<String>,
}

/// One track of the current media (§2.2). All strings are untrusted
/// container metadata (V7). Not `Eq`/`Hash` (contains [`VideoTrackInfo`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TrackInfo {
    /// Track handle.
    pub id: TrackId,
    /// What the track carries.
    pub kind: TrackKind,
    /// Name from container metadata (untrusted text, V7).
    pub name: Option<String>,
    /// BCP-47 language tag when present.
    pub language: Option<String>,
    /// Codec facts.
    pub codec: CodecDescriptor,
    /// Whether the track is the container's default.
    pub is_default: bool,
    /// Whether the track is currently selected.
    pub is_selected: bool,
    /// Where the track came from.
    pub origin: TrackOrigin,
    /// Present for video tracks.
    pub video: Option<VideoTrackInfo>,
    /// Present for audio tracks.
    pub audio: Option<AudioTrackInfo>,
}

/// Where to jump to (§2.2). Wire example (§5.1):
/// `{ "kind": "relative", "deltaMs": -10000 }`. Not `Eq`/`Hash`:
/// `Percent`'s fraction is an `f32` wire scalar.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum SeekTarget {
    /// µs from media start; clamped to `[0, duration]` when the duration
    /// is known (clamping is truth, not failure — §2.4).
    Absolute {
        /// Target position (µs).
        time: Timestamp,
    },
    /// Negative rewinds (§2.4 `playback.seek`).
    Relative {
        /// Signed delta in milliseconds.
        delta_ms: i64,
    },
    /// `0.0..=1.0` of duration (requires a known duration).
    Percent {
        /// Fraction of the duration.
        fraction: f32,
    },
    /// Jump to a chapter start.
    Chapter {
        /// Chapter index.
        index: u32,
    },
}

/// Clean-room source description (§2.2 — no libvlc types). Scheme
/// support is feature-gated (§6.4); `Disc`/`Capture` are future,
/// feature-gated variants, not in v1 availability.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SourceSpec {
    /// A local file.
    File {
        /// Absolute or relative filesystem path.
        path: PathBuf,
    },
    /// A network URL; scheme support is feature-gated (§6.4).
    Url {
        /// The URL, userinfo-free on the wire (V8).
        url: String,
    },
}

/// Options for `session.open` (§2.3): schema-validated; an unknown
/// option is `invalid-argument`. `[VERIFY]` — the "track selectors"
/// bullet of §2.3 is rendered as one optional selector per track kind.
/// Not `Eq`/`Hash`: `rate` is an `f32` wire scalar (§2.2 — same class as
/// [`SeekTarget`] and [`VideoTrackInfo`]).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase", default)]
pub struct OpenOptions {
    /// Start at this position (µs) instead of 0.
    pub start_time: Option<Timestamp>,
    /// Open paused instead of playing.
    pub start_paused: bool,
    /// Initial playback rate.
    pub rate: Option<Rate>,
    /// Disable the video track.
    pub no_video: bool,
    /// Disable the audio track.
    pub no_audio: bool,
    /// Pre-select a video track.
    pub video_track: Option<TrackSelector>,
    /// Pre-select an audio track.
    pub audio_track: Option<TrackSelector>,
    /// Pre-select a subtitle track.
    pub subtitle_track: Option<TrackSelector>,
    /// Only select forced subtitles.
    pub forced_subtitle_only: bool,
}

/// One audio output device (§2.6 `audio.list-devices`). On web there is
/// exactly one pseudo-device (`webaudio`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AudioDevice {
    /// Host device id (opaque string).
    pub id: String,
    /// Display name (untrusted text, V7).
    pub name: String,
    /// Whether this is the current default output.
    pub is_default: bool,
}

/// The playback state machine (§3.3). Wire: the lowercase names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PlaybackState {
    /// No session.
    Idle,
    /// A source is being opened/probed.
    Opening,
    /// Decoders are filling their buffers.
    Buffering,
    /// The pipeline is running.
    Playing,
    /// Paused by the user/engine.
    Paused,
    /// Pipeline torn down; session survives (§2.4 `playback.stop`).
    Stopped,
    /// Current media finished.
    Ended,
    /// Terminal failure (always accompanied by a fatal `playback.error`).
    Error,
}

impl PlaybackState {
    /// The §3.3 valid-transition table, answered for `self → other`.
    ///
    /// Covers `idle→opening`, `opening→{buffering, error, idle,
    /// stopped}`, `buffering→{playing, paused, stopped, error}`
    /// (card 3.C1's stop-during-opening, stop-during-buffering and
    /// buffered-pause edges), `playing↔paused`, `playing→buffering`,
    /// `playing/paused→{stopped, ended, error}`,
    /// `stopped/ended/error→idle` and self-transitions (C8).
    pub fn can_transition_to(self, other: PlaybackState) -> bool {
        use PlaybackState as S;
        if self == other {
            return true; // C8 idempotent confirmations
        }
        matches!(
            (self, other),
            (S::Idle, S::Opening)
                | (S::Opening, S::Buffering | S::Error | S::Idle | S::Stopped)
                | (S::Buffering, S::Playing | S::Error | S::Stopped | S::Paused)
                | (
                    S::Playing,
                    S::Paused | S::Buffering | S::Stopped | S::Ended | S::Error
                )
                | (S::Paused, S::Playing | S::Stopped | S::Ended | S::Error)
                | (S::Stopped | S::Ended | S::Error, S::Idle)
        )
    }
}

/// One chapter marker (§2.10 `media.get-chapters`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Chapter {
    /// Chapter index, 0-based.
    pub index: u32,
    /// Chapter start (µs from media start).
    pub time_start: Timestamp,
    /// Chapter length.
    pub duration: Duration,
    /// Chapter title (untrusted text, V7).
    pub title: Option<String>,
}

/// One user bookmark (issue #32 `media.*-bookmark` commands).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Bookmark {
    /// Stable bookmark id — creation-ordered from 1, unique within the
    /// session (never reused after a delete; matches the keymap's
    /// 1-based bookmark slots, keymaps/default.toml `set-bookmark-N`).
    pub id: u32,
    /// Bookmark position (µs from media start).
    pub time: Timestamp,
    /// Bookmark label (untrusted text, V7).
    pub label: Option<String>,
}

/// A media's metadata snapshot (§2.10 `media.get-info`). All strings
/// are untrusted (V7). Not `Eq`/`Hash` (contains [`TrackInfo`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MediaInfo {
    /// The source the session was opened from.
    pub source: SourceSpec,
    /// Container format name, when probed.
    pub container: Option<String>,
    /// Title from container metadata.
    pub title: Option<String>,
    /// Media duration.
    pub duration: Duration,
    /// Whether the media supports seeking.
    pub seekable: bool,
    /// Whether this is a live source.
    pub is_live: bool,
    /// Free-form container metadata (V7 — render as text, never markup).
    pub metadata: BTreeMap<String, String>,
    /// The track list (same data as the latest `tracks.list-ready`).
    pub tracks: Vec<TrackInfo>,
    /// Chapter markers.
    pub chapters: Vec<Chapter>,
}

/// A half-open time interval (µs), e.g. one buffered range (§2.10
/// `media.get-current-time`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TimeRange {
    /// Interval start (µs).
    pub start: Timestamp,
    /// Interval end (µs).
    pub end: Timestamp,
}

/// Aspect-ratio setting for `video.set-aspect-ratio` (§2.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum AspectRatioSpec {
    /// The track's display aspect ratio.
    Default,
    /// An explicit ratio.
    Ratio {
        /// Ratio width.
        w: u32,
        /// Ratio height.
        h: u32,
    },
}

/// Crop setting for `video.set-crop` (§2.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CropSpec {
    /// No crop.
    None,
    /// Crop to a ratio.
    Ratio {
        /// Ratio width.
        w: u32,
        /// Ratio height.
        h: u32,
    },
    /// Crop to a rectangle (pixel offsets from the top-left).
    Rect {
        /// Left offset.
        x: i64,
        /// Top offset.
        y: i64,
        /// Width.
        w: u32,
        /// Height.
        h: u32,
    },
}

/// Frame-step direction (§2.4 `playback.step-frame`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum StepDirection {
    /// Step to the next frame.
    Next,
    /// Step to the previous frame (may be a micro-seek, `[03]`).
    Previous,
}

/// What kind of seek produced a `timing.seek-completed` (§3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SeekKind {
    /// A `playback.seek`/`set-position`/`set-chapter`.
    Command,
    /// A `playback.step-frame`.
    FrameStep,
}

/// Playlist repeat mode (§2.9 `playlist.set-repeat`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RepeatMode {
    /// Advance once, then stop at the list end.
    #[default]
    Off,
    /// Repeat the current item.
    One,
    /// Wrap around the list.
    All,
}

/// Playlist serialization format (§2.9 `playlist.save`). Defaults from
/// the file extension when omitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PlaylistSaveFormat {
    /// M3U8.
    M3u8,
    /// XSPF.
    Xspf,
    /// Lumen's JSON playlist.
    Json,
}

/// One item of `playlist.enqueue-many` (§2.9).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PlaylistEnqueueItem {
    /// What to enqueue (not opened at enqueue time).
    pub source: SourceSpec,
    /// Display title override.
    pub title: Option<String>,
}

/// Log level for `system.set-log-level` and `system.log` (§2.12, §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum LogLevel {
    /// Most verbose.
    Trace,
    /// Debugging detail.
    Debug,
    /// Normal operation.
    Info,
    /// Recoverable problems.
    Warn,
    /// Failures.
    Error,
}

/// A setting value (§2.11 `config.get-setting`/`set-setting`).
/// `[VERIFY]` — the settings-schema crate owns the authoritative value
/// model; this wire shape must be reconciled when the two meet in a
/// host (the shapes coincide for v1 scalar/list values).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum SettingValue {
    /// A boolean setting.
    Bool(bool),
    /// A numeric setting.
    Number(f64),
    /// A string setting.
    String(String),
    /// A list-of-strings setting.
    List(Vec<String>),
}

/// One settings area summary (§2.11 `config.list-areas`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ConfigAreaInfo {
    /// Area id (a settings-schema namespace).
    pub area: String,
    /// Number of settings in the area.
    pub setting_count: u32,
    /// Localization key for the area's display name.
    pub name_key: String,
    /// Whether any setting in the area requires a restart.
    pub any_restart_required: bool,
}

/// One decoder of the engine's registry (§2.12 `system.get-engine-info`
/// — hardware decode is reported here, truthfully).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DecoderInfo {
    /// Decoder name.
    pub name: String,
    /// What the decoder decodes.
    pub kind: TrackKind,
    /// Whether this is a hardware decoder.
    pub hardware: bool,
    /// OS backend name for hardware decoders.
    pub backend: Option<String>,
    /// Whether the decoder is currently in use by a session.
    pub active: bool,
}

/// One live session of the engine (§2.12 `system.get-engine-info`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SessionSummary {
    /// Session handle.
    pub id: SessionId,
    /// Current playback state.
    pub state: PlaybackState,
    /// Current pipeline generation.
    #[serde(rename = "gen")]
    pub generation: Generation,
}

/// The engine's self-description (§2.12 `system.get-engine-info`).
/// `[VERIFY]` — `plugins` is rendered as the id list of registered
/// plugins; `system.list-plugins` carries the full records.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EngineInfo {
    /// Engine version.
    pub engine_version: String,
    /// Command-catalog version (this file's contract, §6.1).
    pub catalog_version: String,
    /// Control-protocol version.
    pub protocol_version: String,
    /// Host the engine is embedded in (tauri/web/cli/control-server/test).
    pub host: String,
    /// Feature-exposure flags enabled for this product (§6.4).
    pub features: Vec<String>,
    /// Ids of registered plugins.
    pub plugins: Vec<String>,
    /// The decoder registry.
    pub decoders: Vec<DecoderInfo>,
    /// Live sessions.
    pub sessions: Vec<SessionSummary>,
}

/// Where a plugin was loaded from (§2.12 `system.list-plugins`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PluginSource {
    /// Compiled into the engine binary.
    Builtin,
    /// A sandboxed WASM extension.
    Wasm,
}

/// One registered plugin (§2.12 `system.list-plugins`, MP §6).
/// `[VERIFY]` — `kind` stays a string until the plugin taxonomy in
/// doc 06 is frozen into an enum.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PluginInfo {
    /// Plugin id (from its manifest).
    pub id: String,
    /// Plugin kind (manifest-declared).
    pub kind: String,
    /// Plugin version.
    pub version: String,
    /// Declared capabilities.
    pub capabilities: Vec<String>,
    /// Whether the plugin is enabled.
    pub enabled: bool,
    /// Builtin or WASM.
    pub source: PluginSource,
    /// Granted permissions.
    pub permissions: Vec<String>,
}

/// What the engine did after a sandboxed plugin crashed (§3.2
/// `plugins.crashed`, MP §6 error isolation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PluginAction {
    /// The plugin was disabled.
    Disabled,
    /// The plugin was isolated (kept registered, not run again).
    Isolated,
}

/// The payload of `playlist.changed` (§3.2): which model mutation
/// happened, command- or engine-initiated.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PlaylistChange {
    /// Items were appended/inserted.
    ItemsAdded {
        /// The new item ids.
        ids: Vec<PlaylistItemId>,
    },
    /// Items were removed.
    ItemsRemoved {
        /// The removed item ids.
        ids: Vec<PlaylistItemId>,
    },
    /// One item moved.
    Moved {
        /// The moved item.
        id: PlaylistItemId,
        /// Index before the move.
        from: u32,
        /// Index after the move.
        to: u32,
    },
    /// The list was cleared.
    Cleared,
    /// Shuffle was toggled.
    ShuffleSet {
        /// New shuffle flag.
        enabled: bool,
    },
    /// Repeat mode changed.
    RepeatSet {
        /// New repeat mode.
        mode: RepeatMode,
    },
    /// The current item changed.
    CurrentChanged {
        /// The new current item.
        id: PlaylistItemId,
    },
}

/// The `tracks.changed` track kind (§3.2): [`TrackKind`] plus the
/// synthetic `secondary-audio` selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TrackChangeKind {
    /// Video selection.
    Video,
    /// Audio selection.
    Audio,
    /// Subtitle selection.
    Subtitle,
    /// Data selection.
    Data,
    /// Secondary-audio mix (§2.5 `tracks.set-secondary-audio`).
    SecondaryAudio,
}

/// Track-selection state for the snapshot (§2.12
/// `system.get-state-snapshot`). `[VERIFY]` — field set per §2.12's
/// "tracks + selections" bullet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase", default)]
pub struct TrackSelections {
    /// Selected video track.
    pub video: Option<TrackId>,
    /// Selected audio track.
    pub audio: Option<TrackId>,
    /// Selected subtitle track.
    pub subtitle: Option<TrackId>,
    /// Mixed-in secondary audio track.
    pub secondary_audio: Option<TrackId>,
}

/// Playlist summary for the snapshot (§2.12). `[VERIFY]` — "playlist
/// summary + current" rendered as count + current + mode flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase", default)]
pub struct PlaylistSummary {
    /// Number of items in the list.
    pub item_count: u32,
    /// Current item, if one is active.
    pub current: Option<PlaylistItemId>,
    /// Shuffle flag.
    pub shuffle: bool,
    /// Repeat mode.
    pub repeat: RepeatMode,
}

/// Subtitle settings block of the snapshot (§2.12). `[VERIFY]` — the
/// "subtitle settings" bullet rendered as the §2.8 state. Not
/// `Eq`/`Hash` (`scale` is an `f32`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase", default)]
pub struct SubtitleSettings {
    /// Absolute subtitle delay (ms).
    pub delay_ms: i64,
    /// External-subtitle encoding override.
    pub encoding: Option<String>,
    /// Render size multiplier.
    pub scale: f32,
}

/// Full UI hydration (§2.12 `system.get-state-snapshot`, `[PROPOSAL]`
/// per doc 05): everything a reconnecting host needs before it
/// subscribes (§4.4). Called at startup and on reconnect. Not
/// `Eq`/`Hash` (contains `f32` wire scalars).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StateSnapshot {
    /// Current playback state.
    pub state: PlaybackState,
    /// Current position, when a pipeline exists.
    pub time: Option<Timestamp>,
    /// Media duration.
    pub duration: Duration,
    /// Current rate.
    pub rate: Rate,
    /// Current volume.
    pub volume: Volume,
    /// Mute flag.
    pub muted: bool,
    /// Audio delay (ms).
    pub audio_delay_ms: i64,
    /// Video delay (ms).
    pub video_delay_ms: i64,
    /// Subtitle delay (ms).
    pub subtitle_delay_ms: i64,
    /// The track list.
    pub tracks: Vec<TrackInfo>,
    /// Current selections.
    pub selections: TrackSelections,
    /// Playlist summary.
    pub playlist: PlaylistSummary,
    /// Audio output devices.
    pub devices: Vec<AudioDevice>,
    /// Subtitle settings.
    pub subtitle: SubtitleSettings,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seek_target_wire_form_matches_doc_example() {
        let target = SeekTarget::Relative { delta_ms: -10_000 };
        let json = serde_json::to_string(&target).unwrap_or_default();
        assert_eq!(json, r#"{"kind":"relative","deltaMs":-10000}"#);
        match serde_json::from_str::<SeekTarget>(&json) {
            Ok(back) => assert_eq!(back, target),
            Err(err) => unreachable!("round-trip failed: {err}"),
        }
    }

    #[test]
    fn duration_is_never_a_sentinel() {
        let cases = [
            (Duration::Unknown, r#"{"kind":"unknown"}"#),
            (Duration::Infinite, r#"{"kind":"infinite"}"#),
            (
                Duration::Known {
                    time: Timestamp(90_000_000),
                },
                r#"{"kind":"known","time":90000000}"#,
            ),
        ];
        for (value, json) in cases {
            assert_eq!(serde_json::to_string(&value).unwrap_or_default(), json);
            match serde_json::from_str::<Duration>(json) {
                Ok(back) => assert_eq!(back, value),
                Err(err) => unreachable!("round-trip failed for {json}: {err}"),
            }
        }
    }

    #[test]
    fn closed_shapes_reject_unknown_fields() {
        let json = r#"{"id":1,"kind":"audio","bogus":true}"#;
        match serde_json::from_str::<TrackInfo>(json) {
            Ok(_) => unreachable!("unknown field accepted"),
            Err(err) => assert!(err.to_string().contains("unknown field")),
        }
    }

    #[test]
    fn selector_variants_round_trip() {
        for (selector, json) in [
            (
                TrackSelector::Id { id: TrackId(3) },
                r#"{"kind":"id","id":3}"#,
            ),
            (TrackSelector::None, r#"{"kind":"none"}"#),
            (TrackSelector::Auto, r#"{"kind":"auto"}"#),
        ] {
            assert_eq!(serde_json::to_string(&selector).unwrap_or_default(), json);
            match serde_json::from_str::<TrackSelector>(json) {
                Ok(back) => assert_eq!(back, selector),
                Err(err) => unreachable!("round-trip failed for {json}: {err}"),
            }
        }
    }

    /// §3.3's valid-transition table, spot-checked at its edges: every
    /// listed transition is legal, and a few illegal ones are not.
    #[test]
    fn playback_state_transitions_follow_section_3_3() {
        use PlaybackState as S;
        let legal = [
            (S::Idle, S::Opening),
            (S::Opening, S::Buffering),
            (S::Opening, S::Error),
            (S::Opening, S::Idle),
            (S::Buffering, S::Playing),
            (S::Buffering, S::Error),
            (S::Playing, S::Paused),
            (S::Paused, S::Playing),
            (S::Playing, S::Buffering),
            (S::Playing, S::Stopped),
            (S::Paused, S::Ended),
            (S::Stopped, S::Idle),
            (S::Ended, S::Idle),
            (S::Error, S::Idle),
            // Card 3.C1's completion edges.
            (S::Opening, S::Stopped),
            (S::Buffering, S::Stopped),
            (S::Buffering, S::Paused),
        ];
        for (from, to) in legal {
            assert!(from.can_transition_to(to), "{from:?}->{to:?} rejected");
        }
        let illegal = [
            (S::Idle, S::Playing),
            (S::Stopped, S::Playing),
            (S::Ended, S::Buffering),
            (S::Error, S::Playing),
            // Close composes through stop — there is still no direct
            // active→idle edge (the executor macro, not the wire).
            (S::Playing, S::Idle),
            (S::Buffering, S::Idle),
            (S::Paused, S::Idle),
        ];
        for (from, to) in illegal {
            assert!(!from.can_transition_to(to), "{from:?}->{to:?} accepted");
        }
        // C8: self-transitions are always valid confirmations.
        for state in [
            S::Idle,
            S::Opening,
            S::Buffering,
            S::Playing,
            S::Paused,
            S::Stopped,
            S::Ended,
            S::Error,
        ] {
            assert!(state.can_transition_to(state));
        }
    }
}
