# AUDIT: fuzz/corpus/fuzz_codec_mjpeg/seed_avi_mjpeg_422.avi
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A05) |
| Date | 2026-11-24 |
| File verdict | REAL (data) — execution UNWIRED on stable |
| Reachable from shipped entry point? | NO — only the cargo-fuzz campaign consumes it; campaign not wired into any gate (fuzz/Cargo.toml:5-9) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 batchmeta.py + full RIFF walk + JPEG SOF decode; rg "seed_avi_mjpeg_422" (zero consumers) |

## 1. Verdict (data audit)
Real, structurally valid MJPEG-in-AVI seed: 18,108 bytes, sha256
19bf36b2d37bb16873a362fcbe5c7d4f8a95b3dea558a1ed154790db6a49ee9b, RIFF size exact
(0x46B4 = file − 8). Parsed: `LIST hdrl` with avih (84×46, 5 total frames, 100000 µs/frame),
one `strl` — `strh` handler **MJPG**, `strf` BITMAPINFOHEADER 84×46 bitcount 24; `LIST movi`
with **5 × 00dc chunks each a complete JPEG (SOI ffd8 … EOI ffd9)**, JFIF APP0 present, plus a
well-formed `idx1` (80 B = 5×16). First frame's SOF0: 84×46, 3 components with Y 2×2 / Cb 1×2 /
Cr 1×2 sampling — chroma at half horizontal, full vertical resolution = **4:2:2, the filename
is accurate**. Consumer: `fuzz/fuzz_targets/fuzz_codec_mjpeg.rs` via cargo-fuzz convention.
Generated-vs-handmade: machine-generated (JFIF/idx1/JUNK fingerprint of an ffmpeg mux).

F-001 [S3] Dead on the stable lane: no test/gate/script references `seed_avi_mjpeg_422`
(rg: zero hits — the mjpeg codec tests use the different
tests/fixtures/generated/valid/avi_mjpeg_pcm.avi family); the fuzz campaign itself is "a
*scaffold* … NOT wired into any gate" (fuzz/Cargo.toml:5-9) and needs a nightly toolchain the
stable-pinned repo cannot run. Committed corpus bytes with no automated consumer.
Confidence: HIGH — Verification: read (rg + parse); execution UNVERIFIED.
(Sibling seed_mp4_mjpeg_pcm.mp4 in the same dir is a separate file, not in my assignment.)
