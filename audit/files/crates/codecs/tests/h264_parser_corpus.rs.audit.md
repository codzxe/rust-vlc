# AUDIT: crates/codecs/tests/h264_parser_corpus.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-23 |
| Lines / functions | 444 / 9 helpers + 2 tests |
| File verdict | REAL (verified live with an ad-hoc corpus; VACUOUS-PASS when the corpus is absent — which is the default state) |
| Reachable from shipped entry point? | Test: NO. The parser under test (`codecs::video::h264::parser::{nal,sps,pps}`) is production code, reachable via the shipped CLI's h264 lanes (pipeline_host.rs:346 H264SwDecoder::new / hw lane). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | (1) `./target/debug/deps/h264_parser_corpus-bf10919714fec14e --nocapture` (no corpus) → 2 passed, 0 failed, both tests printed SKIP and did nothing. (2) With `LUMEN_CORPUS=/tmp/w06a03/corpus` (ad-hoc corpus I built: 3 committed .mkv fixtures + 2 committed .h264 fixtures renamed to the expected 0019/0020 names): raw lane → `ROW 0019-raw-video.h264: PASS profile=Constrained Baseline level=13 size=Some((320, 240)) chroma=Yuv420 pps=true` (live ffprobe comparison, incl. the 2000-round mutation property pass); mkv lane → 3 lanes `3 PASS, 0 FAIL, 0 SKIP` (real FileAccess + MkvDemuxer + avcC/in-band parse vs live ffprobe), then `assert pass >= 10` FAILED (my corpus has 3 lanes — the assertion fires as designed). |

## 1. One-paragraph verdict
A real field-level differential: our clean-room H.264 NAL/SPS/PPS parser is compared against live ffprobe (profile, level, resolution, pix_fmt) through both entry forms (raw Annex-B split; mkv CodecPrivate avcC with in-band and nonstandard Annex-B fallbacks), driven by the real input/demux stack. I proved the machinery works end-to-end by supplying a small ad-hoc corpus: every lane passed against live ffprobe and the built-in lane-count floor genuinely failed when the corpus was smaller than expected. The damning part is the default state: corpus-v4 does not exist in this sandbox (and nothing in-tree creates it), so BOTH tests silently SKIP and cargo reports them as "ok" — two of the baseline's "4744 passed" that verified nothing in that run. The skip is documented and eprintln'd (visible only with --nocapture), which keeps this short of fraud, but the pass-count inflation is real.

## 2. What actually works (verified)
- Live-oracle run (my corpus): `ROW 0019-raw-video.h264: PASS profile=Constrained Baseline level=13 size=Some((320, 240)) chroma=Yuv420 pps=true`; `H264 MKV CORPUS: 3 lanes, 3 PASS, 0 FAIL, 0 SKIP`.
- The 2000-round Annex-B mutation property pass (byte flips, truncations; parse must return Ok/Err, never panic) ran inside the passing raw test.
- The `pass >= 10` floor fired on a 3-lane corpus — the assertions are live, not decorative.
- avcC parsing, in-band length-prefixed fallback, and the Annex-B-in-mkv last resort all executed without panics across the 3 mkv lanes.

## 3. Findings
F-001 [S3]
Where: crates/codecs/tests/h264_parser_corpus.rs:229-243 (and 313-332)
Code:
```rust
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP h264 raw corpus: corpus-v4 absent (LUMEN_CORPUS unset)");
        return;
    };
```
What is wrong: when the corpus (default `/tmp/my-project/corpus-v4`, or `$LUMEN_CORPUS`) is absent — the state of this machine and of any CI lane that does not image the corpus — the test returns early and cargo reports `ok`. Verified: default run shows `SKIP ... corpus-v4 absent` for both tests yet `test result: ok. 2 passed`. These vacuous passes are indistinguishable from real ones in the standard test output and are counted in the orchestrator baseline's 4744-passed total. The header documents the skip ("skips loudly", L20-22) — loudly only under `--nocapture`. What correct looks like: `#[ignore]` with a corpus-present lane, or panic/skip! semantics that mark the outcome distinctly. Confidence: HIGH — Verification: ran (both modes).
F-002 [S4]
Where: crates/codecs/tests/h264_parser_corpus.rs:35-41
Code:
```rust
    let dir = std::env::var("LUMEN_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/my-project/corpus-v4"));
```
What is wrong: the corpus root defaults to a machine-specific absolute path outside the repo (`/tmp/my-project/...`); nothing in-tree regenerates it, so the suite's default invocation on any fresh clone is the vacuous pass of F-001. Confidence: HIGH — Verification: ran (path absent).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| corpus_dir | 35-41 | REAL | S4 | F-002 machine-specific default |
| ffmpeg_available | 43-50 | REAL | — | ffprobe probe |
| Probe / ffprobe_video | 52-93 | REAL | — | live ffprobe field extraction; executed |
| ffprobe_visible_pix_fmt | 103-112 | REAL-UNPROVEN | — | gray 4:0:0 fallback mapping — not exercised by my corpus (no monochrome lane); UNVERIFIED |
| Parsed / parse_avcc | 114-146 | REAL | — | executed on mkv lanes |
| parse_inband_avc | 148-174 | REAL | — | executed (fallback path) |
| assert_facts | 176-227 | REAL | — | builds PASS/FAIL row; profile/level/size/pix_fmt/pps-id checks; executed live |
| corpus_raw_annexb_0019_field_assertions (test) | 229-311 | REAL | S3 | F-001 vacuous default; live PASS + 2000-round property pass when corpus present |
| corpus_mkv_h264_tracks_field_assertions (test) | 313-443 | REAL | S3 | F-001; 3-lane live PASS verified; `pass >= 10` floor fires correctly; packet-storm guard |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| corpus_raw_annexb_0019 (corpus present) | YES | profile/level/size/pix_fmt/pps asserted against live ffprobe; nals_seen > 2; mutation property loop |
| corpus_mkv_h264_tracks (corpus present) | YES | per-lane assert_facts + `fail == 0` + `pass >= 10` (floor verified to fire) |
| both (corpus absent — the default) | NO | early return; reports ok having checked nothing |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "asserted against **live ffprobe truth** — profile, level, resolution, chroma/pix_fmt" | header L2-4 | REAL (with corpus) | my live run: 4/4 lanes PASS vs ffprobe |
| "When the corpus root (or ffprobe) is absent the suite **skips loudly**" | header L20-22 | SHALLOW | "loudly" = eprintln visible only under --nocapture; cargo still reports ok (F-001) |
| "MP4/MOV/TS lanes ... skipped with a message ... sibling cards not yet merged" | header L15-18 | REAL | only .mkv/.webm filtered at L326; h264 .mp4 fixtures exist in-tree but are consumed by h264_sw_differential.rs instead |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Workspace crates (codecs parser, demux::mkv, input, plugin-api, media-types, testkit) + external ffprobe subprocess. Env knob `LUMEN_CORPUS`. No feature gates; runs in the default lane (vacuously, per F-001).

## 9. Open questions for the next agent
- Where is the worklog "PASS/FAIL table" the header references (card 5.F1)? If the recorded table is the only evidence the ≥10-lane corpus ever ran, that evidence is non-reproducible from this tree.
- The gray-4:0:0 ffprobe fallback (L103-112) needs a monochrome corpus lane to be verified — none exists in-tree.
- Whether CI images corpus-v4: the baseline numbers suggest the vacuous mode is what runs.
