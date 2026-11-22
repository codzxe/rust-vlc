# AUDIT: crates/playback-engine/src/bookmarks.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 205 / 6 production items + 6 `#[test]` fns |
| File verdict | UNREACHABLE (honest, tested pure-state library with no driver) |
| Reachable from shipped entry point? | **NO** — `BookmarkList` is constructed by nothing outside this file's tests. The wire commands it serves (`media.add-bookmark` / `media.get-bookmarks` / `media.remove-bookmark`) exist in the commands catalog (crates/commands/src/catalog/command.rs:386-393) with **no executor** in playback-engine, apps/cli, or apps/desktop-tauri (repo-wide grep: catalog definitions + UI demo-transport only). |
| Findings | S0:1 S1:0 S2:0 S3:0 S4:0 |
| Commands run | rg for `BookmarkList`/`add-bookmark`/`MediaAddBookmark` across crates+apps (only catalog + this file + ui/src/client/demo-transport.ts:226,611 + BookmarksDialog.tsx:141); matrix row read (docs/feature-matrix.toml:1580-1593); sed quote verification |

## 1. One-paragraph verdict

A clean 100-line pure-state module: creation-ordered never-reused ids from 1, negative-position typed refusal, no-renumber delete, µs→ns seek-target resolution through the existing absolute seek, per-media clear that resets numbering, duplicate positions allowed. Six behavioral tests pin every rule. The module doc is honest that it is only "the pure state half" awaiting a host "or a future in-engine session". No such host or session exists: the bookmark commands have no executor anywhere, the `BookmarkList` is never constructed in production, and the UI's BookmarksDialog talks to a demo transport. Yet the feature-matrix row `playback-bookmarks` claims status **"partial" on all four platforms** with this file as evidence — no platform can create, list, or jump to a bookmark through any shipped binary.

## 2. What actually works (verified)

- The list semantics themselves, verified by reading all four operations (add 67-82, remove 86-90, seek_target 96-103, clear 107-110) — id discipline (`checked_add` refusal on u32 exhaustion, 79), no renumbering (retain by id), and the µs→ns conversion matching the engine's `SeekTarget::Absolute`.
- W00 baseline cargo-test.log shows the playback-engine suite green (bookmarks' 6 tests included) — UNVERIFIED by my own execution (ADAPTATION).

## 3. Findings

### F-001 [S0] — The `playback-bookmarks` matrix row claims "partial" on every platform for a feature that is unreachable end-to-end

Where: docs/feature-matrix.toml:1580-1593; the unreachable implementation at crates/playback-engine/src/bookmarks.rs:43-111.

Code (docs/feature-matrix.toml:1589-1590, verbatim):
```toml
status = { windows = "partial", linux = "partial", macos = "partial", web = "partial" }
evidence = "crates/playback-engine/src/bookmarks.rs (BookmarkList add/remove/seek_target/clear + tests); commands catalog media.get-bookmarks/media.add-bookmark/media.remove-bookmark (class S, wire round-trip pinned in command.rs tests + catalog_shape.rs); ui BookmarksDialog.tsx (ui-dialog-bookmarks) + BookmarksDialog.test.tsx; demo-transport bookmark handlers + demo-bookmarks.test.ts; keymap Ctrl+B (DIALOG_KEYMAP seed + keymap.test.ts)"
```

Code (crates/playback-engine/src/bookmarks.rs:7-9, verbatim):
```rust
//! list semantics a host (or a future in-engine session) drives them with.
//! No pipeline invalidation is involved — bookmarks are session-scoped
```

What is wrong: the row's status says the feature is partially *shipped* on Windows/Linux/macOS/web. Traced reality: (1) `BookmarkList` — zero non-test constructors; (2) the three wire commands — catalog definitions and serde round-trip tests only, no handler in any engine or host dispatch (grep over `crates/playback-engine/src`, `apps/cli/src`, `apps/desktop-tauri/src`: zero); (3) the UI dialog dispatches through `demo-transport.ts` (a demo-mode transport, not the real bridge) — and even a real bridge would land on a command nobody answers. The evidence string lists exactly the W00-diagnosed pattern: files that exist, none of which prove a user can bookmark anything.

Why it matters: §4 S0 — the claim (partial = shipped in part) is false on all four platforms. The CLI's own command list (W00 baseline lumen-help.txt) contains no bookmark verb.

What correct looks like: status `not_started` (or `paper-only`) until a session executor owns the commands, or the notes say "catalog + state module landed; executor pending" and the status stays honest.

Confidence: HIGH — Verification: read + exhaustive grep.

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-32 | REAL (honest about being the pure half) | — | the lie is in the matrix, not here |
| `US_TO_NS` const | 38 | REAL-UNPROVEN | — | µs wire → ns engine |
| `BookmarkList` (struct) | 43-46 | UNREACHABLE | S0 | F-001 |
| `new` | 51-56 | UNREACHABLE | — | ids start at 1 |
| `entries` | 60-62 | UNREACHABLE | — | insertion order |
| `add` | 67-82 | UNREACHABLE | — | negative refused; `checked_add` id-exhaustion refusal; clone returned |
| `remove` | 86-90 | UNREACHABLE | — | retain-by-id, no renumber |
| `seek_target` | 96-103 | UNREACHABLE | — | µs×1000 → Absolute; None for unknown id |
| `clear` | 107-110 | UNREACHABLE | — | resets numbering (VLC parity) |
| tests (6) | 122-203 | REAL-UNPROVEN | — | §5 |

## 5. Test honesty (6 `#[test]`)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `add_assigns_creation_ordered_ids_from_one` (123-144) | YES | full entry equality |
| `negative_position_is_refused_typed` (147-153) | YES | −1 refused, 0 valid |
| `remove_deletes_without_renumbering` (156-170) | YES | ids [1,3] after removing 2; next add = 4 |
| `seek_target_resolves_us_to_ns_absolute` (173-183) | YES | ns value + unknown-id None |
| `clear_resets_list_and_numbering` (186-194) | YES | numbering restarts |
| `duplicate_positions_are_distinct_bookmarks` (197-203) | YES | distinct ids, len 2 |

All tests exercise a list no shipped binary ever builds.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| status partial (all platforms) for playback-bookmarks | docs/feature-matrix.toml:1589 | FALSE | F-001 |
| "Dialog + engine list semantics landed (create/jump/delete, per-media reset, no id reuse)" | docs/feature-matrix.toml:1592 | HALF-TRUE | list semantics landed as an unconstructed library; the dialog is demo-transport-only; nothing connects them |
| "a host (or a future in-engine session) drives them with" | bookmarks.rs:8-9 | REAL | honest self-description — the future never arrived |

## 7. Dead / duplicated / unreachable code

The entire module is unreachable from shipped entry points (F-001). Re-exported via lib.rs for hosts that do not exist yet.

## 8. Dependencies & flags

Depends on `crate::seek::SeekTarget` and `commands::catalog::types::Bookmark`. No feature gates; compiled into every binary as dead code.

## 9. Open questions for the next agent

- The UI's `demo-transport.ts` bookmark handlers — W15's dead-button audit should cross-reference: the dialog is "live" only against a demo transport.
- The chapters module (`crate::chapters`) is cited as the sibling pattern — same reachability question (another agent's file).
