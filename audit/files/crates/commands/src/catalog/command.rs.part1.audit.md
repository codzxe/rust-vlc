# AUDIT: crates/commands/src/catalog/command.rs#part1 (lines 1–435)
| Field | Value |
|---|---|
| Auditor | W02-A03 |
| Date | 2026-11-21 |
| Lines / functions | 435 (chunk 1–435 of 871) / 58 enum variants + module docs (system section 437–470 is part2/A01) |
| File verdict | REAL (as wire vocabulary; 49/65 variants have no executor in any host — typed refusals, see F-001) |
| Reachable from shipped entry point? | YES (lumen CLI: apps/cli/src/dispatch.rs matches `Command` variants; baseline rc-smoke "command-layer-play: PASS" proves the dispatch lane executed; `lumen commands` prints all 65) |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:2 |
| Commands run | `rg "AudioSetDelay|VideoSetZoom|SubtitleSetScale"` (whole tree) → definition+ids+spec+wire-test only, no executor; `rg "Command::SessionOpen"` → dispatchers located (apps/cli/src/dispatch.rs, apps/control-server/src/dispatch.rs:55, apps/web/src/host.rs:1403, apps/desktop/src/shell.rs:160); sed -n read of all four dispatchers' match arms; `rg "^\| \`(session|playback|…)\."` docs/05-command-layer.md → 61 §2 table rows enumerated and diffed against the 65 variants; `rg "deny_unknown_fields"` → command.rs:674 only (CommandListing), types.rs policy at 13–16; baseline log lines 1303–1408 (commands 57 passed; catalog_shape 4 passed incl. `spec_table_covers_exactly_the_enum_variants`; wire_forms 7 passed; catalog_drift 2 passed) |

## 1. One-paragraph verdict
This chunk is the `Command` payload enum: 58 of 65 variants (session/playback/tracks/audio/video/subtitle/playlist/media/config), adjacently tagged (`cmd`/`args`), each with an explicit stable-id serde rename. As a wire contract it is real and unusually well-guarded: the enum↔`CommandId`↔spec-table consistency is enforced by an executed test (`catalog_shape::spec_table_covers_exactly_the_enum_variants`, baseline PASS), the §5.1 wire example round-trips byte-for-byte (executed, PASS), and committed codegen artifacts are drift-checked (`catalog_drift`, PASS). The damning part is execution coverage: only 16 of 65 variants are served by any host in the tree (CLI 15 + desktop's frame-channel intercept; control-server 2; web 12) — the other 49 exist as vocabulary plus honest typed refusals (`unsupported-for-host`/`not-ready`), never fake success. Range constraints (volume/rate/delay/zoom/scale/fraction) live only in doc-comment prose; `Volume`/`Rate` are bare `f32` aliases, so no layer enforces the §2 tables' `out-of-range` contracts.

## 2. What actually works (verified)
— Enum↔spec↔id consistency EXECUTED: baseline test-workspace.log.txt:1388-1396 `Running tests/catalog_shape.rs … test spec_table_covers_exactly_the_enum_variants ... ok … test result: ok. 4 passed`.
— Wire forms EXECUTED: baseline:1398-1408 `tests/wire_forms.rs … test result: ok. 7 passed` (includes `doc_05_request_example`, `inline_args_are_camel_case_and_units_are_bare`).
— Round-trip unit tests EXECUTED: baseline:1303-1305 `commands-41bb5ff7683030b3 … test result: ok. 57 passed` (command.rs's own `mod tests` at 693+ lives in part2 but runs in this binary).
— Command lane in the shipped binary EXECUTED: baseline rc-smoke-engine-only.log.txt step 5 `command-layer-play: PASS (reached Playing, ended Stopped, exit 0)`; the 65-command count is pinned by apps/control-server/src/dispatch.rs test `the_catalog_lists_all_65_commands_from_the_live_spec`.
— Typed-refusal honesty EXECUTED: apps/cli/src/dispatch.rs:230-260 `unserved_commands_refuse_typed` and apps/control-server/src/dispatch.rs `everything_else_is_a_typed_not_ready_never_a_fake_success`.

## 3. Findings

### F-001 [S3] — 49 of 65 command variants have no executor anywhere in the tree
Where: crates/commands/src/catalog/command.rs:74-435 (enum body); evidence: apps/cli/src/dispatch.rs:154-157, apps/control-server/src/dispatch.rs:55-67, apps/web/src/host.rs:1403-1434
Code:
```rust
        // ---- everything else: the honest typed refusal ----
        other => Err(unserved(other)),
```
apps/control-server/src/dispatch.rs:64-67:
```rust
            _ => Err(CommandError::new(
                Stage::Control,
                ErrorCode::NotReady,
```
apps/web/src/host.rs:1434:
```rust
            other => Err(unserved(other)),
```
What is wrong: the union of served variants across all four hosts is 16: CLI grammar serves 15 (open/close/play/pause/toggle/stop/seek/info/time/duration/tracks/engine-info/commands/setting-get/setting-set — apps/cli/src/run.rs:794-846 `to_catalog`), web serves 12 (host.rs:1403-1431), control-server serves 2 (engine-info, list-commands), desktop intercepts `SessionSetFrameChannel` (shell.rs:160) and otherwise delegates. `rg "AudioSetDelay|VideoSetDelay|VideoSetZoom|SubtitleSetScale"` over the whole tree: only definition, ids.rs rename, spec.rs row, and a wire-serialization test — zero executors. So 49 variants (all of audio.set-*/video.set-*/subtitle.*/playlist.* [13]/tracks.set-*/media.set-chapter/bookmarks/system.plugin-*/playback.set-rate|step-frame|set-position/config.reset-setting|list-areas/…) are payload definitions with typed refusals only.
Why it matters: the payload enum IS the entire implementation for 49 commands; doc 05 §2 tables specify rich semantics (completion events, error codes) none of which can fire. Honest (never fake success — the refusals are typed and tested), and tracked by roadmap cards (`owner_of` names the owning card per prefix), but any doc/matrix claim that these commands work would be FALSE.
What correct looks like: executors per card, or the feature matrix marking all 49 not-started (the xtask feature-audit's 503 not_started cells presumably covers this — matrix wave to confirm).
Confidence: HIGH — Verification: read all four dispatchers + whole-tree rg per variant group.

### F-002 [S3] — Range constraints exist only as prose; no layer can enforce them
Where: crates/commands/src/catalog/command.rs:206-209 (and 138-141, 231-236, 280-285); types.rs:52-59; spec.rs:344-347
Code:
```rust
    /// `audio.set-delay` — atomic sync parameter (class S).
    #[serde(rename = "audio.set-delay")]
    AudioSetDelay {
        /// Signed delay (ms), ±60 000.
        delay_ms: i64,
    },
```
types.rs:52-53:
```rust
/// (§2.2, `[PROPOSAL]` range per doc 05).
pub type Volume = f32;
```
spec.rs:346-347:
```rust
            args: PayloadDesc::Type("delay_ms: i64"),
```
What is wrong: doc 05 §2 tables promise `out-of-range` errors for volume (`0.0..=audio.max-volume`), rate (`(0, 16]`), delays (±60 000), zoom (`0.25..=4.0`), subtitle scale (`0.2..=5.0`), position fraction (`0.0..=1.0`). The payload fields are bare `i64`/`f32`; `Volume`/`Rate` are type aliases, not newtypes; the spec table's `PayloadDesc` carries only the type string; JsonSchema derive on `f32` emits no bounds. No current layer rejects `{"cmd":"audio.set-volume","args":{"volume":999.0}}` — and since no executor exists (F-001), the range contracts have zero enforcement points today and no mechanical carrier for tomorrow.
Why it matters: when executors land, ranges must be re-typed by hand from prose — classic drift vector; the §2 tables' error taxonomy (`out-of-range`) is unenforceable as specified.
What correct looks like: range-carrying newtypes (as `media-types` does for its types) or bounds in the spec's PayloadDesc consumed by the executor and the schema.
Confidence: HIGH — Verification: read + rg (`60000|0\.25|4\.0|0\.2|5\.0` over commands/src → only doc comments).

### F-003 [S4] — Unknown arg fields inside command payloads are silently ignored
Where: crates/commands/src/catalog/command.rs:67-73 (enum serde attrs; no `deny_unknown_fields`), policy at types.rs:13-16
Code:
```rust
#[serde(
    tag = "cmd",
    content = "args",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum Command {
```
types.rs:14-16:
```rust
//! tagged enums (`kind`) with kebab-case variant names. V1 closed
//! shapes: plain structs carry `deny_unknown_fields`; enum payloads
//! reject unknown ids via their tags. `Auto`/`None` selectors and
```
What is wrong: the declared policy rejects unknown *tags* (unknown `cmd`/`kind` ids) but not unknown *fields* inside a variant payload — `{"cmd":"playback.set-rate","args":{"rate":2.0,"rat":1}}` deserializes successfully, silently dropping `rat`. A typo'd or future field is swallowed without diagnostic; contrast `OpenOptions` (types.rs:241 `deny_unknown_fields`) which does reject.
Why it matters: wire-contract clients get no signal for misspelled args; versioning safety is bought at the cost of typo detection for every command payload.
What correct looks like: `deny_unknown_fields` on the enum (serde applies it to struct variants) once the §6 versioning story allows, or a documented lint lane.
Confidence: HIGH — Verification: read of serde attrs + policy text; `rg "deny_unknown_fields" command.rs` → line 674 only (CommandListing, part2).

### F-004 [S4] — Doc 05 §2 tables document 61 commands; the catalog carries 65
Where: crates/commands/src/catalog/command.rs:386-404 (bookmark trio) vs docs/05-command-layer.md §2.10 (no bookmark rows); spec.rs:161-162
Code:
```rust
    /// `media.get-bookmarks` — the session's bookmark list (class S,
    /// issue #32).
```
spec.rs:161-162:
```rust
/// The full command table, in doc 05 §2.1 order (61 rows + the issue #32
/// bookmark trio + the w31.1 frame-channel bind).
```
What is wrong: `rg "bookmark" docs/05-command-layer.md` → no matches. The three bookmark commands and `session.set-frame-channel` (w31.1) exist in the enum/spec/checksum but not in the doc that the module header (command.rs:1-2 "doc 05 §2.3–2.12") claims to implement. The delta is documented — but only in a spec.rs code comment, not in doc 05.
Why it matters: doc 05 is the normative §2 reference; four shipped catalog ids have no §2 row (no documented class/result/error contract for them outside the code).
What correct looks like: doc 05 §2.10 gains the bookmark rows + §2.3 the frame-channel row (or a §2.13 "post-doc additions" table citing issue #32/w31.1).
Confidence: HIGH — Verification: rg on doc 05 + sed on spec.rs.

## 4. Per-function table (variant = the unit; payload/§2-row/executed-status)
| Variant (lines) | Payload | doc 05 §2 row | Served by | Notes |
|---|---|---|---|---|
| `SessionOpen` (77-84) | source, options | §2.3 ✓ | CLI, WEB | Executed in shipped binary (rc-smoke play lane). OpenOptions denies unknown fields (types.rs:241). |
| `SessionClose` (87-88) | — | §2.3 ✓ | CLI, WEB | Idempotent per §2.3. |
| `SessionSetFrameChannel` (95-101) | channel: u32 | w31.1 (not in §2) | DESKTOP shell (shell.rs:160) | Shell-owned; CLI/web unserved. Desktop build UNVERIFIED in sandbox (baseline #2). |
| `PlaybackPlay` (105-106) | — | §2.4 ✓ | CLI, WEB | Executed (rc-smoke). |
| `PlaybackPause` (108-109) | — | §2.4 ✓ | CLI, WEB | |
| `PlaybackTogglePlayPause` (111-112) | — | §2.4 ✓ | CLI, WEB | |
| `PlaybackStop` (115-116) | — | §2.4 ✓ | CLI, WEB | Executed (rc-smoke: reached Stopped). |
| `PlaybackSeek` (118-122) | target: SeekTarget | §2.4 ✓ | CLI only | Web host unserved (host.rs:1434). |
| `PlaybackSetRate` (124-128) | rate: Rate(f32 alias) | §2.4 ✓ | NONE | Typed refusal (dispatch.rs test pins it). Range (0,16] unenforced (F-002). |
| `PlaybackStepFrame` (130-134) | direction | §2.4 ✓ | NONE | |
| `PlaybackSetPosition` (138-141) | fraction: f32 | §2.4 ✓ | NONE | `[PROPOSAL]` fold-into-seek per §2.4; duplicates seek{Percent} semantics (documented). |
| `TracksList` (145-146) | — | §2.5 ✓ | CLI, WEB | Executed (rc-smoke tracks events). |
| `TracksGetInfo` (148-152) | id | §2.5 ✓ | NONE | |
| `TracksSetVideoTrack` (154-158) | selector | §2.5 ✓ | NONE | |
| `TracksSetAudioTrack` (160-164) | selector | §2.5 ✓ | NONE | |
| `TracksSetSubtitleTrack` (167-171) | selector | §2.5 ✓ | NONE | |
| `TracksSetSecondaryAudio` (173-177) | selector | §2.5 ✓ | NONE | |
| `AudioSetVolume` (181-185) | volume: Volume(f32) | §2.6 ✓ | NONE | F-002 range unenforced. |
| `AudioGetVolume` (187-188) | — | §2.6 ✓ | NONE | |
| `AudioSetMute` (190-194) | muted | §2.6 ✓ | NONE | |
| `AudioListDevices` (196-197) | — | §2.6 ✓ | NONE | |
| `AudioSetDevice` (199-203) | id: String | §2.6 ✓ | NONE | |
| `AudioSetDelay` (205-209) | delay_ms: i64 | §2.6 ✓ | NONE | F-002 (±60 000 prose only). |
| `VideoSetFullscreen` (214-218) | enabled | §2.7 ✓ (shell target) | NONE (no windowed host) | |
| `VideoSetAspectRatio` (220-224) | ratio | §2.7 ✓ | NONE | |
| `VideoSetCrop` (226-230) | crop | §2.7 ✓ | NONE | |
| `VideoSetZoom` (232-236) | factor: f32 | §2.7 ✓ | NONE | F-002. |
| `VideoSetDelay` (238-242) | delay_ms: i64 | §2.7 ✓ | NONE | F-002. |
| `VideoTakeScreenshot` (244-250) | path | §2.7 ✓ | NONE | |
| `SubtitleAddFile` (257-266) | path, encoding, select(default true) | §2.8 ✓ (PathSpec→String deviation marked `[VERIFY]` in-file) | NONE | default_true fn at 474 (part2) — in-file dependency noted. |
| `SubtitleSetSync` (269-273) | delta_ms | §2.8 ✓ | NONE | |
| `SubtitleSetEncoding` (275-279) | encoding | §2.8 ✓ | NONE | |
| `SubtitleSetScale` (281-285) | scale: f32 | §2.8 ✓ | NONE | F-002. |
| `PlaylistEnqueue` (289-295) | source, title | §2.9 ✓ | NONE | |
| `PlaylistEnqueueMany` (297-301) | items (non-empty per §2.9 — not enforced at type) | §2.9 ✓ | NONE | Empty-vec = invalid-argument is executor's job; none exists. |
| `PlaylistPlayItem` (303-307) | id | §2.9 ✓ | NONE | |
| `PlaylistNext` (309-310) | — | §2.9 ✓ | NONE | |
| `PlaylistPrev` (312-313) | — | §2.9 ✓ | NONE | |
| `PlaylistSkip` (315-319) | n: i32 | §2.9 ✓ | NONE | |
| `PlaylistRemove` (322-326) | id | §2.9 ✓ | NONE | |
| `PlaylistMove` (328-334) | id, to_index: u32 | §2.9 ✓ | NONE | |
| `PlaylistClear` (336-337) | — | §2.9 ✓ | NONE | |
| `PlaylistSetShuffle` (339-343) | enabled | §2.9 ✓ | NONE | |
| `PlaylistSetRepeat` (345-349) | mode | §2.9 ✓ | NONE | |
| `PlaylistSave` (351-357) | path, format | §2.9 ✓ | NONE | |
| `PlaylistLoad` (359-365) | path, append | §2.9 ✓ | NONE | |
| `MediaGetInfo` (369-370) | — | §2.10 ✓ | CLI, WEB | Executed (rc-smoke MediaInfoReady path). |
| `MediaGetCurrentTime` (372-373) | — | §2.10 ✓ | CLI, WEB | |
| `MediaGetDuration` (375-376) | — | §2.10 ✓ | CLI, WEB | Executed (smoke `TimingDurationKnown`). |
| `MediaGetChapters` (378-379) | — | §2.10 ✓ | NONE | |
| `MediaSetChapter` (381-385) | index: u32 | §2.10 ✓ | NONE | |
| `MediaGetBookmarks` (388-389) | — | issue #32 (NOT in doc 05) | NONE | F-004. |
| `MediaAddBookmark` (392-398) | time: Timestamp(µs), label | issue #32 | NONE | Wire shape pinned by executed test `bookmark_commands_round_trip_the_wire` (part2 test, 57-passed set). |
| `MediaRemoveBookmark` (400-404) | id: u32 | issue #32 | NONE | |
| `ConfigGetSetting` (408-414) | area, key | §2.11 ✓ | CLI | |
| `ConfigSetSetting` (416-424) | area, key, value | §2.11 ✓ | CLI | |
| `ConfigResetSetting` (426-432) | area, key | §2.11 ✓ | NONE | |
| `ConfigListAreas` (433-435) | — | §2.11 ✓ | NONE | |
(system section 437-470: 7 variants — part2/A01's chunk; boundary verified at 435/437.)

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `catalog_shape::spec_table_covers_exactly_the_enum_variants` (executed, PASS) | Yes | Enum↔spec drift fails it; also `committed_checksum_recomputes`. |
| `wire_forms::doc_05_request_example` etc. (executed, 7 PASS) | Yes | Byte-exact JSON assertions incl. camelCase args and the §5.1 example. |
| `catalog_drift::committed_artifacts_match_regeneration` (executed, PASS) | Yes | Regenerated codegen must equal committed. |
| `unserved_commands_refuse_typed` (apps/cli, executed) | Yes | Pins the typed-refusal contract for unserved variants (F-001's honesty guard). |
| No test in part1 itself | — | Tests live at 693+ (part2). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one variant per catalog entry, variant payloads taken verbatim from the §2 tables' Params columns" | command.rs:5-7 | REAL (61 rows) + 4 extras | spec.rs:161-162; catalog_shape test executed PASS; bookmark trio/frame-channel undocumented in doc 05 (F-004). |
| "unknown option = `invalid-argument`" (OpenOptions) | doc 05 §2.3 (row 227) | REAL | types.rs:241 `deny_unknown_fields` on OpenOptions; wire round-trips executed. |
| §2 tables' `out-of-range` errors for volume/rate/delay/zoom/scale | doc 05 §2.2/§2.4-2.8 | PAPER-ONLY | No enforcement layer exists (F-002); no executor for any of these commands (F-001). |
| "capability gaps are typed, not silent" | doc 05 §5.2 | REAL | All three hosts' `unserved`/NotReady arms + executed tests. |
| `session.set-frame-channel` "the pure CLI answers the typed unsupported-for-host refusal" | command.rs:92-94 | REAL | Not in CLI grammar → any JSON-lane request hits apps/cli/src/dispatch.rs:156 unserved → UnsupportedForHost; desktop shell.rs:160 intercepts. |

## 7. Dead / duplicated / unreachable code
- 49/65 variants: vocabulary-only (F-001) — no executor in any host; NOT dead code (wire contract + codegen + schema consumers) but unexecutable.
- `PlaybackSetPosition` duplicates `PlaybackSeek{Percent}` semantics by §2.4's own admission (`[PROPOSAL]` to fold) — documented duplication.

## 8. Dependencies & flags
serde + schemars (JsonSchema) + std only — matches types.rs's wire-type charter. No feature gates. `#[serde(default)]` on `options` (82-83) and `#[serde(default = "default_true")]` on `select` (264) both verified against doc 05 defaults. Crate builds clean in baseline (#1); its 57 unit + 15 integration tests passed (baseline #3, log lines 1303-1408).

## 9. Open questions for the next agent
- A01 (command.rs#part2): does `CommandResult`'s variant set cover every §2 *Result* column the same way? The emission-ledger test suggests yes — verify count.
- Matrix wave: do any feature-matrix rows claim audio/video/subtitle/playlist commands work on any platform? If yes → S0 per row (F-001 makes them FALSE).
- Whoever audits apps/desktop-tauri: the shell's frame-channel bind is the only desktop-served extra; the tauri build is UNVERIFIED in sandbox (baseline #2) — flag carries.
