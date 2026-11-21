# AUDIT: crates/playback-engine/src/chapters.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2026-11-21 |
| Lines / functions | 236 / 6 production units + 8 tests |
| File verdict | REAL (pure, tested resolution functions) but UNREACHABLE from shipped entry points (zero callers; both hosts refuse the `media.*` command family this module serves) |
| Reachable from shipped entry point? | NO. `rg -ln "chapter_at\|next_chapter\|previous_chapter\|chapter_target" crates/ apps/` → only this file and its re-export (`crates/playback-engine/src/lib.rs:94,109`). The wire commands the module doc names (`media.get-chapters` / `media.set-chapter`) are refused by both shipped hosts: `apps/cli/src/dispatch.rs:205-206` ("media." → "chapters and their navigation are the chapter card") and `apps/web/src/host.rs:1856` identically. The demux side has its own separate `chapter_targets` (crates/demux/src/mkv/mod.rs:276) — not this module. Only the test suite executes it: all 8 `chapters::tests` ran green in the no-fail-fast baseline (orchrun log lines 3749-3757). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | full read via Read tool (236 lines); `rg -ln "chapter_at\|next_chapter\|previous_chapter\|chapter_target" crates/ apps/` (caller map — none); `rg -n "\"media\\.\"" apps/cli/src/dispatch.rs apps/web/src/host.rs` (both hosts refuse the media.* family); `rg -n "chapters" crates/playback-engine/src/lib.rs` (module + re-export only); `sed -n '90p;113p;126p'` (quote verification); `rg -n "test chapters::tests" orchrun log` (8/8 green, lines 3749-3757); boundary math hand-checked (F-002); overflow class identified at 90/113/126 (F-001) |

## 1. One-paragraph verdict
A small, honest, well-tested pure module that nothing shipped can call: the
module doc itself says the v0 engine has no in-session chapter source and
"hosts resolve chapters" — but the hosts refuse the entire `media.*` family,
so the resolution functions live only in the re-export and the test suite
(all 8 green in the no-fail-fast baseline). The semantics are sound for
well-formed lists (containment with open-ended tails, strictly-later next,
restart-then-rewind prev with a documented 10 s [PROPOSAL] threshold, typed
`None` refusals everywhere, titles never touched), with two real defects:
unchecked i64 arithmetic over parsed-container values (`start * 1000`,
`start + length`, `(position − start) * 1000` — a hostile MKV chapter list
with µs starts beyond ±9.2e15 overflows: debug panic, release wrap into a
wrong seek target), and a truncating ns→µs conversion that maps a position
up to 999 ns *before* a chapter start into that chapter. One test comment
mislabels its own mechanism. Nothing here is fake; nothing here is
reachable either.

## 2. What actually works (verified)
— Typed refusal discipline — verified: every function answers `Option`,
no clamping (the doc's "an out-of-range index is a typed None" is the code,
108-115), no panics on empty lists (test `empty_list_is_all_none`),
untrusted titles never read (only `time_start`/`duration`/`index` touched).
— `chapter_at` containment — verified: start ≤ pos < start+len when the
length is known and positive, open-ended otherwise (119-130); before the
first start → None (test 167).
— `next_chapter` strictness — verified: `start > position` (73), so a
position exactly at a start does not count that chapter (test 174-176).
— `previous_chapter` restart-then-rewind — verified by trace and by
hand-computing the test positions: rfind for the last-started chapter
(87-89), elapsed > 10 s restarts (91-93), near-start rewinds to `index − 1`
via the index-based find (99-102), before-every-start and
early-in-first-chapter answer None (96-98, tests 190-203).
— `chapter_target` — verified: µs→ns conversion (113), index-matched,
`SeekTarget::Absolute` for the existing seek flow; unknown index → None
(test 214).
— Executed: 8/8 tests green in the no-fail-fast baseline (orchrun log
3749-3757 — every test in this file ran).

## 3. Findings

### F-001 [S3]
Where: crates/playback-engine/src/chapters.rs:113 (and the same class at 90 and 126)
Code:
```rust
        .map(|chapter| SeekTarget::Absolute {
            position_ns: chapter.time_start.0 * US_TO_NS,
        })
```
(the siblings:)
```rust
    let elapsed_ns = (position_us - current.time_start.0) * US_TO_NS;
```
```rust
            position_us < start + (time.0)
```
What is wrong: unchecked i64 arithmetic over wire chapter values that
ultimately come from parsed containers (MKV editions/atoms, MP4 chapter
traks — the module doc's own upstream, lines 13-15). `time_start.0` is i64
µs; `* 1000` overflows past ±9.22e15 µs (≈ ±292,000 years), `start +
length` overflows when both are large, and the elapsed subtraction can
overflow at opposite extremes. A crafted or corrupt chapter list therefore
yields: debug builds — panic (overflow check); release builds — silent
wrap into a nonsense (possibly negative) seek target or containment
answer. Nothing in this module bounds the values, and whether the demux
layer clamps them is unverified here (demux's `mkv::chapters` has
"extreme_times_land_in_the_tolerant_bucket" tests — baseline log line
1729 — suggesting some tolerance handling exists, but the wire `Chapter`
this module receives is host-held data, not necessarily demux-clamped).
Why it matters: the checklist's "unchecked arithmetic on parsed
box/sample sizes" — chapter atoms are parsed file data, and the failure
is silent in release.
What correct looks like: `checked_mul`/`checked_add`/`checked_sub` with
the same typed-`None` refusal the module already uses everywhere else.
Confidence: HIGH — Verification: read (arithmetic verified; reachability
of a hostile list through a shipped host is currently NO, which is what
contains this at S3 rather than S2).

### F-002 [S4]
Where: crates/playback-engine/src/chapters.rs:59 (and 70, 84)
Code:
```rust
pub fn chapter_at(chapters: &[Chapter], position_ns: i64) -> Option<usize> {
    let position_us = position_ns / US_TO_NS;
```
What is wrong: truncating integer division toward zero. A position up to
999 ns **before** a chapter start converts to the start's exact µs value
and is contained: `chapter_at(list, 9_999_999_999)` → `10_000_000 µs` →
`Some(1)` for a chapter starting at 10 s. Symmetrically, a negative
sub-µs position (−500 ns) truncates to 0 µs and is contained in a chapter
starting at 0. The doc says "positions at/after a start belong to that
chapter" — the code makes "up to 1 µs before" belong to it too.
Why it matters: sub-microsecond wrong-side boundary; real positions come
from media clocks in ns, so the edge is reachable in principle but
harmless in practice (nobody seeks by 1 ns). Undocumented rounding
semantics in an otherwise precisely documented module.
What correct looks like: `div_euclid` (consistent floor for negatives)
and/or a doc line stating the µs-granularity rounding.
Confidence: HIGH — Verification: read + hand-computation of both edge
cases.

### F-003 [S4]
Where: crates/playback-engine/src/chapters.rs:201-202
Code:
```rust
        // Deep into chapter 0: restart chapter 0 itself.
        assert_eq!(previous_chapter(&chapters, secs(11)), Some(0));
```
What is wrong: the comment mislabels the mechanism. 11 s is 1 s into
**chapter 1** (10–20 s); `Some(0)` is the *rewind* branch (elapsed 1 s ≤
threshold → previous index), not "restart chapter 0". Restarting chapter 0
would require a position deep inside 0–10 s — but then current would be
chapter 0 and the answer would still be `Some(0)` via restart, so the
assertion value is right for a different reason than the comment states.
Why it matters: a wrong explanatory comment on the one test that pins the
rewind/restart boundary is how future regressions get rationalized.
What correct looks like: "// 1 s into chapter 1: near-start rewind to
chapter 0 (elapsed ≤ threshold)" — and ideally an added vector at secs(9)
vs secs(10.5) to actually pin the restart-inside-chapter-0 case.
Confidence: HIGH — Verification: read + trace of both branches.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `CHAPTER_RESTART_THRESHOLD_NS` | 46-49 | REAL | — | 10 s, [PROPOSAL] Lead-call documented. |
| `US_TO_NS` | 51-52 | REAL | — | i64 const 1000. |
| `chapter_at` | 58-63 | REAL | S4 | F-002 truncation; first-in-list containment (unsorted lists = scan order, documented at 39-40). |
| `next_chapter` | 69-75 | REAL | — | Strictly-later start; first-in-list on unsorted lists. |
| `previous_chapter` | 83-103 | REAL | S3 | F-001 elapsed arithmetic (90); restart-then-rewind verified; index-based rewind target. |
| `chapter_target` | 108-115 | REAL | S3 | F-001 unchecked µs→ns multiply (113). |
| `chapter_contains` | 119-130 | REAL | S3 | F-001 `start + length` (126); open-ended zero/unknown lengths handled. |
| 8 tests | 160-235 | REAL, EXECUTED | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `chapter_at_maps_containment` (161) | YES | Mid-chapter ×3 + before-first → None. EXECUTED green (orchrun 3749). |
| `next_chapter_finds_strictly_later_start` (171) | YES | At-boundary (secs(10) → Some(2), not Some(1)) pins strictness. EXECUTED green. |
| `previous_chapter_restarts_when_deep_in_chapter` (181) | YES | 11 s-elapsed restart vs 2 s-elapsed rewind. EXECUTED green. |
| `previous_chapter_before_first_start_is_none` (190) | YES | Negative positions. EXECUTED green. |
| `previous_chapter_early_in_first_chapter_is_none` (197) | YES (value) | Both asserts correct; the comment is wrong (F-003). EXECUTED green. |
| `chapter_target_resolves_to_absolute_ns` (206) | YES | µs→ns value pinned + unknown index None. EXECUTED green. |
| `empty_list_is_all_none` (218) | YES | All four functions over `&[]`. EXECUTED green. |
| `open_ended_chapter_contains_from_its_start` (226) | YES | Duration::Unknown branch. EXECUTED green. |
No test covers: overflow (F-001), the sub-µs boundary (F-002), or unsorted
lists — the three edges the doc's "not assumed sorted" and "untrusted"
framing point at.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The v0 engine has no in-session chapter source … hosts resolve chapters to Absolute" | module doc 7-11 | REAL | SeekTarget::Chapter refuses typed (seek.rs, cross-ref); zero engine callers of this module. |
| "MKV editions/entries and MP4 chapter traks both surface through it; crates/demux's ChapterList is the upstream source" | module doc 13-15 | REAL-UNPROVEN | demux has mkv chapters (mkv::chapters tests in baseline log 1723-1732); MP4 chapter trak surfacing unverified here (demux wave's call); the path to THIS module is refused by hosts either way. |
| "positions at/after a start belong to that chapter" | module doc 20-21 | REAL with an undocumented ±1 µs fuzz | F-002. |
| "Chapter lists are not assumed sorted — lookups scan" | module doc 39-40 | REAL | find/rfind/position semantics; no sort anywhere. |
| "restart-then-rewind … 10 s threshold is a picked default — Lead call documented" | module doc 24-31 | REAL | Code 91 + const 49; [PROPOSAL] honestly marked. |
| "chapter_target clamps nothing — an out-of-range index is a typed None" | module doc 40-41 | REAL | Code 108-115. |

## 7. Dead / duplicated / unreachable code
- The entire module is unreachable from shipped entry points (Field row) —
  exported API + tests only. This is the documented "pure resolution half"
  awaiting its host; the host command family is refused by both shipped
  hosts.
- Near-duplicate concept: demux's `chapter_targets` (mkv/mod.rs:276) serves
  the demux-side seek tests — two chapter-resolution implementations in the
  tree (different types, no shared code). Not flagging as duplication —
  different layers — but the next chapter-card owner should unify or
  justify.

## 8. Dependencies & flags
- Deps: `crate::seek::SeekTarget` + `commands::catalog::types::Chapter` —
  consistent with the manifest. No feature gates, no unsafe, no allocation
  beyond the closure captures.

## 9. Open questions for the next agent
- Does the demux layer clamp chapter start/duration values before they
  reach the wire `Chapter`? If not, F-001 becomes an S2 the day any host
  serves `media.get-chapters` — the overflow would then be
  file-controlled.
- The seek.rs owner (W03-A07): confirm `SeekTarget::Chapter`'s typed
  refusal text still matches this module's doc (I did not re-read seek.rs;
  the module doc's cross-reference is asserted, not verified, by me).
