# AUDIT: tests/fixtures/mjpeg_h3/mjpeg_422_odd.avi
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Binary fixture (valid AVI, MJPEG 4:2:2 odd dimensions, ffmpeg-authored) |
| Size / sha256-prefix | 18108 bytes / 19bf36b2d37bb168 (verified locally) |
| File verdict | REAL (consumed with pinned decode MD5) — no hash ledger |
| Reachable from shipped entry point? | N/A — test asset |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256, header sniff (`RIFF...AVI LIST...hdrlavih`), rg consumers |

## 1. Verdict
Real AVI fixture: MJPEG video at odd 4:2:2 dimensions (84x46 per the truth
table), built by ffmpeg (the suite header documents ffmpeg-generated
odd-size fixtures). Consumed by crates/codecs/tests/mjpeg_field_differential.rs
as a decode-differential lane (our decoder vs ffmpeg rawvideo dump) with an
RGBA MD5 pinned, and by the compat-matrix walker (root
`tests/fixtures/mjpeg_h3`, `.avi` extension) — named in
docs/corpus/compat-matrix-20261110.json/raw-rows.jsonl (rg hits).

## 2. What actually works (verified)
- Consumer (verbatim, crates/codecs/tests/mjpeg_field_differential.rs:82-89):
```
    Truth {
        name: "mjpeg_422_odd",
        w: 84,
        h: 46,
        frames: 5,
        format: DecodedFormat::Rgba8,
        md5: "4e44c744a51ac17897d2321265b6fb40",
    },
```
- Suite contract (mjpeg_field_differential.rs:1-16): "our converted output vs ffmpeg's `-pix_fmt rgba -sws_flags neighbor` dump" — a real cross-encoder differential, honesty documented (4:2:2 has no planar vocabulary; RGBA lane with per-side fixed-point color conversion).
- Baseline: codecs suite passing in audit/runs/baseline/cargo-test-workspace.txt (rg `mjpeg_field` -> suite ok; per-test UNVERIFIED, no cargo).

## 3. Findings
F-001 [S3] No manifest/ledger pins this file's bytes (MANIFEST.toml covers
only the 28 xtask rows; mjpeg_h3 family lives outside it). The decode-MD5
truth table indirectly locks the content, but nothing detects silent
fixture corruption before the MD5 mismatch. Confidence: MEDIUM (no direct
hash ledger found by rg over tests/ tools/ docs/).
