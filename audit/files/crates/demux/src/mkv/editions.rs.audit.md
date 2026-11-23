# AUDIT: crates/demux/src/mkv/editions.rs
| Field | Value |
|---|---|
| Auditor | W05-A04 |
| Date | 2025-07-11 |
| Lines / functions | 389 / 7 (+2 in-file tests) |
| File verdict | UNREACHABLE (real, tested code; no shipped entry point calls it) |
| Reachable from shipped entry point? | NO. The MKV demuxer parses editions through the TWIN module: `MkvDemuxer::collect_chapters` (crates/demux/src/mkv/mod.rs:488-495) calls `chapters::parse_chapters`, never `editions::parse_editions`. Repo-wide rg: callers of `parse_editions`/`EditionList` = crates/demux/tests/mkv_editions_tags.rs only. The CLI host (apps/cli/src/containers/mkv.rs) never references editions; tags.rs collects `TagEditionUID`/`TagChapterUID` (tags.rs:216-218, 470-471) but never joins them via `EditionList::by_uid`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `sed -n` quotes (exit 0); `rg` callers (exit 0); `./target/debug/deps/demux-5e0c7194d741938b editions` → 3 passed, exit 0; `./target/debug/deps/demux-036dd705535b2fb3 edition` → 5 passed, exit 0; `./target/debug/deps/mkv_editions_tags-35df1c0ffd2275fe chapter_bomb` → 1 passed, exit 0 |

## 1. One-paragraph verdict
This module is a complete, careful, honestly-documented editions parser that nothing in the shipped build ever calls. The demuxer's `Chapters` handling goes through the parallel D2a module (chapters.rs), which landed its own `EditionEntry` model, its own default-edition selection rule (byte-identical logic, chapters.rs:226-231), and DIFFERENT ceilings and policies (MAX_EDITIONS 64 with skip-with-damage vs this module's 1024 with a hard typed refusal). The merge this module's docs record as the integration point ("when D2a's ChapterAtom parse lands... replace collect_atom_uids") never happened; the "join key the tags zone needs" (`by_uid`) has zero consumers. Within the dead module the parsing itself is sound — bounds-gated value reads, a depth guard, typed truncations via the EBML seam — except the documented atom-count ceiling is not enforced for nested UID-less atoms (the guard checks the UID-list length instead of the atom counter).

## 2. What actually works (verified)
- `./target/debug/deps/demux-5e0c7194d741938b editions` → `mkv::editions::tests::{default_edition_selects_flag_then_first_visible, budget_error_is_typed_resource_limit}` + chapters twin → 3 passed, exit 0.
- `./target/debug/deps/mkv_editions_tags-35df1c0ffd2275fe chapter_bomb` → `chapter_bomb_ceiling_is_the_typed_resource_limit` → 1 passed, exit 0 (65,537 flat top-level atoms → typed `demux.mkv.editions.budget`).
- `./target/debug/deps/demux-036dd705535b2fb3 edition` → 5 passed incl. `mkv::chapters::tests::edition_ceiling_skips_with_damage` — proving the WIRED twin's opposite policy (skip-with-damage) side by side with this module's hard-refusal tests.

## 3. Findings

F-013 [S3] Whole module unreachable in production; parallel twin already diverged
Where: crates/demux/src/mkv/mod.rs:489-490 (entry), crates/demux/src/mkv/mod.rs:488-495 (the wired path)
Code (mod.rs verbatim):
```rust
    fn collect_chapters(&mut self, child: &ElementHeader) -> Result<(), EngineError> {
        let more = chapters::parse_chapters(&mut self.reader, child, self.segment_end)?;
```
Code (editions.rs:19-21 verbatim):
```rust
//! and the **edition→chapter-UID links** (every `ChapterUID` inside
//! the edition's atoms, nested included, in file order) — the join
//! key the tags zone and any edition-aware UI need. When D2a's
```
What is wrong: `parse_editions`, `EditionList::{default_edition, ordered_editions, by_uid}` and `edition_budget` have no production caller (rg across crates/ and apps/: only tests/mkv_editions_tags.rs and the in-file tests). The demuxer retains editions via chapters.rs's `EditionEntry` (mod.rs:492 `self.chapters.editions.extend(more.editions)`); the tags zone collects `edition_uids`/`chapter_uids` but performs no join. `ordered_editions` and `by_uid` are dead accessors even in tests beyond their own assertions. The module doc sells itself as "the join key the tags zone and any edition-aware UI need" — no such consumer exists. The twins have already diverged: chapters.rs `MAX_EDITIONS = 64` with "over-ceiling editions are skipped with damage, the rest survive" vs editions.rs `MAX_EDITIONS = 1024` with a hard `Err(edition_budget(...))` — the same malformed input yields opposite outcomes depending on which dead-or-alive parse you consult.
Why it matters: 389 loc + a dedicated integration suite (mkv_editions_tags.rs) are maintained and green for a code path the player can never reach; any future consumer wiring `editions::by_uid` would read a model the demuxer does not populate.
What correct looks like: delete this module or complete the recorded merge (chapters.rs keeps the structure, this module's UID-link list becomes a chapters.rs output).
Confidence: HIGH — Verification: read + ran (caller graph by rg; both twins' tests executed).

F-014 [S3] `MAX_CHAPTER_ATOMS` not enforced for nested UID-less atoms; doc claims the atom count carries the typed refusal
Where: crates/demux/src/mkv/editions.rs:34-36 (doc), 309-318 (the nested guard)
Code (verbatim):
```rust
            ID_CHAPTER_ATOM => {
                if edition.chapter_uids.len() >= MAX_CHAPTER_ATOMS {
```
```rust
                *atoms_total += 1;
                edition.atom_count += 1;
                collect_atom_uids(reader, &child, child_end, depth + 1, edition, atoms_total)?;
```
Code (doc, verbatim):
```rust
//! - Memory is bounded (MP §12): edition count and atom count carry
//!   the typed `ResourceLimit` refusal — the corpus's own
```
What is wrong: the nested-atom branch guards on `edition.chapter_uids.len()` (the UID list length), not `*atoms_total`. A crafted atom subtree of UID-less nested `ChapterAtom`s increments `atoms_total` and `atom_count` past 65,536 with no refusal; the counter check exists only in `parse_edition`'s TOP-LEVEL branch (line 250). Memory stays de-facto bounded (UID-less atoms allocate nothing; UID-carrying atoms are capped by this same check at 65,536 uids/edition ≈ 512 KiB, and the walk is end-bounded), but the documented invariant — "atom count carr[y] the typed ResourceLimit refusal" — is false for the nested shape.
Why it matters: the module's stated chapter-bomb guard does not do what the doc says for the nested bomb; the integration test only covers flat top-level atoms (`for _ in 0..=editions::MAX_CHAPTER_ATOMS { atoms.push(el(editions::ID_CHAPTER_ATOM, &[])); }`, mkv_editions_tags.rs:507-508), so the gap is invisible to the suite.
What correct looks like: `if *atoms_total >= MAX_CHAPTER_ATOMS` in the nested branch too.
Confidence: HIGH — Verification: read (both branches sed-verified; bomb test run covers only the top-level path).

F-015 [S4] A child's declared end may exceed the parent `Chapters`/`EditionEntry` bound
Where: crates/demux/src/mkv/editions.rs:186-190, 201 (`pos = child_end`)
Code (verbatim):
```rust
        let Some(child_end) = child.declared_end() else {
            // Unknown-size inside Chapters: not the streaming form's
            // scope.
            return Err(error::bad_structure(child.data_pos));
```
What is wrong: `parse_header_at` bounds only the 12-byte header window to `end`; a child declaring a size past its parent's end yields `child_end > end`. The walk then processes the child (value reads clamp to stream length via `read_exact_at`'s `until.min(self.length)`, typed truncation on shortfall) and `pos = child_end` ends the loop. Tolerant lenience, no crash, no unbounded read — but flag/UID values can be sourced from bytes past the parent's declared extent.
Confidence: MEDIUM — Verification: read (ebml.rs:379-394, 439-451 cross-checked).

F-016 [S4] Dead `unwrap_or(0)` fallbacks
Where: crates/demux/src/mkv/editions.rs:241, 303
Code (verbatim):
```rust
                let value = ebml::uint_from_be(&bytes).unwrap_or(0);
```
What is wrong: after the `len > 8` gate, `uint_from_be` returns `Some` for every remaining input (0..=8 bytes; empty → `Some(0)` per ebml.rs:492-501) — the fallback can never fire. Harmless, but it papers over the impossible case instead of asserting the invariant.
Confidence: HIGH — Verification: read (ebml.rs:492-501 sed-checked).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| edition_budget | 91-101 | REAL | — | typed constructor; shape pinned by test |
| EditionList::default_edition | 143-149 | REAL | — | correct flag→first-visible→first rule; duplicated byte-for-byte in chapters.rs:226-231 |
| EditionList::ordered_editions | 153-155 | REAL-UNPROVEN | S3 | dead accessor (F-013) |
| EditionList::by_uid | 159-161 | REAL-UNPROVEN | S3 | the "tags-zone join" — zero consumers (F-013) |
| parse_editions | 173-204 | REAL | S3 | sound loop; edition ceiling enforced; unreachable (F-013) |
| parse_edition | 209-268 | REAL | S3 | flags/UID bounds-gated (len>8); top-level atom ceiling OK; degenerate editions silently skipped (documented) |
| collect_atom_uids | 275-325 | REAL | S3 | depth guard correct (tested); nested ceiling gap (F-014) |
| tests: default_edition_selects… / budget_error_is_typed… | 332-388 | honest / partial | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| default_edition_selects_flag_then_first_visible (332-373) | YES | real three-edition selection ladder incl. fallbacks; executed |
| budget_error_is_typed_resource_limit (375-388) | NO (for enforcement) | constructs the error and asserts its fields — a constructor-shape test; the actual ceiling firing is only proven by the integration test (top-level flat atoms, so F-014's nested gap stays invisible) |
| mkv_editions_tags.rs: atom_nesting_depth_guard_is_typed (477-501) | YES | MAX_ATOM_NESTING+1 wrap → typed Corrupt |
| mkv_editions_tags.rs: chapter_bomb_ceiling_is_the_typed_resource_limit (503-520) | PARTIAL | proves the top-level refusal; does not cover nested UID-less atoms (F-014) |
| mkv_editions_tags.rs: corpus differentials (0007-chapter-bomb etc.) | YES | ffprobe-pinned corpus files through the (dead) module |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the join key the tags zone and any edition-aware UI need" | editions.rs:19-21 | FALSE (no consumer) | rg: by_uid callers = none; tags.rs:216-218 collects UIDs but never joins |
| "Memory is bounded (MP §12): edition count and atom count carry the typed ResourceLimit refusal" | editions.rs:34-36 | PARTLY FALSE | atom-count refusal misses nested UID-less atoms (F-014); memory de-facto bounded |
| "Unknown children are skipped by declared size, never fatal" | editions.rs:30 | TRUE | `_ => {}` + `pos = child_end` at 263/321 |
| "# Errors … ResourceLimit when an edition/atom ceiling is exceeded" | editions.rs:170-172 | PARTLY FALSE | same nested gap |
| "When D2a's ChapterAtom parse lands, the merge point is parse_editions' atom walk" | editions.rs:22-24 | STALE | D2a (chapters.rs) landed with its own EditionEntry model; the merge never happened — this module is now the dead twin |
| feature-matrix.toml:138-139 "5.D2b editions/tags … wired through open() by issue #34 (Tags/Attachments/Chapters retained)" | docs/feature-matrix.toml | AMBIGUOUS-BUT-SURVIVABLE | the concrete retention list names Tags/Attachments/Chapters — all wired via chapters.rs/mod.rs; "editions" is only card naming + a directory-glob evidence path; the editions.rs half of that evidence is unwired |

## 7. Dead / duplicated / unreachable code
The entire module (F-013): unreachable from lumen-cli/desktop; duplicated edition model + selection rule vs chapters.rs; `ordered_editions`/`by_uid` dead even within the crate; divergent ceilings (1024/hard-Err vs 64/skip-with-damage).

## 8. Dependencies & flags
No feature gates. Depends on mkv::ebml (sound seam: `parse_header_at` bound-aware, `read_exact_at` typed truncation) and mkv::error. In-file local `edition_budget` instead of error.rs (the file-disjoint rationale is documented at 83-87).

## 9. Open questions for the next agent
- chapters.rs (the WIRED twin, another agent's file): does its atom walk enforce a total-atom ceiling for nested atoms, or does it share F-014's gap? Its doc says "a total atom …" (line 21) — verify.
- If any Wave-6+ card plans the tags→editions join or an edition-aware UI, it must wire THIS module or move its UID links into chapters.rs — the current state satisfies neither.

---
CORRECTION (orchestrator gate repair): one citation attributed mkv/mod.rs:489 code
(collect_chapters) to editions.rs:173. Where: field corrected; quote verified
against mkv/mod.rs:489 via sed. Logged in audit/verify/fabrications.md.
