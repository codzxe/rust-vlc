# AUDIT: apps/web/src/open.rs
| Field | Value |
|---|---|
| Auditor | W12-A05 |
| Date | 2026-02-14 |
| Lines / functions | 575 / 15 (5 production fns + 2 impls + 1 wrapper struct + 5 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | WEB LANE ONLY: `lumen_web.wasm` ABI exports (abi.rs `lumen_web_dispatch`) → host.rs:1492 `open::open` (compose/open) → this file. NOT in the native release lane (`-p lumen-cli -p desktop`); NOT bundled by the UI's `vite build` (wasm artifact staged only by ui/scripts/web-matrix-run.mjs for the e2e matrix lane). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | rg (callers, StagedSource usage, SelectDemuxer compare); sed quote checks; baseline citations |

## 1. One-paragraph verdict
The open path is real and mirrors the CLI's probe→wire→wrap shape honestly: real `media_probe` probing with position restore, real `AviDemuxer` wiring, real fact harvesting from `strf` stream formats, and a selection wrapper whose `start()` filtering is the CLI's exact shape. All 5 tests exercise real fixtures (AVI MJPEG+PCM, WAV, garbage) and passed in the baseline (cargo-test-workspace.txt:3328-3443, 111 tests total for lumen-web). Refusals are typed at the right stages with capability details. The one real defect: the `http(s)` refusal claims network streaming "is card 7.H4" while the H4 fetch+Range staged source (`io/staged_source.rs`, with passing "serves the real fixture" tests) sits in the same crate completely unwired to the open path — a stale refusal over dead-but-built machinery.

## 2. What actually works (verified)
— baseline cargo-test-workspace.txt: `open::tests::open_first_vertical_fixture_harvests_facts`, `selection_flags_drop_their_kinds`, `unwired_family_is_the_named_refusal`, `garbage_is_the_probe_refusal`, `source_grammar_refusals_are_typed` all `ok` (lines 3374-3378).
— Call chain verified: host.rs:1492 `open::open(...)` inside the session compose, after typed refusals for `start_time` (host.rs:1456-1461), `start_paused`, `rate`, track pre-selection — the exact refusals open.rs's comments promise.
— `probe_stream` restores the stream position (media-probe/src/lib.rs:202-209), so the demuxer sees a non-consumed stream — the "bounded, non-consuming window" comment is true.
— `AviDemuxer::start` caches tracks (demux/src/avi/mod.rs:892-895), so the wrapper's second `start()` (test L445) is a cached clone, not a re-parse.

## 3. Findings
F-001 [S3]
Where: apps/web/src/open.rs:182-186 vs apps/web/src/io/staged_source.rs (whole module) + io/mod.rs:69-76
Code:
```
                "http" | "https" => Err(unsupported_for_host(
                    scheme,
                    "network streaming is card 7.H4 (fetch + Range seekability detection)",
                )
```
What is wrong: the refusal message tells the user network streaming is future work, but the H4 machinery — `StagedSource` with fetch+Range windowing, seekability detection (`io/seekability.rs`), `fetch_port` — exists in the same crate, is exported (`io/mod.rs:76`), and has passing tests (`io::staged_source::tests::fetch_lane_serves_the_real_fixture_to_the_demuxer`, `ranged_server_reads_across_windows`, baseline :3341-3345). rg confirms zero uses of `StagedSource`/`WindowRequest` outside the module and its re-export. The HTTP lane is built, tested, and unreachable from the open path.
Why it matters: a full capability lane is dead code behind a stale error message; the "honest refusal" is honest about the behavior but dishonest about the reason (the work exists, it is just unwired).
What correct looks like: wire `StagedSource` into `open()` for http(s), or change the message to name the wiring gap, and record the dead lane in the capability snapshot.
Confidence: HIGH — Verification: read (rg StagedSource|WindowRequest|StagedStats outside staged_source.rs → 1 hit, the re-export).

F-002 [S4]
Where: apps/web/src/open.rs:270-273
Code:
```
            Some(StreamFormat::MotionJpeg { width, height }) => {
                // One tick of the video base is one frame (AVI scale=1).
                let frame_ns = (u64::from(track.timing.base.num)).saturating_mul(1_000_000_000)
                    / u64::from(track.timing.base.den);
```
What is wrong: the parenthetical "(AVI scale=1)" is false as a justification. One tick = `dwScale/dwRate` seconds = one frame for ANY scale/rate pair (header.rs:261-263: "one tick = `scale/rate` seconds — exactly one frame"); AVI files commonly carry scale≠1 (e.g. scale=2, rate=50 for 25 fps). The formula is correct regardless; the comment misstates the invariant it relies on.
Confidence: HIGH — Verification: read (demux/src/avi/header.rs:261-268).

F-003 [S4]
Where: apps/web/src/open.rs:350-352 vs apps/cli/src/demux_host.rs:578-590
Code:
```
    fn next_event(&mut self) -> Result<plugin_api::DemuxEvent, media_types::EngineError> {
        self.inner.next_event()
    }
```
What is wrong: the web mirror passes every packet through — including packets of kinds disabled at `start()` — while the CLI wrapper at least filters packets against its skip watermark. With `no_audio: true`, audio packets still cross the seam until the engine drops them. Matches the CLI's semantic (the CLI also forwards disabled-kind packets), so this is wasted work, not wrong behavior; but the "mirror" is a partial mirror and the asymmetry is undocumented here.
Confidence: HIGH — Verification: read (both impls compared).

F-004 [S4]
Where: apps/web/src/open.rs:175-179 vs 226-228
Code:
```
                        None => Err(CommandError::new(
                            Stage::Input,
                            ErrorCode::NotFound,
                            format!("no blob named `{name}` is loaded (send loadBlob first)"),
```
What is wrong: the same condition (unknown blob name) maps to `ErrorCode::NotFound` in `resolve_source` but to `ErrorCode::Inaccessible` in `open()`'s `registry.open` mapping. Unreachable in practice (resolve_source checks existence first and the worker is single-threaded), but the two arms of one grammar disagree on the typed code if ever reached.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| TrackFact::span_end_us | 62-67 | REAL | — | Tick→ns→µs via Timestamp::convert; `.ok()?` on non-convertible bases is a silent None (defensible: unknown extent). |
| TrackFact::audio_geometry | 72-78 | REAL | — | PCM pair for audio only. |
| MediaFacts::duration_us | 95-97 | REAL | — | Max declared extent. |
| MediaFacts::fact_for | 101-103 | REAL | — | Linear find, fine for 1-2 tracks. |
| resolve_source | 142-202 | REAL | — | Typed refusals for file/rtsp/rtp/udp/http/unknown; blob resolves via registry. |
| open | 212-261 | REAL | — | resolve → probe → avi-only wiring → facts → wrapper. start_position_ns always 0 (start_time refused upstream, verified host.rs:1456). |
| harvest_facts | 266-315 | REAL | — | MJPEG dims + frame_ns; PCM geometry; `None` → "unknown" codec. |
| SelectDemuxer::new/start | 327-348 | REAL | — | Kind filter at start; matches CLI L566-576. |
| SelectDemuxer::next_event/seek/seekability | 350-361 | REAL | — | Pure pass-through (F-003). |
| wire_error | 368-382 | REAL | — | Stage-prefix strip + playback-engine wire mappings; re-used, not re-invented. |
| host_error | 387-398 | REAL | — | Host-minted EngineError; ErrorKind::Unsupported regardless of message (fixed kind — callers must only use it for unsupported-shaped refusals; protocol.rs callers checked there). |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| open_first_vertical_fixture_harvests_facts | YES | Real fixture bytes through real probe+demuxer; asserts codec strings, geometry (2, 44100), dimensions present, duration present, 2 tracks. |
| selection_flags_drop_their_kinds | YES | no_audio → exactly 1 track, Video. |
| unwired_family_is_the_named_refusal | YES | WAV probes as `wav`; asserts stage Probe + code Unsupported + format named — a silent fallback or crash fails it. |
| garbage_is_the_probe_refusal | YES | 4096 zero bytes; asserts stage Probe. |
| source_grammar_refusals_are_typed | YES | Asserts code + capability detail for file/rtsp/udp/https/blob-unknown. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v1 wires exactly the first vertical's family (`avi`)" | L16-17 | REAL | L238-249 hard gate; WAV test proves refusal. |
| "network streaming is card 7.H4" (i.e. not present) | L183-184 | SHALLOW | StagedSource H4 machinery exists and is tested but unwired (F-001). |
| "the start-time skip watermark is omitted — the web v1 opens at 0, and `start_time` answers the typed refusal in the host's dispatch" | L318-320 | REAL | host.rs:1456-1461 refuses start_time typed. |
| "the mirror is pinned to the same probe→pre-pass→wrapper shape" (CLI parity) | L10-12 | SHALLOW | next_event is NOT mirrored (no packet filtering of any kind, F-003); the CLI filters against its skip watermark. |

## 7. Dead / duplicated / unreachable code
- `open()`'s `SourceSpec::File => String::new()` arm (L224): unreachable — resolve_source refuses File before it. Defensive only.
- Cross-file (this file's grammar): `io/staged_source.rs` StagedSource/WindowRequest — built + tested, zero callers (F-001).
- Duplication: SelectDemuxer is a near-copy of apps/cli/src/demux_host.rs SelectDemuxer minus the skip watermark — acknowledged in comments with a reason (CLI crate not wasm-buildable via codecs).

## 8. Dependencies & flags
Deps: commands, demux, media-probe, media-types, media-clock, plugin-api, playback-engine — all internal; wasm32-clean per Cargo.toml header. No external deps added by this file. Feature gates: none touching open.rs (codec features are host.rs/capabilities.rs concerns).

## 9. Open questions for the next agent
- Who was supposed to wire StagedSource into open()? If 7.H4 closed without wiring, the web lib carries a full dead capability lane (audit io/staged_source.rs separately).
- Does any feature-matrix row claim web http streaming as REAL citing staged_source tests? If so that row is FALSE (evidence exists, capability not reachable).
