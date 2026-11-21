# AUDIT: crates/playback-engine/src/event.rs (part2, lines 531–1061 of 1061)
> Chunk scope: part2 = lines 531–1061 (assignments.tsv LOC: part1 530 /
> **part2 531**). The `as_wire` method straddles the boundary (its match
> starts at 516 in A03's part1) — audited whole here because the arms are
> one exhaustive match; A03 owns the enum definition, `Clone`, and
> `PartialEq` (91–510). Contains: `as_wire` (516-615),
> `is_pipeline_scoped` (625-632), `EngineEvent` + `as_wire_event`
> (640-675), the stage/error bridges (`wire_stage`, `wire_error_code`,
> `fatal_details`, `seek_error_wire`, `ns_to_us`, `seek_failure_reason`),
> the payload bridges (`track_info_wire`, `recovered_damage_details`,
> `suspend_notice_details`, `suspend_notice_message`, `wire_track_kind`,
> `wire_track_change_kind`, `source_spec_wire`, `media_info_wire`,
> `wire_playback_state`), and `mod tests` (957-1061).
> Tooling note: bash output mangles `#[must_use]` (ANSI-strip artifact);
> quotes verified via the Read tool.

| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-21 |
| Lines / functions | 531 (of 1061) / 19 production units + 4 tests |
| File verdict | REAL (executed in the shipped open lane — the baseline `MediaInfoReady` payload IS this file's output) |
| Reachable from shipped entry point? | YES, EXECUTED. `as_wire`/`as_wire_event` run in every engine→wire translation the CLI prints: baseline `mp4-smoke-open-session.txt` line 9 shows `MediaInfoReady { info: MediaInfo { source: File { path: "/…/sample.mp4" } … Timestamp(3000000) … } }` — `media_info_wire`+`source_spec_wire`+`track_info_wire`+`ns_to_us` all executed there; `tracks 2 ready` is `as_wire`'s `TracksChanged`→`TracksListReady` arm; `state-changed` lines are the round-trip `state_changed_round_trips_through_the_wire_envelope` covers. Callers: apps/cli/src/run.rs (event printing), apps/web/src/host.rs:1317 (`track_info_wire`), apps/cli/src/host.rs:1434. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | Read tool over 510-1061 (full range) + 91-510 context (part1, A03); `sed -n '766,772p;624,632p;898,909p' event.rs` (quote verification); `rg -n "ns_to_us|nanos_i64" crates/playback-engine/src` (all 6 call sites + the two saturation sites); `sed -n '186,200p' crates/media-types/src/time.rs` + `Nanoseconds(pub u64)` (as_nanos bounds: u64 ns ≈584 years, `TimeError::Overflow` beyond); `rg -n "AudioDeviceSuspended" crates/playback-engine/src crates/playback-engine/tests` (emission in hotplug.rs:222/358, "terminal for its episode" at hotplug.rs:54); `sed -n '15,60p' hotplug.rs` (suspend semantics); `sed -n '9p' audit/runs/baseline/mp4-smoke-open-session.txt` (the executed MediaInfoReady payload — redacted path, `language: Some("voe")`, video/audio None); `grep -n "event::tests::" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (4/4 tests ok at 3761-3764) |

## 1. One-paragraph verdict
This is the engine→wire translation layer, and it runs in the shipped binary — the baseline MP4 open transcript is literally this file's output rendered. The mapping discipline is high: exhaustive matches, private kinds (`DamagedData`, `FirstFrame`) provably walled off the wire, error bridges that keep stage/code/context structure, and proposal-tagged honesty on every heuristic (`is_live: false`, `container: None`, `origin: Stream`). Two real defects: `ns_to_us` does unchecked `ns + 999` on values that the upstream type system has *deliberately* allowed to reach `i64::MAX` (media-types' `as_nanos` anticipates "adversarial" streams up to 584 years and `nanos_i64` saturates at 292 years — the +999 then overflows: debug panic, release wrap to a negative `Timestamp`), and `is_pipeline_scoped` exempts `AudioDeviceLost`/`AudioDeviceChanged`/`RendererBackendReport` from drain-staleness but NOT `AudioDeviceSuspended` — the one device-row fact that tells a non-draining host playback is stuck — contradicting both the sibling design and the events.rs part1 doc's stated intent that "the audio device rows" speak for the environment and must survive a superseding mint. Hygiene: `source_spec_wire`'s `trim_start_matches("file://")` is dead code for the actual redacted Display form (proven by the transcript's `/…/sample.mp4` — no scheme prefix), and the stage-bridge test pins only 3 of 14 mappings.

## 2. What actually works (verified)
— `as_wire` exhaustiveness + private-kind wall — verified: `DamagedData`/`FirstFrame` → `None` (545-546), pinned by `private_kinds_never_reach_the_wire` (baseline 3762); every other arm maps to its doc-05 catalog kind; the `Fatal` arm's no-doubled-bracket message (`message_summary`, 541) matches the F4 fix claim.
— `ns_to_us` rounding — verified for the sane range: `(ns+999)/1000` rounds up; the baseline `Timestamp(3000000)` (3 s exactly) is its executed output.
— `is_pipeline_scoped` — verified as the exact basis of the drain exemption consumed by `is_stale_at_drain` (events.rs:499-504, my part1 file); Lost/Changed/RendererBackendReport survive intra-epoch mints.
— `wire_stage` judgment calls — verified against the doc comment: 14 L0 stages → 13 wire stages, the six collapses each named; compile-exhaustive.
— `seek_error_wire` — verified: policy refusals (NotSeekable/OutOfRange) surface as Control/Unsupported with stable `reason` keys; carried failures keep their own stage/kind; the D4 rollback position rides `position_us`.
— `track_info_wire` honesty — verified: `video: None`/`audio: None`/`profile: None` because the L0 `Track` type HAS no such fields (struct at media-types; test fixtures confirm) — not a stub hiding data, a type boundary honestly reported, `[PROPOSAL]`-tagged for 2.B2.
— `suspend_notice_details`/`suspend_notice_message` — verified: the "byte-identical to the guard's direct-drive" claim is the contract suite's pinned join (hotplug.rs:222-224, tests/hotplug.rs passing at baseline 3897-3906).
— `media_info_wire` — EXECUTED: the baseline transcript payload matches field-for-field (redacted File source, `duration: Known { Timestamp(3000000) }`, `seekable: true`, empty metadata/chapters, both tracks with selection truth).
— `source_spec_wire` privacy doctrine — verified: Display IS the redacted form; the wire File path carries the ellipsis form (`/…/sample.mp4` in the transcript); the raw path never re-derives (the CLI's own get-info carries the original — host.rs).

## 3. Findings

### F-001 [S3]
Where: crates/playback-engine/src/event.rs:770-772 (the arithmetic); saturation feed at crates/playback-engine/src/seek.rs:330-331; bounds source at crates/media-types/src/time.rs:190-192
Code:
```rust
fn ns_to_us(ns: i64) -> i64 {
    (ns + 999) / 1_000
}
```
and the two ends of the pipe that make `ns == i64::MAX` reachable (seek.rs:330-331):
```rust
pub(crate) fn nanos_i64(ns: Nanoseconds) -> i64 {
    i64::try_from(ns.0).unwrap_or(i64::MAX)
}
```
and the upstream contract (media-types time.rs:190-192): "`TimeError::Overflow` when the value is negative or exceeds `u64` nanoseconds (≈ 584 years — generous, but streams are adversarial)".
What is wrong: `ns + 999` is unchecked. The pipeline's duration/position sources run through `nanos_i64`, which SATURATES any value above i64::MAX ns (≈292 years) to exactly `i64::MAX` — and media-types explicitly admits hostile streams up to 584 years before erroring. So a crafted container declaring a duration (or a track span end, or a pts landing) in the 292–584-year window produces `duration_ns == i64::MAX`, and `ns_to_us(i64::MAX)` = `i64::MAX + 999` overflows: panic in debug builds, wrap to a large negative in release — `TimingDurationKnown { duration: Timestamp(NEGATIVE) }` on the wire. The comment's safety argument ("positions are non-negative by construction") addresses the sign, not the magnitude; the magnitude bound was decided one layer down to be *adversarially wide*, and this bridge undoes it with a bare `+`.
Why it matters: The checklist's "unchecked arithmetic on parsed box/sample sizes" class, on the exact path (wire Timestamp) every host renders. Debug-build panic on a crafted file; release-build garbage. The fuzz registry exists because the project claims hostile-input robustness; this is a one-line hole in it. Blast radius: one wrong wire number or one debug panic — no memory unsafety.
What correct looks like: `ns.div_ceil(1_000)` (i64 stable `div_ceil` handles i64::MAX exactly: 9_223_372_036_854_775_807 → 9_223_372_036_854_776 µs, no overflow), or `(ns / 1_000) + u64::from(ns % 1_000 != 0) as i64`. Add a test at `ns = i64::MAX`.
Confidence: HIGH on the arithmetic and the saturation feed (both traced); MEDIUM on end-to-end craftability (which container parser actually passes a >292-year value through `as_nanos`→`nanos_i64` without its own clamp is the demux wave's territory — UNVERIFIED here; mp4 mvhd 64-bit duration × timescale 1 and mkv float Duration are the candidate feeds).

### F-002 [S3]
Where: crates/playback-engine/src/event.rs:625-632 (the exemption set); contradicted intent at crates/playback-engine/src/events.rs:491-497 (part1)
Code:
```rust
    pub fn is_pipeline_scoped(&self) -> bool {
        !matches!(
            self,
            EngineEventKind::AudioDeviceLost { .. }
                | EngineEventKind::AudioDeviceChanged { .. }
                | EngineEventKind::RendererBackendReport { .. }
        )
    }
```
and the intent it breaks (events.rs:491-495, the drain exemption's doc): "the session-scoped kinds (card 7.C1: the audio device rows and the renderer backend report …) speak for the session's environment, not for a pipeline generation, so a superseding mint (a seek, a track switch) must not swallow them."
What is wrong: `AudioDeviceSuspended` — the non-fatal `playback.error` the engine emits exactly once when the reopen window expires, telling the host playback is suspended at a remembered position (hotplug.rs:29-32, 358) — is NOT in the exemption set, so it is pipeline-scoped: an event stamped with generation G that is still undelivered when any later mint (a seek, a switch) advances the live pair is suppressed at drain by `is_stale_at_drain` (events.rs:478-507). Its two sibling device rows survive; the suspend notice — the one fact that says playback is STUCK — dies. The doc's phrase "the audio device rows" reads as the family; the code exempts only the two rows that have their own catalog ids (lost/changed), leaving the notice (which surfaces as a `playback.error` flavor) on the wrong side of the line.
Why it matters: A non-draining host (the exact host class doc 05 §4.2's staleness rule exists for) that loses its audio device and then seeks: it receives `audio.device-lost` (exempt), the suspend's `state-changed → paused` (pipeline-scoped, suppressed), and never the suspend notice — the UI shows a seek that "completed" over a session that is actually suspended, with no error anywhere. Limited blast radius (drain-per-poll hosts never observe staleness at all), hence S3 not S2.
What correct looks like: Add `EngineEventKind::AudioDeviceSuspended { .. }` to the exemption set (it is the same environment-fact class as its siblings — the suspend state outlives any pipeline mint by design, hotplug.rs:54 "terminal for its episode"), and add a hotplug+seek interleaving test asserting the notice survives the mint.
Confidence: HIGH — Verification: read (exemption set, drain rule, hotplug semantics all traced; no test covers suspend-notice staleness — tests/hotplug.rs exercises payloads, not drain interleavings).

### F-003 [S4]
Where: crates/playback-engine/src/event.rs:899-909
Code:
```rust
pub fn source_spec_wire(source: &SourceUri) -> commands::catalog::types::SourceSpec {
    use commands::catalog::types::SourceSpec;
    let redacted = source.to_string();
    if source.scheme().as_str() == "file" {
        SourceSpec::File {
            path: PathBuf::from(redacted.trim_start_matches("file://")),
        }
    } else {
        SourceSpec::Url { url: redacted }
    }
}
```
What is wrong: Two things. (1) `trim_start_matches("file://")` is dead code for every reachable input: the redacted Display form of a file URI is `/…/basename` (no scheme prefix — proven by the executed baseline payload `File { path: "/…/sample.mp4" }`), so the trim never fires; it reads as if it strips a prefix that is not there. (2) The resulting `File { path }` is not a filesystem path at all — it contains a literal `…` — yet it is typed as `SourceSpec::File { path: PathBuf }`, the same shape hosts use to OPEN sources (`session.open` takes a `SourceSpec`). A host that round-trips `MediaInfo.source` into an open call gets a garbage path. The privacy doctrine is documented and `[PROPOSAL]`-tagged (892-897), so this is a trap note, not a doctrine dispute.
Why it matters: Type says "path", content is a display form; the API shape invites the misuse the doc warns about only in prose. The dead trim suggests the author expected a `file://`-prefixed form that the redactor never produces — a latent misunderstanding, harmless today.
What correct looks like: Drop the trim (or assert the form it strips), and consider a dedicated `SourceSpec::Redacted`-style note or a doc line on `SourceSpec::File` that engine-emitted infos carry the redacted form; at minimum rename the local to `display_form` so the trim's deadness is obvious.
Confidence: HIGH — Verification: read + the executed baseline payload (the path in the transcript has no `file://` prefix — the trim provably did nothing).

### F-004 [S4]
Where: crates/playback-engine/src/event.rs:1032-1060 (`stage_bridge_covers_every_l0_stage`)
Code:
```rust
        for stage in stages {
            let _ = wire_stage(stage); // must answer, never panic
        }
        assert_eq!(wire_stage(PipelineStage::Open), commands::Stage::Input);
```
What is wrong: The test's name promises the bridge is covered; 11 of the 14 stages get only "must answer, never panic" — a wrong mapping (e.g., `Clock` → `Renderer` instead of `Control`) passes. Only Open→Input, Present→Renderer, Mix→AudioOutput are pinned. The six documented judgment calls (677-687) each deserve their assert; three of them have none.
Why it matters: The judgment calls are exactly where a future refactor silently changes wire semantics; the exhaustiveness guarantee covers "answers", not "answers correctly".
What correct looks like: One `assert_eq!` per stage (14 lines), or a table-driven zip of the expected pairs.
Confidence: HIGH — Verification: read (test body vs. the 14-stage array).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `EngineEventKind::as_wire` (whole 516-615; head 516-530 in A03's part1) | 531-615 | REAL | — | Exhaustive; private kinds walled (545-546, test-pinned); `RecoveredDamage`/`AudioDeviceSuspended` map to non-fatal `PlaybackError` flavors with structured details; `TrackSelectionChanged`→`TracksChanged` with `selected.map(...)` (the None = deselect survives the wire). EXECUTED in the baseline transcripts. |
| `EngineEventKind::is_pipeline_scoped` | 625-632 | REAL (defective) | S3 (F-002) | The exemption set's membership error; consumed by the drain rule + `EngineEvent::is_pipeline_scoped`. |
| `EngineEvent` (struct) | 640-652 | REAL | — | Correlation envelope per doc 05 §3.1; `seq`/`session`/`generation`/`origin`/`kind` all pub — envelope integrity is the emitter's job (session.rs emit_raw/emit_terminal, audited in part4). |
| `EngineEvent::is_pipeline_scoped` | 654-660 | REAL | — | Delegation. |
| `EngineEvent::as_wire_event` | 662-675 | REAL | — | Envelope round-trip; `Some(session)`/`Some(generation)` always present for engine events (sessionless forms are the commands-crate's own concern). Test-pinned. |
| `wire_stage` | 689-706 | REAL | S4 (F-004 test) | Compile-exhaustive; 6 documented judgment calls; 3 pinned by test. |
| `wire_error_code` | 712-723 | REAL | — | Seven canonical classes, one-to-one — matches the ErrorKind enum exactly. |
| `fatal_details` | 727-732 | REAL | — | context + stage keys; BTreeMap (stable order). Test-pinned (963-991). |
| `seek_error_wire` | 741-765 | REAL | — | Policy-vs-carried split traced; `position_us` detail = the D4 rollback point. |
| `ns_to_us` | 770-772 | REAL (defective) | S3 (F-001) | Unchecked +999 at the saturation boundary; sane-range rounding correct; EXECUTED (baseline Timestamp(3000000)). |
| `seek_failure_reason` | 775-782 | REAL | — | Stable keys. |
| `track_info_wire` | 796-819 | REAL | — | EXECUTED; honest None-blocks (L0 type lacks the fields — verified); codec string → description; `is_default` from flags. Carries the demux-side `language: Some("voe")` garbage through verbatim (known standing defect, INDEX correction v2 — demux wave's to trace, not this bridge's fault: it maps what it is given). |
| `recovered_damage_details` | 825-829 | REAL | — | count key. |
| `suspend_notice_details` | 835-841 | REAL | — | context/device/reopen_attempts — the guard's own keys, contract-pinned join. |
| `suspend_notice_message` | 847-864 | REAL | — | Two forms (with/without last_error); display-safe; the "same sentence by construction" claim is the hotplug contract suite's pinned join. |
| `wire_track_kind` | 868-875 | REAL | — | 1:1. |
| `wire_track_change_kind` | 882-890 | REAL | — | 1:1 incl. Data; secondary-audio arm noted as unminted (doc-cited). |
| `source_spec_wire` | 899-909 | REAL | S4 (F-003) | EXECUTED; dead trim + not-a-path trap (F-003). |
| `media_info_wire` | 920-947 | REAL | — | EXECUTED (baseline payload field-match); duration None→Unknown honest; `is_live: false` proposal-tagged. |
| `wire_playback_state` | 952-954 | REAL | — | Named delegation. |

## 5. Test honesty (tests in range, 957-1061)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fatal_maps_to_wire_playback_error_with_details` (963) | YES | Pins stage, code, fatal flag, context detail, message substring — a mapping regression fails it. Baseline ok (3761). |
| `private_kinds_never_reach_the_wire` (994) | YES | Both None arms pinned. Baseline ok (3762). |
| `state_changed_round_trips_through_the_wire_envelope` (1004) | YES | Full envelope field-equality. Baseline ok (3764). |
| `stage_bridge_covers_every_l0_stage` (1033) | PARTIALLY | 3/14 mappings pinned (F-004); "never panic" is the only guarantee for the other 11. |
| Coverage holes | — | No test: `ns_to_us` at the saturation boundary (F-001); suspend-notice drain survival across a mint (F-002); the redacted-path shape of `source_spec_wire` (only the executed transcript shows it); `seek_error_wire`'s reason keys (only indirectly via seek_flow suites). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`None` for engine-private events that must not leak onto the wire" | event.rs:513-514 | REAL | 545-546 + test 994 |
| "Positions are non-negative by construction (targets clamp … and landings are keyframe positions)" | event.rs:768-769 | SHALLOW | True for sign; says nothing about magnitude, where the 292–584-year saturation window lives (F-001) |
| "the audio device rows … speak for the session's environment … a superseding mint must not swallow them" | events.rs:491-495 (drain doc) | SHALLOW | Lost/Changed honored; `AudioDeviceSuspended` — an audio device row in any natural reading — is swallowed (F-002) |
| "`file` URIs surface as `File` specs over the redacted path" | event.rs:894-897 | REAL (with trap) | Baseline payload `/…/sample.mp4`; the path is a display form, not a path (F-003) |
| "the engine speaks the URI's redacted display form (V8 — it never re-derives the raw path …)" | event.rs:893-896 | REAL | Display = redacted; no raw accessor in this bridge; the CLI's get-info carries the original (host side) |
| "The fourteen L0 stages collapse onto the wire's thirteen with judgment calls, each `[PROPOSAL]`" | event.rs:679-687 | REAL | All six collapses present and proposal-tagged |
| "`seekable` is the demuxer's declared SeekSupport (non-None), `is_live` is false (no live sources exist in the engine yet)" | event.rs:913-916 | REAL | media_info_wire fields match; no live-source path exists in the engine (verified by the shipped lanes: file only) |
| "`video`/`audio` detail blocks are `None` until the demux implementation (2.B2) reports width/height/rate/channels" | event.rs:791-793 | REAL | The L0 Track type has no such fields — the bridge cannot lie |

## 7. Dead / duplicated / unreachable code
- Dead: the `trim_start_matches("file://")` in `source_spec_wire` (F-003) — unreachable for every producible Display form.
- Duplication: `wire_track_kind` vs `wire_track_change_kind` are parallel 1:1 matches over the same four MediaKind values (the wire types differ, so the duplication is forced by the commands crate's shapes — cosmetic).

## 8. Dependencies & flags
`commands` (catalog types/envelope), `media_types` (Track/SourceUri/Stamp/errors), `std::collections::BTreeMap`, `std::path::PathBuf`. No cfg, no unsafe, no feature gates in range. The only arithmetic in the range is `ns_to_us` (F-001) — everything else is field mapping.

## 9. Open questions for the next agent
1. Demux wave: does any container parser actually deliver a >292-year duration/pts into `nanos_i64` saturation (mp4 mvhd 64-bit × timescale 1, mkv float Duration)? F-001's end-to-end severity rests on that feed; a fixture with a 300-year duration is the test.
2. Hotplug/7.C1 owner: confirm whether `AudioDeviceSuspended`'s pipeline-scoping was a deliberate choice (it surfaces as `playback.error`, not its own catalog row) — if deliberate, the events.rs:491-495 doc needs the narrowing; if not, F-002's one-line fix.
3. Commands-crate auditor: `SourceSpec::File { path }` doubles as the open-command input and the info-event output with different content disciplines (raw vs redacted) — worth a type-level distinction upstream (F-003's structural half).
