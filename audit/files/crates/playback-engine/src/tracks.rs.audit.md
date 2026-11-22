# AUDIT: crates/playback-engine/src/tracks.rs

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 593 / WiredKind (+as_media_kind), UnwiredKind (+Display/Error), TryFrom<MediaKind>, TrackSelector::{as_wire, from_wire}, TrackSelectionError (+Display), ResolvedSelection, default_track, resolve_selector, InFlightTrackSwitch, SwitchOutcome, pipeline_switch, complete_switch + 6 tests (421-593) |
| File verdict | **REAL** (selection logic, engine-wired) — but the whole switch machinery is **UNREACHABLE from every shipped host**, and the module doc's "honest stub" section is built on false factual claims |
| Reachable from shipped entry point? | NO for the switch flow. The CLI refuses all `tracks.set-*` at dispatch: apps/cli/src/dispatch.rs:199-200 routes them to the unserved-command refusal ("track switching is card 3.A3" — a card that has since landed); rg for `TracksSet*Track` across apps/ + control-server: zero served arms (only playback-engine test harnesses dispatch it). Selection vocabulary (`WiredKind`, `default_track`, `resolve_selector` via session.rs:1828/2250) is exercised only through engine-internal paths that no shipped command reaches. |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 2 |
| Commands run | rg (`TracksSet*`, `TrackSelector|selector` in apps/, `from_wire`, demux Subtitle emission, tracks.rs callers); sed reads of session.rs:2225-2345, dispatch.rs:180-210; ls crates/demux/src. No cargo (static). |

## 1. One-paragraph verdict

The selector logic itself is honest and tight: `resolve_selector` is a total, kind-checked resolution (Id must be a live track *of the requested kind*, None deselects, Auto = `default_on` flag else first-of-kind else honest deselect), the `WiredKind`/`UnwiredKind` pair is a genuine type-system fix for a former runtime `expect` (audit F-1, referenced honestly), and the six unit tests are mutation-sensitive. But the file's raison d'être — the seven-step switch flow — cannot be triggered by any user: every shipped host refuses `tracks.set-*` (the CLI's refusal message still cites "card 3.A3", which has landed — this file IS card 3.A3). Worse, the module doc's "honest stubs" section justifies the subtitle dead-end with claims that are false today: "there is no MP4/MKV demuxer" — `crates/demux/src/mp4/mod.rs:529` (`b"sbtl" | b"text" | b"subt" | b"clcp" => MediaKind::Subtitle`), `demux/src/mkv/header.rs:145`, `demux/src/ps/mod.rs:684`, `demux/src/ts/mod.rs:382` all emit subtitle tracks, and the W00 smoke proves the MP4 demuxer is in the shipped binary. On such files a subtitle `Id` selector *resolves*, the decoder build fails, and session.rs silently deselects while still emitting `TrackSelectionChanged { selected: Some(id) }` and a `Completed` receipt (session.rs:2298-2326) — the "truthful menus" promise inverted into a false success.

## 2. What actually works (verified)

- `resolve_selector` (325-341): kind-checked, total, honest Auto-deselect; tested across all arms including kind-mismatch and unknown id (456-532).
- `WiredKind`/`UnwiredKind`/`TryFrom` (155-226): lane-less `MediaKind::Data` is a compile-time-refused conversion; consumed by pipeline.rs:1064 and session.rs:197/1827 (the engine-internal selection paths are real code).
- `default_track` (311-317): default-flag-then-first-of-kind, used by pipeline.rs:1064 (open-time Auto wiring).
- `TrackSelector::as_wire` (251-259): round-trip tested; wire vocabulary faithful.
- Engine-side wiring exists: session.rs:88-89 imports the switch machinery; class-A path at session.rs:1827-1829, class-S at 2250; `pipeline_switch` called at session.rs:2162; `complete_switch` at 2695. The engine can switch tracks when driven — only no shipped host drives it.

## 3. Findings

### F-001 [S2] — the "honest stub" justification is false: four demuxers emit subtitle tracks, and the real behavior on them is a silent deselect plus a lying selection event
Where: crates/playback-engine/src/tracks.rs:114-125; consequence at session.rs:2298-2326
Code (tracks.rs:114-121):
```rust
//! 1. **No demuxer emits subtitle tracks**: the AVI demuxer skips
//!    `txts`/other stream types with damage "unsupported-stream-type"
//!    (`demux/src/avi/header.rs`); there is no MP4/MKV demuxer. Real
//!    subtitle tracks arrive with the Phase-5 subtitle card.
//! 2. **No subtitle decoder exists** (`codecs` carries MJPEG + PCM).
//! 3. Therefore, on every container the current demuxer can parse, a
//!    subtitle `Id` selector resolves to the typed `TrackNotFound`
//!    refusal (truthful menus), and `None`/`Auto` succeed trivially
```
What is wrong: every factual leg is stale. `crates/demux/src/mp4/mod.rs:529` maps `sbtl`/`text`/`subt`/`clcp` handlers to `MediaKind::Subtitle`; `demux/src/mkv/header.rs:145` maps Matroska track type 0x11 to Subtitle; `demux/src/ps/mod.rs:684` (DVD subtitle 0x20) and `demux/src/ts/mod.rs:382` (0x90) likewise. The MP4 demuxer ships (W00 smoke: mjpeg-pcm.mov decodes; h264-aac.mp4 lists 2 tracks). Therefore point 3's promised behavior never occurs on real files: a subtitle `Id` selector *resolves* (session.rs:2250-2255), `switch_limbs` then fails to build a decoder, the `Err` is swallowed (`if let Ok(outcome) = ...`, session.rs:2298), and the code still emits `EngineEventKind::TrackSelectionChanged { track_kind: Subtitle, selected }` with the resolved `Some(id)` (session.rs:2309-2315) plus an unconditional `Completed` receipt (2318-2325) — while the pipeline mirror says nothing is selected.
Why it matters: a user selecting subtitles on a real MP4/MKV gets a success receipt, a wire event claiming the track is selected, and no subtitles ever — the exact "truthful menus" property this section claims, inverted. The doc's D-5 honesty label certifies a falsehood.
What correct looks like: fix the doc; on decoder-build failure for the optional kind, either emit `TrackSelectionChanged { selected: None }` + a completed-with-deselect receipt, or reject with the typed reason.
Confidence: HIGH — Verification: read (demux sources cited; session.rs:2250-2326 control flow).

### F-002 [S2] — the entire switch flow is unreachable from every shipped entry point; the CLI refuses `tracks.set-*` citing the very card that landed
Where: apps/cli/src/dispatch.rs:197-201 (refusal), tracks.rs:1-3 (the card this file implements)
Code (apps/cli/src/dispatch.rs:197-201):
```rust
    } else if id.starts_with("tracks.set-") {
        "track switching is card 3.A3"
    } else if id.starts_with("system.plugin") {
```
What is wrong: card 3.A3 is implemented — this file plus session.rs's class-A/class-S paths, `tests/track_switch.rs` (1891 LOC, assigned to this agent), `tests/soak/dispatch.rs:746` and `tests/scenarios_b/host.rs:1878-1881` all exercise it. But rg across `apps/cli/src/dispatch.rs`, `apps/web/src`, `apps/desktop`, `apps/desktop-tauri/src`, `crates/control-server/src` finds **zero** served `Command::TracksSet*Track` arms; the only dispatchers are test harnesses. The shipped `lumen` binary answers `tracks.set-video-track`/`-audio`/`-subtitle` with `UnsupportedForHost` and the owner string "track switching is card 3.A3".
Why it matters: track switching is a core player feature; the engine work is complete and test-proven but no user of any shipped binary can reach it. The refusal message is a fossil: it names a completed card as the future owner. This is the same shape as the drift-governor finding (W03-A02 S2) — engine-complete, host-orphaned.
What correct looks like: a served arm in `lumen_dispatch` (and the tauri/web bridges) mapping `TracksSet*Track` → the host's engine submission, or an honest refusal string naming the real blocker.
Confidence: HIGH — Verification: read (rg over every host crate; dispatch.rs served-command list at 81-153).
Severity: S2 — a shipped-core feature gap, not style.

### F-003 [S3] — `TrackSelector::from_wire` has no production caller: the wire→engine selector boundary conversion is dead
Where: crates/playback-engine/src/tracks.rs:264-272
Code:
```rust
    #[must_use]
    pub fn from_wire(selector: commands::catalog::types::TrackSelector) -> TrackSelector {
        match selector {
            commands::catalog::types::TrackSelector::Id { id } => TrackSelector::Id {
                id: TrackId::new(id.0),
            },
```
What is wrong: rg for `from_wire(` tree-wide: only tracks.rs's own tests (540/545/549). Session commands carry `command.selector` already in engine form (`session.rs:1828`, `2231`), and no host builds a `SessionCommand` with a selector because no host serves the command (F-002).
Why it matters: dead boundary conversion — consistent with F-002, but it means even when a host is wired, the id-mapping convention (`TrackId::new(id.0)`) is untested against a real wire path.
Confidence: HIGH — Verification: read (rg).

### F-004 [S4] — `pipeline_switch` is a 14-line pure delegation
Where: crates/playback-engine/src/tracks.rs:384-398
Code:
```rust
pub(crate) fn pipeline_switch(
    pipeline: &mut Pipeline,
    generation: media_types::GenerationId,
    kind: WiredKind,
    resolved: ResolvedSelection,
    provider: &dyn crate::pipeline::DecoderProvider,
    plan: &crate::pipeline::QueuePlan,
    flush_budget: u32,
) -> Result<SwitchOutcome, EngineError> {
    let selected = match resolved {
        ResolvedSelection::Track(id) => Some(id),
        ResolvedSelection::Deselect => None,
    };
    pipeline.switch_limbs(generation, kind, selected, provider, plan, flush_budget)
}
```
What is wrong: the function exists to translate `ResolvedSelection` → `Option<TrackId>` and forward; all real work is `Pipeline::switch_limbs`. Layering indirection with no logic — a doc anchor, not code.
Why it matters: trivial, but it inflates the file's apparent substance (the "seven steps" narrative vs the ~40 lines of actual logic in this file; steps 3/6 bodies live in pipeline.rs/session.rs).
Confidence: HIGH — Verification: read.

### F-005 [S4] — `complete_switch` skips the clock re-anchor silently when the first frame has no pts
Where: crates/playback-engine/src/tracks.rs:409-419
Code:
```rust
    if let Some(anchor_ns) = anchor_ns {
        clock.reanchor(Timestamp::new(anchor_ns, NANO_BASE))?;
    }
    clock.set_generation(inflight.generation);
```
What is wrong: `anchor_ns: None` (a `Stamp::Missing` first frame) skips `reanchor` but still publishes the new generation — the clock then runs from the freeze position while the new track's frames carry their own pts; the divergence is unlogged and uncounted. Documented in spirit ("may be `Missing`… the event still fires", event.rs:136-138) but there is no observability for the skipped anchor.
Why it matters: latent A/V desync seed on formats without stamps; dormant (F-002).
Confidence: MEDIUM — Verification: read.

## 4. Per-function table

| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| WiredKind | 155-178 | REAL | — | 3 lanes; consumed by pipeline/session |
| WiredKind::as_media_kind | 170-177 | REAL | — | total widening |
| UnwiredKind + Display + Error | 184-205 | REAL | — | typed refusal for Data; Display tested verbatim |
| TryFrom<MediaKind> for WiredKind | 207-226 | REAL | — | the audit-F-1 fix; exhaustive |
| TrackSelector (enum) | 231-246 | REAL | — | Id/None/Auto; Auto's product-rules engine honestly `[PROPOSAL]`-deferred (243-244) |
| TrackSelector::as_wire | 250-259 | REAL | — | faithful |
| TrackSelector::from_wire | 263-272 | UNREACHABLE | S3 | no production caller (F-003) |
| TrackSelectionError + Display | 277-296 | REAL | — | one variant; message includes the handle |
| ResolvedSelection | 299-305 | REAL | — | Track/Deselect |
| default_track | 311-317 | REAL | — | default-flag-else-first; O(2n); used at pipeline.rs:1064 |
| resolve_selector | 325-341 | REAL | — | the honest core; kind-checked; Auto-deselect honest |
| InFlightTrackSwitch | 345-357 | UNREACHABLE | S2 | per-kind slots on session (session.rs:1348/1353); no shipped command creates one (F-002) |
| SwitchOutcome | 361-371 | UNREACHABLE | — | counters; wired into totals at session.rs:2292-2300 |
| pipeline_switch | 384-398 | UNREACHABLE | S4 | pure delegation (F-004) |
| complete_switch | 409-419 | UNREACHABLE | S4 | reanchor-if-Some + set_generation; F-005 |

## 5. Test honesty (6 tests, lines 421-593)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| id_resolves_only_within_the_requested_kind (457) | YES | kind-mismatch and unknown-id negatives included |
| none_deselects_and_auto_takes_the_default_then_the_first (499) | YES | all four Auto/None arms incl. honest Data deselect |
| auto_prefers_the_flag_even_when_it_is_not_first (523) | YES | kills "first-track-wins" mutations |
| selector_wire_round_trip (535) | YES | Id/Auto/None round-trips |
| wired_kinds_widen_to_their_media_kinds (557) | YES | trivial but exact |
| data_is_the_typed_unwired_kind (568) | YES | Display string asserted verbatim; boxed-Error check |

Solid for the 170 lines of vocabulary logic. Nothing here tests the seven-step flow — that lives in tests/track_switch.rs (this agent's later file) and session.rs tests.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "there is no MP4/MKV demuxer" | tracks.rs:116 | FALSE | demux/src/mp4/mod.rs:529, mkv/header.rs:145 exist and emit Subtitle; W00 smoke decodes .mov |
| "No demuxer emits subtitle tracks" | tracks.rs:114 | FALSE | mp4/mkv/ps/ts all emit MediaKind::Subtitle (citations in F-001) |
| "No subtitle decoder exists (codecs carries MJPEG + PCM)" | tracks.rs:118-119 | REAL | crates/codecs/src has audio/ + video/ only; crates/subtitles exists but wires no codec decoder (checked: no `Subtitle` decoder in codecs) |
| "a subtitle `Id` selector resolves to the typed `TrackNotFound` refusal (truthful menus)" | tracks.rs:119-121 | FALSE on real containers | resolves; silent deselect + false `TrackSelectionChanged{selected:Some}` + Completed receipt (session.rs:2298-2326, F-001) |
| Auto = "container-default … the full product.toml rules engine is a [PROPOSAL] deferred to the config card (4.E3-era)" | tracks.rs:240-245 | REAL | read of default_track; the deferral is honest |
| "track switching is card 3.A3" (CLI refusal owner) | apps/cli/src/dispatch.rs:200 | FALSE (stale) | 3.A3 landed (this file); the CLI still refuses the command (F-002) |

## 7. Dead / duplicated / unreachable code

- The class-A/class-S switch machinery (InFlightTrackSwitch, SwitchOutcome, pipeline_switch, complete_switch, from_wire): unreachable from shipped hosts (F-002/F-003) — exercised only by playback-engine's own test harnesses.
- No duplication found within the file.

## 8. Dependencies & flags

- Depends on `commands` (wire selector), `media_clock` (PlaybackClock reanchor), `media_types`, `crate::pipeline`. No feature flags, no unsafe, no allocation in the selector path.
- Re-exports: lib.rs:145 (`TrackSelectionError, TrackSelector, UnwiredKind, WiredKind`) — public API surface used by tests and the session.

## 9. Open questions for the next agent

- session.rs owner: the class-S swallow-Err + lying `TrackSelectionChanged` event (F-001's consequence) is fixable in session.rs:2298-2326 — confirm and file.
- apps/cli dispatch owner (W13): F-002's one-arm fix (`TracksSet*Track` → host submission) — check whether any other landed cards are still in `owner_of`'s refusal list (e.g. `subtitle.`, `media.` chapters — chapters.rs EXISTS in playback-engine/src!).
- Wave 18/19: card 3.A3 matrix rows should be REAL-engine / UNREACHABLE-product.
