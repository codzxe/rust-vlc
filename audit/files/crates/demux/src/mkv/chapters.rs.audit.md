# AUDIT: crates/demux/src/mkv/chapters.rs

| Field | Value |
|---|---|
| Auditor | W05-A05 |
| Date | 2026-11-22 |
| Lines / functions | 875 / 30 (16 prod fns + 6 test helpers + 11 tests + structs/constants) |
| File verdict | REAL-UNPROVEN (parser) / getters UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | parse: YES — `apps/cli/src/containers/mod.rs:57 "matroska" => mkv::open` → `apps/cli/src/containers/mkv.rs:147 MkvDemuxer::new` → `mod.rs:909 Demuxer::start` → `mod.rs:340 open()` walk → `mod.rs:425 chapters::ID_CHAPTERS => collect_chapters` → `mod.rs:490 parse_chapters`. Getters `chapters()`/`chapter_targets()`: NO production caller (tests only; see F-002). |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:4 |
| Commands run | Read (full file, 2 chunks); Grep for callers (`parse_chapters`, `chapter_targets`, `chapters()`, `MediaGetChapters`); sed -n verification of every quote below. cargo NOT available (adaptation) — no execution. |

## 1. One-paragraph verdict

This file is a genuine, bounded, spec-aware Matroska `Chapters` parser: real EBML walking, real element IDs (verified against the Matroska ID table), real ceilings (`MAX_EDITIONS`=64, `MAX_CHAPTER_ATOMS`=65 536, `MAX_CHAPTER_DISPLAYS`=16, `MAX_CHAPTER_TRACKS`=64, text fields ≤4 KiB), a real recursion depth guard, and correct unit handling (chapter times kept as raw nanoseconds, NOT scale ticks — this matches the Matroska spec and the module's own claim). The tests build real EBML byte streams and assert real values; they are not tautologies. The two real defects: the module doc promises damage rows for over-ceiling track numbers that the code never records, and the entire parsed chapter surface dead-ends — no shipped entry point (CLI command, GUI, engine dispatch) ever reads `MkvDemuxer::chapters()` or `chapter_targets()`; the only `media.get-chapters` "handler" is the UI demo-transport mock. Parsing chapters is pure wasted work in the shipped CLI.

## 2. What actually works (verified)

— Element IDs cross-checked against the Matroska spec table: `Chapters` 0x1043A770, `EditionEntry` 0x45B9, `ChapterAtom` 0xB6, `ChapterTimeStart` 0x91, `ChapterDisplay` 0x80, `ChapLanguageIETF` 0x437D — all correct (Read of lines 55-92).
— Allocation caps are enforced in code, not just claimed: `chapters.rs:292` (editions), `chapters.rs:344-350` (atoms → typed ResourceLimit), `chapters.rs:414-417` (displays), `chapters.rs:479` (tracks) — verified by reading each branch.
— Depth guard is real and tested: `chapters.rs:427-429` + `ebml.rs:46 pub const MAX_PARSE_DEPTH: usize = 16`.
— ns-vs-ticks unit split is correct: chapter times stored raw ns (`chapters.rs:455`), scale conversion happens exactly once, with floor division, in `mod.rs:296 let Some(start_ticks) = i64::try_from(atom.start_ns / scale).ok()` — ticks = ns / (ns-per-tick) is dimensionally right, floor = land-at-or-before (matches test `chapter_targets_seek_lands_at_or_before` in mkv_extras_wiring.rs:609).
— W00 baseline (audit/runs/baseline): workspace tests 1751 pass — the demux crate's tests compile and pass in that run; per-test attribution to this file UNVERIFIED (no cargo in this agent's sandbox).

## 3. Findings

### F-001 [S3] — Doc promise "skipped with damage" is false for track numbers
Where: crates/demux/src/mkv/chapters.rs:17-24 vs 479-485
Code (doc, verbatim):
```
//!   ([`MAX_EDITIONS`], [`MAX_CHAPTER_ATOMS`],
//!   [`MAX_CHAPTER_DISPLAYS`], [`MAX_CHAPTER_TRACKS`]). Over-ceiling
//!   editions/displays/tracks are skipped with damage; a total atom
```
Code (reality, verbatim):
```
        if child.id == ID_CHAPTER_TRACK_NUMBER && taken < MAX_CHAPTER_TRACKS && len <= 8 {
            let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
            if let Some(value) = ebml::uint_from_be(&bytes) {
                parsed.track_numbers.push(value);
                taken += 1;
            }
        }
```
What is wrong: an ordered-edition atom with more than `MAX_CHAPTER_TRACKS` track numbers (or any `ChapterTrackNumber` wider than 8 bytes) drops them silently. `parse_track_numbers` (460-489) has no damage parameter and records nothing — contrast `parse_atom`'s `"display-over-ceiling"` row (417) and `parse_chapters`'s `"edition-over-ceiling"` row (293).
Why it matters: the file's damage contract ("honesty counter") is the audit surface other code sums into `damage_total` (`mod.rs:491`); track-number drops are invisible there. A file claiming 65 scoped tracks will silently lose one and nothing reports it.
What correct looks like: thread `&mut ChapterDamage` into `parse_track_numbers` and push `(child_end - pos, "track-over-ceiling")` (and a `"track-number-too-wide"` row for len > 8), or fix the module doc to say tracks are dropped silently.
Confidence: HIGH — Verification: read

### F-002 [S3] — The whole parsed chapter surface dead-ends in the shipped build
Where: crates/demux/src/mkv/chapters.rs:239-256 (getters' backing types) + crates/demux/src/mkv/mod.rs:270-274
Code (verbatim, mod.rs:270-274 — the honest admission):
```
    /// ticks). `[PROPOSAL: the chapter-navigation command
    /// (feature-matrix `playback-chapter-title`, phase 4) consumes
    /// this surface — `seek(target.span)` + `title` for the UI; the
    /// command layer does not exist yet, so nothing downstream reads
    /// it today]`.
```
Evidence chain: Grep `\.chapters\(\)|\.chapter_targets\(\)` across the repo (excluding demux tests) returns only `apps/cli/src/containers/mp4.rs:640` (the MP4 host's own chapters, different demuxer) and demux test files. Grep `MediaGetChapters` in `*.rs` returns only catalog definitions (commands/src/catalog/{command.rs:379, spec.rs:566, ids.rs:154}) — **no Rust handler exists**; the only `media.get-chapters` handler in the tree is the UI mock `ui/src/client/demo-transport.ts:585`. The CLI host admits it: `apps/cli/src/containers/mkv.rs:112-114` — "**Chapters/Editions/ordered-chapters**: demuxer-side metadata / only — `MediaFacts` carries no chapter vocabulary".
What is wrong: `MkvDemuxer::chapters()` (mod.rs:240) and `chapter_targets()` (mod.rs:276) — this file's entire output vocabulary (`ChapterList`, `ChapterTarget`) — have zero production consumers. Every MKV open pays the chapters parse and the user can never see a chapter.
Why it matters: `docs/feature-matrix.toml:1578` cites this file as evidence for the chapter-navigation row; a parser whose output nobody reads is not chapter navigation. (Row verdict itself belongs to wave 19; recorded here as the reachability fact.)
What correct looks like: either wire `chapter_targets()` into the CLI host's facts / a `media.get-chapters` engine handler, or mark the surface experimental so matrix rows stop citing it.
Confidence: HIGH — Verification: read + Grep (exhaustive caller search)

### F-003 [S4] — parse_display reads and discards payloads for unknown IDs
Where: crates/demux/src/mkv/chapters.rs:512-521
Code (verbatim):
```
        if len <= MAX_TEXT_FIELD as usize {
            let bytes = reader.read_prefix_at(len, child.data_pos, child_end)?;
            let text = ebml::utf8_from(&bytes);
            match child.id {
                ID_CHAP_STRING => parsed.string = text,
```
What is wrong: the bytes are read and UTF-8-validated before the ID is matched; the `_ => {}` arm throws the work away. Unknown `ChapterDisplay` children cost a bounded (≤4 KiB) allocation + decode each.
Why it matters: pure waste on a metadata path; bounded, so hygiene not perf bug.
What correct looks like: `match child.id { known => { read… } _ => {} }`.
Confidence: HIGH — Verification: read

### F-004 [S4] — Over-ceiling text fields vanish without a damage row
Where: crates/demux/src/mkv/chapters.rs:510-522
Code (verbatim):
```
        // Every field here is advisory text: tolerant both ways
        // (over-ceiling or invalid UTF-8 → absent, never fatal).
        if len <= MAX_TEXT_FIELD as usize {
```
What is wrong: a `ChapString` longer than 4 KiB (or invalid UTF-8) becomes `None` with no `(bytes, reason)` row, unlike every other over-ceiling case in this file. The in-code comment admits the behavior, so this is an internal-consistency defect, not a hidden lie.
Confidence: HIGH — Verification: read

### F-005 [S4] — Stale comments: "getter" does work that `open()` does
Where: crates/demux/src/mkv/chapters.rs:150-152 and 234-235
Code (verbatim, 150-152):
```
    /// Tolerant note (set by the demuxer getter): this atom's times
    /// extend past the segment's declared `Duration` — kept, flagged,
    /// never dropped.
```
What is wrong: `past_duration` is set by `MkvDemuxer::open()` (`mod.rs:461 self.chapters.flag_beyond_duration(...)`), not by any getter; same for line 15's "the demuxer getter compares them against the segment duration". The comments describe the pre-issue-#34 design.
Confidence: HIGH — Verification: read (mod.rs:455-463 confirms open() is the caller)

### F-006 [S4] — Inconsistent refusal for >8-byte integers
Where: crates/demux/src/mkv/chapters.rs:479 vs 536-538
Code (verbatim, 536-538):
```
    if len > 8 {
        return Err(error::bad_structure(child.data_pos));
    }
```
What is wrong: `read_uint` types a >8-byte uint as `bad_structure`, but `parse_track_numbers`'s `len <= 8` guard (line 479) silently skips the same shape. Two behaviors for the same grammar violation inside one file.
Confidence: HIGH — Verification: read

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts ID_* (19) | 55-92 | REAL | — | IDs verified against Matroska spec; all correct |
| MAX_EDITIONS / MAX_CHAPTER_ATOMS / MAX_CHAPTER_DISPLAYS / MAX_CHAPTER_TRACKS | 94-112 | REAL | — | caps enforced at use sites (not decorative) |
| struct ChapterDisplay | 117-127 | REAL | — | plain model |
| struct ChapterAtom | 131-154 | REAL | — | model; ns units documented correctly |
| ChapterAtom::title | 160-162 | REAL | — | first titled display; no panic paths |
| ChapterAtom::flatten_into | 165-170 | REAL | — | recursion bounded by parse-time depth guard (≤16) |
| struct EditionEntry | 174-189 | REAL | — | model |
| struct ChapterList | 193-202 | REAL | — | damage rows carried |
| ChapterList::atoms_flattened | 209-217 | REAL | — | allocates; bounded by atom cap |
| ChapterList::default_edition | 225-231 | REAL | — | default → first non-hidden → first; tested |
| ChapterList::flag_beyond_duration | 237-240 | REAL | — | delegates; no overflow (plain u64 compare) |
| struct ChapterTarget | 246-256 | REAL | — | doc says span is scale ticks — matches mod.rs:296 conversion |
| parse_chapters | 270-304 | REAL-UNPROVEN | — | unknown-size clamps to bound; overrun child → typed truncated; editions cap + damage; loop always advances (header ≥1 byte) so no infinite loop |
| parse_edition | 308-363 | REAL-UNPROVEN | — | atom budget shared via `atoms_total`; over-cap → typed refusal (whole open fails — documented choice) |
| parse_atom | 367-457 | REAL-UNPROVEN | — | depth guard 427-429; degenerate atom → Ok(None) + damage; zero-length TimeStart reads 0 (uint_from_be empty=Some(0)) so atom survives — spec-correct |
| parse_track_numbers | 460-489 | REAL-UNPROVEN | S3 | F-001: silent drop over cap / len>8; no damage channel |
| parse_display | 492-526 | REAL-UNPROVEN | S4 | F-003/F-004: reads unknown IDs then discards; over-4KiB text silently absent |
| read_uint | 530-541 | REAL | — | >8 → bad_structure; exact read enforced by read_exact_at (ebml.rs:379-394) |
| read_flag | 544-551 | REAL | — | zero-length → false, matches spec default quirk |
| chapter_table_too_large | 557-567 | REAL | — | typed ResourceLimit(Memory) refusal |
| flag_beyond_duration (free) | 571-579 | REAL | — | recursion bounded |
| flag_atom | 582-595 | REAL | — | no multiplication → no overflow (u64::MAX test passes) |
| test helper id_bytes | 604-610 | REAL | — | correct marker-bit width math |
| test helper size_bytes | 613-624 | REAL | — | 1/2/8-byte vints; correct |
| test helper el / chapters_element / atom | 627-646 | REAL | — | builds real EBML |
| test helper parse | 649-653 | REAL | — | in-memory stream; `.must().must()` = Result then Option, correct |
| 11 tests | 656-874 | REAL-UNPROVEN | — | see §5; cannot execute (no cargo) |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| editions_atoms_and_displays_parse_the_full_record (656) | YES | asserts uid=5, ordered, default, start/end ns, track_numbers=[1], all 4 display strings — any parse regression breaks an assert |
| default_edition_selects_flag_then_first_visible_then_first (704) | YES | exercises all 3 fallback tiers with distinct uids |
| nested_atoms_parse_to_the_depth_guard (733) | YES | 16 levels parse / 17th refuses with exact error code |
| atom_table_ceiling_is_the_typed_resource_limit (760) | YES | 65 537 atoms → asserts ResourceLimit kind + code |
| atom_without_time_start_is_damage_skipped (783) | YES | asserts atoms.len()==1 + damage reason string |
| edition_ceiling_skips_with_damage (799) | YES | asserts 64 editions + 1 damage row |
| truncated_atom_field_is_the_typed_truncation (812) | YES | 8-byte declare / 4-byte stream → asserts `demux.mkv.truncated` |
| times_past_the_duration_are_the_tolerant_note (826) | YES | flags exactly the 2..9 µs atom against 5 µs |
| extreme_times_land_in_the_tolerant_bucket (842) | YES | u64::MAX ns survives without overflow |
| unknown_size_inside_chapters_is_bad_structure (856) | YES | 0xFF size vint → exact code |
| empty_chapters_element_is_an_empty_list (869) | YES (weakly) | only asserts emptiness; lowest-value test here |

Note: no test in this file covers `parse_track_numbers` over-cap or `len > 8` (the F-001 hole) — consistent with the doc's false claim never being caught.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Chapter times are nanoseconds … do not scale with TimestampScale" | chapters.rs:8-16 module doc | REAL (as implemented) | code stores raw value (chapters.rs:455); conversion only in mod.rs:296. The ffprobe-verification story (0007 corpus) is UNVERIFIED — no corpus, no cargo in sandbox |
| "Over-ceiling editions/displays/tracks are skipped with damage" | chapters.rs:21-22 | FALSE for tracks | F-001: parse_track_numbers records nothing |
| "a total atom count past MAX_CHAPTER_ATOMS is the typed … refusal" | chapters.rs:22-24 | REAL | chapters.rs:344-350 + test 760 |
| "a nesting bomb is the typed bad_structure, never a stack overflow" | chapters.rs:27-29 | REAL | chapters.rs:427-429, MAX_PARSE_DEPTH=16 (ebml.rs:46), test 733 |
| matrix row evidence "… + MKV ChapterList (crates/demux/src/mkv/chapters.rs …)" | docs/feature-matrix.toml:1578 | REAL-UNPROVEN as parse; UNREACHABLE as user feature | F-002 chain: no production consumer of the getters |

## 7. Dead / duplicated / unreachable code

- `ChapterTarget` + `ChapterList` + both getters: reachable as library API, **unreachable from any shipped entry point** (F-002). The parse itself runs on every MKV open (so its failure paths ARE user-reachable: a hostile Chapters element can fail the whole open via the atom ceiling — a documented, typed refusal, not a crash).
- No duplicated logic found within the file; the 4 walk loops (parse_chapters/parse_edition/parse_atom/parse_track_numbers/parse_display) share shape but differ in fields — acceptable, not copy-paste drift.

## 8. Dependencies & flags

- No feature gates: module is unconditionally compiled into `demux` (`crates/demux/src/lib.rs:50 pub mod mkv;`), and `demux` is a dependency of the shipped CLI (apps/cli/src/containers/mkv.rs:121 `use demux::mkv::MkvDemuxer;`). No cfg(test) leakage into production: `#[cfg(test)] mod tests` at line 597 is test-only; test_support usage (`crate::avi::test_support::mem_stream`, line 600) is inside that module only.
- External deps used: `media_types` (error/time types) — in-crate types otherwise.

## 9. Open questions for the next agent

- mkv_extras_wiring.rs / mkv_attachments_chapters.rs / mkv_editions_tags.rs (tests): do they cover the F-001 track-number hole? (mkv_editions_tags.rs is mine — answer coming in that audit.)
- Wave 19 (claim ledger): does matrix row `playback-chapter-title` survive F-002? The evidence string cites this file + demo-transport handlers — a mock handler is not an engine implementation.
- Who consumes `playback-engine::chapters::chapter_target`? If also nobody, the entire chapter-navigation chain (demux → engine → UI) is two dead ends joined by a wire type. (Grep suggests only catalog + demo-transport; not fully traced — outside my files.)
