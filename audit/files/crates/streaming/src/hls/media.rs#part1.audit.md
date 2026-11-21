# AUDIT: crates/streaming/src/hls/media.rs#part1 (lines 1–567)
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-21 |
| Lines / functions | 567 audited (chunk part1 of 1134; part2 = 568-1134 is another agent's — open_vod tail lines 568-610, content_range_total, TAG_TIMESTAMP_MAP, parse_media_playlist_typed, playlist_corrupt, map_fetch_err, fetch_playlist_model, and `mod tests`; `open_vod` (479-610) crosses the 567 boundary and is audited whole here with this note) / 21 production items: 3 error helpers, HlsVodConfig (Default+validate), 2 stat/boundary structs, HlsVodStream (3 accessors, fetch_segment, absorb, serve, resolve_origin), resolve_seek_origin, 6 SourceStream methods, open_vod |
| File verdict | UNREACHABLE (real, tested code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO. `open_vod`/`HlsVodStream` callers: `crates/streaming/tests/hls_vod_e2e.rs:120`, `crates/streaming/tests/net_interrupted.rs:186` (rg census) — tests only. The CLI's http lane is RangedResource-only (apps/cli/src/demux_host.rs:427,474-500); media-probe has no m3u8 signature; apps/cli/src/dispatch.rs:191-192 refuses `playlist.*` ("the playlist model is a Phase-4 card"). |
| Findings | S0: 0 S1: 1 S2: 1 S3: 3 S4: 2 |
| Commands run | `grep -n` structure map (21 fn pins); `sed -n` verbatim verification of every quote; `rg "open_vod\|HlsVodStream"` (caller census); `rg "unknown-length"` in crates/demux + apps/cli (constraint verification — REAL: demux/src/ts/mod.rs:592, mkv/ebml.rs:283, avi/riff.rs:228, adts/mod.rs:377, es/*); `grep DEFAULT_RESPONSE_LIMIT` fetch.rs:28 (= 8 MiB — probe-cache bound input); baseline: `orchrun-test-workspace-nofailfast.log.txt` :5052-5059 (8/8 media tests ok), :5212-5219 (hls_vod_e2e 2/2 ok) |

## Chunk boundary note
Part1 = 1–567: module docs, constants, error helpers, config, stats/boundary types, the whole `HlsVodStream` serving machinery and `SourceStream` impl, and `open_vod` (479-610 — crosses the boundary at 567, audited whole; its probe-loop tail 568-610 is nominally part2's). The file's `mod tests` (738+) is part2's range; this audit covers part1's functions and cites the tests' baseline execution as evidence for them.

## 1. One-paragraph verdict
This chunk is the HLS VOD media-playlist session's engine: typed playlist refusals, a bounded retained-prefix byte server, honest seek boundaries, and the open path with its "known-length probe" contract. The serving core is correct on every path I traced — the retained-prefix invariant holds (retained either covers `[retained_from, fetched_upto)` or is empty with `retained_from == fetched_upto`; seek admits only that span), skip-fetch forward seeks are accounted, and the demux-layer `unknown-length` constraint that motivates the design is real (verified in 5 demux lanes). But the open path has two structural defects: on a no-range server every 1-byte size probe returns the FULL body, and `open_vod` caches **all** of them in `probe_cache` with no aggregate budget — the "Bounded memory (MP §12)" module claim is false on that lane and a large VOD resource allocates in full at open (S1); and the design costs one HTTP round-trip per segment before the first read — thousands of sequential requests for a realistic playlist (S2). Beyond that: probe `Content-Range` totals are trusted but never verified against delivered bytes (a lying or changed server yields wrong `size()`/boundary offsets and a silent EOF where the ranged resource raises typed truncation), `bytes_fetched` excludes the playlist body its own doc says it includes, and `read()` silently clamps where `RangedResource` refuses typed. All of it is library-plus-tests: no shipped binary opens an m3u8.

## 2. What actually works (verified)
— 8/8 in-file tests + 2/2 `hls_vod_e2e` executed ok: baseline `orchrun-test-workspace-nofailfast.log.txt:5052-5059` and `:5212-5219` (tests live in part2's range but execute this chunk's `fetch_segment`/`absorb`/`serve`/`seek`/`open_vod`).
— Read-traced the retained-prefix invariant end to end: `absorb` (326-340) drops to `[start, …)` or empty-at-`fetched_upto`; `seek` (426-458) admits only `[retained_from, fetched_upto]` with typed refusals on both ends (`streaming.hls.seek-unretained`, `seek-out-of-range`); `serve`'s `at`/`have` index math (347-348) therefore cannot underflow. `probe_cache[i]` indexing (293) is guarded by `serve`/`seek` only calling with `i < segments.len()`, and `probe_cache` has exactly one slot per segment (one push per loop iteration, 542-585).
— Verified the design's load-bearing claim: `demux.source.unknown-length` typed refusals exist at demux/src/ts/mod.rs:592, mkv/ebml.rs:283, avi/riff.rs:228, adts/mod.rs:377 — the TS/ADTS consumers of this session genuinely require a known size, so fixing the total at open is a real constraint, not a fiction.
— `resolve_seek_origin` (375-399) is the exact i128 shape of ranged.rs's `resolve_origin` with HLS codes — before-start and overflow both typed, `Start` pass-through.

## 3. Findings

### F-001 [S1]
Where: crates/streaming/src/hls/media.rs:581-585 (with 542, 69-77)
Code:
```
        } else {
            // `200`: the whole body — the size AND the bytes.
            total += resp.body.len() as u64;
            probe_cache.push(Some(resp.body));
        }
```
What is wrong: On the no-range server shape (the 6.A2b lane this code exists to serve), every 1-byte size probe is answered `200` with the **full** segment body, and every one of those bodies is pushed into `probe_cache` and held simultaneously until consumed by reads. There is no aggregate bound: the only limits are the per-response `segment_response_limit` (default 8 MiB, fetch.rs:28) and the segment *count* (playlist-controlled). `probe_bytes` is counted (567) but never checked against any budget. The retained-prefix cap (`max_retained_bytes`, default 64 MiB) does not apply — the cache lives outside `retained` until `fetch_segment` consumes it, and consumption only starts at the first read.
Why it matters: The module's "Bounded memory (MP §12)" claim ("Nothing is allocated ahead… never allocates", 69-77) is false on this lane: opening a no-range-served VOD resource larger than RAM allocates the whole resource at open — OOM on valid input, in the exact lane (no-range server) the fallback design says must play. A hostile playlist (thousands of segment URIs) plus a no-range server is a memory-exhaustion vector with attacker-controlled magnitude.
What correct looks like: Bound the cache: keep cached bodies only while `cached_bytes <= max_retained_bytes`, drop the rest (they can be re-fetched whole when reached); or cap `probe_cache` to the first K bodies and fall back to re-fetching later segments.
Confidence: HIGH — Verification: read (allocation sites and absence of any aggregate check traced); OOM scenario reasoned (no execution, per rules).

### F-002 [S2]
Where: crates/streaming/src/hls/media.rs:530-536 (with 547-586)
Code:
```
    // The container layer refuses unknown-length sources at open
    // (`demux.source.unknown-length`), so a VOD session fixes the
    // concatenation total BEFORE the first read: one 1-byte Range
    // probe per segment. A `206` names the total in `Content-Range`
```
What is wrong: The open path issues one sequential HTTP request per segment (plus the playlist GET). For a realistic VOD playlist — 2-second segments over a 2-hour movie ≈ 3,600 segments — that is 3,601 sequential round-trips before the first byte is served. At 30 ms RTT that is ~108 seconds of open latency; nothing is pipelined or parallelized (the loop at 547 is strictly serial), and no cap on the probe count exists.
Why it matters: This is the open path of the P1 network-hls feature; the cost scales with playlist length and is pure latency (1 byte of value per request). The constraint it serves (demuxers refuse unknown-length) is real, but the resolution chosen is the most expensive possible one — VLC-class players demux HLS incrementally without a byte-exact total. Any real-world-sized playlist makes the session unusable regardless of correctness.
What correct looks like: Lazy totals (probe on first need), HEAD/Content-Length probes, parallel probe batches, or an aggregate-duration-based contract with the demux layer for the HLS lane.
Confidence: HIGH — Verification: read (serial loop, no concurrency, no count cap); constraint verified real (see §2).
### F-003 [S3]
Where: crates/streaming/src/hls/media.rs:575-580 (with 568-574)
Code:
```
        if resp.status == 206 {
            let Some(len) = content_range_total(resp.header("content-range")) else {
                return Err(refuse("streaming.hls.unknown-segment-size"));
            };
            total += len;
            probe_cache.push(None);
        }
```
What is wrong: The probe trusts the `Content-Range` total and nothing ever verifies it against the bytes later delivered. `size()` returns the claimed total forever; `SegmentBoundary::offset`s are cumulative claims (569-574). If a server's probe answer disagrees with the segment body later fetched (lying server, or the resource changed between probe and fetch), the session serves the real bytes but reports the fictional total: reads end in a silent clean EOF short of `size()`, boundary offsets drift from real offsets, and a demuxer seeking by boundary lands mid-segment. The ranged resource treats exactly this class of disagreement as typed `Corrupt::Truncated` (`streaming.ranged.truncated`, ranged.rs:596-599); this session has no equivalent check.
Why it matters: Inconsistent trust posture inside one crate for the same protocol situation; silent EOF-short-of-size is the "guessed length" the module's own philosophy ("never a guess") forbids.
What correct looks like: On each real segment fetch, compare delivered `body.len()` to the probe-claimed segment size; on disagreement, typed truncation (or at minimum a warning surfaced in stats).
Confidence: MEDIUM — Verification: read (no verification site exists); disagreement scenario reasoned.

### F-004 [S3]
Where: crates/streaming/src/hls/media.rs:199-200 (doc) with 602-608 (init, part2 range, cited as the produced state)
Code:
```
    /// Sum of all body bytes received (probe bodies included).
    pub bytes_fetched: u64,
```
What is wrong: The playlist GET's body (received at 492-503) is never counted: `bytes_fetched` is initialized to `probe_bytes` only (605). Segment fetches add theirs (317); the playlist body — the first body the session receives — is missing from a field documented as "sum of all body bytes received". (Identical class of defect as master.rs `HlsMasterStats::bytes_fetched`, W10-A08 master.rs#part2 F-002.)
Why it matters: Any efficiency metric computed from `bytes_fetched` (fetch overhead vs served bytes) is systematically wrong by one playlist body per open/reload.
What correct looks like: `bytes_fetched: body.len() as u64 + probe_bytes` at init, or re-document as "probe + segment body bytes".
Confidence: HIGH — Verification: read of all three increment/init sites (317, 605, 492-503).

### F-005 [S3]
Where: crates/streaming/src/hls/media.rs:422-424 (with crates/streaming/src/ranged.rs:861-870 — W10-A08 ranged.rs#part3 audit; crates/streaming/src/hls/master.rs:657-659)
Code:
```
    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        self.serve(len.min(MAX_READ_BYTES))
    }
```
What is wrong: A read above the 64 MiB payload ceiling is silently clamped into a short read; `RangedResource::read` answers the identical input with the typed `streaming.ranged.read-over-budget`. Same trait, same crate, two semantics. (Same defect as master.rs#part2 F-003; this is the second of the two clamping sites.)
Why it matters: Silent clamp vs typed refusal drift across `SourceStream` impls a demuxer consumes interchangeably; neither site documents the choice.
What correct looks like: One crate-wide policy (typed refusal here, or clamp in ranged), documented in the implementor docs.
Confidence: HIGH — Verification: read of all three impls.

### F-006 [S4]
Where: crates/streaming/src/hls/media.rs:406-414
Code:
```
        self.total_len
            .or((self.next == self.segments.len()).then_some(self.fetched_upto))
    }
```
What is wrong: The `.or(...)` fallback is dead code: `open_vod` always constructs with `total_len: Some(total)` (598) — there is no construction path that leaves it `None`. The comment's "A stream whose probes somehow never ran keeps the honest None" describes an unreachable state (probes cannot be skipped: the loop either completes or errors the open).
Why it matters: Dead defensive branch with a comment asserting a state that cannot occur misleads the next maintainer into thinking partial-open sessions exist.
What correct looks like: Delete the fallback and the comment, or make it reachable by design (lazy-probe open, which F-002 argues for anyway).
Confidence: HIGH — Verification: read (sole constructor site 587-609).

### F-007 [S4]
Where: crates/streaming/src/hls/media.rs:176-185 (with crates/streaming/src/ranged.rs:351-357)
Code:
```
        if self.max_retained_bytes > MAX_RETAINED_BYTES {
            return Err(mint(
```
What is wrong: `validate` refuses an over-ceiling retention but accepts `max_retained_bytes == 0` (a legal "retain nothing" config — every backward seek then refuses `seek-unretained`), while the sibling `RangedConfig::validate` refuses `window_bytes == 0` outright as `streaming.ranged.config-zero`. Two configs of the same crate, opposite answers to the same degenerate input, with no rationale recorded on either side.
Why it matters: Config validation philosophy drift; a user setting 0 gets silently-legal here and typed-refused there.
What correct looks like: One rule (both refuse with their `config-zero` codes, or both document 0-as-legal), stated in both validators.
Confidence: HIGH — Verification: read of both validators.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| hls_format / mint / refuse | 115-135 | REAL | — | Error plumbing; `Unsupported` carries the hls-media-playlist FormatInfo. Used by master/live too. |
| HlsVodConfig::Default | 148-156 | REAL | — | 8 MiB response limits (DEFAULT_RESPONSE_LIMIT), 64 MiB retention. |
| HlsVodConfig::validate | 165-187 | REAL | S4 (F-007) | Zero limits + over-ceiling typed; zero retention accepted (inconsistent with ranged). |
| HlsVodStats (struct) | 192-207 | REAL | S3 (F-004) | Doc/behavior mismatch on bytes_fetched. |
| SegmentBoundary (struct) | 219-230 | REAL | — | Data record; offsets are probe-claims (F-003 caveat). |
| HlsVodStream::{playlist, stats, segment_boundaries} | 269-287 | REAL | — | Accessors; whole map available pre-read (by probe claims). |
| HlsVodStream::fetch_segment | 292-320 | REAL | — | Probe-cache hit path skips the request and the double count; miss path maps non-2xx to typed HttpStatus. No panic path (index guarded). |
| HlsVodStream::absorb | 326-340 | REAL | — | Budget drop rule; identical shape to master.rs's (stated shared design). Invariant traced sound. |
| HlsVodStream::serve | 343-361 | REAL | — | Fetch-until-len-or-EOF loop; index math safe under the traced invariant; clean sticky EOF. |
| HlsVodStream::resolve_origin | 366-368 | REAL | — | Delegates to the shared helper. |
| resolve_seek_origin | 375-399 | REAL | — | i128 math, typed before-start/overflow; shared with live session. |
| SourceStream::uri / position | 402-404 / 460-462 | REAL | — | Trivial. |
| SourceStream::size | 406-414 | REAL | S4 (F-006) | Probe-fixed total; dead `.or` fallback. |
| SourceStream::supports_seek | 416-420 | REAL | — | `true` within retained prefix + forward skip-fetch; honest given seek's refusals. |
| SourceStream::read | 422-424 | REAL | S3 (F-005) | Silent 64 MiB clamp. |
| SourceStream::seek | 426-458 | REAL | — | Backward `< retained_from` and forward-past-end typed; forward skip-fetch loop accounted in segments_skipped. No panic path. |
| open_vod (BOUNDARY fn, audited whole) | 479-610 | REAL | S1 (F-001), S2 (F-002), S3 (F-003) | URL parse → playlist GET → UTF-8 → typed model → B1a refusal table (512-526, honest and exhaustive per docs) → N size probes. Probe-cache unbounded on the 200 lane; serial O(N) RTTs; totals trusted unverified. UNREACHABLE from shipped binaries. |

## 5. Test honesty
Tests for this chunk live in part2's range (`mod tests`, 738+). Baseline execution: 8/8 ok (:5052-5059) + hls_vod_e2e 2/2 (:5212-5219, byte-exact + ffmpeg-oracle ADTS lane). From the test names and the assertions they pin (byte-exact concatenation, typed refusal codes, seek shapes, relative-URI resolution, over-budget refusal), they exercise this chunk's functions against real mock-socket responses; part2's auditor owns their line-level honesty adjudication. The defects F-001/F-003 (aggregate cache, unverified totals) have no test coverage in either half — no test opens a no-range session over a multi-segment over-budget-aggregate playlist, and none serves a segment whose body disagrees with its probe total.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Bounded memory (MP §12): Nothing is allocated ahead… never allocates" | media.rs:67-77 | FALSE on the no-range lane | F-001: probe_cache holds every full body simultaneously, unbounded aggregate. |
| "one 1-byte Range probe per segment… Anything else is the typed refusal — never a guess" | media.rs:530-538 | REAL (mechanism) / misleading (cost) | Mechanism verified (loop 547-586 + tests); "never a guess" is false for a *lying* total (F-003). O(N) RTT cost undocumented (F-002). |
| "size() is None until every segment has been fetched, then the exact total" | media.rs:63-65 (module docs) | FALSE | size() returns the probe-claimed total from open (total_len always Some); the doc describes live-session behavior, not this session's. Stale doc from the pre-probe design. |
| "Sum of all body bytes received (probe bodies included)" | media.rs:199-200 | FALSE | F-004: playlist body excluded. |
| feature-matrix network-hls "partial" | docs/feature-matrix.toml:1437-1441 | REAL-UNPROVEN / UNREACHABLE from product | Same as master.rs#part2 §6: code + tests real (baseline cited), zero shipped reachability. |

## 7. Dead / duplicated / unreachable code
- Whole chunk unreachable from shipped entry points (see header) — library + tests only.
- `size()`'s `.or` fallback dead (F-006).
- `absorb`/`serve` duplicated nearly verbatim in master.rs (603-636) and (per its docs) live.rs — the "shared budget rule" is copy-pasted per session, not factored. Stated design choice; a fourth copy in live.rs would make it a hygiene finding for that owner.
- `resolve_origin` (366-368) is a one-line wrapper over `resolve_seek_origin` — vestigial seam.

## 8. Dependencies & flags
`playlists` (MediaPlaylist, HlsPlaylistError), `plugin_api` SourceStream, `crate::fetch` (ByteRange, DEFAULT_RESPONSE_LIMIT, HttpFetch/Request), `crate::hls::timestamp_map`, media-types error taxonomy. No feature gates, no unsafe. `#[must_use]` on accessors (verified real bytes — display pipeline mangles `[m`; checked with od during this session).

## 9. Open questions for the next agent
- W10-A? (media.rs part2): the `mod tests` honesty table; does any test cover the no-range multi-segment cache path (F-001) or a probe/body disagreement (F-003)? From the names I see none — confirm.
- Live-session owner (hls/live.rs): does `open_live` share the probe pattern (F-002 cost) or is it reload-based?
- Matrix-rows owner for network-hls: the "partial" status must account for F-002's open-latency ceiling on real playlists (a 3,600-segment VOD is unusable even once wired).
