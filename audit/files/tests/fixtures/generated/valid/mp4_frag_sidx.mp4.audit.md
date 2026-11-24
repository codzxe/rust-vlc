# AUDIT: tests/fixtures/generated/valid/mp4_frag_sidx.mp4
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Binary fixture (valid fragmented MP4 with sidx) |
| Size / sha256-prefix | 14253 bytes / 7c63db8d40a95392 (verified locally) |
| File verdict | REAL (consumed with pinned truth) — not in MANIFEST |
| Reachable from shipped entry point? | N/A — test asset |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256, header sniff (`ftypiso5` + `sidx` box family), rg consumers |

## 1. Verdict
Real valid fixture: ISO-BMFF `iso5` brand (fragmented-MP4 profile) with a
`sidx` segment index (bytes 8-16: `\x00\x00\x04\xc6` = sidx box size).
Consumed by crates/demux/tests/mp4_fixtures.rs as a `Valid` row asserting
h264+aac tracks and `fragmented: true`, by the compat-matrix walker, and
named in docs/corpus/compat-matrix-20261110.json/raw-rows.jsonl (rg hits).

## 2. What actually works (verified)
- Consumer (verbatim, crates/demux/tests/mp4_fixtures.rs:63-70):
```
    Valid {
        fixture: "generated/valid/mp4_frag_sidx.mp4",
        codecs: &[
            ("h264", media_types::MediaKind::Video),
            ("aac", media_types::MediaKind::Audio),
        ],
        fragmented: true,
        chapters: 0,
    },
```
- Matrix evidence: `rg mp4_frag_sidx docs/corpus/compat-matrix-20261110.json` -> 2 hits (lines 1277, 2286) and raw-rows.jsonl -> hit.

## 3. Findings
F-001 [S3] Not in tests/fixtures/generated/MANIFEST.toml (xtask gate covers
only the 28 avi/mp4 non-frag rows) — unpinned bytes. Confidence: HIGH.
