# AUDIT: crates/plugin-api/src/playlist.rs
| Field | Value |
|---|---|
| Auditor | W12-A08 |
| Date | 2026-11-21 |
| Lines / functions | 324 / 7 API items (2 traits ×3 methods, 1 enum, 1 fn, 2 structs, 1 error type) + 2 toy test impls + 3 tests |
| File verdict | UNREACHABLE (real trait/vocabulary definitions, fully compiled and self-tested — zero implementors and zero consumers anywhere in the tree) |
| Reachable from shipped entry point? | NO. `rg "ParseOutcome\|PlaylistParser\|PlaylistExporter\|bad_entry_uri" --glob '!crates/plugin-api/src/playlist.rs'` → hits only in docs (ROADMAP.md:101, doc 06 §1.2 rows, worklog), plugin-api's own kind.rs (doc comments + the PluginKind taxonomy), and plugin-host-wasm/src/limits.rs:212 (keys `ResourceLimits` by the `PluginKind` ENUM, not by these traits). No `impl PlaylistParser`/`impl PlaylistExporter` exists outside this file's own test module. Nothing in apps/cli, apps/web, apps/desktop, playback-engine imports `plugin_api::Playlist*`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg "plugin_api::(Playlist\|…)"` crates (no consumers) · `rg "impl PlaylistParser" crates apps` (only this file's test toy) · `rg "pub trait PlaylistParser\|pub trait PlaylistExporter" crates/playlists/src` (NO such traits in the domain crate — free `parse()` fns in m3u.rs:114, pls.rs:193, xspf.rs:268, mpd/hls/cue) · `rg "playlists" --glob Cargo.toml crates apps tests tools` (consumed only by crates/streaming) · `rg "PlaylistSave" --glob '!docs/**'` (catalog spec.rs:525, command.rs:352, ids.rs:148, control-protocol policy.rs:41, ui generated TS — and NO engine handler) · `rg "ParseStatus" crates/plugin-api/src/subtitle.rs` (duplicate vocabulary — 50-66) · baseline: plugin-api suite green (`orchrun-test-workspace-nofailfast.log.txt`, crate's 111-test neighbor run; INDEX #3) · `sed -n '1,10p;130,140p'` quote verification |

## 1. One-paragraph verdict
The file defines a clean, honest, streaming-shaped playlist contract: chunk-fed `PlaylistParser` with a sink and a bounded-recovery `ParseOutcome`, a three-method `PlaylistExporter`, a minimal `Playlist`/`PlaylistEntry` vocabulary, and one typed error helper — all lint-clean, all exercised by toy impls whose tests genuinely bite (mid-line chunk split, bad-line recovery, typed error). It is also completely dead: not one type, trait, or function here has an implementor or consumer outside its own test module. The module doc's justification — that the domain crate `playlists` owns the "rich" traits — is false (that crate has free `parse()` functions and no traits at all), the `playlist.save` command these traits exist to back has no engine handler anywhere (catalog vocabulary + generated TS types only), and the repo's real playlist parsing (six formats in the playlists crate) flows through none of this — it feeds only the streaming crate's HLS lane. This is contract theater: a plugin-kind vocabulary for plugins that do not exist, consumed by a command that nothing dispatches.

## 2. What actually works (verified)
— Trait shape + tests: `parse_then_export_round_trips_uris` (242-290) feeds the document split at byte 28 — before the first `\n`, so the partial-line buffering path in the toy parser is genuinely exercised; mutation check: if `next_entries` dropped the partial tail (174-177), the first entry would be lost and `playlist.len()==2` (260) fails. `bad_lines_are_counted_not_fatal` (293-316) pins the `Recovered{skipped:1}` contract; if bad URIs became fatal errors the assert on `Recovered` fails. `bad_uri_error_is_typed` (318-323) pins stage+kind. All green in the baseline workspace test run (INDEX #3; plugin-api suite; lumen_web-independent).
— `bad_entry_uri` (131-140): correctly typed EngineError (stage Open, Corrupt/InvalidStructure, context `plugin.playlist.bad-entry-uri`, source chain via `BadUriLine`). No panics.

## 3. Findings

### F-001 [S3] Module doc asserts the domain crate owns "rich" `playlists::PlaylistParser`/`PlaylistExporter` traits — those traits do not exist anywhere
Where: crates/plugin-api/src/playlist.rs:4-5
Code:
```rust
//! The domain crate `playlists` (L2) owns the rich
//! `playlists::PlaylistParser` / `PlaylistExporter` traits per doc 06
//! §1.2; the minimal-but-real host traits staged here [PROPOSAL:
```
What is wrong: `rg "pub trait PlaylistParser|pub trait PlaylistExporter" crates/playlists/src` → zero hits. The playlists crate exposes free functions (`m3u.rs:114 parse()`, `pls.rs:193`, `xspf.rs:268`, plus mpd/hls/cue) and its own `Playlist` struct — no parser/exporter traits. Doc 06 §1.2 rows 12-13 (docs/06-plugin-architecture.md:65-66) name `playlists::PlaylistParser` and `playlists::PlaylistExporter` as the domain traits — the same fiction.
Why it matters: A plugin author reading this contract module is told the "rich" counterpart lives one crate away; it does not. The doc-vs-code contradiction is exactly the class a hostile audit exists to catch: the §1.2 trait-placement table (the taxonomy's backbone) is wrong for kinds 12-13, and this file's header repeats the error.
What correct looks like: State the truth: the domain crate parses via free functions today; these staged traits have no counterpart and no implementors; doc 06 §1.2 rows 12-13 need the same correction (flag to wave-19 docs auditor).
Confidence: HIGH — Verification: read + repo-wide rg.

### F-002 [S3] The entire vocabulary is dead in-tree: zero implementors, zero consumers — and the `playlist.save` command it claims to back has no handler
Where: crates/plugin-api/src/playlist.rs:1-2, 110-111 (claims) — with crates/commands/src/catalog/spec.rs:525 and the repo-wide absence of a dispatcher
Code:
```rust
//! The playlist kinds (doc 06 §1.2 rows 12–13): playlist parsing
//! (URI/drop → entries) and export (`playlist.save`, doc 05 §2.7).
```
What is wrong: `playlist.save` exists as command-catalog vocabulary (spec.rs:525, command.rs:350-356, ids.rs:148, control-protocol policy.rs:41, ui/src/generated/commands.ts:64) but no engine code handles `CommandId::PlaylistSave` — `rg "PlaylistSave"` outside catalog/policy/generated → nothing. No crate implements `plugin_api::PlaylistParser`/`PlaylistExporter` (only this file's test toys). The `Playlist`/`PlaylistEntry` types are consumed by nobody. Meanwhile the repo's real playlist parsing (playlists crate: m3u/pls/xspf/mpd/hls/cue) uses none of this vocabulary and is reachable only via crates/streaming (HLS) — not from any app.
Why it matters: The file is the plugin-contract face of a user-facing feature that has no implementation path: a `playlist.save` request would reach a catalog entry with no dispatcher, and a hypothetical WASM playlist extension implementing these traits would be loaded by a host (plugin-host-wasm) that no shipped binary depends on. Dead contract surface drifts silently — the L2 `playlists::Playlist` (types.rs:21) and this `Playlist` will diverge with no compiler noticing.
What correct looks like: Either wire it (an engine-side playlist model + a `playlist.save` handler consuming `PlaylistExporter`) or mark the module `#[doc = ..]`-honestly as staged-for-future and drop it from the headline re-exports until it has a consumer.
Confidence: HIGH — Verification: read + three repo-wide rg sweeps (trait impls, symbol consumers, command handler).
### F-003 [S4] Duplicate recovery vocabulary and duplicate Playlist concept inside the same crate/repo
Where: crates/plugin-api/src/playlist.rs:95-108 (with crates/plugin-api/src/subtitle.rs:50-66, crates/playlists/src/types.rs:21)
Code:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseOutcome {
    /// Chunk accepted; entries (if any) went to the sink.
    Accepted,
    /// Document fully parsed.
    Drained,
    /// Damage overcome; `skipped` counts entries lost.
    Recovered {
```
What is wrong: `ParseOutcome` (Accepted/Drained/Recovered{skipped,reason}) is shape-identical to subtitle.rs's `ParseStatus` (Accepted/Drained/Recovered) — same crate, two enums, two names. Separately, `plugin_api::Playlist` duplicates the concept of `playlists::Playlist` (types.rs:21) with a different shape.
Why it matters: Two same-shaped enums in one contract crate guarantee drift the first time one grows a variant; two Playlist types across L1/L2 guarantee drift in the opposite direction (the L2 type carries the richer metadata). A plugin author must now guess which vocabulary the eventual registry will speak.
What correct looks like: One shared bounded-recovery enum in plugin-api (or media-types) used by both parser traits; one Playlist vocabulary (or an explicit conversion) documented as the single source of truth.
Confidence: HIGH — Verification: read of both files' enum definitions.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Playlist` struct | 21-26 | REAL (dead in-tree) | S3 | F-002. Derives Default; positions meaningful. |
| `PlaylistEntry` struct | 29-42 | REAL (dead in-tree) | S3 | uri/title/duration — `Duration::Unknown` vs Known typed absence (MP §8) — good shape. |
| `Playlist::new` | 46-50 | REAL | — | Trivial; duplicates `Default` derive. |
| `Playlist::push` | 44-56 | REAL | — | The only construction path (ordered by construction) — honest doc. |
| `Playlist::len` / `is_empty` | 59-66 | REAL | — | Clippy pair complete. |
| `PlaylistParser::next_entries` (trait) | 79-91 | REAL (0 impls outside tests) | S3 | Sink-based, chunk-bounded, `None`=EOF; `Send` bound; no `&self` (per-job instance scope per doc). Sound design, no consumer. |
| `ParseOutcome` enum | 95-108 | REAL | S4 | F-003 duplicate of subtitle's ParseStatus. |
| `PlaylistExporter` trait (begin/write_entry/finish) | 119-128 | REAL (0 impls outside tests) | S3 | Document/IO split mirrors Access — coherent; no consumer. |
| `bad_entry_uri` | 131-140 | REAL (dead) | S3 | Zero callers outside its test. Correctly typed error construction. |
| `BadUriLine` + `Display` + `Error` | 142-150 | REAL | — | Private source type; fine. |
| Test `LineParser` | 157-216 | REAL (toy, honestly labeled) | — | Exercises the trait incl. partial-line buffering + skip-count recovery. |
| Test `LineExporter` | 219-239 | REAL (toy, honestly labeled) | — | `entry.uri.raw()` with a documented redaction note. |
| Test `parse_then_export_round_trips_uris` | 241-290 | REAL | — | Bites (mutation analysis in §2). |
| Test `bad_lines_are_counted_not_fatal` | 292-316 | REAL | — | Bites. |
| Test `bad_uri_error_is_typed` | 318-323 | REAL | — | Weak-ish (stage+kind only, no source-chain assert) but sufficient. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `parse_then_export_round_trips_uris` | YES | 2-entry round-trip with mid-line chunk split; asserts titles, durations (both entries), and the exported bytes. Break buffering ⇒ len!=2; break export ⇒ bytes mismatch. |
| `bad_lines_are_counted_not_fatal` | YES | Asserts exact `Recovered{skipped:1, reason:"unparsable entry URI"}` AND that the good entry survived (count==1). |
| `bad_uri_error_is_typed` | YES (narrow) | Stage + ErrorKind asserts; would not catch a wrong context string or source chain. |
| Toy impls | — | The subject under test is a trait; a toy impl is legitimate here — but it means the tests prove NOTHING about any real parser (none exists: that is F-002, not test dishonesty). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The domain crate `playlists` (L2) owns the rich `playlists::PlaylistParser` / `PlaylistExporter` traits" | playlist.rs:4-5; doc 06 §1.2 rows 12-13 | FALSE | No such traits in crates/playlists (rg); free parse() fns only. F-001. |
| "export (`playlist.save`, doc 05 §2.7)" — this vocabulary backs it | playlist.rs:2, 111 | FALSE (as wiring) | `playlist.save` has catalog vocabulary + generated TS but no engine handler (rg PlaylistSave). F-002. |
| "Both kinds may load as WASM extensions in v1 (§1.4)" | playlist.rs:10 | REAL-UNPROVEN | kind.rs marks the kinds extension-allowed; no extension of any kind loads in any shipped binary (wave-12 hostile question; plugin-host-wasm is an app-dependency of nothing). |
| "a bad line is data, not a failed document" (bounded recovery) | playlist.rs:76-78 | REAL (trait-level) | Toy-impl test pins `Recovered{skipped}`; no real parser exercises it. |

## 7. Dead / duplicated / unreachable code
- The entire public surface (F-002): dead in-tree, kept alive only by its own tests and the lib.rs/prelude re-exports.
- Duplicates: `ParseOutcome` vs subtitle.rs `ParseStatus`; `Playlist` vs playlists::types::Playlist (F-003).

## 8. Dependencies & flags
- Only media-types types (SourceUri, SharedBytes, Duration, EngineError, PipelineStage) — matches the crate's L1-deps-only claim. No features gate this module.
- Compiled unconditionally in the workspace build (INDEX #1 CLEAN) — i.e., every shipped `lumen` binary carries this dead vocabulary in plugin-api's rlib.

## 9. Open questions for the next agent
- Wave-19 (docs): doc 06 §1.2 rows 12-13 trait-placement column is FALSE for playlist kinds (F-001) — same audit should check rows 6-7 (filter traits) and 16 (discovery) for the same staged-trait fiction.
- A07 (manifest.rs part2): if a playlist-parser extension manifest ever installs, `Capabilities::validate` requires `formats` — cross-check that against the playlists crate's format names (m3u/xspf/pls/mpd/hls/cue) so a future registry and the real parsers agree on spellings.
- Whoever audits crates/playlists: confirm the free-function lane's error taxonomy has no path back into `EngineError` (this crate's sink contract) — today there is no bridge, which is the drift F-003 predicts.
