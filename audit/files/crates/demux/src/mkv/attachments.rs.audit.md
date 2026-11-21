# AUDIT: crates/demux/src/mkv/attachments.rs
| Field | Value |
|---|---|
| Auditor | W05-A03 |
| Date | 2026-11-21 |
| Lines / functions | 546 / 21 items (7 ID consts, 2 ceilings, FileData enum, AttachedFile, AttachmentDamage, AttachmentList + cover_art, parse_attachments, parse_attached_file, 6 test helpers, 9 tests) |
| File verdict | REAL (parse is real, executed, and byte-verified against a fresh ffmpeg file; one disproportionate-fatality design defect and a paper-only escape hatch) |
| Reachable from shipped entry point? | YES (call chain: `lumen open` → apps/cli/src/containers/mod.rs:57 `"matroska"` → mkv::open → MkvDemuxer::start (mod.rs:399/424) → collect_attachments (mod.rs:480) → parse_attachments (this file). No feature gate.) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `rg` (collect_attachments call sites; NonInline consumers repo-wide; cover_art consumers repo-wide); python3 EBML walk of a fresh `ffmpeg -attach` file (element-ID ground truth); `ffmpeg` 7.1.5 generation; `sed -n` quote verification; baseline cites below |

## 1. One-paragraph verdict
This is a clean, well-tested parser: all seven element IDs verified against a fresh ffmpeg 7.1.5 `-attach` file byte-for-byte, child sizes genuinely clamped to the master (the discipline ebml.rs only claims), a non-inline escape for bomb-sized payloads, and nine executed tests (baseline test-workspace.log.txt:1755-1763) that pin the full record, both truncation shapes, the ceiling, UTF-8 tolerance, and cover-art selection, plus a real-file differential in the wiring suite. Two defects: a single corrupt attachment — pure metadata, typically trailing the clusters — aborts the entire `open()` (typed truncation propagates uncontained through `collect_attachments`), so a truncated download of an mkv with attachments is unopenable despite fully intact audio/video; and the `NonInline` note is documented as having a consumer that does not exist anywhere in the tree. The retained `attachments()`/`cover_art()` surface has no consumer outside demux tests — honest per issue #34's recorded retention scope, but the MP4 host has a live cover-art lane (m4a still-image trak) and MKV does not.

## 2. What actually works (verified)
- Element IDs vs on-disk truth: `ffmpeg -attach cov.jpg -metadata:s:t:0 mimetype=image/jpeg` → my python EBML walk shows Attachments `0x1941A469` → AttachedFile `0x61A7` → `0x466E`="cover.jpg", `0x4660`="image/jpeg", `0x465C`="JPEGBYTES", `0x46AE`=FileUID — exactly this file's constants (lines 45-57). (ffmpeg also writes a `0xBF` CRC-32 child of Attachments; parse_attachments skips it via the default arm — the "CRC skipped" doctrine holds.)
- Executed: baseline test-workspace.log.txt:1755-1763 — all 9 `mkv::attachments::tests` `ok`; the wiring suite's generated lane (mkv_extras_wiring.rs, `generated_lane_wires_title_font_cover_and_chapters`, baseline `ok`) pins real TTF+JPEG attachment bytes byte-exact and `cover_art()` on an ffmpeg-authored file (container-matroska.md:249).
- Contained-lie clamp: parse_attachments:169-172 and parse_attached_file:213-217 both refuse a child whose declared end overruns its parent — this file implements the bound-checking that ebml.rs's module doc claims module-wide (contrast: header.rs does not clamp child ends; see my ebml.rs audit F-005).
- Overflow/panic audit: `child_end - pos` only after `child_end > end` refusal or a well-formed header (child_end > pos always); `usize::try_from(child.size)` typed on failure (line 218); no unwrap/expect/index/div in production paths; allocation per attachment capped at 32 MiB inline.

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/mkv/attachments.rs:243-249 (propagation mod.rs:399,424,481; blast radius apps/cli/src/containers/mkv.rs:148-150)
Code:
```
            ID_FILE_DATA => {
                if child.size <= MAX_ATTACHMENT_BYTES {
                    // The exact read: a short stream is the typed
                    // truncation at the payload's own boundary (the
                    // card's truncated-AttachmentFile-data path).
                    let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                    data = Some(FileData::Inline(bytes));
```
What is wrong: the `?` turns one attachment's bad extent into a fatal error for the whole file. Chain: read_exact_at → Err(truncated) → parse_attachments returns Err → `collect_attachments` (mod.rs:481 `parse_attachments(...)?`) → `start()` (mod.rs:399/424 `self.collect_attachments(&child)?`) → apps/cli mkv.rs:148-150 `let tracks = demuxer.start().map_err(|err| wire_error(&err))?;` → **open fails**. Same for the contained-lie refusals at lines 169-172 and 213-216.
Why it matters: attachments are pure metadata extras and muxers write them after the clusters (the ffmpeg file I generated has Attachments last). A truncated download or a single lying attachment size — with every audio/video cluster intact — makes the entire movie unopenable. This is the exact "cannot be opened" failure class the owner reported, applied to a non-load-bearing element. Contrast the module's own containment doctrine one branch up: a record without FileData is damage-skipped, "the file and its data survive" (lines 266-270, test attached_file_without_data_is_damage_skipped), and tracks are damage-contained (header.rs parse_tracks: "a degenerate one is skipped with damage and the others survive"). The truncation branch is the odd one out.
What correct looks like: fold a per-record truncation/contained-lie into `list.damage` (skip the record, keep the rest) the way no-FileData already is; reserve fatal typed truncation for damage that breaks the walk itself (stream cut inside the master header).
Confidence: HIGH — Verification: read (full propagation chain quoted) + the file's own tests pin the fatal behavior (truncated_file_data_is_the_typed_truncation asserts the Err).

### F-002 [S3]
Where: crates/demux/src/mkv/attachments.rs:76-80
Code:
```
    /// An over-ceiling declared payload: its absolute position and
    /// length, **never read by this parse** — the non-inline note.
    /// The consumer fetches it on demand through its own bounded
    /// access; a bomb-sized attachment cannot become a bomb-sized
    /// allocation here.
```
What is wrong: "The consumer fetches it on demand" — no such consumer exists. Repo-wide `rg NonInline` (crates/ + apps/): zero uses outside this file's definition and tests. An over-32 MiB attachment is noted and then permanently unfetchable in this codebase.
Why it matters: the doc sells the non-inline form as a working lazy-fetch seam; it is a dead end. A user attaching a 40 MiB cover/photo gets silently no data path at all (the record survives, nothing can ever read it).
What correct looks like: either implement the bounded on-demand fetch (a `read_non_inline(reader, &note)` helper on the demuxer) or rewrite the doc to "no consumer yet — the note is forward-honest data" the way container-matroska.md:132-133 does for TagBinary.
Confidence: HIGH — Verification: rg sweep (zero consumers).

### F-003 [S3]
Where: crates/demux/src/mkv/attachments.rs:14-20 (doc) vs 63/69 (ceilings)
Code:
```
//! - **Bounded memory (MP §12)**: per-file payloads at or under
//!   [`MAX_ATTACHMENT_BYTES`] are read inline; larger declared
//!   payloads stay the **non-inline note** ([`FileData::NonInline`] —
//!   position + length, never read by this parse) so a bomb-sized
//!   attachment cannot become a bomb-sized allocation. The file count
//!   itself is ceiling-checked ([`MAX_ATTACHMENTS`]); over-ceiling
//!   files are skipped with damage, the rest survive.
```
What is wrong: the budget is per-attachment only. MAX_ATTACHMENTS (1024) × MAX_ATTACHMENT_BYTES (32 MiB) = 32 GiB of inline bytes can be held simultaneously on the demuxer for the whole session; there is no cumulative ceiling. The sibling tags module has one (tags.rs:126 MAX_TAG_LIST_BYTES = 8 MiB cumulative, plus per-value 1 MiB).
Why it matters: a 2 GiB crafted-or-honest mkv with ~60 × 30 MiB attachments holds ~1.8 GiB in RAM indefinitely (retained at start, never freed until the demuxer drops). MP §12's "bounded memory" claim is satisfied only per element; the list total is file-size-bounded, not budget-bounded.
What correct looks like: a cumulative inline budget (e.g., 64 MiB total) with later over-budget files demoted to NonInline notes — the escape hatch already exists and would finally have a use.
Confidence: HIGH — Verification: read (no cumulative counter anywhere in the file; contrast tags.rs:116-126).

### F-004 [S4]
Where: crates/demux/src/mkv/attachments.rs:126-138 (getter) — consumers
Code:
```
    pub fn cover_art(&self) -> Option<&AttachedFile> {
        self.files.iter().find(|file| {
            let image_mime = file
                .mime_type
                .as_deref()
                .is_some_and(|m| m.to_ascii_lowercase().starts_with("image/"));
```
What is wrong: not the function (correct, case-insensitive, first-match — verified by the executed tests) but its consumer set: `cover_art()`/`attachments()` are called by nothing outside the demux crate's tests. The shipped UI never shows MKV cover art or attachments, while the MP4 host has a live cover-art lane (apps/cli/src/containers/mp4.rs:645-663, m4a still-image trak → mjpeg track). Issue #34's recorded scope was retention, and container-matroska.md:224-231 records the chapter-nav non-consumption as a PROPOSAL — the attachment-consumer gap is not recorded anywhere.
Why it matters: retention-only wiring that reads as a feature in the module doc ("the typed `AttachedFile` records a UI needs"); the asymmetry with the MP4 lane is unrecorded.
What correct looks like: a one-line "no consumer yet" note in the module doc (the TagBinary precedent), or wire cover_art() into the CLI's facts harvest.
Confidence: HIGH — Verification: rg cover_art/attachments() across apps/, playback-engine/ (zero hits; mp4.rs:651 is the mp4 lane's own test).

### F-005 [S4]
Where: crates/demux/src/mkv/attachments.rs:287-317 (test helpers)
Code:
```
    fn id_bytes(id: u32) -> Vec<u8> {
        // The marker bits of the id's first byte fix its width:
        // 0x10..0x1F → 4 bytes, 0x20..0x3F → 3, 0x40..0x7F → 2,
        // 0x80..0xFF → 1 (the u32's leading zeros count them).
        let width = 4 - id.leading_zeros() / 8;
        id.to_be_bytes()[4 - width as usize..].to_vec()
    }
```
What is wrong: `id_bytes`/`size_bytes`/`el` are copy-pasted into header.rs:860-885, chapters.rs (same trio), and mkv_extras_wiring.rs — four near-identical EBML builders across the mkv module instead of one shared test-support module (the crate already has the pattern: `crate::avi::test_support::mem_stream`).
Why it matters: divergence risk — a fix in one copy (like the 0x7F-vs-0x3F lesson from 52595bc8) won't propagate; `size_bytes`' >0x4000 branch also silently emits an 8-byte vint for any len < 2^56 with a comment claiming "any width ≥ minimal is legal" while the other copies differ in comments only.
What correct looks like: hoist to `crate::mkv::test_support` (mirroring avi) or demux-shared test kit.
Confidence: HIGH — Verification: rg the helper names across crates/demux/src/mkv/.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ID_* constants (7) | 44-57 | REAL | — | All byte-verified on a fresh ffmpeg -attach file (§2); FileDescription 0x467E not ffmpeg-observable (spec value, no in-sandbox oracle — MEDIUM on that one constant alone). |
| MAX_ATTACHMENTS / MAX_ATTACHMENT_BYTES | 59-69 | REAL | S3 (F-003) | No cumulative budget. |
| FileData enum | 71-87 | REAL | S3 (F-002) | NonInline branch has no consumer. |
| AttachedFile | 89-102 | REAL | — | Plain model; uid default 0 matches spec default. |
| AttachmentDamage / AttachmentList | 104-116 | REAL | — | Damage rows fold into damage_total at mod.rs:482 (verified). |
| AttachmentList::cover_art | 118-139 | REAL | S4 (F-004) | Correct selection; no shipped consumer. |
| parse_attachments | 151-190 | REAL | S2 (F-001) | Clamps master end (156); unknown-size inner = bad_structure (163-167); contained-lie at 169-172 is fatal (F-001); over-ceiling skip bounded (174-176); unknown children skipped by size (186-187). |
| parse_attached_file | 194-278 | REAL | S2 (F-001) | Field-level contained-lie fatal (213-217); text tolerance both ways (220-233); UID ≤8 enforced (234-242); FileData inline/non-inline split (243-258); mandatory-data degenerate skip (266-270). |
| test helpers (6) | 287-343 | REAL | S4 (F-005) | Duplicated builders; `parse` harness is honest (fresh mem_stream + real parse_header_at). |
| tests (9) | 345-546 | REAL | — | See §5. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| attached_files_parse_the_full_record (345-365) | YES | Every field + payload byte asserted. |
| over_ceiling_data_stays_the_non_inline_note (367-402) | YES | 33 MiB vs the 32 MiB boundary; asserts NonInline offset/len exactly; a `<=`→`<` mutation flips it. |
| truncated_file_data_is_the_typed_truncation (404-426) | YES | Pins the typed Err (and thereby pins the F-001 fatal behavior — the test codifies the defect as design). |
| file_data_overrunning_the_master_is_the_typed_truncation (428-446) | YES | Contained-lie shape, all bytes present, refuses. |
| attached_file_without_data_is_damage_skipped (448-456) | YES | Damage row + empty list. |
| invalid_utf8_labels_are_tolerated_as_absent (458-467) | YES | Name None, data survives. |
| cover_art_picks_the_image_attachment_not_the_font (469-510) | YES | Font excluded, MIME and name conventions both qualify. |
| attachment_count_ceiling_skips_with_damage (512-531) | YES | 1025 records → 1024 kept + 1 damage; order preserved. |
| unknown_size_inside_the_master_is_bad_structure (533-545) | YES | Inner 0xFF marker refused typed. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Attachments (0x1941A469) = full parse via MkvDemuxer::attachments()" | container-matroska.md:157-159 (5.REC-1) | REAL | mod.rs:399/424/480-485; baseline tests 1755-1763 + wiring lane. |
| "Bounds: MAX_ATTACHMENTS 1024; >32 MiB FileData = NonInline{offset,len}, never read" | container-matroska.md:167-170 | REAL (but the note is a dead end — F-002) | Lines 63-69/243-257; over-ceiling test. |
| "The consumer fetches it on demand through its own bounded access" | attachments.rs:78-79 | FALSE (no consumer exists) | F-002 rg sweep. |
| "a FileData whose declared extent overruns the master is the typed truncation" | attachments.rs:10-12 | REAL (understates fatality) | F-001: the refusal aborts the whole open, not just the list. |
| "cover_art() picks the JPEG" (generated lane) | container-matroska.md:249 | REAL | Executed wiring test (baseline ok). |

## 7. Dead / duplicated / unreachable code
- `FileData::NonInline`: constructed, matched in tests, never consumed in production (F-002).
- `cover_art()`/`attachments()` getters: test-only consumers (F-004).
- Test-builder duplication across 4 mkv files (F-005).
- No unreachable production code.

## 8. Dependencies & flags
- Deps: media_types, crate::mkv::{ebml,error}, header::MAX_TEXT_FIELD (cross-module constant reuse — fine). No external deps, no unsafe, no feature gates. std-only per C5 — verified.

## 9. Open questions for the next agent
1. mod.rs owner: is per-master fatal truncation the intended doctrine for ALL extras (tags/chapters too)? If yes, a truncated tail kills every mkv open — worth one corpus lane with a truncated-after-clusters file (the existing `malformed/mkv_h264_aac__trunc*.mkv` fixtures have no attachments; adding `-attach` to the generator would expose F-001 immediately).
2. apps/cli mkv host: wire cover_art() or record the gap (mp4 lane has one).
3. Whoever owns tags.rs: the cumulative-budget precedent (MAX_TAG_LIST_BYTES) should be cloned here (F-003).
