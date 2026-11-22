# AUDIT: crates/demux/src/mkv/editions.rs

| Field | Value |
|---|---|
| Auditor | W05-A02 |
| Date | 2026-11-22 |
| Lines / functions | 389 / 11 (7 items incl. 3 parse fns, 3 model methods, 1 error ctor, 2 tests) |
| File verdict | UNREACHABLE — real, non-stub implementation that no shipped entry point calls |
| Reachable from shipped entry point? | NO. Proof: repo-wide rg for `mkv::editions\|parse_editions\|EditionList` matches only: editions.rs itself, a comment in chapters.rs, and the integration test crates/demux/tests/mkv_editions_tags.rs:30,113,754. The demuxer's own chapters path is `MkvDemuxer::collect_chapters` → `chapters::parse_chapters` (mkv/mod.rs:489-495); `pub mod editions;` (mkv/mod.rs:44) is a declared-but-unconsumed export. No app crate (apps/cli, apps/desktop, apps/desktop-tauri, apps/web) references it. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 2 |
| Commands run | rg: consumers of `parse_editions`/`EditionList`/`editions::` (repo-wide, apps+crates), `editions` in mkv/mod.rs + chapters.rs + tags.rs; sed quote verification of editions.rs:186-204, 248-262, 296-318 and chapters.rs:270-301, 283-289; read of ebml.rs read_exact_at/parse_header_at/declared_end/uint_from_be. No cargo (adaptation) → no execution; per-function runtime claims UNVERIFIED, structure verified by reading. |

## 1. One-paragraph verdict

This module parses Matroska `Chapters` → editions (flags, UID, chapter-UID links) with correct EBML bounds discipline (all reads via `ebml::parse_header_at`/`read_exact_at`, which clamp to parent bound and stream length) — and nothing ships it. The live demuxer parses editions through a *second, parallel* implementation in chapters.rs, which re-declares the same element IDs with different ceilings (64 vs 1024 editions) and different damage policies. The module header still describes this file as "the join key the tags zone and any edition-aware UI need" and as a temporary seam awaiting card 5.D2a's merge — both stale: tags.rs never queries `EditionList`, and the D2a parse landed in chapters.rs without the recorded merge happening. Real code, honest internals, orphaned by the wiring: a card that produced a deliverable nobody consumed.

## 2. What actually works (verified)

- Verified statically: EBML reads are bounds-checked — `read_exact_at` refuses `at+want > until.min(stream_len)` with typed truncation (ebml.rs:379-394); `parse_header_at` bounds the header window to the parent end (ebml.rs:439-451). No OOB read path exists from this file.
- Verified statically: ceilings exist at three sites (editions :192, atoms :250/:310), depth guard at :283, unknown children skipped by declared size (:199-201, :263, :320).
- Integration coverage exists: crates/demux/tests/mkv_editions_tags.rs builds hand-made EBML fixtures (nested atoms, all four flags, UIDs) and runs them through `parse_editions` (:109-113, :411-437, :754). Suite-level pass status: W00 baseline (workspace 1751 pass / 2 fail UI-golden / 25 ignored); per-test status UNVERIFIED (no cargo here).

## 3. Findings

### F-001 [S2] Module is dead code in the shipped product while its docs claim it is the tags/UI join seam
Where: crates/demux/src/mkv/editions.rs:19-26 and 6
Code:
```rust
//! the `Tags` zone targets ([`crate::mkv::tags`]) plus the three flags —
...
//! [`parse_editions`]' atom walk: replace [`collect_atom_uids`] with
//! the structured parse and keep `Edition`'s UID list as the link
//! seam. Until then the atoms' non-UID content is skipped by
```
What is wrong: The header sells this module as the edition→tags join key and as a temporary seam pending the "parallel card 5.D2a" (`chapters.rs`) atom parse. Reality: (1) `tags.rs` contains zero references to `editions::`/`EditionList` (rg; its `chapter_uids` at tags.rs:218 is its own parsed field); (2) the D2a parse landed in chapters.rs and the demuxer wired *that* (`collect_chapters` → `chapters::parse_chapters`, mkv/mod.rs:489-495) — the recorded merge point ("replace `collect_atom_uids`") never happened; (3) the only callers of `parse_editions` are the integration test and this file's unit tests.
Why it matters: 389 LOC of maintained, ceiling-guarded parser code that no user can reach, plus a module doc that misstates the system's wiring — exactly the pattern (card delivered, wired to nothing, doc claiming a role) this audit exists to expose. Anyone reading the header believes tags targeting depends on this module; it does not.
What correct looks like: Either wire `EditionList` into the demuxer's chapters surface (one type, one walk) or delete this module and move its two model tests to chapters.rs; fix the header to state the actual consumer set (integration test only).
Confidence: HIGH — Verification: read + repo-wide rg (commands above).

### F-002 [S3] The documented atom-count ceiling is not enforced for nested atoms or multi-UID atoms
Where: crates/demux/src/mkv/editions.rs:307-315 (guard) vs 249-256 (top-level guard)
Code:
```rust
                edition.chapter_uids.push(uid);
            }
            ID_CHAPTER_ATOM => {
                if edition.chapter_uids.len() >= MAX_CHAPTER_ATOMS {
                    return Err(edition_budget(
```
What is wrong: The top-level atom arm guards on `*atoms_total >= MAX_CHAPTER_ATOMS` (:250); the nested arm guards on `edition.chapter_uids.len()` (:310) — a different counter; and the `ChapterUID` push itself (:307) has no guard at all. Consequences: (a) a nesting bomb of UID-less atoms increments `atoms_total` past `MAX_CHAPTER_ATOMS` without ever tripping the nested guard (the walk stays linear-time, memory flat — no allocation per atom); (b) one atom carrying k `ChapterUID` children grows `chapter_uids` by k with no ceiling — allocation is bounded only by file size (~4 file bytes per 8-byte push, ≤2× amplification). The module doc's contract "edition count and atom count carry the typed `ResourceLimit` refusal" (:34-36) is therefore only true for the top-level atom path.
Why it matters: The stated MP §12 memory guarantee is enforced against the wrong counter; a spec-violating file (multiple UIDs per atom) gets a different bound than the documented one. Not S1: allocation remains linear in input size with small constant — no unbounded `with_capacity`, no amplification bomb.
What correct looks like: Guard one counter (`atoms_total` for atoms, a separate `MAX_CHAPTER_UIDS` for UID pushes, or push-then-check both), mirroring chapters.rs's single `atoms_total` discipline (chapters.rs:345-347).
Confidence: HIGH — Verification: read (mutation not executed — UNVERIFIED by run).

### F-003 [S3] No child-overruns-parent refusal: an EditionEntry/atom may declare a size past its master and swallow following bytes
Where: crates/demux/src/mkv/editions.rs:186-190 (same pattern :228-230, :293-295)
Code:
```rust
        let Some(child_end) = child.declared_end() else {
            // Unknown-size inside Chapters: not the streaming form's
            // scope.
            return Err(error::bad_structure(child.data_pos));
        };
        if child.id == ID_EDITION_ENTRY {
```
What is wrong: `child_end` is never compared against the enclosing `end` in any of the three walks. A child declaring `size` beyond its master's end is accepted; if it is an `EditionEntry`/`ChapterAtom`, `parse_edition`/`collect_atom_uids` then walk to `child_end`, parsing whatever follows the `Chapters` element (Tags, Cluster bytes) as edition/atom content. The live twin refuses this shape typed: `if child_end > end { return Err(error::truncated(child_end, end)); }` (chapters.rs:286-289, repeated at :324-325, :388-389). Reads remain clamped by stream length in `ebml::read_exact_at` (ebml.rs:385-388), so there is no OOB — the failure mode is garbage metadata parsed from unrelated bytes, or a typed truncation when the walk runs off the physical stream.
Why it matters: Divergent malformed-input policy from the module that actually ships; if this file is ever wired in per F-001's merge plan, it imports the laxer discipline silently.
What correct looks like: The chapters.rs three-line check at each walk level.
Confidence: HIGH — Verification: read (both files, lines cited).

### F-004 [S3] Parallel duplicate of chapters.rs: same element IDs, different ceilings, different damage policies — two sources of truth for one spec concept
Where: crates/demux/src/mkv/editions.rs:50-81 vs crates/demux/src/mkv/chapters.rs:57-69, 94-104
Code:
```rust
pub const ID_EDITION_ENTRY: u32 = 0x45B9;
...
pub const MAX_EDITIONS: usize = 1024;
```
vs chapters.rs:
```rust
pub const ID_EDITION_ENTRY: u32 = 0x45B9;
...
pub const MAX_EDITIONS: usize = 64;
```
What is wrong: chapters.rs re-declares all seven element IDs with identical values, plus its own `MAX_EDITIONS` (64) vs this file's 1024, its own `MAX_CHAPTER_ATOMS` (65_536 — same value, separate constant), a different nesting policy (`ebml::MAX_PARSE_DEPTH` = 16 at ebml.rs:46 vs `MAX_ATOM_NESTING` = 8 here), a different over-ceiling policy (damage-skip + survive, chapters.rs:292-294, vs typed fatal refusal here, :192-193), and a re-implemented default-edition selection (chapters.rs:226-230 duplicates `EditionList::default_edition` :143-148 — acknowledged in a comment as "the same rule"). A file with 100 editions is "over ceiling" in the shipped path and fine here.
Why it matters: Two implementations of the same parse in one crate guarantee drift (already diverged on four axes); the dead one is the one with the weaker malformed-input discipline (F-003).
What correct looks like: One canonical edition/atom parse module; the other deleted or reduced to type definitions.
Confidence: HIGH — Verification: read + rg of both files.

### F-005 [S4] In-file tests cover only the pure model and an error constructor — the 150-LOC parse core has zero in-file tests
Where: crates/demux/src/mkv/editions.rs:327-389
Code:
```rust
    #[test]
    fn budget_error_is_typed_resource_limit() {
        let err = edition_budget(100, 200);
```
What is wrong: `default_edition_selects_flag_then_first_visible` (:333-373) exercises `Vec` selection logic only; `budget_error_is_typed_resource_limit` (:376-388) asserts a struct constructor builds the struct it builds. Neither would fail under any regression in `parse_editions`/`parse_edition`/`collect_atom_uids` (:173-325) — no in-file test feeds a single EBML byte. The shapes F-002/F-003 describe (overrunning child, multi-UID atom) are pinned nowhere in this file.
Why it matters: The module's own test module documents its robustness model (depth guard, ceilings, skip-by-size) without testing any of it; the real coverage lives one directory away in mkv_editions_tags.rs (which does drive `parse_editions` on hand-built fixtures, :109-113, :411-437).
What correct looks like: Move at least the three malformed-shape pins (overrun child, ceiling overflow, depth bomb) in-file where the code that refuses them lives.
Confidence: HIGH — Verification: read; integration coverage confirmed by rg of mkv_editions_tags.rs.

### F-006 [S4] `edition_budget` reports a fabricated `attempted` value
Where: crates/demux/src/mkv/editions.rs:191-193 (call sites :192-193, :251-254, :311-314)
Code:
```rust
            if list.editions.len() >= MAX_EDITIONS {
                return Err(edition_budget(MAX_EDITIONS as u64, MAX_EDITIONS as u64 + 1));
```
What is wrong: The `ResourceLimit { budget, attempted }` payload hard-codes `attempted = MAX + 1` regardless of the real count — a file declaring 5,000 editions reports "attempted 1025". The typed error carries a made-up number where the true one is available at the call site (`list.editions.len() + 1`, or the declared count).
Why it matters: Diagnostics lie; the taxonomy's whole point (typed, context-carrying errors) is undercut at the payload level.
What correct looks like: Pass the actual attempted count.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `edition_budget` | 91-101 | REAL-UNPROVEN | S4 | Error ctor; F-006 fabricated `attempted`. No panics. Reachable only via parse fns (dead). |
| `struct Edition` | 107-128 | REAL | — | Plain model; doc on `uid: 0` default is honest. |
| `EditionList::default_edition` | 143-149 | REAL | — | Flag → first visible → first. Duplicated in chapters.rs:226-230 (F-004). O(editions). |
| `EditionList::ordered_editions` | 153-155 | REAL | — | Filter+collect; UNREACHABLE in product (no caller outside tests). |
| `EditionList::by_uid` | 159-161 | REAL | — | Linear find; fine at ≤1024. |
| `parse_editions` | 173-204 | REAL-UNPROVEN, UNREACHABLE | S3 | Walks Chapters children; F-003 no overrun check; unknown-size child → typed `bad_structure`; unknown children skipped by size. No panics; all reads via bounded ebml helpers. |
| `parse_edition` | 209-268 | REAL-UNPROVEN, UNREACHABLE | S3 | F-003 (no overrun check at :228-230); `len > 8` guard before uint reads (:237); degenerate entry → `Ok(None)` skip (documented). `usize::try_from` on size — typed refusal, no panic. |
| `collect_atom_uids` | 275-325 | REAL-UNPROVEN, UNREACHABLE | S3 | Depth guard :283 (`bad_structure` past 8); F-002 wrong-counter guard :310; F-003 no overrun check :293-295. Recursion bounded by MAX_ATOM_NESTING → no stack overflow. |
| `tests::default_edition_selects_flag_then_first_visible` | 333-373 | REAL | — | Pure-model test; would fail if selection rule broke. Tests nothing about parsing (F-005). |
| `tests::budget_error_is_typed_resource_limit` | 376-388 | SHALLOW | S4 | Asserts a constructor; cannot fail under any parser regression (F-005). |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `default_edition_selects_flag_then_first_visible` | YES — for the model methods only | Covers `default_edition`/`ordered_editions`/`by_uid` selection; ignores all parsing. |
| `budget_error_is_typed_resource_limit` | NO (for any parse behavior) | Tests `edition_budget`'s struct literal only. |
| (integration, other file) mkv_editions_tags.rs | YES for parse shapes | Drives `parse_editions` on hand-built EBML (nested atoms, flags, UIDs, depth bomb at :335); per-test pass status UNVERIFIED here (no cargo). |

No mocks-of-the-subject; no golden files; no `#[ignore]` in this file (rg: 0 hits).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the UID the `Tags` zone targets ... the join key the tags zone and any edition-aware UI need" | editions.rs:5-6, 19-21 | FALSE (wiring claim) | rg: tags.rs never references `editions::`/`EditionList`; only consumer is the integration test. |
| "When D2a's `ChapterAtom` parse lands, the merge point is ... Until then the atoms' non-UID content is skipped" | editions.rs:22-26 | STALE/FALSE | chapters.rs (D2a) landed with the full parse (times/displays/tracks, chapters.rs:1-26) and mod.rs wired it; this module was never merged or removed. |
| "edition count and atom count carry the typed `ResourceLimit` refusal" | editions.rs:34-36 | PARTIALLY FALSE | F-002: nested-atom path guards the wrong counter; UID pushes unguarded. |
| "The atom descent carries an explicit depth counter refused past `MAX_ATOM_NESTING`" | editions.rs:31-33 | REAL | :283-287 verified; policy diverges from live chapters.rs (`ebml::MAX_PARSE_DEPTH`=16) — F-004. |
| "corpus chapter-bomb (0007) carries 2580 atoms ... the ceiling sits an order above" | editions.rs:36-37 | REAL-UNPROVEN | Constant 65_536 verified; corpus file 0007 not inspected by this agent (other agents' fixtures). |

## 7. Dead / duplicated / unreachable code

- The entire parse surface (`parse_editions`, `parse_edition`, `collect_atom_uids`) + model queries (`ordered_editions`, `by_uid`): UNREACHABLE from shipped binaries (F-001 proof chain).
- Duplicated against chapters.rs: element IDs, ceilings, default-edition rule (F-004).

## 8. Dependencies & flags

- No feature flags gate this module; `pub mod editions;` unconditionally (mkv/mod.rs:44), so it compiles into every `demux` build (release rlib) as dead weight.
- Depends on: `media_types` (EngineError/ErrorKind/ErrorContext/LimitKind), `crate::mkv::ebml` (reader/header/uint), `crate::mkv::error` (bad_structure). All local — no external deps introduced.

## 9. Open questions for the next agent

- mkv_editions_tags.rs (another agent's file): does it pin the overrunning-child and multi-UID-atom shapes (F-002/F-003)? If yes, defects are at least witnessed; if no, they are fully unpinned.
- Claim-ledger agents (W18/W19): does any feature-matrix row cite card 5.D2b / editions.rs as evidence for "MKV editions/chapters" — if so, the shipped truth is chapters.rs and the row's evidence path is misleading.
- chapters.rs's owner: confirm `ebml::MAX_PARSE_DEPTH`=16 vs `MAX_ATOM_NESTING`=8 divergence is intentional or drift.
