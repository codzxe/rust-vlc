# AUDIT: crates/streaming/src/mockhttp.rs#part1
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-21 |
| Lines / functions | 529 (1–529 of 1560) / 24 items in table |
| Chunk boundary | part1 = 1–529 (through the end of `accept_loop`); equal 3-way split is 520 — extended 9 lines to close the function. A08's part2 = 531–1039 (`handle` through `MockClient`); A10's part3 = 1040–1560 (tests). |
| File verdict | REAL (testkit; ships in binary but production-unused) |
| Reachable from shipped entry point? | NO production call path (claim verified): every `mockhttp` consumer is `#[cfg(test)]`-gated — `rg -l mockhttp` over apps/crates/tools/tests; in-crate consumers' gates verified at client.rs:464, ranged.rs:981, fetch.rs:538, remote_playlist.rs:240, hls/media.rs:738, schemes.rs:532; apps/cli use is inside `#[cfg(test)] mod tests` (demux_host.rs:621-622, use at 881); apps/web's mock_fetch is its own `#[cfg(test)]` mirror. BUT the module itself is an un-gated `pub mod` (lib.rs:36), so it COMPILES into the shipped `lumen` binary. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `rg -l "mockhttp" --glob '*.rs' apps crates tools tests` (consumer census); `rg -n "#\[cfg\(test\)\]" <each consumer>` (gates verified); `rg -n "pub mod mockhttp" crates/streaming/src/lib.rs` (36 — un-gated); baseline: orchrun-test-workspace-nofailfast.log.txt:5068-5086 (18/18 mockhttp tests ok); `sed -n` verbatim checks at 1-2, 321-324, 480-483, 511-521. |

## 1. One-paragraph verdict
The first third of the mock HTTP testkit — logged-request type, scripted route/status cursors, a real Basic/Digest auth verifier, the server config surface, and the accept loop — is solid, poison-proofed test infrastructure whose "never used by production code paths" claim I verified consumer-by-consumer (every user is test-gated; nothing shipped calls it). The module still ships: `pub mod mockhttp` is not `#[cfg(test)]`- or feature-gated, so ~1,500 lines of testkit, including the entire mock-auth machinery, are compiled into the release binary as dead weight. The remaining findings are hygiene: a 2 ms accept poll loop, a `client()` doc that misstates what it returns, and a request log whose `Debug` prints `Authorization` headers unredacted while the module preaches "same redaction discipline as the production side".

## 2. What actually works (verified)
— verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5068-5086` → 18/18 `mockhttp::tests` pass (roundtrip, 206/416, redirect chain, self-loop, scripted route/status, timeouts, garbage line, over-budget).
— verified by consumer census (`rg -l mockhttp` + per-file `#[cfg(test)]` check) → the module-doc claim "never used by production code paths" is TRUE: no un-gated caller exists anywhere in the workspace.
— `MockAuth::authorize` is a real verifier: Basic = decode+compare, Digest = recompute `digest_response` over the client's own nc/cnonce/qop (read, lines 283-316) — not a fixture string match, exactly as the doc claims.

## 3. Findings

### F-001 [S4]
Where: crates/streaming/src/mockhttp.rs:1-2 vs crates/streaming/src/lib.rs:36
Code:
```
//! Mock HTTP/1.1 testkit — **test-only; never used by production code
//! paths** (card 6.A1a).
```
What is wrong: the "test-only" posture is a comment, not an enforced property: `pub mod mockhttp;` (lib.rs:36) is un-gated, so the whole testkit — TcpListener server threads, scripted cursors, the auth challenge machinery — is compiled into every release build of `streaming` and linked into the shipped `lumen` binary.
Why it matters: dead weight and attack surface in the shipped artifact; a future contributor can start using mockhttp from production code with zero compiler resistance (the comment is the only guard).
What correct looks like: `#[cfg(any(test, feature = "testkit"))] pub mod mockhttp;` with the integration tests (streaming/tests/*) enabling the feature in dev-dependencies.
Confidence: HIGH — Verification: read of lib.rs:36; census proves no production use today.

### F-002 [S4]
Where: crates/streaming/src/mockhttp.rs:511-526
Code:
```
    while !stop.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, _)) => {
                ...
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(2));
            }
```
What is wrong: the accept loop is a 2 ms poll loop (500 wakeups/s per spawned server) instead of a blocking accept woken by a self-connect (or `poll`/`mio`); Drop latency is bounded by the same 2 ms tick.
Why it matters: testkit-only, so the cost is bounded and invisible outside tests — but it is the exact busy-wait pattern the audit checklist names, and suites that spawn many servers multiply it.
What correct looks like: on Drop, connect a dummy socket to the listener's addr to wake a blocking accept, or keep the poll but document the tick in the shutdown section (which currently says only "polls a stop flag").
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/streaming/src/mockhttp.rs:480-483
Code:
```
    /// A [`MockClient`] pointed at this server.
    pub fn client(&self) -> MockClient {
        MockClient
    }
```
What is wrong: the doc says "pointed at this server" but `MockClient` is a stateless unit struct that connects to whatever URL it is given (its own constructor doc, line 802: "A client for any mock server (stateless; the URL routes it)"); this method is `MockClient::new()` with a misleading name.
Why it matters: a reader assumes server affinity/scoping that does not exist; a test using `server_a.client()` can silently fetch from `server_b`.
What correct looks like: doc: "A stateless `MockClient`; the request URL selects the server", or delete the method in favor of `MockClient::new()`.
Confidence: HIGH — Verification: read of both sites.

### F-004 [S4]
Where: crates/streaming/src/mockhttp.rs:50-58 (with 321-323)
Code:
```
pub struct LoggedRequest {
    /// Request method as sent (`"GET"`).
    pub method: String,
    /// Path (with query, if any) as sent.
    pub path: String,
    /// Headers in wire order, names as sent.
    pub headers: Vec<(String, String)>,
}
```
What is wrong: the module's redaction claim ("same redaction discipline as the production side", 321-323) covers `MockAuth`'s Debug only; `LoggedRequest` derives plain `Debug` and records every header verbatim — including `Authorization: Basic <base64 user:pass>` — so any test assert that formats `requests()` (e.g. schemes.rs:905 prints `r.headers.clone()`) will put credentials into test output.
Why it matters: the auth e2e tests (`streaming/tests/auth_e2e.rs`) feed real-looking credentials through this log; a failing assertion is one `{:?}` away from leaking them into CI logs — the exact class of leak MP §12 forbids on the production side.
What correct looks like: a manual `Debug` for `LoggedRequest` that redacts `authorization` (and `cookie`) values, or a `redacted()` accessor used in asserts.
Confidence: HIGH — Verification: read; derive is `#[derive(Debug, Clone, PartialEq, Eq)]` at line 50, no field redaction.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-29 | REAL | — | Claims verified: test-only (census), shutdown story (traced), mock contract (part2/3 own the wire code). |
| `SERVER_READ_DEADLINE` | 42-44 | REAL | — | 2 s handler read deadline. |
| `HEAD_LIMIT` | 48-47 | REAL | — | 64 KiB head cap; enforced in `read_head` (part2, A08). |
| `LoggedRequest` | 50-58 | REAL | S4 (F-004) | Unredacted Debug of recorded headers. |
| `LoggedRequest::header` | 60-68 | REAL | — | Case-insensitive first-match; correct. |
| `ScriptedRoute` / `ScriptedRouteState` | 80-90 | REAL | — | Shared-handle clone semantics documented. |
| `ScriptedRoute::new` | 92-101 | REAL | — | Empty script ⇒ empty bodies (documented). |
| `ScriptedRoute::take` | 103-117 | REAL | — | Poison-proofed; clamps idx to last; note: empty script returns WITHOUT advancing `next` while exhausted scripts DO advance — `fetches()` counts are inconsistent between the two shapes (testkit quirk, noted). |
| `ScriptedRoute::fetches` | 119-125 | REAL | — | Poison-proofed. |
| `ScriptedStatus` / `ScriptedStatusState` | 135-145 | REAL | — | Same pattern as ScriptedRoute. |
| `ScriptedStatus::new` | 147-154 | REAL | — | — |
| `ScriptedStatus::take` | 156-168 | REAL | — | Advances only while script has entries; poison-proofed. |
| `ScriptedStatus::served` | 170-177 | REAL | — | — |
| `MockAuth` | 189-205 | REAL | — | Fields carry the fixed nonce/opaque/qop contract. |
| `MockAuthScheme` | 207-214 | REAL | — | Basic/Digest. |
| `MockAuth::basic` | 216-229 | REAL | — | — |
| `MockAuth::digest` | 231-244 | REAL | — | Hard-coded RFC 2617 example opaque token — deterministic test vector, fine. |
| `MockAuth::challenge_value` | 246-262 | REAL | — | Escaping of realm/nonce not needed (fixed test vectors); fine for testkit. |
| `MockAuth::authorize` | 264-318 | REAL | — | Real verifier (Basic decode+compare; Digest recompute with client's nc/cnonce/qop, nonce+uri+username checked); `==` compare is timing-unsafe — irrelevant for a testkit. |
| `Debug for MockAuth` | 321-334 | REAL | S4 (F-004 context) | Password redacted; but the claim's scope stops here — see F-004. |
| `MockConfig` | 336-407 | REAL | — | 13 behavior knobs, all documented; precedence order documented per field. |
| `Default for MockConfig` | 409-427 | REAL | — | `honor_range: true` matches `body()`'s "default shape" doc. |
| `MockConfig::body` | 429-437 | REAL | — | — |
| `MockServer::spawn` | 447-473 | REAL | — | Ephemeral 127.0.0.1 bind; nonblocking listener; io::Result surfaces bind failures. |
| `MockServer::url` | 475-478 | REAL | — | `http://addr/path` with slash-trim; correct. |
| `MockServer::client` | 480-483 | REAL | S4 (F-003) | Misleading doc. |
| `MockServer::requests` | 485-491 | REAL | — | Snapshot clone; poison-proofed. Logging happens BEFORE the response is written in `handle` (539 precedes response), so a client that saw its response is already logged — no assert-after-fetch race. |
| `Drop for MockServer` | 494-501 | REAL | — | Stop flag + join accept thread; handler threads are detached (bounded by 2 s deadline) — in-flight handlers can outlive the server by ≤1 deadline, acceptable and documented (26-29). |
| `accept_loop` | 503-529 | REAL | S4 (F-002) | 2 ms poll; EMFILE back-off 10 ms and continue; spawns detached handlers. |

## 5. Test honesty
No tests in this range (tests at 1040+, A10's part3). The tests that exercise this range's items pass in baseline: `scripted_route_advances_and_repeats_last` (5081), `scripted_status_answers_then_recovers` (5082), `request_log_captures_the_range_header` (5078). Mutation check (reasoned, not run): breaking `ScriptedRoute::take`'s clamp would flip `scripted_route_advances_and_repeats_last`; breaking `LoggedRequest::header`'s case-insensitivity would flip `request_log_captures_the_range_header` (the port sends `Range`).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "test-only; never used by production code paths" | mockhttp.rs:1-2 | REAL (use) / hygiene gap (compile) | Consumer census + per-file cfg(test) verification; but un-gated `pub mod` ships it (F-001). |
| "Drop closes the listener … joining cleanly" | mockhttp.rs:23-29 | REAL | Drop impl 494-501 traced; handler bound = read deadline (533, part2). |
| "a real verifier, not a string match" (Digest) | mockhttp.rs:186-188 | REAL | authorize:283-316 recomputes `digest_response` over client's nc/cnonce/qop (read). |
| "same redaction discipline as the production side" | mockhttp.rs:321-323 | SHALLOW | True for MockAuth Debug only; LoggedRequest leaks Authorization (F-004). |
| "sync-only … no external crates" | mockhttp.rs:4-6 | REAL | std::net/std::thread only (imports 31-36, read). |

## 7. Dead / duplicated / unreachable code
No production reachability at all (by design). Within the testkit: nothing dead found in this range; `MockServer::client()` (F-003) is a redundant alias for `MockClient::new()`.

## 8. Dependencies & flags
Depends on `crate::auth` (`base64_decode`, `digest_response`, `parse_auth_params`) — production code paths in auth.rs reused by the testkit (fine, testkit→production direction). `crate::error::NetworkError`, `crate::fetch` types. No feature gates anywhere in the module (F-001). No unsafe.

## 9. Open questions for the next agent
- A08 (part2): `read_head` enforcement of HEAD_LIMIT — confirm the cap is checked DURING accumulation (streaming bound), not after the head is fully read (memory bound).
- A08/A10: `MockClient`'s redirect follower reuses the port's `RedirectPolicy` — confirm the visited-set uses the normalized URL or document the case-sensitivity interaction (schemes.rs F-003 cross-ref).
- Whoever owns binary-size/product hygiene: decide whether mockhttp should be feature-gated out of release builds (F-001) — same question applies to `device-discovery` and other testkit-ish modules.
