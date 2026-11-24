# AUDIT: fuzz/corpus/fuzz_playlist_cue/seed_cue_hostile.cue
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| Size / sha256 / type | 61 B / 636f31579d991a08e0de013c6747d82db78c633219c2807c1f7c5d0d26e1c914 / ISO-8859 text (batch manifest) |
| Generated vs handmade | Read in full: maximal hostility in 61 bytes — non-UTF-8 filename bytes, TRACK 99, MODE2/2352, INDEX 99 99:99:99 (out-of-range everywhere). Hand-authored quality. |
| Consumer | cargo-fuzz campaign only (fuzz_playlist_cue target, nightly tier, zero committed run evidence — see fuzz_demux_avi.rs audit F-002) |
| Verdict | REAL seed bytes, SHALLOW reach. F-001 [S3]: never consumed by any committed run.
