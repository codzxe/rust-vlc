# AUDIT: tests/fixtures/generated/malformed/mp4_h264_aac__flip16.mp4
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Binary fixture (malformed MP4 derivative: 16 seeded header bit-flips) |
| Size / sha256-prefix | 13703 bytes / 0445182384cf28b0 (verified locally) |
| File verdict | REAL (consumed) — not in MANIFEST |
| Reachable from shipped entry point? | N/A — test asset |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256, header sniff, rg consumers |

## 1. Verdict
Real MP4 derivative of the H.264+AAC parent. The flips hit the ftyp brand
hard: first 32 bytes read `\x00\x00\x00 ftypawml\x80@"\x10awml\xe93O"avc1mp41`
— 'isom' corrupted to 'awml' and minor-brand bytes flipped. Consumed by
crates/demux/tests/mp4_fixtures.rs, the compat-matrix walker, and named in
docs/corpus/compat-matrix-20261110.raw-rows.jsonl.

## 2. What actually works (verified)
- Consumer (verbatim, crates/demux/tests/mp4_fixtures.rs:225-231):
```
const MALFORMED: [&str; 14] = [
    "generated/malformed/mp4_h264_aac__flip1.mp4",
    "generated/malformed/mp4_h264_aac__flip4.mp4",
    "generated/malformed/mp4_h264_aac__flip16.mp4",
    "generated/malformed/mp4_h264_aac__garbage0.mp4",
    "generated/malformed/mp4_h264_aac__trunc0.mp4",
    "generated/malformed/mp4_h264_aac__trunc1.mp4",
    "generated/malformed/mp4_h264_aac__trunc2.mp4",
    "generated/malformed/mp4_qt__trunc0.mov",
```
- Matrix evidence: raw-rows.jsonl rg hit (exact name).

## 3. Findings
F-001 [S3] Not in tests/fixtures/generated/MANIFEST.toml — unpinned, no
drift gate (same family-wide gap as the mkv/webm siblings). Confidence: HIGH.
