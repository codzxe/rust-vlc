# AUDIT: crates/media-probe/src/report.rs
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-21 |
| Lines / functions | 475 / 30 items (1 const, `FormatId` + 16 constructors + parse/as_str/Display, `Confidence` ×4, `TrackHint::new`, 3 structs, `SourceMeta` ×2, `extension_of`, 7 tests + 3 test helpers/trait) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES. `lumen open` → `apps/cli/src/demux_host.rs:444-446` `media_probe::ProbeRegistry::with_builtins()` + `probe_stream(...)` → this file's `ProbeReport`/`FormatId`/`Confidence`/`SourceMeta`/`TrackHint` are the probe lane's vocabulary; the report's `format` string then drives `apps/cli/src/containers/mod.rs:50-75`. Executed: `report::tests` 7/7 in baseline (orchrun-test-workspace-nofailfast.log.txt:3309 block: format_id ×3, confidence, track_hint, source_meta, probe_report — all ok) and every shipped open (AVI first-vertical session, MP4 smoke open, INDEX.md) ran through this vocabulary. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Full-file Read (475 lines) · `rg -n "from_score\|score" crates/media-probe/src/signature/mod.rs` (registry enforces the 0..=100 range at collection :181-187 — the `ProbeHit.score` soft boundary is enforced one step away) · Read `signature/mod.rs:178-220` (winner selection + tie-break + min_score gate) · `rg -n "media_probe\|ProbeReport" apps/cli/src/demux_host.rs` (:444-446 — shipped wiring) · `rg -n "pub fn register\|with_builtins"` + `rg "register\(" crates apps` (registry is extensible via `register` :150; the CLI registers builtins only) · per-constructor usage sweep: `rg -c "FormatId::<m>\(\)"` for all 16 ids (every id referenced by signature modules — no dead constructors) · Read `apps/cli/src/containers/mod.rs:50-75` (the raw-string dispatch the vocabulary feeds) · baseline `report::tests` + registry/es/avi test blocks verified executed. |

## 1. One-paragraph verdict
A small, disciplined vocabulary module doing exactly what its docs promise: an open validated `FormatId` newtype with a grammar-checked extension path, a `Confidence` type whose constructor rejects rather than clamps out-of-range scores, `TrackHint`/`ProbeHit`/`ProbeReport`/`SourceMeta` records with honest absent-vs-present typing, and a deliberately naive `extension_of` whose every edge I traced (hidden files, double suffixes, trailing slashes, percent-encoded dots, uppercase, query strings) behaves as documented and as tested. The type-level guarantees are real but two of them delegate their last mile elsewhere: `ProbeHit.score` is a raw `u8` public field validated only when the registry accepts the hit (verified: signature/mod.rs:181-187 filters through `Confidence::from_score`), and the "nothing allocates unboundedly" house rule holds only because in-tree signatures bound their hint vectors — the public `ProbeRegistry::register` accepts any plugin whose `ProbeHit.track_hints` Vec passes through to the report uncapped. The deeper hygiene defect is that this vocabulary was designed to be the single source of format-identity truth yet the shipped open path re-matches raw strings against it — the ASF family (fully implemented demuxer, zero wiring, matrix still claiming `implemented` on two platforms) is the proof that nothing enforces vocabulary-to-wiring coherence.

## 2. What actually works (verified)
— `FormatId::parse` grammar (:135-144): non-empty, ≤24 bytes, `[a-z0-9-.]` only — pinned by `format_id_valid_shapes_parse` / `format_id_rejects_bad_shapes` (both assert the typed `probe.format-id.invalid` code and stage). Executed (orchrun :3309 block).
— `Confidence::from_score` (:178-184): rejects 101..=255 (`Option`, never clamped) — pinned by `confidence_bounds_are_enforced_not_clamped` incl. `from_score(255).is_none()`. The registry's enforcement site verified by read (signature/mod.rs:187 — out-of-range hits are silently dropped from collection, exactly as the doc claims).
— All 16 builtin format ids match the wired registry one-for-one (avi mp4 wav aiff ogg matroska adts mpegts mpegps ivf mp3 mp2 h264 hevc flac) with `annexb` as the documented 17th sub-threshold family id that never reaches the registry — cross-checked against `apps/cli/src/containers/mod.rs:50-73` (15 arms + the es module's multi-id arms) and `containers/mod.rs:105-107`'s own registry-inventory test comment. Constructor-usage sweep: all 16 referenced by signature modules (no dead code).
— `extension_of` (:325-340): traced 10 edge cases against the code — query/fragment stripped, trailing slash → None, `.hidden` → None, `.config.toml` → toml, `archive.tar.gz` → gz, uppercase lowered, >16 chars → None, non-alphanumeric → None, percent-encoded dot (`movie%2Emp4`, no literal `.`) → None. `source_meta_extension_extraction` pins 6 of them — executed.
— `SourceMeta::size: Option<u64>` — the absent-vs-zero house rule, structurally enforced.
— Shipped execution: every `lumen open` (AVI first-vertical full lifecycle, INDEX.md §contrast; MP4 smoke opens, INDEX.md §MP4 SMOKE TEST) flows through `ProbeReport` → format dispatch.

## 3. Findings

### F-001 [S4]
Where: crates/media-probe/src/report.rs:24-33
Code:
```
/// An **open** validated newtype, the [`Scheme`](media_types::ids::Scheme)
/// house pattern: formats are plugin data (each demuxer manifest names
/// the formats it serves), so the vocabulary is a string, not a closed
/// enum — but every id is grammar-checked at construction. The builtin
/// signatures mint their ids through [`FormatId::avi`] /
/// [`FormatId::mp4`]; extension manifests go through
/// [`FormatId::parse`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FormatId(String);
```
What is wrong: The type is documented as the shared format vocabulary, but nothing binds the three places a format id lives: the signature that mints it (here), the CLI dispatch that matches it (`apps/cli/src/containers/mod.rs:50-73` re-matches raw string literals), and the demuxers' own `EXTENSIONS` tables. Coherence rests entirely on cross-crate test discipline (the signature module's "round-trip suite", signature/mod.rs:54). The ASF/WMV family is the live proof of the gap: `crates/demux/src/asf/**` is fully implemented and 37-test-executed, yet no `FormatId::asf()` constructor exists, no signature mints one, no registry arm dispatches one — so `lumen open file.wmv` probe-refuses (`probe.no-demuxer`, the compat-matrix note in feature-matrix.toml:217) while the matrix still carries `windows = "implemented", macos = "implemented"` (my asf/header.rs#part1 audit, §6).
Why it matters: The open-newtype design was chosen precisely so vocabulary and wiring could not drift; today a demuxer card can land "complete" and be unreachable, and only a human-run compat matrix notices (which is what happened — issue #53's triage note). A compile-time or test-time exhaustiveness pin (a `const BUILTIN_FORMAT_IDS: &[FormatId]` that a containers-registry test must cover, or per-family wiring tests keyed off the id list) would convert this class of silent gap into a build failure.
What correct looks like: One canonical list of builtin ids (this crate), consumed by: a media-probe test asserting every builtin id has a signature, and a CLI-side test asserting every builtin id (minus the documented sub-threshold `annexb`) has a containers arm — the ASF gap then fails a test, not a user.
Confidence: HIGH — Verification: read; the ASF evidence chain verified in my asf/header.rs#part1 audit (rg zero-hits for asf in media-probe and containers; 37/37 asf tests in baseline).

### F-002 [S4]
Where: crates/media-probe/src/report.rs:9-12
Code:
```
//! [`Result`](std::result::Result); nothing here allocates
//! unboundedly (the only owned payloads are short id strings and the
//! bounded hint vector a signature produced while scanning a
//! window-bounded prefix).
```
What is wrong: The unbounded-allocation exemption is delegated: `ProbeHit.track_hints: Vec<TrackHint>` (:256) is a public field any `ProbePlugin` fills, and the registry passes it to the report verbatim (`track_hints: hit.track_hints`, signature/mod.rs:204) with no length cap. `ProbeRegistry::register` is public (signature/mod.rs:150), so a third-party (or future in-tree) signature returning a million-hint vector breaks the stated house rule; only in-tree signatures' self-restraint (and the CLI's builtins-only registry, demux_host.rs:444) keeps the claim true today.
Why it matters: A documented invariant that holds by convention instead of by construction is exactly the drift the module's own "House rules (MP §8)" header exists to prevent; the fix is one saturating cap at the registry seam.
What correct looks like: In the registry's collection loop, cap accepted hits' `track_hints` (e.g. `truncate(N)` with a damage-style note, or reject hits with more than N hints as contract violations — the same posture as out-of-range scores).
Confidence: HIGH — Verification: read; registry pass-through verified at signature/mod.rs:204; CLI registers builtins only (demux_host.rs:444) — no in-tree violation exists (the defect is the missing enforcement, not an active overrun).

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (house rules) | 1-12 | REAL | S4 | F-002 (the unbounded claim delegates to signature discipline). |
| `CONFIDENCE_MAX` = 100 | 20-21 | REAL | — | Matches doc 03 §4.2. |
| `FormatId` newtype | 32-33 | REAL | S4 | F-001 (vocabulary not binding wiring). |
| 16 builtin constructors (`avi`…`flac`) | 36-124 | REAL | — | All referenced by signatures (usage sweep); names match the CLI registry 1:1 + documented `annexb` sub-threshold exception. No `asf` — consistent with the ASF unwired state (see F-001). |
| `FormatId::parse` | 135-144 | REAL | — | Grammar-checked; typed `probe.format-id.invalid`. Executed both branches. |
| `FORMAT_ID_MAX_BYTES` / `Display` | 152-159 | REAL | — | 24-byte cap; Display = the raw string. |
| `Confidence` newtype | 168-169 | REAL | — | Validated-only construction. |
| `Confidence::from_score` | 178-184 | REAL | — | Reject-not-clamp; registry enforcement verified downstream. |
| `Confidence::as_u8` / `meets` / `Display` | 187-201 | REAL | — | `meets` is ≥; Display `95/100` pinned by test. |
| `TrackHint` struct | 212-222 | REAL | — | kind + validated descriptor. |
| `TrackHint::new` | 231-236 | REAL | — | Typed `probe.hint-codec-invalid` on bad descriptors — executed. |
| `ProbeHit` struct | 247-260 | REAL | — | `score: u8` raw-by-design (registry validates — verified); doc says so at :251-253. |
| `ProbeReport` struct | 269-281 | REAL | — | `confidence` is the validated type; `bytes_examined` makes the depth bound observable. |
| `SourceMeta` struct | 290-298 | REAL | — | Option-typed size (house rule). |
| `SourceMeta::new` / `from_stream` | 300-322 | REAL | — | from_stream: live size + naive extension parse (documented as deliberate). |
| `extension_of` | 325-340 | REAL | — | 10 edge cases traced by hand (§2); tests pin 6. |
| tests `ok`/`err` helpers | 351-363 | REAL | — | Test-only `unreachable!` unwraps on literals — documented rationale (testkit Must inlined). |
| 7 tests + `MustConf` trait | 365-474 | REAL | — | All executed in baseline (orchrun :3309 block), all would fail under the mutations they target (grammar, bounds, descriptor validation, extension edges, report shape). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `format_id_valid_shapes_parse` / `format_id_rejects_bad_shapes` | YES | Accept/reject tables incl. uppercase, whitespace, tab, overlength; error code+stage asserted. |
| `builtin_format_ids_are_the_documented_names` | YES (weakly) | Only checks avi/mp4 against parse round-trips — would NOT catch a typo in the other 14 constructors (e.g. `FormatId::mpegts()` returning "mpeg-ts"). Cross-crate coherence is pinned only by the signature module's own tests. Feeds F-001. |
| `confidence_bounds_are_enforced_not_clamped` | YES | 0/100/101/255 + meets boundary — kills clamp mutations and off-by-one. |
| `track_hint_validates_its_descriptor` | YES | Empty descriptor refused typed. |
| `source_meta_extension_extraction` | YES | 6 edges incl. query string, dotfile, double suffix, trailing slash. |
| `probe_report_carries_the_card_shape` | YES | Field-by-field equality incl. Display format. |
All 7 executed, 0 failed (orchrun :3309 block).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "a value and its absence are different types (`SourceMeta::size` is an `Option`, never a `0` sentinel)" | module docs :6-8 | REAL | Struct field is `Option<u64>`; no sentinel anywhere in the module. |
| "every public constructor validates and returns Result" | module docs :8-9 | REAL **except the 16 builtin `FormatId` constructors** (infallible by literal — documented as "grammar-checked at construction" only via `parse`) | The claim is true of `parse`/`TrackHint::new`; the builtin constructors bypass validation by design (known-valid literals). Cosmetic over-claim; the registry round-trip pins coherence for 2 of 16. |
| "nothing here allocates unboundedly" | module docs :9-12 | REAL (in-tree) / delegated | F-002 — holds while only builtin signatures register; no enforcement at the registry seam. |
| "score: u8 0..=100, range made unrepresentable-wrong" (`Confidence`) | :161-167 | REAL | from_score rejects >100; registry drops out-of-range hits (signature/mod.rs:187, verified by read + `error::tests::confidence_and_hit_construction_stay_in_range` executed). |
| "The extension is a hint, not truth … never the scoring" | :284-289 | REAL | `SourceMeta` rides only the error payload (verified in signature/mod.rs's no-hit error path — `no_demuxer_names_the_best_subthreshold_hit`/`no_demuxer_carries_sniffed_bytes_and_hints` executed). |

## 7. Dead / duplicated / unreachable code
- None dead: all 16 constructors consumed by signature modules (usage sweep in Commands run).
- Duplication note (feeds F-001): format-name strings exist here, in the CLI registry arms, and in demuxers' `EXTENSIONS` tables; media-probe's own signature/avi.rs:51 and mp4.rs:43 comments admit re-implementing demux EXTENSIONS "by hand" to avoid a crate edge.

## 8. Dependencies & flags
- `media_types` (ids, EngineError), `plugin_api::SourceStream` (for `from_stream`) — no feature gates; compiled unconditionally. `crate::contract::CountingSource` used only by tests.
- No unsafe, no panics outside test helpers (all public paths return Result/Option; `extension_of` slicing all guarded).

## 9. Open questions for the next agent
- The signature module (signature/mod.rs — another agent's file) owns the registry seam where F-002's cap belongs and where `better_tiebreak`'s score→priority→prefix ordering lives; my read of :181-220 found it sound — confirm under its own audit.
- If a wave owns `apps/cli/src/demux_host.rs`: the probe→registry→format-id handoff (:444-446) is the shipped seam F-001's exhaustiveness pin would attach to.
