# AUDIT: crates/demux/src/mkv/chapters.rs
| Field | Value |
|---|---|
| Auditor | W05-A08 |
| Date | 2025-11-23 |
| Lines / functions | 875 / 15 production fns + 6 test helpers + 11 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (parse path): lumen-cli `containers/mod.rs:57 "matroska" => mkv::open` → `containers/mkv.rs:147-150 MkvDemuxer::new` → `start()` → `open()` walk (`mkv/mod.rs:340,425,400`) → `collect_chapters` (`mkv/mod.rs:489`) → `parse_chapters` (`chapters.rs:270`); `flag_beyond_duration` called at open (`mkv/mod.rs:461`). Getters `chapters()`/`chapter_targets()`/`atoms_flattened()` have NO shipped consumer (tests only — matches W04-A08 finding). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 4 |
| Commands run | `sed -n` quotes (exit 0, all verified); `./target/debug/deps/demux-036dd705535b2fb3 mkv::chapters` → 11 passed / 0 failed (exit 0, 0.07s); `rg` caller/ceiling searches (exit 0); `sed -n '159p' \| grep -c must_use` → 1 (exit 0) |

## 1. One-paragraph verdict
This is a real, working EBML chapters parser with correct child-extent bounding (`child_end > end` → typed truncation at every level), a correct depth guard (16, off-by-none, pinned by test), a correct shared per-call atom budget, and a strong unit suite that I ran (11/11 pass). Its one structural defect: every memory ceiling it advertises is per-`Chapters`-master, and the demuxer merges an unbounded number of masters at open, so the module's "never an unbounded allocation" promise is false at the seam (F-001). Secondary defects are doc-vs-code lies about damage rows for over-ceiling track numbers (F-002) and stale/imprecise doc comments; three of the four ceilings have zero negative-test coverage.

## 2. What actually works (verified)
— verified by `./target/debug/deps/demux-036dd705535b2fb3 mkv::chapters` -> `test result: ok. 11 passed; 0 failed; 0 ignored` (exit 0). Depth guard: `nested_atoms_parse_to_the_depth_guard` (16 ok / 17 refuses, asserted code `demux.mkv.bad-structure`). Atom ceiling: 65_537 atoms → `demux.mkv.chapter-table-too-large` + `ErrorKind::ResourceLimit`. Edition ceiling, degenerate-atom damage, contained-lie truncation, ns-past-duration flagging, u64::MAX times — all asserted. Baseline confirms integration coverage: `corpus_attachments_chapters_differential_vs_ffprobe ... ok` (audit/runs/baseline/cargo-test-workspace.txt:2306) — ffprobe differential ran in the orchestrator env, not re-runnable here (corpus absent from sandbox, UNVERIFIED locally).

## 3. Findings
F-001 [S2]
Where: crates/demux/src/mkv/chapters.rs:99-104 (+ crates/demux/src/mkv/mod.rs:489-495)
Code:
```
/// Maximum chapter atoms trusted into the whole list (all editions,
/// nested atoms included)
/// `[PROPOSAL: 65_536 — the corpus chapter bomb carries 2_580; a
/// table past this is the typed ResourceLimit refusal, the cue-table
/// precedent]`.
pub const MAX_CHAPTER_ATOMS: usize = 65_536;
```
and (mkv/mod.rs:489-495):
```
    fn collect_chapters(&mut self, child: &ElementHeader) -> Result<(), EngineError> {
        let more = chapters::parse_chapters(&mut self.reader, child, self.segment_end)?;
        self.damage_total += more.damage.iter().map(|(n, _)| *n).sum::<u64>();
        self.chapters.editions.extend(more.editions);
        self.chapters.damage.extend(more.damage);
        Ok(())
    }
```
What is wrong: `MAX_EDITIONS` (64) and `MAX_CHAPTER_ATOMS` (65_536) are enforced per `parse_chapters` call; `atoms_total` is a local reset per call. The open() walk (`mkv/mod.rs:377-432`) calls `collect_chapters` for **every** top-level `Chapters` child (known-size at :425, unknown-size at :400) and extends the retained list with no aggregate ceiling. K crafted masters → K×64 editions, K×65_536 atoms. Why it matters: the module doc promises "never an unbounded allocation" (line 24) and cites MP §12; the merge seam defeats it. A minimal atom costs ~5 bytes on disk (2 hdr + 1+1+1 TimeStart) and ~112+ B of `ChapterAtom`+Vecs in RAM → ~20× amplification; a 100 MB crafted MKV parks ~2 GB of chapter atoms at open. What correct looks like: carry `atoms_total`/edition count on `MkvDemuxer` across `collect_chapters` calls and type-refuse past the ceiling (the cue-table precedent shape), or document the ceilings as per-master. Confidence: HIGH — Verification: read + call-chain traced; no runtime repro (would need a crafted multi-master file; corpus absent).
F-002 [S3]
Where: crates/demux/src/mkv/chapters.rs:20-22 vs 466,479-485
Code:
```
//!   ([`MAX_EDITIONS`], [`MAX_CHAPTER_ATOMS`],
//!   [`MAX_CHAPTER_DISPLAYS`], [`MAX_CHAPTER_TRACKS`]). Over-ceiling
//!   editions/displays/tracks are skipped with damage; a total atom
```
vs `parse_track_numbers` (no damage parameter exists):
```
        if child.id == ID_CHAPTER_TRACK_NUMBER && taken < MAX_CHAPTER_TRACKS && len <= 8 {
            let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
            if let Some(value) = ebml::uint_from_be(&bytes) {
                parsed.track_numbers.push(value);
                taken += 1;
            }
        }
```
What is wrong: the module doc claims over-ceiling **tracks** are skipped "with damage"; `parse_track_numbers` has no damage plumbing at all — the 65th+ `ChapterTrackNumber` (and any with `len > 8` or unparseable value) is dropped silently. Editions (:292-293) and displays (:414-417) do push damage rows. Why it matters: the damage list is the crate's honesty surface (`damage_total` is surfaced); the doc asserts a behavior the code does not have — FALSE claim, and silent data loss in the tolerant path. What correct looks like: thread `&mut ChapterDamage` into `parse_track_numbers` and push a "track-over-ceiling" row, or fix the doc. Confidence: HIGH — Verification: read + rg (zero other damage sites).
F-003 [S4]
Where: crates/demux/src/mkv/chapters.rs:191-192
Code:
```
/// The parsed `Chapters` element (+ every `Chapters` element found by
/// the demuxer getter, merged).
```
What is wrong: the getter `MkvDemuxer::chapters` (`mkv/mod.rs:240-242`) returns `&self.chapters` — it finds and merges nothing. Merging happens in `collect_chapters` during the open walk, and the module doc itself (:40-42) says the on-demand getter re-scan "is gone" (issue #34). This struct doc describes the removed design. Why it matters: internal contradiction between two doc comments in the same file; a reader wiring the getter would believe it re-scans. What correct looks like: "+ every `Chapters` element the open walk found, merged by `collect_chapters`". Confidence: HIGH — Verification: read of both sites.
F-004 [S4]
Where: crates/demux/src/mkv/chapters.rs:264-269 vs 513
Code:
```
/// # Errors
/// [`error::truncated`] when an inner element's declared extent
/// overruns its parent (the contained lie) or the stream physically
/// ends inside it; [`error::bad_structure`] for unknown-size inner
```
vs `parse_display`:
```
            let bytes = reader.read_prefix_at(len, child.data_pos, child_end)?;
            let text = ebml::utf8_from(&bytes);
```
What is wrong: `read_prefix_at` never errors on a short read (`ebml.rs:326-328`: "Never errors on a short read"), so a stream physically ending mid-`ChapString` yields a silently truncated title (or absent on broken UTF-8), not `error::truncated`. The # Errors claim holds only for the fixed-width reads (`read_uint`→`read_exact_at`). Why it matters: the documented truncation contract is wrong for the display-text path; a truncated file can present a cut-off chapter title as healthy. What correct looks like: scope the # Errors claim to fixed-width fields, or classify short text reads as damage rows. Confidence: HIGH — Verification: read of `ebml.rs:326-352` (`read_prefix_at`) and :379-395 (`read_exact_at`).
F-005 [S4]
Where: crates/demux/src/mkv/chapters.rs:157-162
Code:
```
    /// The first display's title (the UI default; `None` when the
    /// atom has no titled display).
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.displays.iter().find_map(|d| d.string.as_deref())
    }
```
What is wrong: "The first display's title" — the code returns the first display **that has** a title (`find_map`), not `displays[0].string`. With `displays[0].string = None`, `displays[1].string = Some("X")`, it returns `Some("X")`. Why it matters: the main clause lies about which display wins; the parenthetical half-fixes it. What correct looks like: "the first titled display's string". Confidence: HIGH — Verification: read; `#[must_use]` byte-verified via `grep -c must_use` (1) + Read (sed output mangles the literal `[m` sequence — tool artifact, noted in log).
F-006 [S3]
Where: crates/demux/src/mkv/chapters.rs:413-422 (display ceiling), 479 (track ceiling), mod.rs:490 (merge)
Code:
```
            ID_CHAPTER_DISPLAY => {
                if parsed.displays.len() >= MAX_CHAPTER_DISPLAYS {
                    // Over-ceiling displays: skipped with damage (a
                    // display bomb cannot become an unbounded list).
                    damage.push((child_end - pos, "display-over-ceiling"));
```
What is wrong: zero test coverage for the display-over-ceiling path and the track-number ceiling: `rg 'display-over-ceiling' crates/demux/` matches only chapters.rs itself (doc + code); `rg 'ID_CHAPTER_TRACK|track_numbers' crates/demux/tests/` matches nothing. The multi-master merge (F-001) is also untested — no test builds two `Chapters` masters. The 11 unit tests cover only 2 of the 4 ceilings (editions, atom table). Why it matters: the module's core selling point is bomb tolerance; half its bomb guards could be deleted and no test would fail. What correct looks like: one negative test per ceiling + one two-master merge test pinning the aggregate behavior. Confidence: HIGH — Verification: rg across crates/demux (unit + integration tests).
F-007 [S4]
Where: crates/demux/src/mkv/chapters.rs:600,604-632
Code:
```
    use crate::avi::test_support::mem_stream;
```
What is wrong: mkv unit tests import the **avi** module's test stream (cross-format layering wrinkle), and `id_bytes`/`size_bytes`/`el` (604-632) are byte-for-byte re-implementations of the integration helpers in `tests/mkv_extras_wiring.rs:40-66`. Forced by Rust's test-module isolation, but now three EBML-builder copies exist in this crate's test surface (chapters.rs, mkv_extras_wiring.rs, and editions/tags suites per their own helper blocks). What correct looks like: a `#[cfg(test)]` shared `mkv::test_bytes` module or a dev-dependency test-util crate. Confidence: HIGH — Verification: read of both helper blocks.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ChapterAtom::title` | 160-162 | REAL | S4 | F-005 doc imprecision |
| `ChapterAtom::flatten_into` | 165-170 | REAL | — | recursion depth bounded at parse (≤16); unbounded only for hand-built structs |
| `ChapterList::atoms_flattened` | 209-217 | REAL | — | no shipped caller (tests + `chapter_targets`-adjacent suites only) |
| `ChapterList::default_edition` | 225-231 | REAL | — | rule default→first-visible→first, test-pinned |
| `ChapterList::flag_beyond_duration` | 237-240 | REAL | — | called at open (mod.rs:461) |
| `parse_chapters` | 270-304 | REAL | S2 | per-call ceilings only (F-001); unknown-size clamp to `bound` correct |
| `parse_edition` | 308-363 | REAL | — | atom budget shared across editions within one call; over-ceiling editions damage-skipped |
| `parse_atom` | 367-457 | REAL | — | depth guard `depth+1 > MAX_PARSE_DEPTH` correct; degenerate (no TimeStart) → Ok(None), subtree dropped with damage |
| `parse_track_numbers` | 460-489 | REAL | S3 | F-002 silent over-ceiling/len>8 skips, no damage |
| `parse_display` | 492-526 | REAL | S4 | F-004 prefix-read tolerance vs # Errors doc |
| `read_uint` | 530-541 | REAL | — | len>8 → bad_structure; empty → Some(0) per `uint_from_be` (ebml.rs:492-501) |
| `read_flag` | 544-551 | REAL | — | nonzero=set; zero-length reads 0 |
| `chapter_table_too_large` | 557-567 | REAL | — | typed ResourceLimit(Memory), test-pinned |
| `flag_beyond_duration` (free) | 571-579 | REAL | — | plain ns compare, no scale math |
| `flag_atom` | 582-595 | REAL | — | recursive over children, no overflow (no arithmetic) |
| test helpers `id_bytes`/`size_bytes`/`el`/`chapters_element`/`atom`/`parse` | 604-654 | REAL | S4 | F-007 duplication + avi import |

## 5. Test honesty (11 tests, all run by me)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| editions_atoms_and_displays_parse_the_full_record | YES | asserts every field (uid, flags, ns times, track number, all 4 display strings, title, enabled, damage-empty, flatten count) |
| default_edition_selects_flag_then_first_visible_then_first | YES | 3-way fallback asserted through removals |
| nested_atoms_parse_to_the_depth_guard | YES | 16 parses / 17 refuses with exact error code |
| atom_table_ceiling_is_the_typed_resource_limit | YES | 65_537 atoms → exact code + ErrorKind match |
| atom_without_time_start_is_damage_skipped | YES | degenerate skipped + damage reason asserted, sibling survives |
| edition_ceiling_skips_with_damage | YES | 65 editions → 64 kept + 1 damage row |
| truncated_atom_field_is_the_typed_truncation | YES | contained-lie → `demux.mkv.truncated` |
| times_past_the_duration_are_the_tolerant_note | YES | 5_000 ns boundary: 0..500 in, 2_000..9_000 flagged |
| extreme_times_land_in_the_tolerant_bucket | YES | u64::MAX start vs u64::MAX-1 duration → flagged, no overflow |
| unknown_size_inside_chapters_is_bad_structure | YES | 0xFF size inner → exact code |
| empty_chapters_element_is_an_empty_list | WEAK | empty-in/empty-out — appropriate for the degenerate case |
Coverage gaps: display-over-ceiling, track ceiling, multi-master merge — see F-006.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Chapter times are nanoseconds … do not scale with TimestampScale" | chapters.rs:8-16 | REAL-UNPROVEN | consistent with Matroska spec + ffmpeg behavior; ffprobe cross-check ran only in orchestrator env (baseline cargo-test-workspace.txt:2306); corpus absent here |
| "Over-ceiling editions/displays/tracks are skipped with damage" | chapters.rs:20-22 | FALSE (for tracks) | F-002: parse_track_numbers has no damage path |
| "never an unbounded allocation" (atom ceiling) | chapters.rs:23-24 | FALSE at the seam | F-001: collect_chapters merges unbounded masters |
| "a nesting bomb is the typed bad_structure, never a stack overflow" | chapters.rs:25-29 | REAL | depth-guard test run by me (11/11) |
| "MkvDemuxer::chapters is the typed getter over the list the demuxer retains at start" | chapters.rs:39-42 | REAL | mod.rs:240-242 + collect_chapters at open |
| "every Chapters element found by the demuxer getter, merged" | chapters.rs:191-192 | FALSE (stale) | F-003: getter merges nothing; walk does |
| `ChapterTimeStart` "mandatory per spec" → degenerate skip | chapters.rs:34-35, 449-453 | REAL | Matroska spec mandates it; test pins skip+damage |

## 7. Dead / duplicated / unreachable code
No dead production code in this file. `atoms_flattened`/`default_edition`/`ChapterTarget` are test-reachable only (`chapter_targets` has zero shipped callers — W04-A08 already flagged; the mod.rs doc itself admits "nothing downstream reads it today"). Duplication: test EBML builders (F-007).

## 8. Dependencies & flags
No feature gates; std-only crate (demux lib.rs house rules). Test-only deps: `crate::avi::test_support::mem_stream`, `testkit::Must`. No `#[allow]`, no `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!`/`unsafe` in the file (rg, zero matches — house rule upheld).

## 9. Open questions for the next agent
- mkv/mod.rs owner: F-001 aggregate-ceiling fix belongs at `collect_chapters` (mod.rs:489-495); also confirm whether the same per-master-vs-merged escape exists for `tags::parse_tags`/`attachments::parse_attachments` (collect_tags/collect_attachments use the identical extend pattern).
- editions.rs owner: the `editions::EditionList` twin carries a `default_edition` with the same rule — check its ceilings for the same merge escape.
- Whoever owns the corpus: re-run `corpus_attachments_chapters_differential_vs_ffprobe` with LUMEN_CORPUS set (sandbox lacks the corpus; my ffprobe verification is UNVERIFIED).
