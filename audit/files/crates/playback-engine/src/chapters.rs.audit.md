# AUDIT: crates/playback-engine/src/chapters.rs
| Field | Value |
|---|---|
| Auditor | W03-A08 |
| Date | 2025-06-27 |
| Lines / functions | 236 / 5 (+8 unit tests) |
| File verdict | UNREACHABLE (correct, well-tested pure functions exported at lib.rs:109-110 with zero production callers — the shipped CLI refuses chapter navigation outright) |
| Reachable from shipped entry point? | NO. rg repo-wide (excluding audit/, docs/): callers are chapters.rs itself + the lib.rs re-export only. The shipped lumen-cli refuses `media.*` ("chapters and their navigation are the chapter card", apps/cli/src/dispatch.rs:204-206), refuses `seek chapter N` typed ("no chapter {index}: AVI v0 carries no chapter markers", apps/cli/src/host.rs:1080-1086), and reports `chapters: Vec::<Chapter>::new()` (host.rs:1311). The only handlers anywhere are the UI's demo-transport MOCK (ui/src/client/demo-transport.ts:585-589). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `sed -n` quotes (exit 0); `rg 'chapter_at|next_chapter|previous_chapter|chapter_target'` repo-wide (exit 0); `rg MediaGetChapters|set-chapter` (exit 0); baseline cargo-test-workspace.txt:4103-4110 — all 8 chapters::tests pass |

## 1. One-paragraph verdict
The module doc says hosts "resolve chapters to `Absolute`" and this module "owns that resolution" — but the only shipped host refuses chapter navigation entirely and returns an empty chapter list, so these four functions are dead exports. What is here is good: exact, honest semantics (containment with open-ended tails, strict-next, restart-then-rewind prev with a documented [PROPOSAL] 10 s threshold, typed None for out-of-range), µs↔ns conversion discipline, and 8 precise unit tests that all pass. The feature-matrix row is honest about the state ("partial", "session-side chapter source + UI chapter display still open") — so this is registered scaffolding, not fraud — but per the audit's verdict scale the code is UNREACHABLE and must not be counted as working chapter navigation on any platform. Two latent defects are worth fixing before the wiring lands: an unchecked µs→ns multiply on container-supplied chapter starts, and list-order dependence of next/previous on unsorted lists (contradicting the module's own "not assumed sorted" claim).

## 2. What actually works (verified)
— All 8 unit tests pass: baseline cargo-test-workspace.txt:4103-4110.
— The semantics are pinned exactly: `chapter_at` containment incl. before-first-is-None; `next_chapter` strictly-later and None at/after the last start; `previous_chapter` restart-then-rewind (11 s → restart, 2 s → rewind, index 0 → None); `chapter_target` µs→ns and typed None; empty list all-None; open-ended chapter contains from its start.
— `chapter_at`'s position arithmetic cannot overflow (position_ns/1000*1000 ≤ position_ns); `previous_chapter`'s elapsed computation is bounded the same way (verified by derivation).

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/src/chapters.rs:5-15 (module docs) vs apps/cli/src/host.rs:1067-1086
Code:
```
//! The v0 engine has no in-session chapter source ([`crate::seek`]'s
//! `SeekTarget::Chapter` refuses typed and hosts resolve chapters to
//! `Absolute` — the module docs there say exactly this), so this
//! module owns that resolution as testable data functions over the
//! [`commands`] wire [`Chapter`] values a host already holds from
//! `media.get-chapters`
```
What is wrong: no host holds anything. The shipped CLI: refuses every `media.*` command (dispatch.rs:204-206), refuses `seek chapter N` with NotFound (host.rs:1080-1086), and returns an empty chapter list in media info (host.rs:1311). The demuxers parse chapters (mkv `chapter_targets`, mp4 `chapters()`) but the session layer surfaces none of it. The only consumers of `media.get-chapters`/`media.set-chapter` anywhere are the UI's demo-transport mock. The module doc's framing ("a host already holds from media.get-chapters") describes a state that does not exist in any shipped binary. Why it matters: 236 lines of exported engine API with zero runtime effect; the feature-matrix "partial" row is honest but the REAL/252 count must treat this row's engine half as UNREACHABLE. What correct looks like: wire the session-side chapter source (the row's own "still open" item) or unexport until then. Confidence: HIGH — Verification: rg repo-wide + read of the CLI refusal paths.

F-002 [S3]
Where: crates/playback-engine/src/chapters.rs:108-115
Code:
```
    chapters
        .iter()
        .find(|chapter| chapter.index == index)
        .map(|chapter| SeekTarget::Absolute {
            position_ns: chapter.time_start.0 * US_TO_NS,
        })
```
What is wrong: unchecked i64 multiply on a container-supplied value. `time_start.0` is µs from the demux layer (MKV `ChapterTimeStart` / MP4 chapter trak); a crafted container carrying a start ≥ i64::MAX/1000 µs (~292 years) overflows — panic in debug, a wrapped NEGATIVE seek target in release. The MKV parser's own tests show extreme times are bucketed as "tolerant" notes, not clamped (baseline:1942 `extreme_times_land_in_the_tolerant_bucket`), so no upstream guard exists. Latent today only because the module is unreachable (F-001); the day a host wires chapters in, this becomes an S2. What correct looks like: `checked_mul` with a typed None, or clamp at parse. Confidence: HIGH — Verification: read + arithmetic. (Sibling conversions in the CLI use `saturating_mul` — host.rs:1072 `time.0.saturating_mul(1000)` — this module is the outlier.)

F-003 [S4]
Where: crates/playback-engine/src/chapters.rs:69-75 and 39-41
Code:
```
    chapters
        .iter()
        .find(|chapter| (chapter.time_start.0) > position_us)
        .map(|chapter| chapter.index)
```
What is wrong: the module doc claims "Chapter lists are not assumed sorted — lookups scan for containment" (lines 39-40), but only `chapter_at` is order-insensitive. `next_chapter` returns the FIRST list element with a later start (not the earliest such), and `previous_chapter`'s `rfind` takes the LAST list element with an earlier start — both are list-order-dependent on unsorted input, contradicting the doc's blanket claim. Real demux sources emit sorted lists, so the practical impact is nil. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| chapter_at | 57-63 | REAL (code) / UNREACHABLE (prod) | — | order-insensitive containment |
| next_chapter | 68-75 | REAL (code) / UNREACHABLE (prod) | S4 | list-order-dependent (F-003) |
| previous_chapter | 82-103 | REAL (code) / UNREACHABLE (prod) | S4 | restart-then-rewind [PROPOSAL] documented; rfind order-dependent (F-003) |
| chapter_target | 107-115 | REAL (code) / UNREACHABLE (prod) | S3 | unchecked µs→ns multiply (F-002) |
| chapter_contains | 119-130 | REAL | — | open-ended length handling correct |
| 8 tests | 160-235 | REAL | — | exact, boundary-complete |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| chapter_at_maps_containment | YES | exact indices incl. before-first None |
| next_chapter_finds_strictly_later_start | YES | strict boundary at 10 s and 20 s pinned |
| previous_chapter_restarts_when_deep_in_chapter | YES | 11 s vs 2 s threshold behavior |
| previous_chapter_before_first_start_is_none | YES | both negative and small-positive |
| previous_chapter_early_in_first_chapter_is_none | YES | index-0 rewind guard |
| chapter_target_resolves_to_absolute_ns | YES | exact ns value + out-of-range None |
| empty_list_is_all_none | YES | all four functions |
| open_ended_chapter_contains_from_its_start | YES | Unknown duration branch |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "hosts resolve chapters to Absolute — this module owns that resolution" | chapters.rs:7-11 | FALSE (no host does) | host.rs:1080-1086 refuses; dispatch.rs:204-206 refuses; rg shows zero callers |
| "Chapter lists are not assumed sorted — lookups scan for containment" | chapters.rs:39-41 | SHALLOW | true for chapter_at only; next/previous are list-order-dependent (F-003) |
| matrix row "partial" + "session-side chapter source + UI chapter display still open" | feature-matrix.toml:1577-1579 | REAL (honest) | the engine half is UNREACHABLE; catalog + MKV parse + demo-transport mock exist |
| restart-then-rewind prev, 10 s threshold [PROPOSAL] | chapters.rs:28-31, 49 | REAL | documented Lead call; tests pin the semantics |

## 7. Dead / duplicated / unreachable code
- The entire public surface (4 fns + const) is production-unreachable (F-001); exported at lib.rs:109-110.

## 8. Dependencies & flags
commands (wire Chapter type) + crate::seek::SeekTarget; no external deps, no unsafe, no feature gates. Tests run in the default lane (baseline :4103-4110).

## 9. Open questions for the next agent
- crates/demux/src/mkv/chapters.rs and the MP4 chapter path parse real container chapters that nothing surfaces — whoever audits the demux chapter parser should cross-check whether the "tolerant bucket" for extreme times (baseline:1942) lets an overflow-sized start reach `Chapter.time_start` unclamped (feeds my F-002).
- ui/src/client/demo-transport.ts:585-589 serves media.get-chapters/set-chapter in the UI's MOCK backend — if any UI test or doc presents that as engine behavior, it is a fake-backend proof, not evidence.
