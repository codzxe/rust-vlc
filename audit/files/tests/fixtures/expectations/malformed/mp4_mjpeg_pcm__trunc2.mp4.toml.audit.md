# AUDIT: tests/fixtures/expectations/malformed/mp4_mjpeg_pcm__trunc2.mp4.toml
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| What | expectation/1 TOML (28 lines) for the class-B MP4 truncation derivative: [derivation] truncate cut_offset=2,208 "cut at the first mdat sample (mdat header present, no samples)"; [error] union contract with caps |
| Verification | python: fixture mp4_mjpeg_pcm__trunc2.mp4 size 2,208 == size_bytes AND sha256 == 7216c90c...d26a (True/True) — byte-exact pin verified; size == cut_offset (a full prefix truncation, consistent) |
| Consumers (rg) | crates/demux/tests/fixtures_class_b.rs (MP4 arm asserts truncated/typed outcomes); crates/media-probe/tests/fixtures_matrix.rs ([derivation] reader — "not a transcription", L246); playback-engine soak scenario |
| Generated vs handmade | cargo xtask fixtures (declared, drift-gated) |
| Verdict | REAL. No findings.
