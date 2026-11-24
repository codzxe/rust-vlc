# AUDIT: tests/fixtures/mjpeg_h3/mjpeg_interlaced_tff.avi
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A08) |
| Date | 2026-06-14 |
| File verdict | REAL — load-bearing codec-test fixture (pinned MD5, baseline-passing) |
| Reachable from shipped entry point? | NO — test fixture; consumed by crates/codecs/tests/mjpeg_field_differential.rs:91 (TRUTHS: w 160, h 112, frames 4, I420, md5 f89e3b6440f8f1062776ca25b06df25f) and :327 (polarity-free test loop) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3: 30,150 bytes, sha256 ced848152a1fa94e6d1abcdd89024304fa6c758c751653d763728944e195db9d; `file -b` -> RIFF AVI, 160 x 112, 5.00 fps, Motion JPEG; JPEG marker walk (4 SOI frames; APP0 JFIF + COM, NO APP8/AVI1; SOF0 baseline) — matches the polarity-free contract; baseline cargo-test-workspace.txt:1219-1223 (3/3 field-differential tests ok) |

Generated-vs-handmade: structural facts self-consistent — strh vids/MJPG rate 5, dwLength 4 == 4 SOI markers == TRUTHS frames=4; strf 160x112 24-bit sizeImage 53760; interlaced FIELD content encoded as progressive JPEGs without AVI1 polarity markers, exactly what `interlaced_content_fixtures_stay_polarity_free` asserts (no fabricated polarity). Provenance: ffmpeg-dump-differential family (module docs, MD5 measured equal to ffmpeg dump on I420 lanes).
Consumer runs GREEN in the orchestrator baseline: `fixture_lanes_match_pinned_truth ... ok` + `interlaced_content_fixtures_stay_polarity_free ... ok` (cargo-test-workspace.txt:1220-1221) — the fixture's decode output is MD5-pinned by a passing test.
F-001 [S3]: NOT registered in tests/fixtures/MANIFEST.toml (rg `mjpeg_interlaced_tff` over MANIFEST.toml → 0 hits) — outside the fixture registry/sha256 ledger entirely (the mjpeg_h3 family; same unregistration W16-A02 flagged for mjpeg_420_odd.avi). Provenance/drift protection comes only from the codec test's MD5 pin, not the corpus manifest. Where: tests/fixtures/MANIFEST.toml (absence). Confidence: HIGH — Verification: ran (rg).
