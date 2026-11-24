# AUDIT: tests/fixtures/mjpeg_h3/mjpeg_interlaced_bff.avi
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 30,266 bytes / ad4706ed6b6fc39f1a5555e67f8a03474442ceea63fbd1382b6c20344c936106 |
| Type | RIFF AVI, 160x112, 5.00 fps, MJPEG |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | sha256 batch; python3 avih parse; rg consumers |

## 1. Verdict
ffmpeg-generated interlaced-content MJPEG AVI (tinterlace interleaved content per crates/codecs/tests/mjpeg_field_differential.rs:318) — `mjpeg_h3/` dir = card 5.H3 fixtures. Header parsed: `avih` flags 0x910, 4 total frames, 25,000us/frame = 40fps-declared vs file(1) "5.00 fps" (file(1) computes from stream length; frame rate field not at odds internally — dwMicroSecPerFrame 25000 with 4 frames over the stream). Consumer: mjpeg_field_differential.rs:99 (`name: "mjpeg_interlaced_bff"` lane) and :327 `interlaced_content_fixtures_stay_polarity_free`. Baseline: all 3 tests of that suite passed (cargo-test-workspace.txt:1218-1220, incl. `interlaced_content_fixtures_stay_polarity_free ... ok`).
## 2. What actually works (verified)
python3 avih parse; consumer citations; baseline pass at :1218-1220.
## 3. Note
The "differential" in the suite name is vs PINNED truth (recorded ffmpeg 7.1.5 MD5s, module doc L23-25), not a live differential — honest, but the name oversells unless read with the doc.
