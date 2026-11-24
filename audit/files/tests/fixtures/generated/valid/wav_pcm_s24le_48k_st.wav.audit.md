# AUDIT: tests/fixtures/generated/valid/wav_pcm_s24le_48k_st.wav (parts 1-6, L1-4068)
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 57,702 bytes / 09d09d8bb2cdd69b9c089dc4981dd823a7003c53ef2c708b1b475838958e4c37 |
| Type | RIFF WAVE, stereo 48000 Hz |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 header parse; rg consumers |

## 1. Verdict
WAVE_FORMAT_EXTENSIBLE (fmt tag 65534): 2 ch, 48,000 Hz, 288,000 B/s, 6-byte block, 24-bit; data chunk 57,600 bytes = 9600 frames × 2ch × 3B. Consistent with filename. RIFF size field 57,694 = file-8. Consumers: crates/demux/tests/wav_aiff_fixtures.rs:94; crates/transcode/tests/wav_roundtrip.rs:337,510 (`fixtures_roundtrip_decode_encode_redemux_redecode_bitexactly` — baseline ok, audit/runs/baseline/cargo-test-workspace.txt:6429-6434); crates/demux/src/wav/format.rs:549 documents that this fixture exercises the 40-byte extensible fmt path (cbSize). Committed-generated.
## 2. What actually works (verified)
python3 header parse (fmt tuple above); baseline suites pass (cargo-test-workspace.txt:2436-2441, 6429-6434).
