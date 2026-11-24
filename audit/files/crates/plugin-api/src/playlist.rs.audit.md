# AUDIT: crates/plugin-api/src/playlist.rs
| Field | Value |
|---|---|
| Auditor | W12-A04 |
| Date | 2026-11-30 |
| Lines / functions | 324 / 8 items (2 types, 2 traits, 1 enum, 1 fn, 2 test impls) + 3 tests |
| File verdict | SHALLOW scaffolding: types REAL, trait surface has ZERO implementors and ZERO consumers repo-wide |
| Reachable from shipped entry point? | NO — nothing implements or calls `plugin_api::playlist::*` outside this file's own tests |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `cargo test -p plugin-api --lib playlist` → 3 passed, exit 0; rg consumer sweeps |

## 1. One-paragraph verdict
This is a trait vocabulary waiting for a plugin ecosystem that cannot load: `PlaylistParser`/`PlaylistExporter` are well-shaped chunked contracts (bounded work, sink-based, recovery vocabulary), but the only implementations in the entire repo are the two toy structs inside this file's test module. The real playlist functionality lives elsewhere (`crates/playlists`: m3u/pls/xspf/hls/mpd/cue parsers consumed by `crates/streaming`), making this a second, parallel `Playlist` vocabulary per its own [PROPOSAL] admission. With no WASM host embedded in any shipped app and no `playlist.save` executor anywhere, the surface is scaffolding compiled into every engine build and used by no one.

## 2. What actually works (verified)
— verified by `cargo test -p plugin-api --lib playlist` → 3 passed, exit 0 (two-chunk mid-line-split parse round-trip; bad-line counted-not-fatal; typed error shape).
— Baseline: cargo-test-workspace.txt:4808 (`Running unittests src/lib.rs (…plugin_api…)`), plugin-api exit 0.

## 3. Findings
F-001 [S3]
Where: crates/plugin-api/src/playlist.rs:156, 184-192
Code:
```
/// A toy `path|title|seconds` line parser.
...
let duration = match secs.and_then(|s| s.parse::<u64>().ok()) {
    Some(ns) => Duration::Known(media_types::Nanoseconds(ns)),
```
What is wrong: the field is documented as SECONDS, the binding is named `secs`, and the parsed u64 is stored DIRECTLY as `Nanoseconds` — 1000 "seconds" becomes 1000 ns (1 µs), a 10⁹ units error. The test passes only because it asserts round-trip consistency of the same wrong mapping (`Known(Nanoseconds(1000))`). Why it matters: this is the crate's ONLY worked example of implementing the parser trait; a developer copying the reference inherits a billion-fold duration bug. What correct looks like: `Nanoseconds(secs * 1_000_000_000)` or document the third field as nanoseconds. Confidence: HIGH — Verification: read + ran (test passes with the wrong mapping, proving the test cannot catch it).

F-002 [S3]
Where: crates/plugin-api/src/playlist.rs:79-128 (both traits)
Code:
```
pub trait PlaylistParser: Send {
...
pub trait PlaylistExporter: Send {
```
What is wrong: zero implementations outside `mod tests` (rg `PlaylistParser|PlaylistExporter` → plugin-api's own kind.rs/manifest.rs/limits.rs references to the KIND enum, plus this file). Zero consumers of `plugin_api::playlist::Playlist`. The trait surface exists to be implemented by WASM extensions — but no shipped binary embeds the WASM host (imports.rs audit F-001), and the `playlist.save` command these exporters would feed has no executor either (rg `PlaylistSave {` repo-wide → only the catalog definition, crates/commands/src/catalog/command.rs:352). Why it matters: a whole vocabulary compiled into the engine for a lane that is dead at both ends. Confidence: HIGH — Verification: rg sweeps (quoted).

F-003 [S4]
Where: crates/plugin-api/src/playlist.rs:131-140
Code:
```
pub fn bad_entry_uri(line: &str) -> EngineError {
    EngineError::new(
```
What is wrong: dead helper — no non-test caller; and it clones the full offending line into the error source (`BadUriLine(line.to_owned())`), which can be an unbounded user line (the bounded-recovery contract elsewhere caps things; a 100 MiB playlist line would be cloned whole). Bounded in practice by the caller that doesn't exist. Confidence: HIGH — Verification: rg + read.

F-004 [S4]
Where: crates/plugin-api/src/playlist.rs:4-9
Code:
```
//! The domain crate `playlists` (L2) owns the rich
//! `playlists::PlaylistParser` / `PlaylistExporter` traits per doc 06
//! §1.2; the minimal-but-real host traits staged here [PROPOSAL:
```
What is wrong: TWO `Playlist` structs now exist (`plugin_api::playlist::Playlist` and `playlists::types::Playlist` at crates/playlists/src/types.rs:21) with different entry types — the documented [PROPOSAL] admits the duplication but the reconciliation (doc 06 §10) has not happened; drift is unguarded (no test ties the two vocabularies). Confidence: HIGH — Verification: read + rg.

## 4. Per-function table (>100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Playlist::new/push/len/is_empty | 44-67 | REAL | — | trivial, test-exercised; `new()` + derived `Default` duplication |
| PlaylistParser::next_entries (trait) | 79-91 | REAL (contract) / UNIMPLEMENTED | S3 | no impls outside tests (F-002) |
| ParseOutcome | 95-108 | REAL | — | honest recovery vocabulary; exercised by toy impl |
| PlaylistExporter (trait) | 119-128 | REAL (contract) / UNIMPLEMENTED | S3 | same (F-002) |
| bad_entry_uri | 131-140 | REAL / dead | S4 | typed but uncalled, unbounded line clone (F-003) |
| BadUriLine Display/Error | 142-150 | REAL | — | fine |
| tests::LineParser | 157-216 | SHALLOW | S3 | reference impl carries the units bug (F-001) |
| tests::LineExporter | 219-239 | REAL | — | fine as a toy |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| parse_then_export_round_trips_uris | YES (structure) / NO (units) | mid-line chunk split + titles/durations asserted — but duration asserted in the same wrong ns mapping (F-001) |
| bad_lines_are_counted_not_fatal | YES | Recovered{skipped:1} + good-line survival asserted (ran: ok) |
| bad_uri_error_is_typed | YES | stage + kind asserted (ran: ok) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Both kinds may load as WASM extensions in v1 (§1.4)" | playlist.rs:10 | FALSE in shipped builds | no app embeds plugin-host-wasm (imports.rs F-001); nothing can load any extension |
| "minimal-but-real host traits staged here" | playlist.rs:6-7 | SHALLOW | traits real, zero implementors/consumers (F-002) |
| "the engine expands lazily and guards cycles" (playlist→playlist refs) | playlist.rs:31-33 | UNVERIFIED | no code in this crate or plugin-host-wasm implements cycle guarding for playlist refs |

## 7. Dead / duplicated / unreachable code
Both traits + `bad_entry_uri` are dead repo-wide (F-002/F-003). Duplicate `Playlist` vocabulary vs crates/playlists (F-004).

## 8. Dependencies & flags
media-types (L0) only. No features. No unsafe. plugin-api IS in the release lane (demux → plugin-api), so this dead surface ships in every engine binary.

## 9. Open questions for the next agent
- `playlist.save`/`playlist.load` catalog rows: they have UI bindings (ui/src/generated/commands.ts) and NO executor found by rg — confirm which typed error the dispatch returns (likely belongs to the commands-catalog agents' F-lists).
- If the Lead ratifies the [PROPOSAL], does plugin-api's Playlist survive, or does crates/playlists win? Today the engine's real m3u path (playlists::types::to_m3u_string) never touches this file.
