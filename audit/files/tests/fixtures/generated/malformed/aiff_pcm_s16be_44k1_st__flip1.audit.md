# AUDIT: tests/fixtures/generated/malformed/aiff_pcm_s16be_44k1_st__flip1 (no extension)
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | binary, 35,334 B / n/a (assignment chunks #part1..#part5, L1-3165) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture; consumed by crates/demux/tests/wav_aiff_fixtures.rs:573 AIFF_DERIVATIVES) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 byte-diff parent vs derivative; sed of consumer test |

Data audit: class-B derivative of `generated/valid/aiff_pcm_s16be_44k1_st.aiff` (35,334 B). Verified: identical length, EXACTLY one differing byte at offset 18 (00→01), identical past 40. Offset 18 is the 3rd byte of the AIFF COMM chunk-size field: 0x00000012 (18) → 0x00000112 (274) — a COMM size lie that makes the chunk walk jump past the real SSND. The consumer test pins exactly this: wav_aiff_fixtures.rs:569-575 `name: "flip1" … terminal: "demux.aiff.no-ssnd", fails_at_start: true` (comment at :568-569 names byte 18 and 0x12→0x112 — matches my independent byte dump `434f4d4d00000112`).
F-001 [S3]: no file extension on the fixture (also all aiff/wav/ogg derivatives); probe must rely on content sniffing. No expectation TOML exists for this fixture (expectations/malformed has only avi/mp4_mjpeg_pcm families — 59/81 malformed fixtures lack TOMLs while doc 10 §5.4 claims "every fixture has" one and G6 fails a fixture without a ledger row). The pin lives only in the Rust test table. Confidence: HIGH — Verification: read.
