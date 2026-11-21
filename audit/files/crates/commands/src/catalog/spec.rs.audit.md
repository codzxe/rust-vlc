# AUDIT: crates/commands/src/catalog/spec.rs#part2 (lines 549–1097)
| Field | Value |
|---|---|
| Auditor | W02-A06 |
| Date | 2026-11-21 |
| Lines / functions | 549 (chunk part2 of 1097; part1 = 1–548, audited by W02-A07) / 15 items in part2 (8 fns + 8 tests; `catalog_data()` tail shared) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen commands` → apps/cli/src/parse.rs:209 → run.rs:823 `CliCommand::Commands` → dispatch.rs:142-145 `SystemListCommands` → host.rs:1665 `command_listings()` → `catalog_data()`; also `catalog_checksum()` ← apps/control-server/src/run.rs:81, config.rs:169, dispatch.rs:111) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `sed -n` on all quoted ranges (verbatim verification); `rg -c "CommandSpec \\{"` → 65 rows (66 minus struct def); `rg -c "EventSpec \\{"` → 26 rows (27 minus struct def); `rg -ln "Command::<id>"` for 12 part2 ids → zero dispatch sites; `sed -n '630,700p' crates/playback-engine/tests/table.rs` (gen_bumping cross-check); `sed -n '75,120p' apps/control-server/src/dispatch.rs` (2-command surface); `sed -n '38,54p' apps/cli/src/dispatch.rs` (SERVED=15); baseline test log rg `spec::tests` → 8/8 ok (test-workspace.log.txt:1326-1333) |

## 1. One-paragraph verdict
Part2 is the back half of the catalog data table (18 of 65 `CommandSpec` rows: §2.10 media-info incl. the bookmark trio, §2.11 config, §2.12 system), the entire 26-row `EventSpec` table, the FNV-1a checksum machinery, the two lookup helpers, and 8 tests. The data is real, internally consistent, order-pinned against `CommandId::ALL`/`EventId::ALL`, and every test in the file executed and passed in the baseline workspace run. The FNV-1a implementation matches the published vectors. The defects: the drift negative-control test exercises only 2 of the 9 fields the checksum claims to fold; 12 of the 18 part2 command rows describe commands no shipped host implements (verified: zero `Command::<id>` match arms repo-wide) while `system.list-commands` advertises them with no availability marker; and two doc-comment sites still claim the pre-w31.1/pre-issue-#42 catalog shape ("only video.set-fullscreen targets the shell"; "23 rows"/"61+3 rows"), contradicted by the file's own table and tests.

## 2. What actually works (verified)
- All 8 tests in the file passed in baseline run #3: `audit/runs/baseline/test-workspace.log.txt:1326-1333` — `catalog::spec::tests::{class_counts_match_the_doc_05_tables, classification_matches_section_1_5_examples, fnv1a64_reference_vectors, gen_bumping_set_is_exactly_section_4_2, mutated_table_entry_changes_the_checksum, pipeline_scoped_set_matches_section_3_2, shell_targeted_commands_are_the_shell_owned_surface, tables_cover_every_id_exactly_once_in_order}` all `ok`.
- FNV-1a constants and vectors correct: offset basis `0xcbf2_9ce4_8422_2325`, prime `0x0000_0100_0000_01b3` (spec.rs:840-843), reference vectors `""`→basis, `"a"`→`0xaf63_dc4c_8601_ec8c`, `"foobar"`→`0x85944171f73967e8` (spec.rs:1091-1096) match the published FNV-1a 64 spec.
- Row-count claims cross-verified mechanically: `rg -c "CommandSpec {"` = 66 (1 struct def + 65 rows), `EventSpec {` = 27 (1 def + 26 rows); committed artifact asserts "counts: 65 commands / 26 events" (crates/commands/tests/catalog_shape.rs:239-241).
- Reachability verified by call chain above; `lumen commands` is a shipped CLI verb (parse.rs:209).

## 3. Findings

### F-001 [S3] Drift negative control covers only 2 of the 9 checksum-folded fields
Where: crates/commands/src/catalog/spec.rs:1068-1088 (test) vs 848-851 (claim)
Code:
```rust
    /// The canonical text a catalog checksum folds: one line per row,
    /// field order fixed, doc-05 table order. Any catalog change — id,
    /// class, target, gen flag, arg/result shape, event scoping or rate —
    /// changes this text and therefore the checksum.
```
and
```rust
        let mutated_command = {
            let mut rows = catalog_data().to_vec();
            rows[7].gen_bumping = !rows[7].gen_bumping;
            checksum_of(&rows, event_catalog_data())
        };
```
What is wrong: `canonical_catalog_text` (852-878) folds 9 field classes into the digest (command id/class/target/gen/args/result; event id/scoping/rate), and the doc comment promises all of them move the checksum. The only negative control, `mutated_table_entry_changes_the_checksum`, mutates exactly one field per table: `gen_bumping` (row 7 = PlaybackSeek) and `pipeline_scoped` (row 0 = SessionStateChanged). A regression that drops `class`, `target`, `args`, `result`, `id`, or `rate` from the canonical text fails no test in this file — `fnv1a64_reference_vectors` tests the hash, not the text builder; the example/enum tests read the table, not the text.
Why it matters: the checksum is the repo's catalog-drift tripwire ("fails `--check` until the artifacts are regenerated", header line 13-18). The only other guard, `committed_checksum_recomputes` (catalog_shape.rs:229-243), compares the live value to the committed artifact — which is defeated by regenerating the artifact after breaking the builder, exactly the sequence the negative control exists to catch. Fields could silently stop being drift-tracked.
What correct looks like: the negative control mutates each folded field (or at minimum one field per field-class: id, class, target, args, result for commands; id, rate for events) and asserts `checksum_of(mutated) != baseline` for each.
Confidence: HIGH — Verification: read; mutation reasoned (not executed); baseline test pass cited for current behavior.

### F-002 [S3] 12 of 18 part2 command rows describe commands no host implements; introspection advertises them without an availability marker
Where: crates/commands/src/catalog/spec.rs:565-686 (rows), e.g. 573-580 and 647-686
Code:
```rust
        CommandSpec {
            id: CommandId::MediaSetChapter,
            class: CompletionClass::A,
            target: CommandTarget::Engine,
            gen_bumping: true,
            args: PayloadDesc::Type("index: u32"),
            result: PayloadDesc::Type("accepted"),
        },
```
and
```rust
        CommandSpec {
            id: CommandId::SystemPluginEnable,
            class: CompletionClass::S,
            target: CommandTarget::Engine,
            gen_bumping: false,
            args: PayloadDesc::Type("id: String"),
            result: PayloadDesc::Type("PluginToggle"),
        },
```
What is wrong: for the 12 part2 ids `MediaGetChapters, MediaSetChapter, MediaGetBookmarks, MediaAddBookmark, MediaRemoveBookmark, ConfigResetSetting, ConfigListAreas, SystemListPlugins, SystemPluginEnable, SystemPluginDisable, SystemSetLogLevel, SystemGetStateSnapshot` there is no `Command::<id>` match arm anywhere in `apps/` or `crates/` (verified: `rg -ln "Command::<id>"` returns nothing outside the catalog's own definitions and control-protocol/src/policy.rs's sensitive-list). The shipped surfaces serve 15 ids (CLI `SERVED`, apps/cli/src/dispatch.rs:38-54) and 2 ids (control-server `CatalogDispatch`, apps/control-server/src/dispatch.rs:33-36: "this control-server build embeds no engine"); every other id hits the catch-all `other => Err(unserved(other))` (dispatch.rs:155). Yet `CommandSpec`/`CommandListing` carry no availability field, so `system.list-commands` (the documented consumer of this table, header line 5-6) lists all 65 as indistinguishable from the servable 15.
Why it matters: the catalog's stated purpose is machine introspection ("UIs and the control server build menus and validate keymaps without hardcoding the catalog", apps/control-server/src/dispatch.rs:80-82). A client that enumerates and then invokes gets a refusal for 50 of 65 commands with no way to pre-filter. The refusals are typed and name the owning card (honest, by design), but the introspection surface itself cannot express the distinction — the data model omits the one bit that matters.
What correct looks like: an `availability`/`served` (or `owner-card`) field on `CommandSpec`/`CommandListing` emitted by `system.list-commands`, or per-host filtering of the listing.
Confidence: HIGH — Verification: read + repo-wide rg (zero dispatch sites); CLI/control-server dispatch read; no runtime invocation of an unserved command (baseline covers only the served lane).

### F-003 [S4] Module header count claims are stale (pre-w31.1 / pre-issue-#42 shape)
Where: crates/commands/src/catalog/spec.rs:5-8
Code:
```rust
//! [`catalog_data()`] is the single table the generators and
//! `system.list-commands` consume: one [`CommandSpec`] per §2.1 id (61
//! rows + the issue #32 bookmark trio), one [`EventSpec`] per §3.2 id
//! (23 rows). The rows carry the
```
What is wrong: the header says 61+3=64 command rows and 23 event rows; the actual tables (and the tests at 1026-1038, and the committed artifact line "counts: 65 commands / 26 events") are 65 and 26. The function-level docs are current — `catalog_data()` says "61 rows + the issue #32 bookmark trio + the w31.1 frame-channel bind" (161-162) and `event_catalog_data()` says "23 rows + the issue #42 additive trio" (698-700) — only the module header was never updated.
Why it matters: the header is the first thing a maintainer reads; it asserts an invariant ("one CommandSpec per §2.1 id") with numbers that the file's own tests contradict by +1/+3.
What correct looks like: "(61 rows + the issue #32 bookmark trio + the w31.1 frame-channel bind) … (23 rows + the issue #42 additive trio)".
Confidence: HIGH — Verification: read; counts mechanically verified (`rg -c`), tests and artifact cross-checked.
(Note: lines 5-8 sit in part1's chunk; recorded here because the contradicting tests/artifact live in part2. W02-A07 should reconcile.)

### F-004 [S4] "Only `video.set-fullscreen` targets the shell" contradicted by the file's own table and test
Where: crates/commands/src/catalog/spec.rs:23 and 87-89 (claim), 183-189 + 1003-1019 (contradiction)
Code:
```rust
//! matches §1.5's example lists; the gen-bumping set is exactly §4.2's
//! list; only `video.set-fullscreen` targets the shell.
```
```rust
    /// The shell/host (v1: only `video.set-fullscreen`).
    Shell,
```
versus the table row and the test that asserts two shell targets:
```rust
            id: CommandId::SessionSetFrameChannel,
            class: CompletionClass::S,
            target: CommandTarget::Shell,
```
```rust
    /// §1.1.3 + §2.7: `video.set-fullscreen` is the only shell-routed
    /// command in v1; everything else targets the engine.
    #[test]
    fn shell_targeted_commands_are_the_shell_owned_surface() {
```
(the test body at 1012-1018 asserts exactly `[SessionSetFrameChannel, VideoSetFullscreen]`).
What is wrong: three doc sites (23, 88, 1003-1004) still claim the pre-w31.1 shape while the table (183-189) and the test's own assertion enumerate two shell-targeted commands. The test comment is factually wrong about what the test two lines below asserts.
Why it matters: the "only X targets the shell" claim is an audited invariant of the routing layer; stale comments here seed wrong assumptions in the dispatcher waves.
What correct looks like: "the shell-routed commands in v1 are `video.set-fullscreen` and `session.set-frame-channel` (w31.1)".
Confidence: HIGH — Verification: read; all four sites quoted verbatim above.
(Sites 23 and 87-89 are in part1's chunk; the contradicting table row is part1, the contradicting test is part2. W02-A07 should reconcile.)

## 4. Per-function table (part2 items)
| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `catalog_data()` — part2 tail (§2.10 media-info rows, §2.11 config rows, §2.12 system rows; 18 of 65 rows) | 549-696 | REAL | S3 (F-002) | Pure const data; 6 rows served by CLI (`MediaGetCurrentTime`, `MediaGetDuration`, `ConfigGetSetting`, `ConfigSetSetting`, `SystemGetEngineInfo`, `SystemListCommands`), 12 with no implementation anywhere. Order/coverage pinned by test at 908-925 (baseline-passed). |
| `event_catalog_data()` (26 rows) | 701-834 | REAL | — | Pure const data; scoping set (13 scoped) pinned exactly by test 1042-1066; zip-order pinned vs `EventId::ALL` at 914-916. `pipeline_scoped` consumed by tests + generated schema only — runtime scoping is decided elsewhere (envelope presence: crates/commands/src/envelope.rs:139-141; engine-side negative list: crates/playback-engine/src/event.rs:625-635). Divergence between those two independent truth sources is possible; catalog_shape.rs:125-133 pins only the catalog side. |
| `fnv1a64()` | 836-846 | REAL | — | Correct published constants; wrapping_mul; verified against reference vectors (1091-1096, baseline-passed). No panics; O(n) single pass. |
| `canonical_catalog_text()` | 848-878 | REAL | S3 (F-001) | Correct today (all 9 field classes folded, `\t`/`\n` framing; no descriptor string contains a tab — verified by reading every `PayloadDesc::Type` literal in the file). Per-row `format!` + String push is fine for a codegen-time path, not a hot path. Tab/newline injection would require a hand-edited descriptor containing one — no validation, acceptable for 'static literals but unpinned by tests (F-001). |
| `catalog_checksum()` | 880-884 | REAL | — | Thin wrapper; consumed by control-server (run.rs:81, config.rs:169, dispatch.rs:111), codegen.rs:149, conformance.rs:157, catalog_shape.rs:233. |
| `checksum_of()` | 886-890 | REAL | — | Test seam for mutated copies; fine. |
| `command_spec()` | 892-895 | REAL | — | Linear find over 65 rows; sole caller control-server dispatch.rs:88 inside an O(65) loop → 4225 comparisons once per `system.list-commands` — negligible. Returns `Option`; callers `filter_map` (dispatch.rs:87-90) so a missing row would be silently dropped from listings — but test 908-925 pins 1:1 coverage. |
| `event_spec()` | 897-900 | REAL | — | Linear find over 26 rows; no production caller found outside tests/codegen (rg verified: only spec.rs itself) — helper kept for symmetry/future use. Dead-ish but trivial; not counted a finding (S4-borderline). |
| test `tables_cover_every_id_exactly_once_in_order` | 907-925 | REAL | — | Length + zip order + duplicate checks; would fail on any row drop/dup/reorder. |
| test `classification_matches_section_1_5_examples` | 930-975 | REAL | — | 8 S + 17 A example ids; would fail if any of those 25 flipped; the other 40 are covered by counts test. |
| test `gen_bumping_set_is_exactly_section_4_2` | 978-1001 | REAL | — | Exact set equality on 8 ids; strong. Cross-checked against engine behavior by crates/playback-engine/tests/table.rs:675-704 (TRANSITIONS mint exactly on Stop/Seek/SwitchTrack/Close). |
| test `shell_targeted_commands_are_the_shell_owned_surface` | 1005-1019 | REAL | S4 (F-004) | Exact set (2 ids); comment stale, assertion correct. |
| test `class_counts_match_the_doc_05_tables` | 1025-1038 | REAL | — | (19, 46, 65) counts; blind only to simultaneous S↔A swaps on non-example ids — acceptable. |
| test `pipeline_scoped_set_matches_section_3_2` | 1041-1066 | REAL | — | Exact 13-element list; strong. |
| test `mutated_table_entry_changes_the_checksum` | 1071-1088 | REAL (weak) | S3 (F-001) | Only 2 of 9 folded fields exercised. |
| test `fnv1a64_reference_vectors` | 1090-1096 | REAL | — | Published vectors; pins the hash itself. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| tables_cover_every_id_exactly_once_in_order | YES | Drop/dup/reorder any row → len/zip/BTreeSet assert fires. |
| classification_matches_section_1_5_examples | YES (for 25 of 65 ids) | Direct class equality per example id. |
| gen_bumping_set_is_exactly_section_4_2 | YES | Exact set equality, sorted both sides. |
| shell_targeted_commands_are_the_shell_owned_surface | YES | Exact Vec equality against both shell ids. |
| class_counts_match_the_doc_05_tables | YES (counts only) | A single S↔A flip changes both counts; a compensating double-swap on non-example ids would pass here but is caught for the 25 example ids by the classification test. |
| pipeline_scoped_set_matches_section_3_2 | YES | Exact 13-element list equality. |
| mutated_table_entry_changes_the_checksum | PARTIALLY (F-001) | Catches gen_bumping/pipeline_scoped being dropped from the canonical text; silent for id/class/target/args/result/rate. |
| fnv1a64_reference_vectors | YES | Known-answer test with published vectors. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "one CommandSpec per §2.1 id (61 rows + the issue #32 bookmark trio)" (64) | spec.rs:6-7 | FALSE (stale by 1) | File has 65 rows (rg -c, test 1037, artifact "counts: 65 commands / 26 events"). |
| "one EventSpec per §3.2 id (23 rows)" | spec.rs:7-8 | FALSE (stale by 3) | 26 rows (rg -c, test 1036-1037, artifact counts line). |
| "only `video.set-fullscreen` targets the shell" | spec.rs:22-23 | FALSE | Table 183-189 + test 1012-1018: two shell targets. |
| "Any catalog change — id, class, target, gen flag, arg/result shape, event scoping or rate — changes this text and therefore the checksum" | spec.rs:848-851 | REAL (code) / UNPROVEN (test coverage) | canonical_catalog_text 852-878 folds all 9; only 2 are negative-control-tested (F-001). |
| FNV-1a "Parameters per the published FNV-1a spec" | spec.rs:838 | REAL | Vectors 1091-1096 match published values; baseline-passed. |
| Catalog rows = servable commands (implicit in `system.list-commands` consuming the table) | spec.rs:5-6, doc 05 §2.12 [PROPOSAL] | SHALLOW | 12 of 18 part2 rows have no dispatch site repo-wide (F-002); refusals are typed (CLI dispatch.rs:154-155). |
| Gen-bumping set is §4.2's closed list | spec.rs:977-1001, cross-checked playback-engine/tests/table.rs:675 | REAL | Test passed in baseline (test-workspace.log.txt:1329); engine TRANSITIONS mint on exactly Stop/Seek/SwitchTrack/Close. |

## 7. Dead / duplicated / unreachable code
- `event_spec()` (897-900): no production caller found (rg over apps/ + crates/: only spec.rs's own tests/codegen-adjacent references). Trivial, symmetric API; S4-borderline, not counted.
- `pipeline_scoped` on `EventSpec` is duplicated truth: runtime scoping is computed from envelope shape (envelope.rs:139-141 `session.is_some() && generation.is_some()`) and from the engine's own negative list (playback-engine/src/event.rs:625-635). Only the catalog side is pinned by tests; the other two sources are not cross-checked against it (no test ties all three together). Flagged for the event.rs/engine waves.
- No dead rows: every id enum variant is pinned 1:1 by test 908-925.

## 8. Dependencies & flags
- No new deps; std-only (FNV-1a hand-rolled by design, documented rationale at 836-838 — verified sound: std `DefaultHasher` is indeed not stable across releases).
- `serde`/`schemars` derives on `CompletionClass`/`CommandTarget` (lines 29-39, 72-83, part1 range) feed the generated schema; no feature flags gate this file; crate builds default (baseline build #1 clean).
- No `unsafe`, no panics, no unwraps in part2 (the `unwrap_or_else(|| unreachable!(...))` at playback-engine/tests/table.rs:669 is a different file).

## 9. Open questions for the next agent
- W02-A07 (spec.rs part1): reconcile F-003/F-004 doc sites (lines 5-8, 22-23, 87-89) — they sit in your chunk.
- CLI/engine waves: `apps/cli/src/host.rs:1634` reports `catalog_version: "1"` while control-server (run.rs:81) reports `catalog_checksum().to_string()` — the same wire field (`EngineInfo.catalog_version`, types.rs:588 "this file's contract, §6.1") carries two different value spaces across hosts. Doc 05 §6.1 reads "implicit major v1", favoring the CLI's "1"; the control-server's checksum overload contradicts it. Needs a doc-vs-code ruling.
- Catalog-commands wave: `PayloadDesc::Type("id: String")` for `SystemPluginEnable/Disable` — policy.rs (control-protocol) treats these as sensitive ops; confirm the wire arg names match doc 05 §2.12 when those commands get an owner.
- Should `CommandListing` grow an availability field (F-002), the generated TS/schema and every consumer need a coordinated bump; the checksum gate will force it anyway.
