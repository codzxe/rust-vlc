# AUDIT: crates/commands/src/catalog/spec.rs
| Field | Value |
|---|---|
| Auditor | W02-A02 |
| Date | 2025-06-27 |
| Lines / functions | 1097 / 13 items + 8 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (lumen-cli: run.rs:823 -> dispatch.rs:142-144 -> host.rs:1665 `catalog_data()`; xtask codegen -> catalog-gen -> codegen.rs:138-149) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `wc -l` (1097); `sed -n` reads (full file, 2 chunks); `rg` for API callers (multiple, repo-wide, exit 0); python3 row-parse (65 cmd rows, 26 event rows, class A/S=19/46, gen-bumping=8, shell=2, scoped=13); python3 FNV-1a vectors (match); `cat schemas/generated/commands.checksum.txt`; baseline citation |

## 1. One-paragraph verdict
This file is what it claims to be: a pure const data table (65 command rows, 26 event rows) plus a correctly implemented FNV-1a checksum over a canonical rendering, with 8 honest tests that pin the table to `CommandId::ALL`/`EventId::ALL`, to doc 05's classification/gen-bumping/shell-target/scoping lists, and to the committed checksum artifact. I re-derived the row counts, class counts, gen-bumping set, shell set and scoped-event set independently with a python parser and they all match; I re-computed the three FNV-1a reference vectors in python and they match; the baseline run shows all 8 tests `ok` and the committed-artifact recompute `ok`. It forwards to nothing because it is leaf data — but its own doc comments lie in three places (stale counts, a false "only video.set-fullscreen targets the shell" invariant, and an overstated checksum mechanism), and `event_spec()` is dead code with zero callers.

## 2. What actually works (verified)
- Row invariants re-derived: `python3` regex parse of the table -> 65 CommandSpec rows, 19 A / 46 S, gen-bumping = {PlaybackSeek, PlaybackSetPosition, PlaybackStepFrame, MediaSetChapter, PlaybackStop, TracksSetVideoTrack, TracksSetAudioTrack, SessionClose}, shell-target = {SessionSetFrameChannel, VideoSetFullscreen}, 26 EventSpec rows with 13 pipeline-scoped — all identical to what the tests assert and to doc 05 §2/§3.2 (spot-checked `session.close` L228, `playback.seek` L238, `tracks.set-subtitle-track` L251, `audio.set-device` L262, §3.2 table L379-401 of docs/05-command-layer.md).
- FNV-1a: `python3` re-implementation -> `0xcbf29ce484222325`, `0xaf63dc4c8601ec8c`, `0x85944171f73967e8` for "", "a", "foobar" — equal to the assertions at spec.rs:1091-1096.
- Tests executed: audit/runs/baseline/cargo-test-workspace.txt:1459-1466 (`catalog::spec::tests::*` 8/8 `ok`) and :1525 (`committed_checksum_recomputes ... ok` — the committed `catalog-fnv1a64: debb80f9922a192a` at schemas/generated/commands.checksum.txt recomputes).
- Reachability: `rg` -> apps/cli/src/run.rs:823 `CliCommand::Commands => Ok(Command::SystemListCommands)` -> apps/cli/src/dispatch.rs:142-144 -> apps/cli/src/host.rs:1665-1666 `commands::catalog::catalog_data()`; tools/xtask/src/cmd/codegen.rs:67-73 spawns `catalog-gen`.

## 3. Findings
F-001 [S3]
Where: crates/commands/src/catalog/spec.rs:20-23 (and 87, 182-189, 354-361, 1006-1018)
Code:
```
//! Invariants, test-enforced: every id enum variant has exactly one
//! row; rows appear in doc-05 table order; the S/A classification
//! matches §1.5's example lists; the gen-bumping set is exactly §4.2's
//! list; only `video.set-fullscreen` targets the shell.
```
What is wrong: The claimed "test-enforced invariant" is false. Two commands target the shell — the table row at 182-189 sets `target: CommandTarget::Shell` for `SessionSetFrameChannel`, the row at 354-361 does the same for `VideoSetFullscreen`, and the file's own test `shell_targeted_commands_are_the_shell_owned_surface` (1006-1018) asserts exactly `[SessionSetFrameChannel, VideoSetFullscreen]`. The `CommandTarget::Shell` doc at L87 (`/// The shell/host (v1: only \`video.set-fullscreen\`).`) repeats the lie. Why it matters: the shell-target set is the host-executed (non-engine) surface — understating it in the invariant list misleads any maintainer auditing the host attack surface. What correct looks like: header and L87 updated to "v1: `video.set-fullscreen` and `session.set-frame-channel`". Confidence: HIGH — Verification: read + python row-parse + rg of the test.

F-002 [S3]
Where: crates/commands/src/catalog/spec.rs:107-111 (only consumer: 861-862)
Code:
```
/// A compact, human-stable arg/result shape descriptor. Part of the
/// checksum: changing a payload shape changes the descriptor string,
/// which changes the checksum, which fails `--check` until the
/// artifacts are regenerated.
```
What is wrong: The claimed mechanism does not exist. The descriptors are hand-typed free-form `&'static str`; nothing compiles, tests, or generates them from the real `Command`/`CommandResult` payload types. Changing the actual payload in command.rs leaves every descriptor string byte-identical, so `catalog-fnv1a64` does not move. Why it matters: the doc sells the descriptor as automatic drift protection; it is a manually-synced comment that happens to be hashed. Actual payload drift IS caught — but only by the separate `schema-fnv1a64`/`typescript-fnv1a64` digests that codegen.rs:142-150 derives from the real types, a mechanism this comment does not mention. What correct looks like: either generate descriptors from the schema or rewrite the comment to say the catalog line only catches hand-edits to this table. Verified: `rg "spec.args|spec.result|.args.as_str|.result.as_str"` repo-wide -> only spec.rs:861-862. Confidence: HIGH — Verification: read + rg.

F-003 [S3]
Where: crates/commands/src/catalog/spec.rs:897-901
Code:
```
/// Look up one event's row.
pub fn event_spec(id: EventId) -> Option<&'static EventSpec> {
    event_catalog_data().iter().find(|spec| spec.id == id)
}
```
What is wrong: Dead code. Zero callers repo-wide (`rg "event_spec\(" crates/ apps/ tools/ --type rust` -> only the definition). Its sibling `command_spec` has exactly one production caller (apps/control-server/src/dispatch.rs:88). Not re-exported in catalog/mod.rs's `pub use` list, not used by any test. Why it matters: unused public API on a `publish = false` crate is dead weight and implies a lookup path that no host exercises. What correct looks like: delete it or wire it into the control server's event-validation path. Confidence: HIGH — Verification: rg repo-wide.

F-004 [S4]
Where: crates/commands/src/catalog/spec.rs:6-8
Code:
```
//! `system.list-commands` consume: one [`CommandSpec`] per §2.1 id (61
//! rows + the issue #32 bookmark trio), one [`EventSpec`] per §3.2 id
//! (23 rows). The rows carry the
```
What is wrong: Stale counts. The table holds 65 command rows (61 + 3 + the w31.1 frame-channel bind, as the correct comment at 162-163 says) and 26 event rows (23 + issue #42 trio, as 699-701 says). The module header never got the w31.1/#42 updates. Why it matters: the header is the first thing a reader trusts; it disagrees with the file's own body comments and tests (1026-1040 asserts 65 total). Confidence: HIGH — Verification: read + python count.

F-005 [S4]
Where: crates/commands/src/catalog/spec.rs:41-46 vs 174-181
Code:
```
pub enum CompletionClass {
    /// Synchronous-complete: pure queries; atomic mutations with bounded
    /// work, no pipeline invalidation, no unbounded I/O. The response
    /// IS the completion evidence (C2).
    S,
```
What is wrong: `session.close` is class S AND `gen_bumping: true` (row at 174-181) — a pipeline invalidation inside class S, contradicting the "no pipeline invalidation" clause. Doc 05 §2.3 (docs/05-command-layer.md:228) documents this as deliberate ("Chosen S deliberately"), but this comment omits the exception, so the local definition and the local data disagree. Why it matters: a reader classifying a new command from this comment alone would mis-classify close-like commands. What correct looks like: add "…except terminal session teardown (§2.3 session.close)" to the S doc. Confidence: HIGH — Verification: read + doc 05 §2.3.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CompletionClass (enum) + as_str + Display | 41-68 | REAL | — | 2 variants, wire "s"/"a"; used in canonical text + CommandListing |
| CommandTarget + as_str + Display | 72-105 | REAL | — | doc at 87 stale (F-001) |
| PayloadDesc + as_str | 107-129 | REAL | S3 | doc overstates mechanism (F-002); strings hand-maintained |
| CommandSpec (struct) | 133-149 | REAL | — | plain data |
| EventSpec (struct) | 151-161 | REAL | — | plain data |
| catalog_data() | 163-696 | REAL | — | 65 const rows; verified counts/classes/flags by python parse |
| event_catalog_data() | 701-834 | REAL | — | 26 const rows; 13 scoped; rate strings faithful to §3.2 |
| fnv1a64() | 839-850 | REAL | — | vectors independently re-computed (python), match published FNV-1a |
| canonical_catalog_text() | 852-880 | REAL | — | tab-separated, deterministic; not hot path |
| catalog_checksum() | 882-886 | REAL | — | recomputes to committed artifact (baseline :1525 ok) |
| checksum_of() | 888-891 | REAL | — | used by drift test only |
| command_spec() | 893-896 | REAL | — | O(n) lookup, n=65, one caller (control-server dispatch.rs:88) |
| event_spec() | 897-901 | UNREACHABLE | S3 | zero callers (F-003) |
| tests (8) | 903-1097 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| tables_cover_every_id_exactly_once_in_order (908) | YES | zips table against macro-generated ALL, dup-check via BTreeSet |
| classification_matches_section_1_5_examples (931) | YES | asserts concrete class per 25 named ids |
| gen_bumping_set_is_exactly_section_4_2 (979) | YES | sorted-set equality against the 8-id closed list |
| shell_targeted_commands_are_the_shell_owned_surface (1006) | YES | asserts the 2-command shell set (and thereby refutes the header, F-001) |
| class_counts_match_the_doc_05_tables (1026) | YES | 19/46/65 exact |
| pipeline_scoped_set_matches_section_3_2 (1042) | YES | exact 13-id list |
| mutated_table_entry_changes_the_checksum (1072) | YES | negative control: flips row[7].gen_bumping / rows[0].pipeline_scoped and requires checksum change |
| fnv1a64_reference_vectors (1091) | YES | published vectors (independently re-verified in python) |
All 8 ran green in the orchestrator baseline (cargo-test-workspace.txt:1459-1466).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "61 rows + issue #32 trio / 23 rows" | spec.rs:6-8 | FALSE | table is 65/26 (python count; test 1026; committed checksum "counts: 65 commands / 26 events") |
| "only video.set-fullscreen targets the shell" | spec.rs:23, 87 | FALSE | 2 shell rows (182-189, 354-361); test 1006 asserts both |
| "changing a payload shape changes the descriptor string" | spec.rs:108-109 | FALSE (mechanism) | descriptors hand-typed; only consumer 861-862; payload drift caught only via schema digest (codegen.rs:142-150) |
| "FNV-1a … Parameters per the published FNV-1a spec" | spec.rs:837-838 | REAL | python re-implementation of vectors matches (1091-1096) |
| "checksum that cargo xtask codegen commits" | spec.rs:13-15 | REAL | tools/xtask/src/cmd/codegen.rs:67-73 spawns catalog-gen; committed artifact exists and recomputes (baseline :1525) |
| gen-bumping set = §4.2's list | spec.rs test 979 vs doc 05:433 | REAL | both list the same 8 ids |

## 7. Dead / duplicated / unreachable code
- `event_spec()` (897-901): dead, zero callers (F-003).
- No duplication found; the id list exists once (ids.rs macro) and both spec.rs and codegen consume it.

## 8. Dependencies & flags
Crate `commands` v0.1.0: serde 1.0.229 (derive), serde_json 1.0.154, schemars 1.2.3 — all used here only via derives; no feature flags gate anything in this file (no cfg in file). Bin target `catalog-gen` declared in Cargo.toml. No `#[allow]` in this file.

## 9. Open questions for the next agent
- `catalog_version` semantics disagree across hosts: apps/cli/src/host.rs:1634 and apps/web/src/host.rs:715 hardcode `"1"`, while control-server (run.rs:81, config.rs:169, dispatch.rs:111) and control-protocol (server.rs:974) report `catalog_checksum().to_string()`. Same field, different meaning per host — which one is the contract? (Doc 05 §2.12 just says `catalog_version`.)
- Doc 05:234 says `playback.play` from `ended` restarts "(same session, gen bump)" — engine-side bump on a command whose row is `gen_bumping: false` (consistent with §4.2's accept-time list, but the engine behavior needs verification in the engine crate).
- control-server (the only app caller of `command_spec`/`catalog_checksum`) is NOT in the shipped release lane (release.yml:193 builds only lumen-cli + desktop) and its startup is refused by the shipped product model (`product.toml:167 control-server = false`, enforced by config.rs:149-157). It IS built+tested by the CI workspace lane (ci.yml:233-234). See my run.rs audit.
