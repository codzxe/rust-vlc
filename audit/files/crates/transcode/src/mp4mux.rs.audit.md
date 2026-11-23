# AUDIT: crates/transcode/src/mp4mux.rs
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2025-06-14 |
| Lines / functions | 912 / 20 prod fns + 4 tests |
| File verdict | REAL (executed: unit + full demuxer round-trip suite) — but UNREACHABLE from shipped entry points (F-001) |
| Reachable from shipped entry point? | NO. In-crate chain: `record.rs:291 Remuxer::remux` -> `runner.rs:717 run_mp4_remux` -> `runner.rs:767 Mp4Remuxer::new`. Cross-crate: zero — grep of apps/ (cli, desktop, control-server, web) for `transcode|Remuxer|JobHandle` = no matches; no crate Cargo.toml depends on `transcode`. Release lane (lumen-cli + desktop) never links it. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 3 |
| Commands run | `./target/debug/deps/transcode-f17733efc4fd5aa3 mp4mux` -> 4 passed, exit 0; `./target/debug/deps/mp4_remux-2cce5ceb8b8d5666` -> 7 passed, exit 0 |

## 1. One-paragraph verdict
This is a real, careful ISO-BMFF writer: byte-verbatim sample streaming, run-length stts/ctts, v1 ctts for negative offsets (reachable — `Timestamp::ticks` is i64, media-types/src/time.rs:137-138), stss omission = all-sync, one-sample-per-chunk stco, typed 32-bit ceilings checked per-add, and a first-sample timeline re-anchor. I executed its 4 unit tests AND the 7-test acceptance suite (`tests/mp4_remux.rs`) which round-trips committed H.264/AAC fixtures through the project's own demuxer with packet-identical comparison, including fragmented→progressive, edit-list re-anchor, and mid-cancel honest-prefix — all green. The defects: the whole transcode crate ships nowhere (matrix rows honestly `not_started`); the stbl writers contain silent u32 conversions (a >u32 stts delta becomes 0; a >u32 ctts offset truncates via `as u32`) that contradict the module's own "bounded, never wrapped" contract; and the "~10 bytes per sample → stays under the demuxer's 16 MiB moov ceiling" doc claim is wrong at the ceiling (worst case ~24-32 B/sample → up to ~32 MiB moov our own demuxer refuses at MAX_MOOV_BYTES, demux/src/mp4/mod.rs:113,767).

## 2. What actually works (verified)
— `./target/debug/deps/transcode-f17733efc4fd5aa3 mp4mux` -> "test result: ok. 4 passed" (fourcc set, byte-layout honesty of ftyp/mdat/moov, typed refusals, sample ceiling).
— `./target/debug/deps/mp4_remux-2cce5ceb8b8d5666` -> "test result: ok. 7 passed; finished in 0.01s" — `progressive_source_remuxes_and_redemuxes_packet_identical`, `fragmented_source_remuxes_into_a_progressive_output`, `edit_list_source_re_anchors_the_media_timeline`, `mid_remux_cancel_leaves_an_honest_playable_prefix` + 3 typed-refusal tests. This is a genuinely strong acceptance suite (real demuxer as oracle, not self-comparison).
Baseline cross-check: audit/runs/baseline/cargo-test-workspace.txt covers the same suites.

## 3. Findings
F-001 [S2]
Where: crates/transcode/src/mp4mux.rs:194-203 (and crate wiring)
Code: `pub struct Mp4Remuxer {\n    out: Box<dyn WriteSeek>,\n    tracks: Vec<TrackState>,`
What is wrong: no shipped entry point links the transcode crate. `apps/cli` and `apps/desktop` contain zero references to `transcode`, `Remuxer`, or `JobHandle` (grep, no matches). The feature matrix is honest about it: `playback-record` status `not_started`, evidence `""` (feature-matrix.toml:1631-1639), `ui-dialog-convert` `not_started` (:2914-2922). Why it matters: 900+ lines of tested remux code are product-dead today; any user-facing "convert/record" surface does not exist. Not a false claim (matrix says not_started) — a major unwired surface, same class as auth.rs F-001. What correct looks like: ship the 7.E3 Convert/Record UI card or record the dependency explicitly. Confidence: HIGH — Verification: ran (greps) + read.

F-002 [S3]
Where: crates/transcode/src/mp4mux.rs:523-526
Code: `let mut runs: Vec<(u32, u32)> = Vec::new();\n        for delta in sample_deltas(state) {\n            let delta = u32::try_from(delta).unwrap_or(0);`
What is wrong: a per-sample DTS delta that exceeds u32::MAX is silently written as **0**, not saturated and not refused typed. At a 90 kHz timescale u32::MAX ≈ 13.25 hours — a valid sparse track (timelapse, surveillance, long-gap recording) crossing that gap produces a lying stts table: the sample's duration becomes 0, total duration and A/V sync drift for everything after. The module doc (L66) promises "Typed ceilings (MP §12 — bounded, never wrapped)" — this is neither wrapped nor typed; it is silently zeroed, the worst of the three. What correct looks like: `mp4_size_overflow`-style typed refusal (the error vocabulary already has the precedent at :281-287). Confidence: HIGH — Verification: read.

F-003 [S3]
Where: crates/transcode/src/mp4mux.rs:68-71 (doc claim)
Code (verbatim, 4 lines):

        //! * per-track sample count ≤ [`MAX_REMUX_SAMPLES`] (the demux
        //!   layer's own sample ceiling, mirrored — a 1 Mi-sample table stays
        //!   under the demuxer's 16 MiB `moov` parse ceiling at this muxer's
        //!   ~10 bytes per sample);
What is wrong: the arithmetic is optimistic. Worst-case per-sample bytes: stts 8 (uncompressed runs) + ctts 8 (v0) or 12 (v1) + stss 4 + stsz 4 + stco 4 = 24-32 B/sample. At MAX_REMUX_SAMPLES = 1 Mi that is a 24-32 MiB `moov` — up to 2x the demuxer's `MAX_MOOV_BYTES` = 16 MiB (demux/src/mp4/mod.rs:113, enforced :767-768). A VFR B-frame video at the sample ceiling would produce a file the project's own demuxer refuses to open. Common cases (CBR audio, run-compressible GOPs) stay far below, which is why the suites never see it. What correct looks like: lower MAX_REMUX_SAMPLES to what the worst-case table shape actually bounds (e.g. 512 Ki), or compute the bound from the real per-sample maxima. Confidence: HIGH — Verification: read + computed; demux ceiling verified.

F-004 [S4]
Where: crates/transcode/src/mp4mux.rs:512-513
Code: `let entry_len = 8 + state.setup.config.sample_entry.len();\n        let entry_len = u32::try_from(entry_len).unwrap_or(u32::MAX);`
What is wrong: for a sample entry ≥ 4 GiB the stsd entry size field is written as u32::MAX while fewer bytes follow — a structurally corrupt box, the exact "wrapped" shape the module promises never to emit. The 64 KiB `MAX_SAMPLE_ENTRY_BYTES` ceiling is listed under this module's "Typed ceilings" (L72, L111) but is enforced only in graph.rs:361 (the runner path); a direct `Mp4Remuxer::new` caller gets no check. Absurd input sizes make it theoretical, but the ceiling claim is mislocated. What correct looks like: check the ceiling in `Mp4Remuxer::new` (it receives the setups). Confidence: HIGH — Verification: read + grep (only graph.rs:361 checks it).

F-005 [S4]
Where: crates/transcode/src/mp4mux.rs:558-561
Code: `if negative {\n                    body.extend_from_slice(&offset.to_be_bytes());\n                } else {\n                    body.extend_from_slice(&(offset as u32).to_be_bytes());\n                }`
What is wrong: a non-negative composition offset > u32::MAX truncates via `as u32` (classic cast truncation) instead of saturating/refusing — same family as F-002. A cts of 13+ hours (at 90 kHz) is absurd for real composition offsets, hence S4 not S3. Confidence: HIGH — Verification: read.

F-006 [S4]
Where: crates/transcode/src/mp4mux.rs:630-634
Code: `let declared = state.last_duration;\n        let prev = out.last().copied().unwrap_or(declared);\n        out.push(if declared > 0 { declared } else { prev });`
What is wrong: a single-sample track whose only packet declared no duration gets stts delta 0 → a zero-duration track (mvhd/tkhd duration 0). Players show a 0-length file for real one-frame content. Edge case; wrong in a realistic one-screenshot-video case. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| sample_entry_fourcc | 154-160 | REAL | — | stream-copy set only, cross-kind lies refused (tested) |
| Mp4Remuxer::new | 210-233 | REAL | S4 | no sample-entry ceiling (F-004) |
| add_packet | 247-304 | REAL | — | verbatim payload; typed refusals all tested; 32-bit ceilings checked |
| finalize | 314-329 | REAL | — | mdat patch + moov; defensive size re-check |
| write_ftyp / write_mdat_header | 332-355 | REAL | — | byte layout pinned by test |
| build_moov | 360-419 | REAL | S3 | moov wrapper correct; size math F-003 |
| put_mvhd | 423-436 | REAL | — | v0, saturating durations (documented policy) |
| put_tkhd | 439-462 | REAL | — | flags 0x7, dims from entry offsets 24/26 (layout verified) |
| put_mdhd | 466-483 | REAL | — | v0/v1 switch correct |
| put_hdlr | 486-496 | REAL | — | vide/soun |
| put_stbl | 499-610 | REAL | S3/S4 | stts zero-clamp (F-002), ctts truncation (F-005) |
| nanos_to_ticks | 614-620 | REAL | — | round-half-up, correct |
| sample_deltas | 625-636 | REAL | S4 | zero-duration single-sample (F-006) |
| track_media_duration / track_movie_duration | 639-656 | REAL | — | saturating sums; unit conversion correct |
| put_identity_matrix / put_u32_saturating | 659-675 | REAL | — | |
| video_dimensions | 680-688 | REAL | — | VisualSampleEntry offsets 24/26 correct |
| put_box | 691-698 | REAL | — | placeholder + patch |
| write_all / seek_to | 701-710 | REAL | — | typed folds |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fourcc_mapping_is_the_stream_copy_set | YES | includes cross-kind lies + non-copy codecs |
| one_track_two_samples_finalizes_a_wellformed_file | YES | asserts exact ftyp/mdat offsets, patched mdat size, honest moov size field |
| typed_refusals_of_the_direct_api | YES | asserts error codes for ordinal/dts-missing/backwards |
| the_sample_table_ceiling_is_typed | YES | drives the real add_packet guard (white-box pre-fill avoids 1 Mi real samples — the guard itself executes) |
| tests/mp4_remux.rs (7, executed) | YES | real-demuxer round-trip with field-by-field packet comparison — the strongest form (oracle independent of the writer); cancel-prefix honesty pinned |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "an ISO-BMFF container writer for stream copy, never a transcode … every other codec is a typed refusal" | mp4mux.rs:1-5 | REAL | fourcc test + remux_refuses_codecs_outside_the_stream_copy_set (ran) |
| "Typed ceilings (MP §12 — bounded, never wrapped)" | mp4mux.rs:66 | FALSE-in-part | stts zero-clamp (F-002), ctts `as u32` truncation (F-005), entry_len u32::MAX lie (F-004) — silent, not typed |
| "a 1 Mi-sample table stays under the demuxer's 16 MiB moov parse ceiling at ~10 bytes per sample" | mp4mux.rs:68-71 | FALSE at the ceiling | worst case 24-32 B/sample → up to ~32 MiB; demuxer refuses (demux/src/mp4/mod.rs:113,767) |
| "MP4 output to a pipe is honestly NOT offered" | mp4mux.rs:84-87 | REAL | API requires `Write + Seek` (graph.rs WriteSeek) |
| "Stream recording: record-to-WAV … + MP4 remux" | feature-matrix.toml:1633 | REAL code / not shipped | suite executed; status honestly not_started, apps/ has zero references |

## 7. Dead / duplicated / unreachable code
Whole crate unreachable from shipped entry points (F-001). No dead functions inside the file. The `negative` ctts v1 branch IS reachable (i64 ticks allow pts < dts) and is exercised via `edit_list_source_re_anchors_the_media_timeline` in the acceptance suite.

## 8. Dependencies & flags
std + media-types only; no unsafe (matches C5 claim); no feature gates. Sinks abstracted via `WriteSeek` (graph.rs).

## 9. Open questions for the next agent
- job.rs / record.rs / runner.rs audits (other agents): confirm the runner's timescale guarantee ("must be the native 1/timescale form — the runner refuses anything else") actually holds on the graph path, since add_packet trusts packet ticks == track timescale blindly (no cross-check in this file — a wrong-base packet would write a wrong table silently).
- If 7.E3 ever ships the pipe sink, remember mdat patching requires Seek — the module honestly refuses; keep it that way.
