# AUDIT: crates/codecs/tests/adpcm_corpus_differential.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Lines / functions | 296 / 9 (2 Truth tables, corpus_dir, Capture sink, open_via_input, decode_file, check, 2 tests) |
| File verdict | REAL for the in-repo fixture half (executed bit-exact); the corpus half is a loud-skip gate that vacuously passes in this environment |
| Reachable from shipped entry point? | N/A (integration test); runs in the codecs suite — I executed the prebuilt binary |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `./target/debug/deps/adpcm_corpus_differential-b20f58c6c52dee82 --nocapture` -> 2 passed / 0 failed; fixture throughput lines printed (17.9 / 38.6 MB/s PCM); corpus row printed SKIP; `sha256sum` fixtures |

## 1. One-paragraph verdict
A real end-to-end differential: open through the actual `input::FileAccess` stack, demux through the real `WavDemuxer`, configure the real `AdpcmDecoder` with the track's fmt-chunk bytes, decode every packet, and require the concatenated PCM's MD5 to equal a committed ffmpeg-measured constant plus exact frame counts, channels, rate, descriptor, first-PTS-zero, and per-packet Emitted outcomes. I ran it: both in-repo fixtures pass bit-exactly in ~1ms each. The corpus half (0042/0102 stereo lanes, the only stereo coverage in this file) skips loudly ("SKIP" in the output) because /tmp/my-project/corpus-v4/media is absent — the same corpus-gate pattern as slice.rs, though this one at least prints an explicit SKIP and points at the worklog table. The comparison-window documentation (fact-chunk damage, data-derived window, ffmpeg parity) is precise and matches how the demuxer suite documents the same files.

## 2. What actually works (verified)
— `./target/debug/deps/adpcm_corpus_differential-b20f58c6c52dee82 --nocapture` -> **2/2 pass**. `committed_fixtures_decode_bit_exact` decoded adpcm_ima_mono.wav (12 246 frames, MD5 `8a814608…` pinned) and adpcm_ms_mono.wav (12 216 frames, MD5 `a95d2cbb…`) — both matching the committed constants, with the throughput lines proving real decode work ran.
— Pipeline depth: this is not a unit test of a decode function; it exercises input::FileAccess + WavDemuxer + AdpcmDecoder + FrameSink end-to-end (decode_file, L154-231), asserting per-packet `DecodeOutcome::Emitted { frames: 1 }` throughout — any concealment/Recovered on a clean file fails the test.

## 3. Findings
F-001 [S3]
Where: adpcm_corpus_differential.rs:284-296
Code:
#[test]
fn real_corpus_decodes_bit_exact() {
    let Some(dir) = corpus_dir() else {
        eprintln!(
            "corpus(adpcm): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
             — SKIP (the differential table for the 2026-10-24 instance is in the worklog)"
        );
        return;
    };
What is wrong: the only STEREO coverage in this file (0042 adpcm_ima_wav 2ch, 0102 adpcm_ms 2ch) is gated on an external corpus directory that does not exist here; the test passes vacuously while its truth table (168 822 / 128 524 frames, two more MD5s) stays unchecked. The skip is loud (better than slice.rs's plain "skipping") and points at the worklog, but the baseline "4744 passed" still counts it as a pass. Why it matters: stereo IMA/MS paths in the corpus window are evidence-by-worklog only in this environment. What correct looks like: `#[ignore = "corpus"]` so the skip is visible in the summary, or commit small stereo fixtures (the breadth suite already commits stereo payloads — yamaha_st_payload.bin etc. — so the mono-only commitment here is a choice, not a limitation). Confidence: HIGH — Verification: ran (SKIP line in output).
NOTE: the sibling `adpcm_breadth_differential.rs` (4/4 ran green earlier, incl. stereo swf/yamaha/qt lanes) substantially covers the same decoder in stereo — the corpus half's absence is a redundancy loss, not a total coverage hole. That is why this is S3, not S2.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| TRUTHS / CORPUS_TRUTHS tables | 62-98 | REAL | — | committed ffmpeg-measured constants; fixture half live-verified |
| corpus_dir | 102-108 | REAL | — | LUMEN_CORPUS override + sandbox default; None → skip |
| Capture (FrameSink) | 111-137 | REAL | — | concatenates PCM, counts frames, records first PTS; typed error on non-audio |
| open_via_input | 143-149 | REAL | — | real FileAccess path, typed URI refusal |
| decode_file | 154-231 | REAL | — | full pipeline; descriptor/channels/rate asserted pre-decode; per-packet Emitted asserted |
| check | 234-274 | REAL | — | frames + MD5 + PTS-zero; throughput eprintln (observability, no gate) |
| committed_fixtures_decode_bit_exact | 276-282 | REAL (ran) | — | bit-exact, both fixtures |
| real_corpus_decodes_bit_exact | 284-296 | SHALLOW in this env | S3/F-001 | loud skip; stereo corpus rows unchecked here |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| committed_fixtures_decode_bit_exact | YES (ran) | exact MD5 + frame count + per-packet outcome + descriptor/rate/channels |
| real_corpus_decodes_bit_exact | NO (in this env) | skip guard; would be YES with corpus present (same check fn) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decode bit-exactly against the ffmpeg 7.1.5 reference" | adpcm_corpus_differential.rs:1-5 | REAL (fixtures) / UNVERIFIED (corpus rows) | ran: fixture MD5s match; corpus rows skipped (F-001) |
| "ffmpeg decodes the data-derived window … and so does this decoder" (fact-chunk damage) | adpcm_corpus_differential.rs:9-17 | REAL-UNPROVEN | the window math (168_822×4=675_288, 128_524×4=514_096) is consistent and the fixture half executes; the corpus window itself unverified here |
| pinned truth table (4 rows) | adpcm_corpus_differential.rs:21-26 | 2 REAL (ran), 2 UNVERIFIED | as above |

## 7. Dead / duplicated / unreachable code
None. `common::md5_hex` shared with the breadth suite (its RFC-1321 correctness is pinned by `md5_implementation_matches_rfc1321_vectors`, which I ran green earlier).

## 8. Dependencies & flags
codec-api, demux (WavDemuxer), input (FileAccess), media-types, plugin-api, testkit. No features. Fixture hashes: adpcm_ima_mono.wav 7b854ad8…4763, adpcm_ms_mono.wav bc0a9010…19aa.

## 9. Open questions for the next agent
- The `fact`-chunk damage note claims the demuxer suite documents the same files — cross-check demux wav tests for the 0042/0102 rows when the corpus lands; if the corpus never lands in CI, these two Truth rows are dead data.
