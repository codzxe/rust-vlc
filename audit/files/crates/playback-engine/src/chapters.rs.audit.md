# AUDIT: crates/playback-engine/src/chapters.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 236 / 5 production fns + 8 test fns |
| File verdict | UNREACHABLE (real, tested code; no shipped caller) |
| Reachable from shipped entry point? | NO — `chapter_at`/`next_chapter`/`previous_chapter`/`chapter_target` are re-exported (lib.rs:110) and called by nothing outside this file's own tests. The wire commands `media.get-chapters`/`media.set-chapter` exist in the commands catalog (crates/commands/src/catalog/command.rs:377-382) but have ZERO handlers in apps/ (rg over apps/: no hits) and no CLI verb maps to them (run.rs `to_catalog` 803-828 has no chapter arm). |
| Findings | S0:0 S1:0 S2:1 S3:2 S4:0 |
| Commands run | rg for the four fn names across crates/playback-engine, apps/; rg `set-chapter|SetChapter|get-chapters` across apps/ + crates/commands; sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

The chapter-navigation math itself is small and mostly careful — containment, strict-next, restart-then-rewind prev, µs→ns conversion, typed `None` refusals, and eight exact tests that would catch semantic regressions. But the entire module is dead in every shipped binary: no host resolves chapters through it, the CLI has no chapter verb, and the wire commands that would consume its output (`media.get-chapters`/`media.set-chapter`) are defined in the commands catalog with no dispatcher behind them. Chapter navigation is a paper feature: catalog vocabulary + orphaned helpers + tests, zero wiring. On top of that, two correctness defects hide in the orphaned code: order-dependent scans that contradict the module's own "not assumed sorted" contract, and unchecked arithmetic over file-controlled chapter times (and a u32 index increment that can panic in debug builds).

## 2. What actually works (verified)

- The four lookups are pure functions over `&[Chapter]` with typed `None` for not-found; the test table (161-235) pins containment, strict-next, restart-then-rewind, open-ended chapters, and empty lists exactly. As library logic: REAL.
- The restart-then-rewind policy and its 10 s threshold are honestly tagged `[PROPOSAL]` (28-31, 48) rather than presented as VLC parity.
- Module doc's claim that `crate::seek`'s `SeekTarget::Chapter` refuses typed and hosts resolve to `Absolute` is consistent with this module's design (32-33).

## 3. Findings

### F-001 [S2] Whole module unreachable; chapter navigation is unwired end to end
Where: crates/playback-engine/src/chapters.rs:58-115 (all four public fns); crates/commands/src/catalog/command.rs:377-382; apps/cli/src/run.rs:803-828 (no chapter verb)
Code (chapters.rs:7-15):
```rust
//! The **pure resolution half** of chapter navigation. The v0 engine
//! has no in-session chapter source ([`crate::seek`]'s
//! `SeekTarget::Chapter` refuses typed and hosts resolve chapters to
//! `Absolute` — the module docs there say exactly this), so this
//! module owns that resolution as testable data functions over the
```
Code (command.rs:377-382):
```rust
    /// `media.get-chapters` — chapter markers (class S).
    #[serde(rename = "media.get-chapters")]
    MediaGetChapters,
    /// `media.set-chapter` — canonical chapter jump (class A, gen).
    #[serde(rename = "media.set-chapter")]
    MediaSetChapter {
```
What is wrong: The resolution half exists and is tested; the consuming half does not exist anywhere. rg over apps/ finds no handler for `MediaGetChapters`/`MediaSetChapter`, no CLI verb maps to them, and no code outside this file calls the four helpers. The doc (doc 05 §2.10, issue #32) describes chapter navigation as a wire feature; a shipped user cannot perform it by any path.
Why it matters: Another vocabulary-plus-orphans feature: the catalog advertises commands the products cannot execute, and this file's entire purpose (host-side resolution) has no host. Claim-ledger consequence: any matrix row citing chapter navigation cannot be REAL.
What correct looks like: A host (CLI verb or desktop dispatch arm) that lists chapters and maps `set-chapter` through `chapter_target` into the existing seek flow.
Confidence: HIGH — Verification: read (rg call graph, both directions).
### F-002 [S3] Order-dependent scans contradict the "not assumed sorted" contract
Where: crates/playback-engine/src/chapters.rs:71-74 (next_chapter), 87-89 (previous_chapter)
Code (71-74):
```rust
    chapters
        .iter()
        .find(|chapter| (chapter.time_start.0) > position_us)
        .map(|chapter| chapter.index)
```
What is wrong: `next_chapter` returns the FIRST chapter in **list order** whose start is past the position, not the earliest such start; `previous_chapter`'s `rfind` returns the LAST in list order starting at/before the position. With an unsorted list (e.g., MKV editions emitted out of time order), both can pick the wrong chapter. The module doc explicitly promises "Chapter lists are not assumed sorted — lookups scan for containment" (38-39): containment (chapter_at) is order-tolerant, but next/previous are not.
Why it matters: The stated input contract is violated by two of the four functions; on real MKV chapter data (where atom order is file-chosen) the step targets can be nonsense.
What correct looks like: `min_by_key`/`max_by_key` over qualifying starts, or sort-once at the boundary and document it.
Confidence: HIGH — Verification: read.
### F-003 [S3] Unchecked arithmetic over file-controlled chapter values; u32 index overflow panic in debug
Where: crates/playback-engine/src/chapters.rs:90, 101, 113, 126
Code (113):
```rust
        .map(|chapter| SeekTarget::Absolute {
            position_ns: chapter.time_start.0 * US_TO_NS,
```
Code (101):
```rust
        .find(|chapter| chapter.index + 1 == index)
```
What is wrong: (a) `time_start.0 * 1000` and `(position_us − start) * 1000` overflow i64 for chapter starts beyond ~292 years in µs — MKV `ChapterTimeStart` is a file-controlled u64 ns field upstream, so a crafted file can push wire µs values into overflow territory: debug builds panic, release builds wrap to a negative seek target. (b) `start + time.0` in `chapter_contains` (126) has the same exposure. (c) `chapter.index + 1` (101) overflows when `index == u32::MAX`: debug panic, release wrap-to-0 comparison.
Why it matters: The module doc's own "Untrusted inputs" section (36-41) declares the numbers untrusted, then the arithmetic trusts them. Currently unreachable (F-001) so no shipped crash today; the moment it is wired, a malformed MKV panics debug builds.
What correct looks like: `checked_mul`/`saturating_mul` on the µs→ns conversions, `checked_add` on the containment end, and `index.checked_add(1)` with a match.
Confidence: HIGH on the code paths (read); MEDIUM on the upstream range of `time_start` (demux/mkv/chapters.rs is W04's file — flagged).

## 4. Per-function table (production code)

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `chapter_at` | 58-63 | UNREACHABLE | — | Containment via `chapter_contains`; order-tolerant (first containing wins). |
| `next_chapter` | 69-75 | UNREACHABLE | S3 | List-order `find` — F-002. |
| `previous_chapter` | 83-103 | UNREACHABLE | S3 | `rfind` order dependence (F-002); `index + 1` overflow (F-003c); restart-then-rewind logic itself correct per tests. |
| `chapter_target` | 108-115 | UNREACHABLE | S3 | Unchecked µs→ns multiply (F-003a). |
| `chapter_contains` | 119-130 | UNREACHABLE | S3 | `start + duration` overflow exposure (F-003b); open-ended handling correct. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| chapter_at_maps_containment (161-168) | YES | Exact containment incl. before-first. |
| next_chapter_finds_strictly_later_start (171-178) | YES | Strict boundary at 10 s (== start → next). |
| previous_chapter_restarts_when_deep_in_chapter (181-187) | YES | Restart vs rewind split at the threshold. |
| previous_chapter_before_first_start_is_none (190-194) | YES | None semantics. |
| previous_chapter_early_in_first_chapter_is_none (197-203) | YES | Rewind-past-list + restart-first cases. |
| chapter_target_resolves_to_absolute_ns (206-215) | YES | µs→ns value + out-of-range None. |
| empty_list_is_all_none (218-223) | YES | Degenerate input. |
| open_ended_chapter_contains_from_its_start (226-235) | YES | Unknown-duration arm. |

Honest suite — but it only proves the orphan. No test uses an unsorted list (F-002 uncovered), and none uses extreme values (F-003 uncovered).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Chapter navigation — next/previous/current lookup over the wire chapter list (issue #32, `media.get-chapters` / `media.set-chapter`; doc 05 §2.10)" (lines 1-3) | module doc | FALSE (as a product claim) | The wire commands have no handler in any app; the helpers have no caller. Navigation cannot be performed from any shipped entry point (F-001). |
| "Chapter lists are not assumed sorted — lookups scan for containment" (38-39) | module doc | SHALLOW | True for `chapter_at`; false for next/previous (F-002). |
| "Chapter titles are untrusted text (V7) and are never touched here" (36-37) | module doc | REAL | Verified: no field access on `title` outside fixtures. |
| "MKV editions/entries and MP4 chapter traks both surface through it" (13-15) | module doc | UNVERIFIED | Upstream demux chapter harvest is W04's files; nothing in apps/ surfaces either to a user regardless (F-001). |

## 7. Dead / duplicated / unreachable code

The entire module (see F-001). It is re-exported publicly (lib.rs:110), so it compiles into shipped binaries as dead weight.

## 8. Dependencies & flags

Depends on `commands` (wire types) and `crate::seek::SeekTarget`. No feature flags, no unsafe, no panics in the common path (F-003c is the only debug-panic path).

## 9. Open questions for the next agent

- W04 (demux/mkv/chapters.rs): what bounds, if any, cap `ChapterTimeStart`/duration before they reach wire `Timestamp` µs? F-003's reachability depends on it.
- W02/W18: which matrix rows cite chapter navigation (issue #32)? They are UNREACHABLE at best.
