# AUDIT: crates/playback-engine/src/event.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 1061 / 19 enum variants + manual Clone (286-380) + manual PartialEq (382-510) + as_wire (516-615), is_pipeline_scoped (625-632), EngineEvent::{is_pipeline_scoped, as_wire_event} (654-675), 11 bridge fns (689-954), 4 tests (956-1061) |
| File verdict | **REAL** (type + bridge layer, wired into the live emission path) |
| Reachable from shipped entry point? | YES — `lumen` CLI → `apps/cli/src/host.rs:676` (`event.as_wire_event()`) → `EngineEventKind::as_wire` (this file, 516); producer side: `session.rs:73` imports, `emit`/`emit_raw` build `EngineEvent`s from these kinds; W00 smoke observed `state-changed`, `tracks 2 ready`, `TimingDurationKnown` on the wire. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 4 S4: 2 |
| Commands run | rg (as_wire_event/track_info_wire/media_info_wire callers; Track struct fields; SourceUri::redacted); sed quote verification; no cargo (static per adaptation) |

## 1. One-paragraph verdict

This is the engine-event vocabulary (19 `EngineEventKind` variants) and the engine→wire bridge (doc 05 §4.2 mapping), and it is real code on the live path: `as_wire`/`as_wire_event` are total functions (private kinds `DamagedData`/`FirstFrame` correctly return `None`), the manual `Clone`/`PartialEq` exist for a stated reason (drop the boxed error source chain), and the W00 CLI smoke proves the converted events reach a host. The defects are in what the bridge *omits or distorts*: wire `TrackInfo.video`/`.audio` are unconditionally `None` (the L0 `Track` type genuinely has no dimensions — verified in media-types/src/ids.rs:635-656), `MediaInfo.container` is hardcoded `None` even though the engine *does* know the container (`SessionRuntime.container`, session.rs:1411, set from the CLI host's probe at host.rs:965 and used for error enrichment at session.rs:2404), and `SourceSpec::File.path` is a *redacted display string* (`/…/basename`) stuffed into a type whose meaning is a real filesystem path. The AudioDeviceSuspended event is pipeline-scoped while its sibling device events are session-scoped, so a drain-time generation mint can swallow exactly the terminal notice of the device-loss saga. Tests cover 4 of 19 wire mappings here; the rest are covered cross-file (events_contract.rs, hotplug.rs).

## 2. What actually works (verified)

- `EngineEventKind::as_wire` (516-615): every variant mapped or explicitly private; `EventKind::PlaybackError` fatal:true for `Fatal`, fatal:false for `RecoveredDamage`/`AudioDeviceSuspended` — matches doc 05 §3.2 rows.
- `EngineEvent::as_wire_event` (665-674): envelope (seq/session/generation/origin) copied faithfully; verified reachable: apps/cli/src/host.rs:676 `if let Some(wire) = event.as_wire_event() && self.staleness.filter_event(&wire)`.
- `wire_stage` (689-706) / `wire_error_code` (712-723): total maps, 14→13 stage collapse documented per-arm; the unit test pins Open→Input, Present→Renderer, Mix→AudioOutput.
- Private-kind quarantine: test `private_kinds_never_reach_the_wire` (994-1001) asserts `DamagedData`/`FirstFrame` → `None`.
- W00 baseline smoke (audit/runs/baseline/): the shipped binary produced `state-changed Idle → Opening`, `tracks 2 ready`, `TimingDurationKnown { duration: Timestamp(3000000) }` — these come through this file's bridge (TimeTick/DurationKnown → `Timestamp(ns_to_us(ns))`).

## 3. Findings

### F-001 [S3] — wire `MediaInfo.container` is hardcoded `None` while the engine holds the container name
Where: crates/playback-engine/src/event.rs:920-930, call site session.rs:2785-2791
Code:
```rust
pub fn media_info_wire(
    source: &SourceUri,
    duration_ns: Option<i64>,
    seekable: bool,
    tracks: &[Track],
    selected: &[bool],
) -> commands::catalog::types::MediaInfo {
    commands::catalog::types::MediaInfo {
        source: source_spec_wire(source),
        container: None,
```
What is wrong: the function takes no container parameter and hardcodes `container: None`, with a doc claiming "container/title/metadata/chapters are the v0 honest unknowns". Container is not unknown: `SessionConfig::container` (session.rs:454-460) is set by the CLI host from its probe (apps/cli/src/host.rs:965 `config.container = Some(facts.container.clone())`), retained in `SessionRuntime.container` (session.rs:1411/1432), and used for the `NoDecodableTrack` error enrichment (session.rs:2404). The host's own `media.get-info` answer includes it (host.rs:1299).
Why it matters: `media.info-ready` is the event a UI listens to; it omits a fact the engine possesses, and the event and the command (`media.get-info`) disagree about the same snapshot. The "honest unknowns" comment is false for `container`.
What correct looks like: thread `self.runtime.container.as_deref()` into `media_info_wire` (one parameter, one field).
Confidence: HIGH — Verification: read (session.rs:1409-1411/2398-2404/2785-2791; host.rs:961-965/1299).

### F-002 [S3] — `SourceSpec::File.path` carries a redacted display string, not a path
Where: crates/playback-engine/src/event.rs:899-909
Code:
```rust
pub fn source_spec_wire(source: &SourceUri) -> commands::catalog::types::SourceSpec {
    use commands::catalog::types::SourceSpec;
    let redacted = source.to_string();
    if source.scheme().as_str() == "file" {
        SourceSpec::File {
            path: PathBuf::from(redacted.trim_start_matches("file://")),
        }
```
What is wrong: `SourceUri`'s `Display` is the redacted form (media-types/src/ids.rs:97 "Display/Debug show a redacted form (MP §12)"); for `file` it keeps only the final path component prefixed `…/` (ids.rs `redact_file_path`, "keep only the final path component, prefixed `…/`"). So `SourceSpec::File { path }` — a type whose meaning is a filesystem path — receives `/…/sample.mp4`: a string that is not a path, cannot be re-opened, and misrenders any host logic that treats it as one. `trim_start_matches("file://")` also strips by pattern, not grammar.
Why it matters: the event's `info.source` is the only source identity a listening host gets; it is display junk in a path-typed field. The comment (893-897) admits it and pushes the burden to hosts ("hosts that need the exact original carry it themselves") — a documented workaround, still a typed lie.
What correct looks like: either a redacted-string variant in `SourceSpec` or the raw path passed by the host that already holds it.
Confidence: HIGH — Verification: read (media-types/src/ids.rs:240-300).

### F-003 [S3] — `AudioDeviceSuspended` is pipeline-scoped while its sibling device events are session-scoped — the terminal notice can be drained as stale
Where: crates/playback-engine/src/event.rs:625-632
Code:
```rust
    pub fn is_pipeline_scoped(&self) -> bool {
        !matches!(
            self,
            EngineEventKind::AudioDeviceLost { .. }
                | EngineEventKind::AudioDeviceChanged { .. }
                | EngineEventKind::RendererBackendReport { .. }
        )
    }
```
What is wrong: `AudioDeviceSuspended` is absent from the exemption list, so it counts as pipeline-scoped. It is emitted at the live pair via `push_session_event` → `session.emit_hook` (hotplug.rs:348-352) and, unlike `AudioDeviceLost`/`-Changed`, is suppressed by `is_stale_at_drain` (events.rs:499-505: only seek/track terminals and non-pipeline-scoped kinds survive a superseding mint) if a seek or track switch mints a new generation between emission and drain. The final "playback is suspended, reopen budget spent" notice is then lost while the earlier `AudioDeviceLost` fact survives.
Why it matters: it is the terminal of the device-loss saga (7.C1's own doc: "the bounded reopen window expired … with a non-fatal error"); losing exactly the terminal contradicts doc 05 §4.3 invariant 4's spirit ("exactly one terminal per completed generation") that the staleness policy elsewhere enforces. Narrow window (one drain interval), self-consistent state machine elsewhere — but an inconsistency inside the same event family.
What correct looks like: add `AudioDeviceSuspended` to the session-scoped exemption list.
Confidence: MEDIUM — Verification: read (control flow hotplug.rs:348-352 → events.rs:478-507); race window is inferred, not executed.
Severity note: no S2 because the suspend state itself is separately visible (`state-changed → paused` leads per C2 ordering), so a host is not left blind — only the notice detail is droppable.

### F-004 [S3] — wire `TrackInfo` never carries video/audio detail: `video: None, audio: None` unconditionally
Where: crates/playback-engine/src/event.rs:796-819
Code:
```rust
        is_default: track.flags.default_on,
        is_selected,
        origin: WireTrackOrigin::Stream,
        video: None,
        audio: None,
    }
```
What is wrong: the bridge hardcodes both detail blocks to `None`. The L0 `media_types::Track` (ids.rs:635-656) has only id/stream/kind/language/codec/title/flags/timing — no width/height/sample-rate/channels — so this is not a stale bridge; it is a permanent information floor: no shipped host can learn a track's resolution or sample rate from the event stream (the comment defers to "the demux implementation (2.B2)" which never landed an extended Track).
Why it matters: `tracks.list-ready` and `media.info-ready` are the UI's only track facts; the W00 smoke's "tracks 2 ready (h264 video + aac audio identified)" is the ceiling — the product cannot tell the user the video is 320x240. This is one of the reasons the app "identifies" media but cannot describe it.
What correct looks like: extend `media_types::Track` with optional dimension/rate facts from the demuxer and thread them here.
Confidence: HIGH — Verification: read (media-types/src/ids.rs:635-656).
(Cross-file: the type extension belongs to media-types/demux owners; recorded for the remediation map.)

### F-005 [S4] — `ns_to_us` "rounds up" only for non-negative input; unchecked `ns + 999` can overflow at the i64 extremes
Where: crates/playback-engine/src/event.rs:770-772
Code:
```rust
fn ns_to_us(ns: i64) -> i64 {
    (ns + 999) / 1_000
}
```
What is wrong: for negative ns, `(ns + 999) / 1_000` truncates toward zero, not a ceiling (-1500 ns → 0 µs, not -1 µs); for `ns > i64::MAX - 999` the addition overflows (panic in debug, wrap in release). The comment asserts "Positions are non-negative by construction" — that invariant is enforced nowhere in this file and positions/durations originate from demuxer-parsed timestamps (hostile input) upstream.
Why it matters: latent panic/wrap on adversarial media; wrong-unit output for negative timestamps (MPEG-TS wrap can produce negative PTS in the wild).
What correct looks like: `ns.div_ceil(1_000)` (ceiling for both signs) or documented clamping.
Confidence: MEDIUM — Verification: read; reachability of negative/extreme ns UNVERIFIED (depends on demuxer clamping in other crates).

### F-006 [S4] — module-doc mapping table promises "v0 emits only the enter/exit bookends" for Buffering while events.rs added the ≤5 Hz stream
Where: crates/playback-engine/src/event.rs:109-111
Code:
```rust
    /// A buffering episode progress report (doc 05 §3.2: episodes
    /// open with `0.0` and close with `1.0`; v0 emits only the
    /// enter/exit bookends — the ≤ 5 Hz continuous stream is 3.B4).
```
What is wrong: card 3.B4 landed (the ≤5 Hz in-episode stream is wired at session.rs:2483-2490); this doc still describes the pre-3.B4 v0. Stale self-description.
Why it matters: minor, but this module is the "wire reconciliation" reference; internal doc drift is the repo's chronic disease.
What correct looks like: update the variant doc.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| EngineEventKind (19 variants) | 91-284 | REAL | — | `#[non_exhaustive]`; variants documented with doc-05 row citations; two private kinds (DamagedData, FirstFrame) |
| Clone for EngineEventKind | 286-380 | REAL | — | manual; Fatal rebuilt without source chain (documented); TracksChanged clones the Arc (cheap); MediaInfoReady deep-clones (documented once-per-open exception) |
| PartialEq for EngineEventKind | 382-510 | REAL | — | manual; Fatal compares stage+kind+context.code only (documented); TracksChanged compares full Vec<Track> |
| EngineEventKind::as_wire | 516-615 | REAL | S3 | total bridge; F-001/F-004 omissions live in the callees; RecoveredDamage hardcodes `ErrorCode::Corrupt` (policy, documented); SeekKind::Command hardcoded per doc |
| EngineEventKind::is_pipeline_scoped | 625-632 | REAL | S3 | F-003: AudioDeviceSuspended omitted from the exemption |
| EngineEvent | 639-652 | REAL | — | envelope struct, all fields live |
| EngineEvent::is_pipeline_scoped | 654-660 | REAL | — | delegate |
| EngineEvent::as_wire_event | 662-675 | REAL | — | None for private kinds; envelope copied; called by CLI host (host.rs:676) |
| wire_stage | 689-706 | REAL | — | total 14→13 collapse, each judgment call documented |
| wire_error_code | 712-723 | REAL | — | 7→7 one-to-one, total |
| fatal_details | 727-732 | REAL | — | context code + stage; allocates 2 Strings (per-fatal, fine) |
| seek_error_wire | 741-765 | REAL | — | typed refusal mapping; position_us detail; `[PROPOSAL]` tagged |
| ns_to_us | 770-772 | REAL | S4 | F-005 negative/extreme edge |
| seek_failure_reason | 775-782 | REAL | — | stable keys |
| track_info_wire | 796-819 | REAL | S3 | F-004: video/audio detail permanently None; fourcc/profile/level None (registry debt, documented) |
| recovered_damage_details | 825-829 | REAL | — | count detail |
| suspend_notice_details | 835-841 | REAL | — | context/device/reopen_attempts |
| suspend_notice_message | 847-864 | REAL | — | display-safe wording; mirrors recovery.rs notice (contract-pinned per comment) |
| wire_track_kind | 868-875 | REAL | — | total |
| wire_track_change_kind | 882-890 | REAL | — | total; `secondary-audio` arm unminted (documented follow-up) |
| source_spec_wire | 899-909 | REAL | S3 | F-002: redacted display string in a path-typed field |
| media_info_wire | 920-947 | REAL | S3 | F-001: container None despite known; is_live hardcoded false (honest: no live sources); metadata/chapters empty |
| wire_playback_state | 952-954 | REAL | — | trivial delegate, exists "for readers" |

## 5. Test honesty (4 tests, lines 956-1061)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| fatal_maps_to_wire_playback_error_with_details (963) | YES | pins stage/code/fatal/context/message; a bridge swap or detail drop fails it |
| private_kinds_never_reach_the_wire (994) | YES | DamagedData/FirstFrame → None |
| state_changed_round_trips_through_the_wire_envelope (1004) | YES | envelope fields individually asserted |
| stage_bridge_covers_every_l0_stage (1033) | PARTIAL | the all-stages loop only proves no panic; 3 specific mappings asserted; the rest of the 14 mappings are unasserted here (compiler totality is the real guarantee) |

Coverage gap: 15 of 19 variants' wire mappings have no unit test in this file (Seeked, SeekFailed, TimeTick, DurationKnown, MediaInfoReady, EndOfTrack, RecoveredDamage, TrackSelectionChanged, AudioDeviceLost/Changed/Suspended, RendererBackendReport). Cross-file: events_contract.rs and tests/hotplug.rs exercise several — UNVERIFIED by run (no cargo).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| mapping list 13 rows (StateChanged→…→RendererBackendReport) | event.rs:13-47 | REAL | read of as_wire (516-615), all present |
| FirstFrame/DamagedData "engine-private" | event.rs:39-42 | REAL | as_wire returns None (545-546); unit test |
| "video/audio detail blocks are None until the demux implementation (2.B2) reports width/height" | event.rs:791-793 | REAL (and still owed) | media_types Track has no such fields (ids.rs:635-656) — the debt is real, not stale |
| "container … are the v0 honest unknowns" | event.rs:916-918 | FALSE | container IS known (SessionRuntime.container, session.rs:1411; host.rs:965) — F-001 |
| "Positions are non-negative by construction" | event.rs:768-769 | UNVERIFIED | no clamp in this file; demuxer-side enforcement not audited from here |

## 7. Dead / duplicated / unreachable code

- `wire_playback_state` (952-954) — a named delegate of `SessionState::as_wire` "for event-module readers". rg shows it re-exported (lib.rs:116); callers outside: none found in production (hosts use their own converters). Trivial, harmless — S4-level.
- No dead variants: every kind is constructed somewhere in session.rs/hotplug.rs (verified by rg per-variant: StateChanged, TracksChanged, Buffering, EndOfMedia, Fatal, DamagedData, FirstFrame, Seeked, SeekFailed, TimeTick, DurationKnown, MediaInfoReady, EndOfTrack, RecoveredDamage, TrackSelectionChanged all appear in session.rs; the three device kinds + RendererBackendReport in hotplug.rs — the latter family only reachable via the un-called hotplug API, see events.rs F-001).

## 8. Dependencies & flags

- No feature flags. Depends on `commands` (wire types), `media-types` (Track/SourceUri/EngineError/Stamp), `crate::seek`, `crate::state`. All internal.
- `plugin_api::SeekSupport` referenced in a doc link only (914) — inert.

## 9. Open questions for the next agent

- media-types owner: does any demuxer produce negative or >292-year timestamps that reach `ns_to_us`? (F-005)
- session.rs owner: why `emit_hook` (hotplug path) vs `emit` — confirm the envelope stamping parity.
- wave 18/19: `media.info-ready`'s payload poverty (F-001/F-002/F-004) matters for any "media info" matrix row verdicts.
