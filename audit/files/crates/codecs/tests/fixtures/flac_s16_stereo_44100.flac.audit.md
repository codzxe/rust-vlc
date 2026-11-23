# AUDIT: crates/codecs/tests/fixtures/flac_s16_stereo_44100.flac
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | binary (14581 bytes) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture; flac_bitexact.rs:43 lane `flac_s16_stereo_44100` + flac_vlc_differential/flac_fuzz_survival/flac_seek suites) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | file; ffprobe; ffmpeg pcm_s16le decode -> md5sum (== reference pin); rg consumer trace |

## 1. One-paragraph verdict
Genuine FLAC fixture, name-faithful to the field: `file` → "FLAC audio bitstream data, **16 bit, stereo, 44.1 kHz, 22050 samples**" (= 0.5 s); ffprobe agrees (flac, 44100, 2ch, 16 bits-per-raw-sample). My independent ffmpeg decode yields s16le MD5 `e0f96555d1465acbd566af4259f52c92` — exactly the `pcm_s16le_md5` (and `streaminfo_md5`) recorded in tests/data/flac_bitexact_reference.txt:56-58 for `[fixture.flac_s16_stereo_44100]`. The consuming flac suites ran green in baseline.

## 2. What actually works (verified)
- file/ffprobe facts == filename (s16 stereo 44100).
- `ffmpeg -i flac_s16_stereo_44100.flac -map 0:a -c:a pcm_s16le -f s16le - | md5sum` → `e0f96555d1465acbd566af4259f52c92` == flac_bitexact_reference.txt:56 pin (sed-verified).
- Consumers: flac_bitexact.rs (base lane; the separate `_seektable` sibling fixture is another file), flac_vlc_differential, flac_fuzz_survival, flac_seek.

## 3. Findings
None. Checked: container type, codec/depth/channels/rate vs name, decode cleanliness, MD5 vs reference pin, consumers.

## 4-9. (binary asset — tables N/A; consuming suites' pins hold; no dead code; no deps; no open questions)
