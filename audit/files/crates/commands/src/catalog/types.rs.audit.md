# AUDIT: crates/commands/src/catalog/types.rs
| Field | Value |
|---|---|
| Auditor | W02-A07 |
| Date | 2026-11-23 |
| Lines / functions | 907 / 1 method (`can_transition_to`) + 5 tests (45 type definitions) |
| File verdict | REAL (type/contract file; serde wire forms executed; large catalog-only fraction, see F-002) |
| Reachable from shipped entry point? | YES — `lumen` bin → lumen_cli::run::run (apps/cli/src/run.rs:51 `use commands::{Command, …}`) → route (apps/cli/src/dispatch.rs:79-156) matches `Command` payloads whose types are these; engine producers: crates/playback-engine/src/event.rs:65,926 (`WireCodecDescriptor`, `MediaInfo`), apps/cli/src/host.rs:82,1631 |
| Findings | S0: 0 S1: 0 S2: 1 S3: 4 S4: 3 |
| Commands run | `cargo test -p commands --lib catalog::types` → 5 passed/0 failed; rg searches (callers, producers, doc 05 §2.2/§2.3/§3.3) |

## 1. One-paragraph verdict
This is a pure wire-contract file: 45 serde/schemars type definitions for the doc-05 command catalog plus one logic function (`PlaybackState::can_transition_to`, an exact encoding of doc 05 §3.3 line 412's transition table — verified line-by-line). The wire forms are real and test-executed. The hostile finding is that the shipped `lumen` CLI host serves only 14 commands (apps/cli/src/dispatch.rs:81-155); roughly 20 of the 45 types exist only as unserved-catalog payload shapes, including the entire `StateSnapshot` hydration block the UI actively requests and gets a typed refusal for. `OpenOptions` declares 5 fields (rate, track selectors, forced_subtitle_only) that the only shipped host deserializes and then silently drops — a violation of doc 05 §5.2's "capability gaps are typed, not silent".

## 2. What actually works (verified)
— verified by `cargo test -p commands --lib catalog::types` → `test result: ok. 5 passed; 0 failed` (seek wire form `{"kind":"relative","deltaMs":-10000}`, Duration sentinel-free round-trip, deny_unknown_fields rejection, selector round-trips, §3.3 transition table).
— Transition table cross-checked against docs/05-command-layer.md:412 — code (types.rs:326-337) encodes every listed edge and no others. Match is exact.
— Volume/Rate range doc-comments match doc 05 §2.2 lines 186-187 verbatim (both `[PROPOSAL]`).

## 3. Findings
F-001 [S2]
Where: crates/commands/src/catalog/types.rs:247-260 + apps/cli/src/dispatch.rs:81-87
Code:
```
    /// Initial playback rate.
    pub rate: Option<Rate>,
    /// Disable the video track.
    pub no_video: bool,
```
What is wrong: `OpenOptions` carries `rate`, `video_track`, `audio_track`, `subtitle_track`, `forced_subtitle_only`; the only shipped host reads only `start_time`, `no_video`, `no_audio` (+ composes `start_paused` manually): `host.open_session(source.clone(), options.start_time.map(|time| time.0), options.no_video, options.no_audio)`. A `--control-stdio` client sending `session.open { rate: 2.0, videoTrack: … }` gets `Accepted`/success while every one of those options is silently discarded. Why it matters: doc 05 §5.2 (line 492) says "Capability gaps are typed, not silent"; §2.3 (line 227) lists these exact options as schema-validated. This is a silent gap. What correct looks like: host either applies the options or refuses them typed (`unsupported-for-host`/`out-of-range`). Confidence: HIGH — Verification: read (dispatch.rs:81-99).
F-002 [S3]
Where: types.rs:263-288, 405-444, 446-464, 466-500, 502-547, 603-644, 646-720, 722-786
Code:
```
pub struct StateSnapshot {
```
What is wrong: ~20 types (StateSnapshot, TrackSelections, PlaylistSummary, SubtitleSettings, AudioDevice, VolumeKind, AspectRatioSpec, CropSpec, StepDirection, RepeatMode, PlaylistSaveFormat, PlaylistEnqueueItem, PlaylistItemId, LogLevel, ConfigAreaInfo, PluginInfo, PluginSource, PluginAction, PlaylistChange, aliases Volume/Rate-as-arg) have no producer in any shipped host or the engine (rg over crates/ + apps/: no construction sites; the sole `CommandResult::StateSnapshot` construction site is the enum variant itself, command.rs:659). The UI requests the snapshot (ui/src/screens/player/states/context-bridge.ts:208) and the CLI answers the typed refusal — ui tests assert the message "`system.get-state-snapshot` is not served by this build's command-line host". Why it matters: catalog-as-contract is a defensible design (typed refusal at dispatch.rs:154-156 is honest), but as shipped these are dead payload shapes and the UI's reconnect-hydration path is a refusal loop. What correct looks like: either a host serving the snapshot or the UI deferring the request. Confidence: HIGH — Verification: read + rg.
F-003 [S3]
Where: types.rs:740-749
Code:
```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase", default)]
pub struct SubtitleSettings {
```
What is wrong: `derive(Default)` + `#[serde(default)]` with `scale: f32` makes the implicit default `0.0` — a scale of zero (invisible subtitles). The semantically correct default is 1.0. No constructor exists today (rg: only types.rs:742), so it is a landmine: the first `SubtitleSettings::default()` or snapshot JSON missing `"scale"` yields 0.0. What correct looks like: manual `Default` impl with `scale: 1.0`. Confidence: HIGH — Verification: read.
F-004 [S3]
Where: types.rs:596-600 + apps/cli/src/host.rs:1631-1661
Code:
```
    /// Ids of registered plugins.
    pub plugins: Vec<String>,
    /// The decoder registry.
    pub decoders: Vec<DecoderInfo>,
    /// Live sessions.
    pub sessions: Vec<SessionSummary>,
```
What is wrong: the only producer hard-codes the data: `decoders` is a literal two-entry list (`mjpeg`, `pcm_s16le`, always `active: false`), `sessions: Vec::new()` always, `protocol_version: "unwired (control-protocol is a later card)"` (host.rs:1635, 1642-1659). The type doc promises "Live sessions" and a "decoder registry"; the shipped data is constants. (Matches orchestrator baseline: engine-info reports exactly mjpeg, pcm_s16le.) Defect lives in host.rs; recorded here because the type's doc claims are what users read. Confidence: HIGH — Verification: read (host.rs:1629-1661).
F-005 [S3]
Where: types.rs:446-454 vs crates/playback-engine/src/frame_step.rs:49
Code:
```
pub enum StepDirection {
    /// Step to the next frame.
    Next,
```
What is wrong: the engine re-declares the same concept as its own `FrameStepDirection` (frame_step.rs:49) instead of using/bridging the wire type; no conversion function exists, and the wire `StepDirection`'s command (`playback.step-frame`) is unserved, so the two enums can drift silently. What correct looks like: one enum plus an explicit bridge (the file already does this pattern for `TrackChangeKind` — event.rs:882). Confidence: HIGH — Verification: read.
F-006 [S4]
Where: types.rs:793-801, 815-821, 834-849
Code:
```
        let json = serde_json::to_string(&target).unwrap_or_default();
```
What is wrong: `unwrap_or_default()` converts a serialization failure into `""`, so the assert reports a string diff, not the error. Test remains honest (assert still fails) but diagnostics degrade. Confidence: HIGH — Verification: read.
F-007 [S4]
Where: types.rs:277-278
Code:
```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum VolumeKind {
```
What is wrong: `deny_unknown_fields` on a unit-only enum whose wire form is a bare string (`"fixed"`…) — there are no fields, the attribute is dead weight and implies a closed-object shape that does not exist. Confidence: HIGH — Verification: read (compiles — test run above — but attribute is a no-op for unit variants).
F-008 [S4]
Where: types.rs:200-208
Code:
```
    Relative {
        /// Signed delta in milliseconds.
        delta_ms: i64,
    },
```
What is wrong: `SeekTarget` mixes units inside one enum — `Absolute.time` is microseconds (`Timestamp`), `Relative.delta_ms` is milliseconds. It is doc-faithful (doc 05 §5.1 line 471 uses `deltaMs`) and the field name disambiguates, but any consumer not reading the field name gets a 1000x error. Flag for the dispatch_seek auditors. Confidence: HIGH — Verification: read.

## 4. Per-function table (functions/methods/tests; the other 45 items are data types — see F-002 liveness split)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PlaybackState::can_transition_to | 312-339 | REAL | — | Exact encoding of doc 05 §3.3 (line 412); test-executed |
| tests::seek_target_wire_form_matches_doc_example | 792-801 | REAL | — | Executed, passed |
| tests::duration_is_never_a_sentinel | 803-822 | REAL | — | Executed, passed |
| tests::closed_shapes_reject_unknown_fields | 824-831 | REAL | — | Executed; honest (removing deny_unknown_fields fails it with "missing field" ≠ "unknown field") |
| tests::selector_variants_round_trip | 833-849 | REAL | — | Executed, passed |
| tests::playback_state_transitions_follow_section_3_3 | 851-906 | REAL | — | Executed; 17 legal + 8 illegal + 8 self edges |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| seek_target_wire_form_matches_doc_example | YES | Exact-string wire assertion on tag + `deltaMs` rename |
| duration_is_never_a_sentinel | YES | Exact JSON per variant; a numeric sentinel breaks the string |
| closed_shapes_reject_unknown_fields | YES | Dropping `deny_unknown_fields` changes error to "missing field `codec`" — assertion on "unknown field" fails |
| selector_variants_round_trip | YES | Exact strings `{"kind":"id","id":3}` etc. |
| playback_state_transitions_follow_section_3_3 | YES | Positive+negative edge assertions on the matches! table |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "§3.3's valid-transition table" encoded | types.rs:313-320 vs doc 05 line 412 | REAL | Side-by-side read; all edges match exactly |
| Volume/Rate ranges (0.0..=1.25 / (0.0,16.0]) `[PROPOSAL]` | types.rs:51-57 vs doc 05:186-187 | REAL | Verbatim match; no enforcement anywhere (set-volume/set-rate unserved) |
| "Live sessions" in EngineInfo | types.rs:599 | SHALLOW | host.rs:1659 always `Vec::new()` |
| "decoder registry" | types.rs:597-598 | SHALLOW | host.rs:1643-1658 hard-coded 2-entry literal |
| OpenOptions "schema-validated" per §2.3 | doc 05:227 | SHALLOW | Schema-validated on the wire, but 5 of 10 fields silently dropped by the shipped host (F-001) |

## 7. Dead / duplicated / unreachable code
Catalog-only (never produced in shipped build; reachable only as deserialized-then-refused payloads): StateSnapshot, TrackSelections, PlaylistSummary, SubtitleSettings, AudioDevice, VolumeKind, AspectRatioSpec, CropSpec, StepDirection, RepeatMode, PlaylistSaveFormat, PlaylistEnqueueItem, PlaylistItemId, LogLevel, ConfigAreaInfo, PluginInfo, PluginSource, PluginAction, PlaylistChange, Volume. Duplicated concept: StepDirection vs engine FrameStepDirection (F-005). TrackChangeKind::SecondaryAudio (types.rs:703) has no producer — event.rs:877-880 documents it as a follow-up card.

## 8. Dependencies & flags
serde (1.0.229 per Cargo.lock — supports `rename_all_fields` used at types.rs:191), serde_json (tests), schemars (JsonSchema on every type). No feature gates in this file; catalog is unconditionally compiled into `commands`, a dependency of lumen-cli (apps/cli/Cargo.toml:52).

## 9. Open questions for the next agent
- Does any non-CLI host (control-server? desktop?) serve `system.get-state-snapshot` or populate `EngineInfo.sessions`? (apps/desktop/src had zero matches — presume NO.)
- `host.rs:1311` constructs `MediaInfo` with `chapters: Vec::<Chapter>::new()` — do demuxed chapters ever reach the wire, or is the chapter list always empty?
- VolumeKind: is `devices.volume-added` (event.rs:303 payload) ever emitted? No producer found in playback-engine.
