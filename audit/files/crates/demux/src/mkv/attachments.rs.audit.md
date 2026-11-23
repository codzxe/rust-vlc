# AUDIT: crates/demux/src/mkv/attachments.rs
| Field | Value |
|---|---|
| Auditor | W05-A07 |
| Date | 2025-05-24 |
| Lines / functions | 546 / 3 fns + 1 method + 9 tests |
| File verdict | REAL-UNPROVEN (parse real, hostile cases well-tested; exposure seam has no shipped consumer; per-master ceiling does not bound the retained list) |
| Reachable from shipped entry point? | YES (parse): apps/cli containers/mod.rs:57 -> mkv::open -> MkvDemuxer::open (mod.rs:399/424 `collect_attachments` -> `parse_attachments`). NO (surface): `attachments()`/`cover_art()`/`FileData::NonInline` consumed by zero shipped code (rg over apps/ + crates/ outside demux: only demux's own tests). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 1 |
| Commands run | `cargo test -p demux --lib mkv::attachments` -> 9 passed, exit 0; sed quote verification (7 blocks, byte-verbatim); rg consumer searches |

## 1. One-paragraph verdict
The attachments parser is the best-behaved file in my set: unlike header.rs, it actually enforces the parent-extent clamp it documents (`child_end > end` -> typed `truncated` at both the master and record level), the per-file 32 MiB inline ceiling is real (over-ceiling payloads become a never-read `NonInline { offset, len }` note — no allocation from declared size), and 9 tests cover the hostile paths including the contained lie, the bomb note, and the count ceiling (my run: 9/9). Hostile-focus answer on size trust: declared `FileData` sizes are NOT trusted for allocation — inline reads are exact (`read_exact_at` against the record end, master end, and stream length). Defects: one overrunning record/field aborts the ENTIRE file open (fatal `?` chain), inconsistent with this same file's damage-skip doctrine for every other per-record defect; `MAX_ATTACHMENTS` bounds only a single `Attachments` master while mod.rs merges every master's files into one list, so the documented "file count ... ceiling-checked" is not the real bound; and the whole surface (cover art included) is parsed, retained, and tested but wired to nothing in the shipped CLI/desktop.

## 2. What actually works (verified)
— `cargo test -p demux --lib mkv::attachments` -> `test result: ok. 9 passed; 0 failed` (exit 0). Full record parse, over-ceiling non-inline note (33 MiB declared, never read), truncated payload typing, master overrun typing, no-FileData damage skip, invalid-UTF-8 tolerance, cover-art selection, 1025-file ceiling, unknown-size-inside-master refusal. Baseline corroborates: audit/runs/baseline/cargo-test-workspace.txt:1927-1935.

## 3. Findings
F-001 [S3]
Where: crates/demux/src/mkv/attachments.rs:168-173, 213-217; crates/demux/src/mkv/mod.rs:481
Code:
```
        if child.id == ID_ATTACHED_FILE {
            if child_end > end {
                // The record claims bytes past the master: the
                // contained lie, typed at the master's end.
                return Err(error::truncated(child_end, end));
            }
```
```
        if child_end > end {
            // A field claiming bytes past its `AttachedFile`: the
            // contained lie, typed at the record's end.
            return Err(error::truncated(child_end, end));
        }
```
What is wrong: One `AttachedFile` (or one field inside one) whose declared extent overruns its parent aborts the whole `Attachments` parse; `MkvDemuxer::open` calls `collect_attachments` with `?` (mod.rs:399/424), so a single malformed attachment refuses to open the entire media file. Every OTHER per-record defect in this same file is damage-contained (no-FileData -> skip at 180-182, count over-ceiling -> skip at 174-176, bad UTF-8 -> absent at 220-232). The asymmetry is documented ("the contained lie, typed") but it is containment doctrine violated where it matters most — a corrupt attachment killing A/V playback. What correct looks like: skip the overrunning record with a damage row (it already has the byte-count shape) and keep the survivors. Confidence: HIGH — Verification: read + test run (the fatal behavior is what the test asserts, so it is intentional — the finding is that the intention is wrong).

F-002 [S3]
Where: crates/demux/src/mkv/attachments.rs:59-63 (doc) vs 174-176; crates/demux/src/mkv/mod.rs:483
Code:
```
/// Maximum attached files trusted into the list
/// `[PROPOSAL: 1024 — real files carry < 10 (covers, fonts); a file
/// past this is a metadata bomb and is skipped with damage, the rest
/// survive]`.
pub const MAX_ATTACHMENTS: usize = 1024;
```
```
            if list.files.len() >= MAX_ATTACHMENTS {
                list.damage
                    .push((child_end - pos, "attachment-over-ceiling"));
```
```
        self.attachments.files.extend(more.files);
```
What is wrong: `MAX_ATTACHMENTS` is checked per `parse_attachments` call, but a Matroska file may carry multiple `Attachments` masters and mod.rs merges them (`files.extend`), so the RETAINED list is unbounded by this ceiling (N masters x 1024 files). The doc sells the constant as the bound on "files trusted into the list" — the list it does not bound. Memory is still file-size-bounded (inline reads require physically present bytes), so no bomb-from-header; this is a doc/behavior mismatch plus an aggregate ceiling gap. What correct looks like: enforce the ceiling on the accumulating list in mod.rs (or document the per-master scope). Confidence: HIGH — Verification: read.

F-003 [S3]
Where: crates/demux/src/mkv/attachments.rs:28-34 (doc) + 126-138; apps/cli/src/containers/mkv.rs:195-236 (absence)
Code:
```
//! # Exposure seam (the card's "info/attachment surface")
//!
//! [`MkvDemuxer::attachments`] is the typed getter over the list the
//! demuxer **retains at `start`** (issue #34 wired the `open()` seam:
```
```
    pub fn cover_art(&self) -> Option<&AttachedFile> {
        self.files.iter().find(|file| {
```
What is wrong: The doc frames this as "the typed AttachedFile records a UI needs (cover art, fonts ...)" — but no shipped consumer exists: `rg` over apps/ and crates/ (excluding this file) finds `attachments()`, `cover_art()`, and `FileData::NonInline` referenced ONLY by demux's own tests. The CLI host's `harvest_facts` (containers/mkv.rs:195) reads `track_entries`/`TrackPrivate` only; the mp4 lane surfaces cover art as a still-image track (mp4.rs), the mkv lane surfaces nothing. Parsed, retained, tested — wired to nothing. `FileData::NonInline` additionally has a "consumer fetches it on demand" contract (lines 78-80) whose consumer does not exist anywhere. What correct looks like: either a consumer in the UI facts lane or an honest "retained for future issues #17/#34" doc. Confidence: HIGH — Verification: read + rg.

F-004 [S4]
Where: crates/demux/src/mkv/attachments.rs:132-135
Code:
```
            let cover_name = file.name.as_deref().is_some_and(|n| {
                let lower = n.to_ascii_lowercase();
                lower.starts_with("cover.") || lower.starts_with("front.")
            });
```
What is wrong: Name-only matching cannot distinguish a font named `cover.ttf`/`front.otf` from artwork (MIME absent -> the name convention alone qualifies); a font shipped under that name would be picked as cover art. Contrived name, tiny blast radius, and the same heuristic mkvtoolnix consumers use — hygiene note only. Confidence: MEDIUM — Verification: read (no fixture demonstrates it).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `AttachmentList::cover_art` | 126-138 | REAL (unconsumed) | S3/S4 | F-003 dead seam; F-004 name-heuristic nit |
| `parse_attachments` | 151-190 | REAL | S3 | unknown-size inner -> bad_structure; master overrun -> fatal (F-001); per-master ceiling (F-002); unknown children skipped by size; damage doc "(bytes, reason)" CORRECT here (contrast header.rs F-002) |
| `parse_attached_file` | 194-278 | REAL | S3 | field overrun -> fatal (F-001); text fields 4 KiB ceiling + UTF-8 tolerant; FileUID len>8 typed; FileData: exact read <= 32 MiB, over-ceiling NonInline never read (size trust CORRECT); last-FileData-wins tolerant |
| tests: helpers | 287-343 | REAL | — | correct vint/element builders; unknown-size master builder present |
| test `attached_files_parse_the_full_record` | 345-365 | REAL | — | full field round-trip, ran ok |
| test `over_ceiling_data_stays_the_non_inline_note` | 367-402 | REAL | — | 33 MiB declared, synthetic bound, asserts never-read; ran ok |
| test `truncated_file_data_is_the_typed_truncation` | 404-426 | REAL | — | asserts Corrupt::Truncated + code; ran ok |
| test `file_data_overrunning_the_master_is_the_typed_truncation` | 428-446 | REAL | — | proves the F-001 fatal path is intended; ran ok |
| test `attached_file_without_data_is_damage_skipped` | 448-456 | REAL | — | damage containment proven; ran ok |
| test `invalid_utf8_labels_are_tolerated_as_absent` | 458-467 | REAL | — | ran ok |
| test `cover_art_picks_the_image_attachment_not_the_font` | 469-510 | REAL | — | mime + name-convention + font-negative; ran ok |
| test `attachment_count_ceiling_skips_with_damage` | 512-531 | REAL | — | 1025 files -> 1024 + 1 damage; ran ok |
| test `unknown_size_inside_the_master_is_bad_structure` | 533-545 | REAL | — | ran ok |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 9 | YES | every hostile path asserted with typed error codes or exact values; I ran them (9/9). The one doctrine gap: none tests MULTIPLE Attachments masters (F-002's aggregate ceiling hole). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every child read is clamped to the Attachments master's effective end and the stream's physical end" | attachments.rs:8-10 | REAL | enforced at 169-172 + 213-216 + read_exact_at (ebml.rs:385-387); tests 429-446 |
| "a bomb-sized attachment cannot become a bomb-sized allocation" | attachments.rs:17-18, 78-80 | REAL | `if child.size <= MAX_ATTACHMENT_BYTES` gate at 244; NonInline note at 253-256; test 367-402 |
| "The file count itself is ceiling-checked ... over-ceiling files are skipped" | attachments.rs:19-20 | SHALLOW | per-master only; mod.rs:483 `extend` unbounded across masters (F-002) |
| "the typed AttachedFile records a UI needs" | attachments.rs:5 | PAPER-ONLY at app level | zero consumers outside demux tests (F-003) |
| "FileData is mandatory per spec: a record without it is degenerate" | attachments.rs:26-27, 266-270 | REAL | Matroska spec agrees; test 448-456 |

## 7. Dead / duplicated / unreachable code
- `FileData::NonInline`: constructed (251-256), tested (367-402), consumed by NOTHING shipped. The "consumer fetches it on demand" contract has no implementor.
- `cover_art()`: real logic, tested, no shipped caller.
- Test helpers `id_bytes`/`size_bytes`/`el` duplicated byte-for-byte from header.rs tests (287-317 vs header.rs 860-885) — copy-paste across modules in the same crate (hygiene; a shared test_support would own them).

## 8. Dependencies & flags
Depends on: `media_types::EngineError`, `crate::mkv::ebml` (reader/parse_header_at/uint_from_be/utf8_from), `crate::mkv::error`, `crate::mkv::header::MAX_TEXT_FIELD` (cross-module constant reuse — fine), test-only `crate::avi::test_support::mem_stream` + `testkit::Must`. No features, no unsafe. Parse reachable via open() walk; surface unreached (F-003).

## 9. Open questions for the next agent
- Should the record-overrun path (F-001) be damage-contained like the no-FileData path? If the orchestrator accepts the fatal design, header.rs's parent-escape (header.rs F-004) is the inconsistency to resolve — attachments.rs rejects what header.rs silently reads.
- If a UI consumer lands for cover_art, F-004's name heuristic needs the font-vs-art disambiguation (extension allowlist).
- mkv_attachments_chapters.rs / mkv_extras_wiring.rs integration tests were not audited by me (other agents' files) — they cover the retained-list merge my F-002 concerns.
