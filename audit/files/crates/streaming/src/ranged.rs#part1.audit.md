# AUDIT: crates/streaming/src/ranged.rs#part1 (lines 1-630)
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-21 |
| Lines / functions | 1-630 (chunk boundary declared: part1 = 1-630, module docs + error minting/mapping + config/types + `RangedResource::open`/`probe_request`/`absorb`/`scan_get`; part2 = 631-1258, scan state machine + `SourceStream` impl + `parse_content_range` + first test block [A10]; part3 = 1259-1887, remaining tests [A08]) / 30 items |
| File verdict | REAL (production code, reachable from shipped CLI; unit tests exist but were NOT executed in baseline — see §5) |
| Reachable from shipped entry point? | YES — `lumen open <http(s)-url>` → apps/cli/src/demux_host.rs:427 (`"http" | "https" => open_http_source(url, fetch)?`) → :488-495 `streaming::ranged::RangedConfig::default()` + `RangedResource::open(fetch, url, config)` (direct) or `streaming::schemes::SchemeRegistry::new().open_http(...)` (schemes.rs:343-348 → `RangedResource::open`) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `rg -n "streaming" apps/cli/Cargo.toml` (prod dep :62); `rg "streaming::" apps/cli/src` (demux_host.rs:383,476,480,488-492 — the open lane); read of ranged.rs:1-761 (part1 + 131 lines of part2 context for the read loop); cross-reads: error.rs (NetworkError variants), fetch.rs:465-536 (HttpResponse/HttpFetch), client.rs:230-242 (transport surfaces only 2xx as Ok, 416→RangeNotSatisfiable, other 4xx/5xx→HttpStatus), playback-engine/src/recovery.rs:277-300 (`is_retryable_network`: ConnectionLost = retryable); `sed -n` quote verifies at 204-218, 542-559, 589-604, 86-93; baseline: test-workspace.log.txt (2412 lines, ABORTS at apps/desktop zero_alloc_transport — streaming unit tests never ran), test-workspace-allfeatures.log.txt:2358-2468+ (ran audio_engine…device_discovery, first_vertical, then ABORTED — playlists/streaming/transcode unit tests never ran) |

## 1. One-paragraph verdict
This is the real network open path of the shipped `lumen` binary and it is genuinely engineered: bounded windows, typed refusals at every protocol seam, i128 seek math, honest stats, and a documented linear-scan fallback for no-range servers. The defects found are in the error taxonomy and protocol-validation seams, not in the happy path: every HTTP status error (404, 403, 410, 500…) is folded into `Network{ConnectionLost}` with the status code **discarded**, so a dead URL is reported to the user as transient connection weather (and would be retried by any consumer of the engine taxonomy's retry table, which currently classifies ConnectionLost as retryable — recovery.rs:293-300); the 206 validator ignores the `Content-Range` end offset, so a body that disagrees with the server's own declared range is accepted and cached unvouched; and under `RequireRange` the promised `no-range-support` refusal is unreachable for any no-range body larger than one chunk (it surfaces as `response-over-budget` instead). None of the crate's unit tests executed in either committed baseline run — both aborted before the streaming crate's test binary ran.

## 2. What actually works (verified)
— Reachability chain verified by read: `lumen open http://…` → demux_host.rs:427→:474-500→`RangedResource::open` (ranged.rs:493) — this code is the shipped binary's HTTP lane.
— `resolve_origin` (255-269): `i128` addition for `SeekOrigin::Current` with explicit `seek_before_start`/`seek_overflow` typed refusals — traced by hand for from=0,delta=-1 (Err), from=u64::MAX,delta=1 (Err overflow), from=5,delta=-5 (Ok(0)). No panic path.
— `RangedConfig::validate` (351-375): zero-window/zero-chunk → typed `config-zero`; window > 1 GiB schema ceiling → typed `window-over-budget` (refused, not clamped); chunk > window → typed `chunk-over-window`. Traced all three arms.
— `RangedResource::open` (493-562): probe is `Range: bytes=0-(chunk-1)` with transport `response_limit == chunk_bytes` (probe_request 565-572); 416→empty resource `len=Some(0)` with the request counted; `ResponseTooLarge` under `FallbackToScan` → one rangeless re-probe under the window budget (`scan_get`, 623-630). Stats accounting traced: probe-in-`absorb` (+1 request), manual +1 for the refused probe, +1 in `scan_get`; `bytes_fetched` only counts delivered bodies.
— `absorb` (581-617) 206 path: `Content-Range` start must equal the requested start (592 `if rstart != start` → protocol violation); shrunken total → typed `truncated` (596-599); 200 under `RequireRange` → typed `no_range_support` (606-608); non-200/206 → `protocol_violation` (614-615) — the transport only surfaces 2xx as Ok (client.rs:235-242), so this arm is a correct defensive default (e.g. a 204).
— Bounded-memory contract holds in this part: every fetch carries an explicit `response_limit` (`probe_request` :571, `scan_get` :625); no allocation derives from server-controlled counts here.

## 3. Findings

### F-001 [S2]
Where: crates/streaming/src/ranged.rs:204-251 (`map_fetch_err`, arms at :211-217)
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
What is wrong: every `NetworkError::HttpStatus` — a **complete, permanent server answer** (404 Not Found, 403, 410, 500…) — is folded into `NetworkCause::ConnectionLost` under the single code `streaming.ranged.fetch-failed`, and the `{ code, is_client_error, is_server_error }` payload is discarded by the `{ .. }` pattern. The transport dutifully classifies statuses (client.rs:230-242, error.rs:70-78), and this bridge throws the classification away.
Why it matters: reachable from the shipped CLI — `lumen open http://dead.example/x.mp4` produces the error kind `Network{ConnectionLost}` ("network: connection lost", media-types/src/error.rs:359) with no mention of 404: the user is told the connection died when the server answered explicitly and permanently. Worse, the engine taxonomy's retry table treats this cause as retryable weather — `playback-engine/src/recovery.rs:293-300` (`is_retryable_network`: `Timeout | ConnectionLost | Dns` → retry, "stay in Buffering and keep retrying"), so any consumer wiring that guard (it is exported but currently unwired — verified: no callers outside recovery.rs/lib.rs/tests) will re-request a permanent 404 until its window is spent. A permanent client error is misclassified as transient.
What correct looks like: map `HttpStatus { code, is_client_error, .. }` to a non-retryable kind (e.g. `Inaccessible{NotFound}` for 404/410, `AccessDenied` for 403) or at minimum a distinct `NetworkCause` that is not in any retry table, and carry the status code in the `ErrorContext` (the `NetworkError` vocabulary already has `status_code()` for exactly this).
Confidence: HIGH — Verification: read (ranged.rs:204-251, error.rs:70-78/185-193, client.rs:230-242, recovery.rs:277-300 all read; reachability chain demux_host.rs:427→474-500 read).

### F-002 [S3]
Where: crates/streaming/src/ranged.rs:589-604 (`absorb`, 206 validation)
Code:
```
        if resp.status == 206 {
            let (rstart, _rend, total) = parse_content_range(resp.header("Content-Range"))?;
            if rstart != start {
                return Err(protocol_violation());
            }
```
What is wrong: the 206 validator checks only that the `Content-Range` **start** matches the request; the parsed **end** (`_rend`) is discarded and never compared against `resp.body.len()`. A server that declares `Content-Range: bytes 0-99/1000` but delivers a 300-byte body (or declares 0-999 and delivers 10 bytes that are not the tail) passes validation; the full body becomes a cached window covering `[start, start+body.len())` — bytes the server never vouched for, and a length bookkeeping that trusts body over header.
Why it matters: RFC 9110 §14.4 requires a single-part 206's `Content-Range` to describe exactly the bytes sent; the disagreement is precisely the signal of a hostile or broken cache/CDN, and this module's own posture is "a `206` without a parseable `Content-Range`, or one that disagrees" is a protocol violation (doc :193-194) — disagreement is checked for `start` but not for extent. The doc promise and the code differ.
What correct looks like: after parsing, require `body.len() as u64 == rend - rstart + 1` (or treat a short body as the typed truncation this module already mints at :596-599) and refuse an over-long body as `protocol_violation`.
Confidence: HIGH — Verification: read (589-604; `parse_content_range` at :953-979 read in part2's range for cross-reference; window insertion semantics via `insert_window` :753-756 read).

### F-003 [S3]
Where: crates/streaming/src/ranged.rs:552-559 (open, `ResponseTooLarge` arm) + :605-608 (absorb 200 arm)
Code:
```
            Err(NetworkError::ResponseTooLarge { .. })
                if resource.config.no_range == NoRangePolicy::FallbackToScan =>
            {
                resource.stats.requests += 1;
                let resp = resource.scan_get()?;
                resource.enter_scan(resp)?;
            }
            Err(err) => return Err(map_fetch_err(err)),
```
What is wrong: under `NoRangePolicy::RequireRange`, a no-range server whose body exceeds `chunk_bytes` never reaches `absorb`'s `200` arm — the transport refuses it as `ResponseTooLarge` first (probe `response_limit` is the chunk), so the arm at :605-608 (`no_range_support()`) is dead for large bodies and the caller gets `streaming.ranged.response-over-budget` instead. The module doc (:82-84) promises "under [`NoRangePolicy::RequireRange`] the same answer is the typed `streaming.ranged.no-range-support` refusal, **both at open and mid-stream**".
Why it matters: two servers with identical behavior (ignore `Range:`) produce different typed errors depending only on whether the body fits one chunk — the `RequireRange` caller cannot distinguish "server has no range support" from "server hostile/oversized", which is exactly the distinction that policy exists to make.
What correct looks like: under `RequireRange`, treat a probe `ResponseTooLarge` as a suspected no-range shape and re-probe rangeless under the window budget with the sole purpose of classifying it (then refuse `no_range_support` if it is a 200 full body), or amend the doc to state the budget interaction.
Confidence: MEDIUM — Verification: read (552-559, 605-608, doc :78-93; transport `response_limit` semantics fetch.rs:501-513 read; no test pins the RequireRange+oversize combination — tests `no_range_server_is_refused_typed_at_open` :1313 and `scan_body_over_the_window_budget_is_refused_typed` :1670 cover the two halves separately, in part2/part3 ranges).

### F-004 [S4]
Where: crates/streaming/src/ranged.rs:493-503 (`RangedResource::open`, double URL parse)
Code:
```
        crate::fetch::Url::parse(url).map_err(map_fetch_err)?;
        let uri = media_types::ids::SourceUri::parse(url)?;
```
What is wrong: the same URL string is parsed through two independent grammars back-to-back, and the two results are both kept (`self.url` for requests, `self.uri` for the session identity). A URL that one grammar accepts and the other rejects fails with whichever error comes first, and the two parses can theoretically disagree on identity (redirect handling aside).
Why it matters: duplicated grammar, doubled parse cost on open, and a latent divergence seam — the kind of "two definitions of the same thing" the repo's own ARCH rules forbid elsewhere. Minor because both parsers are strict and the CLI pre-parses with `fetch::Url` too (demux_host.rs:480).
What correct looks like: parse once through the port's grammar and derive the `SourceUri` from it (or document that the two grammars are deliberately pinned to agree and pin it with a test).
Confidence: LOW (severity LOW; the mechanical fact is HIGH-confidence read) — Verification: read (493-503; demux_host.rs:480-493 read).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-93 | REAL | — | Accurate contract text; one gap = F-003 (RequireRange promise vs budget interaction). Claims match code otherwise (traced: budget/LRU/scan semantics). |
| `DEFAULT_WINDOW_BYTES` | 105-109 | REAL | — | 64 MiB, mirrors product.toml claim (cross-check: product.toml not re-read this session — noted). |
| `MAX_WINDOW_BYTES` | 114 | REAL | — | 1 GiB schema ceiling. |
| `DEFAULT_CHUNK_BYTES` | 119 | REAL | — | 256 KiB. |
| `MAX_READ_BYTES` | 126 | REAL | — | Restated input-crate ceiling; consumed by `read()` in part2 (:857-930). |
| `STAGE` | 131 | REAL | — | Const. |
| `mint` | 135-137 | REAL | — | Trivial, correct. |
| `seek_out_of_range` | 141-148 | REAL | — | Error ctor; input's stable code. |
| `seek_before_start` | 151-158 | REAL | — | Error ctor. |
| `seek_overflow` | 161-168 | REAL | — | Error ctor. |
| `truncated` | 173-180 | REAL | — | Error ctor w/ expected/found payload. |
| `no_range_support` | 186-191 | REAL | — | Error ctor. |
| `protocol_violation` | 195-200 | REAL | — | Error ctor; no detail string (see F-002: which disagreement is invisible). |
| `map_fetch_err` | 204-251 | REAL | S2 | F-001: HttpStatus payload discarded → ConnectionLost; also `Cancelled`→ConnectionLost is deliberate+documented (:218-224). |
| `resolve_origin` | 255-269 | REAL | — | i128 math, both overflow directions typed; traced 3 cases by hand. |
| `Window::len` / `covers` | 279-286 | REAL | — | Correct range math. |
| `NoRangePolicy` | 298-308 | REAL | — | Enum; default FallbackToScan. |
| `RangedConfig` + `Default` | 313-339 | REAL | — | Plain data. |
| `RangedConfig::validate` | 351-375 | REAL | — | All three refusals traced; no clamping. |
| `RangedStats` | 382-403 | REAL | — | Doc admits "no production path reads them" — honest; only tests/fuzz assert. |
| `Mode` | 409-415 | REAL | — | Enum. |
| `ScanBuffer::covers` | 430-432 | REAL | — | Correct. |
| `Absorbed` | 437-443 | REAL | — | Enum. |
| `RangedResource` (struct) | 452-472 | REAL | — | Fields documented; `windows`/`scan` mutual exclusion maintained by `enter_scan` (part2 :664-665). |
| `RangedResource::open` | 493-562 | REAL | S3 | F-003 (RequireRange budget gap); 416-empty and re-probe arms traced; stats accounting correct. |
| `probe_request` | 565-572 | REAL | — | Bounded range + response_limit; correct. |
| `absorb` | 581-617 | REAL | S3 | F-002 (`_rend` ignored); 200/RequireRange and truncation arms correct. |
| `scan_get` | 623-630 | REAL | — | Rangeless GET under window budget; stats counted. |

## 5. Test honesty (tests for part1 functions live in part2/part3 ranges — cross-reference)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `no_range_server_is_refused_typed_at_open` (:1313, part3) | YES | Serves a 200 body under `RequireRange`, asserts the typed refusal — kills `absorb`'s 200 arm regression. |
| `scan_fallback_reads_the_full_body` (:1525, part3) | YES | Serves a no-range body larger than the chunk; asserts scan-mode content — exercises open's ResponseTooLarge arm + scan_get. |
| `sequential_reads_equal_the_served_bytes` (:1046, part2) | YES (probe) | First read comes from the probe window; a broken probe/absorb breaks it. |
| `config_bounds_are_typed_refusals` (:1411, part3) | YES | Pins validate()'s three refusals. |
| **Baseline execution** | **NEVER RAN** | Both committed baseline test runs aborted BEFORE the streaming crate's test binary: test-workspace.log.txt (2412 lines) dies at apps/desktop `zero_alloc_transport` (cargo fail-fast); test-workspace-allfeatures.log.txt runs audio_engine→device_discovery→first_vertical then dies at `evidence.rs`. The INDEX.md "1681 passed" headline does not cover streaming. These tests are green in CI-claims only, unproven in the committed baseline. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "A seek beyond a known length fails typed before any request goes out (`input.seek.out-of-range`)" | module doc :14-18 | REAL (part2 `seek` :932-943 cross-read) | seek_out_of_range minted here :141-148; length check read in part2. |
| "under RequireRange the same answer is the typed `streaming.ranged.no-range-support` refusal, both at open and mid-stream" | module doc :82-84 | SHALLOW | F-003: unreachable for bodies > one chunk (transport refuses first). |
| "a hostile over-long body fails `ResponseTooLarge` instead of allocating" | module doc :28-30 | REAL | probe_request :571 / scan_get :625 set response_limit; transport budget read (fetch.rs:501-513). |
| "A 416 to the probe is honored as an empty resource (`size() == Some(0)`)" | open doc :491-492 | REAL | :535-541 read. |
| product.toml `limits.cache_network_mib = 64` mirrored | :32-36 | REAL-UNPROVEN | Constant matches; product.toml wiring ("Live product-config plumbing lands with the http open-path card") — the doc itself admits the join has NOT landed: RangedConfig::default() is hard-wired in the CLI (demux_host.rs:488). |

## 7. Dead / duplicated / unreachable code
None dead within part1. Note for the wave: `RangedStats` has no production reader (documented); `MAX_READ_BYTES` duplicated from the input crate by design (comment :121-126).

## 8. Dependencies & flags
Uses `media-types` (error taxonomy), `plugin-api` (SourceStream/SeekOrigin), crate-local `error`/`fetch`. No feature flags gate this file; `lumen-cli` depends on `streaming` unconditionally (apps/cli/Cargo.toml:62) — note this dep is itself an undeclared architecture edge (baseline #11, check-architecture FAILURE, gate-check_architecture.log.txt).

## 9. Open questions for the next agent
- A10 (part2): does the `read`/`read_into` loop handle a **zero-length 206 body** (empty window inserted at the requested offset) with a progress check, or can a hostile server spin it? `fetch_window` returning `Ok(0)` for an empty body is reachable via a server that answers `206` + `Content-Range: bytes N-(N-1)/*`.
- A10: `parse_content_range` (:953-979) — confirm it rejects `bytes */total` forms (probe expects a start).
- Wave gate: baseline evidence gap — `cargo test -p streaming --no-fail-fast` was never captured (see §5). Logged as REQUEST-ORCH-RUN.
