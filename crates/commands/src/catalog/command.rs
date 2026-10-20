//! The command payload enum and the typed results (doc 05 §2.3–2.12 —
//! card 1.D4).
//!
//! [`Command`] is the payload-carrying twin of
//! [`CommandId`](crate::catalog::ids::CommandId): one variant per
//! catalog entry, variant payloads taken verbatim from the §2 tables'
//! *Params* columns. It is adjacently tagged — `"cmd"` carries the
//! stable id, `"args"` the payload — so inside
//! [`CommandRequest<Command>`](crate::envelope::CommandRequest) the
//! wire object is exactly the §5.1 example:
//!
//! ```json
//! { "requestId": 42, "cmd": "playback.seek",
//!   "args": { "target": { "kind": "relative", "deltaMs": -10000 } } }
//! ```
//!
//! [`CommandResult`] carries the §2 tables' *Result / Completion*
//! payloads for class-S commands (class-A commands answer `Accepted`
//! and complete via events, §1.4 — they have no completed payload).
//! Result variants are shared by shape, not per command: three commands
//! "apply a delay in ms", one `applied-delay-ms` variant serves all of
//! them. `[VERIFY]` — payloads doc 05 sketches only in prose
//! (`EngineInfo.plugins`, `SettingValue`, `StateSnapshot`'s blocks)
//! are marked at their types in [`types`](crate::catalog::types).

use std::collections::BTreeMap;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::catalog::spec::CompletionClass;
use crate::catalog::types::AspectRatioSpec;
use crate::catalog::types::AudioDevice;
use crate::catalog::types::Chapter;
use crate::catalog::types::ConfigAreaInfo;
use crate::catalog::types::CropSpec;
use crate::catalog::types::Duration;
use crate::catalog::types::EngineInfo;
use crate::catalog::types::LogLevel;
use crate::catalog::types::MediaInfo;
use crate::catalog::types::OpenOptions;
use crate::catalog::types::PlaylistEnqueueItem;
use crate::catalog::types::PlaylistItemId;
use crate::catalog::types::PlaylistSaveFormat;
use crate::catalog::types::Rate;
use crate::catalog::types::RepeatMode;
use crate::catalog::types::SeekTarget;
use crate::catalog::types::SettingValue;
use crate::catalog::types::SourceSpec;
use crate::catalog::types::StateSnapshot;
use crate::catalog::types::StepDirection;
use crate::catalog::types::TimeRange;
use crate::catalog::types::Timestamp;
use crate::catalog::types::TrackId;
use crate::catalog::types::TrackInfo;
use crate::catalog::types::TrackSelector;
use crate::catalog::types::Volume;

/// One host → engine command, with its args (doc 05 §2 tables).
///
/// `#[non_exhaustive]` per §2.1: future minor versions add variants;
/// unknown ids are rejected with `invalid-argument` (V1) — serde does
/// that for the `"cmd"` tag automatically.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "cmd", content = "args", rename_all = "kebab-case")]
pub enum Command {
    // ---- session (§2.3) ----
    /// `session.open` — open a source as a new session (class A).
    #[serde(rename = "session.open")]
    SessionOpen {
        /// What to open.
        source: SourceSpec,
        /// Open-time options (§2.3, schema-validated).
        #[serde(default)]
        options: OpenOptions,
    },
    /// `session.close` — close the session (class S, gen-bumping,
    /// idempotent).
    #[serde(rename = "session.close")]
    SessionClose,

    // ---- playback (§2.4) ----
    /// `playback.play` — play / resume / restart-from-ended (class A).
    #[serde(rename = "playback.play")]
    PlaybackPlay,
    /// `playback.pause` — pause (class A, idempotent).
    #[serde(rename = "playback.pause")]
    PlaybackPause,
    /// `playback.toggle-play-pause` — the primary keymap target (A).
    #[serde(rename = "playback.toggle-play-pause")]
    PlaybackTogglePlayPause,
    /// `playback.stop` — tear down the pipeline; the session survives
    /// (class A, gen-bumping).
    #[serde(rename = "playback.stop")]
    PlaybackStop,
    /// `playback.seek` — seek (class A, gen-bumping).
    #[serde(rename = "playback.seek")]
    PlaybackSeek {
        /// Where to jump.
        target: SeekTarget,
    },
    /// `playback.set-rate` — atomic rate change (class S).
    #[serde(rename = "playback.set-rate")]
    PlaybackSetRate {
        /// The requested rate.
        rate: Rate,
    },
    /// `playback.step-frame` — auto-pause then step one frame (A, gen).
    #[serde(rename = "playback.step-frame")]
    PlaybackStepFrame {
        /// Step forwards or backwards.
        direction: StepDirection,
    },
    /// `playback.set-position` — seek by fraction (class A, gen-bumping;
    /// slider/scrubber ergonomics, §2.4).
    #[serde(rename = "playback.set-position")]
    PlaybackSetPosition {
        /// `0.0..=1.0` of the duration.
        fraction: f32,
    },

    // ---- tracks (§2.5) ----
    /// `tracks.list` — the live track list (class S).
    #[serde(rename = "tracks.list")]
    TracksList,
    /// `tracks.get-info` — one track's info (class S).
    #[serde(rename = "tracks.get-info")]
    TracksGetInfo {
        /// Which track.
        id: TrackId,
    },
    /// `tracks.set-video-track` — decoder re-init (class A, gen).
    #[serde(rename = "tracks.set-video-track")]
    TracksSetVideoTrack {
        /// `Id`/`None` per §2.5.
        selector: TrackSelector,
    },
    /// `tracks.set-audio-track` — decoder re-init (class A, gen).
    #[serde(rename = "tracks.set-audio-track")]
    TracksSetAudioTrack {
        /// `Id`/`None` per §2.5.
        selector: TrackSelector,
    },
    /// `tracks.set-subtitle-track` — selection over parsed streams
        /// (class S — no pipeline invalidation).
    #[serde(rename = "tracks.set-subtitle-track")]
    TracksSetSubtitleTrack {
        /// `Id`/`None`/`Auto` per §2.5.
        selector: TrackSelector,
    },
    /// `tracks.set-secondary-audio` — mix a second audio track (A).
    #[serde(rename = "tracks.set-secondary-audio")]
    TracksSetSecondaryAudio {
        /// `Id`/`None` per §2.5.
        selector: TrackSelector,
    },

    // ---- audio (§2.6) ----
    /// `audio.set-volume` — the response says what IS (class S).
    #[serde(rename = "audio.set-volume")]
    AudioSetVolume {
        /// Requested volume.
        volume: Volume,
    },
    /// `audio.get-volume` — prefer the snapshot on hydration (S).
    #[serde(rename = "audio.get-volume")]
    AudioGetVolume,
    /// `audio.set-mute` — independent of volume (class S).
    #[serde(rename = "audio.set-mute")]
    AudioSetMute {
        /// Mute or unmute.
        muted: bool,
    },
    /// `audio.list-devices` — exactly one pseudo-device on web (S).
    #[serde(rename = "audio.list-devices")]
    AudioListDevices,
    /// `audio.set-device` — output-stream migration (class A, no gen).
    #[serde(rename = "audio.set-device")]
    AudioSetDevice {
        /// Device id from `audio.list-devices`.
        id: String,
    },
    /// `audio.set-delay` — atomic sync parameter (class S).
    #[serde(rename = "audio.set-delay")]
    AudioSetDelay {
        /// Signed delay (ms), ±60 000.
        delay_ms: i64,
    },

    // ---- video (§2.7) ----
    /// `video.set-fullscreen` — **target: shell** (class S; the only
    /// shell-routed command in v1, §1.1.3).
    #[serde(rename = "video.set-fullscreen")]
    VideoSetFullscreen {
        /// Enter or leave fullscreen.
        enabled: bool,
    },
    /// `video.set-aspect-ratio` — renderer parameter (class S).
    #[serde(rename = "video.set-aspect-ratio")]
    VideoSetAspectRatio {
        /// `Default` or an explicit ratio.
        ratio: AspectRatioSpec,
    },
    /// `video.set-crop` — renderer parameter (class S).
    #[serde(rename = "video.set-crop")]
    VideoSetCrop {
        /// `None`/`Ratio`/`Rect`.
        crop: CropSpec,
    },
    /// `video.set-zoom` — display scaling, `0.25..=4.0` (class S).
    #[serde(rename = "video.set-zoom")]
    VideoSetZoom {
        /// Zoom factor.
        factor: f32,
    },
    /// `video.set-delay` — atomic sync parameter (class S).
    #[serde(rename = "video.set-delay")]
    VideoSetDelay {
        /// Signed delay (ms), ±60 000.
        delay_ms: i64,
    },
    /// `video.take-screenshot` — captures the next presented frame (A).
    #[serde(rename = "video.take-screenshot")]
    VideoTakeScreenshot {
        /// Explicit output path; `None` uses `video.screenshot-dir` +
        /// the filename template (config). `[PROPOSAL]` web path policy
        /// (§2.7 — the binding converts to a browser download).
        path: Option<PathBuf>,
    },

    // ---- subtitles (§2.8) ----
    /// `subtitle.add-file` — external subtitle file (class A).
    /// `[VERIFY]` — `PathSpec` (§2.8) is a plain path string here; the
    /// web host's transferred-`Blob` variant needs the binding-layer
    /// param extension before apps/web consumes it.
    #[serde(rename = "subtitle.add-file")]
    SubtitleAddFile {
        /// Path to the subtitle file.
        path: String,
        /// Explicit encoding override.
        encoding: Option<String>,
        /// Whether to select the track once parsed (default true).
        #[serde(default = "default_true")]
        select: bool,
    },
    /// `subtitle.set-sync` — relative shift; the result carries the new
    /// absolute delay (class S).
    #[serde(rename = "subtitle.set-sync")]
    SubtitleSetSync {
        /// Signed relative adjustment (ms).
        delta_ms: i64,
    },
    /// `subtitle.set-encoding` — re-decode external text subtitles (S).
    #[serde(rename = "subtitle.set-encoding")]
    SubtitleSetEncoding {
        /// IANA encoding name.
        encoding: String,
    },
    /// `subtitle.set-scale` — render size multiplier, `0.2..=5.0` (S).
    #[serde(rename = "subtitle.set-scale")]
    SubtitleSetScale {
        /// Scale factor.
        scale: f32,
    },

    // ---- playlist (§2.9) ----
    /// `playlist.enqueue` — model edit, does not open (class S).
    #[serde(rename = "playlist.enqueue")]
    PlaylistEnqueue {
        /// What to enqueue.
        source: SourceSpec,
        /// Display title override.
        title: Option<String>,
    },
    /// `playlist.enqueue-many` — atomic all-or-nothing enqueue (S).
    #[serde(rename = "playlist.enqueue-many")]
    PlaylistEnqueueMany {
        /// The items (non-empty; empty is `invalid-argument`).
        items: Vec<PlaylistEnqueueItem>,
    },
    /// `playlist.play-item` — full open lifecycle on that item (A).
    #[serde(rename = "playlist.play-item")]
    PlaylistPlayItem {
        /// Which item.
        id: PlaylistItemId,
    },
    /// `playlist.next` — open lifecycle of the next item (class A).
    #[serde(rename = "playlist.next")]
    PlaylistNext,
    /// `playlist.prev` — symmetric, VLC-like restart rule (class A).
    #[serde(rename = "playlist.prev")]
    PlaylistPrev,
    /// `playlist.skip` — open lifecycle of item at `current + n` (A).
    #[serde(rename = "playlist.skip")]
    PlaylistSkip {
        /// Signed offset; negative goes back.
        n: i32,
    },
    /// `playlist.remove` — removing the playing item starts the advance
    /// lifecycle (class S for the model edit).
    #[serde(rename = "playlist.remove")]
    PlaylistRemove {
        /// Which item.
        id: PlaylistItemId,
    },
    /// `playlist.move` — reorder (class S).
    #[serde(rename = "playlist.move")]
    PlaylistMove {
        /// Which item.
        id: PlaylistItemId,
        /// Target index.
        to_index: u32,
    },
    /// `playlist.clear` — stops playback if an item was playing (S).
    #[serde(rename = "playlist.clear")]
    PlaylistClear,
    /// `playlist.set-shuffle` — affects advancement order only (S).
    #[serde(rename = "playlist.set-shuffle")]
    PlaylistSetShuffle {
        /// New shuffle flag.
        enabled: bool,
    },
    /// `playlist.set-repeat` — repeat mode (class S).
    #[serde(rename = "playlist.set-repeat")]
    PlaylistSetRepeat {
        /// New mode.
        mode: RepeatMode,
    },
    /// `playlist.save` — serialize the list (class S).
    #[serde(rename = "playlist.save")]
    PlaylistSave {
        /// Output file (format defaults from the extension).
        path: PathBuf,
        /// Explicit format override.
        format: Option<PlaylistSaveFormat>,
    },
    /// `playlist.load` — parse + insert, nothing opens (class S).
    #[serde(rename = "playlist.load")]
    PlaylistLoad {
        /// Playlist file to read.
        path: PathBuf,
        /// Append instead of replace.
        append: bool,
    },

    // ---- media information (§2.10) ----
    /// `media.get-info` — the metadata snapshot (class S).
    #[serde(rename = "media.get-info")]
    MediaGetInfo,
    /// `media.get-current-time` — snapshot query (class S).
    #[serde(rename = "media.get-current-time")]
    MediaGetCurrentTime,
    /// `media.get-duration` — `Unknown` may become known later (S).
    #[serde(rename = "media.get-duration")]
    MediaGetDuration,
    /// `media.get-chapters` — chapter markers (class S).
    #[serde(rename = "media.get-chapters")]
    MediaGetChapters,
    /// `media.set-chapter` — canonical chapter jump (class A, gen).
    #[serde(rename = "media.set-chapter")]
    MediaSetChapter {
        /// Chapter index.
        index: u32,
    },

    // ---- configuration (§2.11) ----
    /// `config.get-setting` — one setting's value + schema meta (S).
    #[serde(rename = "config.get-setting")]
    ConfigGetSetting {
        /// Settings area.
        area: String,
        /// Setting key.
        key: String,
    },
    /// `config.set-setting` — never silent coercion (class S).
    #[serde(rename = "config.set-setting")]
    ConfigSetSetting {
        /// Settings area.
        area: String,
        /// Setting key.
        key: String,
        /// The value to write.
        value: SettingValue,
    },
    /// `config.reset-setting` — to the layer default (class S).
    #[serde(rename = "config.reset-setting")]
    ConfigResetSetting {
        /// Settings area.
        area: String,
        /// Setting key.
        key: String,
    },
    /// `config.list-areas` — drives the settings screen (class S).
    #[serde(rename = "config.list-areas")]
    ConfigListAreas,

    // ---- system (§2.12) ----
    /// `system.get-engine-info` — hardware decode is reported here (S).
    #[serde(rename = "system.get-engine-info")]
    SystemGetEngineInfo,
    /// `system.list-plugins` — registry from manifests (class S).
    #[serde(rename = "system.list-plugins")]
    SystemListPlugins,
    /// `system.plugin-enable` — enable a *registered* plugin (S).
    #[serde(rename = "system.plugin-enable")]
    SystemPluginEnable {
        /// Plugin id.
        id: String,
    },
    /// `system.plugin-disable` — takes effect next session if in use (S).
    #[serde(rename = "system.plugin-disable")]
    SystemPluginDisable {
        /// Plugin id.
        id: String,
    },
    /// `system.set-log-level` — omit target for global (class S).
    #[serde(rename = "system.set-log-level")]
    SystemSetLogLevel {
        /// New level.
        level: LogLevel,
        /// Module-path target, `None` for global.
        target: Option<String>,
    },
    /// `system.get-state-snapshot` — full hydration (class S).
    #[serde(rename = "system.get-state-snapshot")]
    SystemGetStateSnapshot,
    /// `system.list-commands` — machine-readable introspection (S).
    #[serde(rename = "system.list-commands")]
    SystemListCommands,
}

/// `select` default for [`Command::SubtitleAddFile`] (§2.8: default
/// true). A named function, not a closure — `#[serde(default = "…")]`.
fn default_true() -> bool {
    true
}

/// The completed payload of a class-S command (doc 05 §2 tables'
/// *Result* column; class-A commands never produce one — §1.4 C2/C3).
///
/// Internally tagged (`"kind"`) so the wire form is
/// `{ "completed": { "kind": "audio-volume", "volume": 0.8 } }` inside
/// [`CommandOutcome::Completed`](crate::envelope::CommandOutcome).
/// Variants are shared by shape across commands (see the module docs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CommandResult {
    /// Nothing to report (`session.close`, playlist model edits).
    Unit,
    /// `playback.set-rate` — the applied rate.
    AppliedRate {
        /// The rate now in effect.
        rate: Rate,
    },
    /// `tracks.list` — the live list.
    TrackList {
        /// Every track of the current media.
        tracks: Vec<TrackInfo>,
    },
    /// `tracks.get-info` — one track.
    TrackInfo {
        /// The track's info.
        info: TrackInfo,
    },
    /// `tracks.set-subtitle-track` — the applied selector.
    AppliedSelector {
        /// What is now selected.
        selector: TrackSelector,
    },
    /// `audio.set-volume`/`get-volume` — the applied volume (the truth,
    /// after device clipping).
    AudioVolume {
        /// The volume now in effect.
        volume: Volume,
    },
    /// `audio.set-mute`, `video.set-fullscreen`, `playlist.set-shuffle`
    /// — an applied boolean.
    AppliedBool {
        /// The value now in effect.
        value: bool,
    },
    /// `audio.list-devices` — the device list.
    AudioDevices {
        /// Every available output device.
        devices: Vec<AudioDevice>,
    },
    /// `audio.set-delay`, `video.set-delay`, `subtitle.set-sync` — an
    /// applied delay in ms (for `set-sync`, the new absolute total).
    AppliedDelayMs {
        /// The delay now in effect.
        delay_ms: i64,
    },
    /// `video.set-aspect-ratio` — the applied ratio.
    AppliedAspectRatio {
        /// What is now in effect.
        ratio: AspectRatioSpec,
    },
    /// `video.set-crop` — the applied crop.
    AppliedCrop {
        /// What is now in effect.
        crop: CropSpec,
    },
    /// `video.set-zoom` — the applied factor.
    AppliedZoom {
        /// The factor now in effect.
        factor: f32,
    },
    /// `subtitle.set-encoding` — the applied encoding.
    AppliedEncoding {
        /// The encoding now in effect.
        encoding: String,
    },
    /// `subtitle.set-scale` — the applied scale.
    AppliedScale {
        /// The scale now in effect.
        scale: f32,
    },
    /// `playlist.set-repeat` — the applied mode.
    AppliedRepeat {
        /// The mode now in effect.
        mode: RepeatMode,
    },
    /// `playlist.enqueue` — the new item's id.
    PlaylistItem {
        /// The enqueued item's handle.
        id: PlaylistItemId,
    },
    /// `playlist.enqueue-many`, `playlist.load` — the item ids.
    PlaylistItems {
        /// The handles, in order.
        ids: Vec<PlaylistItemId>,
    },
    /// `media.get-info` — the metadata snapshot.
    MediaInfo {
        /// The snapshot.
        info: MediaInfo,
    },
    /// `media.get-current-time` — position + buffered ranges.
    CurrentTime {
        /// Current position (µs).
        time: Timestamp,
        /// `0.0..=1.0` of duration.
        position: f32,
        /// Buffered ranges.
        buffered: Vec<TimeRange>,
    },
    /// `media.get-duration` — known, unknown or infinite.
    Duration {
        /// The duration.
        duration: Duration,
    },
    /// `media.get-chapters` — chapter markers.
    Chapters {
        /// Every chapter.
        chapters: Vec<Chapter>,
    },
    /// `config.get-setting` — value + schema metadata.
    /// `[VERIFY]` — `schema_meta` is a free-form string map until the
    /// settings-schema crate defines its wire projection.
    SettingResult {
        /// The current value.
        value: SettingValue,
        /// Schema metadata (type, constraints, restart flag…).
        schema_meta: BTreeMap<String, String>,
    },
    /// `config.set-setting`/`reset-setting` — the applied value.
    SettingApplied {
        /// The value now in effect.
        applied: SettingValue,
        /// Whether a restart is required for full effect.
        restart_required: bool,
    },
    /// `config.list-areas` — the settings-screen navigation.
    ConfigAreas {
        /// Every area.
        areas: Vec<ConfigAreaInfo>,
    },
    /// `system.get-engine-info` — the engine's self-description.
    EngineInfo {
        /// The snapshot.
        info: EngineInfo,
    },
    /// `system.list-plugins` — the plugin registry.
    PluginList {
        /// Every registered plugin.
        plugins: Vec<crate::catalog::types::PluginInfo>,
    },
    /// `system.plugin-enable`/`plugin-disable` — the toggle outcome.
    PluginToggle {
        /// Whether the plugin is now enabled.
        applied: bool,
        /// When the toggle takes effect (per manifest).
        takes_effect: Option<String>,
        /// Whether a restart is required.
        restart_required: bool,
    },
    /// `system.set-log-level` — the applied level.
    AppliedLogLevel {
        /// The level now in effect.
        level: LogLevel,
    },
    /// `system.get-state-snapshot` — full hydration.
    StateSnapshot {
        /// The snapshot.
        snapshot: StateSnapshot,
    },
    /// `system.list-commands` — catalog introspection.
    CommandListings {
        /// Every catalog command.
        commands: Vec<CommandListing>,
    },
}

/// One row of `system.list-commands` (§2.12, `[PROPOSAL]` per doc 05):
/// lets UIs and the control server build menus and validate keymaps
/// without hardcoding the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandListing {
    /// The stable command id.
    pub id: String,
    /// Completion class, `"s"` or `"a"` (§1.5).
    pub class: CompletionClass,
    /// Catalog version the command appeared in (`"1"` for the v1
    /// namespace, §6.1).
    pub since: String,
    /// Deprecation note, when deprecated (§6.3).
    pub deprecated: Option<String>,
    /// Other command ids this one requires to be usable (empty in v1).
    pub requires: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::types::AudioTrackInfo;
    use crate::catalog::types::TrackKind;
    use crate::catalog::types::TrackOrigin;
    use crate::catalog::types::VideoTrackInfo;
    use crate::envelope::CommandRequest;
    use crate::envelope::RequestId;

    /// The §5.1 request example, byte for byte (modulo key order, which
    /// JSON does not define).
    #[test]
    fn request_wire_form_matches_doc_example() {
        let request = CommandRequest {
            request_id: RequestId(42),
            command: Command::PlaybackSeek {
                target: SeekTarget::Relative { delta_ms: -10_000 },
            },
        };
        let json = serde_json::to_string(&request).unwrap_or_default();
        assert_eq!(
            json,
            r#"{"requestId":42,"cmd":"playback.seek","args":{"target":{"kind":"relative","deltaMs":-10000}}}"#
        );
        match serde_json::from_str::<CommandRequest<Command>>(&json) {
            Ok(back) => assert_eq!(back, request),
            Err(err) => unreachable!("round-trip failed: {err}"),
        }
    }

    #[test]
    fn unknown_command_ids_are_rejected() {
        let json = r#"{"requestId":1,"cmd":"nope.nope","args":{}}"#;
        match serde_json::from_str::<CommandRequest<Command>>(json) {
            Ok(_) => unreachable!("unknown cmd accepted"),
            Err(err) => assert!(
                err.to_string().contains("variant"),
                "unexpected error: {err}"
            ),
        }
    }

    #[test]
    fn unit_commands_take_no_args() {
        let json = r#"{"requestId":7,"cmd":"playback.pause"}"#;
        match serde_json::from_str::<CommandRequest<Command>>(json) {
            Ok(back) => assert_eq!(
                back.command,
                Command::PlaybackPause,
                "unit command must parse without args"
            ),
            Err(err) => unreachable!("pause rejected: {err}"),
        }
    }

    #[test]
    fn result_variants_round_trip() {
        for (result, json) in [
            (CommandResult::Unit, r#"{"kind":"unit"}"#),
            (
                CommandResult::AudioVolume { volume: 1.25 },
                r#"{"kind":"audio-volume","volume":1.25}"#,
            ),
            (
                CommandResult::TrackList { tracks: Vec::new() },
                r#"{"kind":"track-list","tracks":[]}"#,
            ),
            (
                CommandResult::AppliedSelector {
                    selector: TrackSelector::Auto,
                },
                r#"{"kind":"applied-selector","selector":{"kind":"auto"}}"#,
            ),
        ] {
            assert_eq!(serde_json::to_string(&result).unwrap_or_default(), json);
            match serde_json::from_str::<CommandResult>(json) {
                Ok(back) => assert_eq!(back, result),
                Err(err) => unreachable!("round-trip failed for {json}: {err}"),
            }
        }
    }

    /// The `VideoTrackInfo`/`AudioTrackInfo`/`SessionSummary`/
    /// `DecoderInfo` types are exercised only via schema emission in the
    /// golden tests; this pins one deep payload through serde.
    #[test]
    fn deep_track_info_round_trips() {
        let track = TrackInfo {
            id: TrackId(2),
            kind: crate::catalog::types::TrackKind::Video,
            name: Some("主标题".to_owned()),
            language: Some("zh-Hans".to_owned()),
            codec: Default::default(),
            is_default: true,
            is_selected: true,
            origin: crate::catalog::types::TrackOrigin::Stream,
            video: Some(VideoTrackInfo {
                width: 1920,
                height: 1080,
                frame_rate: Some(23.976),
                bit_depth: Some(10),
                hdr: true,
            }),
            audio: Some(AudioTrackInfo {
                channels: 2,
                sample_rate: Some(48_000),
                bit_depth: None,
                codec_params: None,
            }),
        };
        let result = CommandResult::TrackInfo {
            info: track.clone(),
        };
        match serde_json::from_value(serde_json::to_value(&result).unwrap_or_default()) {
            Ok(CommandResult::TrackInfo { info }) => assert_eq!(info, track),
            Ok(other) => unreachable!("wrong variant: {other:?}"),
            Err(err) => unreachable!("round-trip failed: {err}"),
        }
    }
}
