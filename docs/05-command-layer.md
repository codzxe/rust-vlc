# 05 — Command & Event Layer: Contract Specification (v1)

| Field | Value |
|---|---|
| Task | 1.A5 (Wave 1, planning document — type sketches, no implementation) |
| Status | implemented (awaiting Lead review) |
| Authoritative source | `docs/MASTER-PROMPT.md` (MP). Every section traces to MP §2, §3, §5, §8, §10, §12, §13. |
| Parallel work | `docs/03-engine-plan.md` (agent 1.A3) landed **while this document was being written**; this document was drafted from MASTER-PROMPT alone and reconciled against 03 afterwards (§4.2 note). Cross-references to engine internals are marked `[03]`. |
| Owning crates | `crates/commands` (catalog, envelope, types), `crates/control-protocol` (server wire), `crates/testkit` (conformance), hosts: `apps/desktop`, `apps/web`, `apps/cli`, `apps/control-server` |
| Inventions | Marked `[PROPOSAL]` throughout; collected in §9 with open questions for the Lead |

**Terminology.** *Host* — a process embedding the engine: Tauri shell, web page, CLI, control server, or test harness. *Session* — one open media instance owned by the engine (`SessionId`). *Generation* (`gen`) — a per-session counter that invalidates prior asynchronous work (MP §8). *Catalog* — the enumerated set of command and event IDs defined in this document. *Wire* — the serialized form crossing a host boundary (IPC message, wasm call, CLI/stdout line, control-server frame).

---

## 1. Contract principles

### 1.1 One command layer (Trace: MP §2 “Contract”, §3.5, §3.9)

There is exactly one command layer. The desktop UI (Tauri IPC), the web UI (wasm binding), the CLI, the contract tests and the control server all consume the same catalog, the same envelope, the same request/response/event semantics, defined once in `crates/commands` and re-exported (never re-implemented) by every host.

Consequences, all binding:

1. **No host-specific commands.** If a host needs a new lever, the lever is added to the catalog (versioned, §6) and becomes available to all hosts, possibly feature-gated.
2. **The UI imports engine types only through the generated command-layer types.** `ui/` must not depend on engine crates (`playback-engine`, `media-types`, …). Enforcement is automated (§7.4).
3. **Shell-routed commands are still catalog commands.** A few commands (e.g. `video.set-fullscreen`) are executed by the shell/host rather than the engine core (§2.7); they remain in the catalog so that keymaps, menus, CLI and the control server address them uniformly. Each entry declares its `target`: `engine` or `shell`.
4. **UI actions that are not engine concerns** (open-file dialog, toggle info panel, focus player) live in a separate, schema-validated **UI-action registry** (§8.4), never in the engine catalog. `[PROPOSAL]` — dual-namespace design; alternative is polluting the engine catalog with `ui.*` commands, rejected here.

### 1.2 Typed and validated at the boundary (Trace: MP §3.9, §8, §12)

Every request is deserialized into typed Rust structures at the host boundary, before any engine work, with these mandatory rules:

- **V1 — Closed shapes.** Deserialization uses `deny_unknown_fields`; unknown fields, unknown enum variants, and unknown command/event IDs are rejected with `invalid-argument`, never ignored.
- **V2 — Ranges validated before dispatch.** Scalar ranges (volume, rate, zoom, scale, delays, indexes) are checked against the values declared in this catalog and the settings schema; violations return `out-of-range` with the allowed range in `details`.
- **V3 — No sentinel values.** Missing durations, unknown positions and absent tracks are explicit enum variants (`Duration::Unknown`, `Option<TrackId>`), never `-1` or `0` magic numbers (MP §8).
- **V4 — Checked numeric conversions.** All narrowing conversions use `try_from`; there are no `as` casts on the request path (workspace lints already deny the usual foot-guns).
- **V5 — Reference validity checked at execution, not just parse.** Track IDs and playlist item IDs are validated against live session state when the command executes. Because the list can change concurrently, a race degrades to a typed `not-found` error at execution time — parse-time validation is advisory only.
- **V6 — Size caps.** Strings ≤ 4 096 bytes (paths) / 1 024 bytes (names), playlist arrays ≤ `playlist.max-items` (config, default 10 000), nested depth ≤ 8. Violations are `resource-limit` / `invalid-argument`. `[PROPOSAL]` — exact numbers.
- **V7 — Untrusted input stance.** All strings from media, playlists, subtitles and network origins that appear in results/events are treated as untrusted data: length-capped, UTF-8-validated, and rendered by the UI as text, never markup.
- **V8 — Redaction.** Error messages and `system.log` events are built from redaction-safe templates: URLs have credentials stripped, tokens/secrets never appear, absolute paths may be shown to a local user but are never emitted over the control server to unauthenticated peers (MP §12).

### 1.3 Results are correlated to requests (Trace: MP §3.9)

Every command response carries the `request_id` of the request it answers. Every event carries a global `seq`, an optional `session`, an optional `gen`, and — when the event is the completion of a specific pending request — an `origin: request_id`. The three identifiers (request_id, gen, seq) are the complete correlation machinery; their exact semantics are specified in §4.

### 1.4 The truthful-UI rule: Accepted ≠ Completed (Trace: MP §2 “Truthful UI”)

This is a **hard contract**, not a guideline.

**Definitions.**

- **Accepted** — the engine has validated and queued/started the operation. The request’s response is `CommandOutcome::Accepted { session, gen }`. Nothing about the media has observably changed yet.
- **Completed** — the engine confirms the operation happened. Evidence is either (a) a synchronous `CommandOutcome::Completed(result)` response, or (b) a completion event whose `gen` (and `origin` where present) matches the `Accepted` response.
- **Failed** — a typed `CommandError` in the response, or a terminal failure event (`seek-failed`, fatal `error` event) matching the request.

**Rules (normative, C1–C8).**

- **C1** — The UI may render *pending* affordances from `Accepted` immediately (spinner, detached scrub thumb, dimmed control). It MUST NOT render success before completion evidence.
- **C2** — For synchronous-complete commands (class S, §1.5), the response **is** the completion evidence; no event is required.
- **C3** — For async-observed commands (class A), the UI MUST wait for the completion event. `state_changed`/`seek_completed`/`track_changed`/`device_changed`/`screenshot_saved` are the only acceptable proof for their respective commands.
- **C4** — Playback state in the UI comes **only** from `session.state-changed` events (or a state snapshot query). The UI never keeps an optimistic “playing” flag.
- **C5** — Errors MUST surface to the user (redacted message + typed stage/code), or be explicitly and audibly ignored by user choice (e.g. a non-fatal decode error counter). Silently swallowing a failure is a contract violation.
- **C6** — Stale evidence is not evidence. A completion event whose `gen` is older than the newest `gen` the host has observed for that session is dropped (§4.2).
- **C7** — **Scrubbing pattern.** While dragging, the UI shows a *ghost* position (clearly uncommitted, e.g. a detached thumb). The committed position updates only on `seek-completed`. This is the one sanctioned transient divergence, and it must be visually distinguishable. `[PROPOSAL]` — exact visual treatment follows the Figma exports.
- **C8** — Idempotent confirmations. If a class-A command finds the engine already in the target state (e.g. `pause` while paused), the engine still emits the completion event as a self-transition (`from == to`). This keeps C3 uniform with no special cases in the UI.

### 1.5 Completion classes: which commands are S, which are A

| Class | Meaning | Budget | Examples |
|---|---|---|---|
| **S** (synchronous-complete) | Pure queries; atomic mutations of engine model/config/renderer params with bounded work and **no pipeline invalidation, no unbounded I/O** | Responds within a bounded, small time (target ≤ 50 ms; `[PROPOSAL]` budget, measured in Phase 2) | `set_volume`, `set_rate`, `set_mute`, `set_aspect_ratio`, all `get_*`/`list_*`, playlist model edits, settings |
| **A** (async-observed) | Anything that reconfigures the decode pipeline, performs unbounded I/O, can fail asynchronously, or whose *effect* is only observable through engine state | Returns `Accepted` immediately; completion via a named event | `session.open`, `play`/`pause`/`stop`, `seek`/`set_position`/`set_chapter`/`step_frame`, video/audio track switch, `set_audio_device`, `take_screenshot`, `subtitle.add-file`, `playlist.play-item`/`next`/`prev`/`skip` |

Classification policy (for future entries): default to **A** unless the operation is provably atomic and bounded. When in doubt, A — an unnecessary event is cheap; a false “success” is a truth violation.

### 1.6 Envelope — type sketches

```rust
// crates/commands — authoritative sketch (wire forms in §5 use serde with
// camelCase field names, internally-tagged enums, kebab-case variant names)

pub struct RequestId(pub u64);     // monotonic per host connection, §4.1
pub struct SessionId(pub u64);     // engine-assigned, §4.1
pub struct Generation(pub u64);    // per-session pipeline generation, §4.2

pub struct CommandRequest {
    pub request_id: RequestId,
    pub command: Command,          // tagged enum, one variant per catalog entry
}

pub enum CommandOutcome {
    /// Class A: queued/started. `gen` is the generation under which the
    /// completion event will arrive (new gen for gen-bumping commands).
    Accepted { session: Option<SessionId>, gen: Generation },
    /// Class S: the engine confirms completion *in this response*.
    Completed(CommandResult),
}

pub struct CommandResponse {
    pub request_id: RequestId,             // echoed, always
    pub session: Option<SessionId>,        // session context at handling time
    pub gen: Generation,                   // session gen at handling time
    pub result: Result<CommandOutcome, CommandError>,
}

pub struct CommandError {
    pub stage: Stage,          // where it failed
    pub code: ErrorCode,       // typed failure class
    pub message: String,       // redacted, display-safe
    pub details: BTreeMap<String, String>, // machine-readable hints
}

pub struct Event {
    pub seq: u64,                       // engine-global, strictly increasing
    pub session: Option<SessionId>,     // None for session-less events
    pub gen: Option<Generation>,        // present for pipeline-scoped events
    pub origin: Option<RequestId>,      // present when caused by a command
    pub kind: EventKind,                // tagged enum, one variant per event
}
// Wire note: `kind` is the envelope tag carrying the event ID. Payload field
// names are chosen to never collide with it (hence `seekKind`, `trackKind`).
```

The full error taxonomy (`Stage`, `ErrorCode`) is in §2.13. The state machine referenced below is §3.3.

---

## 2. Command catalog v1

### 2.1 Command IDs: stable kebab-case strings ↔ Rust variants

An ID is `<area>.<verb-noun>` — an area token, a dot, then one or more kebab-case tokens. The Rust mapping is mechanical: split on `.` and `-`, Pascal-case each token, concatenate.

| Wire ID | Rust variant |
|---|---|
| `session.open` | `CommandId::SessionOpen` |
| `playback.toggle-play-pause` | `CommandId::PlaybackTogglePlayPause` |
| `audio.set-volume` | `CommandId::AudioSetVolume` |
| `tracks.set-subtitle-track` | `CommandId::TracksSetSubtitleTrack` |

IDs are **stable forever within major version 1** (§6). The enum is `#[non_exhaustive]`; hosts must match with a catch-all that maps unknown IDs to `invalid-argument` (senders) or a generic no-op with a logged warning (event receivers).

```rust
#[non_exhaustive]
pub enum CommandId {
    // session
    SessionOpen, SessionClose,
    // playback
    PlaybackPlay, PlaybackPause, PlaybackTogglePlayPause, PlaybackStop,
    PlaybackSeek, PlaybackSetRate, PlaybackStepFrame, PlaybackSetPosition,
    // tracks
    TracksList, TracksGetInfo, TracksSetVideoTrack, TracksSetAudioTrack,
    TracksSetSubtitleTrack, TracksSetSecondaryAudio,
    // audio
    AudioSetVolume, AudioGetVolume, AudioSetMute, AudioListDevices,
    AudioSetDevice, AudioSetDelay,
    // video
    VideoSetFullscreen, VideoSetAspectRatio, VideoSetCrop, VideoSetZoom,
    VideoSetDelay, VideoTakeScreenshot,
    // subtitles
    SubtitleAddFile, SubtitleSetSync, SubtitleSetEncoding, SubtitleSetScale,
    // playlist
    PlaylistEnqueue, PlaylistEnqueueMany, PlaylistPlayItem, PlaylistNext,
    PlaylistPrev, PlaylistSkip, PlaylistRemove, PlaylistMove, PlaylistClear,
    PlaylistSetShuffle, PlaylistSetRepeat, PlaylistSave, PlaylistLoad,
    // media information
    MediaGetInfo, MediaGetCurrentTime, MediaGetDuration, MediaGetChapters,
    MediaSetChapter,
    // configuration
    ConfigGetSetting, ConfigSetSetting, ConfigResetSetting, ConfigListAreas,
    // system
    SystemGetEngineInfo, SystemListPlugins, SystemPluginEnable,
    SystemPluginDisable, SystemSetLogLevel,
    SystemGetStateSnapshot, SystemListCommands, // [PROPOSAL] §2.12
}
```

**Table legend for §2.3–2.12.** *Class* = S or A (§1.5). *Completion* = evidence that closes the lifecycle. Every command can additionally fail with the common error set (§2.13); the *Errors* column lists only command-specific additions.

### 2.2 Common parameter and result types

```rust
pub struct Timestamp(pub i64);            // microseconds since media start; can be negative only in Relative
pub enum Duration { Known(Timestamp), Unknown, Infinite } // never a sentinel

pub type Volume = f32;                    // 0.0..=audio.max_volume (config, default 1.25) [PROPOSAL]
pub type Rate = f32;                      // (0.0, playback.max-rate] (config, default 16.0) [PROPOSAL]

pub struct TrackId(pub u32);              // stable within one session, never reused inside it
pub struct PlaylistItemId(pub u64);       // stable within the playlist lifetime, never reused

pub enum TrackKind { Video, Audio, Subtitle, Data }

pub enum TrackSelector { Id(TrackId), None, Auto } // Auto is meaningful for subtitles only

pub struct TrackInfo {
    pub id: TrackId,
    pub kind: TrackKind,
    pub name: Option<String>,             // from container metadata (untrusted text, V7)
    pub language: Option<String>,         // BCP-47 tag when present
    pub codec: CodecDescriptor,           // { fourcc, description, profile, level }
    pub is_default: bool,
    pub is_selected: bool,
    pub origin: TrackOrigin,              // Stream | ExternalSubtitle
    pub video: Option<VideoTrackInfo>,    // width, height, frame_rate, bit_depth, hdr
    pub audio: Option<AudioTrackInfo>,    // channels, sample_rate, bit_depth, codec_params
}

pub enum SeekTarget {
    Absolute { time: Timestamp },         // µs from media start; clamped when duration known
    Relative { delta_ms: i64 },           // negative rewinds
    Percent { fraction: f32 },            // 0.0..=1.0 of duration (requires known duration)
    Chapter { index: u32 },               // jump to chapter start
}

pub enum SourceSpec {                     // clean-room source description (no libvlc types)
    File { path: PathBuf },
    Url { url: String },                  // scheme support is feature-gated (§6.4)
    // future, feature-gated: Disc { .. }, Capture { .. } — not in v1 availability
}
```

### 2.3 Session commands

| ID | Params | Class | Result / Completion | Errors (specific) | Notes |
|---|---|---|---|---|---|
| `session.open` | `source: SourceSpec`, `options: OpenOptions` | A | `Accepted { session, gen: 0 }`; completes on `session.state-changed → playing` (or `paused` if `options.start_paused`); fails via fatal `playback.error { origin }` | `inaccessible`, `unsupported`, `network`, `corrupt`, `resource-limit`, `feature-not-exposed`, `unsupported-for-host` | Replaces any current session (implicit close). New `SessionId`, `gen = 0`. Emits `opening` → (`opening-progress`) → `buffering` → `playing` and `tracks.list-ready`, `media.info-ready` before completion. `OpenOptions`: `start_time`, `start_paused`, `rate`, `no_video`, `no_audio`, `track` selectors, `forced_subtitle_only` — schema-validated, unknown option = `invalid-argument`. |
| `session.close` | — | S | `Completed(Unit)` | — | Idempotent; closing a closed/absent session is still `Unit`. Bumps `gen` (terminal), frees decoder threads with a bounded join deadline (2 s `[PROPOSAL]`), destroys the session. Events already queued for the dead session are dropped by every host (session filter, §4.3). Chosen S deliberately: it has no observable “success” state beyond the session being gone. |

### 2.4 Playback commands

| ID | Params | Class | Result / Completion | Errors (specific) | Notes |
|---|---|---|---|---|---|
| `playback.play` | — | A | Completes on `state-changed → playing` | `not-ready` (no session / idle) | In state `ended`: restarts the current item from position 0 (same session, gen bump). In `paused`: resumes. |
| `playback.pause` | — | A | Completes on `state-changed → paused` | `not-ready` | Idempotent; self-transition echo per C8. |
| `playback.toggle-play-pause` | — | A | Completes on `state-changed → playing` or `→ paused` | `not-ready` | The primary keymap target. |
| `playback.stop` | — | A | Completes on `state-changed → stopped` | `not-ready` | Bumps gen; tears down the pipeline; **session survives** (media info remains queryable) until `close` or replacement. |
| `playback.seek` | `target: SeekTarget` | A | `Accepted { gen: new }`; completes on `seek-completed { gen, position, seekKind: "command" }`; fails on `seek-failed { gen, error }` | `not-ready`, `not-seekable`, `out-of-range` (percent with unknown duration) | Bumps gen. When duration is known, absolute/relative targets are clamped to `[0, duration]` and the clamped position is reported (clamping is truth, not failure). Superseded seeks emit **no** terminal event (§4.2). |
| `playback.set-rate` | `rate: Rate` | S | `Completed(applied rate)` | `out-of-range` | Atomic clock parameter; also emits `playback.rate-changed` (echo + any future engine-side change). |
| `playback.step-frame` | `direction: Next`/`Previous` | A | Auto-pauses first (`state-changed → paused`), then completes on `seek-completed { seekKind: "frame-step", position }` | `not-ready`, `unsupported` (codec cannot step backwards and micro-seek failed) | Bumps gen. `Previous` may be implemented as a micro-seek to the previous frame boundary `[03]` — decoder detail, engine’s choice, invisible on the wire. |
| `playback.set-position` | `fraction: f32` (0.0..=1.0) | A | Same lifecycle as `seek` | `out-of-range`, `not-ready` | Semantically `seek { Percent }`. Exists as a distinct stable ID for slider/scrubber ergonomics and VLC `position`-variable parity. `[PROPOSAL]` — Lead may fold into `seek` before catalog freeze. |

### 2.5 Track commands

| ID | Params | Class | Result / Completion | Errors (specific) | Notes |
|---|---|---|---|---|---|
| `tracks.list` | — | S | `Completed(Vec<TrackInfo>)` | `not-ready` | Reflects the live list including external subtitles. Same data as the latest `tracks.list-ready`. |
| `tracks.get-info` | `id: TrackId` | S | `Completed(TrackInfo)` | `not-ready`, `not-found` | |
| `tracks.set-video-track` | `selector: Id`/`None` | A | Bumps gen; completes on `tracks.changed { kind: video, selected }` | `not-ready`, `not-found` | Decoder re-init; class A is required for truth (menu checkmark waits). |
| `tracks.set-audio-track` | `selector: Id`/`None` | A | Bumps gen; completes on `tracks.changed { kind: audio, selected }` | `not-ready`, `not-found` | |
| `tracks.set-subtitle-track` | `selector: Id`/`None`/`Auto` | S | `Completed(applied selector)` | `not-ready`, `not-found` | Subtitle switching is a selection over already-parsed streams; no A/V pipeline invalidation, hence class S. `Auto` applies the track-selection rules from `product.toml` defaults. |
| `tracks.set-secondary-audio` | `selector: Id`/`None` | A | Completes on `tracks.changed { kind: secondary-audio, selected }` | `not-ready`, `not-found` | Second audio track mixed at `audio.secondary-gain` (config, default −20 dB) — commentary use case. `[PROPOSAL]` — VLC-parity audit of secondary-audio behavior belongs to Phase 0 inventory. |

### 2.6 Audio commands

| ID | Params | Class | Result / Completion | Errors (specific) | Notes |
|---|---|---|---|---|---|
| `audio.set-volume` | `volume: Volume` | S | `Completed(applied volume)` | `out-of-range` | The applied value is the truth (device limits may clip what was requested; the response says what is). |
| `audio.get-volume` | — | S | `Completed(Volume)` | — | Also present in the state snapshot; UI should prefer the snapshot on hydration. |
| `audio.set-mute` | `muted: bool` | S | `Completed(applied bool)` | — | Mute is independent of volume (unmuting restores the previous volume). |
| `audio.list-devices` | — | S | `Completed(Vec<AudioDevice { id, name, is_default }>)` | — | On web: exactly one pseudo-device (`webaudio`). |
| `audio.set-device` | `id: String` | A | Completes on `audio.device-changed { device_id, origin }` | `not-found`, `device` (open failed) | Output-stream migration; no gen bump (pipeline continues). UI keeps old device active until the event. |
| `audio.set-delay` | `delay_ms: i64` (±60 000) | S | `Completed(applied delay)` | `out-of-range` | Positive = audio plays later. Atomic sync-model parameter. |

### 2.7 Video commands

| ID | Params | Class | Result / Completion | Errors (specific) | Notes |
|---|---|---|---|---|---|
| `video.set-fullscreen` | `enabled: bool` | S | `Completed(applied bool)` | `unsupported-for-host` (CLI) | **target: `shell`.** Desktop: Tauri window fullscreen; web: Fullscreen API (requires user gesture — the binding surfaces the browser constraint); CLI: `unsupported-for-host`. |
| `video.set-aspect-ratio` | `ratio: Default`/`Ratio { w, h }` | S | `Completed(applied)` | `invalid-argument` (zero dims) | Renderer parameter; atomic. `Default` = track display aspect ratio. |
| `video.set-crop` | `crop: None`/`Ratio { w, h }`/`Rect { x, y, w, h }` | S | `Completed(applied)` | `invalid-argument` | Renderer parameter; atomic. |
| `video.set-zoom` | `factor: f32` (0.25..=4.0) | S | `Completed(applied)` | `out-of-range` | Display scaling, independent of aspect. |
| `video.set-delay` | `delay_ms: i64` (±60 000) | S | `Completed(applied delay)` | `out-of-range` | Positive = video plays later. |
| `video.take-screenshot` | `path: Option<PathBuf>` | A | Completes on `video.screenshot-saved { path, width, height, origin }` | `io` | Captures the **next presented frame** — inherently async. Default path from `video.screenshot-dir` + filename template (config). Web: the binding converts the result into a browser download. `[PROPOSAL]` — web path policy. |

### 2.8 Subtitle commands

| ID | Params | Class | Result / Completion | Errors (specific) | Notes |
|---|---|---|---|---|---|
| `subtitle.add-file` | `path: PathSpec`, `encoding: Option<String>`, `select: bool` (default true) | A | Completes on `tracks.list-ready` (with the new track present, `origin` set) | `inaccessible`, `unsupported`, `corrupt` | `PathSpec` = file path on desktop/CLI; on web the binding accepts a transferred `Blob` and the engine parses from memory `[PROPOSAL]` — host-specific param variant, same ID. |
| `subtitle.set-sync` | `delta_ms: i64` (relative) | S | `Completed(applied total delay)` | `not-ready` | Relative adjustment (matches VLC hotkeys); the result carries the new absolute subtitle delay so the UI never tracks its own accumulation. |
| `subtitle.set-encoding` | `encoding: String` (IANA name) | S | `Completed(applied)` | `invalid-argument` | Re-decodes external text subtitles; bounded work. |
| `subtitle.set-scale` | `scale: f32` (0.2..=5.0) | S | `Completed(applied)` | `out-of-range` | Render size multiplier. |

### 2.9 Playlist commands

The playlist is a model owned by the engine; model edits are class S. Two edits have **engine-initiated side effects** that are class A lifecycles of their own: removing/clearing the *playing* item and replacing the list while playing trigger the normal advance/stop state events with `origin: None`. The S response covers only the model edit (documented so the UI is not surprised by following events).

| ID | Params | Class | Result | Errors (specific) | Notes |
|---|---|---|---|---|---|
| `playlist.enqueue` | `source: SourceSpec`, `title: Option<String>` | S | `Completed(PlaylistItemId)` | `resource-limit` (max items) | Does not open the item. |
| `playlist.enqueue-many` | `items: Vec<{ source, title? }>` | S | `Completed(Vec<PlaylistItemId>)` | `resource-limit`, `invalid-argument` (empty) | Atomic: all-or-nothing. |
| `playlist.play-item` | `id: PlaylistItemId` | A | Full open lifecycle (as `session.open`) on that item | `not-found` | |
| `playlist.next` | — | A | Open lifecycle of the next item per repeat/shuffle; if none and repeat off → completes on `state-changed → stopped` | — | |
| `playlist.prev` | — | A | Symmetric; VLC-like restart rule: if position > `playlist.prev-restart-threshold` (config, default 5 s), `prev` restarts the current item instead of jumping back `[PROPOSAL]` | — | |
| `playlist.skip` | `n: i32` (negative = back) | A | Open lifecycle of item at `current + n` | `out-of-range` | |
| `playlist.remove` | `id: PlaylistItemId` | S | `Completed(Unit)` | `not-found` | Removing the playing item starts the advance lifecycle (see note above). |
| `playlist.move` | `id`, `to_index: u32` | S | `Completed(Unit)` | `not-found`, `out-of-range` | |
| `playlist.clear` | — | S | `Completed(Unit)` | — | Stops playback if an item was playing (async lifecycle follows). |
| `playlist.set-shuffle` | `enabled: bool` | S | `Completed(applied)` | — | Does not reorder the displayed list; affects advancement order. |
| `playlist.set-repeat` | `mode: Off`/`One`/`All` | S | `Completed(applied)` | — | |
| `playlist.save` | `path: PathBuf`, `format: Option<M3u8`/`Xspf`/`Json>` | S | `Completed(Unit)` | `io`, `unsupported` (format) | Format defaults from file extension. Bounded by the item cap. |
| `playlist.load` | `path: PathBuf`, `append: bool` | S | `Completed(Vec<PlaylistItemId>)` | `inaccessible`, `unsupported`, `corrupt` | Parse + model insert only; nothing opens. `append: false` while playing triggers the stop lifecycle. |

### 2.10 Media-information commands

| ID | Params | Class | Result | Errors (specific) | Notes |
|---|---|---|---|---|---|
| `media.get-info` | — | S | `Completed(MediaInfo)` | `not-ready` | `{ source, container, title, duration: Duration, seekable, is_live, metadata: Map, tracks: Vec<TrackInfo>, chapters: Vec<Chapter> }`. Strings are untrusted (V7). |
| `media.get-current-time` | — | S | `Completed({ time: Timestamp, position: f32, buffered: Vec<Range<Timestamp>> })` | `not-ready` | Snapshot query; continuous updates come from `time-tick`. |
| `media.get-duration` | — | S | `Completed(Duration)` | `not-ready` | `Unknown` may later become known — announced by `duration-known`. |
| `media.get-chapters` | — | S | `Completed(Vec<Chapter { index, time_start, duration, title }>)` | `not-ready` | |
| `media.set-chapter` | `index: u32` | A | Same lifecycle as `seek` (position = chapter start) | `not-ready`, `not-found` | Canonical form of chapter jumps; `seek { Chapter }` is the alias, not the other way round (chapter index is the native concept). |

### 2.11 Configuration commands (Trace: MP §5)

| ID | Params | Class | Result | Errors (specific) | Notes |
|---|---|---|---|---|---|
| `config.get-setting` | `area: String`, `key: String` | S | `Completed({ value, schema_meta })` | `not-found` | Areas/keys come from the one settings schema; no option exists twice. |
| `config.set-setting` | `area`, `key`, `value: SettingValue` | S | `Completed({ applied, restart_required: bool })` | `invalid-argument` (schema violation — never silent coercion), `permission-denied` (admin-locked key via control server) | Writes the user-preference layer (layering per MP §5 is engine-internal policy). Emits `config.setting-changed`. |
| `config.reset-setting` | `area`, `key` | S | `Completed({ applied, restart_required })` | `not-found` | Resets to the layer default (profile or product). |
| `config.list-areas` | — | S | `Completed(Vec<{ area, setting_count, name_key, any_restart_required }>)` | — | Drives the settings screen navigation, generated from the schema. |

### 2.12 System commands

| ID | Params | Class | Result | Errors | Notes |
|---|---|---|---|---|---|
| `system.get-engine-info` | — | S | `Completed(EngineInfo)` | — | `{ engine_version, catalog_version, protocol_version, host, features, plugins, decoders: Vec<{ name, kind, hardware: bool, backend: Option<String>, active: bool }>, sessions: Vec<{ id, state, gen }> }`. **Hardware decode is reported here** — truthful `hardware: true` with the OS backend name (MP §2 hardware-decoding decision). |
| `system.list-plugins` | — | S | `Completed(Vec<PluginInfo { id, kind, version, capabilities, enabled, source: Builtin/Wasm, permissions }>)` | — | Registry from manifests (MP §6). |
| `system.plugin-enable` | `id: String` | S | `Completed({ applied, takes_effect, restart_required })` | `not-found` | Config can enable a *registered* plugin, never invent one (MP §5). Effect timing depends on plugin kind (documented per manifest). |
| `system.plugin-disable` | `id: String` | S | `Completed({ applied, takes_effect, restart_required })` | `not-found` | Disabling a decoder used by an active session takes effect next session. |
| `system.set-log-level` | `level: Trace`/`Debug`/`Info`/`Warn`/`Error`, `target: Option<String>` | S | `Completed(applied)` | `not-found` (unknown target) | Targets are module paths; omit for global. |
| `system.get-state-snapshot` | — | S | `Completed(StateSnapshot)` | — | `[PROPOSAL]` Full hydration: state, time, duration, rate, volume, mute, delays, tracks + selections, playlist summary + current, devices, subtitle settings. Called by UIs at startup and on reconnect (§4.4). |
| `system.list-commands` | — | S | `Completed(Vec<{ id, class, since, deprecated, requires }>)` | — | `[PROPOSAL]` Machine-readable catalog introspection; lets UIs and the control server build menus and validate keymaps without hardcoding. |

### 2.13 Common error set and redaction (Trace: MP §8, §12)

Every command can fail with any of these; the catalog tables above list only additions.

```rust
pub enum Stage { Session, Input, Probe, Demux, Decode, AudioOutput, VideoOutput,
                 Subtitle, Renderer, Playlist, Config, Plugin, Control }

pub enum ErrorCode {
    // media/engine failures (MP §8 — the seven canonical classes)
    Unsupported,        // no demuxer/decoder/filter for this data
    Corrupt,            // malformed bitstream/container
    Inaccessible,       // unreadable file, permission, auth failure
    Network,            // connection, DNS, TLS, timeout
    Decoder,            // decode failure after selection
    Device,             // audio/video device failure
    ResourceLimit,      // bounded memory/queue/probe limits hit
    // command-layer failures (this document)
    InvalidArgument, OutOfRange, NotFound, NotReady, NotSeekable,
    Canceled,           // superseded by a newer generation (§4.2) — rare on the wire
    FeatureNotExposed,  // exists in engine, off in product.toml feature exposure
    UnsupportedForHost, // host cannot do it (web: no discs, no RTSP, no local FS)
    PermissionDenied,   // control server: sensitive op without authorization
    Io, Internal,
}
```

Rules: (1) a failed open or seek is **never** reported as success (MP §8); (2) `message` is redaction-safe display text (V8) — raw diagnostic chains go to the log sink only; (3) `details` carries structured hints (`"format": "matroska"`, `"allowed_range": "0.0..=1.25"`) for the UI and tests; (4) unknown `ErrorCode` values received by older clients must degrade to a generic failure presentation (§6.2 forward-compatibility).

---

## 3. Event catalog v1

### 3.1 Envelope, delivery, ordering (summary; details §4)

Events are engine→host notifications. Each carries `seq` (global, strictly increasing), optional `session`, optional `gen`, optional `origin` (the `request_id` of the command that caused it, when attributable). Within one host connection, events and command responses share a single ordered delivery channel — a response is causally positioned exactly between the events emitted before and after it (§4.3).

### 3.2 Event table

**Legend.** *Rate* = throttling rule (§3.4). “gen” column: whether the event is pipeline-scoped (carries `gen` and is subject to staleness rules).

| Event ID | Payload | gen | Emitted when | Rate |
|---|---|---|---|---|
| `session.state-changed` | `{ from, to: PlaybackState }` | yes | Every playback-state transition; also self-transitions as idempotent command confirmations (C8) | Never coalesced |
| `session.opening-progress` | `{ progress: f32 }` | yes | Network opens, best-effort | ≤ 1 Hz; absent for local files |
| `session.buffering` | `{ progress: f32 }` | yes | Enter/while/exit buffering (initial and rebuffer) | ≤ 5 Hz; must open with 0.0 and close with 1.0 |
| `timing.time-tick` | `{ time: Timestamp, position: f32 }` | yes | Continuous position while `playing` | Default every 250 ms (config `playback.time-tick-interval-ms`, floor 50 ms); immediate after `seek-completed`, on pause, on rate change |
| `timing.duration-known` | `{ duration: Timestamp }` | yes | Duration transitions from `Unknown`/early estimate to final | Once per change |
| `timing.seek-completed` | `{ gen, position, seekKind: "command"`/`"frame-step"` }` | yes | Terminal for the **newest** seek generation | — |
| `timing.seek-failed` | `{ gen, error: CommandError }` | yes | Terminal for the newest seek generation | — |
| `tracks.list-ready` | `{ tracks: Vec<TrackInfo> }` | yes | Track list becomes available/relevant: before first `playing`, after `subtitle.add-file`, after rare list changes | ≤ 1 Hz coalesced |
| `tracks.changed` | `{ trackKind: TrackKind`/`SecondaryAudio`, `selected: Option<TrackId>` }` | yes | Track selection applied (command) or lost (engine: track disappeared) | — |
| `playback.end-of-track` | `{ trackKind, id: TrackId }` | yes | One elementary stream ended while the media continues (e.g. audio ends before video) | — |
| `playback.end-of-media` | `{}` | yes | Current media finished; followed by `state-changed → ended`, then advance lifecycle or nothing | — |
| `playback.error` | `{ stage, code, message, details, fatal: bool, origin? }` | yes | Any stage failure. Fatal errors transition state to `error`; non-fatal (recoverable decode) do not | Non-fatal same-code errors coalesced per 1 s window with a count `[PROPOSAL]` |
| `playback.rate-changed` | `{ rate: f32 }` | no | `set-rate` echo; future engine-side rate changes | — |
| `audio.device-lost` | `{ device_id: String }` | no | OS removed the active device; migration follows (device-changed) or non-fatal `error` | — |
| `audio.device-changed` | `{ device_id, origin? }` | no | Active output device changed (command or auto-migration) | — |
| `video.screenshot-saved` | `{ path, width, height, origin }` | no | Screenshot written | — |
| `playlist.changed` | `{ kind: ItemsAdded{ids}`/`ItemsRemoved{ids}`/`Moved{id,from,to}`/`Cleared`/`ShuffleSet{enabled}`/`RepeatSet{mode}`/`CurrentChanged{id} }` | no | Any playlist model mutation (command- or engine-initiated) | Model edits coalesced per 100 ms `[PROPOSAL]` |
| `config.setting-changed` | `{ area, key, value, layer }` | no | A setting changed through any layer (user, CLI flag, admin, plugin) | — |
| `plugins.loaded` | `{ id, kind, version }` | no | Plugin registered (startup or install flow) | — |
| `plugins.failed` | `{ id, error }` | no | Plugin failed to load; engine continues without it | — |
| `plugins.crashed` | `{ id, error, action_taken: Disabled`/`Isolated }` | no | Sandboxed WASM extension crashed; playback survives (MP §6 error isolation) | — |
| `system.log` | `{ level, target, message }` | no | Log line surfaced to in-app log panel; level-filtered by `system.set-log-level` | Rate-limited per target (default 100/s then aggregate “N suppressed”) `[PROPOSAL]`; ring buffer of last 1 000 for late subscribers |
| `media.info-ready` | `{ info: MediaInfo }` | yes | Metadata snapshot just before first `playing` (hydration convenience) | Once per open |

`session.state-changed`, `timing.seek-completed`, `timing.seek-failed`, `playback.error`, `tracks.changed`, `tracks.list-ready` and `media.info-ready` are the **completion evidence events** for class-A commands (§1.4 C3) — this is their contract role, not an incidental use.

### 3.3 Playback state machine (Trace: MP §8 “bounded, observable recovery”)

```rust
pub enum PlaybackState { Idle, Opening, Buffering, Playing, Paused, Stopped, Ended, Error }
// wire: "idle" | "opening" | "buffering" | "playing" | "paused" | "stopped" | "ended" | "error"
```

Valid transitions (conformance-tested): `idle→opening` (open), `opening→{buffering, error, idle, stopped}` (early probe failure exits via `error` then `idle` on close; stop-during-opening is card 3.C1), `buffering→{playing, paused, stopped, error}` (the buffered pause and stop-during-buffering are card 3.C1), `playing↔paused`, `playing→buffering` (rebuffer), `playing/paused→{stopped, ended, error}`, `stopped/ended/error→idle` (close/replace), plus self-transitions for idempotent confirmations (C8; the work-carrying paused-while-priming bookend included). Any other sequence is a conformance failure. Close from an active state composes through `stop` engine-side (`… → stopped → idle` — no direct active→idle edge on the wire). Every session-lifecycle event after `session.open`’s `Accepted` carries that session’s current `gen`; `state-changed → error` is always accompanied by a fatal `playback.error` (ordering: `playback.error` first, then `state-changed`).

### 3.4 Throttling rules (summary)

1. `time-tick` is the only continuous stream; its interval is config-driven (default 250 ms, floor 50 ms) and it is **never queued** — at most one tick in flight per session; a newer tick replaces an undelivered older one.
2. Progress events (`buffering`, `opening-progress`) are monotonic within one episode and coalesced to their rate cap.
3. All other events are discrete and unthrottled, bounded naturally by the operations that cause them.
4. The engine guarantees a burst bound: no more than 1 000 events/s sustained to any host connection; beyond that `system.log` is dropped first (with a final aggregate line). `[PROPOSAL]` — number to be validated against the UI budget in Phase 2.

---

## 4. Correlation and ordering

### 4.1 `request_id` — monotonic per host connection

- `u64`, assigned by the **sender** (UI, CLI, control client), starting at 1, strictly monotonic within one connection. Reuse is forbidden; hosts that reload (webview refresh) restart at 1 — safe because the engine never compares ids across connections.
- The engine **echoes** `request_id` in every response and in `origin` on attributable events. It does not interpret ids (except the control server, which enforces monotonicity per connection as an anti-replay measure, §5.4).
- The typed clients (§5) expose promise-based APIs: `client.seek(target)` allocates the id, sends, and resolves on the matching response (`Accepted` for class A) — completion is awaited via the event API (`client.waitFor("seek-completed", { gen })`) or a convenience `seekAndComplete()` that does both and enforces C1–C6 internally, so application code cannot accidentally violate the truth rule.

### 4.2 Generation IDs (Trace: MP §8 “generation IDs or cancellation tokens”)

- Per-session `gen: u64`, starting at 0 at open. The engine increments it **atomically at accept time** of any gen-bumping command: `playback.seek`, `playback.set-position`, `playback.step-frame`, `media.set-chapter`, `playback.stop`, `tracks.set-video-track`, `tracks.set-audio-track`, `session.close` (terminal), plus engine-initiated invalidations (end-of-media restart). `[03]` governs how the engine propagates cancellation internally; on the wire only `gen` is visible.
- **Reconciliation with [03] §5 (two-level epochs).** 03 defines a *session epoch* (minted on open/source replacement) and a *pipeline generation* (bumped on seek/switch/stop). Wire mapping: `SessionId` ↔ 03 session epoch; `gen` ↔ 03 pipeline generation. 03’s internal event names map to wire IDs: `StateChanged` → `session.state-changed`, `TracksChanged` → `tracks.list-ready`, `Seeked` → `timing.seek-completed`, `Buffering` → `session.buffering`, `SessionClosed` → (absorbed: `session.close` response + session filtering), `FirstFrame` → internal trigger of the `playing` transition, no separate v1 wire event. Any further 03-internal events are engine-private and must not leak onto the wire.
- `Accepted` responses to gen-bumping commands carry the **new** gen; non-bumping async commands carry the current gen.
- **Staleness rule (hosts):** ignore any pipeline-scoped event whose `gen` is lower than the highest gen observed for that session (via `Accepted` responses or events). The typed clients implement this once; hand-rolled consumers are a contract-test target (§7.3).
- **Staleness rule (engine):** a superseded seek emits **no** terminal event — seek A canceled by seek B is silent; only B’s lifecycle is observable. This prevents error flashes and makes “exactly zero or one terminal seek event per gen” a testable invariant.
- A gen-bumping command raced against a pending class-A command of an older gen implicitly cancels it: the older command’s completion either arrives before the bump (valid) or is suppressed as stale. There is no “zombie success”.

### 4.3 Ordering guarantees

1. `seq` is engine-global, `u64`, strictly increasing, assigned at emission. A host observes a strictly increasing `seq` stream (per connection); gaps indicate filtered events (subscriptions, §4.4) or dropped `system.log` lines — never reordering.
2. Events and responses travel one ordered channel per host connection: the engine emits `Accepted` for request R before any event caused by R; delivery preserves that causal order end-to-end in every binding (Tauri event + invoke ordering caveat noted in §5.1).
3. Per session, events are totally ordered by `seq`; cross-session order is unspecified (concurrent sessions are not a v1 UI concern, but the control server may see them interleaved).
4. Terminal invariants (conformance-tested): exactly one terminal seek event per completed seek gen; `state-changed → error` always preceded by its fatal `playback.error`; `tracks.list-ready` is delivered before the first `state-changed → playing` of a session; `duration-known` is delivered before any `time-tick` that assumes it.

### 4.4 Subscription model

- **In-process hosts** (Tauri, wasm, CLI): all events are delivered to the host; filtering happens client-side (typed client exposes `on(eventId, filter)`).
- **Control server:** explicit `subscribe { topics }` / `unsubscribe` protocol messages. Topic syntax: exact ID, `area.*` (single level), `area.>` (multi-level, MQTT-style). `[PROPOSAL]` Default subscription after `hello`: nothing (deny-by-default); clients subscribe explicitly. Sensitive topics are none today because no event payload carries secrets (V8) — the rule exists so future events must justify inclusion.
- **Reconnect/hydration:** late joiners and reconnecting clients call `system.get-state-snapshot`, then subscribe; `seq` lets them detect anything missed in between (gap = re-snapshot). `[PROPOSAL]`

---

## 5. Bindings — four hosts, one catalog

### 5.1 Tauri IPC (desktop) (Trace: MP §2 “Desktop shell: Tauri”)

**One registered command.** The desktop shell registers a single Tauri command:

```rust
#[tauri::command]
async fn lumen_dispatch(state: tauri::State<EngineHandle>,
                        request: CommandRequest) -> Result<CommandResponse, CommandError>
```

Rationale: a generic entrypoint keeps the catalog data-driven (no per-command Rust glue to drift), exercises exactly one serde path, and makes the anti-bypass test trivial (§7.4 — the shell must register exactly one command). `CommandRequest`/`CommandResponse`/`Event` derive `Serialize`/`Deserialize` with `rename_all = "camelCase"`, internally-tagged enums (`"cmd"`, `"kind"`), and kebab-case variant names — e.g.:

```json
// UI → engine
{ "requestId": 42, "cmd": "playback.seek", "args": { "target": { "kind": "relative", "deltaMs": -10000 } } }
// engine → UI (response)
{ "requestId": 42, "session": 7, "gen": 3, "result": { "accepted": { "session": 7, "gen": 4 } } }
// engine → UI (event, via Tauri event channel "lumen://event")
{ "seq": 1031, "session": 7, "gen": 4, "origin": 42, "kind": "timing.seek-completed",
  "position": 63000000, "seekKind": "command" }
```

**Events.** The shell forwards every engine event with `app_handle.emit("lumen://event", &event)`. Ordering caveat: Tauri `invoke` responses and `emit` travel different JS-side microtask paths, so the shell **must** forward both through the same `tokio::mpsc` before touching the webview, preserving §4.3 guarantee 2; the conformance suite tests this on the Tauri mock host.

**Shared types.** `[PROPOSAL]` **Schema-first codegen**: `crates/commands` derives `schemars::JsonSchema` for every wire type; `cargo xtask schema-gen` emits (a) `schemas/commands.schema.json` — the single machine description of the catalog; (b) `ui/packages/commands/src/types.ts` — the TS types the UI imports; (c) validation JSON Schemas reused by the control server and the keymap validator; (d) this document’s tables can later be generated from the same source. CI fails on drift between Rust types and generated artifacts. No TS type is ever hand-written.

**Typed client.** `ui/packages/client` wraps `invoke("lumen_dispatch")` + `listen("lumen://event")` into the promise/event API of §4.1, implementing the staleness filter and C1–C6 once. The UI imports only `@lumen/client` and `@lumen/commands` types.

### 5.2 WASM binding (web) (Trace: MP §2 “Web build”, §10)

The engine compiles to `wasm32-unknown-unknown`; `apps/web` exposes it through a typed binding package `@lumen/wasm-client` with the same catalog, envelope and semantics.

- **Single-threaded constraint (v1).** The engine’s internal scheduler is cooperatively chunked: `dispatch()` returns a `Promise`; heavy work (probe, demux, decode batches) runs in bounded slices and yields to the event loop so the page stays responsive; timers and I/O come from host shims (`Performance`, `fetch`). No blocking waits exist on this path.
- **Worker-thread plan.** Per MP §10, the production web host runs the engine in a dedicated **Web Worker** with `postMessage` request/response and transferable frame buffers to the main-thread canvas; the typed client API is unchanged (it abstracts the worker channel). Threads inside wasm (`SharedArrayBuffer`, wasm threads) additionally require COOP/COEP headers — served by `apps/web` and recorded as a deploy requirement, not a fallback to native code.
- **Rendering/audio.** WebGPU with WebGL fallback (MP §10); WebAudio output. These are host capabilities behind the same commands (`video.*`, `audio.*`).
- **Capability gaps are typed, not silent.** Commands the web host cannot honor return `unsupported-for-host`: optical discs, capture, RTSP/UDP (no raw sockets), `audio.list-devices` beyond the single WebAudio device, local-filesystem paths in `session.open` (URLs and transferred `Blob`s only — the File System Access API may lift this later `[PROPOSAL]`). Feature-matrix web column tracks each gap with its reason.
- **Param variants for web.** `subtitle.add-file` accepts a transferred `Blob`; `session.open` accepts `Url` sources and blobs. Same command IDs, schema-validated variant. `[PROPOSAL]`

### 5.3 CLI (apps/cli) (Trace: MP §4 layout, §5 settings-driven help)

The CLI is a headless host consuming the same catalog through an in-process channel. Two modes:

- **Interactive/controller mode** (default when a media is given): opens the media, renders a minimal TUI/status line, applies subcommands, and exits on end-of-media or `q`.
- **Machine mode**: `--json` emits every event as one NDJSON line on stdout; requests are accepted on stdin as NDJSON envelopes (`lumen --control-stdio`). `[PROPOSAL]` — this stdio control mode is also the transport used by the conformance suite’s `CliHost` (§7.2) and doubles as a scripting interface.

**Top-30 subcommand mapping** (stable; `--help` text for settings-derived flags is generated from the settings schema per MP §5):

| CLI usage | Command ID | Arg mapping notes |
|---|---|---|
| `lumen open <path\|url> [--start-time 30s] [--rate 1.5] [--start-paused] [--no-video] [--no-audio]` | `session.open` | flags → `OpenOptions` |
| `lumen close` | `session.close` | |
| `lumen play` / `lumen pause` / `lumen toggle` | `playback.play` / `playback.pause` / `playback.toggle-play-pause` | |
| `lumen stop` | `playback.stop` | |
| `lumen seek 90s` / `lumen seek +10s` / `lumen seek -1m` / `lumen seek 50%` / `lumen seek chapter 3` | `playback.seek` | parsed into `SeekTarget` variant |
| `lumen rate 1.5` | `playback.set-rate` | |
| `lumen step` / `lumen step back` | `playback.step-frame` | |
| `lumen volume 80` / `lumen volume +5` | `audio.set-volume` | percent scale 0–125 |
| `lumen mute` / `lumen unmute` | `audio.set-mute` | |
| `lumen tracks` | `tracks.list` | prints table |
| `lumen audio-track 2` / `lumen audio-track none` | `tracks.set-audio-track` | |
| `lumen video-track 1` / `lumen video-track none` | `tracks.set-video-track` | |
| `lumen subtitle-track auto` / `lumen subtitle-track 3` / `none` | `tracks.set-subtitle-track` | |
| `lumen subtitle-add <file>` | `subtitle.add-file` | |
| `lumen info` | `media.get-info` | `--json` for raw |
| `lumen time` / `lumen duration` | `media.get-current-time` / `media.get-duration` | |
| `lumen chapters` / `lumen chapter 2` | `media.get-chapters` / `media.set-chapter` | |
| `lumen devices` / `lumen audio-device <id>` | `audio.list-devices` / `audio.set-device` | |
| `lumen screenshot [path]` | `video.take-screenshot` | |
| `lumen enqueue <items...>` | `playlist.enqueue-many` | |
| `lumen next` / `lumen prev` | `playlist.next` / `playlist.prev` | |
| `lumen playlist save <path>` / `lumen playlist load <path>` | `playlist.save` / `playlist.load` | |
| `lumen shuffle on` / `lumen repeat one` | `playlist.set-shuffle` / `playlist.set-repeat` | |
| `lumen plugins` / `lumen engine-info` | `system.list-plugins` / `system.get-engine-info` | |
| `lumen setting get/set <area.key> [value]` | `config.get-setting` / `config.set-setting` | key validation from schema |
| `lumen log-level debug` | `system.set-log-level` | |

Settings surface as generated long flags (`--<area>-<key> <value>`) with `--help` text from schema descriptions and localization keys — the CLI never defines an option outside the one schema (MP §5). Short flags are a fixed, documented table; collisions with generated flags are rejected at build time by `cargo xtask config-validator`.

### 5.4 Control server (apps/control-server + crates/control-protocol) (Trace: MP §3.5, §12)

An optional local control surface for automation and remote-control-style UIs.

- **Loopback-only bind.** Listens on `127.0.0.1` / `::1` only (default port from config, or `0` = ephemeral; `control-server.enabled` defaults to **false**). Any configuration that requests a non-loopback address is a **startup error**, not a warning — deny-by-default is structural, not advisory.
- **Versioned protocol.** `[PROPOSAL]` Transport: WebSocket with JSON text frames (browser-reachable), defined in `crates/control-protocol` as typed serde enums reused by both ends — never hand-rolled on either side. Envelope: `{"v":1,"type":"hello"|"welcome"|"request"|"response"|"event"|"subscribe"|"unsubscribe"|"protocol-error"}`. Negotiation: client `hello` lists supported major versions; server picks the highest mutually supported major and answers `welcome { v, engineInfo, capabilities }`, otherwise closes with a protocol error code. One major per connection; serving two majors means two listeners/paths during migrations (§6.3).
- **Authentication — token handshake.** `[PROPOSAL]` The server generates a 256-bit random token per start and (a) writes it to `<runtime-dir>/lumen/control-token` with mode 0600, or (b) inherits it via `--control-token-fd` from a launching wrapper, or (c) prints it to stdout with `--print-token`. The client presents the token in `hello`; comparison is constant-time. Failure closes the connection (WS close code 4401); 5 failures impose a 60 s lockout and a log entry. Without a valid token **no command is executed** — deny-by-default. An explicitly configured `read-only` mode admits unauthenticated clients to a fixed safe subset (`media.get-info`, `system.get-engine-info`, `tracks.list`, snapshot) and nothing else; `session.open` stays privileged because it touches the local filesystem.
- **Request validation.** Every frame: max 1 MiB; schema-validated against `schemas/commands.schema.json` (same artifact as §5.1 — one schema, all hosts); unknown fields/IDs rejected; `request_id` monotonicity enforced per connection (anti-replay); per-connection concurrency cap (8 in-flight requests) and rate limit (100 req/s burst 200) `[PROPOSAL]` numbers. Non-negotiable: reflection, arbitrary path access and plugin installation are out of scope for v1 control.
- **Origin check.** Browser WS clients present an `Origin`; the server rejects any origin not on the configured allowlist (default empty → non-browser clients only). `[PROPOSAL]`
- **Redaction.** Tokens never logged; URLs credential-stripped before logging; error messages pass V8.

---

## 6. Versioning and evolution (Trace: MP §5 “schema version and migrations”)

### 6.1 The v1 namespace

All IDs in §2–§3 live under implicit major **v1**. `CommandId` values, param shapes, result shapes, event payloads and error codes are frozen at v1 except for the additive rules below. The wire envelope carries an explicit version on hosts that negotiate (control server `v`; Tauri/wasm/CLI are same-build, so `system.get-engine-info → catalog_version` is the runtime check).

### 6.2 Additive-change rules (within a major)

**Allowed:** new command IDs; new event IDs; new *optional* params (serde default, never required); new optional result fields; new `ErrorCode`/`Stage` values (clients must degrade unknown codes to a generic failure with the message); new `CommandOutcome` shapes are **not** allowed; new `PlaybackState` variants only with a documented transition extension.

**Forbidden:** removing, renaming, retyping, or re-semanticizing any ID, param, result, event, or error code; changing enum variant names; changing `Accepted`/`Completed` classification of an existing command (UI truth logic depends on it); changing generation or ordering semantics.

**Rule of thumb:** a v1 client that ignores everything it does not recognize must keep working against any v1.x engine.

### 6.3 Deprecation and major bumps

Entries gain `deprecated_in` metadata (surfaced via `system.list-commands`); deprecated commands keep working for the entire major. Removal happens only at a major bump, announced in the release notes and in `welcome` capability flags. The control server negotiates majors per connection, so an engine supporting v1+v2 can serve old clients during a migration window. Catalog majors track the `commands` crate major version.

### 6.4 Feature-gating by product.toml (Trace: MP §5 “Feature exposure lists only implementations that exist”)

Each catalog entry declares the features it requires (e.g. `session.open` + `SourceSpec::Disc` → `optical-disc`; URL schemes → `streaming.http`, `streaming.rtsp`; `audio.list-devices` → `audio.device-enumeration`). Effective availability at runtime = engine build features ∩ `product.toml` feature exposure ∩ host capability (web). A command outside the effective set returns `feature-not-exposed` (config-off) or `unsupported-for-host` (host-incapable) with the missing feature named in `details`. UIs render menus from `system.list-commands` + `get-engine-info → features`, so unavailable entries grey out **from data**, never from hardcoded lists. Config can never enable a missing implementation (MP §5).

---

## 7. Contract tests — the testkit conformance suite (Trace: MP §13)

### 7.1 Design

`crates/testkit` defines the host abstraction and one **data-driven conformance suite**; every host implements the abstraction and runs the same scenarios. A scenario file (`tests/contract/scenarios/*.toml`) is a list of steps:

```toml
[[scenario]]
name = "seek-superseded"
steps = [
  { open = "fixture://sine-colorbars-30s.mkv", expect_state = "playing" },
  { send = { cmd = "playback.seek", target = { kind = "absolute", time_us = 1000000 } },
    expect = { accepted = { gen_bump = true } } },
  { send = { cmd = "playback.seek", target = { kind = "absolute", time_us = 20000000 } },
    expect = { accepted = { gen_bump = true } } },
  { expect_events = [
      { kind = "timing.seek-completed", gen = "latest", position_us = 20000000 },
    ],
    forbid_events = [ { kind = "timing.seek-completed", gen = "older" },
                      { kind = "timing.seek-failed" } ] },
]
```

Scenario categories (minimum): open→play→pause→stop happy paths; open of missing/corrupt/unsupported fixtures (`error` paths with exact stage/code); seek on unseekable live stream (`not-seekable`); **stale-generation races** (the case above; also seek-vs-stop, track-switch-vs-seek); idempotent self-transition echoes; event ordering invariants (§4.3.4); state-machine transition legality (§3.3); error redaction (no fixture path/URL leaks in `message`); subscription filtering and `seq` gap detection (control host); throttle conformance (`time-tick` rate, buffered coalescing). Fixtures come from `tests/fixtures` (legally distributable, plus generated malformed/truncated files, MP §13).

### 7.2 The four hosts under test

| Host | Implementation | Exercises |
|---|---|---|
| `TauriMockHost` | runs the real `lumen_dispatch` handler against a mock `AppHandle`, same serde path as IPC | serialization, shell routing, event forwarding order |
| `WasmNodeHost` | `wasm-bindgen-test` in Node (jsdom for any DOM shims), engine compiled to wasm32 | wasm slicing/yielding, capability errors, blob param variants |
| `CliHost` | spawns `apps/cli --control-stdio --json`, NDJSON over stdin/stdout | CLI arg mapping, machine-mode envelopes |
| `ControlHost` | spawns `apps/control-server` on `127.0.0.1:0`, performs token handshake | auth (valid/invalid/lockout), negotiation, validation, rate limits |

The suite is merge-blocking in CI and runs on Windows, Linux and macOS for the native hosts (MP §13); the wasm host runs in the web job.

### 7.3 Truth-rule assertions

Every scenario that contains a class-A command asserts the **no-early-success** property at the client level: the typed client must not resolve `andComplete()` before the terminal event, and mock-UI harnesses (a tiny scripted consumer using `@lumen/client`) assert that no “success” render callback fires before the completion event. This makes C1–C8 executable, not aspirational.

### 7.4 Bypass prevention — “the UI never goes around the layer”

1. **Import allowlist** (lint on `ui/`): engine-adjacent imports limited to `@lumen/commands`, `@lumen/client`; `dependency-cruiser`/ESLint `no-restricted-imports` fails CI on violations. `[PROPOSAL]`
2. **ESLint bans** direct Tauri `invoke`/`listen` calls outside `@lumen/client` (`no-restricted-globals`/`no-restricted-properties` rules). `[PROPOSAL]`
3. **Single-command check**: an architecture test reads the desktop shell’s `invoke_handler` registration and fails if any command other than `lumen_dispatch` is registered.
4. **Web build**: only `@lumen/wasm-client` may import the wasm package (build-time module graph assertion). `[PROPOSAL]`
5. **Cargo boundary check** (already mandated MP §3.8): `apps/*` and `ui` may depend on `crates/commands` types but never on engine internals; the testkit depends on production crates, never the reverse (MP §3.7).

---

## 8. Keymaps and menus as data (Trace: MP §5 “Commands and keymaps are data”)

### 8.1 Keymap format

`config/keymaps/default.toml` (seed) with user-override layering identical to settings precedence (MP §5). Validated by `schemas/keymap.schema.json` and `cargo xtask config-validator`:

```toml
schema = "keymap.v1"
[[binding]]
id = "seek-forward-short"          # stable binding id (menus reference these)
key = "Right"                       # canonical key name; modifiers: "Ctrl+Right", "MediaNext"
command = "playback.seek"
args = { target = { kind = "relative", delta_ms = 10000 } }
mode = "player-focused"             # player-focused | global | always
when = "has-media"                  # condition tokens: has-media, has-video, fullscreen, has-chapters
```

Validator failures are hard errors: unknown command ID, args that fail the command’s param schema, duplicate `key`+`mode` combos, and contradictory conditions. Menu accelerators reference binding **ids** (not raw keys), so a user rebinding updates every menu automatically. `[PROPOSAL]` — binding-id indirection.

### 8.2 Default keymap seed (v1) `[PROPOSAL]`

Sane defaults consistent with common players (VLC/mpv/browser conventions); final defaults are pinned in Phase 0 against the VLC 3.0.23 hotkey inventory — this table is the starting proposal:

| Key | Binding | Command / target |
|---|---|---|
| `Space` | toggle | `playback.toggle-play-pause` |
| `MediaPlayPause` / `MediaPlay` / `MediaPause` / `MediaStop` | media keys | `playback.toggle-play-pause` / `playback.play` / `playback.pause` / `playback.stop` |
| `MediaNext` / `MediaPrev` | media keys | `playlist.next` / `playlist.prev` |
| `←` / `→` | seek short | `playback.seek` relative ∓/±10 s |
| `Shift+←` / `Shift+→` | seek fine | `playback.seek` relative ∓/±3 s |
| `Ctrl+←` / `Ctrl+→` | seek long | `playback.seek` relative ∓/±60 s |
| `PageDown` / `PageUp` | chapters | `media.set-chapter` next / previous |
| `↑` / `↓` | volume | `audio.set-volume` +/−5 % |
| `M` | mute | `audio.set-mute` (toggle semantics at client) |
| `F` / `Esc` | fullscreen | `video.set-fullscreen` true / false |
| `S` | stop | `playback.stop` |
| `N` / `P` | playlist | `playlist.next` / `playlist.prev` |
| `E` / `Shift+E` | frame step | `playback.step-frame` next / previous |
| `[` / `]` | rate | `playback.set-rate` −/＋0.25 (client computes from snapshot) |
| `Backspace` | rate reset | `playback.set-rate` 1.0 |
| `B` | audio cycle | `tracks.set-audio-track` next (client-side cycle from `tracks.list`) |
| `C` | subtitle cycle | `tracks.set-subtitle-track` auto → ids → none |
| `G` / `H` | sub sync | `subtitle.set-sync` +/−100 ms (VLC-style keys `[VERIFY]`) |
| `0`–`9` | percent seek | `playback.set-position` 0 %–90 % |
| `Shift+S` | screenshot | `video.take-screenshot` |
| `Ctrl+O` | open file | UI action → `session.open` (§8.4) |
| `Ctrl+Shift+O` | open URL | UI action → `session.open` |
| `Ctrl+L` | playlist panel | UI action |
| `Ctrl+I` / `J` | info / metadata | UI actions (`media.get-info` consumers) |
| `Q` / `Ctrl+Q` | quit | shell action |

Note the pattern: **composite client-side behaviors** (toggle, cycle, rate-step) are thin recipes in the typed client over primitive commands — the catalog stays free of convenience variants, and the recipes are part of the keymap data (each binding may name a `recipe` instead of a raw command). `[PROPOSAL]`

### 8.3 Menus as data

`layouts/menus.toml` declares menu trees (MP §5 “which panels, menus… exist, and their order, come from layout config”): each leaf is `{ label_key, command | ui_action, args?, accelerator_binding? }`. Labels are localization keys only (MP §5, no literal text). **Dynamic submenus** (audio/video/subtitle tracks, audio devices, chapters) are declared as `{ dynamic: tracks.audio }` and populated at runtime from `tracks.list` / `audio.list-devices` / `media.get-chapters` — always data from queries, never hardcoded entries. Representative v1 menu→command map:

| Menu | Item → target |
|---|---|
| Media | Open File… (ui-action), Open URL… (ui-action), Save Playlist… (`playlist.save`), Quit (shell) |
| Playback | Play/Pause (`playback.toggle-play-pause`), Stop (`playback.stop`), Next/Previous (`playlist.next`/`prev`), Speed ▸ (`playback.set-rate` presets), Chapters ▸ (dynamic → `media.set-chapter`), Record (feature-gated, later phase) |
| Audio | Volume ▸ (`audio.set-volume`), Mute (`audio.set-mute`), Audio Device ▸ (dynamic → `audio.set-device`), Audio Track ▸ (dynamic → `tracks.set-audio-track`), Stereo Mode (feature-gated filter, later) |
| Video | Fullscreen (`video.set-fullscreen`), Aspect Ratio ▸ (`video.set-aspect-ratio`), Crop ▸ (`video.set-crop`), Zoom ▸ (`video.set-zoom`), Video Track ▸ (dynamic → `tracks.set-video-track`), Take Snapshot (`video.take-screenshot`), Deinterlace (feature-gated, later) |
| Subtitle | Add Subtitle File… (ui-action → `subtitle.add-file`), Subtitle Track ▸ (dynamic: auto/none/ids → `tracks.set-subtitle-track`), Sync −100 ms/+100 ms (`subtitle.set-sync`), Scale ▸ (`subtitle.set-scale`), Encoding ▸ (`subtitle.set-encoding`) |
| Playlist | Shuffle (`playlist.set-shuffle`), Repeat Off/One/All (`playlist.set-repeat`), Clear (`playlist.clear`), Save/Load (`playlist.save`/`load`) |
| Tools | Media Information (ui-action ← `media.get-info`), Plugins (ui-action ← `system.list-plugins`), Engine Info (ui-action ← `system.get-engine-info`), Preferences (ui-action → settings screens) |
| Help | About, Online Docs/Privacy/Support (URLs from `product.toml`) |

Menu items whose command is not in the effective feature set (§6.4) render disabled **from data** — consistent with the truthful-UI rule: never show a working-looking control for something the engine cannot do.

### 8.4 UI-action registry `[PROPOSAL]`

Engine-irrelevant actions (dialogs, panel toggles, focus management) are declared in `layouts/ui-actions.toml` (id, label_key, capability such as `file-dialog`). Keymap and menu entries reference either `command` or `ui_action` — never both. The registry is schema-validated so the CI “no hardcoded strings / no orphan references” checks (MP §5) cover it.

---

## 9. Open questions for the Lead (decisions requested, per AGENTS §3.6)

1. **`playback.set-position` vs `seek`** — keep both (slider ergonomics, VLC `position` parity) or fold into `seek` before the catalog freeze? (§2.4)
2. **UI-action dual namespace** in keymaps/menus (§8.4) vs adding `ui.*` commands to the engine catalog — interface-shaping, needs a call.
3. **Control-server transport** — WebSocket (proposed) vs plain TCP NDJSON vs both; and whether read-only unauthenticated mode should exist at all (§5.4).
4. **Token distribution mechanism** for the control server (file / fd / print) — desktop integration detail with security implications (§5.4).
5. **`origin` on events** (request attribution) — cheap and proposed here; confirm as contract, since removing it later would be a breaking change (§4.1).
6. **Secondary audio semantics** — needs the Phase 0 VLC inventory to pin behavior; is the −20 dB mix default acceptable? (§2.5)
7. **Numeric defaults** marked `[PROPOSAL]`: volume ceiling 1.25, max rate 16.0, time-tick 250 ms, S-class budget 50 ms, event burst bound 1 000/s, playlist cap 10 000, prev-restart threshold 5 s. These are config-driven and cheap to change now, expensive after UI code lands.
8. **Web param variants** (`Blob` sources for `session.open`/`subtitle.add-file`) — confirm the polymorphic-param approach over separate `*-from-blob` command IDs (§5.2).

## Unfinished

None. All eight assigned areas are covered (§1–§8). Dependent follow-ups for other agents: 1.A3 engine plan should adopt §4.2’s generation semantics; the Wave B config/UI/testing docs (07/08/10) should reference the catalog tables and the testkit design here.


