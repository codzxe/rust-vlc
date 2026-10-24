# Container record — Matroska / WebM (MKV)

Status: **in progress** (feature-matrix row `container-matroska-webm`,
Phase 5). This record is the container's honest coverage ledger after
cards 5.D1 (core) and 5.D2b (editions + tags). Card 5.D2a
(attachments + chapter atoms) executes in parallel, file-disjoint;
its surface is marked **in flight** below, never claimed.

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
| `Info` `Title` (0x7BA9) | **not retained** (gap, see below) | — |
| `Tracks`/`TrackEntry`: number/type/codec/language/name/flags | full parse; `TrackUID` (0x73C5) **not retained** (gap, see below) | 5.D1 |
| `Cluster`/`SimpleBlock`/`BlockGroup` + Xiph/EBML/fixed lacing | full parse, bit-exact vs ffmpeg | 5.D1 |
| `Cues` seek index, keyframe-safe reanchor | full parse (65 536-entry ceiling) | 5.D1 |
| `Tags` (0x1254C367) → `Tag`/`Targets`/`SimpleTag` | **full parse** (this card, 5.D2b) | 5.D2b |
| `Chapters` (0x1043A770) → `EditionEntry` flags/UID | **full parse** (5.D2b) | 5.D2b |
| `ChapterAtom` (0xB6) times/displays/titles | **in flight** (5.D2a owns); 5.D2b records the UID links only | 5.D2a |
| `Attachments` (0x1941A469) | **in flight** (5.D2a owns) | 5.D2a |

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

1. `title` on the format scope: ffmpeg's muxer writes the container
   title into `Info.Title` (0x7BA9) — a 5.D1 element `SegmentInfo`
   does not retain (pinned by the generated lane).
2. `title` on the stream scope: per-stream titles live in
   `TrackEntry.Name` (0x536E) — parsed by 5.D1 into
   `TrackEntry.title` but not surfaced as stream *metadata* to the
   differential's consumer here.
3. `language` on the stream scope: `TrackEntry.Language` (5.D1
   surface, not a `Tags` product).
4. `TrackUID` (0x73C5) is not retained by 5.D1's track table — the
   differential extracts it raw from `Tracks` (the seam any
   tags→track UI join needs; wiring it into `TrackEntry` is a merge
   follow-up).

## Known gaps / next actions

- **Demuxer wiring seam**: `MkvDemuxer::open` still skips `Tags` and
  `Chapters` (5.D1's tolerance). The parse layer is public
  (`demux::mkv::tags::parse_tags`, `demux::mkv::editions::parse_editions`);
  wiring them into the walk with `tags()`/`editions()` accessors is
  the Lead merge step shared with 5.D2a (mod.rs is the two cards'
  only common file — this card's wiring was append-only 2 lines by
  mandate).
- **ChapterAtom structure** (times, displays, titles): 5.D2a's
  owned parse; when it lands, `editions.rs`' `collect_atom_uids` walk
  is the recorded integration point (replace with the structured
  parse, keep `Edition.chapter_uids` as the link seam).
- **Attachments**: 5.D2a (in flight).
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
