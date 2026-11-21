# AUDIT: crates/playback-engine/src/bookmarks.rs (whole file, 204 lines)

| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-21 |
| Lines / functions | 204 / 6 production units + 6 tests |
| File verdict | UNREACHABLE (real, tested pure-state module; zero consumers — not even the engine's own session uses it) |
| Reachable from shipped entry point? | NO. `rg -n "BookmarkList" --glob '!crates/playback-engine/src/bookmarks.rs' -l .` → only ui/ demo files (the DemoTransport simulation + its tests), docs/feature-matrix.toml, and nothing in any Rust host: the wire commands `media.*-bookmark` exist in the commands catalog (crates/commands/src/catalog/{command,ids,spec}.rs) but no host wires them to this type — apps/cli has no bookmark verb (parse.rs verb list), apps/web host has no bookmark handler, and the engine's own session never constructs a `BookmarkList`. The module's suites ARE executed: 6/6 `bookmarks::tests` pass in `orchrun-test-workspace-nofailfast.log.txt` (3745-3750 region of the 83-test lib run). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | Read tool over the whole file; `rg -n "BookmarkList" --glob '!crates/playback-engine/src/bookmarks.rs' -l .` (consumer proof); `rg -n "bookmark" apps/cli/src/host.rs apps/web/src/host.rs` (no host wiring); `rg -n "AddBookmark" crates/commands/src -l` (catalog rows exist); `sed -n '1584,1594p' docs/feature-matrix.toml` (the row honestly says partial); baseline test lines verified for all 6 tests |

## 1. One-paragraph verdict
A clean, small, honestly-scoped pure-state module: creation-ordered never-reused ids, typed refusals (negative position → None, id exhaustion via checked_add), no renumbering on delete, insertion order preserved, untrusted labels never interpreted — with six real tests that pin every documented semantic and all pass in the new no-fail-fast baseline. It is also completely unwired: no session field, no host command handler, nothing outside its own tests constructs it, and the UI's bookmark feature runs on a DemoTransport simulation — the matrix row for it honestly says `partial`, so no inflation. Three defects, all latent behind the zero-consumer fact: the derived `Default` implementation starts ids at **0**, silently breaking the documented 1-based contract (the keymap slots are 1..10) for any consumer that constructs via `Default` instead of `new`; `seek_target`'s µs→ns multiplication is unchecked while `add` accepts any non-negative i64 µs, so an absurd host-supplied position overflows (debug panic / release wrap to a negative `SeekTarget::Absolute` that the seek resolver would clamp to media start — a silent jump to 0:00); and `add`'s single `None` conflates "bad position" with "id space exhausted" so a caller cannot distinguish input error from system exhaustion.

## 2. What actually works (verified)
— Id semantics — verified: `new()` seeds `next_id: 1`; `checked_add` refusal before push (79) so an exhausted space never wraps nor reuses; delete-then-add does not reuse (test 156-170 pins it).
— Typed refusals — verified: negative position refused with empty-list invariant (test 147-153); unknown id → None, never a neighbor (test 182).
— µs→ns resolution for the sane range — verified: `seek_target` maps to the existing `SeekTarget::Absolute` flow (no separate jump command — the doc's claim); test pins 90 s → 90e9 ns.
- Per-media reset — verified: `clear()` empties and restarts numbering (test 186-194).
— Untrusted label handling — verified: stored verbatim, never parsed (`Bookmark { label }` round-trips; no interpretation anywhere in the module).
— Matrix honesty — verified: row `vlc-playlist-bookmarks`-family row at feature-matrix.toml:1584-1591 says `partial` on all four platforms with an evidence string whose every named artifact exists and is tested (this file + catalog rows + UI dialog + demo tests) — the honest counterpart of the `playback-track-switch` inflation I filed in pipeline part3 F-003.

## 3. Findings

### F-001 [S3]
Where: crates/playback-engine/src/bookmarks.rs:42-46 (the derive), 49-56 (the documented constructor)
Code:
```rust
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BookmarkList {
    entries: Vec<Bookmark>,
    next_id: u32,
}
```
and the contract the derive breaks (49-51):
```rust
    /// An empty list; ids start at 1 on the first add.
    #[must_use]
    pub fn new() -> Self {
```
What is wrong: The derived `Default` sets `next_id = 0` (u32's default), so `BookmarkList::default().add(...)` assigns the first bookmark **id 0** — outside the documented 1-based space and outside the keymap's `set-bookmark-N` slots (N = 1..10, module docs 16-18). Every constructor path except `new()` violates the module's own id contract. The doc comment on `new()` ("ids start at 1 on the first add") states an invariant the type does not enforce.
Why it matters: Any consumer reaching for `Default` (the idiomatic construction for a `Default`-derived type — exactly what a host wiring `media.add-bookmark` would write) gets silently 0-based ids; the first slot hotkey (slot 1) then maps to the second bookmark. Zero consumers today, so latent — but this is the module whose entire value is its documented semantics.
What correct looks like: A manual `impl Default for BookmarkList { fn default() -> Self { Self::new() } }` — one line, makes both constructors agree — or drop the derive.
Confidence: HIGH — Verification: read (derive semantics: u32 field defaults to 0; no test constructs via `default()` — all six use `new()`).

### F-002 [S3]
Where: crates/playback-engine/src/bookmarks.rs:67-82 (unbounded acceptance), 96-103 (unchecked conversion)
Code:
```rust
    pub fn add(&mut self, time_us: i64, label: Option<String>) -> Option<Bookmark> {
        if time_us < 0 {
            return None;
        }
```
```rust
            .map(|bookmark| SeekTarget::Absolute {
                position_ns: (bookmark.time.0) * US_TO_NS,
            })
```
What is wrong: `add` bounds the position only from below (≥ 0); any non-negative i64 µs is accepted and stored. `seek_target` then multiplies by 1_000 unchecked: a position above `i64::MAX/1000` µs (≈ 9.2e15 µs ≈ 292,000 years — wire-constructible by any host, since the catalog's `Timestamp(i64)` has no bound) overflows — debug panic; in release, wrap to a NEGATIVE `position_ns`, and `SeekTarget::Absolute { negative }` flows into the seek resolver, whose documented clamping to `[0, duration]` would send the user to media start instead of the bookmark. The module's "never-invent discipline" (its own comment at 76-78) is defeated by the wrap: the jump invents a position. This is the mirror image of event.rs's `ns_to_us` finding (my event part2 F-001) — the µs↔ns boundary is unchecked in BOTH directions.
Why it matters: Unchecked arithmetic on a wire-controlled value in the one module whose selling point is typed refusal; a hostile or buggy host command turns into a panic (debug) or a silent jump to 0:00 (release). Latent only because nothing wires the commands.
What correct looks like: `add` clamps or refuses positions beyond a sane bound (or the media's known duration when the caller has it), and `seek_target` uses `checked_mul` with the same typed `None` refusal as every other failure in this module.
Confidence: HIGH — Verification: read (arithmetic traced; `Timestamp(i64)` unbounded confirmed in the commands catalog types; the seek resolver's clamp behavior read in session/seek during my part4 audit).

### F-003 [S4]
Where: crates/playback-engine/src/bookmarks.rs:76-79
Code:
```rust
        // Id-space exhaustion is the typed refusal (never-invent
        // discipline: no panic, no id reuse) — the caller learns via
        // `None` exactly as for a negative position.
        self.next_id = self.next_id.checked_add(1)?;
```
What is wrong: The `None` returned for id exhaustion is indistinguishable from the `None` for a negative position. A UI cannot tell "your input was invalid" (retry with a fix) from "the id space is exhausted" (a system condition, ~4 billion bookmarks). The comment presents the conflation as a feature ("exactly as for a negative position").
Why it matters: Two failure classes with different user answers share one signal — the module's typed-refusal discipline stops one step short. Practically unreachable (4.3e9 adds), hence S4.
What correct looks like: A `Result<Bookmark, BookmarkError>` with two variants, or at least a doc note that `None` means either.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `US_TO_NS` (const) | 38 | REAL | — | 1_000; the unchecked multiplier (F-002). |
| `BookmarkList` (struct + derives) | 42-46 | REAL (defective derive) | S3 (F-001) | Default breaks the 1-based invariant. |
| `BookmarkList::new` | 50-56 | REAL | — | Seeds next_id 1. |
| `BookmarkList::entries` | 59-62 | REAL | — | Slice accessor, insertion order. |
| `BookmarkList::add` | 50-… 67-82 | REAL | S3/S4 (F-002 unbounded, F-003 conflated None) | checked_add before push (no partial state); clone of a small struct; no panics. |
| `BookmarkList::remove` | 86-90 | REAL | — | retain + length delta — correct, no renumbering (test-pinned). |
| `BookmarkList::seek_target` | 95-103 | REAL (defective) | S3 (F-002) | Unchecked µs→ns; find-first, no guessed neighbor. |
| `BookmarkList::clear` | 107-110 | REAL | — | List + counter reset (test-pinned). |
| `mod tests` (6 tests) | 113-204 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `add_assigns_creation_ordered_ids_from_one` (123) | YES | Full-struct equality on the entries slice. Baseline ok. |
| `negative_position_is_refused_typed` (147) | YES | Both the refusal and the zero-position validity pinned. Baseline ok. |
| `remove_deletes_without_renumbering` (156) | YES | Id sequence [1,3] + the no-reuse add (id 4) + double-remove false. Baseline ok. |
| `seek_target_resolves_us_to_ns_absolute` (173) | YES | Exact ns value + unknown-id None. Baseline ok. |
| `clear_resets_list_and_numbering` (186) | YES | Restart-at-1 pinned. Baseline ok. |
| `duplicate_positions_are_distinct_bookmarks` (197) | YES | Distinct ids for equal times. Baseline ok. |
| Coverage holes | — | No test constructs via `Default` (F-001's hole); no test drives an extreme position through `seek_target` (F-002); no test distinguishes the two `None` reasons (F-003's premise). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "ids: creation-ordered from 1, unique, never reused after a delete" | bookmarks.rs:15-18 | SHALLOW | True via `new()`; the derived `Default` starts at 0 (F-001) — the type does not enforce its own doc |
| "A negative position is refused typed ([`BookmarkList::add`] returns `None`)" | bookmarks.rs:19-22 | REAL | 68-70 + test |
| "jump: … resolves a bookmark to the existing absolute SeekTarget (µs wire → ns engine)" | bookmarks.rs:23-25 | REAL (sane range) | 100-102; overflow beyond i64::MAX/1000 (F-002) |
| "per-media lifetime: clear resets list AND id counter … the session owner calls it on `session.open`" | bookmarks.rs:26-29 | SHALLOW | clear() does its half; NO session owner exists — nothing calls clear on open (zero consumers) |
| "order: insertion order, not time order" | bookmarks.rs:30-32 | REAL | Vec push; entries() preserves order (test) |
| matrix row `Bookmarks` = partial (all platforms), evidence names this file + catalog + UI + demo tests | docs/feature-matrix.toml:1584-1591 | REAL | Every named artifact verified to exist and be tested; partial is the honest status for an unwired engine module — contrast pipeline part3 F-003 |

## 7. Dead / duplicated / unreachable code
- The entire module is unreachable (no consumers — header). Within the module nothing is dead; every method is test-covered.
- No duplication. The closest sibling is `crate::chapters` (chapter jumps) — same shape, separate file, per the module docs' own mirroring rationale.

## 8. Dependencies & flags
`crate::seek::SeekTarget` + `commands::catalog::types::Bookmark` — engine-internal + the wire vocabulary. No cfg, no unsafe, no feature flags, no panics (all refusals typed). `[lints] workspace = true` inherited.

## 9. Open questions for the next agent
1. Whoever wires `media.add-bookmark` into a host: fix F-001/F-002 in the same PR — the `Default` trap and the overflow are exactly the kind of defect a wiring PR bakes in silently.
2. Commands-crate auditor: the catalog's `Timestamp(i64)` is unbounded by construction — a shared checked µs↔ns helper (one in media-types?) would close this file's F-002 and event.rs's ns_to_us finding together.
3. Matrix auditor: the `set-bookmark-N/play-bookmark-N` slot remainder the row's notes name — when it lands, it will be the first real consumer of this module; re-check the id-space assumption (slots 1..10 vs Default's 0) at that point.
