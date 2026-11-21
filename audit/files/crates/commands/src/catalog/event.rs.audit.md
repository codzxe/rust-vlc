# AUDIT: crates/commands/src/catalog/event.rs
| Field | Value |
|---|---|
| Auditor | W02-A09 |
| Date | 2026-11-21 |
| Lines / functions | 536 / 7 (enum EventKind + is_completion_evidence + 5 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI `apps/cli/src/main.rs` → `apps/cli/src/run.rs:51` imports `commands::WireEvent`; `run.rs:720-735 print_event` matches `commands::EventKind` variants; `apps/cli/src/host.rs:86` `use commands::{… EventKind …}`; crate is a dep of `apps/cli/Cargo.toml:52`, `apps/control-server/Cargo.toml:41`, `apps/desktop-tauri/Cargo.toml:56`) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg -n "EventKind" crates apps` (users: 39 files; playback-engine maps it via `wire_event_id`); `sed -n '367,435p' docs/05-command-layer.md` (§3.2 table, 23 rows); `rg -n "non_exhaustive" docs/05-command-layer.md` (line 141 mandate); `rg -n "EventKind::DevicesVolumeAdded"` etc. (no production constructor); `sed -n '640,795p' crates/playback-engine/src/events.rs` (emission ledger, 9 rows `WiredLater`); `rg -n "catalog::event" audit/runs/baseline/test-workspace.log.txt` (all 5 tests ran, ok); `ls schemas/commands.schema.json ui/src/generated/commands.ts` (codegen artifacts exist, contain event kinds) |

## 1. One-paragraph verdict
This file is the real wire-event payload enum: 26 variants matching doc 05 §3.2's 23 rows plus the issue #42 additive trio, internally tagged `kind` with camelCase fields, round-trip tested byte-for-byte against the §5.1 example, and all 5 of its tests executed green in the baseline workspace run (test-workspace.log.txt:1317-1321). It is reachable from every shipped host. Its defect is contract-level: doc 05 §2.1/§6.2 mandates `#[non_exhaustive]` enums with unknown-`kind` degrade-to-logged-no-op semantics for event receivers, the module doc comment asserts that behavior exists, and neither the attribute nor any degradation path is implemented — the file's own test proves hard rejection, and the control-protocol client turns that into a connection-level `ProtocolError::BadEnvelope`. Plus two doc-comment lies of lesser degree.

## 2. What actually works (verified)
— All 5 unit tests executed and passed in baseline: `audit/runs/baseline/test-workspace.log.txt:1317-1321` (`catalog::event::tests::* … ok`, 5/5).
— Wire form matches doc 05 §5.1's example exactly: test `event_wire_form_matches_doc_example` asserts `{"seq":1031,…,"seekKind":"command"}` and doc 05:475 shows the identical JSON (compared by eye, both pasted above/verified with sed).
— 26 variants = 23 §3.2 rows (counted from `sed -n '373,403p' docs/05-command-layer.md`) + `video.backend-changed`, `devices.volume-added`, `devices.volume-removed`; header claim "23 rows + the issue #42 additive trio" is arithmetically true.
— `is_completion_evidence` matches doc 05:402's seven-event completion-evidence list exactly (SessionStateChanged, TimingSeekCompleted, TimingSeekFailed, PlaybackError, TracksChanged, TracksListReady, MediaInfoReady).
— Codegen artifacts derived from these types exist and contain the event kinds: `schemas/commands.schema.json` (5 matches for volume-added/screenshot-saved), `ui/src/generated/commands.ts` (1 match).

## 3. Findings

### F-001 [S2]
Where: crates/commands/src/catalog/event.rs:52-59 (and 386-396)
Code:
```rust
/// `#[non_exhaustive]`: unknown `kind` values received by older hosts
/// degrade to a logged no-op (§2.1, §6.2). The completion-evidence
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
pub enum EventKind {
```
What is wrong: The doc comment claims the enum is `#[non_exhaustive]` with unknown-`kind` → logged no-op degradation. There is no `#[non_exhaustive]` attribute on `EventKind` (line 57 has only the two shown attributes), no `#[serde(other)]`/fallback variant, and no degradation path anywhere in this crate. The file's own test (lines 386-396, `unknown_event_kinds_are_rejected`) proves the opposite behavior: `serde_json::from_str::<Event<EventKind>>(r#"{"seq":1,"kind":"nope.nope"}"#)` errors. Doc 05:141 mandates: "The enum is `#[non_exhaustive]`; hosts must match with a catch-all that maps unknown IDs to … a generic no-op with a logged warning (event receivers)" and §6.2 (doc 05:554-562) says "Allowed: … new event IDs … a v1 client that ignores everything it does not recognize must keep working against any v1.x engine."
Why it matters: A v1.x engine that adds an event ID (explicitly allowed by §6.2) makes every older host's `Event<EventKind>` deserialization fail. Blast radius confirmed: `crates/control-protocol/src/client.rs:275-283` wraps any `serde_json::from_str` failure in `ProtocolError::BadEnvelope` — a stream-level error, not a logged no-op. The forward-compatibility contract of the protocol is unimplemented and the in-code documentation asserts it is implemented.
What correct looks like: Either a `#[serde(other)]`-style fallback variant (e.g. `Unknown`) plus handling that logs-and-skips, or `#[non_exhaustive]` + a custom `Deserialize` that degrades unknown tags — and if the contract is deliberately not implemented, the doc comment must say so instead of claiming the attribute exists.
Confidence: HIGH — Verification: read (event.rs, docs/05-command-layer.md:141,554-562, client.rs:275-283) + baseline test run proving rejection semantics (test-workspace.log.txt:1321).

### F-002 [S3]
Where: crates/commands/src/catalog/event.rs:290-293
Code:
```rust
    /// `devices.volume-added` — a local volume or removable medium
    /// appeared (card 7.D1; the discovery crate's watcher emits it
    /// from real mount-table diffs, never fabricated). Not
    /// pipeline-scoped.
```
What is wrong: The comment states in present tense that "the discovery crate's watcher emits it". No production code constructs `EventKind::DevicesVolumeAdded` anywhere in `apps/` or `crates/` (rg over all Rust sources: only tests and the `wire_event_id` mapping in `crates/playback-engine/src/events.rs:839` reference it). The discovery crate emits its own internal `VolumeEvent` (`crates/device-discovery/src/events.rs:28-46`) with no bridge to this payload, and the engine's own emission ledger marks the row `WiredLater` (`crates/playback-engine/src/events.rs:775-780`).
Why it matters: A reader of the catalog (or an auditor trusting comments) believes volume events flow from the watcher to hosts today. They do not; the same overstated present tense afflicts `devices.volume-removed` (lines 305-311).
What correct looks like: "Planned emission: the discovery crate's watcher (card 7.D1, currently `WiredLater` in the engine emission ledger)". Present tense reserved for wired behavior.
Confidence: HIGH — Verification: read + rg exhaustively for constructors.

### F-003 [S4]
Where: crates/commands/src/catalog/event.rs:314-316
Code:
```rust
/// Convenience: the id of the (single) playlist item a change speaks
/// about, when it speaks about exactly one.
impl EventKind {
```
What is wrong: The impl-block doc comment describes a playlist-item-id convenience accessor. The impl block (lines 316-331) contains only `is_completion_evidence`. No such function exists anywhere in the crate.
Why it matters: Stale doc promising nonexistent API; classic copy-paste residue from a deleted/never-landed function.
What correct looks like: Delete the comment or move it onto the function that actually implements it (none exists).
Confidence: HIGH — Verification: read; `rg -n "playlist item|speaks about" crates/commands/src` finds only this site.

### F-004 [S4]
Where: crates/commands/src/catalog/event.rs:213-220 vs docs/05-command-layer.md:390-392 (drift noted; both sides quoted)
Code:
```rust
    /// `playlist.changed` — any playlist model mutation, command- or
    /// engine-initiated; model edits coalesced per 100 ms
    /// `[PROPOSAL]`. Not pipeline-scoped.
    #[serde(rename = "playlist.changed")]
    PlaylistChanged {
```
What is wrong: Doc 05 §3.2 sketches the payload as `{ kind: ItemsAdded{ids}/…/CurrentChanged{id} }` and `plugins.loaded` as `{ id, kind, version }` — unimplementable verbatim because `kind` is the envelope's internal tag. The code deliberately nests under `change` and renames to `pluginKind`/`volumeKind`, documenting each deviation in-code (`[VERIFY]` notes at lines 237-240, 300-302; test comment at 398-400). The committed doc 05 table still shows the impossible sketches.
Why it matters: The doc's Payload column is the spec third parties code against; it does not match the wire the schema and tests actually enforce. The code side is honest about it; the doc side is stale.
What correct looks like: Regenerate/patch doc 05 §3.2's payload sketches to the shipped wire forms (`change`, `pluginKind`, `volumeKind`).
Confidence: HIGH — Verification: read both sides; the in-file tests pin the real wire forms.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum EventKind` (26 variants) | 57-312 | REAL | S2 (F-001) | Internally tagged, camelCase fields; round-trip pinned by tests; no `#[non_exhaustive]`/`#[serde(other)]`; 9 variants (`SessionOpeningProgress`, `VideoScreenshotSaved`, `PlaylistChanged`, `ConfigSettingChanged`, `PluginsLoaded/Failed/Crashed`, `DevicesVolumeAdded/Removed`) have zero production constructors — engine ledger marks them `WiredLater` (honest, cross-checked) |
| `EventKind::is_completion_evidence` | 319-330 | REAL | — | Exact 7-element set = doc 05:402; exercised by `completion_evidence_set_matches_section_1_4` (baseline ok) |
| test `event_wire_form_matches_doc_example` | 343-364 | REAL | — | Exact-string assertion + round-trip; fails if tag/rename/camelCase drifts |
| test `sessionless_events_omit_optional_envelope_fields` | 366-384 | REAL | — | Pins `skip_serializing_if none` envelope behavior from the event side |
| test `unknown_event_kinds_are_rejected` | 386-396 | REAL | — | Honestly pins the *actual* (rejecting) semantics that contradict the module doc — evidence for F-001 |
| test `playlist_changed_nests_its_mutation` | 401-425 | REAL | — | Pins the `change` nesting (F-004 wire form) |
| test `completion_evidence_set_matches_section_1_4` | 431-535 | REAL | — | 12 samples (7 positive, 5 negative) round-tripped through the envelope then classified; would fail on any misclassification |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| event_wire_form_matches_doc_example | YES | Mutation: drop `rename_all_fields = "camelCase"` → `seekKind` becomes `seek_kind` → exact-string assert fails. |
| sessionless_events_omit_optional_envelope_fields | YES | If envelope stopped skipping `None` fields, JSON would contain `"session":null` → assert fails. |
| unknown_event_kinds_are_rejected | YES (and it did its job) | If unknown kinds were *accepted*, `unreachable!("unknown kind accepted")` fires. It correctly documents rejection — the module doc is what's wrong (F-001). |
| playlist_changed_nests_its_mutation | YES | If `PlaylistChanged` flattened instead of nesting, exact string mismatches. |
| completion_evidence_set_matches_section_1_4 | YES | Mutation: add `TimingTimeTick` to `is_completion_evidence` → the `(TimingTimeTick, false)` row asserts `false` and fails with the JSON in the message. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "23 rows + the issue #42 additive trio" of §3.2 | event.rs:47-50 | REAL | 26 variants counted; 23 rows counted in doc 05 §3.2 table |
| "`#[non_exhaustive]`: unknown `kind` … degrade to a logged no-op (§2.1, §6.2)" | event.rs:52-53 | FALSE | No attribute, no degrade path; own test proves rejection (F-001); doc 05:141 mandates it |
| "The enum is `#[non_exhaustive]`" (event IDs degrade to no-op with logged warning) | docs/05-command-layer.md:141 | FALSE (for this enum) | event.rs:57-59 attribute list; test at 386-396 |
| §6.2 "a v1 client that ignores everything it does not recognize must keep working against any v1.x engine" | docs/05-command-layer.md:561 | FALSE (events path) | client.rs:281-283 → `ProtocolError::BadEnvelope` on unknown kind |
| §5.1 event example JSON | docs/05-command-layer.md:475-476 | REAL | Test asserts byte-identical string (event.rs:356-359), ran green in baseline |
| Completion-evidence set = 7 events | docs/05-command-layer.md:402 | REAL | `is_completion_evidence` matches 1:1; tested |
| "the discovery crate's watcher emits it from real mount-table diffs" | event.rs:291-293 | FALSE (present tense) | No constructor of the catalog variant anywhere; engine ledger `WiredLater` (F-002) |
| §3.2 payload sketches `{ kind: … }` for playlist.changed / plugins.loaded | docs/05-command-layer.md:390-392 | FALSE (stale sketch) | Wire uses `change`/`pluginKind`; pinned by tests (F-004) |

## 7. Dead / duplicated / unreachable code
- Not unreachable (re-exported at `crates/commands/src/lib.rs` and consumed by CLI/control-server/desktop-tauri/web hosts).
- 9 of 26 variants have no production emitter (`WiredLater` in `crates/playback-engine/src/events.rs:652-784`): wire types only, tests + `wire_event_id` mapping reference them. Declared, not hidden — catalog rows as contract. No S3 filed for the variants themselves; the *comment* overstating one of them is F-002.
- `crates/device-discovery/src/volume.rs:65-88` duplicates a `VolumeKind` vocabulary (Fixed/Removable/Optical/Network) that exists verbatim in `crates/commands/src/catalog/types.rs:279-288` — two enums kept in sync by comment only (drift risk; owner: types.rs/device-discovery auditors).

## 8. Dependencies & flags
- `serde` 1.0.229 (derive), `serde_json` 1.0.154, `schemars` 1.2.3 — all exercised (round-trip tests, schema artifacts). No feature flags gate this file; compiled in every host. `rename_all_fields` requires serde ≥1.0.185 — satisfied, baseline build clean (INDEX.md #1).
- `#[lints] workspace = true` inherited (crates/commands/Cargo.toml) — no local `#[allow]` in this file except the sanctioned literal marker at line 503 (`lumen-allow-literal`, xtask lint-literals regime, INDEX.md #13 shows the regime is active).

## 9. Open questions for the next agent
- envelope.rs auditor (A10): confirm `Event<E>` flatten mechanics and that no custom `Deserialize` there softens unknown-kind rejection (F-001 blast radius).
- control-protocol auditors (A03/A04): does any server-side or client-side layer catch `BadEnvelope` on event frames and skip-and-log (which would partially rescue §6.2)? client.rs:281-283 suggests no.
- doc 05 owner: §3.2 payload column needs regenerating from the schema (F-004).
