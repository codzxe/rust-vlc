# AUDIT: crates/codecs/tests/fixtures/flac_mono_s16_44100.flac (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Type / size / sha256 | FLAC audio bitstream, 16-bit mono 44.1 kHz, 11025 samples (per `file`) / 12243 bytes / aa5969e3636b5aa46e885c6060e4928dd1df5ebbe4e7e51521eb417134bb7aa2 |
| File verdict | REAL (load-bearing fixture; byte-exact pins hold live; fixture sha256 itself pinned in the reference data) |
| Reachable from shipped entry point? | N/A (test fixture; the decoder it exercises — claxon-backed FLAC — is shipped, `"flac"` CLI arm) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. Blunt verdict
A genuine FLAC fixture whose identity is doubly pinned: the reference data (`crates/codecs/tests/data/flac_bitexact_reference.txt:61-69`) records the fixture's own sha256 — which matches my `sha256sum` byte-for-byte (no fixture drift) — plus 3 frames, 1 channel, 44100 Hz, 16 bps, 11025 total samples, and byte-exact s16le/s32le/streaminfo MD5s. The suites asserting those pins pass live.

## 2. Evidence
- Consumers (rg): `crates/codecs/tests/flac_bitexact.rs:44,170` (byte-exact lanes), `flac_fuzz_survival.rs:72,500` (mutation base), `flac_vlc_differential.rs:140-141` (the VLC-or-pending suite), `crates/media-probe/tests/registry_roundtrip.rs:204`.
- Doc evidence: the reference file's own provenance comment (L28: authored via `ffmpeg -c:a flac -sample_fmt s16 -compression_level 8`); prior agent's flac/mod.rs audit independently python-parsed this fixture's frame layout.
- Ran live: `timeout 90 ./target/debug/deps/flac_bitexact-0363eae80343785c --test-threads=2` → `4 passed; 0 failed` (incl. `fixtures_decode_byte_exact_against_the_ffmpeg_reference`).

## 3. Notes
- `first_samples` pin (0,4608,9216,...) — the fixture also carries the seek-table lane's expectations; the seektable test passed in the same run.
- No provenance concern: ffmpeg-authored test fixture, documented in-repo.
