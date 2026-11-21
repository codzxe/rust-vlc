# AUDIT: crates/streaming/src/ranged.rs#part2 (lines 629-1256)
| Field | Value |
|---|---|
| Auditor | W10-A10 |
| Date | 2026-11-21 |
| Chunk boundary | part2 = lines 629-1256, per wave-00 assignments.tsv LOC split (part1=628 lines → A09: 1-628; part2=628 → this audit; part3=630 → A08: 1257-1886). Part2 = tail of `impl RangedResource` (scan-mode machinery, window cache, `impl SourceStream`, `parse_content_range`) + the tests module from 981 through `mid_read_416_is_a_typed_truncation`'s `Shrinker` fixture (its asserts live in part3). |
| Lines / functions | 628 / 27 (11 prod fns/methods + 7 trait-impl methods + 1 free fn + 7 test helpers/fixures + 1 test with asserts in-range + 6 tests fully in-range … see table) |
| File verdict | REAL (reachable from the shipped CLI's `http(s)` open lane; whole test module green in baseline) |
| Reachable from shipped entry point? | YES — `lumen open http://…` → apps/cli/src/demux_host.rs:362 `open_source` → :368 `open_source_with(…, None)` → :427 `open_http_source` → :491 `SchemeRegistry::new().open_http` (schemes.rs:236-240 wraps `UreqClient::new()`; schemes.rs:348 → `RangedResource::open`) → this chunk's `SourceStream` impl serves `media_probe::probe_stream` (demux_host.rs:446) and `crate::containers::open` (demux_host.rs:458). Executed over a real loopback socket by baseline tests (below); execution through the real ureq/rustls transport against a remote host is UNVERIFIED (sandbox forbids network). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | `rg -n "RangedResource" crates apps` (callers: demux_host.rs:490, schemes.rs:348, tests) · `rg -n "UreqClient|SchemeRegistry::new" ` (schemes.rs:236-238 production ctor) · `rg -n "with_retrying_fetch|RetryFetch"` (production never wraps reconnect — test-only callers) · `rg -n "cache_network_mib"` (parsed by product-config merge.rs:432, never mapped to `RangedConfig`) · `rg -n "window_bytes\s*[:=]"` (no production site sets it from config) · `sed -n` quote verification (all quotes below) · baseline `orchrun-test-workspace-nofailfast.log.txt:5087-5132` (all 25 `ranged::tests` ok), `:5260-5267` (`ranged_wav_e2e` 3/3 ok) |

## 1. One-paragraph verdict
This is the real core of the network access lane: scan-mode fallback, LRU window cache with an eviction loop that provably never evicts the window being served, sticky typed EOF/truncation semantics, and a `SourceStream` impl the shipped CLI genuinely dispatches to for every `http(s)` open. The test module is aggressive and honest — it pins exact `Range:` header strings, zero-request cache rewinds, live memory-bound peaks, and error codes, and every one of those tests ran green in the baseline no-fail-fast run, plus a WAV-over-HTTP e2e that demuxes byte-exact. The rot is at the edges: an empty-body `206` silently EOFs a known-length stream in direct contradiction of the module's own "never a silent EOF" contract, and the shipped CLI hard-codes `RangedConfig::default()` — the documented `performance.cache-network-mib` user setting is parsed by product-config and then wired to absolutely nothing.

## 2. What actually works (verified)
— All 25 `ranged::tests` green in baseline `orchrun-test-workspace-nofailfast.log.txt:5087-5132` (run of `cargo test --workspace --no-fail-fast`, release profile), including this chunk's `sequential_reads_equal_the_served_bytes`, `seek_back_rereads_from_cache_with_no_requests`, `seek_forward_beyond_read_ahead_fetches_at_the_right_offset`, `forward_scan_holds_the_window_budget`, `eof_short_read_is_data_then_clean_zero`, `seek_past_end_is_the_input_contract_error`, `mid_read_416_is_a_typed_truncation`.
— End-to-end over a real loopback socket: `ranged_wav_e2e` 3/3 ok (`:5260-5267`: `wav_over_http_demuxes_to_the_on_disk_truth`, `wav_over_no_range_http_demuxes_to_the_on_disk_truth`) — RangedResource as `SourceStream` feeding the real demuxer.
— Hand-traced the LRU invariant: `insert_window` (786-798) evicts only while `windows.len() > 1`, front-only, and `RangedConfig::validate` (ranged.rs:368-373, part1) enforces `chunk_bytes <= window_bytes`, so the just-inserted back window can never be the evicted one — no refetch loop.
— Hand-traced `scan_refetch` termination: `enter_scan` (657-662) turns any shrink below the known length into `Err(truncated)` before `scan_fill`'s defensive `_ => return Ok(0)` arm can loop; a fresh body that doesn't reach `pos` is impossible for `pos < len` bodies (length cross-check at 650-655).
— Call chain to shipped binary verified by `rg` (header above); `lumen engine-info`'s decoder list is irrelevant here — this lane is access, upstream of decode.

## 3. Findings

### F-001 [S3]
Where: crates/streaming/src/ranged.rs:916-922 (with 753-757 and module doc 19-23)
Code:
```
                    if got == 0 {
                        // Unknown-length store judged EOF, or a clean
                        // short-of-chunk end (known-length shortfalls
                        // surface as typed truncation inside
                        // fetch_window).
                        break;
                    }
```
What is wrong: `fetch_window` returns `Ok(body.len())` for a `206` whose `Content-Range` validated — including a **zero-length body** (753-757: `let got = body.len(); … Ok(got)`). With a known length and `pos < len`, a server that answers a valid `206` + `Content-Range: bytes p-q/total` framing with `Content-Length: 0` drives `read_into` to `break` — a clean silent EOF while the resource claims more bytes. The comment's claim ("known-length shortfalls surface as typed truncation inside fetch_window") is false for this shape: `fetch_window` only mints `truncated` on a 416 or a shrinking total, never on an empty 206 body.
Why it matters: the module doc (19-23) makes "never a silent EOF" the typed contract, mirrored from the file source; this is the one wire-shape that violates it. A demuxer sees premature EOF and either stops early (data loss, no diagnostic) or reports its own generic EOF error instead of `streaming.ranged.truncated`.
What correct looks like: in `fetch_window`'s `Absorbed::Ranged(body)` arm, if `self.len` is `Some(len)` and `start < len` and the body is empty, return `Err(truncated(len, start))`.
Confidence: HIGH — Verification: read (both arms traced; no transport-level guard — `HttpResponse.header`/body are delivered verbatim, fetch.rs:468-493, and a 0-length body with `Content-Length: 0` is well-framed HTTP the transport will not refuse).

### F-002 [S2]
Where: crates/streaming/src/ranged.rs:331-338 (default 64 MiB, part1) as consumed at apps/cli/src/demux_host.rs:488-495
Code:
```
    let config = streaming::ranged::RangedConfig::default();
    let resource = match fetch {
        Some(fetch) => streaming::ranged::RangedResource::open(fetch, url, config),
        None => streaming::schemes::SchemeRegistry::new().open_http(
```
What is wrong: the shipped `http(s)` open lane always builds `RangedConfig::default()` — `window_bytes` is the hard-coded `DEFAULT_WINDOW_BYTES` (64 MiB). The user-facing setting `performance.cache-network-mib` (docs/guides/settings.md:67, schemas/generated/settings-docs.md:188, CLI flag `--performance-cache-network-mib`) is parsed into `product.toml`'s `limits.cache_network_mib` (crates/product-config/src/merge.rs:432-433) and then read by nothing: `rg -n "window_bytes\s*[:=]"` shows no production site deriving it from config — only test fixtures and `RangedConfig::default()`.
Why it matters: a user who sets a 4 MiB network cache on a memory-constrained device gets 64 MiB retained (16× the request); a user who raises it to 1024 gets no effect. The ranged.rs module doc (32-36) admits "Live product-config plumbing lands with the http open-path card" — but the http open-path HAS shipped (this very call site, w31.3) and the join was never made. Settings UI/config surface promises a knob that is wired to nothing (checklist: "config declared but never read").
What correct looks like: `open_http_source` reads the effective `limits.cache_network_mib` from the loaded product config and constructs `RangedConfig { window_bytes: mib * 1024 * 1024, ..Default::default() }`, with `validate()`'s existing typed ceiling refusal intact.
Confidence: HIGH — Verification: read + rg (no mapping site exists; product-config parses the key; settings docs present it as live).

### F-003 [S4]
Where: crates/streaming/src/fetch.rs:487-492 as consumed at ranged.rs:590 and 647-649
Code:
```
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
```
What is wrong: first-match-wins for repeated headers. A hostile/buggy server sending two `Content-Range` (or `Content-Length`) headers has only its first one validated; the second is never cross-checked. HTTP response splitting via duplicate headers is not detected anywhere in this chunk.
Why it matters: validation is only as strong as the header lookup; smuggling a second value past `absorb`/`enter_scan` is possible in principle. Blast radius limited — the validated first value is the one used for length accounting.
What correct looks like: `header` (or its ranged.rs call sites) rejects responses carrying more than one `Content-Range`/`Content-Length` as `protocol_violation`.
Confidence: MEDIUM — Verification: read (no duplicate-header guard at any call site in this chunk).

### F-004 [S4]
Where: crates/streaming/src/ranged.rs:871-874
Code:
```
        let mut out = vec![0u8; len];
        let n = self.read_into(&mut out)?;
        out.truncate(n);
        Ok(out)
```
What is wrong: a `read(len)` near EOF allocates and retains `len` bytes even when the read returns a handful of bytes — `truncate` moves the length, not the capacity. A demuxer asking `read(64 MiB)` at the last 1 KiB of a stream holds a 64 MiB allocation for the caller's lifetime of that Vec. The zero-fill is mostly free (fresh pages), the retained capacity is not.
Why it matters: transient 64 MiB spikes per large read; `shrink_to_fit` (or chunked reads) avoids it. Bounded by `MAX_READ_BYTES`, so hygiene not critical.
What correct looks like: `out.shrink_to_fit()` after truncate, or build the result from the filled prefix.
Confidence: LOW — Verification: read (allocation semantics; no measurement run — would need REQUEST-ORCH-RUN to quantify).
Confidence note: severity kept at S4 because the bound is explicit and callers today read small demuxer-sized amounts.

### F-005 [S4]
Where: crates/streaming/src/ranged.rs:953-979
Code:
```
    let (Ok(start), Ok(end)) = (start.trim().parse::<u64>(), end.trim().parse::<u64>()) else {
        return Err(protocol_violation());
    };
```
What is wrong: `parse_content_range` never checks `end >= start`, `total > end`, or `body.len()` against `end - start + 1`. A `Content-Range: bytes 5000-100/9000` (end < start) parses successfully; the `end` is bound to `_rend` and discarded by `absorb` (590), so the lie is inert — but the parser's doc promises strictness ("A `206` without a parseable `Content-Range`" → typed refusal) while actually validating only syntax, not internal consistency.
Why it matters: inconsistent headers from a broken CDN are accepted where the contract says refused; downstream length accounting uses `total` only, which limits the damage.
What correct looks like: reject `end < start` and `Some(total) if total <= end` as `protocol_violation`.
Confidence: HIGH — Verification: read (all uses of the parse result traced: `absorb` discards `_rend`).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `RangedResource::enter_scan` | 640-672 | REAL | — | 200-only, Content-Length cross-check (650-655), shrink→`truncated` (657-662), releases window cache (664-665). Correct on all traced paths. |
| `RangedResource::scan_refetch` | 680-691 | REAL | — | refetch+skip accounting (`skipped_bytes`, `refetches`); termination sound (see §2). |
| `RangedResource::scan_fill` | 698-719 | REAL | — | `_ => return Ok(0)` arm is defensive-dead per length cross-checks; no panic paths (`off < data.len()` guaranteed by `covers`). |
| `RangedResource::scan_trim` | 724-733 | REAL | — | drain bounded by `min(pos-base, data.len())`; correct. |
| `RangedResource::fetch_window` | 738-777 | REAL | S3 (F-001) | 416→truncated (known len) / clean EOF (unknown); saturating end; empty-body hole per F-001. |
| `RangedResource::insert_window` | 786-798 | REAL | — | LRU eviction; fresh-window protection proven (§2). |
| `RangedResource::take_hit` | 802-809 | REAL | — | O(n) window scan, n ≤ window/chunk = 256 default — fine. |
| `RangedResource::copy_hit` | 812-823 | REAL | — | slice math safe (`off < wlen` from `covers`). |
| `RangedResource::cached_bytes` | 828-830 | REAL | — | live memory-bound accessor used by tests. |
| `RangedResource::stats` | 833-835 | REAL | — | `cache_hits` counts loop iterations, not `read()` calls (doc says "reads") — off-by-semantics, harmless. |
| `RangedResource::length` | 839-841 | REAL | — | trivial accessor. |
| `SourceStream::uri` | 845-847 | REAL | — | post-redirect identity, part1 sets it. |
| `SourceStream::size` | 849-851 | REAL | — | `self.len`; unknown-length = None. |
| `SourceStream::supports_seek` | 853-855 | REAL | — | always `true`; honest — scan mode serves seeks by refetch. |
| `SourceStream::read` | 857-875 | REAL | S4 (F-004) | 0-len and >64 MiB typed refusals correct. |
| `SourceStream::read_into` | 877-930 | REAL | — | sticky known-length EOF (884-888); mid-stream 200 resync re-dispatch (910-914); no infinite-loop path found (all `continue`s advance pos or change mode). |
| `SourceStream::seek` | 932-943 | REAL | — | past-end typed, exactly-at-len legal, cursor never moves on failure. |
| `SourceStream::position` | 945-947 | REAL | — | trivial. |
| `parse_content_range` | 953-979 | REAL | S4 (F-005) | syntax-only strictness. |
| `tests::pattern` | 990-992 | REAL | — | period-251 fixture, prime to 1024/4096. |
| `tests::tight` / `tests::scan` | 999-1016 | REAL | — | config fixtures. |
| `tests::served` / `served_no_range` / `open` | 1018-1041 | REAL | — | mockhttp harness; `unreachable!` on spawn failure acceptable in tests. |
| `sequential_reads_equal_the_served_bytes` | 1045-1078 | REAL | — | discriminating (§5). |
| `seek_back_rereads_from_cache_with_no_requests` | 1083-1106 | REAL | — | discriminating; pins request-count == mock log. |
| `seek_forward_beyond_read_ahead_fetches_at_the_right_offset` | 1111-1133 | REAL | — | pins exact `bytes=5000-6023` header string. |
| `forward_scan_holds_the_window_budget` | 1138-1165 | REAL | — | live peak ≤ budget + eviction > 0 + fetched < 1.2× payload. |
| `eof_short_read_is_data_then_clean_zero` | 1169-1187 | REAL | — | short-read-is-data + seek-to-exact-len. |
| `seek_past_end_is_the_input_contract_error` | 1192-1216 | REAL | — | pins `input.seek.out-of-range` / `before-start` codes + zero wire requests. |
| `mid_read_416_is_a_typed_truncation` (setup 1221-1256; asserts in part3) | 1221-1256 | REAL | — | Shrinker fixture; green in baseline; assert body audited by A08. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| sequential_reads_equal_the_served_bytes | YES | byte-exact `assert_eq!(got, data)` + per-read position asserts; any read/pos bug trips one of them. |
| seek_back_rereads_from_cache_with_no_requests | YES | asserts `server.requests().len()` unchanged across a rewind — a cache miss would add a request; also `stats().cache_hits >= 1`. |
| seek_forward_beyond_read_ahead_fetches_at_the_right_offset | YES | exact header string `"bytes=5000-6023"` — off-by-one in chunk math changes the string. |
| forward_scan_holds_the_window_budget | YES | `peak <= 4096` fails if eviction breaks; `bytes_fetched < 60_000` fails on refetch loops; `requests == mock log` fails on accounting drift. |
| eof_short_read_is_data_then_clean_zero | YES | `tail == &data[2000..]` discriminates short-read vs error; clean-zero + seek-to-len pin EOF semantics. |
| seek_past_end_is_the_input_contract_error | YES | asserts the stable code string, unmoved cursor, and zero new requests. |
| mid_read_416_is_a_typed_truncation | YES (asserts in part3) | Shrinker returns 416 past first chunk; baseline green; part3 (A08) owns the assert body. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "A read that hits the wire-end before the open-time length fails typed — never a silent EOF" | ranged.rs module doc 19-23 | REAL with one hole | 416 + shrinking-total paths typed (768-774, 596-602); empty-body 206 silently EOFs (F-001). |
| "window cache holds at most `window_bytes` with least-recently-used eviction" | module doc 30-31 | REAL | insert_window 786-798 + baseline `forward_scan_holds_the_window_budget` ok. |
| "Live product-config plumbing lands with the http open-path card; the constants here are the documented defaults" | module doc 34-36 | SHALLOW | the http open-path shipped (demux_host.rs:488) without the join — F-002. |
| "The default window mirrors product.toml's limits.cache_network_mib = 64" | module doc 32-33 | REAL-UNPROVEN | constant matches product.toml value, but no code path reads the config (rg, F-002). |

## 7. Dead / duplicated / unreachable code
None in this chunk. `scan_fill`'s `_ => return Ok(0)` arm is defensive-dead by the length cross-checks upstream (documented as such in-code at 703-706) — kept deliberately. `RangedStats` is test/telemetry-facing by its own doc (378-380); nothing in production reads it (verified by rg earlier waves' pattern; no production caller in this chunk).

## 8. Dependencies & flags
Depends only on `media_types` (error taxonomy), `plugin_api` (SourceStream/SeekOrigin), `crate::error::NetworkError`, `crate::fetch` (port types) — no feature gates in this chunk; all code is unconditional. No unsafe. Crate built in baseline release build (build-release-workspace.log.txt, exit 0).

## 9. Open questions for the next agent
- A08 (part3): confirm the `mid_read_416` asserts pin `streaming.ranged.truncated` code, and check whether any part3 test covers the empty-body-206 shape of F-001 (I found none in 981-1256).
- Whoever audits `apps/cli/src/demux_host.rs` or product-config wiring: F-002's missing join (`cache-network-mib` → `RangedConfig`) belongs to the open-path card that already shipped.
- The production CLI uses `SchemeRegistry::new()` — NOT `with_retrying_fetch`; the whole reconnect layer (reconnect.rs, card 6.D2) is bypassed by the shipped binary. Assignee of reconnect.rs parts should treat this as the reachability verdict.
