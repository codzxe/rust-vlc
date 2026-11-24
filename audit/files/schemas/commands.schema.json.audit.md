# AUDIT: schemas/commands.schema.json
| Field | Value |
|---|---|
| Auditor | W11-A01 |
| Date | 2026-11-23 |
| Lines / definitions | 4873 (manifest says 4874 loc) / 58 `$defs` = 3 envelopes + 65 command variants + 26 event variants + 32 result variants + 24 payload types/enums |
| File verdict | REAL (machine contract: generated, drift-gated, byte-verified by executed test) — with FALSE prose claims embedded in descriptions (F-001, F-002, F-004) |
| Reachable from shipped entry point? | Build/CI artifact: YES (ci.yml:280 `codegen --check`; crates/commands/tests/catalog_drift.rs byte-compares it; feeds ui/src/generated/commands.ts). Runtime: NO — no shipped binary loads it (no `include_str!`/fs read of the artifact outside codegen/tests; control server validates by key-set + serde only) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 2 |
| Commands run | python3 schema-inventory (exit 0); python3 schema-vs-rust id diff (exit 0, delta empty); `cargo test -p commands` 57/57 (exit 0); `cargo test -p commands --test catalog_drift --test catalog_shape --test catalog_emission_ledger --test wire_forms` 15/15 (exit 0); 3 probe runs of `target/debug/lumen --control-stdio` (exit 0/64); rg/cmp searches (exit 0) |

## 1. One-paragraph verdict
The hostile headline question — "does the generated schema match the actual command catalog?" — answers YES, provably: I executed `cargo test -p commands --test catalog_drift` and `committed_artifacts_match_regeneration ... ok` passed, which byte-compares this file against regeneration from the live Rust types; my own python diff of all `cmd`/`kind` consts against `crates/commands/src/catalog` renames is empty in both directions (65/65 commands, 26/26 events, 32/32 result kinds — exactly Wave 2's counts); and my random-10 spot-check found zero field/required/type divergences. The file is honest as a *machine* contract. What is dishonest is its embedded *prose*: 11 of 26 event entries describe events no production code emits (the repo's own ledger admits 12; it even miscounts `tracks.changed`, which the engine does emit), one event description asserts an emission pipeline that does not exist (`devices.volume-added`), the `invalid-argument` description claims unknown fields are rejected while my executed probes show junk inside `args` and junk envelope fields are silently dropped, and the schema simultaneously *under*-constrains (it validates `{"cmd":"playback.pause","args":{}}`, which the engine rejects with exit 64). Nobody validates against this schema at runtime — it is a CI-gated artifact whose only machine consumers are the drift tests and the TS emitter.

## 2. What actually works (verified)
— verified by `cargo test -p commands --test catalog_drift` -> `test committed_artifacts_match_regeneration ... ok` / `test result: ok. 2 passed` (committed file is byte-identical to `schema_for!(CatalogDocument)` regeneration from current source).
— verified by `cargo test -p commands` (57 unit) + catalog_shape 4/4 + catalog_emission_ledger 2/2 + wire_forms 7/7 -> all ok, including `schema_has_the_doc_05_structure_and_counts` and `spec_table_covers_exactly_the_enum_variants` (schema == CommandId/EventId enums == catalog_data() spec rows).
— verified by python3 set-diff of the 65 `cmd` consts (schema `CommandRequest.oneOf`) vs the 65 `#[serde(rename)]` tags in crates/commands/src/catalog/command.rs and 26 `kind` consts vs event.rs -> `CMD delta schema-rust: [] []` / `EV delta schema-rust: [] []`.
— verified by probe, `./target/debug/lumen --control-stdio`: unknown-field behavior reproduced (see F-002).

## 3. Findings

F-001 [S2]
Where: schemas/commands.schema.json:3010-3012 (`session.opening-progress`), 3258-3260 (`playback.rate-changed`), 3313-3315 (`video.screenshot-saved`), 3368-3370 (`playlist.changed`), 3386-3388 (`config.setting-changed`), 3419-3421 (`plugins.loaded`), 3447-3449 (`plugins.failed`), 3470-3472 (`plugins.crashed`), 3498-3500 (`system.log`), 3544-3546 (`devices.volume-added`), 3572-3574 (`devices.volume-removed`)
Code:
```
        {
          "description": "`session.opening-progress` — network opens, best-effort, ≤ 1 Hz;\nabsent for local files.",
```
```
          "description": "`devices.volume-added` — a local volume or removable medium\nappeared (card 7.D1; the discovery crate's watcher emits it\nfrom real mount-table diffs, never fabricated). Not\npipeline-scoped.",
```
What is wrong: 11 of the 26 event entries describe events that no production code ever constructs (dead schema entries): `session.opening-progress`, `playback.rate-changed`, `video.screenshot-saved`, `playlist.changed`, `config.setting-changed`, `plugins.loaded`, `plugins.failed`, `plugins.crashed`, `system.log`, `devices.volume-added`, `devices.volume-removed`. Proof: the repo's own ledger (crates/commands/tests/catalog_emission_ledger.rs:26-27 — "Arithmetic (post-3.A1, post-3.B4, post-7.C1): **14 emitted / 12 later**") plus my rg: every construction site for these variants is an id-map function or `#[cfg(test)]` fixture (e.g. crates/playback-engine/src/events.rs:816 is `wire_event_id`, :2052 is a test sample list). The `devices.volume-added` description is worse than dead — it asserts "the discovery crate's watcher emits it", but crates/device-discovery emits its own internal `VolumeEvent` (crates/device-discovery/src/events.rs:30-33) and no code anywhere maps it to `EventKind::DevicesVolumeAdded`; playback-engine's own table (events.rs:106) says "wired-by-later … the engine-side relay joins with host wiring" and playback-engine does not even depend on device-discovery. `system.log` carries invented live semantics ("Rate-limited per target (default 100/s then an aggregate \"N suppressed\")") for an event with, in the ledger's words, "no logging bridge". Note the ledger itself miscounts: it files `tracks.changed` under "later" (ledger :35-40 records the divergence) while the engine really does construct it (crates/playback-engine/src/session.rs:1659 -> event.rs:585), so the true dead set is 11, not 12. Why it matters: this file is advertised (root description, L4) as "the single machine description of the catalog"; a UI author reading it will wire panels/handlers for events that can never arrive, with fabricated rate-limit behavior stated as fact. What correct looks like: carry the emitted/wired-by-later partition into the schema descriptions (the data already exists in the ledger), or emit the events. Confidence: HIGH — Verification: ran (ledger partition test passed) + read (rg construction sites) .

F-002 [S2]
Where: schemas/commands.schema.json:2877-2881 (prose), 349-357 + 453-465 (machine constraints)
Code:
```
          "description": "Unknown field, unknown enum variant, unknown command id (V1).",
          "type": "string",
          "const": "invalid-argument"
```
```
      "type": "object",
      "properties": {
        "requestId": {
          "$ref": "#/$defs/RequestId"
        }
      },
      "oneOf": [
```
What is wrong: two opposite dishonesties about unknown fields. (a) The `invalid-argument` description promises "Unknown field … (V1)" rejection, but neither the schema's machine constraints nor the runtime delivers it: `CommandRequest` and every `oneOf` variant (and every inline `args` object) lack `additionalProperties: false` (python3 inventory: 0 of 65 variants carry it), and my executed probes against the shipped CLI show silent acceptance — `{"requestId":1,"cmd":"playback.seek","args":{"target":{...},"junk":true}}` answered `not-ready` (same as the clean request) and `{"requestId":4,"cmd":"playback.pause","evilField":1}` answered `not-ready`; the junk was silently dropped, never `invalid-argument`. This reproduces Wave 2's finding (audit/files/crates/commands/src/catalog/command.rs.audit.md F-001) from the schema side: the prose over-claims what its own machine constraints do not enforce. (b) The converse defect: because the `playback.pause` variant (L453-465) has no `additionalProperties: false` and `required: ["cmd"]` only, `{"requestId":3,"cmd":"playback.pause","args":{}}` is VALID against this schema — but the engine rejects it (`[control.invalid-argument] invalid type: map, expected unit variant Command::PlaybackPause`, exit 64, my probe). A client that validates requests against this artifact will pass objects the engine refuses. Why it matters: the file is the published contract for machine-mode clients; both directions of divergence produce integration failures that the schema was supposed to prevent. What correct looks like: `additionalProperties: false` on the variants with `args` presence/absence pinned per variant (schemars emits this only if the enum can express it — it cannot today, so the fix is a hand-written envelope schema or a serde post-parse key check), and the `invalid-argument` description scoped to where rejection is real (payload structs, command ids, enum variants). Confidence: HIGH — Verification: ran (three probes, output pasted above) + read.

F-003 [S3]
Where: schemas/commands.schema.json:579, 854, 952, 1123, 1189-1191
Code:
```
              "description": "`0.0..=1.0` of the duration.",
```
```
          "description": "`video.set-zoom` — display scaling, `0.25..=4.0` (class S).",
```
What is wrong: the schema's own descriptions state numeric contracts (`0.0..=1.0` fraction, `±60 000` delayMs, `0.25..=4.0` zoom, `0.2..=5.0` subtitle scale, enqueue-many "non-empty; empty is `invalid-argument`") that its machine constraints do not encode — every one of these is emitted as bare `"type": "number", "format": "float"` or an unbounded integer/array with no `minimum`/`maximum`/`minItems`. Wave 2's probe (command.rs.audit.md F-004) already showed `fraction: 7.5` and `delayMs: -999999999` parse clean; the schema likewise passes them, so the "single machine description" understates nothing and overstates nothing — it simply fails to machine-state what its own prose declares. Blast radius today is limited because no shipped host serves these commands (typed refusals), which is why this is S3 and not S2. What correct looks like: newtype ranges with schemars range attributes so the emitted schema carries `minimum`/`maximum`. Confidence: HIGH — Verification: read (python3 scan of all 65 variants: zero min/max constraints on scalar args) + prior-wave probe cross-reference.

F-004 [S3]
Where: schemas/commands.schema.json:873-874
Code:
```
          "description": "`video.set-fullscreen` — **target: shell** (class S; the only\nshell-routed command in v1, §1.1.3).",
```
What is wrong: no shell routes it. `rg "VideoSetFullscreen"` across crates/apps/ui/src hits only the catalog's own three files (spec.rs, command.rs, ids.rs); the desktop shell intercepts exactly one command and it is `session.set-frame-channel` (apps/desktop/src/shell.rs:160-162, verified by sed), not `video.set-fullscreen`. The schema asserts routing behavior ("target: shell", "the only shell-routed command in v1") that the shipped system does not implement — the same FALSE claim Wave 2 found in command.rs's doc comments, here fossilized into the published contract. What correct looks like: describe what exists (typed `unsupported-for-host`/`not-implemented` refusal) or implement the shell routing. Confidence: HIGH — Verification: read (rg both directions; shell.rs intercept verified).

F-005 [S4]
Where: schemas/commands.schema.json:4515-4519
Code:
```
        "scale": {
          "description": "Render size multiplier.",
          "type": "number",
          "format": "float",
          "default": 0.0
        },
```
What is wrong: the snapshot's `SubtitleSettings.scale` defaults to `0.0`, which contradicts the schema's own documented domain for subtitle scale ("`0.2..=5.0`", L1123) — the default value is outside the range the same file declares. Generated from `#[serde(default)]` on a plain `f32` (crates/commands/src/catalog/types.rs:741-749), i.e. a Rust-side default bug surfaced verbatim into the contract. Today harmless: no production code constructs `StateSnapshot`'s subtitle block. Confidence: HIGH — Verification: read (both lines quoted; types.rs read).

F-006 [S4]
Where: schemas/commands.schema.json:2947, 350
Code:
```
      "description": "One engine → host event (doc 05 §1.6/§3.1). Generic over the event\npayload `E` until the catalog lands. `seq` is engine-global and\nstrictly increasing; …",
```
What is wrong: stale wording — "until the catalog lands": the catalog has landed; this document IS the catalog's event description (the def instantiates `Event<EventKind>` via codegen.rs:43,56). Same fossil in `CommandRequest` ("Generic over the command payload `C`"). Leaked envelope doc-comments that describe an intermediate design stage, shipped in the published artifact. Confidence: HIGH — Verification: read.

## 4. Per-definition table (58 `$defs`, all verified against the Rust source by the executed drift test; grouped where structurally identical)
| Definition | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AspectRatioSpec | 26-70 | REAL | — | enum oneOf; matches types.rs |
| AudioDevice | 71-94 | REAL | — | additionalProperties:false honest (deny_unknown_fields types.rs:266) |
| AudioTrackInfo | 95-137 | REAL | — | channels required, rest nullable — matches |
| Bookmark | 138-165 | REAL | — | id+time required; matches issue #32 trio |
| Chapter | 166-198 | REAL | — | |
| CodecDescriptor | 199-233 | REAL | — | all-optional; carries honest `[VERIFY]` marker |
| CommandError | 234-260 | REAL | — | details is open string map (accurate) |
| CommandListing | 261-299 | REAL | — | deny_unknown_fields both sides |
| CommandOutcome | 300-348 | REAL | — | only def family where variants DO carry additionalProperties:false (L329,342) — honest for engine-serialized output |
| CommandRequest | 349-1888 | REAL | S2 | 65 variants, all id/field-verified; permissiveness defects F-002 |
| CommandResponse | 1889-1918 | REAL | — | result -> ResultWire matches runtime adapter |
| CommandResult | 1919-2562 | REAL | — | 32 kinds; Wave 2: 19 never constructed (served-surface gap, not schema drift) |
| CompletionClass | 2563-2577 | REAL | — | "s"/"a" |
| ConfigAreaInfo | 2578-2608 | REAL | — | |
| CropSpec | 2609-2692 | REAL | — | rect x/y int64, w/h uint32 — matches |
| DecoderInfo | 2693-2728 | REAL | — | |
| Duration | 2729-2777 | REAL | — | sentinel-free; matches types test |
| EngineInfo | 2778-2838 | REAL | — | |
| ErrorCode | 2839-2933 | REAL | S2 | 18 codes; invalid-argument prose FALSE (F-002) |
| ErrorEnvelope | 2934-2945 | REAL | — | |
| Event | 2946-3594 | REAL | S2 | 26 variants; 11 dead entries (F-001); stale doc (F-006); no additionalProperties:false (host-side drop, mirrors serde) |
| Generation / RequestId / SessionId / PlaylistItemId / TrackId / Timestamp | 3595-3600, 4156-4161, 4271-4276, 3994-3999, 4578-4583, 4543-4547 | REAL | — | bare integer newtypes |
| LogLevel | 3601-3630 | REAL | — | 5 levels |
| MediaInfo | 3631-3697 | REAL | — | |
| OpenOptions | 3698-3781 | REAL | — | additionalProperties:false + deny_unknown_fields (types.rs:241) — the one place "unknown option is invalid-argument" is TRUE |
| PlaybackState | 3782-3826 | REAL | — | 8 states, matches §3.3 test |
| PlaylistChange | 3827-3972 | REAL | — | 7 mutations |
| PlaylistEnqueueItem / PlaylistSaveFormat / PlaylistSummary | 3973-3993, 4000-4019, 4020-4055 | REAL | — | |
| PluginAction / PluginInfo / PluginSource | 4056-4070, 4071-4120, 4121-4135 | REAL | — | pluginKind rename self-consistent with plugins.loaded |
| RepeatMode | 4136-4155 | REAL | — | off/one/all |
| ResultWire | 4162-4174 | REAL | — | anyOf outcome/error — matches envelope |
| SeekKind / SeekTarget / StepDirection | 4175-4189, 4190-4270, 4482-4496 | REAL | — | SeekTarget 4 variants match §2.2 test |
| SessionSummary / SettingValue / SourceSpec / Stage | 4277-4300, 4301-4325, 4326-4366, 4367-4385 | REAL | — | Stage enum 13 kebab values matches error.rs test |
| StateSnapshot | 4386-4481 | REAL-UNPROVEN | S4 | wire shape real + drift-gated; no production constructor found (Wave 2: never-constructed result variants) |
| SubtitleSettings | 4497-4523 | REAL | S4 | scale default 0.0 out of own documented range (F-005) |
| TimeRange | 4524-4542 | REAL | — | |
| TrackChangeKind / TrackInfo / TrackKind / TrackOrigin / TrackSelections / TrackSelector | 4548-4577, 4584-4658, 4659-4683, 4684-4698, 4699-4753, 4754-4802 | REAL | — | |
| VideoTrackInfo | 4803-4847 | REAL | — | |
| VolumeKind | 4848-4873 | REAL | — | deny_unknown_fields on unit enum is a no-op (types.rs:278; prior wave) |

## 5. Spot-check: 10 random commands, schema vs crates/commands/src/catalog/command.rs (seeded sample, all MATCH)
| Command | Schema (args/required) | Rust | Verdict |
|---|---|---|---|
| session.close | L399-411: no args, required [cmd] | L85-88 `SessionClose` unit | MATCH |
| playback.pause | L453-465: no args, required [cmd] | L107-109 `PlaybackPause` unit | MATCH |
| audio.get-volume | L768-780: no args | L186-188 unit | MATCH |
| audio.set-mute | L781-806: args {muted: boolean}, required [muted] | L189-193 `muted: bool` | MATCH |
| media.get-chapters | L1521-1533: no args | L377-379 unit | MATCH |
| media.add-bookmark | L1575-1607: args {label: string\|null, time: Timestamp}, required [time] | L390-398 `time: Timestamp, label: Option<String>` | MATCH |
| playlist.enqueue | L1149-1181: args {source: SourceSpec, title: string\|null}, required [source] | L288-295 | MATCH |
| playlist.remove | L1290-1315: args {id: PlaylistItemId}, required [id] | L320-326 | MATCH |
| playlist.set-repeat | L1388-1413: args {mode: RepeatMode}, required [mode] | L344-349 | MATCH |
| system.plugin-disable | L1799-1824: args {id: string}, required [id] | L450-455 `id: String` | MATCH |
Plus exhaustive name-set equality (python3): 65/65 commands, 26/26 events, 0 delta both directions.

## 6. Test honesty (tests touching this artifact; all executed by me, all passed)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| catalog_drift::committed_artifacts_match_regeneration | YES | byte-for-byte compare of the committed file vs regeneration; first-diff reported with line numbers |
| catalog_drift::regeneration_is_deterministic | YES | precondition for the above |
| catalog_shape::schema_has_the_doc_05_structure_and_counts | YES | parses THIS file, pins 65/26 counts and wire tags |
| catalog_shape::spec_table_covers_exactly_the_enum_variants | YES | three-surface set equality (schema tags vs id enums vs spec rows) |
| catalog_shape::committed_checksum_recomputes | YES | fnv1a64 over catalog text must equal committed `catalog-fnv1a64: debb80f9922a192a` |
| catalog_emission_ledger::the_ledger_partitions_the_catalog_exactly | YES (but only vs its own buckets) | partitions 26 ids; cannot catch an event that is dead in BOTH ledger and engine (F-001 persisted through it) |
| codegen::check_reports_doctored_artifacts | YES | doctored-byte/missing-file negative controls |

## 7. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the single machine description of the catalog" | schema L4 / codegen.rs doc | REAL | drift test executed; CI gate ci.yml:280 |
| "Unknown field … `invalid-argument`" (V1) | schema L2878 (ErrorCode) | FALSE at envelope/args level | my probes: junk in args -> not-ready; evilField -> not-ready (silently dropped); only payload structs with deny_unknown_fields reject |
| "an unknown option is `invalid-argument`" (OpenOptions) | schema L3699 | REAL | deny_unknown_fields types.rs:241 + additionalProperties:false L3780 |
| "the discovery crate's watcher emits it" (devices.volume-added) | schema L3545-3546 | FALSE | device-discovery emits internal VolumeEvent only; no relay; playback-engine has no dep on device-discovery; engine table events.rs:106 says wired-by-later |
| "`video.set-fullscreen` … the only shell-routed command in v1" | schema L874 | FALSE | rg: no handler anywhere; desktop shell intercepts only set-frame-channel (shell.rs:160-162) |
| ranges `0.0..=1.0` / `0.25..=4.0` / `±60 000` / `0.2..=5.0` | schema L579/952/854/1123 | UNENFORCED | no min/max anywhere in the file (python3 scan) |
| 65 commands / 26 events | checksum file + tests | REAL | recount executed (65/26/32) |

## 8. Dead / duplicated / unreachable code
- 11 dead Event entries (F-001): opening-progress, rate-changed, screenshot-saved, playlist.changed, config.setting-changed, plugins.loaded/failed/crashed, system.log, devices.volume-added/removed.
- Single copy of the artifact (no duplicate under crates/commands/ — earlier suspicion disproved; `cmp` target didn't exist).
- Runtime-unreachable: nothing loads this file at runtime; consumers are CI (ci.yml:280), 3 test files, and the TS emitter output (ui/src/generated/commands.ts, committed, 14,023 B).

## 9. Open questions for the next agent
- Will any host ever validate against this schema? If yes, F-002(b) becomes S1 (schema-valid requests rejected by the engine). The control-protocol doc (lib.rs:30) already falsely implies it does.
- Who owns fixing the emission-ledger `tracks.changed` divergence (ledger :35-40)? The schema is correct; the ledger row is stale relative to engine code.
- `StateSnapshot`/`SubtitleSettings` (F-005): when a snapshot producer lands, scale=0.0 default will ship out-of-range values to renderers.
