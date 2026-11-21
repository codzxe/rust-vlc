# AUDIT: apps/cli/src/containers/ps.rs (part2: lines 551-1056 of 1056 — the `#[cfg(test)] mod tests`; part1 = production code 1-550, also mine)
| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-21 |
| Lines / functions | 506 / 11 items (helpers ×6 incl. 2 structs; tests ×4) |
| File verdict (part2) | REAL |
| Reachable from shipped entry point? | N/A (cfg(test)) — but they exercise the REAL shipped path: every lane opens through `open_source` (canonicalize → access → probe → containers/mod.rs:71 → ps::open), and the suite executed green in the baseline rerun: `containers::ps::tests::*` 4/4 ok (orchrun-test-workspace-nofailfast.log.txt:2830-2833). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | full Read of 551-1056; `rg "REASON_TAIL\|REASON_RESYNC" apps/cli/src/containers/ps.rs` → no test assertion (both consts appear only at definition + part1 use sites); `rg "reanchor" apps/cli/tests/*.rs` → no PS reanchor coverage; `rg "no_video\|no_audio" apps/cli/src/containers/ps.rs` → helper hard-wires `false, false` (ps.rs:593); arithmetic re-derivation of every pinned constant (see §2); `rg "containers::ps::tests" nofailfast log` → 2830-2833 all ok; `ls crates/demux/fixtures/ps/` → all four fixtures exist. |

## 1. One-paragraph verdict
This is the strongest test module I have audited in this wave: all four lanes go through the real `open_source` wiring (not mocks of the subject), pin exact event sequences, exact 39-value stamp ladders, ffmpeg-differential decode counts (44 928 sample frames), typed refusal codes (`cli.codec_unavailable`, `cli.pcm_facts_absent`), and measured `--start-time` skip counts — and I re-derived every pinned constant by hand (anchor arithmetic 9 000 → 11 351; duration 26 122 448 ns; ladder 47 618 + k×2 351 with 71 129/94 639/118 149 riding on it; first survivors 55 800 / 54 671; 39×1 152 = 44 928 = 89 856 s16le bytes / 2). The gaps are the damage paths: the resync-found walk (garbage followed by a valid frame), the EOS tail note, the MaybeTail hold, discontinuity propagation through the assembler, and the reanchor-keeps-the-walk-intact claim from part1 have zero coverage anywhere — the tail note is structurally unobservable in these tests because every walk loop breaks at `EndOfStream` (part1 F-001), so the suite cannot fail on the very defect that exists.

## 2. What actually works (verified)
- All four tests pass in the baseline rerun: `mpg_lane_demuxes_splits_and_decodes_mp2_exactly ... ok`, `start_time_skips_packets_with_slow_linear_seekability ... ok`, `synthetic_lane_splits_spanning_frames_and_drains_typed ... ok`, `vob_lanes_demux_and_refuse_undecodable_codecs_typed ... ok` (orchrun-test-workspace-nofailfast.log.txt:2830-2833).
- Arithmetic of every pin, re-derived by hand: `ticks_for(1152, 44100)` = (1152×90 000 + 22 050)/44 100 = 2 351 → stamps 9 000 / 11 351 (ps.rs:729-730) ✓; duration 1152×1e9/44 100 floored = 26 122 448 (ps.rs:714) ✓; mpg ladder step 2 351 with anchors 71 129 = 47 618 + 10×2 351 + 1 (the ±1 wire wobble the comment documents, 825-831) ✓; 39 AUs × 1 152 = 44 928 == ffmpeg's 89 856 s16le bytes / 2 (886-889) ✓; start-time first survivors 54 671 = 47 618 + 3×2 351 and kept counts 32−2 / 39−3 (1046-1053) ✓; mp2_frame_208: 144×64 000/44 100 = 208 (integer div, byte 0x40 = bitrate idx 4 @ 44.1 kHz, 0xFD = MPEG-1 L2 no-CRC) ✓; seek clamp 2 s target → last SCR 90 000 ticks (739-751) ✓.
- The synthetic lane is a true hostile-shape test: spanning frame across an UNSTAMPED second PES (anchor truth), 30-byte garbage tail (drain), SlowLinear + clamp query (656-753). — verified by read; passing per baseline.
- The mpg lane decodes through the REAL provider (`HostDecoderProvider::decoder_for`) — a genuine end-to-end demux+decode differential against ffmpeg-measured counts, not a mock. — verified by read (601-612, 851-889).
- Fixtures exist: `crates/demux/fixtures/ps/{ps-mp2-video-audio.mpg, ps-vob-ac3.vob, ps-vob-dts.vob, ps-vob-lpcm.vob}` (ls).

## 3. Findings

### F-001 [S3]
Where: apps/cli/src/containers/ps.rs:718-724 (and the same break at 810-816, 947-956, 1024-1044)
Code:
```
                Ok(DemuxEvent::Recovered { skipped, reason }) => {
                    events.push(format!("recovered {skipped} {reason}"));
                }
                Ok(DemuxEvent::Discontinuity) => events.push("discontinuity".to_string()),
                Ok(DemuxEvent::EndOfStream) => break,
                Err(err) => unreachable!("demux: {err}"),
```
What is wrong: The assembler's damage vocabulary has three reasons (`REASON_RESYNC`, `REASON_DRAIN`, `REASON_TAIL`, ps.rs:117-125) but the tests only ever assert `REASON_DRAIN` (once, synthetic lane, line 731). No test drives (a) garbage followed by a valid frame — the `Resync::Found` walk (ps.rs:330-340), (b) end-of-stream leftovers — `flush`/`REASON_TAIL` (ps.rs:416-424), (c) the `MaybeTail` hold-then-judge path (ps.rs:344), or (d) a discontinuous PES propagating `pending_discontinuity` onto the next minted AU (ps.rs:318-320, 386-387). Worse, the walk loops `break` at `EndOfStream`, so a `REASON_TAIL` note could never be observed through this harness even if a truncated-tail fixture existed — the test design mirrors the production ordering defect (part1 F-001) instead of exposing it.
Why it matters: The damage paths are this module's core honesty claim ("bounded and never silent"), and three of the four are mutation-unprotected: deleting the resync branch, the tail flush, or the discontinuity latch would pass the entire suite green. The demux-crate half's own audit (W05-A06) already found untested damage-path bugs in this family — the CLI-side assembler has the same blind spot.
What correct looks like: A synthetic lane with garbage-then-valid-frame (assert a `ps.pes.mpeg-audio-resync` note with the exact skipped count, then the frame), a truncated-tail lane asserting the tail note (which requires fixing part1 F-001's ordering first — notes before EOS), and a discontinuous-PES lane asserting the flag lands on the first post-discontinuity AU.
Confidence: HIGH — Verification: read + grep (`REASON_TAIL`/`REASON_RESYNC` appear nowhere in the test module).

### F-002 [S4]
Where: apps/cli/src/containers/ps.rs:592-597
Code:
```
    fn open(path: std::path::PathBuf, start_us: Option<i64>) -> OpenedMedia {
        match open_source(&SourceSpec::File { path }, start_us, false, false) {
            Ok(opened) => opened,
            Err(err) => unreachable!("{err}"),
        }
    }
```
What is wrong: The helper hard-wires `no_video=false, no_audio=false` — every PS lane opens with both track kinds enabled. The `--no-video`/`--no-audio` selection lane for this family (SelectDemuxer's `start()` filtering and the engine's unwired-track drop) has no test here.
Why it matters: Small gap; SelectDemuxer is exercised generically in demux_host's own tests, but no test proves the PS wiring passes the flags correctly (a swapped-arguments regression at ps.rs:189-190 would pass this suite).
What correct looks like: One lane with `open_source(..., true, false)` asserting the video track vanishes from `start()` and video packets are dropped at the unwired boundary.
Confidence: HIGH — Verification: read + grep (no `no_video`/`true` combination in this module).

### F-003 [S4]
Where: apps/cli/src/containers/ps.rs:540-549 (part1 claim) — no covering test exists
Code:
```
        // The demuxer's compute-then-commit walk (contract D4) —
        // the assemblers' streaming state stays intact across it:
        // a reanchor marks the next packet discontinuous and the
        // walk resumes at a pack boundary, so the next PES pushes
        // into a still-valid buffer (a mid-frame cut at the landing
        // holds exactly like a PES-boundary cut — the assembler
        // neither knows nor needs to know).
```
What is wrong: Part1's `reanchor` forwarding carries a specific behavioral claim — the assembler's partial-frame buffer survives a seek landing mid-frame — with zero test coverage anywhere: `rg "reanchor" apps/cli/tests/*.rs` finds only OGG (native_seek.rs:460) and clock reanchors (sync_scenarios.rs); no test seeks a PS session mid-playback, and none of the four lanes here calls `reanchor` at all.
Why it matters: Seek is a declared capability of this family (`SeekSupport::SlowLinear`); the comment's "holds exactly like a PES-boundary cut" is an untested invariant — a regression that clears or corrupts the assembler buffer on reanchor would pass every test in the repo.
What correct looks like: A lane that opens, walks a few AUs, `reanchor`s to a mid-PES target, and asserts the first post-landing AU is whole, discontinuous-flagged, and correctly stamped from the new anchor.
Confidence: HIGH — Verification: read + repo-wide grep for PS reanchor tests (none).

## 4. Per-function table (part2)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `tests` mod + imports | 556-565 | REAL | — | pulls the real provider/engine/testkit — no mocks of the subject |
| `CountingSink` + `FrameSink::accept` | 569-580 | REAL | — | counts frames via `format.frames_in(len)`, remembers channels; real decode sink |
| `fixture` | 584-588 | REAL | — | path into committed demux-crate fixtures (all four exist on disk) |
| `open` (helper) | 592-597 | REAL | S4 | real `open_source` — the strongest possible harness; F-002 flag hard-wiring |
| `decoder_for` | 601-612 | REAL | — | real `HostDecoderProvider`; typed refusals RETURNED — pinned by code downstream |
| `mp2_frame_208` | 619-626 | REAL | — | header bytes verified against parse_head tables (144×64 k/44 100 = 208); framing-only, honestly documented |
| `TempDir::{new,write,Drop}` | 630-648 | REAL | — | pid-tagged scratch, removes itself; `let _ =` on cleanup acceptable (test-only); allow-literal marker on scratch name |
| `synthetic_lane_splits_spanning_frames_and_drains_typed` | 656-753 | REAL | — | spanning-frame + unstamped-PES + drain + clamp query; exact event-sequence assert (726-734) — would fail on any walk regression |
| `mpg_lane_demuxes_splits_and_decodes_mp2_exactly` | 764-890 | REAL | — | 32 PES + 39 AUs, exact 39-stamp ladder, padding notes, typed video refusal, full ffmpeg-differential decode (44 928 frames) — the family's flagship evidence |
| `vob_lanes_demux_and_refuse_undecodable_codecs_typed` | 903-1002 | REAL | — | 3 fixtures × (facts, nav deselection, PES counts, span pins, typed refusals incl. `cli.pcm_facts_absent`) |
| `start_time_skips_packets_with_slow_linear_seekability` | 1013-1054 | REAL | — | watermark + measured kept/first-survivor pins; unstamped-packets-pass truth exercised (12 unstamped video PES kept) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| synthetic_lane_splits_spanning_frames_and_drains_typed | YES | exact ordered event list `[packet 208B pts 9000, packet 208B pts 11351, recovered 30 ps.pes.mpeg-audio-drain]` + clamp assertion; any split/anchor/drain regression breaks it |
| mpg_lane_demuxes_splits_and_decodes_mp2_exactly | YES | 39-value exact stamp ladder, packet counts, note tuple, refusal code, decode count 44 928 — a wire-wobble of ±2 ticks fails it |
| vob_lanes_demux_and_refuse_undecodable_codecs_typed | YES | per-fixture counts/spans/refusal codes; wrong substream attribution or a misclassified codec fails |
| start_time_skips_packets_with_slow_linear_seekability | YES | exact kept counts + first-survivor stamps + per-packet watermark assert |
| (missing) resync/tail/hold/discontinuity lanes | — | F-001: the damage paths have NO test — deleting ps.rs:330-340 (resync), 416-424 (flush), or the discontinuity latch passes the suite |
| (missing) reanchor lane | — | F-003: part1's reanchor claim untested repo-wide |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the FULL mp2 stream decodes 44 928 sample frames == ffmpeg (measured this session: 89 856 s16le bytes)" | ps.rs:885-888 | REAL | assert at 888; passing in baseline (nofailfast:2830); arithmetic 39×1 152 = 44 928, ×2 bytes = 89 856 ✓ |
| "4 of the 39 frames span a PES boundary (the trailing 14-byte unstamped PES exists only to complete frame 39)" | ps.rs:26-28 | REAL-UNPROVEN | the 39-AU count and stamp ladder are pinned; the specific "4 spanning / 14-byte" split is a measured muxer claim I cannot independently re-derive without the fixture bytes (fixture exists, ffmpeg not runnable in sandbox) |
| "the family has NO corpus lanes — the census truth; these four files are the lanes" | ps.rs:582-583 | REAL | four fixtures in crates/demux/fixtures/ps/; no PS rows found in the corpus manifests by earlier waves (W05-A07 reached the same conclusion) |
| "the 2 early-stamped video PES and the 3 early AUs drop; the 12 unstamped video PES pass" | ps.rs:1009-1011 | REAL | pinned by kept counts 30/36 + first survivors (1050-1053), passing in baseline |

## 7. Dead / duplicated / unreachable code
None. All helpers are used by at least one lane; no duplicated fixture logic (the demux crate's own test_support provides the pack/PES builders — imported, not copied).

## 8. Dependencies & flags
Test-only imports (`codec_api`, `playback_engine::DecoderProvider`, `testkit::Must`, `commands::catalog::types::SourceSpec`) — all used. `testkit::Must` used for IO panics in test setup — acceptable convention. No feature gates; suite compiled and ran in the baseline release profile (nofailfast log).

## 9. Open questions for the next agent
- Whoever fixes part1 F-001 (EOS note ordering): the fix will make a tail-note lane testable — add it together (F-001 here).
- The `mp2_frame_208` filler bytes are non-decodable garbage by design; if a future lane needs decode-through-a-spanning-frame, a real encoder-produced fixture is required — flag to the fixtures owner.
