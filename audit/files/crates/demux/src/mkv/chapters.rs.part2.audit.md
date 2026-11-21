# AUDIT: crates/demux/src/mkv/chapters.rs#part2 (lines 438–875)
| Field | Value |
|---|---|
| Auditor | W05-A04 |
| Date | 2026-11-21 |
| Lines / functions | 438 (chunk 438–875 of 875) / 8 production + 6 test helpers + 11 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open x.mkv` → containers/mod.rs:66 `"matroska" => mkv::open` → `MkvDemuxer` walk → mkv/mod.rs:490 `chapters::parse_chapters(&mut self.reader, child, self.segment_end)?`; getter mkv/mod.rs:240 `pub fn chapters()`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "chapters::tests::" audit/runs/baseline/test-workspace.log.txt` → 11/11 ok (log lines 1764–1774). `sed -n '2130,2160p'` same log → `corpus_attachments_chapters_differential_vs_ffprobe ... ok`, `generated_fixture_attachments_chapters_differential ... ok`, mkv_editions_tags 14/14 ok — the chapters lane is ffprobe-differential-tested. `rg -n "parse_chapters" crates/demux/src/mkv/mod.rs` → :490 (reachability). `sed -n '492,500p' crates/demux/src/mkv/ebml.rs` → `uint_from_be` returns `Some(0)` for empty input (loop over zero bytes). All quoted ranges verified with `sed -n`. |

Chunk boundary: part1 (W05-A05) = lines 1–437 (module docs, element ids, ceilings, data model, `ChapterList`/`ChapterTarget`, `parse_chapters`, `parse_edition`, `parse_atom` head incl. display/track dispatch); part2 (this audit) = 438–875: `parse_atom` tail (nested-atom arm + degenerate check), `parse_track_numbers`, `parse_display`, `read_uint`, `read_flag`, `chapter_table_too_large`, `flag_beyond_duration`/`flag_atom`, and the whole `#[cfg(test)] mod tests`.

## 1. One-paragraph verdict
The part2 parse tail and the flag walk are real, typed, ceiling-bounded, and differentially validated: 11 unit tests plus two ffprobe-differential corpus tests ran green in the baseline, and the whole lane is reachable from `lumen open file.mkv`. The recursion depth guard, atom-table ceiling (65 536), edition ceiling, and tolerant beyond-duration flagging all work as documented and are pinned by tests. The defects are honesty gaps in the small lanes: `parse_track_numbers` silently drops over-ceiling, over-wide, and unparseable track numbers with no damage row — directly contradicting the module doc's "Over-ceiling editions/displays/tracks are skipped with damage"; a present-but-zero-length `ChapterFlagEnabled` reads as disabled while the same field absent reads as enabled (spec default 1); and over-length display text vanishes with zero observability.

## 2. What actually works (verified)
— `rg -n "chapters::tests::" audit/runs/baseline/test-workspace.log.txt` → 11/11 `... ok` (lines 1764–1774), incl. `nested_atoms_parse_to_the_depth_guard` and `atom_table_ceiling_is_the_typed_resource_limit`.
— `sed -n '2130,2160p' audit/runs/baseline/test-workspace.log.txt` → `corpus_attachments_chapters_differential_vs_ffprobe ... ok` + `generated_fixture_attachments_chapters_differential ... ok` — chapter titles/times match ffprobe on the corpus and on ffmpeg-generated fixtures (the module doc's ns-not-scale-ticks claim, lines 8-16, is the thing these differentials pin).
— Vint helpers hand-checked: `id_bytes(0x1043A770)` → 4-byte vint (7 leading zeros → width 4), `id_bytes(0x45B9)` → `[0x45,0xB9]`; `size_bytes` produces legal 1/2/8-byte size vints. ✓

## 3. Findings

### F-001 [S3] `parse_track_numbers` silently drops over-ceiling / over-wide / unparseable track numbers — the module doc promises damage rows
Where: crates/demux/src/mkv/chapters.rs:479-485 (code), 21-22 (doc claim)
Code (479-485):
```rust
        if child.id == ID_CHAPTER_TRACK_NUMBER && taken < MAX_CHAPTER_TRACKS && len <= 8 {
            let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
            if let Some(value) = ebml::uint_from_be(&bytes) {
                parsed.track_numbers.push(value);
                taken += 1;
            }
        }
```
Code (doc, 21-22):
```rust
//!   [`MAX_CHAPTER_DISPLAYS`], [`MAX_CHAPTER_TRACKS`]). Over-ceiling
//!   editions/displays/tracks are skipped with damage; a total atom
```
What is wrong: the function takes no `damage` parameter at all. A 65th `ChapterTrackNumber` (ceiling 64), a `len > 8` number, or any unparseable value falls through the `if` with no record — contradicting the module doc's explicit "Over-ceiling editions/displays/tracks are skipped with damage". Editions over ceiling push `"edition-over-ceiling"` (line 293, tested at 798-809); displays over ceiling push `"display-over-ceiling"` (line 417, part1); tracks push nothing. Also `uint_from_be` can never return `None` here (len ≤ 8 is pre-checked and `uint_from_be` only refuses >8 — ebml.rs:492-500), so the `if let Some` guard is dead.
Why it matters: an ordered-edition scoping bomb is dropped without a trace, and a reader auditing the damage ledger the doc promises will find the track lane silent; the doc-vs-code contradiction is exactly the drift the damage-row convention exists to prevent.
What correct looks like: thread `&mut ChapterDamage` in and push `("chapter-track-over-ceiling", …)` / `("chapter-track-invalid", …)` rows; fix the doc or the code — they currently disagree.
Confidence: HIGH — Verification: read (signature + call sites; doc quote).

### F-002 [S4] `ChapterFlagEnabled` present-but-empty reads as disabled; the same field absent reads as enabled
Where: crates/demux/src/mkv/chapters.rs:543-551 (read_flag), 140-144 (struct doc), 375-378 (init, part1)
Code:
```rust
/// A flag field (nonzero = set; a zero-length element reads 0).
fn read_flag(
    reader: &mut EbmlReader,
    child: &ElementHeader,
    len: usize,
    child_end: u64,
) -> Result<bool, EngineError> {
    Ok(read_uint(reader, child, len, child_end)?.unwrap_or(0) != 0)
}
```
Code (struct doc, 142-143):
```rust
    /// `ChapterFlagEnabled` (default on).
    pub enabled: bool,
```
What is wrong: EBML zero-length elements mean "default value". `ChapterFlagEnabled`'s Matroska default is 1 (on) — the struct doc itself says "default on", and the constructor honors it (`enabled: true`, line 376). But a present-but-zero-length `ChapterFlagEnabled` goes through `read_uint` → `uint_from_be(&[])` → `Some(0)` (verified ebml.rs:492-500: loop over zero bytes yields 0) → `read_flag` returns `false` — the chapter is silently disabled. Absent and empty, the two shapes that must mean the same thing, produce opposite results. (`ChapterFlagHidden`/`EditionFlag*` all default 0, so the same bug is invisible there.)
Why it matters: a spec-legal encoding flips chapter enablement; the in-file doc and the code disagree about the default.
What correct looks like: `read_flag` needs the field's default — e.g. `read_flag_or(default: bool)` mapping empty → default; or reject zero-length flags as bad_structure instead of guessing.
Confidence: HIGH — Verification: read + ebml.rs empty-slice behavior verified by sed (492-500). Rarity of zero-length flags in the wild keeps this S4, not S3.
Confidence note: blast radius judged MEDIUM-rare; if the corpus contains zero-length flags, raise to S3.

### F-003 [S4] Over-length display text and invalid UTF-8 vanish with zero observability
Where: crates/demux/src/mkv/chapters.rs:510-522
Code:
```rust
        // Every field here is advisory text: tolerant both ways
        // (over-ceiling or invalid UTF-8 → absent, never fatal).
        if len <= MAX_TEXT_FIELD as usize {
            let bytes = reader.read_prefix_at(len, child.data_pos, child_end)?;
            let text = ebml::utf8_from(&bytes);
```
What is wrong: a `ChapString` longer than `MAX_TEXT_FIELD` (4 KiB, header.rs:60) or with invalid UTF-8 becomes `None` — no damage row, no note. The in-line comment documents the tolerance, but every other skip lane in this module (editions, displays, atoms-without-start) writes a damage row; the UI shows an untitled chapter with no way to know the file carried one.
Why it matters: silent data loss on the one field a chapter exists to carry (the title); a 5 KiB chapter title is unusual but legal, and a mojibake title is common in the wild.
What correct looks like: push a `("chapter-text-over-ceiling"|"chapter-text-invalid-utf8", bytes)` damage row while still keeping the chapter.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse_atom` (tail; full fn 365-457 straddles boundary) | 438-457 | REAL | — | Nested push, degenerate check (no TimeStart → subtree dropped, documented, matches ffmpeg behavior claim); `start_ns` assignment. Head audited by W05-A05. |
| `parse_track_numbers` | 459-489 | REAL (flawed) | S3 | F-001: silent drops, no damage param; parent/child overrun checks present; `taken` bounded. |
| `parse_display` | 491-526 | REAL (flawed) | S4 | F-003: silent text loss; overrun/truncation checks present; unknown ids skipped by size. |
| `read_uint` | 528-541 | REAL | — | `len > 8` → typed bad_structure; zero-length → Some(0) (spec-0 semantics; for TimeStart that means start_ns=0, atom kept). |
| `read_flag` | 543-551 | REAL (flawed) | S4 | F-002: empty ≠ default for the one default-on flag. |
| `chapter_table_too_large` | 553-567 | REAL | — | Typed ResourceLimit constructor, code `demux.mkv.chapter-table-too-large`. |
| `flag_beyond_duration` | 569-579 | REAL | — | Iterates editions→atoms; correct. |
| `flag_atom` | 581-595 | REAL | — | Comparison-only ns math (no scale multiplication → no overflow); recursion depth bounded by parse-time MAX_PARSE_DEPTH guard. |
| `tests::id_bytes` | 604-610 | REAL | — | Vint width from leading zeros; hand-verified for 1/2/4-byte ids. |
| `tests::size_bytes` | 613-624 | REAL | — | 1/2/8-byte size vints; hand-verified. |
| `tests::el` / `chapters_element` / `atom` / `parse` | 627-654 | REAL | — | Fixture builders + parse harness (`.must().must()` on the Option-Result header parse). |
| `editions_atoms_and_displays_parse_the_full_record` | 656-702 | REAL | — | Full record: uid, ordered/default/hidden, times, track numbers, all four display fields, title(), flatten count, zero damage. |
| `default_edition_selects_flag_then_first_visible_then_first` | 704-730 | REAL | — | Three-step selection rule pinned. |
| `nested_atoms_parse_to_the_depth_guard` | 732-757 | REAL | — | MAX_PARSE_DEPTH ok / +1 typed bad_structure. |
| `atom_table_ceiling_is_the_typed_resource_limit` | 759-780 | REAL | — | 65 537 atoms → typed ResourceLimit; kind pinned. |
| `atom_without_time_start_is_damage_skipped` | 782-796 | REAL | — | Degenerate skip + damage row + sibling survival. |
| `edition_ceiling_skips_with_damage` | 798-809 | REAL | — | 65 editions → 64 + damage row. |
| `truncated_atom_field_is_the_typed_truncation` | 811-823 | REAL | — | Declares 8, carries 4 → `demux.mkv.truncated`. |
| `times_past_the_duration_are_the_tolerant_note` | 825-839 | REAL | — | 5 µs duration vs 2..9 µs atom → flagged, kept. |
| `extreme_times_land_in_the_tolerant_bucket` | 841-853 | REAL | — | u64::MAX start: no overflow (comparison only). |
| `unknown_size_inside_chapters_is_bad_structure` | 855-866 | REAL | — | 0xFF size vint → typed refusal. |
| `empty_chapters_element_is_an_empty_list` | 868-874 | REAL | — | Empty master → empty list, zero damage. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 11 unit tests | YES | Concrete asserts (uids, flags, ns values, damage reasons, typed codes, counts). Mutation reasoning: break the depth guard → `nested_atoms…` fails; break the atom ceiling → `atom_table_ceiling…` fails; break degenerate skip → `atom_without_time_start…` fails. |
| Coverage gaps | — | No unit test for: display-over-ceiling damage (part1 lane), track-number ceiling/invalid (F-001's silent path — nothing observable to assert, which is the bug), over-length/invalid-UTF-8 text (F-003), `len > 8` uint refusal. The ffprobe differentials cover happy-path titles/times only. |
| Baseline execution | — | 11/11 ok (test-workspace.log.txt:1764-1774); differentials ok (:2131-2136); mkv_editions_tags 14/14 ok (:2145-2160). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Over-ceiling editions/displays/tracks are skipped with damage" | chapters.rs:17-24 | FALSE for tracks | F-001: `parse_track_numbers` records nothing; REAL for editions (test 798-809) and displays (line 417, part1). |
| "ChapterFlagEnabled (default on)" | chapters.rs:142-143 | SHALLOW | Absent → true (line 376); present-but-empty → false (F-002). |
| "Chapter times are nanoseconds … do not scale with TimestampScale" | chapters.rs:8-16 | REAL | `corpus_attachments_chapters_differential_vs_ffprobe ... ok` (baseline :2133) — the differential this claim cites. |
| "times beyond the segment duration … kept and flagged, never dropped" | chapters.rs:30-33 | REAL | Tests 825-853 pin flagging and retention. |
| "a nesting bomb is the typed bad_structure, never a stack overflow" | chapters.rs:25-29 | REAL | Test 732-757 pins depth-guard refusal. |

## 7. Dead / duplicated / unreachable code
- The `if let Some(value) = ebml::uint_from_be(&bytes)` guard in `parse_track_numbers` (481-484) is dead: `len <= 8` is pre-checked and `uint_from_be` only returns `None` for >8 bytes (ebml.rs:492-495).
- No unreachable-from-shipped code: the whole module is wired through mkv/mod.rs:490 on the `matroska` open path.

## 8. Dependencies & flags
No feature gates (`pub mod chapters` under mkv; demux/src/lib.rs:50). No unsafe. Uses `ebml::{parse_header_at, uint_from_be, utf8_from, MAX_PARSE_DEPTH}`, `header::MAX_TEXT_FIELD` (4 KiB), `error::{truncated, bad_structure}`. `mem_stream` borrowed from `crate::avi::test_support` (test-only, cross-module borrow is intentional per the comment culture).

## 9. Open questions for the next agent
- W05-A05 (part1): does `parse_atom`'s display-over-ceiling lane (414-417) have any unit coverage anywhere? If not, one test would pin it.
- Does the corpus contain zero-length flag elements (would promote F-002 to S3)? `mkv_editions_tags.rs` corpus lanes would show it.
- `ChapterTarget` (243-256, part1) promises scale-tick spans — who consumes it (`MkvDemuxer::seek`)? Verify the ns→tick conversion lives in mod.rs and is tested.
