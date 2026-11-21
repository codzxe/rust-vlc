# AUDIT: crates/playlists/src/hls.rs#part1 (lines 1-629, all production code)
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-21 |
| Lines / functions | 1-629 (chunk boundary declared: part1 = 1-629, ALL production code — media-playlist model + master-playlist model + attr-list parser; part2 = 630-997, the `#[cfg(test)]` module [A06]) / 18 items |
| File verdict | REAL (library; UNREACHABLE from shipped binary) — consumed by streaming/src/hls/{media,master}.rs, which no shipped entry point calls |
| Reachable from shipped entry point? | NO — production callers are streaming/src/hls/media.rs:637 (`playlists::parse_media_playlist`) and streaming/src/hls/master.rs:730 (`playlists::parse_master_playlist`); `rg "hls|Hls" apps/**` → zero matches; the CLI's http lane (`open_http_source`) opens every URL as a whole-file `RangedResource` through the container registry, which has no m3u8 family. Plus the netfuzz harness (tests) and playlists' own tests. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 4 S4: 1 |
| Commands run | `rg "parse_master_playlist|parse_media_playlist|media_playlist_from_parsed" **/*.rs` (callers listed above); `rg "hls" apps/**` (nothing); `rg "BadAttributeValue" crates/playlists/src` (definition only — dead); read of hls.rs:1-700 (all production + test module head); cross-reads: m3u.rs:100/119-122/157/195-308 (BOM strip, 64 KiB line cap, directive attach/dangling rules), lib.rs:48-49 (D-1 promise), fetch.rs:28 (DEFAULT_RESPONSE_LIMIT 8 MiB), hls/media.rs:140-152 (playlist_response_limit); `rg "hls" docs/feature-matrix.toml` + sed of :1430-1441 (network-hls row); `sed -n` quote verifies at 223, 458, 526-528, 539-553; baseline: playlists unit tests NEVER RAN (both baseline test runs abort before the playlists test binary) |

## 1. One-paragraph verdict
The HLS model layer is disciplined, honestly documented ("records, not resolves"), and mostly correct — typed refusals for master/media confusion and bad numerics, first-wins tag promotion, RFC 3986 resolution, a strict little attribute-list parser that rejects duplicates and dangling junk. Its defects cluster at the dirty-world edges the crate's own posture promises to handle: a BOM'd playlist is refused `NotHls` even though the underlying M3U parser deliberately tolerates BOMs; a zero-segment playlist silently loses every playlist-level tag because tag promotion rides on the parser's attach-to-next-location rule; `MasterWarningKind::BadAttributeValue` is dead code (bad `RESOLUTION`/`AVERAGE-BANDWIDTH` values drop silently, not with the documented warning); and the master parser bypasses the crate's D-1 bounded-input discipline entirely (no line cap, no size cap). Overarching: none of this is reachable from the shipped binary — no app opens HLS — while the feature matrix's `network-hls` row claims `partial` on three platforms with evidence strings whose test runs never executed in the committed baseline.

## 2. What actually works (verified)
— `media_playlist_from_parsed` (236-320) traced on the module's own VOD fixture shape: VERSION/TARGETDURATION/MEDIA-SEQUENCE promoted first-wins, exact-name master-tag refusal (EXT-X-MEDIA-SEQUENCE cannot trip it — the match is on the full directive name, :256-259), PLAYLIST-TYPE verbatim, `media_sequence.unwrap_or(0)`, ENDLIST from the parser's own record, segments resolved via `resolve_uri`, EXTINF sum restricted to finite non-negative values (:291-298 — `-1` unknown marker excluded, matching the doc).
— `parse_tag_uint` (324-329): trim + `u64` parse; "10.5"/negative/hex all refuse `BadTagValue` — correct decimal-integer discipline.
— `split_attr_list` (599-628) hand-traced on: `BANDWIDTH=100,CODECS="avc1,mp4a"` (comma inside quotes does not split), duplicate name → None, trailing comma → None, junk after closing quote → None, missing `=` → None, empty list → Some([]) (which then warns VariantWithoutBandwidth downstream — correct). Name charset restricted to `[A-Za-z0-9-]` (:605).
— `variant_from_attrs` (534-566): no parseable BANDWIDTH → None (row skipped with warning at the call site) — never a fabricated 0; `RESOLUTION` split on lowercase `x` only.
— `rendition_from_attrs` (570-593): missing TYPE → None (spec-REQUIRED enforced); unknown TYPE kept verbatim as `Other` (recorded, not refused); DEFAULT/AUTOSELECT strictly `YES`.
— `parse_master_playlist` (454-530): pending-STREAM-INF + URI-line pairing; media-shape refusal fires only when variants AND renditions are empty AND an `#EXTINF`/`#EXT-X-TARGETDURATION` marker was seen (:526-528) — an empty master returns Ok with empty lists (recorded posture, downstream owns the refusal).

## 3. Findings

### F-001 [S2]
Where: docs/feature-matrix.toml:1431-1441 (network-hls row) vs the wiring reality this file participates in
Code (matrix row, verbatim):
```
status = { windows = "partial", linux = "partial", macos = "partial", web = "not_started" }
evidence = "crates/streaming/src/hls/** (media/live/master/timestamp_map) + crates/playlists/src/hls.rs (cards 6.B1a VOD session ffmpeg-oracle e2e, ... e2e hls_vod_e2e/hls_live_e2e/hls_master_e2e/hls_conformance_e2e over the mockhttp real socket; streaming 173/0 lib tests; record docs/formats/streaming-hls.md"
```
What is wrong: the row claims partial HLS **playback** on three shipping platforms, but no shipped entry point reaches one line of HLS code: `apps/` contains zero HLS references, the CLI http lane opens every URL as a whole-file `RangedResource`, and the container registry has no m3u8 family — an `lumen open http://cdn/x.m3u8` cannot enter the HLS stack. Additionally, the evidence's "streaming 173/0 lib tests" and the mockhttp e2e suite never executed in the committed baseline (both baseline `cargo test` runs abort before the streaming/playlists test binaries — test-workspace.log.txt, test-workspace-allfeatures.log.txt). And the matrix contradicts its own standard: `network-auth` (code exists, equally unwired) is honestly `not_started` with empty evidence, while `network-hls` (code exists, equally unwired) is `partial` with a full evidence string.
Why it matters: a reader of the matrix believes the shipped Linux/macOS/Windows builds can play HLS streams (partially). They cannot — the capability is library-only. This is exactly the claim-vs-wiring gap this wave exists to expose, and the row also launders never-executed test counts as evidence.
What correct looks like: either status `not_started`/`partial` with an explicit "library complete, open-path wiring pending" note (mirroring network-auth's honesty), or wire an HLS route into `open_http_source` and cite a real-binary run.
Confidence: HIGH — Verification: read + rg (apps sweep; demux_host.rs:474-500 read; baseline log section inventories read).

### F-002 [S3]
Where: crates/playlists/src/hls.rs:223 and :458 (the EXTM3U scans) vs m3u.rs:120-122 (BOM strip)
Code:
```
    let saw_extm3u = text.lines().any(|line| line.trim() == "#EXTM3U");
```
What is wrong: `str::trim` does not strip U+FEFF (it is not Unicode whitespace), so a playlist whose first line is `\u{FEFF}#EXTM3U` fails the magic scan and is refused `NotHls` — while the very same text parses cleanly through `parse_detailed`, which explicitly strips "exactly one leading U+FEFF" (m3u.rs:120-122, documented at m3u.rs:7 "A leading UTF-8 BOM is tolerated and skipped"). Same defect in `parse_master_playlist` (:458), where the BOM'd header line additionally falls through to the silent junk-URI branch.
Why it matters: BOM'd `.m3u8` files are a real Windows-toolchain shape; the crate deliberately tolerates BOMs in its base parser and the HLS layer then rejects the same file, contradicting the crate's own dirty-world posture. No test pins a BOM'd HLS playlist (test list :649-990 read).
What correct looks like: `let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);` before the scan in both entry points (or scan with BOM-insensitive comparison).
Confidence: HIGH — Verification: read (hls.rs:223, 458; m3u.rs:119-122); `trim` semantics are standard-library fact.

### F-003 [S3]
Where: crates/playlists/src/hls.rs:249-282 (tag promotion) + m3u.rs:280-303 (the attachment rule it rides on)
Code:
```
    // Playlist-level tags land as directives on the first location
    // line they precede (the parser's attachment rule); the spec puts
    // them before the first segment, and the first occurrence wins.
```
What is wrong: the promotion loop iterates `parsed.playlist.entries` — but the M3U parser only flushes accumulated directives when a **location line** appears (m3u.rs:280-298); a trailing directive block with no following location becomes a `DanglingEntry` warning and is dropped (m3u.rs:301-303). A zero-segment media playlist (`#EXTM3U`, VERSION, TARGETDURATION, MEDIA-SEQUENCE, PLAYLIST-TYPE, `#EXT-X-ENDLIST`, no segment lines — the shape a live origin emits at window reset) therefore types with `version/target_duration/media_sequence/playlist_type` all `None` and `media_sequence == 0` even though every tag was present in the text. The only trace is the generic `DanglingEntry` parse warning.
Why it matters: the model's fields lie by omission: the doc says absence is recorded "honestly (`None`)" (:58-61) — this is not absence, it is loss. `MEDIA-SEQUENCE` defaulting to 0 on an empty playlist is precisely the value a live client uses to detect window slides (downstream the B1a session refuses no-ENDLIST shapes, which contains the worst case, but empty VOD/EVENT playlists still lose their metadata silently).
What correct looks like: promote playlist-level tags from the parsed directive stream *before* attachment (the parser could record playlist-scope directives separately), or have `media_playlist_from_parsed` refuse/flag a zero-segment playlist whose text contained promoted tags.
Confidence: HIGH — Verification: read (hls.rs:249-282; m3u.rs:279-308 — flush-on-location and dangling-drop traced).

### F-004 [S3]
Where: crates/playlists/src/hls.rs:437-441 (`MasterWarningKind::BadAttributeValue`) + 539-553 (silent drops)
Code (the dead variant's doc):
```
    /// A well-formed attribute value this model does not type (e.g.
    /// a non-`WxH` `RESOLUTION`): the attribute is dropped, the row
    /// survives.
    BadAttributeValue,
```
What is wrong: `BadAttributeValue` is never constructed anywhere in the crate (rg: single hit = the definition). A malformed `RESOLUTION` (`variant_from_attrs` :547-553 → `None`) or an unparseable `AVERAGE-BANDWIDTH`/`PROGRAM-ID` (:544-545) is dropped **silently** — the variant row is still pushed, no warning is emitted. The documented diagnostic posture (drop the attribute, warn) is not implemented.
Why it matters: dead enum variant + a silently-swallowed attribute the docs promise to report — the exact "swallowed errors" pattern this audit hunts; a packager emitting `RESOLUTION=1920X1080` (capital X) loses resolution data with zero telemetry.
What correct looks like: emit `MasterWarning { line, kind: BadAttributeValue }` when a present, well-formed attribute fails to type (needs the line context threaded into `variant_from_attrs`), or delete the variant and fix the docs.
Confidence: HIGH — Verification: read + rg (`BadAttributeValue` definition-only).

### F-005 [S3]
Where: crates/playlists/src/hls.rs:454-530 (`parse_master_playlist`) vs lib.rs:48-49 / m3u.rs:100,157
Code (the unbounded walk):
```
    for (idx, raw) in text.lines().enumerate() {
```
What is wrong: the master parser is a fresh hand-rolled line walk with **no** `MAX_LINE_BYTES` cap (the M3U parser makes lines over 64 KiB fatal — m3u.rs:100/157) and **no** total-size cap (the MPD parser carries `MAX_MPD_BYTES`). A single multi-megabyte attribute line is walked and its values cloned (`split_attr_list` allocates a `String` pair per attribute — ~8× structural amplification on comma-stuffed input) with no typed refusal. The crate's own D-1 promise ("lines over `MAX_LINE_BYTES` are rejected with a typed error; nothing unbounded is processed", lib.rs:48-49) is violated for one of its three parsers.
Why it matters: in the streaming lane the input arrives transport-bounded (`playlist_response_limit`, default 8 MiB — hls/media.rs:141,151), and the fuzz harness bounds inputs itself, so the blast radius is contained by callers today — but the crate-level contract is broken, and any future caller (a local-file .m3u8 open) inherits an unbounded parse. The asymmetry (media playlists capped, master playlists not) is the defect.
What correct looks like: enforce the same 64 KiB line cap (and ideally an explicit total cap) with the typed refusal, or route masters through the capped M3U line walk like the media model does.
Confidence: HIGH — Verification: read (454-530, 599-628; m3u.rs:100/157; lib.rs:48-49; hls/media.rs:140-152; fetch.rs:28).

### F-006 [S4]
Where: crates/playlists/src/hls.rs:512-523 (dangling pending / orphan URI)
Code:
```
        // A URI with no pending STREAM-INF is damage the row-skip
        // posture tolerates silently only when variants exist; with
        // none at all the media-shape refusal below stays decisive.
```
What is wrong: a pending `#EXT-X-STREAM-INF` left dangling at EOF (no URI line followed) is dropped without a warning, and orphan URI lines are ignored silently — while the *other* damaged-row shapes (bad attr list, bandwidth-less row, bad rendition) all warn. The row-skip diagnostics are inconsistent.
Why it matters: a truncated master playlist (connection cut mid-download) loses its last variant with no signal, where the same file with a mangled attr list would warn.
What correct looks like: a `MasterWarning` for the dangling pending row (the line number is already in hand, :465).
Confidence: HIGH — Verification: read (464-524).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-42 | REAL | — | Accurate on the record-not-resolve split; BOM/zero-segment gaps are unpromised, not misdocumented. |
| `MediaPlaylist` | 48-83 | REAL | S3 | Fields sound; `media_sequence` default 0 interacts with F-003. |
| `MediaSegment` | 48-108 | REAL | — | Recorded fields only. |
| `HlsPlaylistError` + `code` + `Display` + `Error` | 110-202 | REAL | — | Stable dotted codes; Display carries no URIs/line text (MP §12 honored — verified each arm). |
| `parse_media_playlist` | 206-226 | REAL | S3 | F-002 (BOM scan). |
| `media_playlist_from_parsed` | 228-320 | REAL | S3 | F-003 (zero-segment tag loss); promotion/refusal/sum logic verified. |
| `parse_tag_uint` | 322-329 | REAL | — | Correct decimal-integer discipline. |
| `MasterPlaylist` | 333-350 | REAL | — | Plain data. |
| `Variant` / `Resolution` | 352-380 | REAL | — | Plain data; BANDWIDTH never fabricated. |
| `Rendition` / `RenditionType` | 382-417 | REAL | — | Unknown TYPE verbatim — recorded posture. |
| `MasterWarning` / `MasterWarningKind` | 419-443 | REAL | S3 | F-004 (BadAttributeValue dead). |
| `parse_master_playlist` | 445-530 | REAL | S3/S4 | F-002 (BOM), F-005 (no caps), F-006 (silent drops); pairing + refusal logic verified. |
| `variant_from_attrs` | 532-566 | REAL | S3 | F-004's silent RESOLUTION/avg-bw drops. |
| `rendition_from_attrs` | 568-593 | REAL | — | TYPE-required enforced; spec-REQUIRED GROUP-ID/NAME tolerated absent (documented). |
| `split_attr_list` | 595-628 | REAL | — | Strict and correct (duplicates/trailing/junk refused); traced on 6 shapes. |

## 5. Test honesty (tests live in part2 :630-997 [A06]; cross-reference)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `vod_playlist_types_and_resolves` (:649) | YES | Every promoted field + resolution + duration sum asserted. |
| `bad_numeric_values_refuse_typed` (:739) | YES | BadTagValue pinned for each numeric tag. |
| `ffmpeg_fixture_ts_vod_types_fully` (:827) | YES | Real ffmpeg-generated fixture — the model typed against an external artifact. |
| `master_malformed_rows_skip_with_warnings` (:927) | PARTIAL | Pins bandwidth-less/bad-list rows — but cannot pin BadAttributeValue (never emitted — F-004) and has no dangling-STREAM-INF case (F-006). |
| **BOM'd playlist** | NO TEST EXISTS | F-002 unpinned. |
| **Zero-segment playlist** | NO TEST EXISTS | F-003 unpinned. |
| **Baseline execution** | **NEVER RAN** | Both baseline runs abort before the playlists test binary. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| network-hls `partial` on win/linux/macos | docs/feature-matrix.toml:1438 | SHALLOW | F-001: real library + tests, zero shipped reachability; evidence runs never executed in baseline. |
| "streaming 173/0 lib tests" | docs/feature-matrix.toml:1440 | UNPROVEN | Streaming test binary never ran in either committed baseline run. |
| "A leading UTF-8 BOM is tolerated and skipped" (crate posture) | m3u.rs:7, lib.rs docs | SHALLOW for HLS | F-002: the HLS entry points refuse BOM'd input the base parser tolerates. |
| "Bounded input (D-1) — lines over MAX_LINE_BYTES are rejected" | lib.rs:48-49 | FALSE for masters | F-005: parse_master_playlist has no cap. |
| "a non-WxH RESOLUTION: the attribute is dropped, the row survives [warning]" | hls.rs:437-440 | FALSE | F-004: dropped silently, warning never emitted. |

## 7. Dead / duplicated / unreachable code
`MasterWarningKind::BadAttributeValue` dead (F-004). Whole file unreachable from shipped (header). Master parser duplicates the M3U parser's line-walk (CRLF strip, trim, comment skip) instead of reusing it — the duplication is what dropped the D-1 cap (F-005's root cause).

## 8. Dependencies & flags
`crate::error` (ParseWarning), `crate::m3u` (parse_detailed/resolve_uri), `crate::types` (HlsKey/HlsMap/ByteRange). No feature gates. `use std::fmt;` sits mid-file at :204 (after the impl that uses it) — works, style-debt only.

## 9. Open questions for the next agent
- A06 (part2): does any test feed a BOM'd or zero-segment playlist (would F-002/F-003 be caught)? Name list says no.
- A06/A07 (streaming hls/media.rs, master.rs): does the session layer refuse empty-master/empty-playlist shapes typed (containing F-003's blast radius)? And does `playlist_response_limit` actually reach the master fetch (I verified the config field exists, not its wiring)?
- Orchestrator: the network-hls row's evidence claims need a gate decision (F-001) — same class as the standing engine-info hard-coding defect.
