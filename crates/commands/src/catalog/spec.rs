//! The catalog as DATA: per-command and per-event metadata rows, the
//! completion classes and the catalog checksum (doc 05 §1.5, §1.1.3,
//! §2 tables, §3.2, §4.2 — card 1.D4).
//!
//! [`catalog_data()`] is the single table the generators and
//! `system.list-commands` consume: one [`CommandSpec`] per §2.1 id (61
//! rows), one [`EventSpec`] per §3.2 id (23 rows). The rows carry the
//! metadata the payload types cannot express — completion class
//! (§1.5), target (§1.1.3: engine vs shell), the gen-bumping flag
//! (§4.2's closed list) and compact arg/result shape descriptors.
//!
//! The whole table folds into a stable FNV-1a checksum
//! ([`catalog_checksum`], [`checksum_of`]) that `cargo xtask codegen`
//! commits alongside the generated artifacts — a one-glance fingerprint
//! for catalog drift. FNV-1a is implemented here (not `DefaultHasher`)
//! because std's default hasher is explicitly NOT stable across
//! releases and the checksum is a committed artifact.
//!
//! Invariants, test-enforced: every id enum variant has exactly one
//! row; rows appear in doc-05 table order; the S/A classification
//! matches §1.5's example lists; the gen-bumping set is exactly §4.2's
//! list; only `video.set-fullscreen` targets the shell.

use crate::catalog::ids::CommandId;
use crate::catalog::ids::EventId;

/// Completion class (doc 05 §1.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CompletionClass {
    /// Synchronous-complete: pure queries; atomic mutations with bounded
    /// work, no pipeline invalidation, no unbounded I/O. The response
    /// IS the completion evidence (C2).
    S,
    /// Async-observed: reconfigures the pipeline, performs unbounded
    /// I/O, or its effect is only observable through engine state.
    /// Answers `Accepted`; completes via a named event (C3).
    A,
}

impl CompletionClass {
    /// The stable wire letter, `"s"`/`"a"` (§1.5, §2.12
    /// `system.list-commands`).
    pub fn as_str(self) -> &'static str {
        match self {
            CompletionClass::S => "s",
            CompletionClass::A => "a",
        }
    }
}

impl std::fmt::Display for CompletionClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Who executes a catalog command (doc 05 §1.1.3): shell-routed
/// commands are still catalog commands, so keymaps, menus, CLI and the
/// control server address them uniformly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CommandTarget {
    /// The engine core (default).
    Engine,
    /// The shell/host (v1: only `video.set-fullscreen`).
    Shell,
}

impl CommandTarget {
    /// The stable wire name.
    pub fn as_str(self) -> &'static str {
        match self {
            CommandTarget::Engine => "engine",
            CommandTarget::Shell => "shell",
        }
    }
}

impl std::fmt::Display for CommandTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A compact, human-stable arg/result shape descriptor. Part of the
/// checksum: changing a payload shape changes the descriptor string,
/// which changes the checksum, which fails `--check` until the
/// artifacts are regenerated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PayloadDesc {
    /// No payload (unit command / `Accepted`-only result).
    None,
    /// A named wire shape, e.g. `"SeekTarget"`, `"Vec<TrackInfo>"`,
    /// or a field signature like `"source: SourceSpec, options:
    /// OpenOptions"` for inline command args.
    Type(&'static str),
}

impl PayloadDesc {
    /// The descriptor text (`"none"` / the type string).
    pub fn as_str(self) -> &'static str {
        match self {
            PayloadDesc::None => "none",
            PayloadDesc::Type(text) => text,
        }
    }
}

/// One catalog command row (doc 05 §2 tables, one per §2.1 id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandSpec {
    /// The catalog id.
    pub id: CommandId,
    /// Completion class (§1.5).
    pub class: CompletionClass,
    /// Who executes it (§1.1.3).
    pub target: CommandTarget,
    /// Whether accepting it bumps the session generation (§4.2).
    pub gen_bumping: bool,
    /// The args shape (§2 tables' *Params* column, normalized).
    pub args: PayloadDesc,
    /// The completed-result shape (§2 tables' *Result* column; class-A
    /// commands never complete in-response and say `"accepted"`).
    pub result: PayloadDesc,
}

/// One catalog event row (doc 05 §3.2, one per id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventSpec {
    /// The catalog id.
    pub id: EventId,
    /// The §3.2 "gen" column: whether the event is pipeline-scoped
    /// (carries `gen`, subject to the §4.2 staleness rule).
    pub pipeline_scoped: bool,
    /// The §3.2 *Rate* column (§3.4 throttling rule, compact form).
    pub rate: &'static str,
}

/// The full command table, in doc 05 §2.1 order (61 rows).
pub const fn catalog_data() -> &'static [CommandSpec] {
    &[
        // ---- session (§2.3) ----
        CommandSpec {
            id: CommandId::SessionOpen,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("source: SourceSpec, options: OpenOptions"),
            result: PayloadDesc::Type("accepted"),
        },
        CommandSpec {
            id: CommandId::SessionClose,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: true,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("Unit"),
        },
        // ---- playback (§2.4) ----
        CommandSpec {
            id: CommandId::PlaybackPlay,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("accepted"),
        },
        CommandSpec {
            id: CommandId::PlaybackPause,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("accepted"),
        },
        CommandSpec {
            id: CommandId::PlaybackTogglePlayPause,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("accepted"),
        },
        CommandSpec {
            id: CommandId::PlaybackStop,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: true,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("accepted"),
        },
        CommandSpec {
            id: CommandId::PlaybackSeek,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: true,
            args: PayloadDesc::Type("target: SeekTarget"),
            result: PayloadDesc::Type("accepted"),
        },
        CommandSpec {
            id: CommandId::PlaybackSetRate,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("rate: Rate"),
            result: PayloadDesc::Type("AppliedRate"),
        },
        CommandSpec {
            id: CommandId::PlaybackStepFrame,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: true,
            args: PayloadDesc::Type("direction: StepDirection"),
            result: PayloadDesc::Type("accepted"),
        },
        CommandSpec {
            id: CommandId::PlaybackSetPosition,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: true,
            args: PayloadDesc::Type("fraction: f32"),
            result: PayloadDesc::Type("accepted"),
        },
        // ---- tracks (§2.5) ----
        CommandSpec {
            id: CommandId::TracksList,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("TrackList"),
        },
        CommandSpec {
            id: CommandId::TracksGetInfo,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("id: TrackId"),
            result: PayloadDesc::Type("TrackInfo"),
        },
        CommandSpec {
            id: CommandId::TracksSetVideoTrack,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: true,
            args: PayloadDesc::Type("selector: TrackSelector"),
            result: PayloadDesc::Type("accepted"),
        },
        CommandSpec {
            id: CommandId::TracksSetAudioTrack,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: true,
            args: PayloadDesc::Type("selector: TrackSelector"),
            result: PayloadDesc::Type("accepted"),
        },
        CommandSpec {
            id: CommandId::TracksSetSubtitleTrack,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("selector: TrackSelector"),
            result: PayloadDesc::Type("AppliedSelector"),
        },
        CommandSpec {
            id: CommandId::TracksSetSecondaryAudio,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("selector: TrackSelector"),
            result: PayloadDesc::Type("accepted"),
        },
        // ---- audio (§2.6) ----
        CommandSpec {
            id: CommandId::AudioSetVolume,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("volume: Volume"),
            result: PayloadDesc::Type("AudioVolume"),
        },
        CommandSpec {
            id: CommandId::AudioGetVolume,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("AudioVolume"),
        },
        CommandSpec {
            id: CommandId::AudioSetMute,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("muted: bool"),
            result: PayloadDesc::Type("AppliedBool"),
        },
        CommandSpec {
            id: CommandId::AudioListDevices,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("AudioDevices"),
        },
        CommandSpec {
            id: CommandId::AudioSetDevice,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("id: String"),
            result: PayloadDesc::Type("accepted"),
        },
        CommandSpec {
            id: CommandId::AudioSetDelay,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("delay_ms: i64"),
            result: PayloadDesc::Type("AppliedDelayMs"),
        },
        // ---- video (§2.7) ----
        CommandSpec {
            id: CommandId::VideoSetFullscreen,
            class: CompletionClass::S,
            target: CommandTarget::Shell,
            gen_bumping: false,
            args: PayloadDesc::Type("enabled: bool"),
            result: PayloadDesc::Type("AppliedBool"),
        },
        CommandSpec {
            id: CommandId::VideoSetAspectRatio,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("ratio: AspectRatioSpec"),
            result: PayloadDesc::Type("AppliedAspectRatio"),
        },
        CommandSpec {
            id: CommandId::VideoSetCrop,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("crop: CropSpec"),
            result: PayloadDesc::Type("AppliedCrop"),
        },
        CommandSpec {
            id: CommandId::VideoSetZoom,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("factor: f32"),
            result: PayloadDesc::Type("AppliedZoom"),
        },
        CommandSpec {
            id: CommandId::VideoSetDelay,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("delay_ms: i64"),
            result: PayloadDesc::Type("AppliedDelayMs"),
        },
        CommandSpec {
            id: CommandId::VideoTakeScreenshot,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("path: Option<PathBuf>"),
            result: PayloadDesc::Type("accepted"),
        },
        // ---- subtitles (§2.8) ----
        CommandSpec {
            id: CommandId::SubtitleAddFile,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("path: String, encoding: Option<String>, select: bool"),
            result: PayloadDesc::Type("accepted"),
        },
        CommandSpec {
            id: CommandId::SubtitleSetSync,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("delta_ms: i64"),
            result: PayloadDesc::Type("AppliedDelayMs"),
        },
        CommandSpec {
            id: CommandId::SubtitleSetEncoding,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("encoding: String"),
            result: PayloadDesc::Type("AppliedEncoding"),
        },
        CommandSpec {
            id: CommandId::SubtitleSetScale,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("scale: f32"),
            result: PayloadDesc::Type("AppliedScale"),
        },
        // ---- playlist (§2.9) ----
        CommandSpec {
            id: CommandId::PlaylistEnqueue,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("source: SourceSpec, title: Option<String>"),
            result: PayloadDesc::Type("PlaylistItem"),
        },
        CommandSpec {
            id: CommandId::PlaylistEnqueueMany,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("items: Vec<PlaylistEnqueueItem>"),
            result: PayloadDesc::Type("PlaylistItems"),
        },
        CommandSpec {
            id: CommandId::PlaylistPlayItem,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("id: PlaylistItemId"),
            result: PayloadDesc::Type("accepted"),
        },
        CommandSpec {
            id: CommandId::PlaylistNext,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("accepted"),
        },
        CommandSpec {
            id: CommandId::PlaylistPrev,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("accepted"),
        },
        CommandSpec {
            id: CommandId::PlaylistSkip,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("n: i32"),
            result: PayloadDesc::Type("accepted"),
        },
        CommandSpec {
            id: CommandId::PlaylistRemove,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("id: PlaylistItemId"),
            result: PayloadDesc::Type("Unit"),
        },
        CommandSpec {
            id: CommandId::PlaylistMove,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("id: PlaylistItemId, to_index: u32"),
            result: PayloadDesc::Type("Unit"),
        },
        CommandSpec {
            id: CommandId::PlaylistClear,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("Unit"),
        },
        CommandSpec {
            id: CommandId::PlaylistSetShuffle,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("enabled: bool"),
            result: PayloadDesc::Type("AppliedBool"),
        },
        CommandSpec {
            id: CommandId::PlaylistSetRepeat,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("mode: RepeatMode"),
            result: PayloadDesc::Type("AppliedRepeat"),
        },
        CommandSpec {
            id: CommandId::PlaylistSave,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("path: PathBuf, format: Option<PlaylistSaveFormat>"),
            result: PayloadDesc::Type("Unit"),
        },
        CommandSpec {
            id: CommandId::PlaylistLoad,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("path: PathBuf, append: bool"),
            result: PayloadDesc::Type("PlaylistItems"),
        },
        // ---- media information (§2.10) ----
        CommandSpec {
            id: CommandId::MediaGetInfo,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("MediaInfo"),
        },
        CommandSpec {
            id: CommandId::MediaGetCurrentTime,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("CurrentTime"),
        },
        CommandSpec {
            id: CommandId::MediaGetDuration,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("Duration"),
        },
        CommandSpec {
            id: CommandId::MediaGetChapters,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("Chapters"),
        },
        CommandSpec {
            id: CommandId::MediaSetChapter,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: true,
            args: PayloadDesc::Type("index: u32"),
            result: PayloadDesc::Type("accepted"),
        },
        // ---- configuration (§2.11) ----
        CommandSpec {
            id: CommandId::ConfigGetSetting,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("area: String, key: String"),
            result: PayloadDesc::Type("SettingResult"),
        },
        CommandSpec {
            id: CommandId::ConfigSetSetting,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("area: String, key: String, value: SettingValue"),
            result: PayloadDesc::Type("SettingApplied"),
        },
        CommandSpec {
            id: CommandId::ConfigResetSetting,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("area: String, key: String"),
            result: PayloadDesc::Type("SettingApplied"),
        },
        CommandSpec {
            id: CommandId::ConfigListAreas,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("ConfigAreas"),
        },
        // ---- system (§2.12) ----
        CommandSpec {
            id: CommandId::SystemGetEngineInfo,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("EngineInfo"),
        },
        CommandSpec {
            id: CommandId::SystemListPlugins,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("PluginList"),
        },
        CommandSpec {
            id: CommandId::SystemPluginEnable,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("id: String"),
            result: PayloadDesc::Type("PluginToggle"),
        },
        CommandSpec {
            id: CommandId::SystemPluginDisable,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("id: String"),
            result: PayloadDesc::Type("PluginToggle"),
        },
        CommandSpec {
            id: CommandId::SystemSetLogLevel,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("level: LogLevel, target: Option<String>"),
            result: PayloadDesc::Type("AppliedLogLevel"),
        },
        CommandSpec {
            id: CommandId::SystemGetStateSnapshot,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("StateSnapshot"),
        },
        CommandSpec {
            id: CommandId::SystemListCommands,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::None,
            result: PayloadDesc::Type("CommandListings"),
        },
    ]
}

/// The full event table, in doc 05 §3.2 order (23 rows).
pub const fn event_catalog_data() -> &'static [EventSpec] {
    &[
        EventSpec {
            id: EventId::SessionStateChanged,
            pipeline_scoped: true,
            rate: "never-coalesced",
        },
        EventSpec {
            id: EventId::SessionOpeningProgress,
            pipeline_scoped: true,
            rate: "<=1hz",
        },
        EventSpec {
            id: EventId::SessionBuffering,
            pipeline_scoped: true,
            rate: "<=5hz",
        },
        EventSpec {
            id: EventId::TimingTimeTick,
            pipeline_scoped: true,
            rate: "config-interval-never-queued",
        },
        EventSpec {
            id: EventId::TimingDurationKnown,
            pipeline_scoped: true,
            rate: "once-per-change",
        },
        EventSpec {
            id: EventId::TimingSeekCompleted,
            pipeline_scoped: true,
            rate: "terminal",
        },
        EventSpec {
            id: EventId::TimingSeekFailed,
            pipeline_scoped: true,
            rate: "terminal",
        },
        EventSpec {
            id: EventId::TracksListReady,
            pipeline_scoped: true,
            rate: "<=1hz-coalesced",
        },
        EventSpec {
            id: EventId::TracksChanged,
            pipeline_scoped: true,
            rate: "discrete",
        },
        EventSpec {
            id: EventId::PlaybackEndOfTrack,
            pipeline_scoped: true,
            rate: "discrete",
        },
        EventSpec {
            id: EventId::PlaybackEndOfMedia,
            pipeline_scoped: true,
            rate: "discrete",
        },
        EventSpec {
            id: EventId::PlaybackError,
            pipeline_scoped: true,
            rate: "nonfatal-coalesced-1s",
        },
        EventSpec {
            id: EventId::PlaybackRateChanged,
            pipeline_scoped: false,
            rate: "discrete",
        },
        EventSpec {
            id: EventId::AudioDeviceLost,
            pipeline_scoped: false,
            rate: "discrete",
        },
        EventSpec {
            id: EventId::AudioDeviceChanged,
            pipeline_scoped: false,
            rate: "discrete",
        },
        EventSpec {
            id: EventId::VideoScreenshotSaved,
            pipeline_scoped: false,
            rate: "discrete",
        },
        EventSpec {
            id: EventId::PlaylistChanged,
            pipeline_scoped: false,
            rate: "model-edits-coalesced-100ms",
        },
        EventSpec {
            id: EventId::ConfigSettingChanged,
            pipeline_scoped: false,
            rate: "discrete",
        },
        EventSpec {
            id: EventId::PluginsLoaded,
            pipeline_scoped: false,
            rate: "discrete",
        },
        EventSpec {
            id: EventId::PluginsFailed,
            pipeline_scoped: false,
            rate: "discrete",
        },
        EventSpec {
            id: EventId::PluginsCrashed,
            pipeline_scoped: false,
            rate: "discrete",
        },
        EventSpec {
            id: EventId::SystemLog,
            pipeline_scoped: false,
            rate: "100/s-then-aggregate",
        },
        EventSpec {
            id: EventId::MediaInfoReady,
            pipeline_scoped: true,
            rate: "once-per-open",
        },
    ]
}

/// FNV-1a 64-bit (deterministic, dependency-free; std's `DefaultHasher`
/// is explicitly unstable across releases and this feeds a committed
/// artifact). Parameters per the published FNV-1a spec.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The canonical text a catalog checksum folds: one line per row,
/// field order fixed, doc-05 table order. Any catalog change — id,
/// class, target, gen flag, arg/result shape, event scoping or rate —
/// changes this text and therefore the checksum.
pub fn canonical_catalog_text(
    commands: &[CommandSpec],
    events: &[EventSpec],
) -> String {
    let mut out = String::new();
    for spec in commands {
        out.push_str(&format!(
            "command\t{}\t{}\t{}\t{}\t{}\t{}\n",
            spec.id.as_str(),
            spec.class.as_str(),
            spec.target.as_str(),
            if spec.gen_bumping { "gen" } else { "static" },
            spec.args.as_str(),
            spec.result.as_str(),
        ));
    }
    for spec in events {
        out.push_str(&format!(
            "event\t{}\t{}\t{}\n",
            spec.id.as_str(),
            if spec.pipeline_scoped { "scoped" } else { "global" },
            spec.rate,
        ));
    }
    out
}

/// The committed catalog checksum: FNV-1a over the canonical text of
/// both tables.
pub fn catalog_checksum() -> u64 {
    checksum_of(catalog_data(), event_catalog_data())
}

/// Checksum of arbitrary (possibly mutated) tables — the drift tests
/// mutate a copy and expect a different value.
pub fn checksum_of(commands: &[CommandSpec], events: &[EventSpec]) -> u64 {
    fnv1a64(canonical_catalog_text(commands, events).as_bytes())
}

/// Look up one command's row.
pub fn command_spec(id: CommandId) -> Option<&'static CommandSpec> {
    catalog_data().iter().find(|spec| spec.id == id)
}

/// Look up one event's row.
pub fn event_spec(id: EventId) -> Option<&'static EventSpec> {
    event_catalog_data().iter().find(|spec| spec.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table covers every id, exactly once, in doc-05 order.
    #[test]
    fn tables_cover_every_id_exactly_once_in_order() {
        assert_eq!(catalog_data().len(), CommandId::ALL.len());
        assert_eq!(event_catalog_data().len(), EventId::ALL.len());
        for (spec, id) in catalog_data().iter().zip(CommandId::ALL) {
            assert_eq!(spec.id, *id, "command rows must follow §2.1 order");
        }
        for (spec, id) in event_catalog_data().iter().zip(EventId::ALL) {
            assert_eq!(spec.id, *id, "event rows must follow §3.2 order");
        }
        let mut seen = std::collections::BTreeSet::new();
        for spec in catalog_data() {
            assert!(seen.insert(spec.id), "duplicate row for {}", spec.id);
        }
        let mut seen = std::collections::BTreeSet::new();
        for spec in event_catalog_data() {
            assert!(seen.insert(spec.id), "duplicate row for {}", spec.id);
        }
    }

    /// §1.5's example lists, verbatim: the S examples are S here, the A
    /// examples are A here, and the §2 tables' per-command classes
    /// agree with them.
    #[test]
    fn classification_matches_section_1_5_examples() {
        let s_examples = [
            CommandId::AudioSetVolume,
            CommandId::PlaybackSetRate,
            CommandId::AudioSetMute,
            CommandId::VideoSetAspectRatio,
            CommandId::TracksList,
            CommandId::AudioListDevices,
            CommandId::ConfigSetSetting,
            CommandId::PlaylistRemove,
        ];
        let a_examples = [
            CommandId::SessionOpen,
            CommandId::PlaybackPlay,
            CommandId::PlaybackPause,
            CommandId::PlaybackStop,
            CommandId::PlaybackSeek,
            CommandId::PlaybackSetPosition,
            CommandId::MediaSetChapter,
            CommandId::PlaybackStepFrame,
            CommandId::TracksSetVideoTrack,
            CommandId::TracksSetAudioTrack,
            CommandId::AudioSetDevice,
            CommandId::VideoTakeScreenshot,
            CommandId::SubtitleAddFile,
            CommandId::PlaylistPlayItem,
            CommandId::PlaylistNext,
            CommandId::PlaylistPrev,
            CommandId::PlaylistSkip,
        ];
        for id in s_examples {
            assert_eq!(
                command_spec(id).map(|spec| spec.class),
                Some(CompletionClass::S),
                "{id} must be class S (§1.5)"
            );
        }
        for id in a_examples {
            assert_eq!(
                command_spec(id).map(|spec| spec.class),
                Some(CompletionClass::A),
                "{id} must be class A (§1.5)"
            );
        }
    }

    /// §4.2's closed gen-bumping list, exactly.
    #[test]
    fn gen_bumping_set_is_exactly_section_4_2() {
        let expected = [
            CommandId::PlaybackSeek,
            CommandId::PlaybackSetPosition,
            CommandId::PlaybackStepFrame,
            CommandId::MediaSetChapter,
            CommandId::PlaybackStop,
            CommandId::TracksSetVideoTrack,
            CommandId::TracksSetAudioTrack,
            CommandId::SessionClose,
        ];
        let actual: Vec<CommandId> = catalog_data()
            .iter()
            .filter(|spec| spec.gen_bumping)
            .map(|spec| spec.id)
            .collect();
        assert_eq!(actual, expected, "§4.2 gen-bumping list must match");
    }

    /// §1.1.3 + §2.7: `video.set-fullscreen` is the only shell-routed
    /// command in v1; everything else targets the engine.
    #[test]
    fn only_fullscreen_targets_the_shell() {
        let shell: Vec<CommandId> = catalog_data()
            .iter()
            .filter(|spec| spec.target == CommandTarget::Shell)
            .map(|spec| spec.id)
            .collect();
        assert_eq!(shell, [CommandId::VideoSetFullscreen]);
    }

    /// §1.5's classification policy consequences, as counts: the doc's
    /// tables classify 19 commands A and 42 S.
    #[test]
    fn class_counts_match_the_doc_05_tables() {
        let a = catalog_data()
            .iter()
            .filter(|spec| spec.class == CompletionClass::A)
            .count();
        let s = catalog_data()
            .iter()
            .filter(|spec| spec.class == CompletionClass::S)
            .count();
        assert_eq!(a, 19, "class-A count");
        assert_eq!(s, 42, "class-S count");
        assert_eq!(a + s, 61);
    }

    /// §3.2's "gen" column: exactly the 13 pipeline-scoped events.
    #[test]
    fn pipeline_scoped_set_matches_section_3_2() {
        let scoped: Vec<EventId> = event_catalog_data()
            .iter()
            .filter(|spec| spec.pipeline_scoped)
            .map(|spec| spec.id)
            .collect();
        assert_eq!(
            scoped,
            [
                EventId::SessionStateChanged,
                EventId::SessionOpeningProgress,
                EventId::SessionBuffering,
                EventId::TimingTimeTick,
                EventId::TimingDurationKnown,
                EventId::TimingSeekCompleted,
                EventId::TimingSeekFailed,
                EventId::TracksListReady,
                EventId::TracksChanged,
                EventId::PlaybackEndOfTrack,
                EventId::PlaybackEndOfMedia,
                EventId::PlaybackError,
                EventId::MediaInfoReady,
            ]
        );
    }

    /// The drift negative control: mutating one table entry (a temp
    /// copy) must change the checksum — this is what makes the
    /// `--check` gate catch silent catalog edits.
    #[test]
    fn mutated_table_entry_changes_the_checksum() {
        let baseline = catalog_checksum();
        let mutated_command = {
            let mut rows = catalog_data().to_vec();
            rows[7].gen_bumping = !rows[7].gen_bumping;
            checksum_of(&rows, event_catalog_data())
        };
        assert_ne!(mutated_command, baseline, "command mutation invisible");
        let mutated_event = {
            let mut rows = event_catalog_data().to_vec();
            rows[0].pipeline_scoped = !rows[0].pipeline_scoped;
            checksum_of(catalog_data(), &rows)
        };
        assert_ne!(mutated_event, baseline, "event mutation invisible");
        // …and an unmoved copy is stable.
        assert_eq!(
            checksum_of(&catalog_data().to_vec(), &event_catalog_data().to_vec()),
            baseline
        );
    }

    #[test]
    fn fnv1a64_reference_vectors() {
        // Published FNV-1a 64-bit test vectors.
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x85944171f73967e8);
    }
}
