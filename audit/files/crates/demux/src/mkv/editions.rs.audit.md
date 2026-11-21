# AUDIT: crates/demux/src/mkv/editions.rs (lines 1-389, whole file)
| Field | Value |
|---|---|
| Auditor | W05-A09 |
| Date | 2026-11-21 |
| Lines / functions | 389 / 13 items (consts ×8 + ceilings ×3, error fn, 2 structs + 3 methods, 3 parse fns, 2 tests) |
| File verdict | UNREACHABLE (real, working code wired to nothing in production) |
| Reachable from shipped entry point? | NO — repo-wide grep: `editions::parse_editions`/`EditionList` are consumed ONLY by `crates/demux/tests/mkv_editions_tags.rs` (30, 113, 754). The shipped MKV demuxer parses `Chapters` through **chapters.rs** instead: `mkv/mod.rs:489-493` `collect_chapters` → `chapters::parse_chapters` (also :400, :425), model `ChapterList`/`EditionEntry` with full atoms. No production file references `mkv::editions` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | full Read; repo-wide `rg "editions::parse_editions|mkv::editions"` → tests only; `rg "chapters"` in mkv/mod.rs → the live wiring (489-493); `rg "editions|EditionList"` in mkv/tags.rs → zero hits (the claimed "join key" consumer does not exist); baseline log greps (mkv::editions unit tests ok, test-workspace:1788-1789; mkv_editions_tags suite 14 ok in 0.41 s, :2146-2163); ceiling-policy comparison against chapters.rs (its MAX_EDITIONS=64 + damage-skip vs this module's 1024 + hard refusal); sed -n verification of every quote |

## 1. One-paragraph verdict
This module works — its EBML walk, depth guard, typed budget errors, and selection ladder are implemented and its hand-built tests execute green — but it is a parallel implementation the product abandoned: the shipped MKV demuxer parses Matroska Chapters through chapters.rs (card 5.D2a, a fuller model with atoms, displays, times), and nothing in production calls `editions::parse_editions`; the tags zone the doc names as this module's consumer has zero references to it. The doc's own merge plan ("When D2a's ChapterAtom parse lands, the merge point is parse_editions' atom walk: replace collect_atom_uids") was overtaken by events — D2a landed standalone, the merge never happened, and the two twins have since diverged (edition ceiling 1024-hard-refusal here vs 64-damage-skip in the live one). On top of the dead-wiring, the module's "Memory is bounded" claim is bypassable: the `chapter_uids` push has no ceiling — the guard sits in the nested-atom branch keyed to a UID-count proxy — so a single atom carrying a million ChapterUID children grows the Vec without limit.

## 2. What actually works (verified)
- Element IDs (48-64) verified against the Matroska ID table: Chapters 0x1043A770, EditionEntry 0x45B9, EditionUID 0x45BC, EditionFlagHidden 0x45BD, EditionFlagDefault 0x45DB, EditionFlagOrdered 0x45DD, ChapterAtom 0xB6, ChapterUID 0x73C4 — all correct.
- The walk's progress invariant: every iteration advances `pos = child_end` where `child_end = data_pos + size > pos` (headers are ≥1 byte), and unknown-size children are the typed `bad_structure` refusal (186-190, 228-230, 293-295) — no infinite loop; `end = chapters.declared_end().unwrap_or(bound).min(bound)` bounds the whole parse (178).
- Depth guard (283-287): typed refusal past MAX_ATOM_NESTING=8 — executed by `atom_nesting_depth_guard_is_typed ... ok` (test-workspace:2150) and mirrored in the integration test (mkv_editions_tags.rs:477-500).
- The nested-UID link order: parent's own UID children and nested atoms are pushed in file order (tested: `vec![101, 202, 203]` with 203 nested inside atom2, mkv_editions_tags.rs:450 — ok at :2149-lane).
- Budget errors are typed `ResourceLimit`/`demux.mkv.editions.budget` — in-crate unit test ok (test-workspace:1788); top-level atom ceiling executed by `chapter_bomb_ceiling_is_the_typed_resource_limit ... ok` (:2152).
- Selection ladder `default_edition` (143-149): flag → first non-hidden → first — matches its doc; in-crate test ok (test-workspace:1789) with real mutation coverage (removes the flagged edition, then hides all — asserts each fallback tier).
- Executed evidence (in-crate + hand-built, no corpus gate): `mkv::editions::tests` ×2 ok (:1788-1789); `edition_flags_and_chapter_uid_links_parse`, `edition_defaults_follow_the_spec_when_absent`, `atom_nesting_depth_guard_is_typed`, `chapter_bomb_ceiling_is_the_typed_resource_limit` ok (:2149-2152).

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/mkv/editions.rs:18-26 (doc), wiring disproof at crates/demux/src/mkv/mod.rs:489-493
Code:
```rust
//! the edition's atoms, nested included, in file order) — the join
//! key the tags zone and any edition-aware UI need. When D2a's
//! `ChapterAtom` parse lands, the merge point is
//! [`parse_editions`]' atom walk: replace [`collect_atom_uids`] with
//! the structured parse and keep `Edition`'s UID list as the link
//! seam.
```
```rust
    fn collect_chapters(&mut self, child: &ElementHeader) -> Result<(), EngineError> {
        let more = chapters::parse_chapters(&mut self.reader, child, self.segment_end)?;
```
What is wrong: the module is production-dead and its stated consumer does not exist. (1) The shipped demuxer's `Chapters` handling is `chapters::parse_chapters` (mod.rs:489-493, dispatched at :400/:425) with its own `EditionEntry`/`ChapterList` model — `editions::parse_editions` has zero production callers (repo-wide grep: only `tests/mkv_editions_tags.rs`). (2) "the join key the tags zone … need" is false: `mkv/tags.rs` contains no reference to `EditionList`/`Edition`/`chapter_uids` — the tags zone keeps its own targeting. (3) The doc's merge plan assumed D2a would plug into this walk; D2a instead landed as a full standalone parse (times, displays, tracks) and the merge never happened — the twins have since diverged: this module refuses >1024 editions with a hard `ResourceLimit` error (192-193) while the live chapters.rs skips its 65th edition with a damage note (`MAX_EDITIONS: usize = 64`, chapters.rs:97, 292-293); chapters.rs even documents the kinship ("the same rule `editions::EditionList` applies to its twin", chapters.rs:221) — a rule that now holds only for `default_edition`, not for ceilings or policy.
Why it matters: 389 lines of parallel model + parser + test lane masquerading as "the structure half of the 5.D2 EXTRAS metadata" while the product ships the other half only; a reader (or the next agent) tracing "how does the player get editions" through this doc lands on code that never runs, with error-policy claims (`demux.mkv.editions.budget`) the shipped binary can never emit.
What correct looks like: either delete the module (chapters.rs supersedes it) or actually merge: make chapters.rs's `EditionEntry` carry the UID-link list and route the tags-zone join through it, then update this doc to name the real consumer.
Confidence: HIGH — Verification: repo-wide grep (callers, tags-zone references); read of mod.rs wiring and chapters.rs ceilings; baseline executed lines for both modules' tests.

### F-002 [S3]
Where: crates/demux/src/mkv/editions.rs:297-318 (guard at 310-315, unguarded push at 307)
Code:
```rust
            ID_CHAPTER_ATOM => {
                if edition.chapter_uids.len() >= MAX_CHAPTER_ATOMS {
                    return Err(edition_budget(
                        MAX_CHAPTER_ATOMS as u64,
                        MAX_CHAPTER_ATOMS as u64 + 1,
                    ));
                }
```
What is wrong: the only in-walk ceiling for `chapter_uids` sits in the *nested-atom* branch and keys on `chapter_uids.len()` — a proxy for the atom count. The UID push itself (307) has no ceiling. Two bypasses follow: (a) a single `ChapterAtom` carrying N `ChapterUID` children (spec says one per atom; a hostile file can carry any number) pushes N entries with no check ever firing — the Vec grows to ~8 bytes per ~5 bytes of file (≈2.7× amplification; a 1 GB hostile Chapters element → ~2.7 GB RAM); (b) nested atoms *without* UID children inflate `atoms_total` past MAX_CHAPTER_ATOMS unrefused (the top-level check at 250-254 runs once per top-level atom only) — memory-benign but the documented "atom count carr[ies] the typed ResourceLimit refusal" (34-35) is then false for the count it names. The module doc claims "Memory is bounded (MP §12)". The bomb test (mkv_editions_tags.rs:503-521) uses 65 537 *empty top-level* atoms — the tested lane is exactly the one guard that works; the UID-bomb and nested-count lanes are untested.
Why it matters: the ceiling exists to make the chapter-bomb survivable; keying it to the wrong counter means the bomb shape it exists for has an unguarded variant. Today this is test-reachable only (F-001 — the shipped path uses chapters.rs), but the module's whole stated purpose is the merge that would wire it in.
What correct looks like: check `edition.chapter_uids.len() >= MAX_CHAPTER_ATOMS` at the push site (307) and check `*atoms_total >= MAX_CHAPTER_ATOMS` in the nested-atom branch (the counter it names); add the UID-bomb test.
Confidence: HIGH — Verification: read of both branches + the test body; no execution (module unreachable; mutation reasoning only).

### F-003 [S4]
Where: crates/demux/src/mkv/editions.rs:192-193 (same shape at 250-254 and 310-315)
Code:
```rust
            if list.editions.len() >= MAX_EDITIONS {
                return Err(edition_budget(MAX_EDITIONS as u64, MAX_EDITIONS as u64 + 1));
            }
```
What is wrong: every `edition_budget` call site hard-codes `attempted = budget + 1` — the error's "attempted" field never carries the actual count (e.g. `list.editions.len() + 1`, `*atoms_total + 1`). For a file with 5000 editions the refusal still claims 1025 were attempted.
Why it matters: `attempted` is the diagnostic that tells the user how far the file got; a constant lie misleads every consumer of the typed error.
What correct looks like: pass the real attempted value; the in-crate test (376-388) already takes (budget, attempted) as parameters — extend it to pin a call site's real value.
Confidence: HIGH — Verification: read of all three call sites + the test.

### F-004 [S4]
Where: crates/demux/tests/mkv_editions_tags.rs:776-778 (corpus gate for this module's differential), evidence re-grade for the baseline
Code:
```rust
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP: corpus-v4 absent (LUMEN_CORPUS unset, /tmp/my-project/corpus-v4 missing)");
        return;
    };
```
What is wrong: the module's only real-file evidence (`corpus_editions_match_ffprobe_chapter_counts`, asserting the 0007 chapter-bomb's 2580 UIDs == ffprobe's chapter count and 0063-multi-av's 10) skip-passes when the corpus is absent — and the baseline run's timings prove it was absent: the whole 14-test suite finished in 0.41 s (a real run spawns ffprobe ≥2× and parses 31 mkv lanes in the tags twin), and the sibling corpus suites clocked 0.01 s (corpus_mp4, mkv_corpus — pure skips). The baseline's `corpus_editions_match_ffprobe_chapter_counts ... ok` (test-workspace:2150) is therefore a skip, not execution.
Why it matters: the doc's chapter-bomb claim ("the corpus's own chapter-bomb (0007) carries 2580 atoms", 36-37) is pinned only by a test that did not run in the recorded baseline; the "corpus differential" evidence layer for the whole demux crate has this same skip-pass shape (see my correction note on ts/mod.rs#part1).
What correct looks like: assert `ran > 0` (fail loudly when the corpus is missing in an environment that claims to test it), or mark corpus tests `#[ignore]` with a documented runner so "ok" cannot be a skip.
Confidence: HIGH — Verification: read of the skip path; baseline timings compared across four corpus suites.

### F-005 [S4]
Where: crates/demux/src/mkv/editions.rs:109-112 (field doc) vs 159-161 (`by_uid`)
Code:
```rust
    /// `EditionUID` — the join key for `TagEditionUID` targeting (0
    /// when the file declares none — the spec default; still a valid
    /// edition, just untargetable).
    pub uid: u64,
```
What is wrong: the doc says a uid-0 edition is "untargetable", but `by_uid(0)` returns the first uid-0 edition — a tag carrying `TagEditionUID=0` (the spec's global/no-target form, which this crate's own tags test `uid_zero_targets_are_the_global_form` pins) would spuriously "target" every untargetable edition if the join ever went through this API.
Why it matters: model doc and method contradict each other at exactly the seam (UID join) the module exists to provide; dead today (F-001) but a landmine for the merge.
What correct looks like: `by_uid` returns None for uid==0, or the field doc drops "untargetable".
Confidence: HIGH — Verification: read of both sites + the tags integration test name.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-37 | SHALLOW | S3/S4 | join-key consumer claim FALSE (F-001); bounded-memory claim bypassable (F-002); chapter-bomb corpus claim unexecuted in baseline (F-004); the rest (skip-by-size, depth guard) verified |
| consts `ID_*` ×8 | 47-64 | REAL | — | all IDs match the Matroska spec table |
| ceilings ×3 | 68-81 | REAL | — | 1024/65 536/8; values sane; enforcement gaps are F-002's |
| `edition_budget` | 91-101 | REAL | S4 | constructor fine; call sites fabricate `attempted` (F-003) |
| `Edition` | 107-128 | REAL | S4 | uid-0 doc vs by_uid (F-005) |
| `EditionList` | 130-135 | REAL | — | — |
| `default_edition` | 143-149 | REAL | — | ladder matches doc; hidden+default beats visible non-default — as documented |
| `ordered_editions` | 153-155 | REAL | — | allocates per call; fine for the model's scale |
| `by_uid` | 159-159-161 | REAL | S4 | F-005 |
| `parse_editions` | 173-204 | REAL (unreachable) | — | bounded walk; >1024 editions = hard refusal (vs chapters.rs's damage-skip — F-001 divergence) |
| `parse_edition` | 209-268 | REAL (unreachable) | — | degenerate (atom-less) entries skipped via Ok(None) — others survive; flags/UID parse len-capped at 8 |
| `collect_atom_uids` | 275-325 | DEFECTIVE | S3 | F-002 — the only in-walk ceiling keys on the wrong counter; push site unguarded |
| test `default_edition_selects_flag_then_first_visible` | 333-373 | REAL | — | real mutation coverage of all three selection tiers |
| test `budget_error_is_typed_resource_limit` | 376-388 | REAL | — | pins the constructor only — no call-site value pinned (F-003) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `default_edition_selects_flag_then_first_visible` (333-373) | YES | drives all three tiers by mutation of the list (remove flagged → first visible; hide all → first) |
| `budget_error_is_typed_resource_limit` (376-388) | YES for the constructor, NO for call sites | F-003: fabricated `attempted` values pass |
| `edition_flags_and_chapter_uid_links_parse` (integration) | YES | exact UID vector + flags + nested count on a hand-built fixture |
| `edition_defaults_follow_the_spec_when_absent` (integration) | YES | absent-flags defaults pinned |
| `atom_nesting_depth_guard_is_typed` (integration) | YES | MAX+1 nesting → exact Corrupt kind |
| `chapter_bomb_ceiling_is_the_typed_resource_limit` (integration) | YES for the empty-top-level-atom lane, NO for the UID-bomb lane | F-002: only the working guard's lane is tested |
| `corpus_editions_match_ffprobe_chapter_counts` (integration) | UNVERIFIED | skip-passed in the baseline (F-004) — corpus-v4 absent |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the join key the tags zone and any edition-aware UI need" | editions.rs:19-21 | **FALSE** | tags.rs has zero references to EditionList/Edition/chapter_uids (grep); no production caller at all (F-001) |
| "When D2a's ChapterAtom parse lands, the merge point is parse_editions' atom walk" | editions.rs:21-24 | STALE/FALSE | D2a (chapters.rs) landed standalone; mod.rs:489-493 wires it; the merge never happened (F-001) |
| "Memory is bounded (MP §12): edition count and atom count carry the typed ResourceLimit refusal" | editions.rs:34-35 | **SHALLOW** | F-002: the UID-push path has no ceiling; nested atom count is unrefused past the ceiling |
| "the corpus's own chapter-bomb (0007) carries 2580 atoms in one edition" | editions.rs:36-37 | REAL-UNPROVEN | pinned only by a corpus test that skip-passed in the baseline (F-004); the constant's 25× headroom claim is arithmetic, not execution |
| "Unknown children are skipped by declared size, never fatal" | editions.rs:30 | REAL | `_ => {}` + `pos = child_end` at all three walks (verified) |
| "The atom descent carries an explicit depth counter refused past MAX_ATOM_NESTING" | editions.rs:31-33 | REAL | 283-287; executed by the depth-guard test (ok, test-workspace:2150) |
| "still a valid edition, just untargetable" (uid 0) | editions.rs:110-112 | SHALLOW | by_uid(0) targets it (F-005) |

## 7. Dead / duplicated / unreachable code
- The entire module is production-dead (F-001): `Edition`/`EditionList`/`parse_editions`/`collect_atom_uids` + all consts exist only for the integration test lane. The live twin is chapters.rs (its own `EditionEntry`, `MAX_EDITIONS=64`, damage-skip policy, full atom parse).
- `Edition::atom_count` duplicates information derivable from `chapter_uids.len()` only when atoms carry exactly one UID each — the two counters already disagree on UID-less atoms (F-002's lane (b)).
- The ID consts are private duplicates of chapters.rs's public `ID_CHAPTERS` family (mod.rs imports `chapters::ID_CHAPTERS` at :400) — same values, two sources.

## 8. Dependencies & flags
- `mkv::ebml` (reader/headers — A03's file), `mkv::error` (bad_structure). No feature gates; no unsafe; compiled unconditionally into the lib (pub mod) despite zero production consumers — ~389 lines of dead weight in every shipped binary that includes the demux crate.
- Bounded reads throughout (`read_exact_at(len, pos, child_end)` with `len ≤ 8` on value elements); allocation is the `chapter_uids` Vec (F-002) and the editions Vec (capped at 1024 by refusal).

## 9. Open questions for the next agent
- A04/A05 (chapters.rs part1/part2): does the LIVE chapters parse have the same UID-push bypass (F-002's shape)? Its ceiling `demux.mkv.chapter-table-too-large` fires where — per-atom, per-UID, per-display? That is the shipped path's version of this question and it matters far more than this dead module's.
- W05-A01 (tags.rs): confirm the tags zone keeps its own edition targeting and never needed this "join key" (I verified zero references; your file owns the definitive statement).
- Lead/orchestrator: the corpus-suite skip-pass pattern (F-004, and my correction notes on ts/mod.rs#part1 + mp4/test_support.rs) means no demux corpus differential executed in the recorded baseline. REQUEST-ORCH-RUN (optional, for the editions + chapters + tags lanes): `LUMEN_CORPUS=<corpus-v4 path> cargo test -p demux --test mkv_editions_tags --test mkv_corpus --test ts_corpus --test ts_pes_corpus --test corpus_mp4` — to turn the skip-passed "ok"s into real differential evidence.
