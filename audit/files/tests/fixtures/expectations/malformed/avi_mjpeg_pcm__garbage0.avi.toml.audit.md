# AUDIT: tests/fixtures/expectations/malformed/avi_mjpeg_pcm__garbage0.avi.toml
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| What | expectation/1 TOML (28 lines) for the class-B AVI garbage-append derivative: [derivation] garbage-append appended_bytes=4,096 past every declared structure end; [error] union contract with caps |
| Verification | python: fixture avi_mjpeg_pcm__garbage0.avi size 429,612 == size_bytes AND sha256 == d479e132...abe68 (True/True) — byte-exact pin verified; 429,612 − 425,516 = 4,096 matches appended_bytes (arithmetic cross-check consistent) |
| Consumers (rg) | crates/demux/tests/fixtures_class_b.rs (typed-outcome assertions; L335 "the walk never touches past-extent bytes" is the garbage-append contract test); crates/media-probe/tests/fixtures_matrix.rs; playback-engine soak scenario |
| Generated vs handmade | cargo xtask fixtures (declared, drift-gated) |
| Verdict | REAL. No findings.
