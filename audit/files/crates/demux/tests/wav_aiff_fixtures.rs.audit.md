# AUDIT: crates/demux/tests/wav_aiff_fixtures.rs
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-23 |
| Lines / functions | 720 / 8 tests + 2 drain helpers + 2 truth structs |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — integration test (`tests/` dir), run by cargo test; all 8 pass in baseline :2436-2450 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | python3 independent MD5 verification of all 10 fixture pins (all OK); python3 byte reads; rg over tools/xtask/src/cmd/fixtures.rs; sed quote checks; baseline citation :2436-2450 |

## 1. One-paragraph verdict
This is the strongest kind of test file in the repo: real committed
ffmpeg-produced fixture bytes on disk (all 10 valid + 18 malformed
derivatives verified present under tests/fixtures/generated/), pinned
MD5/byte-total/error-code/damage-reason outcomes, sticky-replay checks, and
a cross-format fuzz-harness pass over every file. I independently
re-verified every one of the 10 MD5 pins with python3 hashlib against the
raw chunk payloads — all match, so the "measured this session, never
transcribed by hand" claim holds for the constants. The tests would fail
under real regressions (exact `assert_eq!` on payload hashes, reason
vectors, terminals). Defects are two S4 observability nits. Notably: like
the whole WAV surface, no per-packet pts is asserted here either (confirms
wav/tests.rs F-001 repo-wide).

## 2. What actually works (verified)
- Independent MD5 re-verification (my command, python3 hashlib RIFF/FORM
  walk): all 10 fixtures — e.g. `wav_pcm_u8_44k1_mono.wav md5:OK len:OK
  (8820 bytes)`, `wav_extensible_6ch_48k.wav md5:OK len:OK (115200 bytes)`,
  `aifc_sowt_s16le_44k1_st.aifc md5:OK len:OK (35280 bytes)` — every
  pinned constant in TRUTHS (76-157) matches the actual fixture bytes.
- All 8 tests executed ok: baseline cargo-test-workspace.txt:2438-2450
  (`every_wav_fixture_matches_the_ffmpeg_truth ... ok` etc., 8 passed,
  0 failed, 0.01s).
- Fixtures exist on disk: tests/fixtures/generated/valid/ (10 files) and
  generated/malformed/ (9 wav + 9 aiff derivatives) — `ls` verified.
- Module-doc claim "the xtask generator does not own them": consistent —
  rg over tools/xtask/src/cmd/fixtures.rs (5341 lines) finds no wav/aiff
  generation (MEDIUM; the --check drift-gate logic itself not traced).

## 3. Findings
F-001 [S4]
Where: crates/demux/tests/wav_aiff_fixtures.rs:181 (and 201)
Code:
```
            Ok(DemuxEvent::Recovered { reason, .. }) => reasons.push(reason),
            Ok(DemuxEvent::Discontinuity) => {}
            Ok(DemuxEvent::EndOfStream) => break,
```
What is wrong: the drain helpers swallow `Discontinuity` events silently —
if the WAV/AIFF demuxers started emitting unexpected discontinuities, no
assertion in this suite would notice (they are not counted, collected, or
asserted absent). Every other event class is pinned. Confidence: HIGH —
Verification: read.
F-002 [S4]
Where: crates/demux/tests/wav_aiff_fixtures.rs:67-68
Code:
```
    /// ffprobe `duration_ts` (sample frames for PCM streams; the
    /// `fact`/COMM declaration for FLAC-in-WAV).
```
What is wrong: the comment documents a FLAC-in-WAV truth case, but TRUTHS
contains no FLAC fixture (10 entries: 6 wav PCM/RF64 + 4 aiff/aifc) — a
leftover from the corpus suite's vocabulary; mildly misleading. The
FLAC-in-WAV coverage lives in wav/tests.rs and corpus_wav.rs instead.
Confidence: HIGH — Verification: read (all 10 Truth entries inspected).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Truth struct + TRUTHS const | 56-157 | REAL | — | 10 pins; all independently re-verified (python3 hashlib) |
| md5_impl_matches_rfc1321_vectors | 161-169 | REAL | — | pins the test-side MD5 to RFC 1321 A.5 before any differential use — correct ordering of trust |
| drain_wav / drain_aiff | 173-210 | REAL | S4 | F-001 Discontinuity swallow |
| every_wav_fixture_matches_the_ffmpeg_truth | 216-281 | REAL | — | codec/base/span/frames/md5/byte-total/no-damage/clean-EOS — strict |
| every_aiff_fixture_matches_the_ffmpeg_truth | 283-330 | REAL | — | same strictness (no byte-total check; md5 + frames suffice) |
| rf64_fixture_carries_the_ds64_truth | 334-346 | REAL | — | guards the RF64 path actually being taken |
| WavDerivative + WAV_DERIVATIVES | 352-448 | REAL | — | 9 pinned outcomes incl. exact reason ORDER |
| wav_mutated_family_outcomes_are_pinned_exactly | 450-496 | REAL | — | sticky replay pinned (477-482); start-failure vs walk-failure discriminated |
| AiffDerivative + AIFF_DERIVATIVES | 498-594 | REAL | — | 9 pinned outcomes |
| aiff_mutated_family_outcomes_are_pinned_exactly | 596-641 | REAL | — | twin of the WAV family test |
| derivative_payloads_are_parent_prefixes | 646-694 | REAL | — | byte-slice prefix proofs against parents (hardcoded offsets 78/54, documented) |
| every_wav_and_aiff_fixture_survives_the_fuzz_harnesses | 701-720 | REAL | — | 28 files × both harnesses; total-over-any-bytes property exercised |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 8 | YES | exact md5/len/reason/terminal assertions over real committed bytes; I re-derived the pins independently and they match the fixtures |
| (gap) pts continuation | not covered | no pts assertion in this file either (confirms wav/tests.rs F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "demux bit-exactly against the recorded ffmpeg raw-dump truth" | :1-6 | REAL | baseline :2438-2450 pass + my independent MD5 re-verification of all 10 pins |
| "Every pinned value below was measured this session … never transcribed by hand" | :18-21 | REAL-UNPROVEN for the measurement act (cannot rerun ffmpeg in sandbox — no ffmpeg binary claim verified), REAL for the values: my hashlib re-derivation matches every pin |
| "verified this session: truncations are exact parent prefixes at the recorded cut offsets" | :38-40 | REAL | derivative_payloads_are_parent_prefixes (646-694) asserts exactly this, passing in baseline |
| "extra files are invisible to it [xtask --check] by design" | :15-16 | REAL-UNPROVEN | rg shows no wav/aiff in tools/xtask/src/cmd/fixtures.rs; drift-gate logic not fully traced |

## 7. Dead / duplicated / unreachable code
drain_wav and drain_aiff are near-duplicate bodies (173-210) — acceptable
for two demuxer types without a trait-object helper; noted as duplication,
not flagged.

## 8. Dependencies & flags
Integration test target; dev-deps: testkit (fixture_path/bytes, Must),
input (open_via_input — the real access stack), demux, media_types,
plugin_api. No feature gates. Uses `mod common` (tests/common — the RFC
1321 MD5 lives there, pinned by test at 161-169).

## 9. Open questions for the next agent
- tests/common (not assigned to me): the MD5 implementation itself — the
  RFC vectors pin it, low risk.
- corpus_wav.rs (my next file): whether the randomized corpus re-uses these
  pins or adds independent coverage.
