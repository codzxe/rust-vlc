# AUDIT: crates/codecs/tests/alac_bitexact.rs
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2025-06-27 |
| Lines / functions | 272 / 4 fns + 1 test |
| File verdict | REAL — executed; all 5 pinned oracle MD5s independently reproduced by my own ffmpeg runs |
| Reachable from shipped entry point? | NO — integration test (exercises the shipped ALAC path: input → Mp4Demuxer → AlacDecoder) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/alac_bitexact-c46b612c74b2627b` -> 1/1 ok; `ffmpeg -i <4 fixtures> -f s16le/s24le - | md5sum` -> all four match the pins; sed verification |

## 1. One-paragraph verdict
Clean, honest, and now doubly verified. The suite decodes all five committed
ALAC fixtures through the real composition (input FileAccess → Mp4Demuxer →
codec-api AlacDecoder with the sample-entry cookie) and asserts: packet counts
(8/8/9/9/13), channels, rates, pts monotonicity, interleaved layout, decoded
extent (duration × rate × channels × width, exact), and the whole-output PCM
MD5 against pinned ffmpeg dumps. I ran the test (passes) AND independently
reproduced all five pinned MD5s with this sandbox's ffmpeg 7.1.5 —
e33ff912590e04d68117017a1a2ccd23, a2a11b40df939a1d604fd71233cf83e8,
04ca6310cf048ddca4f12374b1c3849e, 109eea10ed4c881a5516143c817f1595,
1ee6321067a82b421e33038f4fc00b70 (noise fixture, verified in its own asset
audit) — every pin is a real ffmpeg output, not a self-golden. The only gap:
unlike h264_sw_differential, this suite never re-runs ffmpeg live, so a future
ffmpeg-behavior change would not be caught at test time (the pins are frozen
truth; that is a defensible choice, but it is a choice).

## 2. What actually works (verified)
— `./target/debug/deps/alac_bitexact-c46b612c74b2627b` -> "fixtures_bitexact_vs_ffmpeg
... ok", 1/1 (0.15s).
— My independent oracle runs (ffmpeg 7.1.5-0+deb13u1):
  alac_stereo16 -f s16le md5 = e33ff912590e04d68117017a1a2ccd23 (== pin)
  alac_mono16  = a2a11b40df939a1d604fd71233cf83e8 (== pin)
  alac_stereo24 -f s24le = 04ca6310cf048ddca4f12374b1c3849e (== pin)
  alac_mono24 = 109eea10ed4c881a5516143c817f1595 (== pin)
  alac_stereo24_noise = 1ee6321067a82b421e33038f4fc00b70 (== pin, verified in
  the fixture's asset audit).
— Baseline corroborates (cargo-test-workspace.txt green for this suite).

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/alac_bitexact.rs:245-259 (assert block)
Code:
        let ours = md5_hex(&pcm);
        assert_eq!(
            ours, truth.md5,
            "{}: whole-output PCM MD5 vs ffmpeg {} dump",
            truth.fixture, truth.dump
        );
What is wrong: the oracle is pinned constants only — no live ffmpeg re-check
when ffmpeg is on PATH (the h264_sw_differential.rs:341-355 pattern). A
regenerated fixture with a decoder-drifted pin would still pass, and a future
ffmpeg behavior change goes unnoticed. Mitigation: I verified all five pins
against live ffmpeg today, so as of this audit the pins are genuine; the gap is
only about future drift. What correct looks like: add the optional live
re-check leg the h264 suite has.
Confidence: HIGH — Verification: read + my own live ffmpeg reproduction of
every pin.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixture_path | 118-121 | REAL | — | |
| open_via_input | 128-137 | REAL | — | shared SourceUri constructor, typed refusal propagated |
| Capture (FrameSink) | 140-170 | REAL | — | pcm concat + pts monotonicity |
| decode_file | 175-238 | REAL | — | full composition; asserts single audio track, `alac` descriptor, every packet Emitted{1}; cookie parse + output_format preview |
| fixtures_bitexact_vs_ffmpeg | 240-272 | REAL | S4 | executed; pin-only oracle (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixtures_bitexact_vs_ffmpeg | Yes | whole-output MD5 vs external-oracle pins (independently reproduced by me), exact decoded-extent length, packet counts, pts monotonicity |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decode **bit-exactly** against the ffmpeg 7.1.5 reference, through the full L1→L3 composition" | L1-5 | REAL | my test run + my independent ffmpeg reproductions of all 5 pins |
| "No synthetic fixture anywhere in the path" | L5-6 | REAL | real m4a bytes through real demuxer |
| "8 packets per file incl. the final **partial frame**" (0.7s lanes) | L4-5 | REAL | packet counts asserted (8/8/9/9/13 — the 24-bit lanes are 9, the table itself says so) |
| "the prior wave-5 session's suspected engine bug (alac.frame.truncated ...) is **not reproduced** here" | L35-38 | REAL-UNPROVEN | the tripwire exists and passes; the historical bug's absence is what the run shows |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
codecs::audio::alac (symphonia-codec-alac engine), demux::mp4, input,
codec-api, media-types, plugin-api, testkit. No feature gates.

## 9. Open questions for the next agent
- The audio-alac format doc's evidence table cites these pins; nothing further
  to chase — this suite is the strongest pattern in the audio side (would be
  even stronger with the live re-check leg).
