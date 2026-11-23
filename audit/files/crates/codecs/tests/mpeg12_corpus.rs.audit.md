# AUDIT: crates/codecs/tests/mpeg12_corpus.rs
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2025-06-27 |
| Lines / functions | 526 / 10 fns + 3 tests |
| File verdict | REAL — fixture/container lanes verified live against ffprobe in this sandbox |
| Reachable from shipped entry point? | NO — integration test (the parser it exercises is currently unreachable from shipped binaries; see headers.rs audit F-004) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/mpeg12_corpus-e7a93b397405d06f` -> 3/3 ok; rerun `--nocapture` -> 7 ROW lines all PASS (live ffprobe), corpus half SKIP (corpus-v4 absent); sed/rg verification |

## 1. One-paragraph verdict
This is the differential that makes the mpeg12 sequence-layer parser trustworthy:
every committed fixture lane (4 raw ES + TS through the landed TsDemuxer PES seam
+ 2 PS through the landed PsDemuxer) was compared by me, live, against ffprobe
7.1.5 in this sandbox — 7/7 rows PASS, none took the "PINNED (no ffprobe)" path.
The pinned table (dimensions/profile/level/chroma/rate/interlace/picture+slice
censuses/bad_headers==0) is asserted independently of the oracle, and the oracle
re-checks it. The 4:2:2 escaped-profile lane passing at profile="4:2:2"
empirically confirms headers.rs's escape modeling against ffprobe. Defects: the
file-header doc claims corpus lanes are "demuxed through the matching landed
demuxer" while the code extracts the ES with ffmpeg `-c copy` (the function's
own comment admits why — the AVI/MP4 demuxers cannot feed this parser); a
doc/code mismatch on the extraction format flag (`-f mpegvideo` in the comment,
`-f data` in the code); and the corpus half is unverifiable here (corpus-v4
absent) — it skipped, as designed.

## 2. What actually works (verified)
— `./target/debug/deps/mpeg12_corpus-e7a93b397405d06f --nocapture` -> exit 0,
7 ROW lines:
"ROW mpeg2-in-ts.ts: PASS 192x108 profile=Some(\"Main\") level=Some(8) chroma=yuv420p rate=25/1 ... (ts demux)"
"ROW mpeg2-main-ipb.m2v: PASS 320x240 ... (raw es)" / "ROW ps-mp2-video-audio.mpg: PASS ... (ps demux)" / "ROW mpeg2-422.m2v: PASS 192x108 profile=Some(\"4:2:2\") level=Some(5) chroma=yuv422p ..." / "ROW ps-vob-ac3.vob: PASS 720x480 ... rate=24000/1001 ..." / "ROW mpeg2-interlaced-ntsc.m2v: PASS ... interlaced=true ... rate=30000/1001 ..." / "ROW mpeg1-ipb.m1v: PASS 320x240 profile=None ... (raw es)"
— "SKIP mpeg12 corpus: corpus-v4 absent (LUMEN_CORPUS unset)" (corpus half, as
designed; /tmp/my-project/corpus-v4/media does not exist in this sandbox).
— Fixtures present: 5 in tests/fixtures/mpeg12/ + ps fixtures in
crates/demux/fixtures/ps/ (ps-mp2-video-audio.mpg, ps-vob-ac3.vob).
— Baseline: cargo-test-workspace.txt:1281 runs the same suite.

## 3. Findings
F-001 [S3]
Where: crates/codecs/tests/mpeg12_corpus.rs:14-16 vs 489-494
Code:
//! - corpus: every `.ts`/`.mpg`/`.vob`/`.avi`/`.mp4`/`.mkv` lane
//!   whose first video stream is mpeg2video, demuxed through the
//!   matching landed demuxer, packets concatenated, facts scanned.
...
        let out = Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-i"])
            .arg(path)
            .args(["-map", "0:v:0", "-c", "copy", "-f", "data"])
What is wrong: the file header promises corpus lanes are demuxed through OUR
demuxers; the code shells out to ffmpeg for the ES extraction. The function's
own doc (L448-453) admits the truth ("the landed AVI demuxer is MJPG-only v0
and the MP4 demuxer maps `mp4v` to `mpeg4video` ... those two containers cannot
yet feed this parser their own bytes") — the header oversells. Why it matters:
a reader of the first 25 lines believes the mpeg2-in-AVI/MP4 corpus lanes
exercise Lumen demux paths; they exercise ffmpeg's. What correct looks like:
fix the header to match the function comment.
Confidence: HIGH — Verification: read both blocks.
F-002 [S4]
Where: crates/codecs/tests/mpeg12_corpus.rs:447-448 vs 492
Code:
/// elementary stream extracted with `ffmpeg -map 0:v:0 -c copy -f
/// mpegvideo` (the ps_fixtures byte-source precedent — ...
...
            .args(["-map", "0:v:0", "-c", "copy", "-f", "data"])
What is wrong: the comment names `-f mpegvideo`, the code passes `-f data`.
For mpeg2video both dump the same ES bytes, so no behavioral lie, but the
comment documents a different command than the one run. What correct looks
like: make them identical.
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/codecs/tests/mpeg12_corpus.rs:22-23, 457-459
Code:
//! When the corpus root (or ffprobe) is absent the corpus half
//! **skips loudly** (the h264-parser-suite precedent); the fixture
//! half always runs (committed bytes, pinned truth).
...
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP mpeg12 corpus: corpus-v4 absent (LUMEN_CORPUS unset)");
        return;
    };
What is wrong: "skips loudly" is only loud under `--nocapture` or on failure —
the default test harness swallows eprintln on pass (my first run's stderr was
0 lines; the SKIP was invisible). A green CI run silently exercises 0 corpus
lanes and nobody notices. What correct looks like: print to stderr is not
enough; fail or emit a machine-readable marker the CI summarizer surfaces.
Confidence: HIGH — Verification: ran both ways (0-line stderr vs --nocapture).
Also: the corpus half itself is UNVERIFIED here — corpus-v4 is absent in this
sandbox (reason: fixture not present), so its lane-walking logic has no
execution evidence from me (baseline environment equally skipped).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture_dir / ps_fixture_dir / corpus_dir | 38-55 | REAL | — | corpus default /tmp/my-project/corpus-v4 (absent here) |
| ffmpeg_available / ffprobe_video | 57-113 | REAL | — | oracle probe; N/A filtered |
| assert_facts | 118-183 | REAL | — | size/profile/level/pix_fmt/interlace/rate(r OR avg) vs oracle |
| fixture_raw_es_facts_match_ffprobe | 188-290 | REAL | — | executed: 4/4 lanes PASS live |
| scan_through_demuxer + DemuxerCore | 295-343 | REAL | — | PS only; `let _ = name;` dead param (nit) |
| open_via_input | 345-350 | REAL | — | input::FileAccess |
| PesTap / bad_lock / scan_ts_pes | 356-405 | REAL | — | TS PES-consumer seam, storm guards |
| fixture_transport_lanes_ts_and_ps | 407-444 | REAL | — | executed: TS + 2 PS lanes PASS live |
| corpus_mpeg2video_lanes_field_assertions | 455-526 | REAL-UNPROVEN | S3 | skipped here (corpus absent); ffmpeg extraction not our demuxers (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixture_raw_es_facts_match_ffprobe | Yes | pinned facts + live ffprobe double-assert; picture/slice/mb censuses; bad_headers==0 |
| fixture_transport_lanes_ts_and_ps | Yes | executed live (TS PES + PS lanes) |
| corpus_mpeg2video_lanes_field_assertions | Yes when corpus present | fail==0 + pass>=3 asserts; UNVERIFIED here (skip) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "asserted against **measured ffprobe truth** (ffmpeg 7.1.5 ...)" | L3-5 | REAL | my --nocapture run: 7/7 ROW PASS vs live ffprobe |
| "Decode is not attempted — the module is honest about that boundary" | L19-20 | REAL | no decode calls in file; mpeg12 has no decoder |
| "demuxed through the matching landed demuxer" (corpus) | L14-16 | FALSE | ffmpeg extraction (F-001) |
| "the fixture half always runs (committed bytes, pinned truth)" | L23-24 | REAL | executed |
| "skips loudly" | L22-23 | SHALLOW | harness capture hides it (F-003) |

## 7. Dead / duplicated / unreachable code
`let _ = name;` (L302) dead param. DemuxerCore enum is PS-only (single variant)
— the "thin enum bridges" comment anticipates more cores.

## 8. Dependencies & flags
codecs (mpeg12 facts), demux (ts/ps), input, media-types, plugin-api, testkit.
No feature gates. External: ffprobe/ffmpeg on PATH (present here, 7.1.5).

## 9. Open questions for the next agent
- Does corpus-v4 exist in the orchestrator's full environment? If yes, run
  `LUMEN_CORPUS=... mpeg12_corpus` and record the MPEG12 CORPUS row counts —
  the `pass >= 3` expectation ("2 avi + 1 mp4") has never been verified in
  this audit.
- headers.rs F-001 (vbv units): no ffprobe `buffer_size` is compared anywhere
  in this suite — the units question remains open with no oracle lane.
