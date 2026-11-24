# AUDIT: tests/fixtures/generated/valid/ogg_opus_48k_mono.opus
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Binary fixture (valid Ogg/Opus, ffmpeg-authored) |
| Size / sha256-prefix | 1526 bytes / ebd5e40071a50579 (verified locally) |
| File verdict | REAL (consumed with pinned decode truth) — not in MANIFEST |
| Reachable from shipped entry point? | N/A — test asset |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256, header sniff (`OggS...Opus` — OpusHead), rg consumers |

## 1. Verdict
Real valid Ogg/Opus fixture (mono 48 kHz, 21 packets, per the pinned truth
table; header shows OpusTags at the expected offset). ffmpeg-authored
(encode argv recorded in docs/worklog/archive-2026-10.md:8587:
"ogg_opus_48k_mono.opus (516290875)"), so — unlike the xtask fixtures —
these bytes are external-encoder output; CORPUS.md's "no external encoder
output" claim applies only to its own 28-file table, and this file lives
outside that table.

## 2. What actually works (verified)
- Consumer (verbatim, crates/demux/tests/ogg_fixtures.rs:165-170):
```
    Truth {
        fixture: "generated/valid/ogg_opus_48k_mono.opus",
        codec: "opus",
        rate: 48_000,
        packets: 21,
        granule: 19_512,
        md5: "3e6f08d3445bc0faa0e77cc686475569",
```
(decode MD5 pinned against the demuxer output — a real differential anchor.)
- Also consumed by apps/cli/tests/native_seek.rs, apps/cli/src/containers/ogg.rs, crates/media-probe/tests/registry_roundtrip.rs, crates/codecs/src/audio/opus/decoder.rs (rg hits) — the same bytes feed the codec-side tests.
- Compat matrix: valid-root walker collects `.opus`? `oga`/`ogg` are listed in media_extensions; `.opus` is NOT in the extension list (compat-matrix.json) — so the compat walker SKIPS this file (extension mismatch). Matrix raw-rows.jsonl does not name it. Not a defect of the fixture itself; recorded as matrix-coverage gap in the ogg-vorbis sibling audit.

## 3. Findings
F-001 [S3] Not in tests/fixtures/generated/MANIFEST.toml; `.opus` missing
from tests/compat/compat-matrix.json media_extensions so the compat-matrix
walker never collects it. Confidence: HIGH (read both).
