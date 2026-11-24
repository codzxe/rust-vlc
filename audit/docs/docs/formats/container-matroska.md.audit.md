# CLAIM AUDIT: docs/formats/container-matroska.md
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A07) |
| Date | 2026-11-24 |
| Claims extracted | 12 |
| REAL 8 / REAL-UNPROVEN 2 / FALSE 0 / UNVERIFIED 2 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The MKV/WebM container record with three layered addenda (5.REC-1 audit, issue-#34 wiring).
Its code claims are REAL in today's tree — modules, accessors, wiring suite, command seam,
typed refusal all present and grep-verifiable; the matrix row it cites is "implemented" with
this record as evidence (no drift). One proposal bracket (engine has no chapter source) is
now half-superseded: playback-engine/src/chapters.rs landed (issue #32) as the pure
resolution half while seek.rs:297 still refuses typed Chapter seeks — exactly the split the
doc's module docs describe. Test-run counts are UNVERIFIED (cargo budget). No fabrication.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Status: implemented (feature-matrix row container-matroska-webm)" | :3-4 | REAL | docs/feature-matrix.toml:129 row + status "windows/linux/macos implemented, web not_started", evidence string cites this record |
| 2 | "`MkvDemuxer::tags()/attachments()/chapters()/title()/track_uid()/chapter_targets()`" | :120-123,222-231 | REAL | crates/demux/src/mkv/mod.rs:222,231,240,249,262,276 — all six accessors present |
| 3 | "`cover_art()` picks the JPEG" | :249 | REAL | crates/demux/src/mkv/attachments.rs:126 `pub fn cover_art()` |
| 4 | "mkv_extras_wiring.rs — 9 tests" | :235-236 | REAL | `rg -c '#\[test\]'` -> 9 |
| 5 | "commands catalog carries media.get-chapters/media.set-chapter (command.rs:363-368)" | :225-227 | REAL | crates/commands/src/catalog/command.rs:377-380 (line drift) |
| 6 | "playback-engine/src/seek.rs:297 refuses SeekTarget::Chapter typed" | :227-229 | REAL | crates/playback-engine/src/seek.rs:297 `SeekTarget::Chapter { .. } =>` refusal arm (+ :526 test) |
| 7 | "TrackEntry.uid (0x73C5) parsed" | :215-217 | REAL | crates/demux/src/mkv/header.rs:933 test `info_title_and_track_uid_parse_into_the_models` |
| 8 | "open()'s segment walk now runs to the segment end" | :197-199 | REAL | crates/demux/src/mkv/mod.rs:137,374 `segment_end` handling |
| 9 | "PROPOSAL: chapter-navigation command layer... engine has no chapter source" | :224-231 | REAL-UNPROVEN (superseded half) | crates/playback-engine/src/chapters.rs:1-14 landed (issue #32) as "pure resolution half"; seek.rs still refuses typed Chapter — split state matches |
| 10 | "cargo test -p demux 587 passed / 0 failed / 4 ignored" | :257-259 | UNVERIFIED | cargo test forbidden (load discipline, build budget); not in orchestrator baseline granularity |
| 11 | "31/31 mkv lanes... global + stream scopes match ffprobe exactly" | :92-94 | UNVERIFIED | corpus-v4 media not in tree; doc itself records the honest exclusion ("skip loudly... recorded, not re-run here") |
| 12 | "clippy -p demux -- -D warnings exit 0; fmt clean" | :260-262 | REAL-UNPROVEN | W17 baseline records clippy -D warnings CLEAN workspace-wide — consistent, per-crate split not re-run |

## 3. Contradictions
| This doc says | Other source says | Which is true |
|---|---|---|
| "engine has no chapter source (PROPOSAL bracket, 2026-11-09)" | crates/playback-engine/src/chapters.rs exists (issue #32) | Both at their dates: chapters.rs is host-side resolution; the typed engine refusal (seek.rs:297) persists — the doc's own prose survives as accurate |

## 4. Fabrication / inflation indicators
None. Gaps are listed openly (stream-scope title/language), honest corpus-absence
disclosure, and the "in flight" supersession note at the top. Matrix row and record agree
(no W18-style drift on this row).

## 5. Notes for next agents
- W13 context: this record's decoders claims are NOT involved — container row only;
  the four-registry disagreement concerns codec decoders, not the mkv demuxer.
- mkv_corpus.rs / mkv_editions_tags.rs suites still skip without corpus-v4 — any
  "tests pass" claim about MKV corpus coverage must cite the recorded runs, not a fresh one.
