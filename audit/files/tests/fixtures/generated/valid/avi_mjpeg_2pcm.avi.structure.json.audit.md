# AUDIT: tests/fixtures/generated/valid/avi_mjpeg_2pcm.avi.structure.json
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | data (structure-map/1), 3404 bytes |
| File verdict | REAL (60/60 chunk rows verified against the real AVI bytes) |
| Reachable from shipped entry point? | NO (test artifact; in the manifest drift gate) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 json-vs-bytes verification (exit 0) |

## 1. Verdict
The structure map is EXACTLY right, verified by me against the real AVI: all 4 top-level
chunks (RIFF/hdrl/movi/idx1 offsets+fourccs) match; ALL 60 movi chunk rows (20 ×
{00dc video, 01wb audio-1, 02wb audio-2}) match the on-disk fourcc + size at the recorded
offsets — 60/60 verified. Cross-artifact closure: audio sums 384 000 / 88 200 bytes ==
expectation TOML's `audio_streams_bytes = [384_000, 88_200]` == 20 × 19 200 (48 kHz stereo
s16 @10 fps) / 20 × 4 410 (44.1 kHz mono u8). sha256 2f0d059fb27379ff... (recomputed, MATCH
with MANIFEST.toml:31 block).

## 2. Consumers
The class-B derivative cut/patch positions derive from this map (per the note in-file and
CORPUS.md:27-28); `cargo xtask fixtures --check` byte-compares it (in-manifest row).
Expectations and demux tests consume the parent it describes.

## 3. Findings
None. This artifact is the strongest data contract in the fixture tree: byte-verified,
cross-consistent, drift-gated.

## 4. Open questions
None.
