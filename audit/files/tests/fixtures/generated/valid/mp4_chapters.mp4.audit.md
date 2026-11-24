# AUDIT: tests/fixtures/generated/valid/mp4_chapters.mp4
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 24839 bytes |
| File verdict | REAL (chpl parsed byte-level: 3 titled chapters, monotone starts) |
| Reachable from shipped entry point? | NO (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 box-tree walk + chpl hexdump (exit 0); rg consumers (exit 0) |

## 1. Verdict
Genuine chaptered MP4: top boxes ftyp(32)/free(8)/mdat(22345)/moov(2454); moov = mvhd + 3 trak
+ udta(157) whose chpl box (size 60) carries 3 entries — verbatim hex at offset 24783:
`...\x00\x00\x00<chpl\x01\x00\x00\x00\x00\x00\x00\x00\x03...` then entries `05 'First'`,
`0x5b8d80`/`06 'Second'`, `0xe4e1c0`/`05 'Third'` — three titled chapters, strictly monotone
timestamps. sha256 81ca6cd5a47b4a07... (recomputed).

## 2. Consumers (test honesty cross-check)
- crates/demux/tests/mp4_fixtures.rs:72 pins `chapters: 3` for this fixture; :180-190
  `chapters_fixture_surfaces_titled_markers` asserts `chapters.len() == 3`, non-empty titles,
  monotone starts — the fixture's byte content (verified by me) matches the pinned truth.
- apps/cli/src/containers/mp4.rs:613-619 asserts the chapter trak is HIDDEN from facts.tracks
  (len 2) — matches the 3-trak box tree (one text trak is tref/chap-referenced).
- docs/corpus jsonl/json cite it with recorded outcomes (rg, exit 0).

## 3. Findings
F-001 [S3] — same systemic gap: not in MANIFEST.toml's 28-file drift gate; no generator row
in tools/xtask/src/cmd/fixtures.rs; CORPUS.md:10-11's tree-wide authorship claim FALSE here.
Confidence HIGH. Verification: ran.

## 4. Open questions
- chpl timestamps 0x5b8d80/0xe4e1c0 (6M/15M units): if the demuxer reads them as ms, chapter
  starts land at 100/250 min on a 22 KB mdat — units convention worth a W-next look at the
  chpl parser (crates/demux mp4 code, not this file's defect).
