# AUDIT: crates/streaming/src/mockhttp.rs#part2 (lines 780–1559)
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-21 |
| Lines / functions | 780 audited (chunk part2 of 1559; part1 = 1-779 is another agent's — server side: LoggedRequest, ScriptedRoute/Status, MockAuth, MockConfig, MockServer, accept/handle loops, read_head, write_response, redirect_head, serve_body, reason_phrase; boundary is clean: part2 opens at `parse_range_spec` :780) / 18 production items (parse_range_spec; MockClient: new, exchange, connect, send_request, fetch, fetch_with_challenge, fetch_inner; strip_authorization, classify, is_redirect, ResponseHead, read_response_head, parse_response_head_text, fuzz_response_head, content_length, read_bounded_body, read_eof_body, map_body_read_error) + 3 test helpers + 18 tests |
| File verdict | REAL (testkit by design; executed in baseline; no production caller) |
| Reachable from shipped entry point? | NO production path calls it (module doc's own claim, verified by census: every `mockhttp` reference in client.rs/schemes.rs/fetch.rs/demux_host.rs sits inside `#[cfg(test)]` mods — fetch.rs:538, schemes.rs:532, client.rs:464, demux_host.rs tests at 869-881; apps/cli/tests/unsupported_path.rs is a test). The module is `pub mod` in the shipped rlib (lib.rs:36) — compiled, link-time-dropped, call-unreachable. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `grep -n` structure map (18-item pin); `sed -n` verbatim verification of every quote; `rg "MockClient\|mockhttp::"` across crates/+apps/ (caller census — all test mods); `ls fuzz/fuzz_targets/` + `rg fuzz_http_header` (filename verification); `grep -n "cross_host"` client.rs vs mockhttp.rs (policy-divergence verification); baseline: `orchrun-test-workspace-nofailfast.log.txt` :5068-5086 (18/18 mockhttp tests ok), :5250-5255 (netfuzz_sweep: http_headers_survives ok; 60s variants ignored by design); `od` byte-verification of `#[must_use]` lines (display pipeline artifact ruled out) |

## Chunk boundary note
Part2 = 780–1559: the entire mock **client** half (`MockClient` and its HTTP/1.1 response grammar, redirect loop, and bounded-body readers — the `HttpFetch` port implementation tests actually exercise), the fuzz entry `fuzz_response_head`, and `mod tests` (1188-1559, 18 socket-based tests). Part1 (1-779) owns the mock **server** half. The 780 boundary lands exactly on `parse_range_spec`'s doc line — no function is split.

## 1. One-paragraph verdict
This chunk is the mock HTTP/1.1 client that every streaming test drives through the real `HttpFetch` port, and it is built properly: header-injection refusal before the socket, a bounded redirect loop with revisit detection, https→http downgrade refusal and per-hop Authorization stripping in parity with the real ureq client, and body readers that refuse over-budget responses before allocating (the exact mechanism the ranged resource's no-range tests depend on — verified end-to-end across my three earlier files). All 18 tests ran green in the baseline and they assert against real socket behavior (hop-path logs, typed error shapes, wall-clock timeout with 25× margin). I traced every client function and found no correctness defect: no panic path on hostile input (lossy UTF-8, colon-less header lines dropped, HEAD_LIMIT bounds, empty-head handled), no budget bypass (bounded and EOF readers agree on the exact-at-limit boundary). Two documentation defects only: the fuzz-entry doc cites a fuzz target filename that does not exist (`fuzz_http_header.rs` vs the actual `fuzz_http_headers.rs`, which itself carries a "naming note" about this exact confusion), and the module doc claims redirects are followed "under the request's RedirectPolicy" while `fetch_inner` ignores the policy's `cross_host` field — a divergence from the real client (client.rs:440-446) that no current test trips but a future one could silently.

## 2. What actually works (verified)
— 18/18 in-file tests executed ok: baseline `orchrun-test-workspace-nofailfast.log.txt:5068-5086`; `netfuzz_sweep_http_headers_survives` ok at :5255 (the two 60-second fuzz sweeps are `#[ignore]`d long runs by design — baseline's 24-ignored count includes them).
— Read-traced the budget mechanism the ranged suite relies on: `read_bounded_body` (1112-1143) checks `expected > limit` BEFORE `vec![0u8; expected]` (1118-1124) → `ResponseTooLarge{limit, actual}` with zero allocation, and the request was already logged server-side (part1's `handle`), which is exactly why ranged.rs's `scan_fallback_reads_the_full_body` can assert `bytes_fetched == 10_000` and a 2-entry request log with `bytes=0-1023` on entry 0. Cross-file mechanism confirmed consistent.
— Read-traced the redirect loop (911-972): visited-set seeded with the start URL (loop-back-to-start detected), budget checked before the hop, scheme-downgrade refused (952-954), Authorization stripped from every followed hop (919-927 + 977-983) — matching the real client's `RedirectAuthHeaders::Never` parity claim; `redirect_chain_of_three_lands_on_final_body` pins the hop path from the server's request log (:1300-1302).
— Header-injection defense (861-868) refuses control bytes in header names/values before any socket write; the test reaches it by direct field assembly (1468-1472), bypassing the builder — a genuine defense-in-depth pin, not a tautology of `with_header`'s own check.
— `parse_response_head_text` handles the hostile shapes the fuzz harness throws: empty head, bad version, non-numeric code → typed `ProtocolViolation`; hostile bytes → lossy U+FFFD decode, never a panic; input capped at HEAD_LIMIT (1089-1093).

## 3. Findings

### F-001 [S4]
Where: crates/streaming/src/mockhttp.rs:1079-1081
Code:
```
/// lines). `pub` for the crate-external fuzz harness
/// (`crates/streaming/tests/netfuzz/fuzz.rs`, `#[path]`-included by
/// `fuzz/fuzz_targets/fuzz_http_header.rs`); test-support only — no
```
What is wrong: The cited file does not exist. The target is `fuzz/fuzz_targets/fuzz_http_headers.rs` (plural) — its own header even carries a "Naming note: the target is `fuzz_http_headers`, plural" warning about this exact confusion (:18), which this doc comment ignores.
Why it matters: The doc is the wiring map for the fuzz lane; a contributor following it lands on a missing file. The registry check (`scripts/fuzz_registry_check.sh`, baseline #18 PASS — 35 targets) uses the real name, so the doc is the only stale artifact.
What correct looks like: `fuzz/fuzz_targets/fuzz_http_headers.rs` (and ideally drop the naming-note need by aligning the two).
Confidence: HIGH — Verification: `ls fuzz/fuzz_targets/` → `fuzz_http_headers.rs`; `rg fuzz_http_header` → only this doc line uses the singular form.

### F-002 [S4]
Where: crates/streaming/src/mockhttp.rs:15-19 (module doc) vs 911-972 (fetch_inner)
Code:
```
//! [`NetworkError::HttpStatus`], and redirects are followed under the
//! request's [`RedirectPolicy`](crate::fetch::RedirectPolicy) with
//! revisit detection. The real client's policy decisions land in
```
What is wrong: `fetch_inner` honors `RedirectPolicy::mode` (Manual/Follow, :933) and `max_redirects` (:941) but ignores `cross_host` entirely — a cross-host redirect is followed silently regardless of policy. The real client enforces it (client.rs:440-446, `NetworkError::CrossHostRedirect`, tested at client.rs:624). The doc's "followed under the request's RedirectPolicy" is true for 2 of the policy's 3 fields; the next sentence's "simplified transport contract" disclaimer does not name the gap.
Why it matters: A future test that wants to pin cross-host refusal semantics through `MockClient` will silently pass-through-follow and assert nothing — a false-negative hazard on a security-adjacent policy (redirect-based credential scopes). No current test is misled (the only `cross_host` reference in this file is a test setting it `true`, :1315 — the permissive direction).
What correct looks like: Either implement the flag (`if cross-host && !request.redirects.cross_host { Err(CrossHostRedirect) }` — the host comparison shape exists in client.rs:444) or doc the exclusion explicitly ("mode and budget only; cross-host policy is the real client's").
Confidence: HIGH — Verification: read of both loops + `grep cross_host` over both files (client.rs enforces at :440-446; mockhttp.rs references it once, in a test).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parse_range_spec | 780-792 | REAL | — | `bytes=a-b`/`bytes=a-`; rejects end<start, wrong unit, non-numeric. Suffix ranges (`bytes=-500`, legal RFC 9110) return None → silent 200 fallback; doc calls them "malformed" — unreachable in practice (the port's `ByteRange` has no Suffix variant), noted not counted. Tested :1481-1487. |
| MockClient::new | 802-804 | REAL | — | Trivial. |
| MockClient::exchange | 808-825 | REAL | — | One fresh connection per request; head+body under the request's response_limit. No panic path. |
| MockClient::connect | 827-853 | REAL | — | Happy-eyeballs-lite over resolved addrs with connect_timeout; error-kind mapping (Timeout/Refused/Io). `set_read/write_timeout` results ignored (`let _ =`) — best-effort on TCP, acceptable. |
| MockClient::send_request | 855-889 | REAL | — | Injection check (control bytes) before write; Host/headers/Range/Connection: close. Write-timeout mapped. |
| HttpFetch::fetch / fetch_with_challenge | 893-903 | REAL | — | Thin dispatch to fetch_inner; challenge mode surfaces 401 (card 6.E1 parity). |
| MockClient::fetch_inner | 911-972 | REAL | S4 (F-002) | Redirect loop: Manual mode, Location required, budget bound, scheme-downgrade refusal, visited-set loop detection, per-hop Authorization strip. `cross_host` ignored (F-002). O(n²) visited contains at n≤11 — fine. |
| strip_authorization | 977-983 | REAL | — | Case-insensitive Authorization removal on followed hops. |
| classify | 987-1000 | REAL | — | 2xx Ok; 416+range → RangeNotSatisfiable; 4xx/5xx → HttpStatus; 1xx/3xx-non-redirect surfaced as-is (documented). |
| is_redirect | 1002-1004 | REAL | — | 301/302/303/307/308. 303 method-change is moot (GET-only port). |
| ResponseHead / read_response_head | 1007-1032 | REAL | — | Reuses part1's bounded read_head; Timeout/Eof/OverLimit typed. |
| parse_response_head_text | 1041-1069 | REAL | — | `HTTP/1.` prefix, u16 code, reason; colon-less lines silently dropped (lenient by design — the fuzz lane wants no-panic, not strictness). Empty head → typed violation. |
| fuzz_response_head | 1088-1099 | REAL | S4 (F-001) | HEAD_LIMIT pre-check, lossy decode, same grammar as the wire path. Doc filename stale (F-001). Executed via netfuzz_sweep in baseline (:5255). |
| content_length | 1101-1106 | REAL | — | First Content-Length wins; duplicate/conflicting headers unresolved — testkit-acceptable (real client is client.rs's audit). |
| read_bounded_body | 1112-1143 | REAL | — | Budget BEFORE allocation (the ranged suite's load-bearing behavior, §2); premature EOF/reset → BodyTruncated with exact counts. exact-at-limit admitted (consistent with read_eof_body). |
| read_eof_body | 1147-1174 | REAL | — | Chunked 8 KiB reads, budget checked per chunk with would-be `actual`, EOF clean. Timeout → ReadTimeout. |
| map_body_read_error | 1178-1186 | REAL | — | Deadline/reset/io triage; matches the typed vocabulary. |
| tests: 3 helpers + 18 tests | 1195-1559 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| roundtrip_200_body_equality (1210) | YES | Byte-equality + status/reason/header + request-log shape; any corruption or framing slip fails it. |
| partial_content_206_with_content_range_echo (1229) / open_ended_range_reads_to_end (1244) | YES | Body slices compared against the source pattern; Content-Range echo pinned. |
| range_ignored_config_serves_200_full_body (1255) | YES | The no-range shape pinned (ranged.rs 6.A2b's vehicle). |
| status_416_is_range_not_satisfiable (1272) | YES | Typed variant + dotted code asserted. |
| redirect_chain_of_three_lands_on_final_body (1287) | YES | Hop path asserted from the server log (`[/start/here, /r1, /r2, /r3]`) — fails if any hop is skipped or re-ordered. |
| too_many_redirects_is_bounded_by_policy (1306) | YES | Exact `TooManyRedirects{limit:2, followed:2}` destructured. |
| self_loop_is_detected_as_redirect_loop (1332) | YES | `RedirectLoop{hops:1}` + revisit-before-second-request. |
| read_timeout_fires_on_delayed_response (1348) | YES | Real 400 ms server delay vs 120 ms deadline, 25× margin on the 3 s bound; typed ReadTimeout. Timing-based (sandbox-load flake possible, margin generous). |
| hard_reset_mid_body_is_body_truncated (1375) | YES | Exact expected/received counts destructured. |
| garbage_first_line_is_a_framing_error (1396) | YES | Detail string + code pinned. |
| request_log_captures_the_range_header (1416) | YES | Range wire form `bytes=250-749` pinned in the log (inclusive end). |
| refused_port_is_connect_refused (1430) | YES | Bind-drop-connect; tiny port-reuse race, standard technique. |
| response_over_budget_is_response_too_large (1444) | YES | Exact limit/actual destructured; the pre-allocation refusal pinned. |
| header_injection_is_rejected_typed (1461) | YES | Reaches the transport's own check via direct field assembly — not a tautology of the builder. |
| range_spec_parser_accepts_and_rejects (1481) | YES | Pure grammar table incl. inverted range. |
| scripted_route_advances_and_repeats_last (1493) | YES | Advance/repeat/handle-count/404-typed all asserted. |
| scripted_status_answers_then_recovers (1522) | YES | 503→500→200 sequence + handle counts + other-path isolation. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "test-only; never used by production code paths" | mockhttp.rs:1-2 | REAL | Census: all references in production files are inside `#[cfg(test)]` (fetch.rs:538→554, schemes.rs:532→536, client.rs:464→468, demux_host.rs tests 869-881). |
| "redirects are followed under the request's RedirectPolicy with revisit detection" | mockhttp.rs:15-19 | PARTIALLY FALSE | F-002: `cross_host` ignored; mode + max_redirects honored; revisit detection real. |
| "`#[path]`-included by `fuzz/fuzz_targets/fuzz_http_header.rs`" | mockhttp.rs:1080 | FALSE (filename) | F-001: actual file `fuzz_http_headers.rs` (which itself warns about the name). |
| "Bounds: input is capped at HEAD_LIMIT… never an unbounded parse" | mockhttp.rs:1084-1087 | REAL | 1089-1093 + netfuzz_sweep_http_headers_survives ok (baseline :5255). |

## 7. Dead / duplicated / unreachable code
- No dead code in the chunk; every function has a caller (tests or the port seam) and every test ran.
- Deliberate near-duplication: `fetch_inner`'s redirect loop mirrors client.rs's (visited set, budget, downgrade refusal, auth-strip) with the mock's simplifications — the parity claim is documented at 906-910 and holds for everything except `cross_host` (F-002).
- The module ships `pub` in the production rlib by design (integration tests in 4 crates consume it) — compiled but call-unreachable in shipped binaries.

## 8. Dependencies & flags
std-only (net, io, sync, thread, time) per the crate's C5 discipline; reuses `crate::auth` helpers (base64/digest — part1's server side), `crate::error::NetworkError`, `crate::fetch` types. No feature gates, no unsafe, no external crates. Test idiom: `testkit::Must` + `unreachable!` panic-on-unexpected-Ok.

## 9. Open questions for the next agent
- W10-A?? (mockhttp.rs part1): the server half's `serve_body`/`handle` — confirm the log-before-serve ordering my ranged.rs cross-file verification assumed (request logged even when the client refuses the body), and adjudicate `MockAuth`'s digest path (shared helpers with auth.rs).
- client.rs owner: the real client's `cross_host` enforcement (:440-446) is the parity reference for F-002 — if the real client ever relaxes it, the mock's gap becomes moot; if it tightens (per-path scopes), the mock drifts further.
- Fuzz-lane owner: the two 60 s `netfuzz_sweep` tests are `#[ignore]`d — only the short sweep runs in CI; confirm the cargo-fuzz nightly lane actually runs the plural-named target (baseline #18 says the registry diff is empty, but no run evidence exists in this baseline).
