# AUDIT: crates/playback-engine/src/event.rs (part1, lines 1–530 of 1061)
> Chunk scope: part1 = lines 1–530 — module docs (the doc 03→doc 05 wire-mapping
> contract), imports, the 17-variant `EngineEventKind` enum (91-284), the manual
> `Clone` (286-380, source-chain-dropping), the manual `PartialEq` (382-510), and
> the head of `as_wire` (512-530: StateChanged/TracksChanged/Buffering/EndOfMedia
> arms). `as_wire`'s remaining arms, `is_pipeline_scoped`, `EngineEvent`, the wire
> bridges (`wire_stage`, `wire_error_code`, `fatal_details`, `track_info_wire`,
> …) and the in-file `mod tests` (956-1061) are W03-A09's part2 — `as_wire`
> straddles the boundary at its TracksChanged arm's close; A09 is flagged in §9.
> File name carries `.part1`.

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-21 |
| Lines / functions | 530 (of 1061) / 5 units (enum with 17 variants, manual Clone, manual PartialEq, `as_wire` head) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/host.rs` → `playback_engine::Engine` → `session.rs` `emit` → `EngineEventKind` → `as_wire` at the CLI's event-printing edge; baseline `mp4-smoke-open-session.txt` shows the mapped wire lines verbatim: `event: state-changed Idle → Opening`, `event: tracks 2 ready`, `event: TimingDurationKnown { duration: Timestamp(3000000) }`, `event: MediaInfoReady { … }`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read of lines 1–540 + test bodies 956-1035 (part2 context); `sed -n` quote verification (109-115, 512-531); `rg` construction sites of `TracksChanged`/`selected` (session.rs:1659-1661, 2331-2337, pipeline.rs:1022-1024, 1060 — length invariant traced); read `media-types/src/error.rs:223-282` (`ErrorContext` = code-only; `EngineError` = stage+kind+context+source); baseline cites: `orchrun-test-workspace-nofailfast.log.txt:3761-3763` (`event::tests::{fatal_maps_to_wire_playback_error_with_details, private_kinds_never_reach_the_wire, stage_bridge_covers_every_l0_stage}` all ok) and `mp4-smoke-open-session.txt` (wire lines from these mappings, executed). |

## 1. One-paragraph verdict
This chunk is the engine-side event vocabulary and the first half of its wire reconciliation, and it is disciplined code: 17 variants each documented with its doc 05 wire row (or its deliberate engine-private status), a manual `Clone` whose one lossy case (`Fatal`'s boxed `source` chain) is documented at the type and faithfully implemented, and a manual `PartialEq` that compares exactly the wire-visible fields — I verified `ErrorContext` has no field besides `code`, so "stage + kind + context code" really is the complete wire-visible state and nothing is silently unequal-comparable. The mapping arms in range match the module-doc contract one-for-one, and the contract is executed evidence: the baseline MP4 smoke prints these exact mappings as wire lines, and the in-file bridge tests ran green. The single finding is a latent fragility, not a live bug: `as_wire`'s `TracksChanged` arm zips `tracks` with `selected` and would silently truncate the wire track list if the two vectors ever diverged in length — today they cannot (both are born from `pipeline.tracks()` where `selected = vec![false; tracks.len()]`, pipeline.rs:1060), but the boundary has no assert and no diagnostic.

## 2. What actually works (verified)
— Wire mapping executed end-to-end: `audit/runs/baseline/mp4-smoke-open-session.txt` lines `event: state-changed Idle → Opening`, `event: tracks 2 ready`, `event: TimingDurationKnown …`, `event: MediaInfoReady { … tracks: [Video h264, Audio aac] … }` — each is a `StateChanged`/`TracksChanged`/`DurationKnown`/`MediaInfoReady` variant from this enum crossing `as_wire` in the shipped `lumen` binary.
— In-file bridge tests executed: baseline `orchrun-test-workspace-nofailfast.log.txt:3761-3763` — `event::tests::fatal_maps_to_wire_playback_error_with_details` (asserts stage/code/fatal/details/context mapping), `private_kinds_never_reach_the_wire` (asserts `DamagedData`/`FirstFrame` → `None`), `stage_bridge_covers_every_l0_stage` — all `ok` (in the playback_engine lib's 83 passed, :3741).
— Manual `Clone`/`PartialEq` semantics verified against the real `EngineError` shape (media-types error.rs:259-265): the only dropped field is `source: Option<Box<dyn Error>>`, exactly as documented at lines 84-88; `PartialEq::Fatal` (404-406) compares `stage && kind && context.code` — complete except the non-wire `source`.
— `#[non_exhaustive]` on the enum (line 90) matches doc 05 §6.2's forward-compat rule, stated at 79-82.

## 3. Findings

### F-001 [S4]
Where: crates/playback-engine/src/event.rs:522-529
Code:
```
            EngineEventKind::TracksChanged { tracks, selected } => {
                Some(EventKind::TracksListReady {
                    tracks: tracks
                        .iter()
                        .zip(selected.iter())
                        .map(|(track, &is_selected)| track_info_wire(track, is_selected))
                        .collect(),
                })
            }
```
What is wrong: `zip` silently truncates to the shorter iterator. If `selected.len() != tracks.len()`, the wire `tracks.list-ready` event drops the tail tracks (or ignores surplus flags) with no panic, no counter, no log — the host just sees fewer tracks than the engine demuxed.
Why it matters: today the lengths match by construction (`Pipeline` sets `selected = vec![false; self.tracks.len()]` at pipeline.rs:1060 and `sync_selected_mirror` copies the pair together, session.rs:2331-2337), so this is latent. But the invariant lives two files away from the consumption site with zero enforcement at the boundary; a future partial update of one vector (e.g., a track-list mutation path that appends tracks without resizing `selected`) would silently shrink the user-visible track list — precisely the class of drift this audit exists to catch.
What correct looks like: `debug_assert_eq!(tracks.len(), selected.len())` before the zip (the mismatch is a programming error, not runtime data — the same constructor-contract posture `ErrorContext::new` takes), or zip over `tracks.iter().map(|t| …)` with `selected.get(i).copied().unwrap_or(false)`.
Confidence: HIGH — Verification: read + construction-site trace (session.rs:1659-1661, 2331-2337; pipeline.rs:1022-1024, 1060).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (wire-mapping contract) | 1-58 | REAL | — | 17 mapping claims; the 4 verifiable against in-range arms all match (§6). One name drift: doc line 17 says engine event "`Ended`" — the variant is `EndOfMedia` (doc 03's name vs the Rust name; harmless but a claim-table row). |
| `EngineEventKind` (enum, 17 variants) | 91-284 | REAL | — | StateChanged, TracksChanged, Buffering, EndOfMedia, Fatal, DamagedData(private), FirstFrame(private), Seeked, SeekFailed, TimeTick, DurationKnown, MediaInfoReady (boxed — D-2 rationale documented at 189-197), EndOfTrack, RecoveredDamage, TrackSelectionChanged, AudioDeviceLost/Changed/Suspended, RendererBackendReport. Every variant carries a wire-row or engine-private doc. No payload-less "log-line-only" variants. |
| `impl Clone for EngineEventKind` | 286-380 | REAL | — | All 17 arms present; Fatal rebuilds via `EngineError::new` (source chain dropped, documented); TracksChanged clones the Arc (cheap); MediaInfoReady deep-copies the Box (documented once-per-open exception). |
| `impl PartialEq for EngineEventKind` | 382-510 | REAL | — | All 17 arms paired; catch-all `_ => false` (cross-variant never equal); Fatal compares the complete wire-visible field set (verified against ErrorContext's single field); TracksChanged compares full track vectors. |
| `EngineEventKind::as_wire` (head) | 512-530 | REAL | S4 | 4 arms in range (StateChanged/TracksChanged/Buffering/EndOfMedia), each matching the module-doc contract; **F-001 zip fragility**. Remaining 13 arms + private-variant `None`s are part2 (A09). |

## 5. Test honesty (in-file tests are part2's range; coverage verified for the arms in range)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `event::tests::fatal_maps_to_wire_playback_error_with_details` (963-992, part2 range) | YES | Asserts stage (`Stage::Demux`), code (`ErrorCode::Inaccessible`), `fatal: true`, `details["context"] == "demux.header.missing"`, message content — a wrong bridge arm or a lost detail fails it. Executed ok, baseline :3761. |
| `event::tests::private_kinds_never_reach_the_wire` (994-1002) | YES | Asserts `DamagedData`/`FirstFrame` → `as_wire() == None`. Executed ok, baseline :3762. |
| `event::tests::state_changed_round_trips_through_the_wire_envelope` (1004+) | YES (arm in my range) | Round-trips StateChanged through the wire envelope — pins the `from.as_wire()/to.as_wire()` arm at 518-521. |
| Mutation check (reasoned): break `TracksChanged` mapping to return `None` | Caught | Every open lane emits tracks-list-ready (baseline smoke `event: tracks 2 ready`); events_contract + first-vertical suites assert the wire row. |
| Mutation check (reasoned): `Clone` for Fatal panics or drops `code` | Caught | `Fatal` clones flow through OutputHub-era receipts/events in contract suites; `private_kinds`/`fatal_maps` pin the wire-visible fields. UNVERIFIED: no test clones a `Fatal` and asserts field equality directly (the manual Clone's Fatal arm is exercised only incidentally). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`StateChanged` → `session.state-changed`" | module docs :13 | REAL | as_wire arm 518-521; executed as `event: state-changed Idle → Opening` (mp4-smoke-open-session.txt). |
| "`TracksChanged` → `tracks.list-ready`" | module docs :14 | REAL | as_wire arm 522-530; executed as `event: tracks 2 ready` (same run). |
| "`Buffering` → `session.buffering` (episodes open 0.0 / close 1.0)" | module docs :15-16 | REAL (arm) | Arm 531-533 (part2 boundary); the 0.0/1.0 bookend cadence is emitted by session.rs (part3/4) — smoke shows `Buffering` transitions; bookend values asserted in events_contract suites. |
| "`Ended` → `playback.end-of-media`" | module docs :17-18 | REAL (naming drift) | Variant is `EndOfMedia` (118); doc 03's engine-side name is "Ended". Mapping arm is part2's; the smoke's `state-changed … Stopped → Idle` lane emits the media-end sequence. |
| "FirstFrame / DamagedData → engine-private, no v1 wire row" | module docs :39-42 | REAL | `private_kinds_never_reach_the_wire` ok (baseline :3762); catalog has no such rows (W02-A07's spec.rs audit: 26 event rows, none first-frame/damaged). |
| "`Clone`/`PartialEq` are manual … equality compares stage + kind + context code — the fields the wire event exposes" | type docs :84-88 | REAL | ErrorContext has exactly one field (`code`, media-types error.rs:223-225); eq arm 404-406 matches; clone arm 301-312 drops only `source`. |

## 7. Dead / duplicated / unreachable code
None in range. All 17 variants have emitters (session.rs emit paths, hotplug hooks) — spot-verified for AudioDeviceLost/Changed/Suspended and RendererBackendReport via `replay_backend_report_at_open` (session.rs:1087) and the hotplug module.

## 8. Dependencies & flags
- Unconditional code; no feature gates. Deps: `commands` (wire types), `media-types` (L0 vocabulary), crate-internal `seek`/`state`. Layering correct (L3 → L2/L0).
- `#[non_exhaustive]` per doc 05 §6.2 — hosts must degrade unknown kinds; matches the wire catalog's own rule.

## 9. Open questions for the next agent
- **For W03-A09 (part2):** `as_wire` straddles at line 530/531 — the Buffering arm's close, Error/Fatal arms, seek terminals, TimeTick, DurationKnown, MediaInfoReady, EndOfTrack, RecoveredDamage, TrackSelectionChanged, the three audio-device variants, RendererBackendReport, and both private-variant `None` returns are yours; my F-001 (zip) sits at the boundary arm — do not double-file it.
- The `Buffering` "episodes open with 0.0 and close with 1.0" bookend claim is only half-verifiable from this chunk (the arm maps the float verbatim); the emission cadence is session.rs part3/4's — A01/A09 should confirm the bookends are asserted somewhere (events_contract.rs is the likely home).
- No test found that clones a `Fatal` event and asserts the rebuilt error's fields (the manual Clone arm's only direct exercise is incidental); a cheap unit test would pin the documented clone semantics.
