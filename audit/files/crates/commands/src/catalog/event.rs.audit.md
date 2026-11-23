# AUDIT: crates/commands/src/catalog/event.rs
| Field | Value |
|---|---|
| Auditor | W02-A08 |
| Date | 2026-11-23 |
| Lines / functions | 536 / 7 (enum 26 variants, 1 impl method, 5 test fns, 1 test-loop) |
| File verdict | REAL (wire catalog, tests executed; 11/26 variants never emitted by design; 2 false doc claims; 1 dead method) |
| Reachable from shipped entry point? | YES — engine `EngineEvent::as_wire_event` (crates/playback-engine/src/event.rs:665, compiler-exhaustive `as_wire` at 516-615) → CLI host `drain_outputs` (apps/cli/src/host.rs:678 `self.events.push(wire)`) → `print_event` (apps/cli/src/run.rs:720-740) matches these variants; baseline mp4-smoke.md shows the live lines |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `./target/debug/deps/commands-cd658e8b0c406ea7 catalog::event` → 5 passed; `./target/debug/deps/catalog_emission_ledger-f20954af8148cb44` → 2 passed; `./target/debug/deps/catalog_shape-a6be788a761b3aed` → 4 passed; rg construction-site census; baseline mp4-smoke.md cited |

## 1. One-paragraph verdict
This is the real event half of the catalog: 26 internally-tagged variants, cross-pinned to `EventId::ALL` and the committed schema by the catalog_shape tests (ran, pass), with 5/5 unit tests passing (ran). The events the CLI prints provably originate here — `print_event` (run.rs:720) matches `EventKind::SessionStateChanged/SessionBuffering/TracksListReady/PlaybackEndOfMedia/PlaybackError/TimingTimeTick` and the baseline MP4 smoke printed exactly those lines plus Debug-fallback dumps for `TimingDurationKnown` and `MediaInfoReady`. The emission story is honestly bookkept (catalog_emission_ledger, ran: 14 emitted / 12 later) but that means 11 of 26 variants are declared-and-never-emitted in production, one doc claim about `#[non_exhaustive]` + graceful degradation is false on both counts (attribute absent; the file's own test proves hard rejection), and the one method on the type (`is_completion_evidence`) is dead code wired to nothing while the CLI re-implements a subset of it.

## 2. What actually works (verified)
— `./target/debug/deps/commands-cd658e8b0c406ea7 catalog::event` → `test result: ok. 5 passed` (§5.1 wire example byte-pinned, sessionless omission, unknown-kind rejection, playlist nesting, completion-evidence set).
— `./target/debug/deps/catalog_shape-a6be788a761b3aed` → 4 passed: 26 variants ↔ 26 `EventId` ↔ 26 schema tags, no orphans/missing.
— `./target/debug/deps/catalog_emission_ledger-f20954af8148cb44` → 2 passed: 14+12=26 partition with owner notes, including the recorded tracks.changed divergence.
— Baseline `audit/runs/baseline/mp4-smoke.md` (cmd 1): live output `event: state-changed Idle → Opening`, `event: tracks 2 ready`, `event: buffering 0.00`, `event: MediaInfoReady { info: MediaInfo { … } }` — these strings come from run.rs:720-740 matching this file's variants.

## 3. Findings
F-001 [S3]
Where: crates/commands/src/catalog/event.rs:52-58
Code:
```
/// `#[non_exhaustive]`: unknown `kind` values received by older hosts
/// degrade to a logged no-op (§2.1, §6.2). The completion-evidence
/// events for class-A commands (§1.4 C3) are `session.state-changed`,
```
What is wrong: The `#[non_exhaustive]` attribute does not exist on `EventKind` (L57-58 carry only derive + serde attrs; rg `non_exhaustive` in this file = 1, the claim itself). And "degrade to a logged no-op" is contradicted by this file's own test 30 lines down — `unknown_event_kinds_are_rejected` (L386-396) proves an unknown `kind` fails the whole `Event<EventKind>` parse. Why it matters: §6.2's version-skew scenario (newer engine → older host) is documented as graceful but is actually a hard parse error of the event stream; any host deserializing events from JSON drops the event (or worse, the connection) instead of logging it. What correct looks like: add the attribute plus an `Unknown` catch-all carrying the raw tag (serde `#[serde(other)]` cannot capture payloads, so a manual Deserialize or untagged fallback), or delete the claim. Confidence: HIGH — Verification: read + ran (the rejection test).
F-002 [S3]
Where: crates/commands/src/catalog/event.rs:70-76, 168-174, 189-199, 213-233, 234-270, 271-282, 290-311 (the never-emitted variants)
Code:
```
    SessionOpeningProgress {
        /// `0.0..=1.0`.
        progress: f32,
    },
```
What is wrong: 11 of 26 variants are never constructed by any production code: SessionOpeningProgress, PlaybackRateChanged, VideoScreenshotSaved, PlaylistChanged, ConfigSettingChanged, PluginsLoaded, PluginsFailed, PluginsCrashed, SystemLog, DevicesVolumeAdded, DevicesVolumeRemoved (rg over apps/ + crates/: their only construction sites are events.rs's test fixtures at crates/playback-engine/src/events.rs:2035-2065, inside `#[cfg(test)]` since L1544). The engine's `as_wire` mapping (event.rs:516-615) produces only 15 kinds (14 per the catalog ledger + the recorded tracks.changed divergence). `devices.volume-*` has discovery-side internal events (crates/device-discovery/src/events.rs:31 `VolumeEvent`) but no code converts them to `EventKind` — the relay is unwired. Why it matters: catalog rows exist, schema/TS artifacts export them to UI clients, and any UI building on the schema will wait for events that never fire. Mitigation: the catalog_emission_ledger test (ran: pass) partitions exactly this and names owning cards — the gap is declared, not hidden, which is why this is S3 not S0. Confidence: HIGH — Verification: rg + read + ran (ledger test).
F-003 [S3]
Where: crates/commands/src/catalog/event.rs:316-331
Code:
```
impl EventKind {
    /// Whether this event kind is one of the §1.4 C3 completion-evidence
    /// events for class-A commands.
    pub fn is_completion_evidence(&self) -> bool {
```
What is wrong: Dead code. The only caller in the entire tree is its own test (event.rs:528 `back.kind.is_completion_evidence()`). No host, engine, or control-server consults it; the CLI re-implements a subset locally (apps/cli/src/host.rs:1424-1427 `is_seek_terminal` matches `EventKind::TimingSeekCompleted | TimingSeekFailed` directly), and the control-protocol server does its own request correlation. The §1.4 C3 truthful-UI classification this method encodes is exactly what a generic host would need — it exists and is wired to nothing. Confidence: HIGH — Verification: rg across apps/, crates/, tests/, tools/ (2 hits: definition + own test).
F-004 [S4]
Where: crates/commands/src/catalog/event.rs:77-83 (also 84-92, 168-174)
Code:
```
    /// `session.buffering` — enter/while/exit buffering, ≤ 5 Hz; opens
    /// with 0.0 and closes with 1.0.
    #[serde(rename = "session.buffering")]
    SessionBuffering {
        /// `0.0..=1.0`.
        progress: f32,
    },
```
What is wrong: `0.0..=1.0` is a comment, not a constraint — same unenforced-range class as command.rs F-002 (schema emits bare `format: float`, no minimum/maximum); the receiving side cannot rely on the documented range. The rate claims (≤ 5 Hz, ≤ 1 Hz coalescing) are engine-side behavior, unverifiable from this file — UNVERIFIED here (owned by playback-engine's emitter). Confidence: HIGH for the range half (schema read), UNVERIFIED for frequency halves. What correct looks like: range newtypes or schemars bounds so the exported schema carries the contract.
F-005 [S4]
Where: crates/commands/src/catalog/event.rs:189-199
Code:
```
    /// `video.screenshot-saved` — the screenshot was written; completion
    /// evidence for `video.take-screenshot`. Not pipeline-scoped.
```
What is wrong: `video.take-screenshot` is itself refused by every shipped host (command.rs audit F-005 set), so its completion-evidence event is a doubly-dead surface: no command produces it, no emitter emits it. Consistent with the ledger's honest "later" bucket; listed here so the feature matrix does not count screenshot as working. Confidence: HIGH — Verification: rg (no construction outside tests) + command.rs audit.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum EventKind` (26 variants) | 59-312 | REAL (15/26 produced) / UNREACHABLE (11/26) | S3 | Wire contract; 26↔id↔schema pinned (ran). 11 never emitted in production (F-002). |
| `EventKind::is_completion_evidence` | 316-331 | UNREACHABLE | S3 | Only caller is its own test (F-003). |
| `tests::event_wire_form_matches_doc_example` | 343-364 | REAL | — | Byte-pins the §5.1 event JSON. Ran: pass. |
| `tests::sessionless_events_omit_optional_envelope_fields` | 366-384 | REAL | — | Ran: pass. |
| `tests::unknown_event_kinds_are_rejected` | 386-396 | REAL | — | Ran: pass; also the evidence that F-001's claim is false. |
| `tests::playlist_changed_nests_its_mutation` | 401-425 | REAL | — | Pins the nested `change` discriminator. Ran: pass. |
| `tests::completion_evidence_set_matches_section_1_4` | 431-535 | REAL | — | Builds 12 kinds, round-trips, classifies. Ran: pass. Honest (would fail if the matches! set changed). |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| event_wire_form_matches_doc_example | YES | Exact-string JSON; any tag/field rename breaks it. |
| sessionless_events_omit_optional_envelope_fields | YES | Exact string omits session/gen/origin. |
| unknown_event_kinds_are_rejected | YES (weakly) | Asserts rejection but only checks the word "variant" in the error. |
| playlist_changed_nests_its_mutation | YES | Exact nested JSON. |
| completion_evidence_set_matches_section_1_4 | YES | Round-trips 12 kinds and asserts classification both ways. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`#[non_exhaustive]`: unknown `kind` values … degrade to a logged no-op" | event.rs:52-53 | FALSE | Attribute absent; own test L386-396 proves hard rejection. |
| "one variant per §3.2 table row, payloads verbatim" | event.rs:4-5 | REAL-UNPROVEN | 26↔EventId↔schema pinned (ran); doc 05 §3.2 prose not diffed line-by-line by me. |
| `origin`/`gen` appear exactly once (flattened envelope) | event.rs:16-21 | REAL | Wire test L355-359 pins `"gen":4,"origin":42` beside payload keys. |
| `devices.volume-added` "emitted from real mount-table diffs, never fabricated" (ledger row) | catalog_emission_ledger.rs | SHALLOW | The discovery crate's `VolumeEvent` exists (device-discovery/src/events.rs:31) but no production code converts it to `EventKind::DevicesVolumeAdded` — the relay is unwired; "crate-side emission" refers to the internal enum, not the catalog event. |
| CLI prints `state-changed`, `buffering`, `MediaInfoReady` from this catalog | run.rs:720-740 | REAL | Baseline mp4-smoke.md live output lines match the match-arms; MediaInfoReady prints via the `other => {other:?}` Debug fallback. |

## 7. Dead / duplicated / unreachable code
- `is_completion_evidence` — dead method (F-003).
- 11 never-emitted variants (F-002 list) — declared forward contract, ledger-bookkept.
- CLI's `is_seek_terminal` (host.rs:1424-1427) duplicates a subset of the dead method — the classification logic exists twice, once dead once local.

## 8. Dependencies & flags
- serde/schemars only; no feature gates; no unsafe; no allow attrs. Part of the release lane (baseline exit 0).

## 9. Open questions for the next agent
- Does the control-server event subscription path (`subscribe` frames) filter by `EventId` and forward only emitted kinds, or would it forward never-emitted kinds if the engine ever produced them? (See W02 dispatch audit.)
- apps/web parses events from JSON — does any code path there implement the "logged no-op" degradation §6.2 promises (F-001)? If not, the version-skew story is broken there too.
- `EngineEventKind::TracksChanged` maps to catalog `TracksListReady` while `TrackSelectionChanged` maps to catalog `TracksChanged` (playback-engine/src/event.rs:522-530, 582-588) — name-shift is intentional per the ledger divergence note, but the desktop/web UI consumption of `tracks.list-ready` vs `tracks.changed` deserves a look when auditing the UI bridge.
