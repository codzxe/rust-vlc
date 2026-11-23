# AUDIT: crates/demux/tests/corpus_mp4.rs
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-23 |
| Lines / functions | 345 / 2 tests (corpus_dir, TRUTHS table of 30 lanes) |
| File verdict | REAL-UNPROVEN (live, non-vacuous pins — proven by execution — but both tests take the skip path in this tree) |
| Reachable from shipped entry point? | Integration test; the code under test (`Mp4Demuxer`, `open_via_input` real access stack) is the shipped MP4 open path (containers/mod.rs "mp4"/"mov"/"m4a" arms) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | baseline citation (below); `LUMEN_CORPUS=/tmp/w04mp4/media ./target/debug/deps/corpus_mp4-324c0f2c426050ba --exact --nocapture corpus_mp4_family_matches_the_ffmpeg_truth` -> FAILED at corpus_mp4.rs:272 `0008-fps-extremes.mp4: track 0 packet count left: 75 right: 345` (pins live; my substitute file demuxed correctly to its own 75) |

## 1. One-paragraph verdict
The same story as the other corpus differentials, with two additional soft spots. The 30-lane TRUTHS table (23 mp4 + 5 mov + 2 m4a, with 0128's mpeg2video vocabulary gap honestly documented) compares codec identity, per-track packet counts, and first-video keyframe counts against ffprobe — and the pins are live: my substitute `0008` file failed the count assert (75 vs 345), and in failing proved the demuxer itself works on fresh bytes (the codec assert passed and all 75 packets of my 3 s @ 25 fps file were counted correctly through the real access stack). But both tests skip in the baseline (`finished in 0.01s` — the documented loud-skip path; the corpus dir is absent), so the table's evidence is worklog-provenance only. Beyond the skip: the seek test never asserts that a single packet is delivered after landing (its "resumes" claim is unchecked — a demuxer that instant-EOSes post-seek passes), and the family differential has no PTS/DTS or payload-digest comparison (the Ogg suite carries md5 parity; this one would pass a demuxer that emits the right count of garbage bytes).

## 2. What actually works (verified)
- Pins are non-vacuous: substitute-file run fired `assertion left == right failed: 0008-fps-extremes.mp4: track 0 packet count left: 75 right: 345` at line 272.
- Incidental REAL demux verification: before the mismatch, my fresh ffmpeg h264 mp4 went through `open_via_input` + `Mp4Demuxer::start` (track count 1, codec "h264" assert passed at line 244) and drained exactly 75 packets for a 3 s @ 25 fps file — the shipped MP4 path demuxes a novel real file correctly.
- Table arithmetic: 23 mp4 + 5 mov + 2 m4a = 30 TRUTHS entries, consistent with the doc's "24 .mp4" minus the documented 0128 mpeg2video refusal (17-26, 162-163).
- Budget guards: 2M-event cap per file (256, 342) — honest.

## 3. Findings

F-001 [S3]
Where: crates/demux/tests/corpus_mp4.rs:219-227 (and 211-217), baseline audit/runs/baseline/cargo-test-workspace.txt:2210-2214
Code:
```rust
fn corpus_mp4_family_matches_the_ffmpeg_truth() {
    let Some(dir) = corpus_dir() else {
        eprintln!(
            "corpus(mp4): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
             — SKIP (the differential table for the 2026-10-24 instance is in the worklog)"
        );
        return;
    }
```
What is wrong: both tests (family + seek) take the skip path in this tree — baseline `test result: ok. 2 passed ... finished in 0.01s` cannot have demuxed 30 ISO-BMFF files. Zero regression protection for MP4/MOV/M4A codec-identity/count/keyframe parity in the default lane; the pinned table is REAL-UNPROVEN (worklog provenance only). Same structural issue as corpus_ogg.rs and ts_pes_corpus.rs (cross-file pattern: every corpus differential is skip-gated on an absent directory). Confidence: HIGH — Verification: ran (baseline line citation + live-fire on substitute files).
Confidence: HIGH — Verification: ran

F-002 [S3]
Where: crates/demux/tests/corpus_mp4.rs:330-343
Code:
```rust
        // The walk resumes and finishes.
        let mut seen = 0u64;
        loop {
            match mp4
                .next_event()
                .unwrap_or_else(|e| unreachable!("{probe}: event: {e}"))
            {
                DemuxEvent::EndOfStream => break,
                DemuxEvent::Packet(_) => seen += 1,
                _ => {}
            }
            seen += 1;
            assert!(seen < 2_000_000, "{probe}: budget");
        }
```
What is wrong: the comment promises "The walk resumes and finishes" — but `seen` is used only as a budget counter and is never asserted after the loop. A demuxer whose `seek` left `next_event` returning instant `EndOfStream` (zero post-seek packets) passes this test; so does one that resumes at the wrong position. "Finishes" is asserted (EOS reached without error); "resumes" is not. What correct looks like: `assert!(seen > 0, "{probe}: no packets after seek")` plus a position/stamp check on the first post-seek packet. Confidence: HIGH — Verification: read (no post-loop assert exists).
Confidence: HIGH — Verification: read

F-003 [S4]
Where: crates/demux/tests/corpus_mp4.rs:1-7 vs the assert set at 237-278
Code:
```rust
//! demuxed through the real access stack and compared against the
//! **measured ffmpeg truth** — `ffprobe -count_packets` stream codecs
//! and packet counts, plus the first video stream's keyframe-flagged
//! packet count (`ffprobe -show_packets packet=flags`, `K`-flagged).
```
What is wrong: the doc is honest about what is compared, but the test *name* ("matches the ffmpeg truth") is the thinnest of the corpus differentials: no PTS/DTS series, no payload digests (the Ogg suite carries full md5 payload parity; the TS suite carries a 53-value PTS series). A demuxer that emitted the correct count of wrong bytes would pass. Confidence: HIGH — Verification: read (assert inventory at 237-278).
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| corpus_dir | 211-217 | REAL | — | override + absent-default loud skip |
| corpus_mp4_family_matches_the_ffmpeg_truth | 219-280 | REAL-UNPROVEN | S3 | 30-lane codec/count/keyframe parity; live pins (proven); never runs in-tree (F-001); no payload/timing parity (F-003) |
| corpus_mp4_seek_lands_and_resumes | 284-345 | REAL-UNPROVEN | S3 | 5 probes, at-or-before-250ms landing asserted; resume NOT asserted (F-002); never runs in-tree |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| corpus_mp4_family... | YES (when corpus exists) | codec names, exact per-track counts, keyframe counts — proven live on a substitute file |
| corpus_mp4_seek... | PARTLY (when corpus exists) | landing position and clean EOS asserted; post-seek delivery NOT (F-002) |
| In-tree reality | BOTH SKIP | baseline 0.01s (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every ISO-BMFF file in the randomized real corpus ... compared against the measured ffmpeg truth" | corpus_mp4.rs:1-7 | REAL-UNPROVEN | pins live (substitute run fired); in-tree run = skip |
| "0128-param-midchange.mp4 is MPEG-2 video ... refuses the file typed (no-tracks) — recorded [PENDING]" | corpus_mp4.rs:19-22 | UNVERIFIED | requires the absent corpus; no in-tree fixture covers mpeg2video refusal |
| "0029-cover-audio.m4a's cover art is a real attached-pic video trak — compared like any other stream" | corpus_mp4.rs:23-24 | UNVERIFIED | absent corpus |
| "the per-file PASS/FAIL table for this instance lives in the worklog" | corpus_mp4.rs:12-15 | REAL | worklog table exists (docs/worklog/archive-2026-10.md, MP4 section) |

## 7. Dead / duplicated / unreachable code
None. TRUTHS fully consumed. `mod common` shared with sibling corpus tests.

## 8. Dependencies & flags
`demux::mp4::Mp4Demuxer`, `common::open_via_input` (real access stack), `plugin_api`, `media_types`. LUMEN_CORPUS override. No features. Binary `corpus_mp4-324c0f2c426050ba` newer than source (verified).

## 9. Open questions for the next agent
- The mp4 seek path (src/mp4/*, another agent's files): F-002 means the corpus seek test would not catch a seek-then-starve regression — check whether the in-tree mp4 unit tests assert post-seek packet delivery.
- If a small committed mp4 fixture lane were wired into this same assert body (as with the ASF fixture family), the family differential would run in-tree without the 120 MB corpus.
