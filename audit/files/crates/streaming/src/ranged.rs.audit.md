# AUDIT: crates/streaming/src/ranged.rs
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-12 |
| Lines / functions | 1886 (979 production + 907 tests) / 37 production fns + 24 tests + 6 helpers + 5 mock impls |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: lumen-cli `apps/cli/src/demux_host.rs:427` `"http" | "https" => open_http_source(url, fetch)?` → `demux_host.rs:490-491` `RangedResource::open` / `SchemeRegistry::new().open_http` → `crates/streaming/src/schemes.rs:348` `RangedResource::open(Arc::clone(&self.fetch), ...)` with `schemes.rs:238` `fetch: Arc::new(UreqClient::new())` (real ureq transport) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 4 |
| Commands run | rg (RangedResource/UreqClient/tls/If-Range/etag/retry), sed -n (quotes verified), baseline citations only (no cargo run: cargo-build-budget not needed, tests already in baseline) |

## 1. One-paragraph verdict
This is a real, thorough implementation of a bounded-memory ranged HTTP resource: correct seek math (i128 intermediate, typed before-start/overflow/out-of-range), correct window cache with LRU eviction that provably never evicts the window being served, a working no-range linear-scan fallback with honest refetch/skip accounting, and 24 baseline-passing unit tests that pin exact request-log shapes (`bytes=5000-6023`, rangeless re-probe, resync counts). All range requests are bounded `bytes=start-(end-1)` (never open-ended `bytes=N-`); unknown-length streams are judged by the store (416/short body = clean EOF); 416 mid-read on a known length is typed `Corrupt::Truncated`. The transport-level memory story is real: the ureq client enforces `response_limit` before allocation (client.rs:282-320). The defects are classification and validation gaps, not fakeness: every HTTP status failure (incl. 403/404 from an expired signed URL) is mapped to `Network{ConnectionLost}`, which the playback engine's `is_retryable_network` treats as retryable weather — permanent errors burn the retry budget and the status code is destroyed; no If-Range/ETag/validator exists anywhere in the crate; the 416-at-open path discards the standard `Content-Range: bytes */N` total; `parse_content_range` ignores the parsed `end` entirely.

## 2. What actually works (verified)
— verified by orchestrator baseline `audit/runs/baseline/cargo-test-workspace.txt:5399-5733` → all 24 `ranged::tests::*` pass, package result "212 passed; 0 failed; 2 ignored".
— verified by baseline `cargo-test-workspace.txt:3061` `test demux_host::tests::http_url_opens_through_the_ranged_resource ... ok` and `:3251` `test http_url_opens_over_a_real_loopback_socket ... ok` (real `lumen open http://...` through the production ureq transport, exit 0).
— verified by code read: seek math (L255-269 i128, no overflow), eviction invariant (L786-798 `windows.len() > 1` guard keeps the fresh window), scan-mode drain math (L680-733), no unwrap/expect/panic in production code.

## 3. Findings
F-001 [S2]
Where: crates/streaming/src/ranged.rs:211-217 (+ crates/playback-engine/src/recovery.rs:293-300)
Code:
```
        NetworkError::ConnectRefused
        | NetworkError::BodyTruncated { .. }
        | NetworkError::TooManyRedirects { .. }
        | NetworkError::RedirectLoop { .. }
        | NetworkError::CrossHostRedirect { .. }
        | NetworkError::CrossSchemeRedirect { .. }
        | NetworkError::HttpStatus { .. } => NetworkCause::ConnectionLost,
```
What is wrong: every `HttpStatus` — including permanent 4xx (403 expired signed URL, 404 deleted resource, 410 gone) and 5xx — is folded into `Network{cause: ConnectionLost}` and the status code is discarded. `playback-engine/src/recovery.rs:293-300` `is_retryable_network` returns `true` for `NetworkCause::ConnectionLost`, and `recovery.rs:506` gates the live-source retry window on it, so a permanent 403/404 is retried as "transient network weather" until the budget is exhausted, and the final error (`source.retry.exhausted` chained to `Network{ConnectionLost}`) never tells the user the server said 403. Signed CDN URLs expiring mid-playback is the normal case, not the edge. Why it matters: minutes of pointless Buffering retries plus an error message that hides the actual cause. What correct looks like: map `HttpStatus{is_client_error}` to a non-retryable cause (or `Inaccessible/AccessDenied`), preserve the code in `ErrorContext`. Confidence: HIGH — Verification: read (mapping cited; `is_retryable_network` body cited; 401-only interception confirmed in auth.rs:534).

F-002 [S3]
Where: crates/streaming/src/ranged.rs:594-602
Code:
```
            if let Some(total) = total {
                match self.len {
                    Some(known) if total < known => {
                        // The resource shrank below its open-time
                        // length: typed truncation, never silent.
                        return Err(truncated(known, total));
                    }
                    _ => self.len = Some(total),
                }
            }
```
What is wrong: shrink is detected, but growth silently overwrites the length and no validator (If-Range/ETag/Last-Modified) is ever sent or checked — `rg -i "if-range|etag|last-modified" crates/streaming/` returns nothing. A resource replaced mid-stream with a same-or-larger length (regenerated file, CDN edge inconsistency) yields silently mixed-version bytes across window fetches. Why it matters: byte-exact reads are the module's core promise ("byte-exact through the bounded window" is asserted in tests) and version skew breaks it invisibly. What correct looks like: capture ETag/Last-Modified at open and send `If-Range` on subsequent range fetches (RFC 9110 §13.1.5 / §14.5), or document the no-validator limitation. Confidence: HIGH — Verification: read + rg (exit 1, no matches).

F-003 [S3]
Where: crates/streaming/src/ranged.rs:535-541
Code:
```
            Err(NetworkError::RangeNotSatisfiable) => {
                // `bytes=0-(chunk-1)` unsatisfiable: nothing exists at
                // offset 0 — the empty-resource shape. The probe did go
                // out on the wire; count it.
                resource.stats.requests += 1;
                resource.len = Some(0);
```
What is wrong: a 416 is interpreted as "empty resource" (`size() == Some(0)`) without reading the 416's `Content-Range: bytes */N` unsatisfied-range value (the RFC 9110 §15.5.17 standard shape, which names the real total) and without inspecting the 416 body. Any server that 416s the probe for a non-empty reason (policy, suffix-only ranges, broken impl) opens as a zero-byte resource and every read is a clean EOF — silent wrong data, no typed error. Why it matters: a probe 416 is the one status where the standard tells you the truth and this code throws it away. What correct looks like: parse `bytes */N` from the 416's Content-Range; if N > 0, either use it or fail `protocol-violation`. Confidence: HIGH — Verification: read (the response object is dropped; only `stats.requests` is touched).

F-004 [S3]
Where: crates/streaming/src/ranged.rs:590 and 953-979
Code:
```
            let (rstart, _rend, total) = parse_content_range(resp.header("Content-Range"))?;
            if rstart != start {
                return Err(protocol_violation());
            }
```
What is wrong: `parse_content_range` parses `end` but the caller discards it (`_rend`), and nothing validates `end >= start`, `end < total`, or `body.len() == end - start + 1`. A 206 whose body does not match its declared range (short or long) is served as window data at face value; only `response_limit` (= chunk) caps the damage. Why it matters: the Content-Range cross-check is the ranged protocol's integrity check; skipping it means a lying server silently skews the byte stream. What correct looks like: reject `end < start`, `end+1 > total`, and `body.len() != end - start + 1` as `protocol-violation`. Confidence: HIGH — Verification: read (L967 parses `end`; grep of `_rend` shows sole use discarded).

F-005 [S4]
Where: crates/streaming/src/ranged.rs:552-559
Code:
```
            Err(NetworkError::ResponseTooLarge { .. })
                if resource.config.no_range == NoRangePolicy::FallbackToScan =>
            {
                resource.stats.requests += 1;
                let resp = resource.scan_get()?;
                resource.enter_scan(resp)?;
            }
```
What is wrong: under `RequireRange`, a no-range server whose full body exceeds the chunk budget surfaces `streaming.ranged.response-over-budget` (via the fall-through `Err(err) => return Err(map_fetch_err(err))`), not `no-range-support` — the re-probe that would distinguish the two shapes is gated on the fallback policy only. The caller cannot tell "server ignores Range" from "hostile over-long body". Confidence: MEDIUM — Verification: read. Severity S4 (typed refusal either way; classification only).

F-006 [S4]
Where: crates/streaming/src/ranged.rs:871
Code:
```
        let mut out = vec![0u8; len];
        let n = self.read_into(&mut out)?;
        out.truncate(n);
```
What is wrong: `read(len)` allocates `len` bytes up front (ceiling `MAX_READ_BYTES` = 64 MiB) regardless of resource size; a 1 KiB resource read with len=64 MiB transiently allocates 64 MiB. Bounded and documented (L121-126), but allocation-before-knowledge is avoidable (clamping to known length, or chunked read_into). Confidence: HIGH — Verification: read.

F-007 [S4]
Where: crates/streaming/src/ranged.rs:802-809
Code:
```
    fn take_hit(&mut self, pos: u64) -> Option<(usize, usize)> {
        let idx = self.windows.iter().position(|w| w.covers(pos))?;
        let window = self.windows.remove(idx)?;
        let off = (pos - window.start) as usize;
        let len = window.len();
        self.windows.push_back(window);
        Some((off, len))
    }
```
What is wrong: linear scan over all windows per read step plus `VecDeque::remove(idx)` O(n) memmove — repeated linear search on the read path. Default config caps windows at 256 (64 MiB / 256 KiB), so the cost is negligible vs network I/O; noted as a pattern, not a hot defect. Confidence: HIGH — Verification: read.

F-008 [S4]
Where: crates/streaming/src/ranged.rs:754-757 and 786-789
Code:
```
                Absorbed::Ranged(body) => {
                    let got = body.len();
                    self.insert_window(Window { start, data: body });
                    Ok(got)
                }
```
What is wrong: a 206 with an empty body inserts a zero-length window; zero-length windows never satisfy `covers()` (L284-286 `pos - self.start < self.data.len()` is false for len 0), add 0 to `cached`, and therefore never trigger the `cached > window_bytes` eviction — a server answering empty 206s at EOF plus a caller polling `read()` grows `windows` without bound (a few dozen bytes of struct per poll). No crash; slow structural leak. Confidence: MEDIUM — Verification: read + mutation reasoning; no test covers empty-206 bodies (UNVERIFIED by execution).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint | 135-137 | REAL | — | stage-Open error ctor |
| seek_out_of_range / seek_before_start / seek_overflow | 141-168 | REAL | — | stable input-contract codes |
| truncated | 173-180 | REAL | — | Corrupt::Truncated{expected,found} |
| no_range_support / protocol_violation | 186-200 | REAL | — | typed refusals |
| map_fetch_err | 204-251 | REAL | S2 | F-001: HttpStatus→ConnectionLost destroys code, feeds engine retry |
| resolve_origin | 255-269 | REAL | — | i128 math, correct bounds |
| Window::len/covers | 279-286 | REAL | — | |
| RangedConfig::Default/validate | 331-375 | REAL | — | zero/ceiling/chunk-over-window all typed |
| ScanBuffer::covers | 430-432 | REAL | — | |
| RangedResource::open | 493-562 | REAL | S3 | F-003 416→empty; F-005 RequireRange miscode |
| probe_request | 565-572 | REAL | — | `bytes=0-(chunk-1)`, limit=chunk |
| absorb | 581-617 | REAL | S3 | F-002 growth silent, F-004 `_rend` unused |
| scan_get | 623-630 | REAL | — | rangeless GET under window budget |
| enter_scan | 640-672 | REAL | — | Content-Length cross-check; shrink typed |
| scan_refetch | 680-691 | REAL | — | skip=[0,pos), counted |
| scan_fill | 698-719 | REAL | — | EOF-safe (returns 0 when body short) |
| scan_trim | 724-733 | REAL | — | retention=[pos,end) |
| fetch_window | 738-777 | REAL | S4 | F-008 empty-window insert; 416 mid-read typed/EOF correct |
| insert_window | 786-798 | REAL | — | fresh window never evicted (len()>1 guard) |
| take_hit | 802-809 | REAL | S4 | F-007 linear scan |
| copy_hit | 812-823 | REAL | — | |
| cached_bytes/stats/length | 828-841 | REAL | — | live accessors used by tests |
| SourceStream::{uri,size,supports_seek} | 845-855 | REAL | — | |
| read | 857-875 | REAL | S4 | F-006 pre-alloc; MAX_READ_BYTES typed |
| read_into | 877-930 | REAL | — | loop always makes progress (analyzed: no infinite loop) |
| seek | 932-943 | REAL | — | past-len typed, ==len legal, unknown-len free |
| position | 945-947 | REAL | — | |
| parse_content_range | 953-979 | REAL | S3 | F-004 no end/total/body cross-checks |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| sequential_reads_equal_the_served_bytes | YES | byte-exact reconstruction + sticky EOF + position tracking |
| seek_back_rereads_from_cache_with_no_requests | YES | asserts request-count equality against the mock log |
| seek_forward_beyond_read_ahead_fetches_at_the_right_offset | YES | pins literal `Range: bytes=5000-6023` header from the log |
| forward_scan_holds_the_window_budget | YES | peak cached_bytes ≤ 4096, evictions>0, fetched<60_000 |
| eof_short_read_is_data_then_clean_zero | YES | short read = data; then Ok(0); seek==len legal |
| seek_past_end_is_the_input_contract_error | YES | asserts stable code + no wire request + cursor unmoved |
| mid_read_416_is_a_typed_truncation | YES | asserts kind destructuring (5000,1024) |
| small_file_smaller_than_the_window | YES | requests==1 |
| no_range_server_is_refused_typed_at_open | YES | code assertion via mock `honor_range:false` |
| unknown_length_server_is_honest | YES | size None; star-total wrapper rewrites header |
| config_bounds_are_typed_refusals | YES | three typed codes + valid passes |
| oversized_read_is_refused_typed | YES | MAX_READ_BYTES+1 |
| content_range_parsing_is_strict | YES | 5 hostile shapes refused |
| cache_invariant_holds_across_a_seek_storm | YES | budget asserted at every seek target |
| default_policy_is_the_scan_fallback | YES | constants pinned |
| scan_fallback_reads_the_full_body | YES | requests==2, rangeless re-probe pinned, bytes==10_000 |
| scan_seek_forward_within_the_buffer_is_free | YES | retention arithmetic asserted |
| scan_seek_backward_refetches_once_from_byte_zero | YES | exactly 1 refetch, rangeless, skipped==512 |
| scan_refetch_accounting_is_exact_across_rewinds | YES | counters exact across 4 requests |
| scan_body_over_the_window_budget_is_refused_typed | YES | ResourceLimit budget/attempted (4096,5000) |
| scan_tiny_file_is_served_by_the_probe_alone | YES | requests==1 |
| mid_stream_200_resyncs_into_scan_and_recovers | YES | ranged-log vector [t,t,t,t,f] + resyncs==1 |
| mid_stream_200_under_require_range_is_typed | YES | typed mid-stream refusal |
| scan_content_length_disagreement_is_a_protocol_violation | YES | lying Content-Length refused |
All 24 run in baseline (no #[ignore] in this module). Weakness: every test runs against MockServer/scripted `HttpFetch` — the real-transport behaviors (ureq redirect policy, response_limit enforcement, TLS) are pinned only in client.rs/schemes.rs suites, and no test covers an empty-206 body (F-008) or a Content-Range lying about `end` (F-004).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A server that shrinks the resource mid-stream (416, or a smaller Content-Range total) surfaces the same typed truncation" | ranged.rs:19-23 module doc | REAL | L594-602 + L768-774; tests mid_read_416 / shrink arms; baseline pass |
| "Live product-config plumbing lands with the http open-path card; the constants here are the documented defaults" | ranged.rs:33-36 | REAL | `limits.cache_network_mib = 64` claimed default matches DEFAULT_WINDOW_BYTES; **but** demux_host.rs:488 uses `RangedConfig::default()` — product.toml value never plumbed: user's configured cache size is ignored on the CLI open path (see §9) |
| "The 6.A1a port returns whole bodies ... Reading N bytes forward costs ceil(N/chunk_bytes) requests" | ranged.rs:40-46 | REAL | client.rs single_hop whole-body read confirmed |
| feature-matrix:1288 "open path's http arm, the production ureq transport + RangedResource ... loopback e2e through the REAL binary, exit 0" | docs/feature-matrix.toml | REAL | baseline cargo-test-workspace.txt:3251 `http_url_opens_over_a_real_loopback_socket ... ok` |
| doc "bounded response memory (MP §12 — HttpRequest::response_limit)" | crates/streaming/Cargo.toml:13 | REAL | client.rs:282-320 advertised-length + take(limit+1) enforcement |

## 7. Dead / duplicated / unreachable code
None dead. `RangedStats` is test/telemetry-only by its own admission (L378-380 "no production path reads them") — acceptable, documented. MAX_READ_BYTES duplicates input's constant by documented design (L121-126).

## 8. Dependencies & flags
Depends on: media-types, plugin-api (workspace), crate::fetch (port), crate::error. No HTTP client here — the transport is injected (`Arc<dyn HttpFetch>`); production wiring is ureq 3.5.0 (`=3.5.0`, default-features=false, features `["rustls","rustls-webpki-roots"]`, Cargo.toml:52) — **TLS is real rustls (ring 0.17.14), never native-tls**; verification always on (client.rs:91-96 `TlsProvider::Rustls` + `RootCerts::WebPki`; test `agent_config_pins_transport_safety` asserts `!tls.disable_verification()`, client.rs:926). No in-tree test performs a real TLS handshake — https_activation.rs:24-27 admits "a real TLS handshake ... cannot prove in-tree ... [VERIFY: needs a networked CI lane]". So: ureq = YES (verified), TLS stack = rustls real (verified config + code path), TLS handshake end-to-end = REAL-UNPROVEN (no networked lane; out-of-band record R10 claimed).

## 9. Open questions for the next agent
- Who plumbs `product.toml` `limits.cache_network_mib` into `RangedConfig`? demux_host.rs:488 hardcodes `RangedConfig::default()` — if no caller reads the product config, the doc's "documented defaults for that join" is a promise still unfulfilled on the shipped CLI path (config declared but never read — check product-config crate consumers).
- Does the desktop (Tauri) open path wire http sources at all, or is ranged CLI-only? (desktop build unverifiable in sandbox per baseline.)
- Engine retry window size for `Network{ConnectionLost}` (F-001 blast radius): how many attempts/seconds before `source.retry.exhausted`? (recovery.rs guard params.)
