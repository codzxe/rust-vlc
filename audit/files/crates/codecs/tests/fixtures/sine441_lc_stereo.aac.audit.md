# AUDIT: crates/codecs/tests/fixtures/sine441_lc_stereo.aac (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Type / size / sha256 | MPEG ADTS AAC v4 LC 44.1 kHz stereo (`file` + ffprobe concur) / 25219 bytes / 1c934f5b8f71aacea8890f13390e8662c422077b1dbd781b5884e179bc4c0b6e |
| File verdict | REAL (differential decode evidence with recorded tolerance, executed) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; `ffprobe` -> aac/LC/44100/2ch; `sha256sum`; `./target/debug/deps/aac_bitexact-1e99e0008b8875c2 --nocapture` -> 2 passed / 0 failed, 1.11s |

## 1. Verdict
Real ffmpeg-authored ADTS AAC-LC fixture, one of two committed lanes in the AAC bit-exact suite. The consuming test (`fixtures_decode_the_recorded_window_exactly`, aac_bitexact.rs:38) decodes it through the real pipeline and compares against `tests/data/aac_bitexact_reference.txt`, whose header records the honest lossy-codec posture: sample-exactness is NOT claimed — the recorded tolerance is max delta 1 LSB vs ffmpeg's decode (measured 84 samples of 134 144 = 0.03% differ by 1 LSB for this fixture, per the worklog table). The suite ran green when I executed it, including the `lc_frames_are_exactly_1024_samples` shape check.

## 2. Test usage (verified by execution)
`./target/debug/deps/aac_bitexact-1e99e0008b8875c2 --nocapture` -> **2/2 pass** (1.11s — real decode of both fixtures). Also feeds `aac_corpus_differential.rs` (corpus-independent lane: 131 packets → 134 144 sample frames through the real provider, per worklog 10402) and `aac_bench.rs` (perf numbers committed under docs/perf/raw/r1: 1082× realtime).

## 3. Matrix evidence
This fixture is the codec-card AAC-LC lane's in-repo evidence (no corpus dependency — the corpus-independent lane per the worklog). The reference file pins the fixture bytes via the recorded ffmpeg PCM MD5 (a re-imaged corpus fails loudly), so the differential is against fixed bytes, not a moving target. No inflation for this asset.
