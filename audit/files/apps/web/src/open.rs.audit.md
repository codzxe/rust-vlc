# AUDIT: apps/web/src/open.rs
| Field | Value |
|---|---|
| Auditor | W12-A04 |
| Date | 2026-11-21 |
| Lines / functions | 575 / 20 items (incl. 6 tests) |
| File verdict | UNREACHABLE (real, fixture-tested open path; crate not linked by any shipped entry point) |
| Reachable from shipped entry point? | NO. Internal chain: worker.js → abi.rs dispatch → WebHost session-open (host.rs:1492 `open::open`) → this file. External: `lumen`/`lumen-tauri` do not depend on lumen-web (see audit/files/apps/web/Cargo.toml.audit.md). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:3 |
| Commands run | Read open.rs 1–576 (full); rg `struct SelectDemuxer` + sed 565–620 apps/cli/src/demux_host.rs (twin comparison: CLI overrides reanchor, web does not); sed containers/avi.rs 70–142 (harvest twin is verbatim); rg `pub enum StreamFormat` header.rs:92–117 (exactly two variants); rg `pub fn probe_stream` media-probe lib.rs:193–211 (rewind verified); rg `push_packet` playback-engine/pipeline.rs:2191–2197 (unwired-track packets dropped by policy); rg `fn time_base` header.rs:267–269 (base=(scale,rate), frame_ns math correct); rg `reanchor` plugin-api demux.rs:101–121 (default = typed refusal) + playback-engine seek.rs:390 (engine calls it); rg `playback.seek` host.rs (dispatch refuses at 1837 — masks F-001); rg `open::` host.rs (use sites 356/419/600/680/1308/1492/1733–1751/1935). |

## 1. One-paragraph verdict
A disciplined, genuinely tested mirror of the CLI's AVI open path (resolve → probe → typed family refusal → wiring → facts harvest → selection wrapper), with every refusal typed and the probe's non-consuming window verified real. The one substantive defect is a mirror-fidelity gap the CLI twin does not have: the web `SelectDemuxer` forwards `seekability()` but not `reanchor`, so it advertises a capability (indexed AVI seek) that its own default impl refuses — currently masked only because the host dispatch refuses `playback.seek` one layer earlier. The rest is doc drift and an inherited (CLI-shared) diagnostic weakness. As with the whole crate: no shipped product lane reaches it.

## 2. What actually works (verified)
- The full open path over the REAL fixture: `open_first_vertical_fixture_harvests_facts` (open.rs:410–447) asserts container `avi`, 2 tracks, `mjpeg` + dimensions, `pcm_s16le` + (2, 44100), duration present, and wrapper pass-through — would fail on any harvest/wiring mutation. Ran in baseline #3 (`cargo test --workspace`, lumen-web compiled+run, test-workspace.log.txt:21).
- Probe is truly non-consuming: `media_probe::probe_stream` restores the pre-probe position (crates/media-probe/src/lib.rs:202–209, sed verified) — the claim at open.rs:229 is REAL.
- Deselected-kind packets are safe: the engine drops packets for unwired tracks by policy (playback-engine/src/pipeline.rs:2191–2197) — the wrapper's pass-through `next_event` cannot choke the engine.
- `frame_ns` math is correct: AVI base = (scale, rate) (header.rs:267–269), one tick = one frame, `num*1e9/den` with u64 saturating mul — no overflow for u32 scale, no div-by-zero (TimeBase::new validates den).
- Typed refusals all verified by tests: File (local-filesystem), rtsp/rtp/udp (rtsp-rtp-udp-access), http(s) (http-streaming), unknown blob (NotFound), non-AVI family (probe Unsupported naming `wav`), garbage (probe refusal).

## 3. Findings

### F-001 [S3] — SelectDemuxer forwards `seekability` but inherits the default `reanchor` refusal
Where: apps/web/src/open.rs:354-360
Code:
```
    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, media_types::EngineError> {
        self.inner.seek(target)
    }

    fn seekability(&self) -> plugin_api::SeekSupport {
        self.inner.seekability()
    }
```
What is wrong: The impl block ends here — no `reanchor` override. The trait default (plugin-api/src/demux.rs:110–121) is the typed refusal `plugin.demux.reanchor.unsupported`, and the trait's own contract says "a demuxer that claims seekability but does not implement re-anchoring fails honestly rather than pretending." The wrapper claims the inner demuxer's seekability (e.g. AVI idx1 → `Fast { indexed }`, and the host reports `seekable: true` at host.rs:1660) while every engine-level re-anchor (playback-engine/src/seek.rs:390 → `demuxer_reanchor` → this default) refuses. The CLI twin overrides `reanchor` to forward (apps/cli/src/demux_host.rs:612–620, sed verified) — the web mirror dropped that half.
Why it matters: Today it is masked — the web dispatch refuses `playback.seek` up front (host.rs:1837–1839). The moment that host duty lands (the module docs' own roadmap), every seek on a seekable AVI fails with a plugin-vocabulary refusal inside the wrapper, and the bug will look like an engine/plugin problem, not a 4-line forwarding gap. `seekable: true` is already advertised to the page while seek refuses — the contradiction is live in media-info today.
What correct looks like: `fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, media_types::EngineError> { self.inner.reanchor(target) }` (no skip watermark to clear on web).
Confidence: HIGH — Verification: read + sed of trait default, CLI twin, engine seek flow.

### F-002 [S4] — `start_position_ns` doc promises a `start_time` that does not exist on this host
Where: apps/web/src/open.rs:113-114
Code:
```
    /// The position the session anchors at (ns; `start_time` or 0).
    pub start_position_ns: i64,
```
What is wrong: `WebOpenOptions` (119–127) has only `no_video`/`no_audio`; no path sets anything but `0` (line 258). The SelectDemuxer docs (317–320) say `start_time` "answers the typed refusal in the host's dispatch" — so the field doc's "start_time or" half is dead vocabulary.
Why it matters: Doc drift; a reader will search for the start_time seam that is not here.
What correct looks like: "The position the session anchors at (ns; always 0 in v1 — `start_time` is refused typed at dispatch)."
Confidence: HIGH — Verification: read + rg (no other writer of the field).

### F-003 [S4] — "unknown" codec discards the track's real codec name
Where: apps/web/src/open.rs:296
Code:
```
            None => ("unknown".to_string(), None, None, None),
```
What is wrong: For any AVI track whose `strf` is neither MJPEG nor PCM — `StreamFormat` has exactly two variants (header.rs:92–117) — the harvested codec is the literal string `unknown`, discarding `track.codec` (the L0 registry descriptor the demuxer already produced). The downstream refusal then reads "no decoder for `unknown` in this build" instead of naming the actual codec. Verbatim-mirrored from the CLI (apps/cli/src/containers/avi.rs:117) — a shared defect, not web-local. Consequence worth recording: the host.rs codec-* opt-in rows (flac/mp3/vorbis/opus/aac/alac) can NEVER be selected by AVI media — the harvest cannot produce those strings; they are wasm size-measurement lanes only (which host.rs:561–572 admits).
Why it matters: Diagnostic quality on valid non-first-vertical AVIs; misleading decoder-refusal text.
What correct looks like: `None => (track.codec.as_str().to_string(), …)` with the facts fields still `None`, so the refusal names the real codec.
Confidence: HIGH — Verification: read + sed of both harvests + StreamFormat enum.

### F-004 [S4] — AVI header parsed twice per open
Where: apps/web/src/open.rs:252-254
Code:
```
    let tracks = demuxer.start().map_err(|err| wire_error(&err))?;
    let facts = harvest_facts(&demuxer, &tracks);
    let wrapper = SelectDemuxer::new(demuxer, options.no_video, options.no_audio);
```
What is wrong: `open()` calls `start()` to harvest facts; the engine's Opening phase later calls the wrapper's `start()` again, which re-runs the inner parse (header + strl walk) a second time per session open. Bounded (header-sized), mirrored from the CLI's identical shape — but on a wasm host where every parse is main-thread budget.
Why it matters: Wasted cooperative-slice budget; trivially avoidable by caching the first `start()` result.
What correct looks like: Wrapper seeds from the already-parsed track list (CLI could adopt the same fix).
Confidence: HIGH — Verification: read + wrapper `start` re-delegation at 338–348 (no caching).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-21 | REAL | — | Honest registry posture; claims verified. |
| TrackFact (struct) | 39-57 | REAL | — | Mirrors CLI demux_host.rs:144–170 minus `extra_data` (documented omission, host.rs:563–572). |
| TrackFact::span_end_us | 60-67 | REAL | — | Ticks→ns→µs, truncating `/1000`; `?` on missing. |
| TrackFact::audio_geometry | 69-78 | REAL | — | PCM-only; CLI's generalized non-PCM geometry (card 6.OPW-ogg fix) NOT mirrored — equivalent for AVI-only v1 (StreamFormat has no non-PCM audio variant). |
| MediaFacts (struct) | 82-90 | REAL | — | tracks + parallel facts. |
| MediaFacts::duration_us | 92-97 | REAL | — | max of span ends. |
| MediaFacts::fact_for | 99-104 | REAL | — | Linear find; fine at open time. |
| OpenedMedia (struct) | 106-117 | REAL | F-002 | start_position_ns doc drift. |
| WebOpenOptions (struct) | 119-127 | REAL | — | Two flags; Default derived. |
| resolve_source | 129-202 | REAL | — | Scheme grammar; all refusal arms test-pinned; blob name edge cases benign (empty name → NotFound). |
| open | 204-261 | REAL | F-004 | Double `start()`; probe rewind verified; non-AVI typed refusal names the format. |
| harvest_facts | 263-315 | REAL | F-003 | frame_ns math verified; exhaustive over StreamFormat's two variants; span end from timing. |
| SelectDemuxer (struct + new) | 317-335 | REAL | — | v1 scope: kind selection only (documented). |
| SelectDemuxer::start | 337-348 | REAL | — | Filters by kind incl. `_ => false` (subtitle/data dropped — same as CLI:573). |
| SelectDemuxer::next_event | 350-352 | REAL | — | Pass-through; safe (engine drops unwired-track packets, pipeline.rs:2191–2197). |
| SelectDemuxer::seek | 354-356 | REAL | — | Forwarded. |
| SelectDemuxer::seekability | 358-360 | REAL | F-001 | Forwarded while reanchor is the default refusal — the contradiction. |
| wire_error | 363-382 | REAL | — | Strip-prefix on Display is defensive (`unwrap_or` full); reuses playback-engine's single mapping (no local copy). |
| host_error | 384-398 | REAL | — | Always `ErrorKind::Unsupported` — semantically loose for wiring-bug codes (`web.audio_lane_absent`) but the stable dotted code disambiguates. |
| tests (6) | 400-575 | REAL | — | All assert real behavior over real fixtures; see §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| open_first_vertical_fixture_harvests_facts (410) | YES | Real fixture; asserts container/tracks/codecs/geometry/dimensions/duration/wrapper-passthrough. |
| selection_flags_drop_their_kinds (452) | YES | no_audio → exactly 1 video track. |
| unwired_family_is_the_named_refusal (479) | YES | Real WAV fixture → probe-stage Unsupported naming `wav`. |
| garbage_is_the_probe_refusal (505) | YES | 4096 zero bytes → probe-stage refusal. |
| source_grammar_refusals_are_typed (527) | YES | Five refusal arms, stage+code+capability detail each. |
| (no test for seek/reanchor through the wrapper) | — | Gap that let F-001 land; the wrapper's seek surface has zero coverage. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v1 wires exactly the first vertical's family (`avi`)" | open.rs:16-17 | REAL | Line 238 refuses every other probe id; test-pinned (479). |
| Probe is "the bounded, non-consuming window" | open.rs:229 | REAL | media-probe lib.rs:202–209 restores position (sed). |
| "The lumen-cli crate itself cannot be a dependency here … so the AVI wiring is mirrored" | open.rs:7-12 | REAL | apps/web/Cargo.toml has no lumen-cli edge; harvest is a verbatim mirror of containers/avi.rs:84–142. |
| "`start_time` answers the typed refusal in the host's dispatch" | open.rs:318-320 | REAL | host.rs:1837 refuses `playback.seek`-class host duties (the start_time skip is part of that refusal family). |
| TrackFact is "the CLI's demux_host::TrackFact, mirrored" | open.rs:37-38 | SHALLOW | Mirror minus `extra_data` and minus the CLI's generalized audio_geometry — divergences documented in host.rs but not in this file's own claim. |

## 7. Dead / duplicated / unreachable code
- Product-unreachable (crate-level). No dead items inside: every pub item has callers in host.rs (rg list in header). Duplication: harvest_facts + SelectDemuxer are deliberate mirrors of apps/cli/src/containers/avi.rs:84–142 and demux_host.rs:517–577 — three-way drift risk (CLI, web, and the F-001 reanchor divergence already proves the risk is real).

## 8. Dependencies & flags
- No feature gates in this file. `plugin_api::{Demuxer, DemuxEvent, SeekSupport}` — plugin-api is this crate's trait vocabulary (not a plugin runtime); `demux`, `media-probe`, `media-types`, `media-clock`, `commands` — all workspace crates, wasm32-clean (rc-smoke #6 PASS).

## 9. Open questions for the next agent
- A03 (part4, host dispatch): when `playback.seek`'s host duties land, F-001 becomes user-visible — the reanchor forwarding fix belongs in this file, 4 lines.
- A08/A09 (abi.rs/worker.js): does the wire surface expose `seekable: true` to the page today (it does via media-info per host.rs:1660)? If so, the page can offer a seek UI that always refuses — UX-level contradiction worth a UI-side note.
- Plugin-domain note for the wave: `plugin.demux.reanchor.unsupported` (the trait default this file tripped) is plugin-api vocabulary surfacing in a non-plugin host — see my access.rs/filter.rs audits for how much of plugin-api is vocabulary vs. plugin machinery.
