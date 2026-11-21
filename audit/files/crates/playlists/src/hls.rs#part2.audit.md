# AUDIT: crates/playlists/src/hls.rs#part2
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-21 |
| Chunk boundaries | **part2 = lines 331–997** (file is 997 lines; split at the file's own section marker `// ====== 6.B2a: master playlists` at 331 — a functional split: part1 (A09) = 1–330, the 6.B1a media-playlist model + parser; part2 = the entire 6.B2a master-playlist half: model structs, `parse_master_playlist`, attribute machinery, and the whole `tests` module (630–997, which covers both halves). The raw midpoint (498) falls inside `parse_master_playlist`.) |
| Lines / functions | 667 / 20 (7 type defs + 4 fns + 9 tests; `parse_tag_uint` at 324-329 belongs to part1) |
| File verdict | UNREACHABLE for the public functions (REAL, well-tested parser code; compiled into the shipped binary via streaming→cli but no entry point calls it) |
| Reachable from shipped entry point? | **NO.** `parse_master_playlist` ← `crates/streaming/src/hls/master.rs` only (rg repo-wide: no other non-test caller); `master.rs` itself has no app callers (see media.rs#part2 audit). `apps/cli` does not even depend on `playlists` directly (rg "playlists" apps/*/Cargo.toml → nothing); the crate rides in only as streaming's dependency, and the CLI touches streaming only via fetch/ranged/schemes. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `rg -n "BadAttributeValue|HlsPlaylistError::Parse"` → Parse constructed at exactly one site (hls.rs:222, the media path); BadAttributeValue constructed nowhere. `rg -n "parse_master_playlist|parse_media_playlist"` outside playlists → streaming hls modules + one fuzz target only. Fixtures verified to exist (`tests/fixtures/hls/master/{multi,single,malformed}.m3u8`, `ts_vod/pl.m3u8`). Baseline: `orchrun-test-workspace-nofailfast.log.txt:4263-4277` — 14/14 `hls::tests` ok. `sed -n` on every quote. |

## 1. One-paragraph verdict
The master-playlist half of the HLS parser is careful, honest work — spec-cited field docs, a strict attribute splitter (duplicates, unterminated quotes, trailing commas all refused), damaged-row-skips-with-warnings, and a 14/14-green test suite pinning exact values including committed fixtures. But the module's two headline contracts are each broken in one spot: (1) the crate's bounded-input D-1 promise ("lines over 64 KiB are rejected") is enforced only on the media path — `parse_master_playlist` iterates raw `text.lines()` with no cap while its own `# Errors` doc claims the line-cap `Parse` error, a claim that is structurally impossible since that variant has exactly one construction site, in the other function; (2) the "damaged rows are warnings" posture silently swallows two row shapes (a STREAM-INF with no URI at EOF, and a STREAM-INF overwritten by the next) and never emits the `BadAttributeValue` warning its enum doc promises — bad `RESOLUTION`/`AVERAGE-BANDWIDTH` values vanish without record. All of it unreachable from the shipped binary anyway.

## 2. What actually works (verified)
— 14/14 `hls::tests` pass in the citable baseline (`orchrun-test-workspace-nofailfast.log.txt:4263-4277`), including `master_malformed_rows_skip_with_warnings` (4 warning shapes: missing BANDWIDTH, unparseable BANDWIDTH, unterminated quote, trailing comma — asserts 1 surviving variant + 4 warnings) and `master_types_variants_and_renditions` (full attribute surface: AVERAGE-BANDWIDTH, PROGRAM-ID, CODECS with embedded comma inside quotes, RESOLUTION, DEFAULT/AUTOSELECT, GROUP-ID/LANGUAGE, URI resolution).
— `split_attr_list` strictness traced by hand: `"BANDWIDTH=900000,CODECS=\"unterminated"` → `find('"')` on the unterminated tail → `None` → warning (test 935-936 proves); duplicate names → `None` (608-610); trailing comma → `None` (621); junk after closing quote → `None` (624). No panic path found on any byte input I traced (all indexing is `find`-based).
— The `RenditionType::Other(String)` design records unknown TYPE tokens verbatim instead of refusing — verified by `master_single_variant_and_uri_less_rendition` (CLOSED-CAPTIONS + INSTREAM-ID rows).

## 3. Findings

### F-001 [S2] — `parse_master_playlist` has no line cap while claiming one; the crate's D-1 bounded-input invariant does not hold on this path
Where: hls.rs:449-453 (doc claim) vs 454-530 (no cap anywhere in the body); single `Parse` construction at 222
Code:
```rust
/// # Errors
/// [`HlsPlaylistError`] — `NotHls` (no `#EXTM3U` anywhere),
/// `MediaPlaylist` (segment lines/`#EXTINF` where variant rows were
/// expected), `Parse` (line-cap violation). Damaged rows are
/// warnings, not errors (dirty-world posture).
pub fn parse_master_playlist(
```
What is wrong: the doc lists `Parse` (line-cap violation) among this function's errors, but the function never constructs it — `HlsPlaylistError::Parse` has exactly one construction site, in `parse_media_playlist` (line 222, via `parse_detailed`, which is what enforces `MAX_LINE_BYTES`). The master parser iterates `text.lines()` raw: a master playlist with one multi-megabyte `#EXT-X-STREAM-INF:…` line is processed in full (`split_attr_list` allocates over the whole line). The crate's own lib.rs contract ("Bounded input (D-1) — lines over MAX_LINE_BYTES (64 KiB) are rejected with a typed error; nothing unbounded is processed") is therefore false for this public entry point.
Why it matters: this is a public parser for untrusted network input. Today the only caller (streaming's master session) bounds total input by a response budget, so no OOM — but the crate-level invariant that makes the parser safe to call standalone is broken, and the doc points a reviewer at a guarantee that does not exist.
What correct looks like: run the same line-cap check `parse_detailed` performs (or route through it) before the loop, returning `HlsPlaylistError::Parse` on a >64 KiB line — and then the doc becomes true.
Confidence: HIGH — Verification: read + rg (single construction site proven); no test feeds an over-cap line to the master parser (the media-path cap test lives in m3u.rs's suite).

### F-002 [S3] — two damaged-row shapes are swallowed silently, violating the "damaged rows are warnings" posture
Where: hls.rs:474-482 (overwrite) and 511-524 (EOF drop)
Code:
```rust
        if let Some(attrs) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            match split_attr_list(attrs) {
                Some(list) => pending_stream_inf = Some((line_no, list),
```
(and, at loop end:)
```rust
        // A URI with no pending STREAM-INF is damage the row-skip
        // posture tolerates silently only when variants exist; with
        // none at all the media-shape refusal below stays decisive.
    }
```
What is wrong: (a) a `#EXT-X-STREAM-INF` followed to EOF by no URI line leaves `pending_stream_inf` set when the loop exits — the row is dropped with **no warning and no variant**; (b) a second `#EXT-X-STREAM-INF` while one is pending replaces the first (`pending_stream_inf = Some(...)` unconditionally at 476) — the first row's attributes vanish silently. Both are exactly the "damaged row" class the function's contract (`Damaged rows are warnings, not errors`) promises to record via `MasterWarning`.
Why it matters: a truncated master playlist (very realistic on a dropped connection) loses its last variant with zero diagnostic — the caller cannot distinguish "1 variant by design" from "2 variants, one silently eaten", which matters for ladder selection.
What correct looks like: after the loop, `if let Some((line_no, _)) = pending_stream_inf.take() { warnings.push(...VariantWithoutBandwidth) }`; on overwrite, warn for the displaced row before replacing.
Confidence: HIGH — Verification: read + hand-trace; no test covers either shape (`master_malformed_rows_skip_with_warnings` covers only the four warn-emitting shapes).
What correct looks like: see above.

### F-003 [S3] — `MasterWarningKind::BadAttributeValue` is dead; bad attribute values are dropped silently, not "with a warning"
Where: hls.rs:436-441 (definition + doc) vs variant_from_attrs 544-553 (silent drop)
Code:
```rust
    /// A well-formed attribute value this model does not type (e.g.
    /// a non-`WxH` `RESOLUTION`): the attribute is dropped, the row
    /// survives.
    BadAttributeValue,
```
(and the actual behavior:)
```rust
    let average_bandwidth = find("AVERAGE-BANDWIDTH").and_then(|v| v.trim().parse::<u64>().ok());
    let program_id = find("PROGRAM-ID").and_then(|v| v.trim().parse::<u64>().ok());
```
What is wrong: the enum variant is never constructed anywhere in the crate (rg: zero construction sites). A malformed `RESOLUTION=abc`, a non-numeric `AVERAGE-BANDWIDTH`, or a bad `PROGRAM-ID` causes the attribute to become `None` with **no warning** — while the variant's doc explicitly describes warning behavior ("the attribute is dropped, the row survives" as a `MasterWarning` kind). `variant_from_attrs` returns only `Option<Variant>` and has no channel to report the dropped attribute.
Why it matters: the doc promises diagnostics that do not exist; a playlist author's typo (RESOLUTION=1920x — note also capital-`X` is dropped silently per the strict `split_once('x')`) disappears without trace, undermining the dirty-world observability this crate sells.
What correct looks like: either thread a warnings sink through `variant_from_attrs`/`rendition_from_attrs` and emit `BadAttributeValue` on each dropped attribute, or delete the variant and fix the doc to say "silently dropped".
Confidence: HIGH — Verification: read + rg (zero construction sites).

### F-004 [S4] — `INSTREAM-ID` (REQUIRED for CLOSED-CAPTIONS renditions, RFC 8216 §4.3.4.1) is not modeled
Where: hls.rs:384-402 (Rendition model) vs the test at 908 that feeds `INSTREAM-ID="CC1"`
Code:
```rust
pub struct Rendition {
    /// `TYPE` — the spec's enumerated set, with unknown values kept
```
What is wrong: the `Rendition` model has no `instream_id` field; the attribute is parsed by `split_attr_list` and then discarded in `rendition_from_attrs` (only TYPE/GROUP-ID/NAME/LANGUAGE/DEFAULT/AUTOSELECT/URI are read). RFC 8216 marks `INSTREAM-ID` REQUIRED for `TYPE=CLOSED-CAPTIONS` rows — the model cannot represent the one thing a CC row exists to declare.
Why it matters: limited blast radius (the matrix itself records "alternate renditions parsed+recorded but not selected"), but the "no invented metadata / record what was said" posture has a hole exactly where the spec is strictest.
What correct looks like: `pub instream_id: Option<String>` on `Rendition`, populated like `group_id`.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MasterPlaylist`/`Variant`/`Resolution`/`Rendition`/`RenditionType` (types) | 342-417 | REAL | S4 (F-004) | Data models, honest field docs; order-preserving; no mutation in the model (selection policy documented as the streaming crate's). |
| `MasterWarning`/`MasterWarningKind` | 422-443 | REAL | S3 (F-003) | One dead variant with a false doc. |
| `parse_master_playlist` | 454-530 | REAL | S2 (F-001) + S3 (F-002) | No-panic loop (all `find`/`strip_prefix` based); pending-row state machine correct for the happy path; silent-loss holes F-002; no line cap F-001. Media-shape refusal requires zero variants AND zero renditions AND `saw_media_shape` (EXTINF/TARGETDURATION prefix) — hand-traced, matches the mirror test. |
| `variant_from_attrs` | 534-566 | REAL | S3 (F-003 silent drops) | BANDWIDTH strictly required (`?` on parse) → skip+warn upstream; other bad values silently `None`. URI resolution via `resolve_uri`. |
| `rendition_from_attrs` | 570-593 | REAL | S4 (F-004) | TYPE required; unknown TYPE → `Other` verbatim (good); `DEFAULT`/`AUTOSELECT` strict `== "YES"`; URI resolved. |
| `split_attr_list` | 599-628 | REAL | — | Strict per RFC 8216 §4.2: duplicate names, unterminated quotes, trailing commas, junk-after-quote, bad name charset all → `None`. Hand-traced four hostile shapes. |
| `tests` (14 fns, 630-997) | 630-997 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `vod_playlist_types_and_resolves` / `no_base_keeps_locations_verbatim` / `uri_resolution_shapes` / `plain_m3u_refuses_not_hls` / `master_playlist_refuses_typed` / `bad_numeric_values_refuse_typed` / `model_records_not_refuses_b1b_shapes` / `dirty_world_warnings_flow_through` / `ffmpeg_fixture_ts_vod_types_fully` (part1 subjects) | YES | Exact-value assertions on the part1 model/parser (these live in my chunk; the code they pin is A09's — noted for reconciliation). |
| `master_types_variants_and_renditions` (866) | YES | Every attribute field asserted incl. quoted-string commas and resolution; mutation: drop quote-awareness in `split_attr_list` → CODECS splits at the comma → assertion fails. |
| `master_single_variant_and_uri_less_rendition` (906) | YES | CC row, URI None, exact bandwidth/uri. |
| `master_malformed_rows_skip_with_warnings` (927) | YES | 4 warning shapes, exact count 4, exact surviving variant. Does NOT cover F-002's two silent shapes or F-003's dropped-attribute case — the suite cannot catch them. |
| `master_refuses_media_shape_and_non_hls` (956) | YES | Both mirror refusals + codes. |
| `master_fixture_files_type` (970) | YES | Committed fixtures (verified to exist) incl. the malformed one with non-empty warnings asserted. |
| Missing coverage | — | No over-cap master line (F-001), no trailing/consecutive STREAM-INF (F-002), no bad RESOLUTION (F-003), no INSTREAM-ID (F-004). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`Parse` (line-cap violation)" among `parse_master_playlist` errors | hls.rs:452 | FALSE | F-001: no construction path; only line 222 (media path) builds `Parse`. |
| "Bounded input (D-1) — lines over MAX_LINE_BYTES (64 KiB) are rejected with a typed error; nothing unbounded is processed" | playlists lib.rs (module doc) | FALSE for the master path | F-001. |
| "Damaged rows are warnings, not errors (dirty-world posture)" | hls.rs:453 | PARTIAL | F-002: two row shapes vanish with no warning at all. |
| "the attribute is dropped, the row survives" (as a `MasterWarning` kind) | hls.rs:437-440 | FALSE | F-003: never emitted; the drop is silent. |
| matrix network-hls: "6.B2a master parse EXT-X-STREAM-INF/EXT-X-MEDIA + highest-BW-under-cap selection" | feature-matrix.toml:1441 evidence | REAL (code+tests exist, baseline green) — UNREACHABLE from shipped binary | Baseline `:4263-4277`; reachability chain in header. |

## 7. Dead / duplicated / unreachable code
- Dead: `MasterWarningKind::BadAttributeValue` (zero construction sites).
- Unreachable: `parse_master_playlist` and (via the streaming hls modules) `parse_media_playlist` — no shipped entry point calls either; the playlists crate is compiled into `lumen` only as streaming's dependency.

## 8. Dependencies & flags
- std + crate-internal (`m3u::{parse_detailed, resolve_uri}`, `error::ParseError/ParseWarning`). No unsafe, no feature gates. Fixtures committed under `tests/fixtures/hls/` (existence verified).
- The `include_str!` fixture tests make the corpus self-contained — no generated goldens (the ffmpeg fixture is a real ffmpeg output; see ts_vod fixture test in part1's suite).

## 9. Open questions for the next agent
- For A09 (hls.rs part1): does `media_playlist_from_parsed` share the same no-warning silent-drop holes for promoted tags? (e.g. a `#EXT-X-VERSION` inside a segment block — where does it land?) Also please confirm the part1/master-boundary at 330/331 does not orphan `parse_tag_uint` coverage.
- For the streaming master.rs owner (A08): when F-002's truncated-master case hits selection, an empty-`variants` master — does `select_highest_under_cap` refuse typed on an empty ladder, or panic? (If it indexes `variants[0]`, the silent row loss becomes a crash two layers up.)
- For the crate-level docs owner: the lib.rs D-1 claim needs a master-path qualifier or a fix (F-001).
