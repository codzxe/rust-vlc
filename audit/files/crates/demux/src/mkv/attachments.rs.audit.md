# AUDIT: crates/demux/src/mkv/attachments.rs

| Field | Value |
|---|---|
| Auditor | W05-A04 |
| Date | 2026-11-17 |
| Lines / functions | 546 / 2 parse fns + 1 method + 7 ID consts + 2 ceiling consts + 3 types + 1 alias + 6 test helpers + 9 tests |
| File verdict | REAL-UNPROVEN (genuine bounded parser, reachable from the CLI open walk; execution blocked by no-cargo adaptation — W00 baseline ran the demux suite) — but the exposure seam (`attachments()`/`cover_art()`/`FileData::NonInline`) has ZERO production consumers (F-004) |
| Reachable from shipped entry point? | Parse: YES — `lumen` CLI → `demux_host.rs:458` → `containers/mod.rs:57` → `containers/mkv.rs:147` → `MkvDemuxer::start` → `mod.rs:399/424 collect_attachments` → `mod.rs:481 parse_attachments`. Getter/data: NO — `mod.rs:231 attachments()` and `attachments.rs:126 cover_art()` are called only from `crates/demux/tests/*` (verified by workspace-wide grep); no apps/, ui/, or engine caller exists |
| Findings | S0:0 S1:0 S2:1 S3:3 S4:1 |
| Commands run | Read (full file, both chunks of `mod.rs` regions); Grep workspace-wide for `cover_art|NonInline|attachments()|AttachmentList|FileData::`; `sed -n` verification of quotes (attachments.rs 8-20, 28-34, 168-185, 210-218, 243-257; mod.rs 226-233, 480-486) — exit 0. No cargo (adaptation); runtime = static trace + W00 baseline |

## 1. One-paragraph verdict

This is a real, carefully bounded `Attachments` parser: correct Matroska element IDs, per-file 32 MiB inline ceiling with a never-read `NonInline` note for bombs, a 1024-file count ceiling with damage rows, tolerant advisory text, and nine honest tests that pin the interesting paths. It has two structural problems. First, containment is inconsistent: a corrupt attachment record — non-essential metadata — returns a typed error that propagates through `collect_attachments`'s `?` and kills the **entire file open** (F-001), while the ceiling paths one screen earlier skip-with-damage; the count and inline-bytes ceilings are per-element/per-file with **no aggregate budget** across merged `Attachments` masters (F-002, F-003). Second, the feature is parsed, retained in memory for the whole session, and then **consumed by nobody**: `attachments()`, `cover_art()` and every `FileData::NonInline` note have zero callers outside the test suite — no UI shows the cover art, no ASS font stack exists to read the fonts, and the "consumer fetches it on demand" the NonInline doc promises does not exist (F-004). The demuxer spends up to whole-file-size memory at `start()` on data no shipped binary can display.

## 2. What actually works (verified)

- Element IDs match the Matroska spec table: `Attachments` 0x1941A469, `AttachedFile` 0x61A7, `FileDescription` 0x467E, `FileName` 0x466E, `FileMimeType` 0x4660, `FileData` 0x465C, `FileUID` 0x46AE (`:44-57`) — verified against the spec's ID table.
- Per-file bomb protection is real and test-pinned: a 33 MiB declared payload becomes `NonInline { offset, len }` and the bytes are never read (`:243-257`; test `over_ceiling_data_stays_the_non_inline_note` `:368-402` asserts the note values).
- Short `FileData` is the typed truncation at the payload's own boundary (`read_exact_at`, `:248`) — test-pinned (`:405-426`, asserts `demux.mkv.truncated`).
- Degenerate record (no `FileData`) is damage-skipped, not fatal (`:268-270`) — test-pinned (`:449-456`).
- Invalid-UTF-8 labels become absent, never fabricated (`:224-231`, `ebml::utf8_from` → `None`) — test-pinned (`:459-467`).
- `cover_art()` matching is correct: case-insensitive `image/` MIME prefix, `cover.`/`front.` name prefix with the dot (no false positive on `covering.jpg`), fonts excluded — test-pinned (`:470-510`).
- `AttachmentDamage` alias doc says "(bytes, reason)" (`:104-105`) and the values pushed are byte counts (`child_end - pos`, `:176`, `:182`) — internally consistent, unlike header.rs's `TrackDamage` alias doc.
- Allocations bounded per item: text ≤ 4 KiB (`MAX_TEXT_FIELD`), inline ≤ 32 MiB **and every inline byte must physically exist in the stream** (`read_exact_at` clamps to stream length, `ebml.rs:385-393`), element headers ≤ 12 B. Loop progress structurally guaranteed (`pos = child_end`, `data_pos > pos`). No panic path, no `unwrap`/`expect` outside `unreachable!` test arms.

## 3. Findings

### F-001 [S2] A corrupt attachment record aborts the entire file open — non-essential metadata has whole-file blast radius
Where: crates/demux/src/mkv/attachments.rs:169-172 and 213-216; blast radius crates/demux/src/mkv/mod.rs:480-481
Code (attachments.rs:169-172):
```rust
            if child_end > end {
                // The record claims bytes past the master: the
                // contained lie, typed at the master's end.
                return Err(error::truncated(child_end, end));
```
Code (attachments.rs:213-216):
```rust
        if child_end > end {
            // A field claiming bytes past its `AttachedFile`: the
            // contained lie, typed at the record's end.
            return Err(error::truncated(child_end, end));
```
Code (mod.rs:480-481):
```rust
    fn collect_attachments(&mut self, child: &ElementHeader) -> Result<(), EngineError> {
        let more = attachments::parse_attachments(&mut self.reader, child, self.segment_end)?;
```
What is wrong: any `AttachedFile` whose declared extent overruns its master, **any child of a record** (including unknown elements — the check at `:213` runs before the match) whose declared extent overruns its record, or any unknown-size inner element (`:163-166`, `:210-211` → `bad_structure`) returns `Err`, which `mod.rs:481`'s `?` propagates through `open()` → `start()` — the **whole MKV refuses to open**. Attachments are cover art, fonts, and companion files: no playback path needs them.
Why it matters: a truncated download or a muxer bug in one attachment record makes an otherwise perfectly playable video unopenable. The containment ladder in this very file is inconsistent: over-ceiling payload → survive (NonInline), record without data → survive (damage row), record overrunning by **one byte** → the file dies. The sibling doctrine 100 lines away (header.rs:13-16) promises "the other tracks and the file survive". The behavior is documented (`:8-13`) and test-pinned (`:429-446`), i.e. deliberate — but it is the wrong blast radius for a player, and it contradicts the module's own tolerance rules for the same element family.
What correct looks like: skip the offending record with a damage row (the pattern `:174-183` already uses) and let the walk continue; reserve `Err` for the case where the stream physically ends inside the master.
Confidence: HIGH — Verification: read (full static trace mod.rs:399/424 → 481 → open(); runtime UNVERIFIED, no cargo).

### F-002 [S3] `MAX_ATTACHMENTS` is enforced per `Attachments` element, not per file — the documented count ceiling is not the real ceiling
Where: crates/demux/src/mkv/attachments.rs:174-176 vs. mod.rs:483; doc claim attachments.rs:19-20
Code (attachments.rs:174-176):
```rust
            if list.files.len() >= MAX_ATTACHMENTS {
                list.damage
                    .push((child_end - pos, "attachment-over-ceiling"));
```
Code (mod.rs:483):
```rust
        self.attachments.files.extend(more.files);
```
What is wrong: the ceiling check counts the **local** list of one `parse_attachments` call. Matroska allows multiple top-level `Attachments` masters; `mod.rs:483` merges each master's up-to-1024 files into the session list with no aggregate check. The module doc claims "The file count itself is ceiling-checked ([`MAX_ATTACHMENTS`])" (`:19-20`) — true per element, false per file.
Why it matters: the stated metadata-bomb ceiling (1024) can be exceeded N×; the aggregate list grows without bound (bounded in practice only by file size / minimum record size).
What correct looks like: check `self.attachments.files.len() + list.files.len()` at the merge, or pass the running count into the parse.
Confidence: HIGH — Verification: read.

### F-003 [S3] No aggregate inline-bytes budget: the retained attachment list can hold the whole file in RAM at `start()`
Where: crates/demux/src/mkv/attachments.rs:14-18, 243-249; retention mod.rs:483, struct field mod.rs:130
Code (attachments.rs:14-18):
```rust
//! - **Bounded memory (MP §12)**: per-file payloads at or under
//!   [`MAX_ATTACHMENT_BYTES`] are read inline; larger declared
//!   payloads stay the **non-inline note** ([`FileData::NonInline`] —
//!   position + length, never read by this parse) so a bomb-sized
//!   attachment cannot become a bomb-sized allocation. The file count
```
Code (attachments.rs:247-249):
```rust
                    let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                    data = Some(FileData::Inline(bytes));
```
What is wrong: the bomb protection is per-file (≤ 32 MiB each). With up to 1024 files per master (F-002: ×N masters), all inline payloads are read at `start()` and **retained on the demuxer for the whole session** (`mod.rs:130 attachments: AttachmentList`). There is no total-bytes budget: a crafted 1 GiB file whose `Attachments` masters carry 32 × 32 MiB payloads (or 1024 × 1 MiB) inlines ~1 GiB of heap at open — exactly the "bomb-sized allocation" the doc claims cannot happen, just spread across records.
Why it matters: no amplification (every inline byte must physically exist — `read_exact_at` clamps to stream length), so this is not an OOM multiplier; but "Bounded memory (MP §12)" is only true per record, and the retained-session lifetime makes it worse than a transient read. Real files carry KBs (the doc's own words, `:67-68`); the ceiling allows GiBs.
What correct looks like: a session-wide inline budget (e.g., 64 MiB total) after which further payloads degrade to the `NonInline` note.
Confidence: HIGH — Verification: read.

### F-004 [S3] The entire feature is consumed by nobody: `attachments()`, `cover_art()` and every `NonInline` note have zero production callers
Where: crates/demux/src/mkv/attachments.rs:28-34 (the seam doc), 76-80 (NonInline consumer promise); getter mod.rs:226-233
Code (attachments.rs:28-34):
```rust
//! # Exposure seam (the card's "info/attachment surface")
//!
//! [`MkvDemuxer::attachments`] is the typed getter over the list the
//! demuxer **retains at `start`** (issue #34 wired the `open()` seam:
//! the 5.D2a card's on-demand positional re-scan is gone — the files
//! are parsed once during the walk and held on the struct, so the
//! packet path never re-reads them and the getters are `&self`).
```
Code (attachments.rs:76-80):
```rust
    /// An over-ceiling declared payload: its absolute position and
    /// length, **never read by this parse** — the non-inline note.
    /// The consumer fetches it on demand through its own bounded
    /// access; a bomb-sized attachment cannot become a bomb-sized
    /// allocation here.
```
What is wrong: a workspace-wide grep for `cover_art|NonInline|attachments()|AttachmentList|FileData::` finds callers **only** in `crates/demux/tests/mkv_extras_wiring.rs`, `mkv_attachments_chapters.rs` and this file's own tests. No `apps/cli`, `apps/desktop`, `apps/desktop-tauri`, `ui/`, or engine code reads the retained list. The "exposure seam" exposes to nobody; the `NonInline` "consumer [that] fetches it on demand" does not exist anywhere in the repo.
Why it matters: the demuxer performs real work — including up-to-whole-file-size inline reads (F-003) — whose result no shipped binary can display. This is the accepted-and-ignored pattern at feature level: the parse is real, the wiring stops at a getter. For the claim ledger: any matrix row claiming MKV cover art / attachments as a user-facing feature is FALSE at the consumption end; rows claiming "parsed and retained" are REAL. The docs' own hedge (`mod.rs:228-229` "The ASS font *consumer* (the render stack) is issue #17's surface; this seam only makes the bytes available") admits the font half; the cover-art half has the same status with no such hedge.
What correct looks like: either a consumer (UI cover-art lane, ASS font provider) or a lazy parse-on-demand design that does not retain inline bytes nobody reads.
Confidence: HIGH — Verification: read + workspace-wide grep (all call sites enumerated above).

### F-005 [S4] EBML test builders duplicated four times across the mkv zone (two verbatim, two reimplemented)
Where: crates/demux/src/mkv/attachments.rs:287-317 vs. header.rs:860-892 (verbatim) vs. tests/mkv_extras_wiring.rs:40-65 and tests/mkv_editions_tags.rs:49-71 (reimplemented)
Code (attachments.rs:287-292):
```rust
    fn id_bytes(id: u32) -> Vec<u8> {
        // The marker bits of the id's first byte fix its width:
        // 0x10..0x1F → 4 bytes, 0x20..0x3F → 3, 0x40..0x7F → 2,
        // 0x80..0xFF → 1 (the u32's leading zeros count them).
        let width = 4 - id.leading_zeros() / 8;
        id.to_be_bytes()[4 - width as usize..].to_vec()
    }
```
What is wrong: `id_bytes`/`size_bytes`/`el` are duplicated **verbatim** in `header.rs` tests (`:860-892`) and `attachments.rs` tests (`:287-317`); `tests/mkv_extras_wiring.rs:40-65` and `tests/mkv_editions_tags.rs:49-71` each carry a **third and fourth** implementation of the same builders under the names `id_bytes`/`size_vint`/`el` (the wiring file's own comment `:36` says "the mkv_editions_tags.rs vocabulary"). Four copies of EBML-building logic in one crate's mkv surface.
Why it matters: a bug in a builder (e.g., the size-vint width choice) must be found and fixed four times, and the copies have already drifted (leading-zeros vs. position-based id_bytes, different size-vint algorithms).
What correct looks like: one shared `test_support` module in the mkv zone (the crate already crosses zones for `crate::avi::test_support::mem_stream`).
Confidence: HIGH — Verification: read of all four sites (grep line numbers cited).

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ID consts (`ID_ATTACHMENTS` … `ID_FILE_UID`) | 44-57 | REAL | — | All 7 match the Matroska spec ID table |
| `MAX_ATTACHMENTS` = 1024 | 59-63 | REAL | — | **F-002**: enforced per-element only |
| `MAX_ATTACHMENT_BYTES` = 32 MiB | 65-69 | REAL | — | **F-003**: per-file only, no aggregate budget |
| `FileData` enum | 71-87 | REAL | — | **F-004**: `NonInline` has zero consumers repo-wide |
| `AttachedFile` struct | 89-102 | REAL | — | Data only |
| `AttachmentDamage` alias | 104-106 | REAL | — | Doc "(bytes, reason)" matches usage — correct, contrast header.rs F-004 |
| `AttachmentList` struct | 108-116 | REAL | — | Derives Default; damage + files |
| `AttachmentList::cover_art` | 118-139 | REAL | — | Correct matching (case-insensitive, dot-anchored prefixes, fonts excluded); **F-004**: no production caller |
| `parse_attachments` | 141-190 | REAL | — | **F-001** (child_end > end → Err, whole-file blast radius); unknown-size inner child → `bad_structure` Err (same blast radius); unknown children skipped by declared size; ceiling + damage rows correct per element |
| `parse_attached_file` | 194-278 | REAL | — | **F-001** (`:213-217` applies to ALL children incl. unknown); advisory labels tolerant both ways (`:220-233`); FileUID len>8 typed (`:235-237`); FileData ceiling → NonInline (`:243-257`); multiple FileData elements → last wins (tolerant, undocumented); empty FileData accepted as Inline(vec![]) |
| test helpers `id_bytes`/`size_bytes`/`el`/`attachments_element`/`attachments_element_unknown`/`parse` | 286-343 | REAL | — | **F-005** duplication; helpers themselves correct |
| test `attached_files_parse_the_full_record` | 345-365 | REAL | — | Full-record round-trip, concrete values |
| test `over_ceiling_data_stays_the_non_inline_note` | 367-402 | REAL | — | Constructs 33 MiB declared payload; asserts NonInline offset+len; bound deliberately beyond stream length (legal — payload never read) |
| test `truncated_file_data_is_the_typed_truncation` | 404-426 | REAL | — | Asserts `demux.mkv.truncated` + Corrupt::Truncated kind |
| test `file_data_overrunning_the_master_is_the_typed_truncation` | 428-446 | REAL | — | Pins the F-001 behavior — the design is intentional |
| test `attached_file_without_data_is_damage_skipped` | 448-456 | REAL | — | Damage reason asserted |
| test `invalid_utf8_labels_are_tolerated_as_absent` | 458-467 | REAL | — | name None, data survives |
| test `cover_art_picks_the_image_attachment_not_the_font` | 469-510 | REAL | — | MIME, name-convention, font-exclusion lanes |
| test `attachment_count_ceiling_skips_with_damage` | 512-531 | REAL | — | 1025 records → 1024 kept + 1 damage; order preserved |
| test `unknown_size_inside_the_master_is_bad_structure` | 533-545 | REAL | — | Pins `demux.mkv.bad-structure` |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `attached_files_parse_the_full_record` | YES | Asserts all 5 fields + Inline bytes + empty damage |
| `over_ceiling_data_stays_the_non_inline_note` | YES | Asserts NonInline offset/len values — an inline read of 33 MiB or a wrong note fails |
| `truncated_file_data_is_the_typed_truncation` | YES | Asserts error kind + context code |
| `file_data_overrunning_the_master_is_the_typed_truncation` | YES | Asserts `demux.mkv.truncated` (pins F-001's design) |
| `attached_file_without_data_is_damage_skipped` | YES | Asserts empty files + exact damage reason |
| `invalid_utf8_labels_are_tolerated_as_absent` | YES | Asserts name None + data Inline |
| `cover_art_picks_the_image_attachment_not_the_font` | YES | Three lanes with concrete assertions |
| `attachment_count_ceiling_skips_with_damage` | YES | Asserts exact count, damage len, reason, uid order |
| `unknown_size_inside_the_master_is_bad_structure` | YES | Asserts context code |

Coverage gaps (no test): a child overrunning its **record** (`:213-217`, only the master-level overrun is tested); over-ceiling text labels (name > 4 KiB → silently absent); multiple `Attachments` masters merging (F-002 untested); aggregate inline memory (F-003 untested). Runtime pass/fail: UNVERIFIED directly (no cargo); W00 baseline ran the workspace suite (1751 pass / 2 fail / 25 ignored) containing these tests.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a `FileData` whose declared extent overruns the master is the typed truncation" | attachments.rs:8-13 | REAL (as stated) — but the blast radius is the whole file (F-001) | `:169-172` + mod.rs:481 `?` |
| "Bounded memory (MP §12) … a bomb-sized attachment cannot become a bomb-sized allocation" | attachments.rs:14-18 | SHALLOW | True per file (`:243-257`); no aggregate budget across 1024 files × N masters, all retained for the session (F-003) |
| "The file count itself is ceiling-checked ([`MAX_ATTACHMENTS`])" | attachments.rs:19-20 | SHALLOW | Per-element only; mod.rs:483 merges without aggregate cap (F-002) |
| "`MkvDemuxer::attachments` is the typed getter … retained at `start`" | attachments.rs:30-34 | REAL | mod.rs:226-233 getter exists; **but no production caller** (F-004) |
| "The consumer fetches it on demand through its own bounded access" (NonInline) | attachments.rs:78-79 | FALSE | Workspace-wide grep: no consumer of `NonInline` exists |
| "cover art / fonts / arbitrary companion files … a UI needs" | attachments.rs:4-6 | PAPER-ONLY (consumption end) | No UI/engine code reads the list (F-004) |
| "Attachments … full parse, wired through `open()` (issue #34: retained, `attachments()`)" | docs/formats/container-matroska.md:31 | REAL (parse) / SHALLOW (wiring) | Parse real (this file); "wired" ends at an unconsumed getter; doc's line ref "attachments.rs:317" (line 159) is also stale — the getter is mod.rs:231 |

## 7. Dead / duplicated / unreachable code

- `FileData::NonInline` variant: constructed here, matched only in this file's tests — dead in production (F-004).
- `AttachmentList::cover_art`: production-dead (test-only callers) (F-004).
- Test-helper duplication across three files (F-005).

## 8. Dependencies & flags

- No feature flags; unconditional module (`mod.rs` `mod attachments;` in the mkv zone, no `#[cfg]` outside `#[cfg(test)]`).
- Imports: `media_types::EngineError`, `crate::mkv::ebml`, `crate::mkv::error`, `crate::mkv::header::MAX_TEXT_FIELD` (cross-module ceiling reuse — good). Tests again use `crate::avi::test_support::mem_stream` + `testkit::Must` (AVI-zone dependency).

## 9. Open questions for the next agent

- Wave 18/19: does any feature-matrix row claim MKV cover art or attached-font *display* (not parse)? If yes it is FALSE at the consumption end (F-004 evidence above).
- `tests/mkv_attachments_chapters.rs` (another agent's file) may test multi-master merging — if not, F-002's merge path is entirely untested.
- The ASS/subtitle wave (9): issue #17's font consumer was the stated reason fonts are retained; confirm it still does not exist (my grep says nothing consumes the list repo-wide).
