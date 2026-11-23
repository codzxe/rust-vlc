# AUDIT: crates/playback-engine/src/event.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-23 |
| Lines / functions | 1061 / 21 prod + 4 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/host.rs:675 `if let Some(wire) = event.as_wire_event()` (and apps/web/src/host.rs:1324) drains engine events through this module's mapping on every poll; emission sites session.rs/hotplug.rs construct the enum |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | `./target/debug/deps/playback_engine-30ee2bde67c1e3d9 event::tests` -> 4 passed, 0 failed, exit 0; grep reachability (apps/cli host.rs:675, apps/web host.rs:1324, apps/cli host.rs:1433-1434 track_info_wire reuse, apps/cli host.rs:1441-1466 enrich_track_info); sed quote verification |

## 1. One-paragraph verdict
This is the real vocabulary-to-wire bridge of the engine's event stream: 18 `EngineEventKind` variants, the doc 05 §4.2 mapping (`as_wire`), and the L0→wire bridges (stage, error code, track info, source spec, media info). It is on the live path of both shipped hosts (CLI host.rs:675, web host.rs:1324 call `as_wire_event`), the mappings are total (no panic arms), private kinds (`DamagedData`, `FirstFrame`) correctly answer `None`, and the fatal ordering/message conventions are tested. The defects are documentation-level lies and one real payload gap: `track_info_wire` hard-codes `video: None, audio: None` so `tracks.list-ready`/`media.info-ready` can NEVER carry dimensions/channels, while the same repo's CLI host enriches `media.get-info` with exactly those facts from demuxer `strf` — making the twice-repeated claim "the same data `media.get-info` answers" false. The module header also still says device events "remain Phase 3+" although the 7.C1 device variants sit 200 lines below it.

## 2. What actually works (verified)
- `./target/debug/deps/playback_engine-30ee2bde67c1e3d9 event::tests` -> `test result: ok. 4 passed; 0 failed` (exit 0): Fatal→PlaybackError with details, private kinds never reach the wire, StateChanged round-trips the envelope, wire_stage covers all 14 L0 stages.
- Reachability: every emitted engine event crosses `as_wire`/`as_wire_event` (apps/cli/src/host.rs:675, apps/web/src/host.rs:1324); `track_info_wire` additionally reused by the CLI (host.rs:1433-1434) and web (host.rs:1317) hosts.
- The 18-variant mapping is compiler-exhaustive (no `_ =>` arm in `as_wire`), so a new variant cannot silently map wrong.

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/src/event.rs:796-819 (track_info_wire), 199-202 + 911-913 (doc claims)
Code:
```
        is_default: track.flags.default_on,
        is_selected,
        origin: WireTrackOrigin::Stream,
        video: None,
        audio: None,
```
What is wrong: the wire `TrackInfo`'s `video`/`audio` detail blocks are permanently `None` on the engine path because L0 `media_types::Track` (media-types/src/ids.rs:635-653) carries no codec parameters. Meanwhile the CLI host DOES have the facts (demuxer `strf`) and enriches its `media.get-info` answer: apps/cli/src/host.rs:1448-1464 `if let Some((width, height)) = fact.dimensions { info.video = Some(...) } ... info.audio = Some(...)`. So the event-stream payloads are strictly poorer than the command answer for the same media, and the doc claim on the `MediaInfoReady` variant — "the same data `media.get-info` answers" (event.rs:200, repeated at 912) — is false for dimensions/channels/rates. Why it matters: hosts (desktop/web UI track pickers) consuming `tracks.list-ready`/`media.info-ready` can never show resolution or channel count; the doc tells them the data is equivalent. What correct looks like: extend L0 `Track` with the demuxer-declared geometry (the facts already exist host-side) or delete the equivalence claim. Confidence: HIGH — Verification: read + grep (both host paths). UNVERIFIED: whether any UI screen actually reads these event payloads for dimensions (UI audit is another agent's domain).

F-002 [S4]
Where: crates/playback-engine/src/event.rs:44-48 (module doc)
Code:
```
//! evidence (doc 05 §1.4 C3 — completion arrives as the terminal
//! events above). Device events and `ClockMasterChanged` remain
//! Phase 3+ (`2.C3`/`7.C1`, `3.B2`). Event *completeness* — every
```
What is wrong: stale header — `AudioDeviceLost`/`AudioDeviceChanged`/`AudioDeviceSuspended`/`RendererBackendReport` (card 7.C1) ARE variants in this file (244-283) with live emission sites (hotplug.rs:196/281/374/392). The module doc contradicts its own enum. Only `ClockMasterChanged` genuinely remains absent. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/playback-engine/src/event.rs:767-772 (ns_to_us)
Code:
```
/// Nanoseconds → microseconds, rounded up (the wire `Timestamp` unit,
/// doc 05 §2.2). Positions are non-negative by construction (targets
/// clamp to `[0, duration]` and landings are keyframe positions).
fn ns_to_us(ns: i64) -> i64 {
    (ns + 999) / 1_000
}
```
What is wrong: for negative `ns` this is round-toward-zero, not "up": `-1500 + 999 = -501`, `-501 / 1_000 = 0` (true ceil is -1). `ns + 999` also overflows at `i64::MIN` (panic in debug). Both are harmless iff the "non-negative by construction" claim holds for every producer (`Seeked.actual` = "first audio frame's own timestamp" — frame stamps from demuxers of TS/PS-class containers can be negative in principle). I could not verify all stamp producers; current first-vertical demuxers (AVI/WAV/OGG/ADTS) produce non-negative stamps. Confidence: HIGH on the arithmetic, LOW on reachability — Verification: read; producer audit UNVERIFIED (demux crate is another agent's file).

F-004 [S4]
Where: crates/playback-engine/src/event.rs:84-88 + 404-406 (PartialEq for Fatal)
Code:
```
/// Cloning rebuilds the typed error without the source chain (events
/// are data, not error-chain carriers); equality compares
/// stage + kind + context code — the fields the wire event exposes.
```
What is wrong: the claim "the fields the wire event exposes" is inaccurate — the wire `PlaybackError` also exposes `message` (as_wire:541 `message: error.message_summary()`), which PartialEq ignores. Two Fatal events with identical stage/kind/context but different messages compare equal, so a test asserting "the right fatal" can pass on the wrong message. Blast radius: assertions only (production never compares events). Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/playback-engine/src/event.rs:949-954 (wire_playback_state)
Code:
```
/// The wire playback state of an engine state — trivially
/// [`SessionState::as_wire`], named here for the event-module readers.
#[must_use]
pub fn wire_playback_state(state: SessionState) -> PlaybackState {
    state.as_wire()
}
```
What is wrong: the only caller in the tree is tests/soak/dispatch.rs:897 — a one-line re-export wrapper with zero production consumers (hosts call `SessionState::as_wire` directly, e.g. apps/cli/src/host.rs:1417). Exported dead API. Confidence: HIGH — Verification: read + grep (apps/, crates/).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| impl Clone for EngineEventKind | 286-380 | REAL | — | Fatal rebuilt w/o source chain (documented); Arc-shared track list clones cheaply |
| impl PartialEq for EngineEventKind | 382-510 | REAL | S4 | F-004: message not compared, doc overstates |
| EngineEventKind::as_wire | 512-615 | REAL | — | exhaustive; private kinds → None; ns→µs via ns_to_us (F-003) |
| EngineEventKind::is_pipeline_scoped | 617-632 | REAL | — | 3 session-scoped kinds per doc 05 §3.2; consumed by drain policy (events.rs:504) |
| EngineEvent::is_pipeline_scoped | 654-660 | REAL | — | delegate |
| EngineEvent::as_wire_event | 662-675 | REAL | — | called by both shipped hosts (verified) |
| wire_stage | 677-706 | REAL | — | 14 L0 → 13 wire stages; judgment calls [PROPOSAL]-tagged; test-pinned (ran) |
| wire_error_code | 708-723 | REAL | — | 7→7 one-to-one |
| fatal_details | 725-732 | REAL | — | context + stage keys |
| seek_error_wire | 734-765 | REAL | — | typed reason + position_us detail; NotSeekable/OutOfRange → control/unsupported |
| ns_to_us | 767-772 | REAL | S4 | F-003 negative/overflow edges |
| seek_failure_reason | 774-782 | REAL | — | stable reason keys |
| track_info_wire | 784-819 | REAL | S3 | F-001: video/audio always None; fourcc/profile/level always None |
| recovered_damage_details | 821-829 | REAL | — | count key |
| suspend_notice_details | 831-841 | REAL | — | context/device/reopen_attempts keys |
| suspend_notice_message | 843-864 | REAL | — | display-safe wording, two arms |
| wire_track_kind | 866-875 | REAL | — | 1:1 |
| wire_track_change_kind | 877-890 | REAL | — | 1:1; secondary-audio arm unminted (documented) |
| source_spec_wire | 892-909 | REAL | — | redacted display form; file:// strip via trim_start_matches (repeat-trim quirk harmless) |
| media_info_wire | 911-947 | REAL | S3 | F-001 doc claim; container/title/metadata/chapters always empty ("v0 honest unknowns") |
| wire_playback_state | 949-954 | UNREACHABLE(prod) | S4 | F-005 test-only |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fatal_maps_to_wire_playback_error_with_details (962-991) | YES | pins stage/code/fatal flag/context detail/message substring (ran) |
| private_kinds_never_reach_the_wire (993-1001) | YES | DamagedData/FirstFrame → None (ran) |
| state_changed_round_trips_through_the_wire_envelope (1003-1030) | YES | envelope fields + kind equality (ran) |
| stage_bridge_covers_every_l0_stage (1032-1060) | PARTIAL | totality is compiler-given; only 3 of 14 mappings asserted — weak pin of the judgment calls |

Coverage gap: 13 of 18 variants' `as_wire` arms have no unit test in this file (Seeked, SeekFailed, TimeTick, DurationKnown, MediaInfoReady, EndOfTrack, RecoveredDamage, TrackSelectionChanged, AudioDeviceLost/Changed/Suspended, RendererBackendReport, TracksChanged) — they are exercised only by the integration suites (events_contract.rs / hotplug.rs / track_switch.rs; claimed, not all re-verified here).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| mapping table StateChanged→state-changed ... FirstFrame/DamagedData private (13-42) | module doc | REAL | as_wire:518-546 matches; test ran |
| TracksChanged → tracks.list-ready (14) | module doc | REAL | as_wire:522-529 (naming inversion vs wire `TracksChanged` is confusing but consistent) |
| "Device events ... remain Phase 3+" (47-48) | module doc | FALSE | 7.C1 variants present at 244-283 with live emitters (F-002) |
| MediaInfoReady "the same data media.get-info answers" (200, 912) | variant + fn doc | FALSE | F-001: host enriches get-info with dimensions/channels; engine event cannot |
| "equality compares ... the fields the wire event exposes" (87-88) | type doc | FALSE | message is wire-exposed but not compared (F-004) |
| ns_to_us "rounded up" (767) | fn doc | FALSE for ns<0 | arithmetic verified (F-003); non-negativity UNVERIFIED |
| seek payload "position the session continues from ... never a fabricated landing" (153-156) | variant doc | REAL-UNPROVEN | session.rs seek rollback logic is W03-A01's file; mapping itself verified |

## 7. Dead / duplicated / unreachable code
- `wire_playback_state` (F-005) — test-only wrapper.
- Duplication risk: `suspend_notice_message`/`suspend_notice_details` deliberately mirror crate::recovery's notice wording "byte-identical" (831-846) — a second copy of the same strings exists in recovery.rs (not re-verified here; contract suite claims to pin the join).

## 8. Dependencies & flags
Depends on commands (wire types), media-types (L0 Track/EngineError), crate::seek/state. `#[non_exhaustive]` IS present on EngineEventKind (line 90) — unlike the catalog's EventKind where W02-A08 found the attribute missing despite the same claim; here the doc and code agree. No feature gates; no unsafe.

## 9. Open questions for the next agent
- AudioDeviceSuspended is pipeline-scoped (not in the 624-632 exemption), so an intra-epoch mint (a seek accepted while suspended) suppresses the "recovery gave up" notice at drain. Consistent with doc 05's gen-column for playback.error, but the sibling device facts are exempt — is the asymmetry intended? (tests/hotplug.rs:675-712 covers only Lost/Changed/BackendReport.)
- Do TS/PS demuxers ever hand the engine negative frame stamps (making F-003 reachable)? Owner: demux crate auditors.
- recovery.rs's notice strings vs suspend_notice_message duplication — verify the "byte-identical" contract-suite pin when auditing recovery.rs/tests.
