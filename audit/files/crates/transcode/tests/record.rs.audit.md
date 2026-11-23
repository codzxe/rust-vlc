# AUDIT: crates/transcode/tests/record.rs
| Field | Value |
|---|---|
| Auditor | W10-A07 |
| Date | 2026-11-27 |
| Lines / functions | 584 / 5 tests (+9 helpers/doubles) |
| File verdict | REAL (as a test suite) — proves an UNREACHABLE feature |
| Reachable from shipped entry point? | N/A (test file); the `Recorder` it proves has no production caller (see runner.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg (Recorder callers, fixture_path) exit 0; sed -n verifications exit 0; ls/file on fixtures exit 0; baseline citation |

## 1. One-paragraph verdict
An honest, strong acceptance suite: recording a committed WAV fixture is verified by
re-demuxing the OUTPUT with the tree's own demuxer and asserting byte-identity of the
payload (decoder-independent) plus bit-exact re-decode; the s16→s32 widening test
asserts per-sample `i32::from(n) << 16 == wide` (value preservation, not just length);
refusals (no decoder, no audio track) assert typed error codes; the cancel test uses a
scripted mpsc demuxer, a bounded `eventually` poll (no sleep-and-hope), and checks the
partial file's RIFF/data sizes against the actual bytes AND re-demuxes it. All 5 tests
pass in the baseline. The problem is what it proves: `Recorder` is part of the
transcode crate, which no shipped app depends on — this suite is high-quality evidence
for a feature users cannot reach.

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt (transcode PACKAGE, tests/record.rs):
`test result: ok. 5 passed; 0 failed` (record-1521879f9b9a5709 line).
— verified by `ls`: both fixtures exist (tests/fixtures/generated/valid/
wav_pcm_s16le_44k1_st.wav 35358 bytes; wav_pcm_f32le_48k_st.wav 76892 bytes) and
`file` confirms RIFF/WAVE PCM 16-bit stereo 44100 Hz.
— verified by `rg "Recorder" crates/ apps/`: Recorder referenced only inside
crates/transcode — no production caller.

## 3. Findings
F-001 [S3]
Where: crates/transcode/tests/record.rs:1-13 (suite scope) + transcode/src/record.rs:154
Code:
```
//! The card 7.E2 record acceptance suite (issue #44 AC): the
//! [`Recorder`] one-call assembly records a committed fixture to
//! WAV, and the recording is verified by OUR own demux + decode —
```
What is wrong: the suite is genuine, but `Recorder` (record.rs:154-230) is reachable
only from this suite — the whole transcode crate is absent from every shipped app
manifest (proof in runner.rs audit F-001). If any feature-matrix/doc row cites "card
7.E2 record suite passing" as evidence that RECORDING works in the product, that is
inflation: the capability exists and is tested but is not wired to any entry point.
Confidence: HIGH — Verification: read (rg over apps/, manifests).
F-002 [S4]
Where: crates/transcode/tests/record.rs:3-4
Code: `//! WAV, and the recording is verified by OUR own demux + decode —\n//! bit-exact against the source's own decoded samples.`
What is wrong: self-referential verification (the producer's sibling verifies the
output). Mitigated: L311 additionally asserts raw payload byte-identity
(`assert_eq!(round.payload, demuxed.payload, "payload identity")`), which is
decoder-independent, so the core claim does not rest on the shared decoder. The
widening test cross-checks sample VALUES. Residual risk: a WAV-header bug shared by
writer and demuxer would pass both. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/transcode/tests/record.rs:299
Code: `    assert_eq!(stats.sample_frames_written, 8_820);`
What is wrong: golden literal duplicating fixture knowledge (35358-44 bytes / 4 =
8820.5→8820 frames) — a fixture swap silently breaks the test with a magic-number
diff. Harmless but brittle; deriving from `demuxed.payload.len()` (already computed)
would self-document. Confidence: HIGH — Verification: read (arithmetic re-derived).

## 4. Per-function table (tests + doubles)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SharedCursor (Write+Seek) | 41-86 | REAL | — | resize-on-demand shared sink; test double |
| MemStream (SourceStream) | 89-145 | REAL | — | in-memory, size-known, typed seek errors |
| open_fixture / demux_all / decoded_bytes | 161-221 | REAL | — | real FileAccess + real WavDemuxer + real PcmDecoder |
| FlattenSink | 223-235 | REAL | — | silently skips non-audio frames (test-only, fine) |
| FixturePcm / NoDecoder factories | 239-269 | REAL | — | fixture-seeded; refusal factory typed |
| recorder_output_redemuxes_and_redecodes_bitexactly | 275-312 | REAL | — | payload identity + bit-exact + stats |
| recorder_widening_target_records_losslessly | 316-372 | REAL | — | per-sample <<16 value check |
| recorder_surfaces_the_factory_refusal_typed | 381-397 | REAL | — | typed code + stage asserted |
| recorder_without_audio_fails_typed | 401-442 | REAL | — | selection refusal precedes decoder open |
| ScriptedDemuxer + eventually + mid_record_cancel | 449-584 | REAL | — | mpsc parking, bounded poll, honest-prefix RIFF checks, re-demux |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| recorder_output_redemuxes_and_redecodes_bitexactly | YES | byte-identity of payload is independent of the writer's own claims; stats cross-checked |
| recorder_widening_target_records_losslessly | YES | per-sample value assertion `i32::from(*narrow) << 16 == *wide` |
| recorder_surfaces_the_factory_refusal_typed | YES | exact context.code + stage |
| recorder_without_audio_fails_typed | YES | exact context.code, VideoOnlyDemuxer double |
| mid_record_cancel_leaves_an_honest_playable_prefix | YES | Cancelled outcome + whole-packet prefix bound [1,5] + RIFF/data sizes equal actual bytes + re-demux succeeds; bounded `eventually` (10 s deadline) instead of sleeps |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "records a committed fixture to WAV ... bit-exact" | record.rs:2-4 | REAL | test 1 passes (baseline), payload identity |
| "a source with no audio track fails typed" | record.rs:5-6 | REAL | test 4: `transcode.source.audio-track-missing` |
| "a mid-record cancel leaves a playable honest prefix (sizes patched ...)" | record.rs:6-8 | REAL | test 5: sizes == actual bytes, prefix re-demuxes |
| (implied) recording is a shipped capability | card 7.E2 framing | UNREACHABLE | no app depends on transcode (F-001) |

## 7. Dead / duplicated / unreachable code
Test doubles are local by stated policy ("integration suites do not share code across
files", L36-38) — duplication of wav_roundtrip.rs shapes is deliberate, recorded.

## 8. Dependencies & flags
dev-deps: codec-api, codecs (PcmDecoder), demux (WavDemuxer), input (FileAccess),
media-types, plugin-api, testkit, transcode. No features. No unsafe. No env reads.

## 9. Open questions for the next agent
- Does the feature matrix cite card 7.E2 record evidence for a shipped platform row?
  If yes, that row is UNREACHABLE — flag to the matrix-audit agent.
