# AUDIT: crates/commands/src/catalog/spec.rs (part1, lines 1–548 of 1098)
> Chunk scope: part1 = lines 1–548 (module doc, the five metadata types,
> `catalog_data()` header and its first 47 rows, ending exactly at the
> `MediaGetInfo` row). Part2 (lines 549–1098: remaining rows, the event
> table, `catalog_checksum`/`checksum_of`/`command_spec`/`event_spec`,
> the in-file `mod tests`) is W02-A06's assignment — not audited here.
> File name carries `.part1` to avoid colliding with A06's part2 audit.

| Field | Value |
|---|---|
| Auditor | W02-A07 |
| Date | 2026-11-21 |
| Lines / functions | 548 (of 1098) / 11 units + 47 data rows |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI `CliCommand::Commands` → `apps/cli/src/run.rs:823` `Command::SystemListCommands` → `apps/cli/src/dispatch.rs:142-144` → `apps/cli/src/host.rs:1665` `command_listings()` → `commands::catalog::catalog_data()` (this table); same table read by `apps/control-server/src/dispatch.rs:88` and `apps/web/src/host.rs:1432`; generated artifacts via `crates/commands/src/bin/catalog_gen.rs:112` and `catalog/codegen.rs:138`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `sed -n` verbatim-quote verification (5-9, 20-23, 163-165, 182-189, 902-1019 of spec.rs); `grep -c 'id: CommandId::'` → 65 rows, `grep -c 'id: EventId::'` → 26 rows; `rg` callers of `catalog_data`/`catalog_checksum`/`command_listings`/`gen_bumping`/`CommandTarget`/`pipeline_scoped`/`EventSpec` (consumer map below); read doc 05 §1.5, §2.1, §2.3–§2.10, §2.12, §3.2, §4.2, §6.2; `grep` baseline `test-workspace.log.txt` (spec tests executed, lines 1326-1333, 1388); `grep -c 'CompletionClass::A,'/'S,'` → 19/46 |

## 1. One-paragraph verdict
This chunk is the catalog metadata table as data, and the data is honest: all 47 command rows in range were cross-checked field-by-field against doc 05 §2.3–§2.10 and every class/target/gen-bumping/args/result value matches the doc (the one deliberate oddity, `tracks.set-subtitle-track` class S while video/audio are A, is the documented §2.5 rationale: "no A/V pipeline invalidation"). The table is genuinely consumed by the shipped CLI's `system.list-commands` lane and regenerated artifacts, and the baseline workspace test run executed the in-file spec tests against exactly this data (all ok). The defects are documentation drift inside the file's own module header: the header's "test-enforced invariants" paragraph asserts "only `video.set-fullscreen` targets the shell", which the table itself contradicts seven rows later (`SessionSetFrameChannel`, target Shell), and its row counts ("23 rows" events; "61 rows + trio" commands) understate the real 26/65 — the header was never updated when w31.1 and issue #42 additions landed.

## 2. What actually works (verified)
— All 47 rows in range match doc 05 §2.3–§2.10 — verified by field-by-field comparison of each `CommandSpec` (class, target, gen_bumping, args, result) against the doc tables (session §2.3: 2 rows + frame-channel; playback §2.4: 8; tracks §2.5: 6; audio §2.6: 6; video §2.7: 6; subtitles §2.8: 4; playlist §2.9: 13; media §2.10: 1).
— Table integrity executed: baseline run #3 (`audit/runs/baseline/test-workspace.log.txt:1326-1333`) shows `catalog::spec::tests::{class_counts_match_the_doc_05_tables, classification_matches_section_1_5_examples, gen_bumping_set_is_exactly_section_4_2, mutated_table_entry_changes_the_checksum, pipeline_scoped_set_matches_section_3_2, shell_targeted_commands_are_the_shell_owned_surface, tables_cover_every_id_exactly_once_in_order}` all `ok`, and `:1388` `tests/catalog_shape.rs` ran (schema 65/26 counts, enum↔table set equality, checksum recompute — all in the 1681 passed of run #3).
— Reachability: `rg "command_listings"` → `apps/cli/src/dispatch.rs:144` (`commands: crate::host::command_listings()`) inside the `Command::SystemListCommands` arm; `apps/cli/src/run.rs:823` maps a CLI subcommand to `Command::SystemListCommands`. The `lumen` binary can print this table.
— 19 A / 46 S class split: `grep -c` → 19 A / 46 S, matching the test's asserted counts and the doc-derived arithmetic (61 doc rows → 19A/42S, +3 S bookmarks, +1 S frame-channel).

## 3. Findings

### F-001 [S3]
Where: crates/commands/src/catalog/spec.rs:20-23 (contradicted by :182-189 and :1006-1018)
Code:
```
//! Invariants, test-enforced: every id enum variant has exactly one
//! row; rows appear in doc-05 table order; the S/A classification
//! matches §1.5's example lists; the gen-bumping set is exactly §4.2's
//! list; only `video.set-fullscreen` targets the shell.
```
and the row 160 lines below:
```
        CommandSpec {
            id: CommandId::SessionSetFrameChannel,
            class: CompletionClass::S,
            target: CommandTarget::Shell,
```
What is wrong: The header's "test-enforced invariants" list claims "only `video.set-fullscreen` targets the shell". The table itself (line 185) and the enforcing test `shell_targeted_commands_are_the_shell_owned_surface` (spec.rs:1006-1018, asserts `[SessionSetFrameChannel, VideoSetFullscreen]`) both say two commands target the shell. The header was not updated when w31.1 added the frame-channel bind.
Why it matters: The header is the contract summary a maintainer reads first; it asserts an invariant ("only") that the code and its own test contradict. Anyone reasoning from the header (e.g. writing a host that special-cases exactly one shell command) builds on a false statement.
What correct looks like: Header lists both shell-targeted commands (`video.set-fullscreen`, `session.set-frame-channel`) and notes the w31.1 addition, mirroring the test at :1006-1018.
Confidence: HIGH — Verification: read (sed -n '20,23p', '182,189p', Read of :1006-1018) + baseline test run proves the test enforces the two-command set.

### F-002 [S4]
Where: crates/commands/src/catalog/spec.rs:5-8
Code:
```
//! [`catalog_data()`] is the single table the generators and
//! `system.list-commands` consume: one [`CommandSpec`] per §2.1 id (61
//! rows + the issue #32 bookmark trio), one [`EventSpec`] per §3.2 id
//! (23 rows). The rows carry the
```
What is wrong: Two stale counts in one sentence. (a) Events: the file's `event_catalog_data()` has **26** rows (`grep -c 'id: EventId::'` → 26), not 23; the extra three (`video.backend-changed`, `devices.volume-added`, `devices.volume-removed`, issue #42) exist in no doc 05 table — doc 05 §3.2 lists exactly 23. (b) Commands: "61 rows + the issue #32 bookmark trio" sums to 64, but the table has 65 rows; the w31.1 frame-channel bind is omitted here even though the comment at :161-162 counts it ("61 rows + the issue #32 bookmark trio + the w31.1 frame-channel bind").
Why it matters: The header claims a §3.2-only event table; three shipped events are undocumented in doc 05 (only traceable via `worklog.md:683`), so "one EventSpec per §3.2 id" is false as written and the doc-side catalog (§3.2) silently rots. The internal 64-vs-65 inconsistency between :5-8 and :161-162 shows the two comments were updated at different times.
What correct looks like: Header: "65 command rows (§2.1's 61 + issue #32 bookmark trio + the w31.1 frame-channel bind), 26 event rows (§3.2's 23 + issue #42 trio — doc 05 §3.2 update pending)" — and doc 05 §3.2 gets the three missing rows.
Confidence: HIGH — Verification: read + `grep -c` counts + doc 05 §3.2 read (23 rows listed) + `rg "backend-changed|volume-added" docs/` (only wave logs, no doc 05 hit). Note: `tests/catalog_shape.rs:6-7` cites "doc 05 §6.2" as the trio's documentation, but §6.2 is the additive-change policy and contains no event list — mis-citation in the test's comment (A-scope: that test file, flagged here for the record).

### F-003 [S4]
Where: crates/commands/src/catalog/spec.rs:139 and 84-89 (field with no runtime consumer)
Code:
```
pub enum CommandTarget {
    /// The engine core (default).
    Engine,
    /// The shell/host (v1: only `video.set-fullscreen`).
    Shell,
}
```
(and `pub target: CommandTarget,` at :139)
What is wrong: Two-part staleness/unused-surface: (a) the `Shell` variant's own doc comment still says "v1: only `video.set-fullscreen`" — false since w31.1 (same defect class as F-001, on the type instead of the header); (b) `CommandSpec.target` has **zero runtime consumers**: `rg "spec.target|\.target"` over `crates/commands/`, `apps/` finds no reader — the class field is read on the wire (`spec.class` at apps/cli/src/host.rs:1670, apps/web/src/host.rs:751, apps/control-server/src/dispatch.rs:90) and `gen_bumping` is cross-checked by `crates/playback-engine/tests/table.rs:663-675`, but `target` feeds only the checksum and the spec tests. Actual shell routing is hardcoded per host: `apps/desktop/src/shell.rs:160` intercepts by variant match (`if let Command::SessionSetFrameChannel { channel } = &request.command`), not by consulting the catalog.
Why it matters: Doc 05 §1.1.3 says the target is declared so keymaps/menus/CLI/control-server "address them uniformly"; nothing enforces that. A host that routes a shell-targeted command to the engine (or forgets to intercept a new one) passes every committed test — the catalog's declared routing truth is unverifiable aspiration, not machine-checked fact.
What correct looks like: Either a test that cross-checks each shell-targeted catalog row against the hosts' interception sets, or wire exposure (§2.12's `CommandListing` has no `target` field either), or an honest comment that `target` is checksum/documentation metadata only.
Confidence: HIGH — Verification: read + exhaustive rg for consumers (none found); shell.rs:150-163 read.

### F-004 [S4]
Where: crates/commands/src/catalog/spec.rs:403-410 (vs doc 05 §2.8 and catalog/command.rs:254-260)
Code:
```
        CommandSpec {
            id: CommandId::SubtitleAddFile,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("path: String, encoding: Option<String>, select: bool"),
```
What is wrong: The `args` descriptor says `path: String`, but doc 05 §2.8's Params column says `path: PathSpec` (a host-variant type: file path on desktop, transferred Blob on web). `PayloadDesc`'s own doc (spec.rs:142-143) claims to be "the §2 tables' *Params* column, normalized".
Why it matters: Minor, but this descriptor is part of the committed checksum artifact, so the normalization silently canonicalizes a doc-vs-code divergence instead of surfacing it: the code (`catalog/command.rs:260` `path: String`, with a `[VERIFY]` comment admitting PathSpec "is a plain path string here") drifted from the doc, and the descriptor papered over it.
What correct looks like: Descriptor `"path: PathSpec, …"` matching the doc, or a doc 05 §2.8 correction — one of the two sources of truth moves deliberately.
Confidence: HIGH — Verification: read of spec.rs:403-410, doc 05 §2.8, command.rs:254-260.

## 4. Per-function table
| Function / unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `CompletionClass` enum (+serde/schemars derives) | 28-50 | REAL | — | Two variants S/A; wire letters via `rename_all = "lowercase"`; consumed on the wire (`spec.class` → CommandListing). Derives verified by baseline schema/codegen tests. |
| `CompletionClass::as_str` | 52-61 | REAL | — | Returns "s"/"a"; matches §2.12 listing shape. |
| `impl Display for CompletionClass` | 63-67 | REAL | — | Delegates to as_str; no allocation. |
| `CommandTarget` enum | 72-89 | REAL | S4 | F-003: stale variant doc ("only video.set-fullscreen"), no runtime consumer of the field. |
| `CommandTarget::as_str` | 91-99 | REAL | — | "engine"/"shell". |
| `impl Display for CommandTarget` | 101-105 | REAL | — | Delegates to as_str. |
| `PayloadDesc` enum | 111-119 | REAL | — | None / Type(&'static str) — part of checksum input (checksum_of is part2). |
| `PayloadDesc::as_str` | 121-129 | REAL | — | "none" / the type string. |
| `CommandSpec` struct | 132-147 | REAL | — | 6 fields; `gen_bumping` cross-checked by playback-engine tests; `target` see F-003. |
| `EventSpec` struct | 150-159 | REAL | — | `pipeline_scoped`/`rate` are checksum+test metadata only (by design — engine-side scoping uses `EventKind::is_pipeline_scoped()`, playback-engine/event.rs:625). |
| `catalog_data()` (header + first 47 rows) | 163-548 | REAL | — | `pub const fn`, returns &'static slice; order enforced == `CommandId::ALL` by in-file test (baseline ok). |
| Rows: session §2.3 — `SessionOpen` 166-173, `SessionClose` 174-181 | 166-181 | REAL | — | Match doc §2.3 exactly (open: A/accepted, no gen bump — gen resets to 0 on new session; close: S/Unit, gen-bumping terminal). |
| Row: `SessionSetFrameChannel` | 182-189 | REAL | — | w31.1 addition, NOT in doc 05 §2.1 (acknowledged at :161-162); payload `channel: u32` matches `command.rs:96-101`; class S / target Shell / Unit matches the shell's actual answer (`apps/desktop/src/shell.rs:186-190` returns `Completed(Unit)`). |
| Rows: playback §2.4 — `PlaybackPlay` 191-198, `PlaybackPause` 199-206, `PlaybackTogglePlayPause` 207-214, `PlaybackStop` 215-222, `PlaybackSeek` 223-230, `PlaybackSetRate` 231-238, `PlaybackStepFrame` 239-246, `PlaybackSetPosition` 247-254 | 191-254 | REAL | — | All 8 match doc §2.4 field-for-field (A except set-rate S; gen-bumping on stop/seek/step-frame/set-position per §4.2). |
| Rows: tracks §2.5 — `TracksList` 256-263, `TracksGetInfo` 264-271, `TracksSetVideoTrack` 272-279, `TracksSetAudioTrack` 280-287, `TracksSetSubtitleTrack` 288-295, `TracksSetSecondaryAudio` 296-303 | 256-303 | REAL | — | Match doc §2.5; the S(subtitle)/A(video,audio) asymmetry is documented doc rationale, not a bug. gen-bumping on video/audio only ✓. |
| Rows: audio §2.6 — `AudioSetVolume` 305-312, `AudioGetVolume` 313-320, `AudioSetMute` 321-328, `AudioListDevices` 329-336, `AudioSetDevice` 337-344, `AudioSetDelay` 345-352 | 305-352 | REAL | — | Match doc §2.6 (set-device A, no gen bump — "pipeline continues" ✓). |
| Rows: video §2.7 — `VideoSetFullscreen` 354-361, `VideoSetAspectRatio` 362-369, `VideoSetCrop` 370-377, `VideoSetZoom` 378-385, `VideoSetDelay` 386-393, `VideoTakeScreenshot` 394-401 | 354-401 | REAL | — | Match doc §2.7; fullscreen S + target shell ✓; take-screenshot A/accepted ✓. |
| Rows: subtitles §2.8 — `SubtitleAddFile` 403-410, `SubtitleSetSync` 411-418, `SubtitleSetEncoding` 419-426, `SubtitleSetScale` 427-434 | 403-434 | REAL | S4 | All match doc §2.8 except F-004 (path String vs PathSpec descriptor). |
| Rows: playlist §2.9 — `PlaylistEnqueue` 436-443, `PlaylistEnqueueMany` 444-451, `PlaylistPlayItem` 452-459, `PlaylistNext` 460-467, `PlaylistPrev` 468-475, `PlaylistSkip` 476-483, `PlaylistRemove` 484-491, `PlaylistMove` 492-499, `PlaylistClear` 500-507, `PlaylistSetShuffle` 508-515, `PlaylistSetRepeat` 516-523, `PlaylistSave` 524-531, `PlaylistLoad` 532-539 | 436-539 | REAL | — | All 13 match doc §2.9 (model edits S, navigation A, no gen bumps — correct: §4.2 lists none of them). |
| Row: media §2.10 — `MediaGetInfo` | 541-548 | REAL | — | Matches doc §2.10 (S / none / MediaInfo). Chunk ends exactly at this row's closing brace. |

## 5. Test honesty
Tests covering this chunk live in part2's line range (`mod tests`, spec.rs:902-1097) and `tests/catalog_shape.rs` — evaluated here because they pin this chunk's data:
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `tables_cover_every_id_exactly_once_in_order` (spec.rs:907-925) | YES | Zip against `CommandId::ALL` + BTreeSet duplicate check; flipping a row's id or reordering fails. Mutation check: swap two rows → zip assert fails. |
| `classification_matches_section_1_5_examples` (spec.rs:930-975) | YES | Hardcoded §1.5 example lists; flipping any listed row's class fails. Does NOT cover non-example rows (e.g. `TracksSetSecondaryAudio` class is unpinned by this test — but `class_counts_match_the_doc_05_tables` pins the aggregate 19/46, so a single flip is caught). |
| `gen_bumping_set_is_exactly_section_4_2` (spec.rs:978-1001) | YES | Set equality both directions; flipping any gen_bumping flag in range fails. |
| `shell_targeted_commands_are_the_shell_owned_surface` (spec.rs:1005-1019) | YES | Exact vec equality `[SessionSetFrameChannel, VideoSetFullscreen]`. |
| `class_counts_match_the_doc_05_tables` (spec.rs:1025-1038) | YES | Pins 19/46/65. |
| `catalog_shape.rs` (schema counts 65/26, enum↔table↔schema set equality, checksum recompute) | YES | Mutation of any row's id breaks set equality; class mutation breaks counts; descriptor mutation breaks the committed checksum file match. |
All executed in baseline run #3 (`test-workspace.log.txt:1326-1333, 1388` — `ok`). These are real assertions on real data; no tautologies, no mocks-of-subject found.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "one CommandSpec per §2.1 id (61 rows + the issue #32 bookmark trio)" | spec.rs:6-7 | FALSE (stale count) | 65 rows exist; frame-channel bind omitted from this sentence (F-002). |
| "one EventSpec per §3.2 id (23 rows)" | spec.rs:7-8 | FALSE (stale count) | 26 rows; 3 events absent from doc 05 §3.2 (F-002). |
| "only `video.set-fullscreen` targets the shell" (test-enforced invariant) | spec.rs:22-23 | FALSE | Row :182-189 + test :1006-1018 assert two shell commands (F-001). |
| "`catalog_data()` is the single table the generators and `system.list-commands` consume" | spec.rs:5-6 | REAL | rg: host.rs:1665/cli, host.rs:1432/web, dispatch.rs:88/control-server, catalog_gen.rs:112, codegen.rs:138. |
| §2.3–§2.10 per-command class/params/results (47 rows in range) | doc 05 §2.3-§2.10 | REAL | Field-by-field comparison, all rows match (§4 table). |
| "args: the §2 tables' Params column, normalized" | spec.rs:142-143 | SHALLOW | One known non-normalization: SubtitleAddFile `path: String` vs §2.8 `PathSpec` (F-004). |
| "session.set-frame-channel (w31.1, class S, shell-targeted)" | spec.rs:161-162, command.rs:92-95 | REAL-UNPROVEN | Catalog row + shell interception code exist (shell.rs:159-190); no baseline run exercises the frame-channel bind end-to-end (tauri build UNVERIFIED-BUILD, baseline #2). |

## 7. Dead / duplicated / unreachable code
- No dead code in this chunk; every type is used (enums on the wire, rows by tests/generators).
- `CommandSpec.target` is data with no runtime consumer (F-003) — not dead (checksum + tests read it), but its declared routing role is unenforced.
- `EventSpec.rate` / `pipeline_scoped`: metadata-only by design (engine uses `EventKind::is_pipeline_scoped()` instead); consistent with the file's "catalog as data" purpose.
- Duplication: the 47 rows duplicate information that also lives in `catalog/command.rs` payload types and doc 05 §2 — this duplication is the file's explicit purpose (checksum drift detection), mitigated by the shape tests.

## 8. Dependencies & flags
- Uses `serde`/`schemars` derives on `CompletionClass`/`CommandTarget` (schema generation), `crate::catalog::ids::{CommandId, EventId}`. No feature gates on this chunk; only `#[cfg(test)]` at :902 (part2). Crate lints: workspace-inherited (baseline hygiene counts).
- No `unsafe`, no panics, no arithmetic in this chunk — pure const data + trivial methods. Nothing to check for overflow/endianness.

## 9. Open questions for the next agent
- A06 (part2): verify the FNV-1a `checksum_of` implementation against the claimed reference vectors (`fnv1a64_reference_vectors` test) and whether `command_spec`/`event_spec` linear scans are hot-path (they are O(n) lookups — fine for 65 rows).
- A06: the event table rows (701-880) — verify each of the 26 `EventSpec` rows against §3.2's rate column, and confirm the 13 pipeline-scoped set.
- Doc-wave agent: doc 05 §3.2 is missing 3 shipped events and §2.1 is missing `session.set-frame-channel` + the bookmark trio (61→65); `tests/catalog_shape.rs:6-7` mis-cites §6.2 as their documentation.
- Host-wave agent: nothing enforces that shell-targeted catalog rows correspond to actual host interceptions (F-003) — a cross-check test is the fix.
