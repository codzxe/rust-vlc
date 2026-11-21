# AUDIT: crates/streaming/src/hls/media.rs#part2
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-21 |
| Chunk boundaries | **part2 = lines 612–1134** (file is 1134 lines; the midpoint 567 falls inside `open_vod` (479–610), so the split was moved to the nearest function boundary — `open_vod` ends at 610, `content_range_total` starts at 614). Part1 (A08) = lines 1–611 (module docs, config, structs, stream machinery, `SourceStream` impl, `open_vod`). My part: `content_range_total`, `TAG_TIMESTAMP_MAP`, `parse_media_playlist_typed`, `playlist_corrupt`, `map_fetch_err`, `fetch_playlist_model`, and the entire `tests` module (738–1134). |
| Lines / functions | 523 / 16 (6 fns/consts + 3 test helpers + 1 fixture const + 8 tests + RouterFetch impl) |
| File verdict | UNREACHABLE (code REAL, tested, and e2e-proven — but no shipped entry point reaches it) |
| Reachable from shipped entry point? | **NO.** `open_vod` (the only public constructor of `HlsVodStream`) is called only by `crates/streaming/tests/hls_vod_e2e.rs:118` and `crates/streaming/tests/net_interrupted.rs` — no app, no engine crate. `apps/cli` uses `streaming::{fetch, ranged, schemes, mockhttp}` only (rg "streaming::" apps/cli/src → demux_host.rs); `crates/input` and `apps/desktop` mention streaming in comments only. `fetch_playlist_model` is called by live.rs (516, 693) and master.rs (493) — themselves equally hostless. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "open_vod|HlsVodStream"` repo-wide → only media.rs + live.rs doc-mentions + 2 test files. `rg -n "fetch_playlist_model"` → live.rs:516,693, master.rs:493 (NOT open_vod). `rg -n "streaming::"` apps/cli, crates/input, apps/desktop → no hls usage. `sed -n` on every quote. Baseline: `orchrun-test-workspace-nofailfast.log.txt:5052-5059` (8/8 hls::media tests ok), `:5212-5219` (hls_vod_e2e 2/2 ok), `:5228-5234` (net_interrupted 3/3 ok). |

## 1. One-paragraph verdict
The helpers in my chunk are the shared plumbing of the HLS sessions — typed playlist-parse mapping, transport-error folding, the shared fetch prelude — and they work, with a genuinely strong test suite behind them (exact-value unit tests, a real-socket relative-URI test asserting the probe wire shape, and an e2e that demuxes TS and decodes ADTS to an ffmpeg oracle, all green in the citable baseline). The findings are structural, not functional: the "shared prelude" is not actually shared by the VOD session that sits ten lines above it (three near-copies of the same fetch/status/UTF-8/parse sequence), and the error folding throws away the HTTP status class it just computed — a permanent 404 surfaces as "connection lost", identical to a dropped socket, and the unit test pins that lossy mapping as the contract. Plus one lenient `Content-Range` parse. As with every other W10 lane so far: none of it is reachable from the shipped binary.

## 2. What actually works (verified)
— The full VOD lane through real sockets: `orchrun-test-workspace-nofailfast.log.txt:5212-5219` — `ts_lane_streams_byte_exact_and_demuxes_over_the_session ... ok`, `adts_lane_is_byte_exact_and_decodes_to_the_ffmpeg_oracle ... ok` (2 passed) — real MockServer socket, real TS/ADTS demux over the `SourceStream`, AAC frame count against an ffmpeg oracle.
— Unit suite 8/8 ok (`:5052-5059`), including `relative_uris_resolve_against_the_playlist_url`, which asserts the exact request sequence (playlist, `bytes=0-0` probe, whole-body GET) on a real socket.
— `parse_media_playlist_typed`'s "unreachable variant" comment verified: `HlsPlaylistError::MediaPlaylist` is emitted only by `parse_master_playlist` (playlists/src/hls.rs:527, inside the function starting at 454), never by `parse_media_playlist` (218) — the mapping arm is dead-but-exhaustive exactly as documented.

## 3. Findings

### F-001 [S3] — the "shared" fetch prelude is not shared by the VOD session; three near-copies exist
Where: media.rs:705-712 (doc) vs 718-736 (impl); open_vod's inline copy at 490-506 (part1); master.rs:721-729 (partial third copy)
Code:
```rust
/// Fetch and type one media playlist (shared by the VOD and live
/// sessions, 6.B1b's extraction of the identical open/reload
/// prelude): one budgeted GET, the success check, the UTF-8 check,
/// and the typed model. Returns the model, the `X-TIMESTAMP-MAP`
```
What is wrong: the doc claims the VOD and live sessions share this prelude, but `open_vod` — the VOD session, 230 lines earlier in the same file — inlines its own copy of the identical GET/status-check/UTF-8/`parse_media_playlist_typed` sequence (490-506) instead of calling `fetch_playlist_model`; only live.rs (516, 693) and master.rs (493) call it, and master.rs:721-729 carries a third partial copy of the UTF-8-mapping arm. The extraction the doc describes ("6.B1b's extraction of the identical open/reload prelude") simply was not applied to the function it names first.
Why it matters: three copies of the transport prelude means a fix to the status/UTF-8/parse handling (e.g. F-002's status mapping, or a future redirect policy) must be applied three times; the doc actively tells a reviewer there is one copy.
What correct looks like: `open_vod` calls `fetch_playlist_model` (ignoring the TimestampMap it doesn't need), or the doc names its actual callers (live + master).
Confidence: HIGH — Verification: read + rg callers (grep output in header).

### F-002 [S3] — error folding discards the HTTP status class it just computed; a permanent 404 is indistinguishable from a dropped connection
Where: media.rs:665-677 (mapping); flag computation then discarded at 722-726 (this chunk) and 308-312, 496-500, 558-562 (part1 sites)
Code:
```rust
        | NetworkError::CrossHostRedirect { .. }
        | NetworkError::CrossSchemeRedirect { .. }
        | NetworkError::HttpStatus { .. } => NetworkCause::ConnectionLost,
```
(and)
```rust
            return Err(map_fetch_err(NetworkError::HttpStatus {
                code,
                is_client_error: (400..=499).contains(&code),
                is_server_error: (500..=599).contains(&code),
            }));
```
What is wrong: every non-2xx answer constructs `HttpStatus` with `is_client_error`/`is_server_error` carefully computed — and `map_fetch_err` reduces the whole variant to `NetworkCause::ConnectionLost` under the generic code `streaming.hls.fetch-failed`. The status code, the client/server classification, and the `code` field carried by the variant all die here: nobody reads them. A 404 (permanent, never retryable) and a mid-transfer socket drop (transient, retryable) produce byte-identical `EngineError`s.
Why it matters: any retry/reconnect policy keyed on the cause (this crate's own reconnect.rs exists for exactly that) will treat dead URLs as weather and hammer them; a user reading the error learns "connection lost" when the server said "not found". `resolve_seek_origin` six lines up uses `AccessDenied::NotFound` — the taxonomy has the right cause and it is not used here.
What correct looks like: map `HttpStatus { is_client_error: true, .. }` to a NotFound/AccessDenied cause (or at least keep `code` in the error context), and stop computing flags nobody consumes.
Confidence: HIGH — Verification: read; `missing_segment_fails_typed_at_open` (955-973) pins the current lossy mapping (`404 → ConnectionLost`), so the behavior is intended — intent does not make the taxonomy right.
What correct looks like: see above.

### F-003 [S4] — `content_range_total` accepts any `…/NNN` header without validating the bytes-unit shape
Where: media.rs:614-622
Code:
```rust
fn content_range_total(value: Option<&str>) -> Option<u64> {
    let value = value?.trim();
    let slash = value.rfind('/')?;
    let total = &value[slash + 1..];
    if total == "*" {
        return None;
    }
    total.parse::<u64>().ok()
}
```
What is wrong: the parser only looks for the last `/` — `"garbage/999"` yields `Some(999)` just like `"bytes 0-0/999"`. A nonconforming 206 answer (wrong unit, wrong range body) silently fixes `size()` to the claimed total; the failure surfaces later as a confusing `seek-out-of-range` or short read instead of a typed refusal at open.
Why it matters: the module's stated posture is "never a guess" (open_vod doc, 530-538); trusting the tail of an unvalidated header is a small guess. Blast radius is low — the server controls the total even in the conforming form, so this is diagnostics quality, not a security hole.
What correct looks like: require the `bytes ` prefix and a `start-end` range component before trusting the total (a 20-line strict parse, or reuse the ranged resource's Content-Range validation if one exists — cross-check with ranged.rs owner).
Confidence: HIGH — Verification: read (no test feeds a malformed Content-Range; the unit router never emits 206 at all — only the mock-socket test does, with a conforming value).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `content_range_total` | 614-622 | REAL | S4 (F-003) | No-panic on all inputs (rfind/parse both Option-typed); lenient shape (F-003). |
| `TAG_TIMESTAMP_MAP` | 628 | REAL | — | Const; consumed by timestamp_map.rs:46 (verified). |
| `parse_media_playlist_typed` | 633-649 | REAL | — | Variant→code mapping; the "unreachable variant" arm verified genuinely unreachable from the media parser (hls.rs:527 is in the master parser). |
| `playlist_corrupt` | 653-660 | REAL | — | Typed Corrupt mint; used here + master.rs:729. |
| `map_fetch_err` | 664-703 | REAL | S3 (F-002) | Dns/Timeout/Tls causes preserved; ResponseTooLarge keeps budget/attempted (good); HttpStatus flattened (F-002). `Cancelled` → ConnectionLost is a documented generation-bump decision (678-682) — deliberate. |
| `fetch_playlist_model` | 712-736 | REAL | S3 (F-001 doc) | Logic itself correct (budgeted GET, status, UTF-8, typed parse, TimestampMap non-fatal); callers are live+master, not VOD. |
| `tests::RouterFetch` + `router` | 747-795 | REAL | — | In-test port adapter; implements the ResponseTooLarge budget contract itself (778-783) — honest seam, not a bypass. Answers 200-with-whole-body to Range probes, exercising the probe-cache lane (the 206 lane is covered by the mock-socket test). |
| `tests::PLAIN_VOD` | 797-806 | REAL | — | Fixture playlist. |
| `vod_concatenation_is_byte_exact_and_ordered` | 813-847 | REAL | — | Byte-exact concat + stats accounting (requests=3, size_probes=2, bytes_fetched=10) — would fail on double-fetch or miscount. |
| `b1b_shapes_refuse_typed_with_their_codes` | 852-893 | REAL | — | All 8 refusal codes asserted individually. |
| `vod_discontinuity_is_recorded_not_refused` | 900-949 | REAL | — | Full boundary map (offsets, absolute sequences, durations, seam flag) + byte-exact concat across the seam. |
| `missing_segment_fails_typed_at_open` | 954-973 | REAL | — | 404 at the size probe → typed error at open. Pins F-002's mapping. |
| `oversize_segment_refuses_over_budget` | 977-999 | REAL | — | Budget refusal via the port contract, ResourceLimit kind asserted. |
| `seek_shapes_are_honest` | 1004-1059 | REAL | — | Retained-prefix backward seek (request-count asserted unchanged), budget head-drop, `seek-unretained`, `seek-out-of-range` — the full seek contract. |
| `relative_uris_resolve_against_the_playlist_url` | 1063-1108 | REAL | — | Real socket; request order + `bytes=0-0` on the probe only. |
| `config_bounds_refuse_typed` | 1112-1133 | REAL | — | Zero budget + over-ceiling retention → `streaming.hls.config`. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 8 unit tests | YES | Exact-value assertions incl. stats counters and wire shapes. Mutation: make `absorb` always retain → `seek_shapes_are_honest` fails (no head drop → no seek-unretained). Make the probe re-fetch cached bodies → `requests == 3` fails. Skip the ENDLIST check → `b1b_shapes` live-playlist row fails. |
| hls_vod_e2e (other file, feeds this module) | YES | Byte-parity with on-disk cat(segments), demux track identification, ADTS frame count == ffmpeg oracle (ORACLE_AAC_PACKETS), request-count 18 with probe ranges asserted — a regression in any layer fails it. Baseline green (`:5212-5219`). |
| Not covered | — | No test feeds a malformed `Content-Range` (F-003); no test asserts what a 5xx maps to (only 404 is pinned). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "shared by the VOD and live sessions, 6.B1b's extraction of the identical open/reload prelude" | media.rs:705-707 | FALSE for VOD | F-001: open_vod inlines its own copy (490-506); only live+master call it. |
| "e2e hls_vod_e2e/hls_live_e2e/hls_master_e2e/hls_conformance_e2e over the mockhttp real socket; streaming 173/0 lib tests" | feature-matrix.toml network-hls evidence (line 1441) | REAL (executed) — but the lane is UNREACHABLE from the shipped binary | Baseline `:5212-5219` (vod 2/2); rg: zero non-test callers of open_vod; apps/cli touches no hls code. |
| "anything else is the typed `streaming.hls.unknown-segment-size` refusal — never a guess" | media.rs:537-538 (part1 doc, governs my `content_range_total`) | PARTIAL | The refusal exists (576-578), but the total itself is taken from an unvalidated header tail (F-003). |

## 7. Dead / duplicated / unreachable code
- Duplication: the fetch/status/UTF-8/parse prelude exists 3× (F-001).
- Dead computation: `is_client_error`/`is_server_error` flags constructed at 4 sites in this file, consumed at 0 (F-002).
- Unreachable: the entire `HlsVodStream` surface (only tests construct it).

## 8. Dependencies & flags
- Uses `crate::fetch::{HttpRequest, HttpResponse, Url, ByteRange, DEFAULT_RESPONSE_LIMIT}` (the port), `playlists::{parse_media_playlist, HlsPlaylistError, MediaPlaylist}`, `plugin_api::SourceStream`, `media_types` error taxonomy. No feature gates. Tests use `mockhttp` (in-crate testkit) — real sockets, no external net.
- The `RouterFetch` test adapter reimplements the response-limit contract rather than disabling it — the budget path stays under test (good).

## 9. Open questions for the next agent
- For A08 (media.rs part1): the `serve`/`absorb` retained-prefix math and the probe loop — I traced `seek_shapes_are_honest` through them by hand and they hold; `open_vod`'s probe loop charges probe bodies to `bytes_fetched` but not to `segments_fetched` (correct per field docs).
- For the ranged.rs owners (A09/A10): does the ranged resource already have a strict Content-Range validator `content_range_total` should reuse (F-003)?
- For whoever owns reconnect.rs (A09): does the retry policy distinguish `NetworkCause` variants? If it retries `ConnectionLost` blindly, F-002 turns 404s into retry storms once a host is wired.
