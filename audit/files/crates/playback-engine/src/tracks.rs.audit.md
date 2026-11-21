# AUDIT: crates/playback-engine/src/tracks.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2026-11-21 |
| Lines / functions | 593 / 17 production units + 6 tests |
| File verdict | REAL (code implemented and executed by the test suite) — but the seven-step switch flow is UNREACHABLE from every shipped host (see Field below); selector vocabulary partially reachable |
| Reachable from shipped entry point? | SPLIT. **YES, EXECUTED**: `WiredKind` + `default_track` run in the shipped open lane — `pipeline.rs:1064-1067` iterates `WiredKind` and calls `crate::tracks::default_track` inside `Pipeline::wire`, called from session.rs:2360 (`open_step`), which every baseline open transcript exercises (INDEX.md AVI lifecycle, sample-420.mp4 play). **NO — REFUSED, VERIFIED**: the switch flow (`TrackSelector` switching, `pipeline_switch`, `complete_switch`, `InFlightTrackSwitch`) is reachable only through `SessionHandle::set_audio_track`/`set_video_track`, and BOTH shipped hosts refuse the wire commands: `apps/cli/src/dispatch.rs:199-200` answers `tracks.set-*` with the typed refusal "track switching is card 3.A3" (the CLI `SERVED` table at dispatch.rs:39-56 has no TracksSet* id), and `apps/web/src/host.rs:1850-1851` refuses identically; `rg -ln "set_audio_track" apps/` → zero hits. desktop-tauri is UNVERIFIED-BUILD (baseline #2 failed on missing system libs). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | full read via Read tool (593 lines); `rg -n "SwitchTrack" crates/playback-engine/src/state.rs` + `sed -n '558,600p'` (the three SwitchTrack rows — effects `[BumpPipeline, SwitchPipeline]`, NO ResumeClock); `sed -n '2144,2160p;2160,2220p;2674,2740p' crates/playback-engine/src/session.rs` (pause at 2157, switch completion 2674-2722 — no resume anywhere); `rg -n "ResumeClock" crates/playback-engine/src/*.rs` (only emits: state.rs:437/445/605 — Paused→Play, Buffering→Primed/EofReached→Playing); `sed -n '537,575p;788,793p' crates/media-clock/src/clock.rs` + `sed -n '365,375p'` (pause latches frozen_ns; reanchor does NOT touch `paused`; extrapolate returns Paused(frozen) while paused); `sed -n '570,590p;700,715p' apps/cli/src/host.rs` (`advance_time`/`advance_time_wall` return early when `clock.is_paused()`); `rg -n "\.resume\(\)" apps/ crates/` (host resume only in `drain_tail_at_end` at Ended); `sed -n '617,700p;760,830p' crates/playback-engine/tests/track_switch.rs` (audio-switch test asserts clock POSITION == first pts, never `is_paused()`/running); `rg -n "set_audio_track" apps/` (zero shipped callers); `sed -n '40,56p;190,205p' apps/cli/src/dispatch.rs` + `sed -n '1840,1855p' apps/web/src/host.rs` (both hosts refuse `tracks.set-*`); `ls crates/demux/src/` + `sed -n '529,531p' crates/demux/src/mp4/mod.rs` + `sed -n '144,146p' crates/demux/src/mkv/header.rs` (MP4 and MKV demuxers EXIST and map subtitle track types); `ls crates/codecs/src/audio crates/codecs/src/video` (16 codec families, not "MJPEG + PCM"); `sed -n '2221,2337p' crates/playback-engine/src/session.rs` (subtitle_select_step: `if let Ok` silent skip at 2278 + `sync_selected_mirror` at 2331-2337); `sed -n '114,118p;289,294p;409,419p' tracks.rs` (quote verification); baseline: playback_engine lib 83/83 (orchrun log 3740-3743), input mp4 smoke (INDEX.md CORRECTION v2) |

## 1. One-paragraph verdict
The selector/resolution half of this file is real, tight, and executed in the
shipped open lane; the switch-flow half is real, elaborately documented, and
executed only by tests — both shipped hosts refuse `tracks.set-*` with a typed
"track switching is card 3.A3" (a card that, per this very file, has landed —
the refusal text is stale). Inside that unreachable flow sits one genuine
engine defect: the documented seven-step audio switch freezes the clock at
step 3 ("the audio master's samples stop") and **no step ever unfreezes it** —
`complete_switch` re-anchors and publishes the generation but never resumes,
`reanchor` under pause provably keeps the clock paused (clock.rs:788-793 +
extrapolate at 371), the Playing→Playing SwitchTrack row carries no
ResumeClock (state.rs:562-568), and the CLI host's entire render loop returns
early while `clock.is_paused()` (host.rs:577, 704) — so the first audio track
switch through any future host that serves the command freezes playback while
the session keeps reporting `playing`. No test catches it: the audio-switch
test asserts the clock *position* equals the new anchor, which passes while
paused (the freeze moves to the anchor). Second, the module doc's "honest
stubs" section has drifted into falsehood on three counts (there IS an MP4 and
an MKV demuxer, both mapping subtitle track types; codecs carries 16 codec
families, not "MJPEG + PCM"), and its promised behavior for MP4 subtitle
tracks ("a subtitle Id selector resolves to the typed TrackNotFound refusal")
is contradicted by what actually happens: session.rs silently skips the failed
swap and still emits `TrackSelectionChanged { selected: Some(id) }` +
`Completed` while the selection mirror stays false.

## 2. What actually works (verified)
— `WiredKind`/`UnwiredKind`/`TryFrom<MediaKind>` — verified: the typed-lane
vocabulary the audit-F-1 fix promises; total widening, typed refusal for
`Data`; `wire()` (pipeline.rs:1064) iterates it in the EXECUTED open lane
(every baseline open transcript).
— `default_track` (312-317) — verified: `default_on` flag else
first-of-kind; called from `Pipeline::wire` (open, EXECUTED) and
`resolve_selector::Auto`.
— `resolve_selector` (325-341) — verified: Id resolves only within the
requested kind (kind mismatch = NotFound), None → Deselect, Auto →
default-or-Deselect; in-file tests pin all branches (EXECUTED green, 83-lib
suite).
— `TrackSelector::as_wire`/`from_wire` (251-272) — verified lossless round-trip
(test `selector_wire_round_trip`, EXECUTED green).
— 6 in-file unit tests — all mutation-sensitive (see §5), all green in the
no-fail-fast baseline (playback_engine lib 83/83).

## 3. Findings

### F-001 [S2]
Where: crates/playback-engine/src/tracks.rs:409-419 (complete_switch); freeze at crates/playback-engine/src/session.rs:2156-2158; no-resume proof: crates/playback-engine/src/state.rs:562-568 + session.rs:2674-2722
Code:
```rust
pub(crate) fn complete_switch(
    clock: &media_clock::MediaClock,
    inflight: &InFlightTrackSwitch,
    anchor_ns: Option<i64>,
) -> Result<(), EngineError> {
    if let Some(anchor_ns) = anchor_ns {
        clock.reanchor(Timestamp::new(anchor_ns, NANO_BASE))?;
    }
    clock.set_generation(inflight.generation);
    Ok(())
}
```
What is wrong: the audio switch freezes the clock at accept
(session.rs:2156-2158, "S2.3f — the audio switch freezes the clock at the old
position now"), and the completion step re-anchors + publishes the generation
but **never resumes the clock**. Verified against every plausible unfreeze
site: the SwitchTrack state rows are Playing→Playing / Paused→Paused /
Buffering→Buffering with effects `[BumpPipeline, SwitchPipeline]` only
(state.rs:562-568) — `ResumeClock` is emitted solely by Paused→Play and
Buffering→Primed/EofReached→Playing rows (state.rs:437/445/605);
`complete_switch_kind` (session.rs:2674-2722) calls nothing but
`complete_switch`; `MediaClock::reanchor` does not touch `core.paused`
(clock.rs:788-793) and `extrapolate` returns `Paused(frozen_ns)` while paused
(clock.rs:371) — the trait doc even states "While paused, the freeze moves to
`media`" (clock.rs:218). The CLI host's render loop hard-gates on the flag:
`advance_time`/`advance_time_wall` return early when `clock.is_paused()`
(host.rs:577, 704). Net effect: after one audio track switch from `Playing`,
audio rendering and video presentation stop while the session state stays
`playing` — until the user happens to toggle pause/play (Paused→Play carries
ResumeClock).
Why it matters: switching audio tracks (language switch) is a core VLC use
case; the engine wedges its own clock on it. It is unreachable from today's
shipped hosts only because both refuse `tracks.set-*` (see F-004) — the
engine's public `SessionHandle::set_audio_track` is one dispatch-table edit
away from shipping this wedge, and the desktop-tauri shell (UNVERIFIED-BUILD
in baseline) registers the same single dispatch.
Test gap verified: `audio_switch_happy_path_is_the_full_ordered_sequence`
(track_switch.rs:627) asserts `position_ns == first_new_pts` (773-781) — which
passes while paused, because reanchor-under-pause moves the frozen position to
the anchor; no test asserts the clock is running after a switch.
What correct looks like: step 6 must unfreeze — either `complete_switch`
calls `clock.resume()` after the re-anchor (and the deselect/provably-never
paths too, since 2157 froze them), or the freeze moves into the effect system
with a matching ResumeClock effect at completion, mirroring the seek flow's
FreezeClock/ResumeClock pairing.
Confidence: HIGH — Verification: read (full chain traced and cited; no
execution because the shipped hosts refuse the command and I am barred from
cargo).

### F-002 [S3]
Where: crates/playback-engine/src/tracks.rs:114-118 (doc), refuted by crates/demux/src/mp4/mod.rs:529-531 and crates/demux/src/mkv/header.rs:144-146
Code:
```rust
//! 1. **No demuxer emits subtitle tracks**: the AVI demuxer skips
//!    `txts`/other stream types with damage "unsupported-stream-type"
//!    (`demux/src/avi/header.rs`); there is no MP4/MKV demuxer. Real
//!    subtitle tracks arrive with the Phase-5 subtitle card.
//! 2. **No subtitle decoder exists** (`codecs` carries MJPEG + PCM).
```
What is wrong: three false statements about the current tree. (a) "there is
no MP4/MKV demuxer" — `crates/demux/src/mp4/` and `crates/demux/src/mkv/`
both exist, and the MP4 demuxer is proven working in the baseline smoke
(INDEX.md: sample-420.mp4 opens, 2 tracks, plays). (b) "No demuxer emits
subtitle tracks" — the MP4 demuxer maps `sbtl`/`text`/`subt`/`clcp` handler
fourccs to `MediaKind::Subtitle` (mp4/mod.rs:529-531) and MKV maps track type
0x11 to Subtitle (mkv/header.rs:145): an MP4 with a subtitle track WILL list
it. (c) "`codecs` carries MJPEG + PCM" — the codecs crate carries 9 audio
families (aac, adpcm, alac, flac, mp2, mp3, opus, pcm, vorbis) and 7 video
families (av1, dnx, h264, hevc, mjpeg, mpeg12, vp9); the h264 sw decoder is
compiled and wired per the baseline CORRECTION v2 (INDEX.md §"CORRECTION v2"
item 1).
Why it matters: the doc's promised consequence — "on every container the
current demuxer can parse, a subtitle `Id` selector resolves to the typed
`TrackNotFound` refusal (truthful menus)" — is false for MP4/MKV with
subtitle tracks: the selector resolves, and the actual behavior is
session.rs's silent `if let Ok` skip (session.rs:2278) which still emits
`TrackSelectionChanged { selected: Some(id) }` + a `Completed` receipt while
`sync_selected_mirror` (session.rs:2331-2337) keeps the mirror at
not-selected — an event/mirror contradiction on the exact container family
(VLC's subtitle heartland) the stub note waves off.
What correct looks like: rewrite the stub note against the current tree, and
make the failed subtitle swap report the truth (deselect in the event, or a
typed receipt) instead of skipping silently.
Confidence: HIGH — Verification: read (demux sources + ls + baseline INDEX
cited; session.rs behavior traced at 2261-2337).

### F-003 [S4]
Where: crates/playback-engine/src/tracks.rs:288-296
Code:
```rust
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrackSelectionError::NotFound { id } => {
                write!(f, "no live {}-kind track", id.handle())
            }
```
What is wrong: the id is formatted into the kind slot — the message reads
"no live 42-kind track" for a request of audio track 42. The requested kind
is not carried by the error at all, so the text cannot say what was wrong.
Why it matters: this is the typed refusal the UI surfaces for a bad track
id; the current text is word salad.
What correct looks like: carry the kind in `TrackSelectionError::NotFound`
and print "no live audio-kind track with id 42".
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: apps/cli/src/dispatch.rs:199-200 (refusal), stale relative to this file's existence
Code:
```rust
    } else if id.starts_with("tracks.set-") {
        "track switching is card 3.A3"
```
What is wrong: both shipped hosts refuse `tracks.set-*` with "track switching
is card 3.A3" — but card 3.A3 is implemented (this file, 593 lines + a full
integration suite). The refusal text describes a future that already
happened; the host surfaces were never widened after the card landed.
Why it matters: the entire switch flow (including the F-001 wedge) is dead
code from the shipped-binary perspective, and the refusal message misleads a
reader into thinking the feature is unimplemented rather than unserved. The
parity report's 0/252 is consistent with this, but the engine/test investment
is invisible to users.
What correct looks like: either serve the commands (and fix F-001 first) or
update the refusal to "track switching is implemented in the engine but not
yet served by this host".
Confidence: HIGH — Verification: read (SERVED table dispatch.rs:39-56 lacks
TracksSet*; web host identical at apps/web/src/host.rs:1850-1851; engine side
fully implemented and tested).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `WiredKind` + `as_media_kind` | 155-178 | REAL, EXECUTED | — | Total widening; used by wire() in the shipped open lane. |
| `UnwiredKind` + Display + Error | 184-205 | REAL | — | Typed refusal for Data; message quality fine here. |
| `TryFrom<MediaKind> for WiredKind` | 207-226 | REAL | — | Exhaustive match; future lane-less variants won't compile silently. |
| `TrackSelector` enum | 231-246 | REAL | — | Data; Auto's product-rules deferral documented. |
| `TrackSelector::as_wire` / `from_wire` | 248-273 | REAL | — | Lossless round-trip (test-pinned). |
| `TrackSelectionError` + Display | 277-296 | REAL | S4 | F-003: id formatted into the kind slot. |
| `ResolvedSelection` | 299-305 | REAL | — | Data. |
| `default_track` | 311-317 | REAL, EXECUTED | — | default_on else first-of-kind; runs in wire() at every open. |
| `resolve_selector` | 325-341 | REAL, EXECUTED (tests) | — | Id/None/Auto semantics correct; kind-checked ids; switch-lane use unreachable from shipped hosts. |
| `InFlightTrackSwitch` | 345-357 | REAL | — | Data; carries generation + origin for the terminal. |
| `SwitchOutcome` | 361-371 | REAL | — | Data (u32 counters, saturating use at session side). |
| `pipeline_switch` | 384-398 | REAL (UNREACHABLE from shipped hosts) | — | Thin wrapper over `Pipeline::switch_limbs`; prepare-before-commit documented. |
| `complete_switch` | 409-419 | REAL (UNREACHABLE from shipped hosts) | S2 | F-001: re-anchor + set_generation, no resume — the wedge's completion half. |
| 6 unit tests | 456-592 | REAL, EXECUTED | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `id_resolves_only_within_the_requested_kind` (457) | YES | Three vectors: in-kind id, cross-kind id, unknown id. EXECUTED green (83-lib suite). |
| `none_deselects_and_auto_takes_the_default_then_the_first` (499) | YES | None/Auto-with-flag/Auto-first-of-kind/Auto-no-track vectors. EXECUTED green. |
| `auto_prefers_the_flag_even_when_it_is_not_first` (523) | YES | Flag beats stream order. EXECUTED green. |
| `selector_wire_round_trip` (535) | YES | Round-trip of all three variants. EXECUTED green. |
| `wired_kinds_widen_to_their_media_kinds` (557) | YES | Enumerate all three lanes. EXECUTED green. |
| `data_is_the_typed_unwired_kind` (568) | YES | Data refused typed + Display text pinned. EXECUTED green. |
| (integration) `audio_switch_happy_path_is_the_full_ordered_sequence` (track_switch.rs:627) | PARTIAL | Asserts receipts, event order, zero stale frames, queue-drain pts, and clock position == anchor — but NOT that the clock resumed; the position assertion passes while paused (F-001's test gap). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The audio kind also freezes the clock at the old position now (the audio master's samples stop)" (step 3) | tracks.rs:56-57 | REAL | session.rs:2156-2158 does exactly this. |
| Step 6 "re-anchor … and move the clock's generation" | tracks.rs:76-81, 400-403 | REAL but INCOMPLETE | complete_switch matches — and the missing unfreeze is F-001. |
| "there is no MP4/MKV demuxer" | tracks.rs:116 | **FALSE** | crates/demux/src/{mp4,mkv}/ exist; MP4 proven in baseline smoke (INDEX.md). |
| "No demuxer emits subtitle tracks" | tracks.rs:114 | **FALSE** (MP4/MKV map subtitle track types) | mp4/mod.rs:529-531; mkv/header.rs:145. |
| "`codecs` carries MJPEG + PCM" | tracks.rs:118 | **FALSE** | 16 codec family modules; h264 sw decoder compiled+wired (INDEX.md CORRECTION v2). |
| "a subtitle `Id` selector resolves to the typed `TrackNotFound` refusal (truthful menus)" on every parseable container | tracks.rs:119-121 | **FALSE** for MP4/MKV-with-subtitles | Selector resolves; swap silently skipped at session.rs:2278; event/mirror contradiction (F-002). |
| "the engine-level path … is exercised on synthetic tracks by tests/track_switch.rs" | tracks.rs:122-125 | REAL | track_switch.rs suite passed in baseline (playback-engine integration binaries all ok, orchrun log 3740-3940). |
| "tracks.set-subtitle-track mints nothing … class S" | tracks.rs:105-110 | REAL | subtitle_select_step operates at current generation (session.rs:2269-2281). |

## 7. Dead / duplicated / unreachable code
- The switch flow (`pipeline_switch`, `complete_switch`, `InFlightTrackSwitch`,
  the switch half of `resolve_selector`) is unreachable from both shipped
  hosts (F-004) — reachable only via the public engine handle from tests.
- Nothing duplicated in-file; `SwitchOutcome`/`WireOutcome` are siblings by
  design (open-time vs switch-time halves of the same observability).

## 8. Dependencies & flags
- Deps: commands (RequestId + wire types), media-clock (MediaClock, NANO_BASE),
  media-types, crate::pipeline — all workspace, consistent with Cargo.toml
  (playback-engine W01-A01 audited the manifest).
- No feature gates in this file.

## 9. Open questions for the next agent
- F-001's wedge needs an execution proof once any host serves `tracks.set-*`;
  a 3-line script test (open multi-track → set_audio_track → assert
  `!clock.is_paused()` and ≥1 further present) would pin it. I could not run
  it (no cargo); REQUEST-ORCH-RUN in my log.
- The session.rs event/mirror contradiction for failed subtitle swaps
(F-002's behavioral half) belongs to the session.rs owner — the silent
`if let Ok` at session.rs:2278 deserves its own finding there.
- If the hosts ever serve tracks.set-*, F-001 and F-002 both become
user-visible immediately — fix order matters.
