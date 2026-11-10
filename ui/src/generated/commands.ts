// @generated — Lumen command-catalog TypeScript types (card 1.D4, doc 05 §2-§3, §5.1).
// Regenerate with `cargo xtask codegen` — NEVER hand-edit; the catalog
// checksum gate fails on drift. Source of truth: crates/commands/src/catalog
// (Rust wire types) -> schemas/commands.schema.json -> this file.

export type AspectRatioSpec = ({ kind: "default" } | { h: number; kind: "ratio"; w: number });

export type AudioDevice = { id: string; isDefault: boolean; name: string };

export type AudioTrackInfo = { bitDepth?: (number | null); channels: number; codecParams?: (string | null); sampleRate?: (number | null) };

export type Bookmark = { id: number; label?: (string | null); time: Timestamp };

export type Chapter = { duration: Duration; index: number; timeStart: Timestamp; title?: (string | null) };

export type CodecDescriptor = { description?: (string | null); fourcc?: (string | null); level?: (string | null); profile?: (string | null) };

export type CommandError = { code: ErrorCode; details: Record<string, string>; message: string; stage: Stage };

export type CommandListing = { class: CompletionClass; deprecated?: (string | null); id: string; requires: string[]; since: string };

export type CommandOutcome = ({ accepted: { gen: Generation; session?: (SessionId | null) } } | { completed: CommandResult });

export type CommandRequest = ({ requestId: RequestId } & ({ args: { options?: OpenOptions; source: SourceSpec }; cmd: "session.open" } | { cmd: "session.close" } | { cmd: "playback.play" } | { cmd: "playback.pause" } | { cmd: "playback.toggle-play-pause" } | { cmd: "playback.stop" } | { args: { target: SeekTarget }; cmd: "playback.seek" } | { args: { rate: number }; cmd: "playback.set-rate" } | { args: { direction: StepDirection }; cmd: "playback.step-frame" } | { args: { fraction: number }; cmd: "playback.set-position" } | { cmd: "tracks.list" } | { args: { id: TrackId }; cmd: "tracks.get-info" } | { args: { selector: TrackSelector }; cmd: "tracks.set-video-track" } | { args: { selector: TrackSelector }; cmd: "tracks.set-audio-track" } | { args: { selector: TrackSelector }; cmd: "tracks.set-subtitle-track" } | { args: { selector: TrackSelector }; cmd: "tracks.set-secondary-audio" } | { args: { volume: number }; cmd: "audio.set-volume" } | { cmd: "audio.get-volume" } | { args: { muted: boolean }; cmd: "audio.set-mute" } | { cmd: "audio.list-devices" } | { args: { id: string }; cmd: "audio.set-device" } | { args: { delayMs: number }; cmd: "audio.set-delay" } | { args: { enabled: boolean }; cmd: "video.set-fullscreen" } | { args: { ratio: AspectRatioSpec }; cmd: "video.set-aspect-ratio" } | { args: { crop: CropSpec }; cmd: "video.set-crop" } | { args: { factor: number }; cmd: "video.set-zoom" } | { args: { delayMs: number }; cmd: "video.set-delay" } | { args: { path?: (string | null) }; cmd: "video.take-screenshot" } | { args: { encoding?: (string | null); path: string; select?: boolean }; cmd: "subtitle.add-file" } | { args: { deltaMs: number }; cmd: "subtitle.set-sync" } | { args: { encoding: string }; cmd: "subtitle.set-encoding" } | { args: { scale: number }; cmd: "subtitle.set-scale" } | { args: { source: SourceSpec; title?: (string | null) }; cmd: "playlist.enqueue" } | { args: { items: PlaylistEnqueueItem[] }; cmd: "playlist.enqueue-many" } | { args: { id: PlaylistItemId }; cmd: "playlist.play-item" } | { cmd: "playlist.next" } | { cmd: "playlist.prev" } | { args: { n: number }; cmd: "playlist.skip" } | { args: { id: PlaylistItemId }; cmd: "playlist.remove" } | { args: { id: PlaylistItemId; toIndex: number }; cmd: "playlist.move" } | { cmd: "playlist.clear" } | { args: { enabled: boolean }; cmd: "playlist.set-shuffle" } | { args: { mode: RepeatMode }; cmd: "playlist.set-repeat" } | { args: { format?: (PlaylistSaveFormat | null); path: string }; cmd: "playlist.save" } | { args: { append: boolean; path: string }; cmd: "playlist.load" } | { cmd: "media.get-info" } | { cmd: "media.get-current-time" } | { cmd: "media.get-duration" } | { cmd: "media.get-chapters" } | { args: { index: number }; cmd: "media.set-chapter" } | { cmd: "media.get-bookmarks" } | { args: { label?: (string | null); time: Timestamp }; cmd: "media.add-bookmark" } | { args: { id: number }; cmd: "media.remove-bookmark" } | { args: { area: string; key: string }; cmd: "config.get-setting" } | { args: { area: string; key: string; value: SettingValue }; cmd: "config.set-setting" } | { args: { area: string; key: string }; cmd: "config.reset-setting" } | { cmd: "config.list-areas" } | { cmd: "system.get-engine-info" } | { cmd: "system.list-plugins" } | { args: { id: string }; cmd: "system.plugin-enable" } | { args: { id: string }; cmd: "system.plugin-disable" } | { args: { level: LogLevel; target?: (string | null) }; cmd: "system.set-log-level" } | { cmd: "system.get-state-snapshot" } | { cmd: "system.list-commands" }));

export type CommandResponse = { gen: Generation; requestId: RequestId; result: ResultWire; session?: (SessionId | null) };

export type CommandResult = ({ kind: "unit" } | { kind: "applied-rate"; rate: number } | { kind: "track-list"; tracks: TrackInfo[] } | { info: TrackInfo; kind: "track-info" } | { kind: "applied-selector"; selector: TrackSelector } | { kind: "audio-volume"; volume: number } | { kind: "applied-bool"; value: boolean } | { devices: AudioDevice[]; kind: "audio-devices" } | { delayMs: number; kind: "applied-delay-ms" } | { kind: "applied-aspect-ratio"; ratio: AspectRatioSpec } | { crop: CropSpec; kind: "applied-crop" } | { factor: number; kind: "applied-zoom" } | { encoding: string; kind: "applied-encoding" } | { kind: "applied-scale"; scale: number } | { kind: "applied-repeat"; mode: RepeatMode } | { id: PlaylistItemId; kind: "playlist-item" } | { ids: PlaylistItemId[]; kind: "playlist-items" } | { info: MediaInfo; kind: "media-info" } | { buffered: TimeRange[]; kind: "current-time"; position: number; time: Timestamp } | { duration: Duration; kind: "duration" } | { chapters: Chapter[]; kind: "chapters" } | { bookmarks: Bookmark[]; kind: "bookmarks" } | { bookmark: Bookmark; kind: "bookmark-added" } | { kind: "setting-result"; schemaMeta: Record<string, string>; value: SettingValue } | { applied: SettingValue; kind: "setting-applied"; restartRequired: boolean } | { areas: ConfigAreaInfo[]; kind: "config-areas" } | { info: EngineInfo; kind: "engine-info" } | { kind: "plugin-list"; plugins: PluginInfo[] } | { applied: boolean; kind: "plugin-toggle"; restartRequired: boolean; takesEffect?: (string | null) } | { kind: "applied-log-level"; level: LogLevel } | { kind: "state-snapshot"; snapshot: StateSnapshot } | { commands: CommandListing[]; kind: "command-listings" });

export type CompletionClass = ("s" | "a");

export type ConfigAreaInfo = { anyRestartRequired: boolean; area: string; nameKey: string; settingCount: number };

export type CropSpec = ({ kind: "none" } | { h: number; kind: "ratio"; w: number } | { h: number; kind: "rect"; w: number; x: number; y: number });

export type DecoderInfo = { active: boolean; backend?: (string | null); hardware: boolean; kind: TrackKind; name: string };

export type Duration = ({ kind: "known"; time: Timestamp } | { kind: "unknown" } | { kind: "infinite" });

export type EngineInfo = { catalogVersion: string; decoders: DecoderInfo[]; engineVersion: string; features: string[]; host: string; plugins: string[]; protocolVersion: string; sessions: SessionSummary[] };

export type ErrorCode = ("unsupported" | "corrupt" | "inaccessible" | "network" | "decoder" | "device" | "resource-limit" | "invalid-argument" | "out-of-range" | "not-found" | "not-ready" | "not-seekable" | "canceled" | "feature-not-exposed" | "unsupported-for-host" | "permission-denied" | "io" | "internal");

export type ErrorEnvelope = { error: CommandError };

export type Event = ({ gen?: (Generation | null); origin?: (RequestId | null); seq: number; session?: (SessionId | null) } & ({ from: PlaybackState; kind: "session.state-changed"; to: PlaybackState } | { kind: "session.opening-progress"; progress: number } | { kind: "session.buffering"; progress: number } | { kind: "timing.time-tick"; position: number; time: Timestamp } | { duration: Timestamp; kind: "timing.duration-known" } | { kind: "timing.seek-completed"; position: Timestamp; seekKind: SeekKind } | { error: CommandError; kind: "timing.seek-failed" } | { kind: "tracks.list-ready"; tracks: TrackInfo[] } | { kind: "tracks.changed"; selected?: (TrackId | null); trackKind: TrackChangeKind } | { id: TrackId; kind: "playback.end-of-track"; trackKind: TrackKind } | { kind: "playback.end-of-media" } | { code: ErrorCode; details: Record<string, string>; fatal: boolean; kind: "playback.error"; message: string; stage: Stage } | { kind: "playback.rate-changed"; rate: number } | { deviceId: string; kind: "audio.device-lost" } | { deviceId: string; kind: "audio.device-changed" } | { height: number; kind: "video.screenshot-saved"; path: string; width: number } | { backend: string; kind: "video.backend-changed"; reason: string } | { change: PlaylistChange; kind: "playlist.changed" } | { area: string; key: string; kind: "config.setting-changed"; layer: string; value: SettingValue } | { id: string; kind: "plugins.loaded"; pluginKind: string; version: string } | { error: string; id: string; kind: "plugins.failed" } | { actionTaken: PluginAction; error: string; id: string; kind: "plugins.crashed" } | { kind: "system.log"; level: LogLevel; message: string; target: string } | { info: MediaInfo; kind: "media.info-ready" } | { id: string; kind: "devices.volume-added"; label: string; volumeKind: VolumeKind } | { id: string; kind: "devices.volume-removed" }));

export type Generation = number;

export type LogLevel = ("trace" | "debug" | "info" | "warn" | "error");

export type MediaInfo = { chapters: Chapter[]; container?: (string | null); duration: Duration; isLive: boolean; metadata: Record<string, string>; seekable: boolean; source: SourceSpec; title?: (string | null); tracks: TrackInfo[] };

export type OpenOptions = { audioTrack?: (TrackSelector | null); forcedSubtitleOnly?: boolean; noAudio?: boolean; noVideo?: boolean; rate?: (number | null); startPaused?: boolean; startTime?: (Timestamp | null); subtitleTrack?: (TrackSelector | null); videoTrack?: (TrackSelector | null) };

export type PlaybackState = ("idle" | "opening" | "buffering" | "playing" | "paused" | "stopped" | "ended" | "error");

export type PlaylistChange = ({ ids: PlaylistItemId[]; kind: "items-added" } | { ids: PlaylistItemId[]; kind: "items-removed" } | { from: number; id: PlaylistItemId; kind: "moved"; to: number } | { kind: "cleared" } | { enabled: boolean; kind: "shuffle-set" } | { kind: "repeat-set"; mode: RepeatMode } | { id: PlaylistItemId; kind: "current-changed" });

export type PlaylistEnqueueItem = { source: SourceSpec; title?: (string | null) };

export type PlaylistItemId = number;

export type PlaylistSaveFormat = ("m3u8" | "xspf" | "json");

export type PlaylistSummary = { current?: (PlaylistItemId | null); itemCount?: number; repeat?: RepeatMode; shuffle?: boolean };

export type PluginAction = ("disabled" | "isolated");

export type PluginInfo = { capabilities: string[]; enabled: boolean; id: string; kind: string; permissions: string[]; source: PluginSource; version: string };

export type PluginSource = ("builtin" | "wasm");

export type RepeatMode = ("off" | "one" | "all");

export type RequestId = number;

export type ResultWire = (CommandOutcome | ErrorEnvelope);

export type SeekKind = ("command" | "frame-step");

export type SeekTarget = ({ kind: "absolute"; time: Timestamp } | { deltaMs: number; kind: "relative" } | { fraction: number; kind: "percent" } | { index: number; kind: "chapter" });

export type SessionId = number;

export type SessionSummary = { gen: Generation; id: SessionId; state: PlaybackState };

export type SettingValue = (boolean | number | string | string[]);

export type SourceSpec = ({ kind: "file"; path: string } | { kind: "url"; url: string });

export type Stage = ("session" | "input" | "probe" | "demux" | "decode" | "audio-output" | "video-output" | "subtitle" | "renderer" | "playlist" | "config" | "plugin" | "control");

export type StateSnapshot = { audioDelayMs: number; devices: AudioDevice[]; duration: Duration; muted: boolean; playlist: PlaylistSummary; rate: number; selections: TrackSelections; state: PlaybackState; subtitle: SubtitleSettings; subtitleDelayMs: number; time?: (Timestamp | null); tracks: TrackInfo[]; videoDelayMs: number; volume: number };

export type StepDirection = ("next" | "previous");

export type SubtitleSettings = { delayMs?: number; encoding?: (string | null); scale?: number };

export type TimeRange = { end: Timestamp; start: Timestamp };

export type Timestamp = number;

export type TrackChangeKind = ("video" | "audio" | "subtitle" | "data" | "secondary-audio");

export type TrackId = number;

export type TrackInfo = { audio?: (AudioTrackInfo | null); codec: CodecDescriptor; id: TrackId; isDefault: boolean; isSelected: boolean; kind: TrackKind; language?: (string | null); name?: (string | null); origin: TrackOrigin; video?: (VideoTrackInfo | null) };

export type TrackKind = ("video" | "audio" | "subtitle" | "data");

export type TrackOrigin = ("stream" | "external-subtitle");

export type TrackSelections = { audio?: (TrackId | null); secondaryAudio?: (TrackId | null); subtitle?: (TrackId | null); video?: (TrackId | null) };

export type TrackSelector = ({ id: TrackId; kind: "id" } | { kind: "none" } | { kind: "auto" });

export type VideoTrackInfo = { bitDepth?: (number | null); frameRate?: (number | null); hdr: boolean; height: number; width: number };

export type VolumeKind = ("fixed" | "removable" | "optical" | "network");

// The schema root: one object referencing the three wire envelopes.
export type CatalogDocument = { event: Event; request: CommandRequest; response: CommandResponse };
