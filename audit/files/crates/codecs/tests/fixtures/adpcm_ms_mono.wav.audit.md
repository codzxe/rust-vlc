# AUDIT: crates/codecs/tests/fixtures/adpcm_ms_mono.wav (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-R2 (recovery; credit W06-A07) |
| Date | 2026-11-23 |
| Size / type | 6,268 bytes — `RIFF (little-endian) data, WAVE audio, Microsoft ADPCM, mono 8000 Hz` |
| md5 / sha256 | 424718120444a0078904495ee07fa87d / bc0a9010d55715a9b960d3080db068d3ca28b9ba3c5e07a5f3b8033109e019aa |
| Structure (my parse) | fmt tag=2 (MS ADPCM), mono, 8000 Hz, block_align 1024, 4-bit, cbSize 32 (standard 7 coef pairs); fact chunk (nominal 12000); data chunk 6144 B = exactly 6 × 1024 B blocks |
| File verdict | REAL — ffmpeg-authored Microsoft ADPCM mono 8 kHz fixture; pinned decode bit-exact and reproduced against the live oracle |
| Reachable from shipped entry point? | N/A (test fixture; adpcm_corpus_differential.rs:71-78 TRUTHS row + adpcm_fuzz_survival.rs:112 substrate) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file`; python3 RIFF chunk parse; `ffmpeg -f md5` + `-f s16le \| wc -c`; `./target/debug/deps/adpcm_corpus_differential-b20f58c6c52dee82 --nocapture` → 2/2; `./target/debug/deps/adpcm_fuzz_survival-4bcf0cfa52427a86` → 4/4 (1.00s) |

## 1. One-paragraph verdict
Genuine, name-faithful Microsoft-ADPCM WAV: my own RIFF parse confirms fmt tag 2 (MS ADPCM), mono, 8 000 Hz, 1 024-byte block align with the standard 32-byte coef extension, and a 6 144-byte data chunk = exactly the documented "6 × 1024 B" packet shape (docs/worklog/archive-2026-10.md:8943). Every pinned fact reproduced live with the sandbox ffmpeg 7.1.5: decoded PCM MD5 `a95d2cbb66f65a4adcb36b57bc68e48e` == the TRUTHS pin (adpcm_corpus_differential.rs:77), and 24 432 s16le bytes == 12 216 samples == the pinned `frames = 12_216` (:76) — the honest data-derived window (the WAV's `fact` chunk says a nominal 12 000; 6 blocks × 2 036 samples/block = 12 216 is what the bytes actually decode to, on both sides).

## 2. What actually works (verified)
- `ffmpeg -i adpcm_ms_mono.wav -map 0:a -c:a pcm_s16le -f md5 -` → `MD5=a95d2cbb66f65a4adcb36b57bc68e48e` — byte-exact vs the pin; `-f s16le | wc -c` → 24 432 (12 216 mono samples).
- `adpcm_corpus_differential` 2/2, lane output captured with `--nocapture`: `adpcm-differential[adpcm_ms_mono.wav]: 12216 frames in 886.404µs — 0.02 MB PCM decoded (4× file bytes), throughput 27.6 MB/s of PCM` — the repo decoder ran this exact file and the bit-exact MD5 assert held (`committed_fixtures_decode_bit_exact`). The corpus lane SKIPs loudly (absent dir) — disclosed, unrelated to this fixture.
- `adpcm_fuzz_survival` 4/4: this fixture is a mutation substrate (`["adpcm_ima_mono.wav", "adpcm_ms_mono.wav"]`, adpcm_fuzz_survival.rs:112) — unmutated deterministic decode + mutated-block typed-outcome survival both green.
- Prebuilt-binary caveat as in the av1 fixture audits (cargo-mtime staleness; fixture bytes read fresh from disk at runtime).

## 3. Findings
None. Checked: RIFF structure vs name (MS ADPCM mono 8 kHz), block geometry vs documented 6 × 1024, coef-table presence, fact-chunk nominal vs data-derived sample count (decoder window honesty), pinned MD5/frame-count vs live ffmpeg reproduction, both consuming suites executed green, doc citations.

## 4. Doc citations
- docs/formats/audio-adpcm.md:46 — "| fixture adpcm_ms_mono.wav | mono | 12 216 | bit-exact |" — re-verified (12 216 samples, bit-exact assert executed).
- docs/worklog/archive-2026-10.md:8943 — "| tests/fixtures/adpcm_ms_mono.wav (ffmpeg `-c:a adpcm_ms`, same source) | adpcm_ms | 1/8000 | 6×1024 B | 12_216 | `a95d2cbb66f65a4adcb36b57bc68e48e` ✓ |" — every field re-verified here (chunk parse + live oracle).

## 5-9. (binary asset — tables N/A; consuming suites' pins hold; no dead code; no deps; no open questions)
