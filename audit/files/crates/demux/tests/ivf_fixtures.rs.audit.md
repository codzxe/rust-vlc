# AUDIT: crates/demux/tests/ivf_fixtures.rs
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-02-14 |
| Lines / functions | 304 / 5 tests + 2 helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (integration test). Demuxer under test reachable: `lumen open` → probe (`IvfSignature`, signature/mod.rs:141) → containers/mod.rs:55 `"ivf" => ivf::open`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 (zero — checked list in §3) |
| Commands run | `target/debug/deps/ivf_fixtures-4619cabe8c2451e3` → 5 passed, 0 failed (exit 0); `ffmpeg -i <both fixtures> -c copy -f md5 -` → MD5s match the pinned constants exactly; `ffprobe -count_packets …` on av1 fixture → codec/fourcc/dims/fps/nb_read_packets all match TRUTHS |

## 1. One-paragraph verdict
A real, executed, externally-corroborated differential. The 5 tests ran green in this sandbox
(0.05 s — the ~9 k truncation iterations and 1024 LCG mutation rounds genuinely executed over
the committed 4 483 bytes), and I independently re-derived the "recorded ffmpeg truth" with the
sandbox's own ffmpeg 7.1.5: both pinned payload MD5s reproduce byte-identically
(`0db6161ace241fb5966eaea0e3339aac`, `e62f0aba008315efcb91734112aad24e`) and the ffprobe stream
parameters match. The documented divergences (short-final-frame vs typed refusal, header-only
refusal, VP80 scope refusal, hlen≠32) are each pinned by an assertion, not just prose. No
fakeness, no env-gating, no transcription risk left unproven.

## 2. What actually works (verified)
- `target/debug/deps/ivf_fixtures-4619cabe8c2451e3` → `test result: ok. 5 passed; 0 failed` (exit 0).
- Independent oracle re-derivation (my commands, this sandbox):
  - `ffmpeg -v error -i av1_64x48_30fps.ivf -c copy -f md5 -` → `MD5=0db6161ace241fb5966eaea0e3339aac` == TRUTHS[0].payload_md5
  - `ffmpeg -v error -i vp9_64x48_25fps.ivf -c copy -f md5 -` → `MD5=e62f0aba008315efcb91734112aad24e` == TRUTHS[1].payload_md5
  - `ffprobe -count_packets` → av1/AV01/64x48/30/1/30 packets — all equal to TRUTHS[0] fields.
- Fixture files committed and present (`ls tests/fixtures/generated/ivf/`: 1 937 + 2 546 bytes,
  matching the doc's "≈ 4.5 KiB total").

## 3. Findings
None. What was checked and how:
- Fixture existence + sizes — `ls -la tests/fixtures/generated/ivf/` (present, sizes match doc).
- Truth constants not fabricated — re-derived with live ffmpeg/ffprobe (§2, all match).
- Test execution — prebuilt binary run, 5/5 green (§2).
- Mutation coverage real — read: every truncation `1..len` (test 3), 512 rounds × 1-8 LCG bit
  flips × 2 fixtures + step-7 truncation sweep through the fuzz harness (test 5); shift/index
  arithmetic in the LCG verified in-range (`lcg % 8 < 8`, `mutant.len() ≥ 1`).
- Divergence honesty — read: each of the 5 documented divergences has a pinning assertion
  (keyframe-never-invented at line 194; frame-beyond-file at 240/275; no-frames/bad-header in
  the whitelist at 221-228).
- Test-honesty mutation reasoning: breaking the demuxer's pts ladder, md5 payload, parameter
  harvest, or typed-error vocabulary each fails an exact assert in tests 1-4.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Truth/TRUTHS | 77-114 | REAL | — | constants independently re-derived (§2) |
| demux_fixture | 119-141 | REAL | — | real access stack; recovered/discontinuity forbidden on clean input |
| stream_parameters_match_ffprobe | 143-174 | REAL | — | 7-field parity + tick base ratio |
| frame_counts_pts_ladders_and_payload_md5_match_ffmpeg | 176-205 | REAL | — | full ladder 0..N−1 + payload md5 |
| truncated_derivatives_are_typed_never_panics | 207-251 | REAL | — | every cut, typed-code whitelist |
| crafted_frame_beyond_file_refuses_where_ffmpeg_shortens | 253-276 | REAL | — | divergence pinned (29 packets + typed refusal) |
| fuzz_lite_over_the_real_ffmpeg_bytes | 278-304 | REAL | — | deterministic LCG flips + truncation sweep |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 5 | YES | exact parameter/ladder/md5/code assertions over committed bytes; executed green; oracle constants reproduced by me with live ffmpeg |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the committed ffmpeg-generated AV1/VP9 fixtures demux **bit-exactly** against the recorded ffmpeg truth" | ivf_fixtures.rs:1-4 | REAL | tests executed + MD5s re-derived with sandbox ffmpeg (§2) |
| "Measured this session … never transcribed by hand" | ivf_fixtures.rs:25-27 | REAL | my independent re-derivation matches every constant |
| the 5 honest divergences | ivf_fixtures.rs:45-64 | REAL | each pinned by an assertion (§3) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
- `common::{md5_hex, open_via_input}` (real access stack), `demux::ivf::{IvfDemuxer, fuzz,
  test_support}`, `testkit`. No env gates (contrast adts_corpus.rs), no features.

## 9. Open questions for the next agent
- None for this file. (For the demuxer itself: `demux.ivf` header/`hlen` handling is asserted
  only through the truncation sweep's `bad-header` whitelist — the crafted `hlen = 64` case the
  doc mentions is pinned in the unit suite, not here; someone auditing src/ivf/mod.rs should
  confirm.)
