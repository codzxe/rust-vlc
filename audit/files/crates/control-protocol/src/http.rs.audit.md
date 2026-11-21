# AUDIT: crates/control-protocol/src/http.rs
| Field | Value |
|---|---|
| Auditor | W02-A08 |
| Date | 2026-11-21 |
| Lines / functions | 493 / 17 + 5 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NOT from `lumen` CLI or `lumen-tauri` (rg `control_protocol|control-server` in apps/cli/src, apps/desktop-tauri/src, apps/desktop/src → zero hits). YES from the third workspace binary `lumen-control-server` (apps/control-server/Cargo.toml `[[bin]] name = "lumen-control-server"`): server.rs:522 `http::read_upgrade_request(stream)` → `read_head`/`validate_head`/`accept_value`/`response_101`/`write_all` all on that path. Binary built clean in baseline run #1; handshake executed over a real loopback socket in baseline run #3. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | `rg -n "read_upgrade_request|http::"` → server.rs:522-539 call chain; `rg -n "set_read_timeout|BufReader"` in control-protocol → timeouts set (server.rs:449, client.rs:57), NO buffering anywhere; `rg -n "upgrade.path|\.path"` server.rs → 0 hits (dead field); `sed -n` quote verification on every quoted range; baseline test-workspace.log.txt lines 1411-1438 (47 lib tests, all 5 http::tests ok), 1464 (conformance.rs), 1518 (e2e.rs); `rg -n "origin"` in control-protocol src → only the documented-gap comment server.rs:32 |

## 1. One-paragraph verdict
A real, careful, in-tree RFC 6455 §4.2 handshake: bounded 8 KiB head read, fail-closed loopback-Host fence (DNS-rebinding), static reason tokens that never echo request bytes, RFC-golden accept computation, and five honest tests — all executed and green in the baseline workspace run, plus a real-socket handshake in conformance.rs (baseline log line 1464). The defects are bounded: the handshake has no TOTAL deadline (only per-read timeouts), so a drip-feeding local peer can hold the deliberately single-connection server hostage for days — directly contradicting the "a silent peer can never wedge the server" invariant the server cites; the head is read one syscall per byte; `UpgradeRequest.path` is validated and stored but read by no production code; duplicate `Upgrade`/`Connection` headers use last-wins instead of the RFC's combine semantics; and the advertised bare-LF tolerance has zero test coverage.

## 2. What actually works (verified)
— `accept_value` matches the RFC 6455 §1.3 golden: test `accept_value_matches_rfc6455_example` asserts `s3pPLMBiTxaQ9kYGzzhZRbK+xOo=` — executed in baseline (test-workspace.log.txt:1434 `test http::tests::accept_value_matches_rfc6455_example ... ok`). Would fail if in-tree sha1.rs or base64.rs were broken.
— Full handshake over a real loopback socket executed in baseline run #3: conformance.rs (log line 1464) `handshake_success_full_session_round_trip` + apps/control-server/tests/e2e.rs (line 1518) drive `TestClient::connect` → `client_request` → server `read_upgrade_request` → `response_101` → `validate_server_response`.
— Loopback-Host fence is fail-closed: I traced `is_loopback_host` (169-183) by hand through `evil.example`, `127.0.0.1.evil.example`, `localhost:4355:80`, `[fe80::1]`, `localhost:`, `127.0.0.1:999…` — every non-loopback shape falls out of the `matches!` set and is refused; the port-suffix strip (175-178) only fires when the tail is ALL digits, and the bracketless `::1` gets an exact pre-check (179-181). Regression test `non_loopback_hosts_are_refused_and_loopback_forms_accepted` (438-471) covers 8 accept / 10 refuse forms — executed ok (baseline log line 1438).
— No request-byte echo: every error reason in the file is a static token (`"eof-mid-request"`, `"request-head-too-large"`, `"host-not-loopback"` …) and `error_response` (71-73) interpolates only a caller-chosen status and static reason — verified by reading all 14 reason sites.

## 3. Findings
### F-001 [S3]
Where: crates/control-protocol/src/http.rs:101-105 (deadline semantics; caller: server.rs:443-449, 147)
Code:
```
fn read_head<R: Read>(stream: &mut R) -> Result<Option<String>, ProtocolError> {
    let mut buf: Vec<u8> = Vec::with_capacity(512);
    let mut byte = [0u8; 1];
    loop {
        match stream.read(&mut byte) {
```
What is wrong: The handshake's only time bound is the socket's per-read timeout (server.rs:449 sets it from `Timeouts::documented()`, read = 60 s, server.rs:147). Each 1-byte read gets a FRESH 60 s window, and `MAX_REQUEST_HEAD` allows 8 KiB, so a peer that drips one byte every ≤60 s keeps `read_head` alive for up to 8192 × 59 s ≈ 5.6 days — while the server is "deliberately single-connection, synchronous" (server.rs:28-29), so no other control client is served. server.rs:445-446 claims "Bounded waits on both directions BEFORE any session work: a silent peer can never wedge the (single-connection) server" — true for silent peers, false for dripping ones; there is no total handshake deadline anywhere on the path.
Why it matters: Any local process (pre-authentication, no token needed) can deny the control surface indefinitely. Blast radius is bounded — loopback-only bind, optional feature (`control-server.enabled` defaults false per doc 05 §5.4), no playback impact — which is why this is S3 and not S2; but the code contradicts the wedge-resistance invariant its own sibling module states.
What correct looks like: An overall `Instant` deadline threaded into `read_upgrade_request` (e.g. `Duration::from_secs(10)` for the whole head), checked per iteration, mapping to `ProtocolError::TimedOut`; per-read timeout can then stay as the coarse backstop.
Confidence: HIGH — Verification: read http.rs + server.rs (timeouts, single-connection design, accept→run call chain).

### F-002 [S4]
Where: crates/control-protocol/src/http.rs:103-105
Code:
```
    let mut byte = [0u8; 1];
    loop {
        match stream.read(&mut byte) {
```
What is wrong: The head is read one byte per `read()` call — on the real `TcpStream` (server.rs:522, no `BufReader` anywhere in the crate), a ~200-byte browser handshake costs ~200 syscalls where one 512-byte read would typically suffice.
Why it matters: Pure syscall overhead on a per-connection path. Negligible in absolute terms for a loopback single-connection control server, which is why this is S4. Partial mitigation exists by design: a buffered reader that over-read past the `\r\n\r\n` terminator would steal pipelined WebSocket-frame bytes from the subsequent raw-stream frame loop unless explicitly handed off — the byte-at-a-time loop cannot over-read.
What correct looks like: A fixed 512-byte scratch buffer with `read` into it and an index, discarding nothing past the terminator (or a `BufReader` whose residual buffer is passed to the frame loop).
Confidence: HIGH — Verification: read + `rg -n "BufReader" crates/control-protocol/src` (zero hits).
### F-003 [S4]
Where: crates/control-protocol/src/http.rs:34-40
Code:
```
pub struct UpgradeRequest {
    /// The request target (path). v1 accepts any path — routing is a
    /// future concern (documented; not a security decision).
    pub path: String,
```
What is wrong: `path` is parsed, length-checked (line 202: `path.len() > 2048`) and stored — then read by NO production code: `rg -n "upgrade.path|\.path"` over server.rs returns nothing; the only readers are this file's own tests (369) and client.rs's request builder. Dead field today.
Why it matters: Struct field that exists only for tests; the "future routing" is honest but the field is cargo that implies routing exists to readers of the type.
What correct looks like: Either drop the field until routing lands, or keep it and mark `#[allow(dead_code)]`-style intent in one place — it already has the honest doc comment, so the cheapest fix is a `// read by tests only today` note.
Confidence: HIGH — Verification: read + rg zero production readers.

### F-004 [S4]
Where: crates/control-protocol/src/http.rs:226-235
Code:
```
            "upgrade" => {
                upgrade = value
                    .split(',')
                    .any(|token| token.trim().eq_ignore_ascii_case("websocket"));
            }
            "connection" => {
                connection_upgrade = value
                    .split(',')
                    .any(|token| token.trim().eq_ignore_ascii_case("upgrade"));
            }
```
What is wrong: Duplicate `Upgrade`/`Connection` headers overwrite: a request with `Connection: Upgrade` followed by `Connection: keep-alive` ends with `connection_upgrade = false` and is refused. HTTP semantics (RFC 9110 §5.2, which RFC 6455 leans on) say repeated headers are equivalent to a comma-joined list; the RFC-correct computation is `upgrade |= …` / `connection_upgrade |= …` (last-wins `=` instead of accumulating `|=`).
Why it matters: Fail-closed, not exploitable — a weird-but-legal client splitting tokens across duplicate header lines is rejected. Conformance nit only; browsers never emit these split forms.
What correct looks like: `upgrade |= value.split(',').any(…)` and the same for `connection_upgrade`.
Confidence: HIGH — Verification: read; no test covers duplicate headers either way.
### F-005 [S4]
Where: crates/control-protocol/src/http.rs:148-151 (claim at 4-5)
Code:
```
/// `\r\n\r\n` or (dirty-world) a bare `\n\n` terminator.
fn ends_with_head_terminator(buf: &[u8]) -> bool {
    buf.ends_with(b"\r\n\r\n") || buf.ends_with(b"\n\n")
}
```
What is wrong: The module doc advertises "dirty-world tolerant: bare-LF lines accepted" (lines 4-5), and `ends_with_head_terminator` + `trim_end_matches('\r')` (190) implement it — but NO test anywhere feeds a bare-LF handshake: all five in-file tests and conformance.rs use `\r\n` forms exclusively. Deleting the `|| buf.ends_with(b"\n\n")` arm would leave every test green.
Why it matters: A documented dirty-world behavior with zero regression protection.
What correct looks like: One test case: `"GET / HTTP/1.1\nHost: 127.0.0.1\nUpgrade: websocket\nConnection: Upgrade\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\nSec-WebSocket-Version: 13\n\n"` must parse.
Confidence: HIGH — Verification: read all tests + `rg -n "bare|\\n\\n" conformance.rs` (no hits).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `WEBSOCKET_GUID` const | 23 | REAL | — | Byte-checked against RFC 6455 §1.3: `258EAFA5-E914-47DA-95CA-C5AB0DC85B11` — correct. |
| `WEBSOCKET_VERSION` const | 26 | REAL | — | 13, correct. |
| `MAX_REQUEST_HEAD` const | 30 | REAL | — | 8 KiB cap, enforced in read_head:117. |
| `UpgradeRequest` struct | 33-40 | REAL | S4 | `path` dead in production — F-003. |
| `accept_value` | 49-55 | REAL | — | Decodes→re-encodes canonical base64 (drift-proof note at 43-48 is accurate); RFC-golden tested + baseline-executed. Non-canonical senders get a mismatched accept and fail closed — stricter than RFC, documented. |
| `response_101` | 58-67 | REAL | — | Static lines + accept; RFC-correct shape; `\`-continuations strip leading whitespace (verified by roundtrip test). |
| `error_response` | 71-73 | REAL | — | Static reason; no CRLF injection surface (no dynamic interpolation). |
| `read_upgrade_request` | 87-97 | REAL | — | Clean EOF→`Closed`; validation failure→typed `HttpUpgrade`. |
| `read_head` | 101-146 | REAL | S3/S4 | Per-byte read (F-002); no total deadline (F-001); head cap enforced AFTER push (117) so max buffer 8 KiB+1; WouldBlock/TimedOut→typed `TimedOut` (live: server.rs:449 sets the socket timeout); disconnect-with-empty→`Ok(None)`. |
| `ends_with_head_terminator` | 149-151 | REAL | S4 | Correct; bare-LF arm untested (F-005). |
| `is_disconnect` | 154-159 | REAL | — | ConnectionReset/BrokenPipe only; conservative. |
| `is_loopback_host` | 169-183 | REAL | — | Fail-closed under every adversarial form I traced (see §2); rejects `127.0.0.2` (conservative, fine). |
| `validate_head` | 189-272 | REAL | S4 | Method/version/path checks sound; malformed lines tolerated then caught by required-header checks; key decode length-guarded (no panic; `copy_from_slice` only in the 16-byte arm, 260-266); duplicate-header last-wins (F-004). |
| `client_request` | 275-287 | REAL | — | Test-client only (client.rs doc: "Not part of any production path") — shipped in lib but documented as conformance tooling. |
| `validate_server_response` | 294-323 | REAL | — | Test-client only; exact accept compare; multiple accept headers → last wins (test-scope, fine). |
| `read_response_head` | 327-333 | REAL | — | Test-client only; reuses bounded read_head. |
| `write_all` | 336-341 | REAL | — | Trivial typed wrapper; used by server (server.rs:539) and client. |

## 5. Test honesty (in-file tests; all executed in baseline run #3, log lines 1434-1438)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `accept_value_matches_rfc6455_example` (348) | YES | RFC golden value; breaks if sha1/base64 regress. (`unwrap_or_default` at 349 is sloppy but a decode failure still fails the assert.) |
| `valid_request_parses` (358) | YES | Real header shape incl. `Connection: keep-alive, Upgrade` token list; `unreachable!` on rejection. |
| `dirty_requests_are_stable_reason_tokens` (377) | YES | 8 dirty cases with exact expected tokens + good-case sanity; any token rename/reorder fails. |
| `non_loopback_hosts_are_refused_and_loopback_forms_accepted` (427) | YES | 8 accept / 10 refuse hosts incl. `localhost.evil.example`, `127.0.0.1.evil.example`; asserts the exact `host-not-loopback` token. |
| `client_roundtrip_against_server_validation` (475) | YES (weakly) | Self-roundtrip + doctored-accept rejection; the crypto strength comes from the RFC-golden test, not this one. |
| Untested: bare-LF handshake | — | F-005 — documented tolerance with no test. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "bounded read" (8 KiB head cap) | http.rs:28-30 (card 7.J1 / doc 01 §2.3) | REAL | read_head:117 enforces `MAX_REQUEST_HEAD`; executed in baseline conformance run. |
| "dirty-world tolerant: bare-LF lines accepted" | http.rs:4-5 | REAL-UNPROVEN | Implemented (150, 190) but zero tests feed bare-LF — F-005. |
| "request bytes never echoed into errors" | http.rs:5 | REAL | All 14 reason tokens are static strings; `error_response` interpolates status only. |
| "base64(SHA-1(key + GUID)) over the RFC 6455 GUID — the two in-tree primitives" | http.rs:11-13 | REAL | RFC 6455 §1.3 golden asserted and executed (baseline log 1434). |
| Loopback Host fence (DNS-rebinding, sweep 3/4) | http.rs:161-168 | REAL | Implemented + adversarial test executed (log 1438). |
| "Host: evil.com rebinding … refused" + "Origin check … `[PROPOSAL]`" | docs/05-command-layer.md:545 §5.4 | PAPER-ONLY (proposal) | Origin is checked NOWHERE (`rg -n origin` → only server.rs:32 "recorded gaps for 8.A2"); the Host fence is the implemented mitigation. Honest documented gap, not a false claim. |
| "Transport: WebSocket … never hand-rolled on either side" | docs/05-command-layer.md:540 | REAL (for the envelope) | The serde envelope (protocol.rs) is typed; the WS transport itself is deliberately in-tree (card 7.J1) — the doc's "never hand-rolled" binds to the envelope, which holds. |
| `UpgradeRequest.path` "v1 accepts any path — routing is a future concern" | http.rs:35-36 | REAL | No routing exists; field itself is dead — F-003. |

## 7. Dead / duplicated / unreachable code
`UpgradeRequest.path` — dead in production (F-003). `client_request`/`validate_server_response`/`read_response_head` — production-lib code serving only the conformance client (documented as such in client.rs:14). No duplication; no `#[allow]`; no unsafe.

## 8. Dependencies & flags
No external deps (std only) + in-tree `crate::sha1`/`crate::base64`. The crate is wired into `apps/control-server` (Cargo.toml:42), which is a workspace member built by baseline run #1. Not referenced by apps/cli, apps/desktop, apps/desktop-tauri (rg zero hits) — the control surface is its own binary, gated by `control-server.enabled` (default false, doc 05 §5.4).

## 9. Open questions for the next agent
- server.rs auditor: confirm the accept loop is strictly sequential (one `accept()` per `run()`), which is what turns F-001's drip into a total service wedge; and whether `Session::run`'s frame loop inherits the same per-read-only deadline pattern post-handshake (larger attack window: whole session, 1 MiB frames).
- policy.rs/server.rs auditor: when `read_only = false` (default, policy.rs:140), what exactly does an unauthenticated browser client (Host fence passes, no token) get? If the answer is "nothing + close", the missing Origin check (doc 05 §5.4 [PROPOSAL]) is harmless; if any hello-without-token reaches dispatch, it is not.
- sha1.rs/base64.rs auditors: the RFC golden in this file is your execution evidence — but check base64::decode's strictness on non-canonical input; http.rs is safe regardless (re-encode pattern), other callers may not be.
