# AUDIT: crates/streaming/src/hls/media.rs
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-23 |
| Lines / functions | 1134 / 21 prod fns + 8 tests |
| File verdict | REAL (code+tests genuine, executed) — UNREACHABLE from shipped entry points; one latent panic path untested |
| Reachable from shipped entry point? | NO — `open_vod` has zero production callers (rg repo: only crates/streaming/tests/net_interrupted.rs:35,186 and tests/hls_vod_e2e.rs:43,120 plus this file). The shipped `lumen open <url>` path routes http(s) URLs to `streaming::ranged::RangedResource` + media-probe (apps/cli/src/demux_host.rs), which has no `#EXTM3U` signature; an .m3u8 URL dies at the probe stage, never reaching this module. Matches the prior live.rs audit (audit/files/crates/streaming/src/hls/live.rs.audit.md, same conclusion for open_live). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `./target/debug/deps/streaming-032d0d746c2d060d hls::media` → 8 passed/0 failed exit 0 (same for streaming-c6cf1f9f1d2bf8f0); rg `open_vod|HlsVodStream|HlsVodConfig` repo-wide; sed quote verification |

## 1. One-paragraph verdict
The VOD media-playlist session is real, carefully typed, and honestly tested: I ran its 8 unit tests from the prebuilt binary (byte-exact concatenation, refusal table codes, discontinuity record, over-budget refusal, seek shapes, real-socket relative-URI resolution) — all pass. But the whole session is dead code in the shipped binary (no caller routes an HLS URL to it), and `serve` contains a deterministic panic: when a single segment body exceeds `max_retained_bytes` (a config `validate()` explicitly accepts — it only rejects zero limits and retention > 1 GiB), `pos - retained_from` underflows and the subsequent slice indexing panics in both debug and release. No test covers that branch; the seek test stops exactly at its edge (bodies 4+6 vs budget 8 — each body fits alone). Additionally the open-time "known-length contract" serially issues one HTTP probe per segment before the first read, and `size()` is never cross-checked against the bytes actually fetched.

## 2. What actually works (verified)
— verified by `./target/debug/deps/streaming-032d0d746c2d060d hls::media` → `test result: ok. 8 passed; 0 failed` (exit 0): concatenation byte-exact and ordered (`AAAABBBBBB`), size fixed at open via probes, all 8 refusal codes exact (`streaming.hls.live-playlist/.empty-playlist/.encrypted/.byterange-segments/.init-segment/.master-playlist/.bad-tag-value/.not-hls`), discontinuity map (offset/media-sequence/duration/flag), 404→typed Network, over-budget→typed ResourceLimit, seek-unretained/seek-out-of-range refusals, and a real loopback-socket run proving the wire shape (`/hls/pl.m3u8`, probe `bytes=0-0`, then whole-body GET).

## 3. Findings
F-001 [S2]
Where: crates/streaming/src/hls/media.rs:343-351 (with 331-339)
Code:
```
    fn serve(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        let mut out = Vec::new();
        while out.len() < len {
            if self.pos < self.fetched_upto {
                let at = (self.pos - self.retained_from) as usize;
                let have = self.retained.len() - at;
                let take = have.min(len - out.len());
                out.extend_from_slice(&self.retained[at..at + take]);
```
What is wrong: when `absorb` hits a body larger than the whole retention budget it sets `retained_from = self.fetched_upto` (L337) with `retained` empty; the read cursor `pos` is then strictly below `retained_from`, so `self.pos - self.retained_from` underflows (debug: subtract-overflow panic; release: wraps, then `self.retained.len() - at` wraps, then `&self.retained[at..at+take]` fails the slice bounds check since `at` is astronomically > 0). Panic is guaranteed in both profiles. Trigger: `HlsVodConfig { segment_response_limit: 1024, max_retained_bytes: 4, .. }` (accepted by `validate()` L165-187, which does not cross-check the two budgets) + any segment > 4 bytes + one `read()`. Defaults mask it (bodies ≤ 8 MiB < 64 MiB retention), and the shipped binary never calls `open_vod` — else this would be S1. None of the 8 tests exercises a single body over the whole budget (`seek_shapes_are_honest` uses 4+6-byte bodies against budget 8). What correct looks like: a typed `streaming.hls.seek-unretained`-style refusal (or clamp of retention semantics), plus a test.
Confidence: HIGH — Verification: read (arithmetic deterministic; not executed — no way to add a test under audit constraints; UNVERIFIED by execution)

F-002 [S3]
Where: crates/streaming/src/hls/media.rs:479-483
Code:
```
pub fn open_vod(
    fetch: Arc<dyn HttpFetch>,
    url: &str,
    config: HlsVodConfig,
) -> Result<HlsVodStream, EngineError> {
```
What is wrong: the entire 610-line session (plus `fetch_playlist_model`, `parse_media_playlist_typed`, `map_fetch_err` shared with live) is unreachable from the shipped entry points — only test binaries call it. The module doc's refusal table (`streaming.hls.*` codes) is vocabulary a user of the shipped binary can never see for an HLS URL. Why it matters: 1134 lines of tested engine code are dead weight in the release lane, and any doc reader believing "HLS VOD opens typed" is wrong about the binary. What correct looks like: an open-lane branch sniffing `#EXTM3U`/content-type routing to `open_vod`, or matrix text stating "not wired".
Confidence: HIGH — Verification: read + rg

F-003 [S3]
Where: crates/streaming/src/hls/media.rs:547-586
Code: (547-553)
```
    for (i, url) in segments.iter().enumerate() {
        let request = HttpRequest::get(url.clone())
            .with_range(ByteRange::Bounded {
                start: 0,
                end_exclusive: 1,
            })
            .with_response_limit(config.segment_response_limit);
```
What is wrong: (a) Perf — one serial HTTP round-trip per segment at open before the first read (a 2-hour VOD at 6 s segments = 1200 serial probes; at 50 ms RTT that is a minute of open latency; the count is bounded only by the playlist response limit, 8 MiB of text ≈ 100k lines). The tradeoff is disclosed ("the known-length contract") but the latency is unbounded and unbatched. (b) Correctness — `total` is taken from the probe (Content-Range total or 200 body length) and NEVER re-verified when the segment body is actually fetched (`fetch_segment` absorbs without comparing against the probe-derived total); a server answering the probe and the whole-body GET inconsistently makes `size()` (L412: `self.total_len`) a lie while the served bytes drift. What correct looks like: parallel/batched probes or a HEAD, and a `streaming.hls.size-drift` typed refusal on mismatch.
Confidence: HIGH (design), MEDIUM (field realism of lying servers) — Verification: read

F-004 [S4]
Where: crates/streaming/src/hls/media.rs:406-414
Code:
```
        self.total_len
            .or((self.next == self.segments.len()).then_some(self.fetched_upto))
```
What is wrong: dead fallback. The only constructor (`open_vod` L598) always sets `total_len: Some(total)`; there is no path where "probes somehow never ran" — the comment describes an impossible state. Misleading.
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| hls_format / mint / refuse | 115-135 | REAL | — | typed error mints; used by refusals |
| HlsVodConfig::Default / validate | 148-187 | REAL | S2-adjacent | validate misses the segment_limit>retention cross-check that enables F-001 |
| HlsVodStream::playlist/stats/segment_boundaries | 269-287 | REAL | — | advisory accessors |
| fetch_segment | 292-320 | REAL | — | probe-cache consume; non-2xx typed; no size cross-check (F-003b) |
| absorb | 326-340 | REAL | S2 | body-over-budget branch creates F-001 state |
| serve | 343-361 | REAL | S2 | F-001 underflow/slice panic |
| resolve_origin / resolve_seek_origin | 366-399 | REAL | — | i128 math; before-start/overflow typed |
| SourceStream::size | 406-414 | REAL | S4 | F-004 dead `.or` |
| SourceStream::supports_seek | 416-420 | REAL | — | unconditional true; honest per docs (refusals are typed) |
| SourceStream::read | 422-424 | REAL | — | len capped at 64 MiB silently (documented) |
| SourceStream::seek | 426-458 | REAL | — | backward-in-prefix / forward skip-fetch / typed refusals; tested |
| SourceStream::position/uri | 401-404, 460-462 | REAL | — | trivial |
| open_vod | 479-610 | REAL | S3 | F-002 unreachable; F-003 probe storm |
| content_range_total | 614-622 | REAL | — | `*`/malformed → None → typed refusal |
| TAG_TIMESTAMP_MAP | 628 | REAL | — | const used by live session |
| parse_media_playlist_typed | 633-649 | REAL | — | code-per-cause mapping; tested via refusal table |
| playlist_corrupt | 653-660 | REAL | — | |
| map_fetch_err | 664-703 | REAL | — | full NetworkError coverage; `MediaPlaylist` arm admitted unreachable |
| fetch_playlist_model | 712-736 | REAL | — | shared VOD/live prelude; used by live.rs (test-verified there) |
| 8 tests | 738-1134 | REAL | — | all would fail on mutation; F-001 branch untested |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| vod_concatenation_is_byte_exact_and_ordered | YES | exact bytes, size, request/probe/byte counts |
| b1b_shapes_refuse_typed_with_their_codes | YES | exact code strings for all 8 refusals |
| vod_discontinuity_is_recorded_not_refused | YES | full boundary map + byte-exact seam |
| missing_segment_fails_typed_at_open | YES | kind + code match |
| oversize_segment_refuses_over_budget | YES | ResponseTooLarge → ResourceLimit code |
| seek_shapes_are_honest | YES | unretained/out-of-range refusals; budget=8 with 4+6-byte bodies — stops at F-001's edge |
| relative_uris_resolve_against_the_playlist_url | YES | real socket; request order + Range header asserted |
| config_bounds_refuse_typed | YES | zero/over-ceiling codes |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A hostile segment over its budget fails … never allocates" | media.rs:75-77 | REAL | oversize_segment_refuses_over_budget (ran, ok) |
| "seekable within the retained prefix … backward seek below the retained prefix … refuses typed" | media.rs:55-63 | REAL | seek_shapes_are_honest (ran, ok) — but the READ path (not seek) panics when the head is dropped by an oversized body (F-001) |
| "Nothing is allocated ahead" | media.rs:69 | REAL | retained grows lazily via absorb |
| "retain nothing for a single body over the whole budget" | media.rs:74-75 / absorb docs | FALSE-in-effect | the documented state (retained_from=fetched_upto, empty retained) makes the next serve iteration panic instead of serving/refusing (F-001) |

## 7. Dead / duplicated / unreachable code
Whole module unreachable from shipped entry points (F-002). `size()` `.or` fallback dead (F-004). No internal duplication; `fetch_playlist_model`/`resolve_seek_origin`/`map_fetch_err` are shared with live.rs (good).

## 8. Dependencies & flags
Depends on playlists (parse_media_playlist), plugin_api (SourceStream), media_types (error taxonomy), crate::fetch port. No feature gates. Compiled into the streaming lib that the CLI links — dead weight in the release binary.

## 9. Open questions for the next agent
- If/when an HLS open lane lands, F-001 becomes S1 immediately — re-audit then.
- Does the DASH sched module share the probe-per-segment pattern (same open-latency defect)? (dash/sched.rs not in my set.)
