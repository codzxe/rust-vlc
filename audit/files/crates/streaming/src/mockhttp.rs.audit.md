# AUDIT: crates/streaming/src/mockhttp.rs
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-23 |
| Lines / functions | 1559 / 37 (incl. impls) + 18 tests |
| File verdict | REAL (test infrastructure — real TCP sockets, real HTTP/1.1 wire; exercised live) |
| Reachable from shipped entry point? | NO as code path (all callers are #[cfg(test)] — verified below); YES as compiled module (`pub mod mockhttp;` is not cfg-gated) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 4 |
| Commands run | `~/.cargo/bin/cargo test -p streaming --lib mockhttp` → 18 passed / 0 failed (exit 0, 0.13s); rg over src/tests/apps for callers |

## 1. One-paragraph verdict
This is NOT a function-call fake: `MockServer::spawn` binds a real ephemeral loopback TCP socket and speaks real HTTP/1.1 over it (`TcpListener::bind(("127.0.0.1", 0))?`, L454), and the request log records what actually arrived on the wire. The module doc's claim "test-only; never used by production code paths" (L1-2) is TRUE at the call-graph level: every non-test reference in the crate is inside a `#[cfg(test)]` module (ranged.rs:981/985, hls/media.rs:738/742, remote_playlist.rs:240/243, client.rs:464/468, schemes.rs:532/536, fetch.rs:538/554), plus apps/cli/src/demux_host.rs:881 under its `#[cfg(test)]` (line 621). All 18 in-module tests pass live. The honest criticism is not fakeness of the server but the CLIENT side: `MockClient` is a second, test-owned HTTP client implementing redirect policy, status classification, and body reading independently of the production `UreqClient` — and it diverges from it (F-002). Every e2e suite in crates/streaming/tests/ drives `MockClient`, not `UreqClient` (rg count: 11 files with Mock*, 0 with UreqClient); `UreqClient` is exercised only by client.rs's own cfg(test) unit tests against this server, and its two live-network tests are `#[ignore]`d forever (baseline: `live_example_com_returns_200 ... ignored, live network required`). TLS is never round-tripped in-tree (https_activation.rs admits this at its head).

## 2. What actually works (verified)
— `~/.cargo/bin/cargo test -p streaming --lib mockhttp` → `test result: ok. 18 passed; 0 failed; 0 ignored` (real sockets, real timeouts, 0.13s). Baseline workspace run corroborates: `test mockhttp::tests::scripted_status_answers_then_recovers ... ok` etc. (audit/runs/baseline/cargo-test-workspace.txt:5526-5549), and client.rs's real-ureq-over-mockhttp tests pass in the same run (212 passed / 2 ignored, line 5617).

## 3. Findings
F-001 [S3]
Where: crates/streaming/src/mockhttp.rs:941-947 vs crates/streaming/src/client.rs:444-445
Code (mockhttp.rs):
```
                if followed >= request.redirects.max_redirects {
                    return Err(NetworkError::TooManyRedirects {
                        limit: request.redirects.max_redirects,
                        followed,
                    });
                }
```
What is wrong: `MockClient::fetch_inner` enforces `max_redirects`, revisit detection, and scheme downgrade — but NEVER reads `request.redirects.cross_host`. The real client does: client.rs:444-445 `let cross_host = !next.host.eq_ignore_ascii_case(&url.host); if cross_host && !request.redirects.cross_host {` refuses. Why it matters: all 11 session-layer e2e suites (HLS vod/live/master/conformance, ranged_wav, auth, netfault, net_interrupted, https_activation, netfault_proxy) run through MockClient, so a cross-host redirect-policy regression in the real transport is invisible to every e2e test; the only coverage is client.rs's own unit tests. A test double that silently accepts a policy the port documents (fetch.rs:130 "`cross_host` records whether...") diverges from the thing under test. What correct looks like: MockClient implements the same cross-host gate (it has the `absolute_redirect` machinery to test it) or a doc line stating the mock omits it. Confidence: HIGH — Verification: read (rg cross_host: mockhttp.rs hits only L1315, a test setting the flag).

F-002 [S3]
Where: crates/streaming/src/lib.rs:36 and crates/streaming/src/mockhttp.rs:1-2
Code:
```
//! Mock HTTP/1.1 testkit — **test-only; never used by production code
//! paths** (card 6.A1a).
```
plus lib.rs:36 `pub mod mockhttp;` (ungated).
What is wrong: the "never used by production code paths" claim is true, but the module is a `pub`, non-`cfg(test)` member of the shipped crate, so the test double (server + client + auth verifier) is part of streaming's public API and is compiled into every release build of the crate that apps/cli links. Unreferenced symbols are stripped at final link, so the binary cost is ~nil, but the API surface ships a test server as if it were product. Why it matters: any future crate can legitimately `use streaming::mockhttp::MockServer` in production code because the module is public; the "test-only" contract is enforced by nothing but a doc comment. What correct looks like: `#[cfg(any(test, feature = "testkit"))]` gating or a separate `streaming-testkit` crate. Confidence: HIGH — Verification: read (lib.rs:36 has no attribute).

F-003 [S4]
Where: crates/streaming/src/mockhttp.rs:304-315
Code:
```
            let expected = digest_response(&crate::auth::DigestInputs {
                username: &self.username,
                password: &self.password,
                realm: &self.realm,
                method: &request.method,
```
What is wrong: the mock's Digest verifier recomputes the response with the SAME `crate::auth::digest_response` the production client uses to build it — a shared-implementation verifier, not an independent one. A bug in `digest_response` agrees with itself and the e2e auth tests still pass. Mitigated: auth.rs pins an RFC 2617 known vector (`test auth::tests::digest_rfc2617_known_vector ... ok`, baseline line 5405), so the shared function itself is externally anchored. The doc's "a real verifier, not a string match" (L188) remains accurate. S4 because the mitigation exists and is verified. Confidence: HIGH — Verification: read + baseline.

F-004 [S4]
Where: crates/streaming/src/mockhttp.rs:519-521
Code:
```
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(2));
            }
```
What is wrong: the accept loop is a 2 ms non-blocking poll — ~500 wakeups/s per spawned server for the server's whole lifetime; e2e files spawn several servers per test. Test-only and CPU-cheap in absolute terms, but it is a busy-wait where a blocking accept + stop-pipe would be idle. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/streaming/src/mockhttp.rs:571-578
Code:
```
    loop {
        if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&raw[..pos]).into_owned();
            let rest = raw[pos + 4..].to_vec();
            return HeadOutcome::Head(head, rest);
        }
        if raw.len() > HEAD_LIMIT {
            return HeadOutcome::OverLimit;
```
What is wrong: `read_head` rescans the accumulated buffer from byte 0 after every 4 KiB chunk — O(n²) over the head, bounded by HEAD_LIMIT (64 KiB) so worst case ~1 MiB of comparisons per request; also `raw.len() > HEAD_LIMIT` is checked before the read, letting the buffer overshoot the limit by one chunk (≤4096 bytes) — harmless. Test-only. Confidence: HIGH — Verification: read.

F-006 [S4]
Where: crates/streaming/src/mockhttp.rs:1118-1124
Code:
```
    if expected > limit {
        return Err(NetworkError::ResponseTooLarge {
            limit,
            actual: expected,
        });
    }
    let mut body = vec![0u8; expected as usize];
```
What is wrong: the pre-allocation is guarded only by the caller's `response_limit`; default is 8 MiB (fetch.rs:28 `pub const DEFAULT_RESPONSE_LIMIT: u64 = 8 * 1024 * 1024;`) so a hostile Content-Length aborts cleanly by default, but a test raising the limit to u64-range values can request a multi-GiB zeroed allocation on the mock's word. Also `expected as usize` truncates on 32-bit targets if a test sets a huge limit. Test-only client, reachable only from test code. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory, >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| LoggedRequest::header | 62-67 | REAL | — | case-insensitive first-match, correct |
| ScriptedRoute::{new,take,fetches} | 97-125 | REAL | — | last-repeats cursor; poison-safe |
| ScriptedStatus::{new,take,served} | 150-177 | REAL | — | exhaustion→normal, poison-safe |
| MockAuth::basic/digest | 219-244 | REAL | — | fixed nonce/opaque; deterministic |
| MockAuth::challenge_value | 247-262 | REAL | — | correct Basic/Digest challenge text |
| MockAuth::authorize | 267-318 | REAL | S4 | shared-impl digest verify (F-003) |
| MockAuth::Debug | 324-334 | REAL | — | password redacted, verified |
| MockConfig::default/body | 409-436 | REAL | — | defaults match docs |
| MockServer::spawn | 453-473 | REAL | — | real ephemeral loopback bind |
| MockServer::url/client/requests | 476-491 | REAL | — | — |
| MockServer::drop | 494-501 | REAL | — | stop flag + join; handlers detached ≤2 s (documented L27-29) |
| accept_loop | 505-529 | REAL | S4 | 2 ms poll busy-wait (F-004) |
| handle | 532-550 | REAL | — | logs before serving; write errors swallowed (testkit-acceptable) |
| read_head | 568-589 | REAL | S4 | rescan O(n²), overshoot ≤1 chunk (F-005) |
| parse_request_head | 593-608 | REAL | — | best-effort; colonless lines dropped |
| write_response | 611-711 | REAL | — | precedence garbage>absolute>self_loop>redirects>auth>scripted_status>scripted_routes>routes>404>body matches docs |
| redirect_head | 714-718 | REAL | — | — |
| serve_body | 721-762 | REAL | — | 206/416/200 logic correct; suffix ranges unsupported (documented L777-779) |
| reason_phrase | 766-775 | REAL | — | fallback "Status" |
| parse_range_spec | 780-792 | REAL | — | rejects end<start; no suffix/open-start forms |
| MockClient::new/exchange | 802-825 | REAL | — | fresh connection per exchange |
| MockClient::connect | 827-853 | REAL | — | Happy Eyeballs-ish iteration, typed errors |
| MockClient::send_request | 855-889 | REAL | — | header-injection refusal is real (test at 1461-1478) |
| HttpFetch::{fetch,fetch_with_challenge} | 893-902 | REAL | — | port impl |
| MockClient::fetch_inner | 911-972 | REAL | S3 | ignores cross_host (F-001); loop/max/downgrade implemented |
| strip_authorization | 977-983 | REAL | — | — |
| classify | 987-1000 | REAL | — | documented simplified policy |
| is_redirect | 1002-1004 | REAL | — | — |
| read_response_head | 1018-1032 | REAL | — | Timeout/EOF→typed |
| parse_response_head_text | 1041-1069 | REAL | — | shared with fuzz entry |
| fuzz_response_head | 1088-1099 | REAL | — | HEAD_LIMIT-bounded, lossy decode |
| content_length | 1101-1106 | REAL | — | — |
| read_bounded_body | 1112-1143 | REAL | S4 | pre-alloc trust (F-006) |
| read_eof_body | 1147-1174 | REAL | — | incremental limit checks correct |
| map_body_read_error | 1178-1186 | REAL | — | — |

## 5. Test honesty (18 tests)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| roundtrip_200_body_equality | YES | asserts status+reason+body+logged path over real socket |
| partial_content_206_with_content_range_echo | YES | asserts exact Content-Range + slice |
| open_ended_range_reads_to_end | YES | bytes 990-999/1000 exact |
| range_ignored_config_serves_200_full_body | YES | 200 + full body under range request |
| status_416_is_range_not_satisfiable | YES | typed error + error code string |
| redirect_chain_of_three_lands_on_final_body | YES | asserts hop path sequence [start,/r1,/r2,/r3] |
| too_many_redirects_is_bounded_by_policy | YES | limit:2 followed:2 exact |
| self_loop_is_detected_as_redirect_loop | YES | RedirectLoop{hops:1}, one request |
| read_timeout_fires_on_delayed_response | YES | 120 ms read vs 400 ms delay, <3 s wall |
| hard_reset_mid_body_is_body_truncated | YES | expected:1000 received:400 |
| garbage_first_line_is_a_framing_error | YES | exact detail string |
| request_log_captures_the_range_header | YES | bytes=250-749 + host header |
| refused_port_is_connect_refused | WEAK-YES | real closed-port race: another process can rebind the ephemeral port in the drop→connect gap (rare flake, not fraud) |
| response_over_budget_is_response_too_large | YES | limit:64 actual:1000 |
| header_injection_is_rejected_typed | YES | dirty header via direct field assembly |
| range_spec_parser_accepts_and_rejects | YES | pure parser, 5 vectors |
| scripted_route_advances_and_repeats_last | YES | body sequence + fetches()==3 + 404 typed |
| scripted_status_answers_then_recovers | YES | 503,500 then 200 BODY |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "test-only; never used by production code paths" | mockhttp.rs:1-2 | TRUE (call-graph) | rg: all src/ uses under #[cfg(test)]; demux_host.rs:881 under cfg(test) at 621; but module is pub+ungated (F-002) |
| "A minimal HTTP/1.1 server on an ephemeral localhost port" | mockhttp.rs:4-5 | REAL | L454 bind(("127.0.0.1",0)); 18 tests pass over sockets |
| "tests exercise the real port — the same surface the ureq client ... will implement" | mockhttp.rs:12-13 | TRUE but MISLEADING | e2e suites exercise the port via MockClient, not the ureq implementation; only client.rs unit tests exercise UreqClient; 2 live tests #[ignore]d (baseline 5615-5616) |
| "Drop closes the listener ... joining cleanly" | mockhttp.rs:25-26 | REAL | drop at 494-501; handler threads detached ≤ deadline (documented) |
| "a real verifier, not a string match" (digest) | mockhttp.rs:186-188 | REAL | recomputes via digest_response — but shared impl with client (F-003) |

## 7. Dead / duplicated / unreachable code
No dead code found. Duplication: MockClient reimplements redirect/classification/body-read logic that UreqClient also implements (client.rs) — a deliberate but risky test seam; the divergence already materialized (cross_host, F-001). The module is unreachable from shipped code paths by design (testkit).

## 8. Dependencies & flags
std-only (net, io, sync, thread, time) + crate::auth/fetch/error — no external crates, matching Cargo.toml's C5 claim. No feature flags. mockhttp itself is not cfg-gated (F-002).

## 9. Open questions for the next agent
- Does any doc/matrix row cite an e2e streaming test as evidence of "real network playback"? If so, the MockClient-not-UreqClient gap (F-001 context) matters for that row's verdict.
- The two #[ignore]d live tests (`live_example_com_returns_200`, `live_httpbin_redirect_chain_lands_on_200`) are the only real-internet evidence in the repo — has any CI lane ever run them? UNVERIFIED here.
