# AUDIT: fuzz/corpus/fuzz_demux_mp3stream/seed_mp3_mpeg2_24k.mp3
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 6,476 B / f9e387c2b3fdddb69e23725bfe1c938a28c6e300798c3c7232d2737058ab92d7 / MP3 ID3v2.4, MPEG layer III v2 64kbps 24kHz stereo (batch manifest) |
| Generated vs handmade | Real MP3 stream bytes (ID3v2.4 tag + MPEG2 LIII frames per file(1)); provenance undocumented — UNVERIFIED authorship |
| Consumer | cargo-fuzz campaign only (fuzz_demux_mp3stream target, nightly tier, zero committed run evidence — see fuzz_demux_avi.rs audit F-002) |
| Verdict | REAL bytes, SHALLOW reach. F-001 [S3]: never consumed by any committed run.
