# AUDIT: crates/commands/src/catalog/command.rs#part2 (lines 436–870)
| Field | Value |
|---|---|
| Auditor | W02-A01 |
| Date | 2026-11-21 |
| Lines / functions | 435 (chunk 436–870 of 871) / 1 fn + 2 types + 32 result variants + 7 test fns |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/run.rs:555-660` (matches `CommandResult` for display) → `apps/cli/src/dispatch.rs:58-155` (constructs `CommandResult::TrackList/EngineInfo/CommandListings/…`) → this file; also `lumen-control-server` e2e, baseline log 1526 `e2e_full_session_through_the_app_stack ... ok`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -o "CommandResult::[A-Za-z]+" apps/*/src crates/` (which variants any host can produce); `rg -n "CommandResult" crates/commands/tests/` (wire-pin coverage); `sed -n` (all quotes verified); baseline `sed -n '1305,1316p' test-workspace.log.txt` (all 6 chunk tests executed + passed) |

Chunk boundaries: part1 = A03 (1–435, `Command` enum), part2 = mine (436–870:
tail of `Command` (System* variants), `default_true`, `CommandResult` (32
variants), `CommandListing`, and the whole `mod tests`). Line 470 (`}` closing
`Command`) and line 436 (`#[serde(rename = "config.list-areas")]` area) sit at
the seam; the System variant payloads 437–469 are mine.

## 1. One-paragraph verdict
Part2 is the response half of the command catalog: `CommandResult` (32
internally-tagged wire variants), `CommandListing`, and seven wire-pinning
tests — all six of which executed and passed in the baseline
(`test-workspace.log.txt:1311-1316`, `commands` binary, 57/57 ok). The serde
plumbing is correct (adjacent/internal tagging, camelCase fields, unit-command
bare wire form, unknown-variant rejection — each pinned byte-exact), and the
host side guards the one NaN crash candidate (position fraction, host.rs:1345-1349).
The rot is breadth: **23 of 32 `CommandResult` variants cannot be produced by
any shipped host** (every app's production code constructs only 9; the rest are
typed-refused at dispatch), and **26 of 32 variants have no byte-exact wire
pin** in any committed test, so a tag/field rename in most of the vocabulary
would pass the entire suite. The type is real; two-thirds of it is protocol
vocabulary waiting for an engine.

## 2. What actually works (verified)
— Verified by baseline `test-workspace.log.txt:1311-1316`: all 6 in-file tests
(`bookmark_commands_round_trip_the_wire`, `deep_track_info_round_trips`,
`request_wire_form_matches_doc_example`, `result_variants_round_trip`,
`unit_commands_take_no_args`, `unknown_command_ids_are_rejected`) — ok, plus
`wire_forms.rs` 7/7 ok (1305-1316 and 1396-1404).
— Verified by baseline `mp4-smoke-open-session.txt` (`ok: accepted session=1
gen=0`) + `e2e_full_session_through_the_app_stack ... ok` (log:1526): the
envelope/`Command` wire forms run end-to-end through a real shipped binary.
— Verified by reading `apps/cli/src/host.rs:1345-1349`: `CurrentTime.position`
is guarded (`Some(duration) if duration > 0 … else 0.0`) — no NaN can reach
serde_json (which would error on NaN) from the shipped CLI's position math.

## 3. Findings

### F-001 [S3] 23 of 32 `CommandResult` variants are unproducible by any shipped host
Where: crates/commands/src/catalog/command.rs:491-668 (variants) + apps/cli/src/dispatch.rs:155
Code (dispatch fall-through):
```
        other => Err(unserved(other)),
```
What is wrong: union of production constructions across apps/cli, control-server,
desktop, web, desktop-tauri (rg `CommandResult::[A-Za-z]+` over `apps/*/src`,
excluding tests) = {Unit, MediaInfo, CurrentTime, Duration, TrackList,
EngineInfo, CommandListings, SettingResult, SettingApplied} — 9 of 32. The
other 23 (AppliedRate, TrackInfo, AppliedSelector, AudioVolume, AppliedBool,
AudioDevices, AppliedDelayMs, AppliedAspectRatio, AppliedCrop, AppliedZoom,
AppliedEncoding, AppliedScale, AppliedRepeat, PlaylistItem, PlaylistItems,
Chapters, Bookmarks, BookmarkAdded, ConfigAreas, PluginList, PluginToggle,
AppliedLogLevel, StateSnapshot) are constructed only by test files
(crates/commands/tests, control-protocol/tests, playback-engine/tests). The
shipped CLI answers their commands with the typed refusal above.
Why it matters: `system.list-commands` (served, run.rs:644) advertises the full
catalog to UIs; a client acting on it will find 70% of the result vocabulary
dead. Consistent with the repo's own parity 0/252 (baseline INDEX #9), but the
doc-05-derived breadth is invisible at the type level — nothing in this file
marks which variants any host can emit.
What correct looks like: per-variant host-support metadata in the catalog
(like `CompletionClass`), or a feature-matrix row set the codegen emits, so
`list-commands` reports capability honestly.
Confidence: HIGH — Verification: read + rg (all apps enumerated; crate hits are
test-only, verified by file paths).

### F-002 [S3] 26 of 32 `CommandResult` wire shapes have no byte-exact pin — renames pass the suite
Where: crates/commands/src/catalog/command.rs:807-831
Code:
```
        for (result, json) in [
            (CommandResult::Unit, r#"{"kind":"unit"}"#),
            (
                CommandResult::AudioVolume { volume: 1.25 },
                r#"{"kind":"audio-volume","volume":1.25}"#,
```
What is wrong: byte-exact wire pins exist for only 6 of 32 variants — Unit,
AudioVolume, TrackList, AppliedSelector (this test), Bookmarks (bookmark test,
773-775), SettingApplied (tests/wire_forms.rs:130-135). A serde regression
(renamed tag, per-variant `rename_all_fields` override, field rename) in any
of the other 26 is invisible: the JsonSchema/TS codegen regenerates *from* the
code (catalog_drift then passes after recommitting the artifact), and the
emission-ledger payload-shape test covers `EventKind`, not `CommandResult`
(catalog_emission_ledger.rs:279 — `Vec<(EventId, EventKind, Vec<&str>)>`).
Why it matters: the wire is a stability contract (doc 05 §6 "stable id");
nothing enforces it for most of the result vocabulary.
What correct looks like: extend the `result_variants_round_trip` table (or the
emission ledger) to pin one JSON literal per variant — 32 rows, one afternoon.
Confidence: HIGH — Verification: read (test bodies + ledger types checked).

### F-003 [S4] `unwrap_or_default()` swallows serialization errors into `""`/`Value::Null`
Where: crates/commands/src/catalog/command.rs:719 (also 825, 864)
Code:
```
        let json = serde_json::to_string(&request).unwrap_or_default();
        assert_eq!(
```
What is wrong: on a serialization failure the assert compares `""` (or
`Value::Null`) against the expected literal — the test still fails, but with a
useless diff instead of the serde error; the file already defines `round_trip`
(697-702) for exactly this.
Why it matters: degraded failure diagnostics in the tests that pin the wire
contract; inconsistent with the sibling tests in the same module that use the
helper.
What correct looks like: `round_trip(serde_json::to_string(&request))`.
Confidence: HIGH — Verification: read.

### F-004 [S4] Inert `rename_all = "kebab-case"` on `Command`
Where: crates/commands/src/catalog/command.rs:68-73
Code:
```
#[serde(
    tag = "cmd",
    content = "args",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
```
What is wrong: all 65 `Command` variants carry an explicit `#[serde(rename =
"…")]` (65 renames counted vs 65 variants), so `rename_all` never fires for
the tag; only `rename_all_fields` does work. (`CommandResult` at 486-490 is
the enum that genuinely relies on `rename_all`.)
Why it matters: an attribute that does nothing invites the belief that removing
an explicit rename is safe ("kebab-case covers it") — it does not: `SessionOpen`
would become `"session-open"`, not `"session.open"`.
What correct looks like: drop the attribute or comment that it is vestigial.
Confidence: HIGH — Verification: read + counted (rg -c rename vs variant regex).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Command` System* variants (tail) | 437-470 | REAL | — | 8 variants (`system.*`), all dispatched in apps/cli (SystemGetEngineInfo/ListCommands served, others typed-refused) and control-server dispatch.rs:56-61. Executed: e2e baseline. |
| `default_true` | 474-476 | REAL | — | serde default for `SubtitleAddFile.select` (line 264, part1). Constant `true` — correct per §2.8. |
| `CommandResult` (32 variants) | 491-668 | REAL (9 variants) / UNREACHABLE-in-production (23, F-001) | S3 | Internally tagged `"kind"`, kebab tags, camelCase fields; unit variant `{"kind":"unit"}` pinned at 809. No `deny_unknown_fields` (unsupported with internal tagging — correctly absent). |
| `CommandListing` | 673-687 | REAL | — | `deny_unknown_fields` + camelCase; produced by `SystemListCommands` (dispatch.rs:143, host.rs `command_listings`); consumed by run.rs:644. |
| `tests::round_trip` (helper) | 697-702 | REAL | — | Lint-legal unwrap; comment explains no-testkit constraint. |
| `tests::request_wire_form_matches_doc_example` | 711-728 | REAL | — | Byte-exact §5.1 pin; executed+passed (baseline 1313). Mutation: change tag/content attrs → assert fails. |
| `tests::bookmark_commands_round_trip_the_wire` | 730-779 | REAL | — | Pins bookmark trio incl. `label:null` round-trip; executed+passed (1311). Note: the corresponding *commands* are typed-refused by every host (F-001) — the pin protects a shape nothing produces yet. |
| `tests::unknown_command_ids_are_rejected` | 781-791 | REAL | — | Executed+passed (1316). Asserts error text contains "variant" — brittle against serde wording changes, but a real rejection check. |
| `tests::unit_commands_take_no_args` | 793-804 | REAL | — | Executed+passed (1315). Guards the bare-wire form for unit commands. |
| `tests::result_variants_round_trip` | 806-831 | REAL | — | Executed+passed (1314); only 4/32 variants (F-002). |
| `tests::deep_track_info_round_trips` | 836-869 | REAL | — | Executed+passed (1312); pins Unicode + deep nesting. Fixture is domain-impossible (Video track with `audio: Some(...)`, 847-859) — fine for serde depth-pinning, misleading as a domain sample. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| request_wire_form_matches_doc_example | YES | Exact-string assert on the serialized JSON; any tag/content/case regression diverges. Executed in baseline. |
| bookmark_commands_round_trip_the_wire | YES | Exact-string asserts ×3 incl. null-Option encoding. |
| unknown_command_ids_are_rejected | YES | `unreachable!("unknown cmd accepted")` fires if serde ever accepted an unknown tag. |
| unit_commands_take_no_args | YES | Parse-without-args must succeed; args-shape regression → Err → unreachable! panic. |
| result_variants_round_trip | YES (for its 4) | Exact-string round-trips; covers only 4/32 (F-002). |
| deep_track_info_round_trips | YES | Value round-trip + variant + field equality on a deep payload. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`[VERIFY]` — payloads doc 05 sketches only in prose (`EngineInfo.plugins`, `SettingValue`, `StateSnapshot`'s blocks)" | command.rs:22-24 | REAL (the markers exist at their types) | Markers present in-file (614-615 for `schema_meta`); honest uncertainty labeling. |
| "Variants are shared by shape across commands (see the module docs)" | command.rs:484 | REAL | e.g. AppliedDelayMs serves audio/video/subtitle delays (532-533). |
| "class-A commands answer `Accepted` and complete via events … they have no completed payload" | command.rs:17-19 | REAL | AppliedBool documents serving `audio.set-mute` (class S) etc.; no class-A command has a result variant; dispatch answers Accepted via receipt_outcome. |
| §5.1 wire example `{ "requestId": 42, "cmd": "playback.seek", … }` | command.rs:12-15 | REAL | Pinned byte-exact by executed test (711-728, baseline 1313 ok). |

## 7. Dead / duplicated / unreachable code
- 23 `CommandResult` variants: no production constructor in any app (F-001) —
  vocabulary-only, deliberately typed-refused at dispatch.
- `rename_all = "kebab-case"` on `Command`: inert (F-004).

## 8. Dependencies & flags
- serde/schemars derive-only usage; no runtime deps beyond std collections —
  consistent with the wire-crate role. No feature gates on this file.

## 9. Open questions for the next agent
- ids.rs auditor (my next file): does `CommandListing.id`/`since`/`requires`
  stay consistent with the `CommandId` table (spec.rs) — `requires` is
  documented "empty in v1" (686); is that asserted anywhere?
- Who serves `CommandResult::StateSnapshot`? `system.get-state-snapshot` is
  in the catalog; no host constructs the variant (F-001) — desktop hydration
  presumably wants it; check apps/desktop's snapshot path for a parallel
  non-catalog mechanism (drift risk).
