# Container record — Matroska / WebM (MKV)

Status: **implemented** (feature-matrix row `container-matroska-webm`,
Phase 5). This record is the container's honest coverage ledger after
cards 5.D1 (core), 5.D2a (attachments + chapter atoms), 5.D2b
(editions + tags) and issue #34 (the open() wiring + TrackUID/
Title retention — see the issue-#34 addendum at the bottom). Card
5.D2a landed at wave-4 close — see the audit addendum (5.REC-1);
its "in flight" markers in the table below are superseded.

- Layer: L2 demux (`crates/demux/src/mkv/`), C5 std-only, zero
  external dependencies, no `unsafe`.
- Oracle: ffmpeg/ffprobe 7.1.5 (live differential, corpus-v4).
- Corpus evidence: **31 mkv lanes** (corpus-v4/media, `*.mkv`);
  WebM exercised via 5.D1's committed fixtures + fuzz derivatives.

## Structure coverage

| Element (id) | Coverage | Card |
|---|---|---|
| `EBML` header (0x1A45DFA3), vints, unknown-size | full parse + typed refusal (`unknown-doctype`, `ebml-limits`) | 5.D1 |
| `Segment` (0x18538067) walk, SeekHead (0x114D9B74) | walk + skip | 5.D1 |
| `Info` (0x1549A966): `TimestampScale`, `Duration` | full parse | 5.D1 |
| `Info` `Title` (0x7BA9) | **retained** (issue #34: `SegmentInfo.title` + `MkvDemuxer::title()`, the fallback to the global `TITLE` tag pair included) | #34 |
| `Tracks`/`TrackEntry`: number/type/codec/language/name/flags | full parse; `TrackUID` (0x73C5) **retained** (issue #34: `TrackEntry.uid` + `MkvDemuxer::track_uid()`) | 5.D1, #34 |
| `Cluster`/`SimpleBlock`/`BlockGroup` + Xiph/EBML/fixed lacing | full parse, bit-exact vs ffmpeg | 5.D1 |
| `Cues` seek index, keyframe-safe reanchor | full parse (65 536-entry ceiling) | 5.D1 |
| `Tags` (0x1254C367) → `Tag`/`Targets`/`SimpleTag` | **full parse, wired through `open()`** (issue #34: retained on the demuxer, `tags()`) | 5.D2b, #34 |
| `Chapters` (0x1043A770) → `EditionEntry` flags/UID | **full parse, wired through `open()`** (issue #34: retained, `chapters()`, seek targets via `chapter_targets()`) | 5.D2b, #34 |
| `ChapterAtom` (0xB6) times/displays/titles | **full parse, wired** (issue #34) | 5.D2a, #34 |
| `Attachments` (0x1941A469) | **full parse, wired through `open()`** (issue #34: retained, `attachments()`) | 5.D2a, #34 |

### Tags semantics implemented (5.D2b)

- `Tag` records with `Targets` scoping: `TargetTypeValue` (default 50,
  unknown values **tolerantly recorded**, never refused),
  `TargetType`, `TagTrackUID` / `TagEditionUID` / `TagChapterUID` /
  `TagAttachmentUID` (repeatable lists).
- The **global form** — absent `Targets` or all-zero UIDs (the
  global-tag UID 0 rule) — and the per-track scope, both with
  differential accessors (`global_text_pairs`, `track_text_pairs`).
- `SimpleTag`: `TagName`, `TagLanguage` (default `und`),
  `TagLanguageIETF`, `TagDefault`, `TagString` (NUL padding stripped,
  matching ffmpeg's reader — the corpus's `DURATION` pairs carry a
  trailing `\0`), `TagBinary` (raw passthrough), and **nested
  `SimpleTag`** (depth-guarded at 8).
- Ceilings (typed `ResourceLimit` refusals, `demux.mkv.tags.budget`):
  4096 tags, 16 384 simple tags, 1 MiB per text/binary value, 8 MiB
  cumulative value budget.

### Edition semantics implemented (5.D2b)

- `EditionEntry`: `EditionUID`, `EditionFlagHidden`,
  `EditionFlagDefault`, `EditionFlagOrdered` (the ordered-edition
  playlist form) with spec defaults when absent.
- The **edition→chapter-UID links**: every `ChapterUID` (0x73C4) in
  the edition's atoms, nested included, file order — the join key for
  `TagChapterUID`/`TagEditionUID` targeting and edition-aware UI.
- Selection helper (`default_edition`: flagged → first visible →
  first), `ordered_editions`, `by_uid`.
- Ceilings: 1024 editions, 65 536 atoms, 8-deep atom nesting.

### Error paths (all typed, no panics, workspace lints clean)

- Truncated `SimpleTag` (stream cut mid-value): `Corrupt::Truncated`
  (`demux.mkv.truncated`).
- Over-deep nesting (tags and atoms): `Corrupt::InvalidStructure`
  (`demux.mkv.bad-structure`).
- Unknown `TargetTypeValue`: tolerant (recorded), per above.
- Bomb ceilings: `ResourceLimit{Memory}`
  (`demux.mkv.tags.budget`, `demux.mkv.editions.budget`).
- File-disjointness: this card mints its zone-specific constructors
  inside `tags.rs`/`editions.rs` (error.rs belongs to 5.D1; the
  parallel D2a agent owns its own additions).

## Corpus evidence (corpus-v4, live ffprobe, this tree)

Differential shape: my parsed global tag pairs vs ffprobe
`format_tags`; per-`TrackUID` pairs vs ffprobe `stream_tags`
(matched by stream order; `TrackUID`s walked from `Tracks`).

| File | Global pairs | Streams scoped | Tag records | Notes |
|---|---|---|---|---|
| 0017-meta-bomb-av.mkv | 5 (COPYRIGHT/COMMENT/ARTIST/LYRICS/ENCODER) | 2 | 3 | 47 908 B `Tags`; largest value 32 768 B (`LYRICS`); UTF-8 ARTIST; all match ffprobe |
| 0003-multi-av.mkv | 1 | 5 | 6 | per-track ENCODER/DURATION |
| 0014-muxed-av.mkv | 1 | 2 | 3 | the card's required lane |
| 0007-chapter-bomb.mkv | 1 | 2 | 3 | + 1 edition, **2580** chapter-UID links == ffprobe chapter count |
| 0063-multi-av.mkv | 1 | 4 | 5 | + 1 edition, 10 chapter links == ffprobe |
| 0023-rot-display.mkv | 4 | 1 | 2 | |
| all other mkv lanes | 1 (ENCODER) | 1–6 | 2–7 | |

- **31/31 mkv lanes**: every `Tags` element parses; global + stream
  scopes match ffprobe exactly (set equality) — with the recorded-gap
  exclusions below.
- **2/2 chapter lanes** (0007, 0063): edition count, default flag,
  and chapter-UID count == ffprobe chapter count.
- Generated lanes (ffmpeg 7.1.5, `-metadata title/artist/comment` and
  ffmetadata chapters): global `ARTIST`/`COMMENT` and per-stream
  `ENCODER`/`DURATION` match; 1 default edition, 2 chapter links ==
  ffprobe.

### Recorded gaps (honest exclusions in the differential)

1. `title` on the format scope: **CLOSED by issue #34** —
   `SegmentInfo.title` now retains `Info.Title` (0x7BA9) and
   `MkvDemuxer::title()` surfaces it (pinned by the generated lane:
   ffmpeg `-metadata title=` → `Info.Title` → `title()`).
2. `title` on the stream scope: per-stream titles live in
   `TrackEntry.Name` (0x536E) — parsed by 5.D1 into
   `TrackEntry.title` but not surfaced as stream *metadata* to the
   differential's consumer here.
3. `language` on the stream scope: `TrackEntry.Language` (5.D1
   surface, not a `Tags` product).
4. `TrackUID` (0x73C5): **CLOSED by issue #34** — `TrackEntry.uid`
   retained, `MkvDemuxer::track_uid(TrackId)` the accessor; the
   tags→track join (`track_text_pairs(uid)`) runs through it.

## Known gaps / next actions

- **Demuxer wiring seam**: **CLOSED by issue #34** (see the addendum
  below) — `open()` parses and retains `Tags`/`Attachments`/
  `Chapters`; `tags()`/`attachments()`/`chapters()` are the retained
  accessors; the positional-scan getters remain for the 5.D2 suites.
- **ChapterAtom structure** (times, displays, titles): landed (5.D2a)
  and wired (#34); `editions.rs`' `collect_atom_uids` walk remains
  the link seam.
- **Attachments**: landed (5.D2a) and wired (#34) — name/MIME/bytes
  on the model; the ASS font *consumer* is issue #17's surface.
- Ordered-edition **playback semantics** (playing an edition as an
  ordered part sequence) is engine-level work, not demux; the flags
  and links are exposed for it.
- `TagBinary` values parse but have no consumer yet (cover-art
  attachment interplay is D2a-adjacent).
- Nested `SimpleTag` children are parsed structurally; no flattening
  to text pairs (ffprobe doesn't flatten either — the differential
  compares top-level pairs only, matching behavior).

## Gates (this card, exact commands)

`cargo fmt -p demux` clean; `cargo clippy -p demux --all-targets
--all-features -- -D warnings` exit 0; `cargo test -p demux
--all-features` **316 passed / 0 failed / 4 ignored** (was 295/0/4
at 5.D1 merge; +7 unit, +14 integration). Note: run with a
worktree-local `CARGO_TARGET_DIR` — the shared `/home/z/my-project/
target` races with the parallel wave-4 worktrees compiling the same
crate.

---

## Audit addendum (5.REC-1, 2026-10-24 — records consolidation)

Re-audit of this record against the tree at `774ac5b` (wave-7
close). Verified in-tree this session:

- **5.D2a LANDED** (wave-4 merge, 5.W4-CLOSE: "5.D2a
  attachments+chapters 315/0, 34-lane differential, ns-vs-ticks
  spec fix"). The table's two "in flight" rows are therefore
  **superseded**: `Attachments` (0x1941A469) = full parse via
  `MkvDemuxer::attachments()` (attachments.rs:317);
  `ChapterAtom` (0xB6) = full parse via
  `MkvDemuxer::chapters()` (chapters.rs:567) with the **ns
  doctrine** (ChapterTimeStart/End are nanoseconds BY SPEC and do
  NOT scale with TimestampScale — the inherited ticks×scale model
  was removed at the root; ffprobe matches byte-for-byte on every
  lane). Evidence: 0007-chapter-bomb 2580 atoms matched; 0063 10
  chapters matched; 30 remaining lanes 0/0; generated fixture
  (ffmpeg -attach + ffmetadata) attachment bytes exact. Bounds:
  MAX_ATTACHMENTS 1024; atom budget 65 536; depth guard
  ebml::MAX_PARSE_DEPTH; >32 MiB FileData = NonInline{offset,len},
  never read.
- **The wiring seam is STILL OPEN** (re-verified:
  mkv/mod.rs:285 — the `open()` walk still skips
  SeekHead/Tags/Attachments/Chapters). The parse layers remain
  public positional-scan getters (`scan_segment_for`), so this is
  an ergonomics gap, not a correctness one — unchanged from the
  record's "Known gaps" list. Still the Lead merge follow-up.
- **TrackUID (0x73C5) still not retained** by the track table
  (re-verified: no 0x73C5/track_uid in header.rs) — gap 4 stands.
- Test counts superseded: card-time 316/0/4 → wave-7 workspace
  **3040 passed / 0 failed** (5.W7-CLOSE, fmt + clippy clean).
- Codec rides landed since: AV1 decode via rav1d (5.E1, bit-exact
  ×5), HEVC parser (5.G1, 3/3 corpus incl. mkv hvcC lane),
  MPEG-1/2 sequence layer (5.H1), H264 parser corpus 18/18 mkv
  lanes (5.F1a) — demux carriage unchanged.
- Matrix row `container-matroska-webm` still reads `not_started` /
  empty evidence (Lead-owned backfill; same amendment shape as the
  9 records written by this card).

---

## Issue #34 addendum (2026-11-09, W4-I — finish MKV/WebM)

The Lead merge follow-up the two addenda above waited on, landed in
three milestones (3e82379 wiring, 2e7472b the wiring suite +
livelock fix, this commit's record/matrix flip):

- **Wiring seam CLOSED**: `open()`'s segment walk now runs to the
  segment end (the 5.D1 early break at the first cluster lost the
  trailing metadata masters ffmpeg's layout puts after the
  clusters), parsing and retaining every `Tags`/`Attachments`/
  `Chapters` element (masters may repeat; lists merge in file
  order). `tags()`/`attachments()`/`chapters()` are `&self`
  retained-data accessors. Unknown-size `Tags`/`Attachments`/
  `Chapters` masters (the streaming form) parse at `open()` AND
  the mid-playback skip in `load_next` runs to the segment end
  (the former typed refusal is gone). Cluster-walk regression
  fixed on the way: a block that fails mid-cluster now restores
  the window on the error path, so the recovery arm advances the
  walk — the leaked-window form re-loaded the same cluster forever
  (`Recovered` loop, never EOS).
- **Title seam** (gap 1 closed): `Info.Title` (0x7BA9) retained on
  `SegmentInfo`, surfaced by `MkvDemuxer::title()` with fallback to
  the global `TITLE` tag pair (the mkvtoolnix spelling,
  case-insensitive).
- **TrackUID retention** (gap 4 closed): `TrackEntry.uid`
  (0x73C5) parsed; `MkvDemuxer::track_uid(TrackId)` is the stable
  identity (nonzero-UID join against `TagTrackUID` via
  `track_text_pairs(uid)`); `None` = out-of-table handle, `Some(0)`
  = the file declared none.
- **Chapter seek targets**: `chapter_targets()` — the default
  edition's (flagged → first visible → first) visible atoms,
  depth-first, ns times converted onto the segment clock (scale
  ticks), hidden atoms retained on the model but not offered.
  `[PROPOSAL: the chapter-navigation command layer — matrix
  `playback-chapter-title`, phase 4 — does not exist yet: the
  commands catalog carries `media.get-chapters`/`media.set-chapter`
  (commands/src/catalog/command.rs:363-368) but the engine has no
  chapter source (playback-engine/src/seek.rs:297 refuses
  `SeekTarget::Chapter` typed; events carry `chapters:
  Vec::new()`), so nothing downstream consumes the targets; the
  data is retained, no untested plumbing wired]`.

### Corpus vectors (the wiring evidence, this tree)

`crates/demux/tests/mkv_extras_wiring.rs` — 9 tests, the
record-corpus convention (hand-built spec vectors + a live
ffmpeg-generated differential lane), all run this session:

| Vector | Covers | Pinned assertions |
|---|---|---|
| `tags_attachments_chapters_wire_through_start_retained` | the namesake: Tags (simple + nested + track-scoped) + Attachments (font + image) + Chapters (single edition, 2 atoms), Cues before clusters, extras trailing (ffmpeg's layout) | 2 tag records, nested `SimpleTag` structure, `track_text_pairs(77)` joins the retained UID, font/cover name+MIME+UID+bytes byte-exact, edition flags + ns times + titles, `chapter_targets()` ns→ticks (1500/3900 at the 1 ms scale), drain 2 packets, 0 damage |
| `title_falls_back_to_the_global_title_tag` | Tags simple (global `TITLE` pair) | `title()` = Info.Title → else global tag → else `None` |
| `track_uids_are_stable_across_reopens` | TrackUID retention | 2 tracks × 2 fresh opens agree; model/accessor agree; out-of-range = `None` |
| `multi_edition_chapters_retain_all_and_target_the_default` | Chapters multi-edition (hidden + default + plain) | 3 editions retained, targets = default edition's visible atoms only (hidden atom excluded), whole-model flatten = 5 |
| `chapter_targets_seek_lands_at_or_before` | chapters as seek targets | `seek(target.span)` lands on the cue-indexed cluster at-or-before the chapter start (1000 ≤ 1500), chapter 0 lands at 0 |
| `unknown_size_tags_master_parses_and_playback_reaches_eos` | streaming shape | unknown-size `Tags` parsed at `open()`, playback still reaches EOS past it |
| `damaged_block_recovers_once_and_still_reaches_eos` | the livelock regression | one `Recovered`, walk advances, EOS reached, trailing Tags still retained |
| `degenerate_attachment_damage_folds_into_the_counter` | damage folding | attachment without `FileData` → list damage row + honesty counter, survivor retained |
| `generated_lane_wires_title_font_cover_and_chapters` | **ffmpeg 7.1.5 differential** (`-metadata title=`, `-attach` TTF + JPEG, ffmetadata chapters) | ffprobe title matches `title()`; real TTF + JPEG attachment bytes byte-exact on the model; `cover_art()` picks the JPEG; distinct nonzero TrackUIDs join per-stream `ENCODER` pairs; 2 chapter targets at 0/250 ms; drain > 0 |

Honest exclusions: corpus-v4 is absent in this session's sandbox, so
the 31-lane corpus differential (mkv_corpus.rs) and the 5.D2b tags
sweep (mkv_editions_tags.rs) skip loudly — their recorded results
stand as recorded, not re-run here. Gaps 2/3 (stream-scope title via
`TrackEntry.Name`, stream language) remain 5.D1 surfaces, unchanged.

Gates this session (worktree `wt/w3i` @ the issue-#34 commits):
`cargo test -p demux` **587 passed / 0 failed / 4 ignored** (28
suites; the 9-test wiring suite included, the 4 ignores are the
absent-corpus skips); `cargo clippy -p demux --all-targets
-- -D warnings` exit 0; `cargo fmt --check --all` clean. (Note:
the shared `CARGO_TARGET_DIR` raced once with a sibling
worktree's artifacts — a stale-rlib clippy failure that a
fingerprint refresh cleared; the record's worktree-local-target
advice stands.)
