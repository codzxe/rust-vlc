# AUDIT: crates/codecs/tests/h264_sw_differential.rs
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2025-06-27 |
| Lines / functions | 651 / 13 fns + 17 tests |
| File verdict | REAL — executed against the live ffmpeg oracle in this sandbox |
| Reachable from shipped entry point? | Test-only file (not shipped); the decoder it exercises (`H264SwDecoder` = `rust_h264` 0.6.1 engine behind codec-api) is production code — wiring audited under sw.rs (not in my list) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `timeout 240 ./target/debug/deps/h264_sw_differential-9b87180403a77d06` -> exit 0, "17 passed; 0 failed", 29.65s, stderr EMPTY (no SKIP lines → live oracle ran); baseline cargo-test-workspace.txt:1163-1182 same 17/17 |

## 1. One-paragraph verdict
The hostile-focus question — "differential against WHAT oracle?" — has a clean
answer: against ffmpeg 7.1.5 itself, twice. Each lane pins an MD5 of ffmpeg's
own `-f rawvideo -pix_fmt yuv420p` decode of the committed fixture, and when
ffmpeg is on PATH the test re-decodes the fixture LIVE and asserts three-way
equality (our dump == live ffmpeg dump == pinned constant, lines 342-352). I ran
the prebuilt test binary in this sandbox — /usr/bin/ffmpeg 7.1.5 exists, stderr
is empty (the absent-ffmpeg SKIP path at line 354 never fired), 17/17 passed:
the bit-exact claim is verified against the real oracle, executed by me, on
both entry forms (Annex-B via the real `demux::es` h264 demuxer, MP4/AVCC via
the real `Mp4Demuxer`). This is the honest pattern the ALAC suite uses, plus
the live leg ALAC lacks. The engine is the admitted external `rust_h264`
"=0.6.1" (sw.rs:7), not an in-house decoder — the module docs say so plainly.
Defects are cosmetic: one decorative tautological assert, a truncation sweep
that covers one packet's prefixes, and a test-side mirror of CLI box-walking.

## 2. What actually works (verified)
— `./target/debug/deps/h264_sw_differential-9b87180403a77d06` -> exit 0,
"test result: ok. 17 passed; 0 failed; 0 ignored" (29.65s);
audit/runs/W06-A08.h264_sw_diff.{stdout,stderr}; stderr has 0 lines → the
live ffmpeg re-check (L342-352) executed on all 12 lane tests, not skipped.
— Fixtures present: 6 lanes × (.h264 + .mp4) in tests/fixtures/h264/ (ls).
— Baseline corroborates: cargo-test-workspace.txt:1163-1182, 17/17 ok, 24.74s.

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/h264_sw_differential.rs:549-563
Code:
        (sink.frames, Vec::<String>::new())
    };
    assert!(errors.is_empty());
What is wrong: `errors` is a hard-coded empty Vec, so `assert!(errors.is_empty())`
asserts a literal. Typed decode errors still fail the test — but via `.must()`
panics at L554/L558, not via this assert. Why it matters: the assert is
decorative; a reader believes errors were collected and checked. What correct
looks like: collect `err.context.code` in the loop like `decode_all` (L228-234)
does, then assert.
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/codecs/tests/h264_sw_differential.rs:517-541
Code:
fn hostile_truncation_sweep_is_panic_free() {
    let packets = es_packets("A_baseline_ip_320x240_45f");
    let first = packets.iter().find(|p| p.data.len() > 64).must();
    ...
    for cut in 0..bytes.len() {
What is wrong: the doc comment promises "every truncation of a real packet
must be panic-free" but the sweep covers ONE packet (the first >64 bytes) ×
its prefixes. The other 44 packets are never truncated. Why it matters: the
name promises more coverage than the body delivers. Mitigation: h264/fuzz.rs
(file 17 of this assignment) and the engine's own fuzzing exist; this is the
lite tier. What correct looks like: iterate all packets (or say "one packet"
in the name/comment).
Confidence: HIGH — Verification: read + ran (passed as written).

F-003 [S4]
Where: crates/codecs/tests/h264_sw_differential.rs:148-179
Code:
/// One ISO-BMFF child box header (declared size, fourcc) at `pos`,
/// `None` past the entry's end (the CLI host's `mp4.rs` walker,
/// mirrored test-side).
fn child_header(entry: &[u8], pos: usize) -> Option<(usize, [u8; 4])> {
What is wrong: `find_child_box`/`child_header` duplicate the CLI host's
avcC-harvest convention (fixed prefixes 28/76/78). If the host convention
drifts, the test keeps passing against a stale mirror — the classic
copy-paste-test failure mode. What correct looks like: reuse a shared helper
from the demux crate's API.
Confidence: MEDIUM — Verification: read (host source not examined; UNVERIFIED
that the host's walker matches — outside my file list).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture_path | 117-121 | REAL | — | env!(CARGO_MANIFEST_DIR) |
| mem_stream | 124-126 | REAL | — | demux crate test helper |
| es_packets | 131-146 | REAL | — | real demux::es::h264 demuxer, verified by run |
| child_header / find_child_box | 151-179 | REAL | S4 | test-side mirror (F-003) |
| mp4_packets | 185-209 | REAL | — | real Mp4Demuxer + avcC harvest, verified by run |
| decode_all | 214-237 | REAL | — | configure→decode→flush contract path |
| video_bytes | 240-251 | REAL | — | asserts CPU-frame shape |
| ffmpeg_reference | 255-272 | REAL | — | LIVE oracle; presence-probed; empty-on-failure is fail-loud downstream |
| cached_run | 284-304 | REAL | — | OnceLock cache; one 25-30s run shared by all tests |
| assert_lane | 310-356 | REAL | — | count/size/format/md5 + 3-way live re-check |
| raw_packet / configured | 455-481 | REAL | — | |
| 17 tests | 358-651 | REAL | — | all pass, live oracle confirmed (§2) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| lane_*_annexb / lane_*_mp4 (12) | Yes | bit-exact MD5 vs pinned + LIVE ffmpeg re-decode; any decode drift, crop bug, reorder bug fails |
| lane_c ES pts-Missing + MP4 pts-increasing | Yes | pins reordering semantics on both wires |
| hostile_garbage_packets_recover | Yes | vocabulary assert (NeedMoreInput/Recovered only) |
| hostile_truncation_sweep_is_panic_free | Yes (for packet #1) | prefix sweep asserts recovery vocabulary |
| hostile_garbage_then_real_recovers | Yes | 45 frames + exact lane-A MD5 after garbage — strong resync proof |
| contract_state_rules | Yes | typed codes h264.sw.not_configured / codec_descriptor / stale-gen / Drained |
| contract_damaged_extradata_refuses_typed | Yes | h264.sw.extradata* + clean re-configure |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decoded ... on BOTH entry forms ... compared against the ffmpeg 7.1.5 libx264 oracle" | L1-8 | REAL | my run: 17/17, live oracle, empty stderr |
| "both entry forms pin the SAME MD5 constant, re-verified live when ffmpeg is on PATH" | L21-26 | REAL | L342-352 executed (no SKIP) |
| "ffmpeg decodes both byte-identically (measured this session)" | L23-24 | REAL | 3-way assert passed for both forms |
| "the off-by-one truncation sweep is panic-free" | L30-31 | REAL | sweep passed (one packet — F-002) |
| pinned MD5s (6 lanes) | L65-102 | REAL | each compared to live ffmpeg output in my run |

## 7. Dead / duplicated / unreachable code
None dead. Duplicated: box-walk mirror (F-003).

## 8. Dependencies & flags
rust_h264 "=0.6.1" via codecs crate (admitted dependency,
docs/dependencies/records/rust_h264-0.6.1.md); demux, media-types, plugin-api,
codec-api, testkit. No feature gates on this test.

## 9. Open questions for the next agent
- The sw.rs wrapper (not in my list): does `Recovered { CorruptInput }`
  accounting really count every skipped NAL, or only some? The garbage tests
  here only assert the vocabulary, not the counts.
- rust_h264 0.6.1's own conformance coverage vs the 6-lane claim — the
  dependency record may say; UNVERIFIED here.
