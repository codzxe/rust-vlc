# AUDIT: crates/playback-engine/src/bookmarks.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-23 |
| Lines / functions | 204 / 5 methods + 6 tests |
| File verdict | UNREACHABLE (real, correct code — zero production callers; the live bookmark feature is a TypeScript reimplementation in the UI demo transport) |
| Reachable from shipped entry point? | NO — grep over apps/ and crates/: `BookmarkList` appears only in this file (definition + tests) and lib.rs:93 `pub mod bookmarks;`. No host dispatches MediaGetBookmarks/MediaAddBookmark/MediaRemoveBookmark (apps/cli/src, apps/desktop/src: zero matches). The user-visible bookmark feature is ui/src/client/demo-transport.ts:313-330+ (TS reimplementation mirroring this module's semantics by comment) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/playback_engine-30ee2bde67c1e3d9 bookmarks` -> 6 passed, 0 failed, exit 0; grep reachability (apps/, crates/); baseline cargo-test-workspace.txt:4097-4102 (same 6 green) |

## 1. One-paragraph verdict
The list semantics themselves are correct and well-tested: creation-ordered 1-based ids that are never reused after delete, typed refusal of negative positions, u32 id-space exhaustion as a typed `None` (checked_add, no panic), insertion-order entries, per-media clear that restarts numbering, and jump resolution to the existing absolute seek flow. All 6 tests pass (executed). But the module is engine-side DEAD WEIGHT: no shipped entry point constructs a `BookmarkList` — the bookmark wire commands (media.get/add/remove-bookmark, commands/src/catalog/ids.rs:156-158) are catalog-only (part of W02-A09's un-served majority; CLI and desktop dispatch have zero bookmark references), and the bookmark feature users actually see is a TypeScript reimplementation in the UI's demo transport that cites this module's contract in comments (demo-transport.ts:313 "the engine-side BookmarkList semantics"). The module doc's own parenthetical — "a host (or a future in-engine session) drives them with" — is the tell: the "future" has not arrived.

## 2. What actually works (verified)
- `./target/debug/deps/playback_engine-30ee2bde67c1e3d9 bookmarks` -> `test result: ok. 6 passed; 0 failed` (exit 0).
- Semantics verified by reading + tests: negative refusal (67-70), checked_add exhaustion refusal (79), no-renumber remove (86-90), µs→ns jump (96-103), clear reset (107-110).

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/src/bookmarks.rs:5-11 (module doc) + ui/src/client/demo-transport.ts:313
Code:
```
//! The **pure state half** of the bookmark feature, mirroring how the v0
//! engine owns chapters ([`crate::chapters`]): the wire commands above are
//! carried by the [`commands`] catalog, and this module is the testable
//! list semantics a host (or a future in-engine session) drives them with.
```
What is wrong: the code is real but UNREACHABLE — zero production callers (verified by grep across apps/ and crates/; only self-tests + the lib.rs module declaration). The wire commands it exists to serve (media.get/add/remove-bookmark) are not dispatched by ANY host (CLI dispatch.rs: no bookmark matches; desktop src: none), and the live user-facing implementation is a separate TypeScript copy in the UI demo transport that mirrors this module's semantics by comment, not by reference. Why it matters: the feature exists twice in two languages with only comments keeping them aligned; a semantics change here changes nothing a user can trigger, and a change there drifts silently from this "contract". What correct looks like: either wire the commands to this type in a host (the CLI's command layer would be the natural owner) or delete the module and keep the contract doc next to the TS implementation. Confidence: HIGH — Verification: read + grep + ran.
Cross-ref: W02-A09's finding that 50-63 of 65 catalog commands answer typed unsupported — the three bookmark commands are in that un-served set.

F-002 [S4]
Where: crates/playback-engine/src/bookmarks.rs:96-103 (seek_target)
Code:
```
            .map(|bookmark| SeekTarget::Absolute {
                position_ns: (bookmark.time.0) * US_TO_NS,
            })
```
What is wrong: unchecked i64 multiplication µs→ns — overflows for `time_us > i64::MAX / 1_000` (~9.2e15 µs ≈ 292 years): debug panic, release wrap (a negative ns target that the seek flow would clamp to 0 — a wrong-but-tame jump). `add()` accepts any non-negative i64 µs, so a hostile/buggy wire client sending `Timestamp(i64::MAX)` arms it. Currently DEAD CODE (F-001), so no shipped impact; if the commands are ever wired, this becomes an S3 on an externally-supplied input path (control-protocol server relays client commands). Contrast: the seek-side ns arithmetic elsewhere uses saturating/checked forms. What correct looks like: `time_us.checked_mul(US_TO_NS)` mapping overflow to the same typed `None` refusal family. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| BookmarkList::new | 50-56 | REAL (unreachable) | — | ids start at 1 |
| BookmarkList::entries | 58-62 | REAL (unreachable) | — | insertion order |
| BookmarkList::add | 64-82 | REAL (unreachable) | — | negative refused; checked_add exhaustion refused; label untrusted-text passthrough (V7) |
| BookmarkList::remove | 84-90 | REAL (unreachable) | — | retain, no renumbering |
| BookmarkList::seek_target | 92-103 | REAL (unreachable) | S4 | F-002 unchecked µs→ns multiply |
| BookmarkList::clear | 105-110 | REAL (unreachable) | — | resets list AND counter |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| add_assigns_creation_ordered_ids_from_one | YES | full-structure equality on entries (ran) |
| negative_position_is_refused_typed | YES | -1 → None, empty; 0 valid (ran) |
| remove_deletes_without_renumbering | YES | ids [1,3] after removing 2; next add id 4; double-remove false (ran) |
| seek_target_resolves_us_to_ns_absolute | YES | exact ns value + unknown-id None (ran) — does NOT cover the overflow arm (F-002) |
| clear_resets_list_and_numbering | YES | next id 1 after clear (ran) |
| duplicate_positions_are_distinct_bookmarks | YES | distinct ids, len 2 (ran) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the testable list semantics a host (or a future in-engine session) drives them with" (7-9) | module doc | TRUE-but-damning | no host drives it (grep); "future" = not shipped |
| "the session owner calls it [clear] on session.open" (27-29) | module doc | FALSE in practice | no session owner calls clear() — no callers at all (grep); the TS demo transport implements its own per-media reset (demo-transport.ts:745) |
| "ids ... never reused after a delete" (15-16) | module doc | REAL | test pins (ran) |
| "1-based to match the keymap's bookmark slots (keymaps/default.toml set-bookmark-N, N = 1..10)" (16-18) | module doc | REAL-UNPROVEN | keymap file not audited by me (input crate domain) |

## 7. Dead / duplicated / unreachable code
The entire module is production-dead (F-001): reachable only from `cargo test`. The duplication is cross-language: ui/src/client/demo-transport.ts:313-330, 607-630 reimplements id ordering, negative refusal, and per-media reset in TypeScript with this module as the commented "contract".

## 8. Dependencies & flags
commands::catalog::types::Bookmark (the wire payload type), crate::seek::SeekTarget. `pub mod bookmarks;` unconditional (lib.rs:93) but unconsumed. No features, no unsafe.

## 9. Open questions for the next agent
- ui/src/client/demo-transport.ts — does the TS copy implement the SAME refusal semantics (id exhaustion? negative time? overflow?) as this module, or does the "contract" already drift? (UI domain agent.)
- keymaps/default.toml set-bookmark-N — are the 10 keymap slots wired to anything that stores bookmarks persistently, or are they also dead? (input crate / UI agents.)
- If Wave-8's "bookmarks" consolidation minted these catalog commands expecting engine backing, the commands-side ledger should record them as unserved — W02-A09's audit already covers the surface; this file confirms the engine half is also absent from any live path.
