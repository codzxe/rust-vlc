# AUDIT: crates/commands/src/catalog/types.rs#part2 (lines 455-907)
| Field | Value |
|---|---|
| Auditor | W02-A05 |
| Date | 2026-11-21 |
| Chunk boundary | part1 (W02-A02) = lines 1-454 (through `StepDirection`); part2 (this audit) = lines 455-907 (`SeekKind` … `StateSnapshot`, plus the `#[cfg(test)] mod tests` at 788-907). File total 907 lines. |
| Lines / functions | 453 / 19 types + 5 tests |
| File verdict | REAL (wire contract: derived, schema-pinned, round-trip tested for 5 of 19 types; most part2 payloads UNREACHABLE at runtime — served only as typed refusals by the shipped host) |
| Reachable from shipped entry point? | PARTIAL — chain: `lumen` CLI → `apps/cli/src/dispatch.rs::route` → `Command::…` payloads. Only `SettingValue` (dispatch.rs:151-153, run.rs:755-884), `EngineInfo`/`DecoderInfo`/`SessionSummary` (dispatch.rs:139-141 → `host::engine_info()`), `SeekKind` (playback-engine event mapping, event.rs:65) are constructible at runtime. The other 12 types ride commands/events the CLI host refuses (`SERVED` = 15 of 65 ids, dispatch.rs:38-54). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | rg usage of every part2 type in `crates/commands/src` (all referenced by command.rs/event.rs payload enums — no orphans); rg part2 type names across `apps/` (runtime producers: only dispatch.rs/host.rs/run.rs hits); rg `PlaylistSetRepeat\|PlaylistSave\|SystemListPlugins\|SystemGetStateSnapshot` in `**/tests/**` → 0 constructions; rg `lumen_cli\|native_seek` in `audit/runs/baseline/test-workspace.log.txt` → NOT RUN (see Baseline note); sed verification of all quotes |

## 1. One-paragraph verdict
Part2 is the back half of the catalog's wire vocabulary: 19 enum/struct definitions for playlist, config, plugin, and snapshot surfaces, all consistently derived (`Serialize/Deserialize/JsonSchema`), all kebab-case-tagged or camelCase-renamed per the file's §5.1 convention, and all referenced by the `Command`/`EventKind` payload enums (no orphan types). The shapes are pinned twice — by schemars into the committed schema/TS goldens (drift test passed, baseline log 1375) and by round-trip tests for five of them in `event.rs`. The defects: not one of these 19 types has a direct wire-form test in this file's own test module (all five tests there target part1 types), the snapshot block structs deserialize all-defaults from `{}` while their own container requires every field, and `SettingValue`'s untagged enum produces context-free deserialization errors on the `--control-stdio` wire. Biggest context finding: the CLI host serves only 15 of 65 catalog ids, so most of this vocabulary is contract-without-runtime — honest (typed refusals), but unreachable as behavior.

## 2. What actually works (verified)
— verified by `audit/runs/baseline/test-workspace.log.txt:1334-1338`: the 5 tests in this chunk's range (`closed_shapes_reject_unknown_fields`, `duration_is_never_a_sentinel`, `playback_state_transitions_follow_section_3_3`, `seek_target_wire_form_matches_doc_example`, `selector_variants_round_trip`) ran **ok**.
— verified by `audit/runs/baseline/test-workspace.log.txt:1375` (`committed_artifacts_match_regeneration` ok) + `sed -n '88,94p' ui/src/generated/commands.ts`: every part2 type's shape is serialized into the committed schema and TS (`SettingValue = (boolean | number | string | string[])`, `StateSnapshot` with all 14 fields) and byte-matches regeneration — shape drift on any part2 type fails the golden test.
— verified by rg over `crates/commands/src`: every part2 type is imported by `command.rs` or `event.rs` payload definitions (e.g. `command.rs:300` `PlaylistEnqueueItem`, `command.rs:466` `SystemGetStateSnapshot`, `event.rs:219` `PlaylistChange`) — zero dead definitions within the catalog.
— verified by `apps/cli/src/dispatch.rs:148-153` + baseline `rc-smoke-engine-only.log.txt` (engine lane): the settings seam (`SettingValue`) and `lumen engine-info` (`EngineInfo`) are served commands in the shipped binary.

## 3. Findings
### F-001 [S3]
Where: crates/commands/src/catalog/types.rs:788-907 (test module) vs 455-786 (19 untested types)
Code:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seek_target_wire_form_matches_doc_example() {
        let target = SeekTarget::Relative { delta_ms: -10_000 };
```
What is wrong: The chunk's test module contains 5 wire-form/round-trip tests, and every one targets a **part1** type (`SeekTarget`, `Duration`, `TrackInfo`, `TrackSelector`, `PlaybackState`). None of the 19 types defined in part2 has a direct test here. Repo-wide rg for constructions of the payload-bearing ones (`PlaylistSetRepeat`, `PlaylistSave`, `SystemListPlugins`, `SystemGetStateSnapshot`, `PlaylistEnqueueMany`) in `**/tests/**` → 0 hits. Borrowed coverage exists but is thin: `event.rs:402-535` round-trips `SeekKind`, `PlaylistChange::{Moved,Cleared}`, `TrackChangeKind`, `LogLevel`, `PluginAction`; `tests/wire_forms.rs:90-96,130-136` round-trips `SeekKind` and `SettingValue::Bool`; `tests/catalog_emission_ledger.rs:276-399` pins wire KEYS (not values) for `SeekKind`'s event. `SettingValue`'s other three variants (`Number`, `String`, `List`) and the untagged ordering are never round-tripped; `SettingValue` is otherwise exercised only via `apps/cli` dispatch tests — which did **not** run in baseline (see §Baseline-integrity note).
Why it matters: The schema golden pins the *declared* shape, but serde *behavior* (tag forms, `default`, `deny_unknown_fields` interaction, untagged ordering) is only proven where a round-trip exists. Five of seven `PlaylistChange` variants, three of four `SettingValue` variants, and all of `RepeatMode`, `PlaylistSaveFormat`, `PlaylistEnqueueItem`, `ConfigAreaInfo`, `DecoderInfo`, `SessionSummary`, `EngineInfo`, `PluginInfo`, `PluginSource`, `TrackSelections`, `PlaylistSummary`, `SubtitleSettings`, `StateSnapshot` have no serde round-trip anywhere in the repo's tests.
What correct looks like: One parameterized round-trip test per part2 type (the file already has the pattern — `duration_is_never_a_sentinel`), or at minimum one per payload enum carrying part2 types.
Confidence: HIGH — Verification: read + rg (0 constructions in tests).

### F-002 [S4]
Where: crates/commands/src/catalog/types.rs:709-720 (pattern repeated at 724-735, 740-749)
Code:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase", default)]
pub struct TrackSelections {
```
What is wrong: The three snapshot block structs (`TrackSelections`, `PlaylistSummary`, `SubtitleSettings`) carry `#[serde(default)]`, so `{}` deserializes to an all-defaults block, while their container `StateSnapshot` (755-757) requires every field. Strictness is inconsistent within a single payload, and the file header (part1, lines 13-16) advertises "V1 closed shapes".
Why it matters: A host that forgets to fill `selections` silently renders every track unselected instead of failing loudly — a decode-drift class the "closed shapes" rule exists to prevent. All three carry unresolved `[VERIFY]` markers (706-708, 722-723, 737-739), so this is acknowledged-but-unreconciled.
What correct looks like: Drop `default` (required fields, like `StateSnapshot` itself), or document the tolerance as intentional per §4.4 reconnect semantics.
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/commands/src/catalog/types.rs:522-533
Code:
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum SettingValue {
```
What is wrong: `untagged` deserialization failures report serde's context-free "data did not match any variant of untagged enum SettingValue" — no field name, no accepted shapes. This is the payload type of `config.set-setting`, one of the 15 served commands, parsed on the `--control-stdio` machine wire.
Why it matters: A machine-mode client sending `"value": 5.0.1` (invalid number) or `{"v":1}` gets an error that names neither the command, the field, nor the four accepted shapes — contrary to the layer's own "typed, actionable errors" posture (dispatch.rs:162-173).
What correct looks like: Keep untagged for the wire but wrap with `#[serde(try_from = "serde_json::Value")]`-style conversion that names the accepted shapes, or use `serde(untagged)` with a custom `expecting` via a newtype.
Confidence: HIGH (failure mode is serde's documented behavior) / UNVERIFIED (no execution — the CLI tests that would show the message did not run in baseline).
Verification: read.

### F-004 [S4]
Where: crates/commands/src/catalog/types.rs:549-553
Code:
```rust
/// One decoder of the engine's registry (§2.12 `system.get-engine-info`
/// — hardware decode is reported here, truthfully).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DecoderInfo {
```
What is wrong: The doc comment promises "truthfully", but the only shipped producer of `DecoderInfo` (`apps/cli/src/host.rs::engine_info()`, reached via `dispatch.rs:139-141`) is a hard-coded literal listing `mjpeg, pcm_s16le` with `active: false` — under-reporting the real registry (baseline INDEX "Standing defects", CORRECTION v2; the h264 sw decoder exists but is not listed).
Why it matters: The type is honest; the comment's promise is not kept by the shipped path — a reader of types.rs would believe `system.get-engine-info` reflects the live registry.
What correct looks like: Either the host derives the list from the decoder registry, or this comment says "intended to be populated from the live registry; the first vertical's host currently hard-codes it".
Confidence: HIGH — Verification: read + baseline INDEX CORRECTION v2 (already-recorded standing defect; cross-referenced, not re-litigated).

## 4. Per-function table (per type; all are plain wire definitions — no methods in part2)
| Type | Lines | Verdict | Severity | Notes (runtime reachability) |
|---|---|---|---|---|
| `SeekKind` | 456-464 | REAL | — | kebab tags `command`/`frame-step`; round-tripped in event.rs:350-359 AND wire_forms.rs:90-96 (both baseline ok); emitted at runtime — `catalog_emission_ledger.rs:80-82` lists `TimingSeekCompleted` in the 14 EMITTED rows (owner 3.A1), keys pinned at ledger:309-315. |
| `RepeatMode` | 466-477 | REAL | F-001 | `#[default] Off`; tags `off`/`one`/`all`; only reachable via `playlist.set-repeat` — UNSERVED by CLI host. No round-trip test. |
| `PlaylistSaveFormat` | 479-490 | REAL | F-001 | tags `m3u8`/`xspf`/`json`; `playlist.save` UNSERVED. No round-trip test. |
| `PlaylistEnqueueItem` | 492-500 | REAL | F-001 | `deny_unknown_fields`; `playlist.enqueue-many` UNSERVED. No round-trip test. |
| `LogLevel` | 502-516 | REAL | F-001 | round-tripped in event.rs:501-507 (system.log). Runtime: NOT emitted — `catalog_emission_ledger.rs:161-164` records `SystemLog` as wired-by-later ("no logging bridge"); `system.set-log-level` UNSERVED. |
| `SettingValue` | 518-533 | REAL | F-003 | untagged; served at runtime (dispatch.rs:151-153); only `Bool` round-tripped (wire_forms.rs:130-136); no Eq/Hash (f64) — consistent with header; `[VERIFY]` marker on reconciliation with settings-schema. |
| `ConfigAreaInfo` | 535-547 | REAL | F-001 | `config.list-areas` UNSERVED. No round-trip test. |
| `DecoderInfo` | 549-564 | REAL | F-004 | constructed at runtime by the hard-coded `host::engine_info()`. |
| `SessionSummary` | 566-577 | REAL | F-001 | `#[serde(rename = "gen")]` — envelope convention; only inside `EngineInfo.sessions` (empty in the hard-coded literal). No round-trip test. |
| `EngineInfo` | 579-601 | REAL | — | served (`lumen engine-info`, baseline smoke output exists); `[VERIFY]` on `plugins` id-list rendering. |
| `PluginSource` | 603-611 | REAL | F-001 | `system.list-plugins` UNSERVED (owner: "the plugin registry is a Phase-6 card", dispatch.rs:201-202). |
| `PluginInfo` | 613-633 | REAL | F-001 | UNSERVED path; `[VERIFY]` on string `kind`. No round-trip test. |
| `PluginAction` | 635-644 | REAL | — | round-tripped in event.rs:508-515 (plugins.crashed); runtime: WIRED-BY-LATER ("sandboxed WASM host, Phase 7", ledger:157-160). |
| `PlaylistChange` | 646-687 | REAL | — | internally tagged `kind`; 2 of 7 variants round-tripped (event.rs:402-425, 495-499); `playlist.changed` WIRED-BY-LATER ("no playlist model in the engine, Phase 4", ledger:142-144). |
| `TrackChangeKind` | 689-704 | REAL | — | round-tripped in event.rs:477-483; `tracks.changed` is WIRED-BY-LATER per `catalog_emission_ledger.rs:129-132` ("no engine-side track-loss trigger exists in v0") — with a RECORDED DIVERGENCE: the engine-side map counts it emitted since 3.A3 (ledger module doc, lines 35-40). Runtime status ambiguous by the repo's own admission. |
| `TrackSelections` | 706-720 | REAL | F-002 | `default` tolerance; snapshot UNSERVED. |
| `PlaylistSummary` | 722-735 | REAL | F-002, F-001 | `default` tolerance; snapshot UNSERVED. No round-trip test. |
| `SubtitleSettings` | 737-749 | REAL | F-002, F-001 | `f32 scale` — correctly no Eq/Hash (documented); snapshot UNSERVED. |
| `StateSnapshot` | 751-786 | REAL | F-001 | all 14 fields required (no `default` — inconsistent with its blocks, see F-002); `system.get-state-snapshot` UNSERVED by the shipped host. No round-trip test. |
| tests (5) | 788-907 | REAL | F-001 | All target part1 types; see §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `seek_target_wire_form_matches_doc_example` | YES | Exact JSON string `{"kind":"relative","deltaMs":-10000}` + parse-back; any rename/tag change fails. (Part1 type.) |
| `duration_is_never_a_sentinel` | YES | All 3 variants pinned byte-for-byte. (Part1 type.) |
| `closed_shapes_reject_unknown_fields` | YES | Real negative control: `bogus` field must error with "unknown field". (Part1 type.) |
| `selector_variants_round_trip` | YES | All 3 selector tags pinned. (Part1 type.) |
| `playback_state_transitions_follow_section_3_3` | YES | 18 legal + 8 illegal + 8 self-transition edges against `can_transition_to` — a genuine table test; deleting a legal edge or adding an illegal one flips an assert. (Part1 method.) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "V1 closed shapes: plain structs carry `deny_unknown_fields`" | file header (types.rs:15, part1 range) | REAL for part2 structs (attrs verified at 494, 537, 552, 568, 583, 617, 710, 725, 741, 756) — but `default` on the three blocks weakens field-level closedness (F-002) | read |
| "hardware decode is reported here, truthfully" | types.rs:550 | FALSE for the shipped producer | host.rs `engine_info()` hard-codes the list (baseline CORRECTION v2 standing defect); see F-004. |
| `system.get-state-snapshot` "everything a reconnecting host needs" (§2.12) | doc 05 §2.12, mirrored at types.rs:751-753 | REAL as wire shape / UNREACHABLE as runtime behavior | Shape is schema-pinned (golden ok); command is not in `SERVED` (dispatch.rs:38-54) → typed refusal `unsupported-for-host`. |
| `TimingDurationKnown { duration: Timestamp(3000000) }` — µs wire convention | baseline mp4-smoke-open-session.txt | REAL | Baseline smoke shows `Timestamp` on the wire; header's µs convention (part1) honored in the shipped path. |

## 7. Dead / duplicated / unreachable code
- No dead definitions inside the catalog (every type referenced by command.rs/event.rs; `spec.rs` payload descriptors reference them too).
- Runtime-unreachable payload types (by the shipped host's typed-refusal design): `RepeatMode`, `PlaylistSaveFormat`, `PlaylistEnqueueItem`, `ConfigAreaInfo`, `PluginSource`, `PluginInfo`, `PlaylistChange`, `PluginAction` (event side), `TrackSelections`, `PlaylistSummary`, `SubtitleSettings`, `StateSnapshot`.
- The `apps/control-server` `SessionSummary` (logging.rs:165) is a DIFFERENT type from control-protocol, not this one — no duplication issue.

## 8. Dependencies & flags
- No new imports in part2 beyond part1's (`BTreeMap`, `PathBuf` used by part1's `MediaInfo`; `Generation`/`SessionId` from `crate::envelope` used at 571/576).
- No unsafe, no panics, no arithmetic — pure data definitions. No feature gates.

## 9. Open questions for the next agent
- The emission ledger self-reports a divergence with `crates/playback-engine/src/events.rs` on the `tracks.changed` row (emitted vs later, unresolved since 3.A3 — ledger module doc lines 35-40). Which file is true belongs to the playback-engine/events wave; it decides whether `TrackChangeKind` has a runtime producer.
- Whether `SettingValue`'s untagged `Number(f64)` should be `i64`-tolerant (settings like `log-level` are enums-as-strings today; numeric settings unverified against settings-schema ranges) — settings wave to reconcile the `[VERIFY]` at types.rs:519-521.

## Baseline-integrity note (affects evidence citations repo-wide)
`cargo test --workspace` (baseline #3) is a **fail-fast** run: it aborted at the first failing package (`desktop`, zero_alloc_transport, log tail line 2405-2412) and never reached any package sorted after `desktop` — including `lumen-cli` (rg for `lumen_cli`/`native_seek`/`sync_scenarios`/`pipeline_accounting` in the log → 0 hits; the package compiled at log line 16 but no test binary ran). Same for the all-features run (#4), aborted at `first-vertical`. So the INDEX's "1681 passed / 1 failed / 24 ignored" is a PARTIAL workspace run; the shipped binary's own test suite has no baseline execution evidence. REQUEST-ORCH-RUN: `cargo test --workspace --locked --no-fail-fast` (or at minimum `cargo test -p lumen-cli --locked`).
