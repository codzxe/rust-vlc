# AUDIT: tests/fixtures/generated/valid/aifc_raw_u8_44k1_mono.aifc
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Binary fixture (valid AIFF-C, raw u8 PCM, 44.1k mono) |
| Size / sha256-prefix | 8892 bytes / 85ae4c5103a4fbc3 (verified locally) |
| File verdict | REAL (consumed with pinned truth) — not in MANIFEST |
| Reachable from shipped entry point? | N/A — test asset |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256, header sniff (`FORM...AIFC FVER COMM`), rg consumers |

## 1. Verdict
Real valid AIFF-C fixture (FVER version chunk present, COMM 18 bytes, raw
u8 mono 44.1k). Consumed by crates/demux/tests/wav_aiff_fixtures.rs with an
exact frame count + payload MD5 pinned, by apps/cli/src/containers/aiff.rs,
and named in docs/corpus/compat-matrix-20261110.json (lines 1603, 1907 — rg
hits). 8892 bytes = header + 8820 u8 samples — self-consistent.

## 2. What actually works (verified)
- Consumer (verbatim, crates/demux/tests/wav_aiff_fixtures.rs:150-156):
```
    Truth {
        fixture: "generated/valid/aifc_raw_u8_44k1_mono.aifc",
        codec: "pcm_u8",
        rate: 44_100,
        channels: 1,
        frames: 8_820,
        md5: "d5c54db7b4ade5332cfd96a9566a4ef4",
    },
```
- docs/worklog/archive-2026-10.md:10358 records the cross-check: "aifc_raw_u8_44k1_mono 1ch 44.1k -> 8820 (exact) — 4/4 exact vs the ffmpeg-measured truths".
- Compat-matrix: `.aifc` IS in media_extensions → walked by the valid-root.

## 3. Findings
F-001 [S3] Not in tests/fixtures/generated/MANIFEST.toml — unpinned (ffmpeg-
authored family; see sibling audits). Confidence: HIGH.
