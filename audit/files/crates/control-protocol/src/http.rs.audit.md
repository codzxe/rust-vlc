# AUDIT: crates/control-protocol/src/http.rs
| Field | Value |
|---|---|
| Auditor | W02-A07 |
| Date | 2026-11-23 |
| Lines / functions | 493 / 13 functions + 5 tests |
| File verdict | REAL — a real, executed RFC 6455 §4.2 WebSocket upgrade handshake (NOT a general HTTP server, and it never claims to be). UNREACHABLE from the release lane (lumen-cli/desktop have no edge on control-protocol). |
| Reachable from shipped entry point? | NO for the release lane (apps/cli/Cargo.toml and apps/desktop/Cargo.toml list no control-protocol). YES via workspace bin `lumen-control-server`: main → control_server::run → control_protocol::server::Session::run (server.rs:522 `http::read_upgrade_request`) → this file. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | `cargo test -p control-protocol --lib http` → 5 passed/0 failed; `cargo test -p control-protocol --test conformance` → 25 passed/0 failed (real loopback TCP, 0.16s); rg callers/error_response/.path |

## 1. One-paragraph verdict
Answering the hostile question head-on: http.rs is neither a stub nor an HTTP server. It is a complete, hostile-input-tested implementation of exactly one HTTP exchange — the RFC 6455 §4.2 WebSocket upgrade (server: bounded 8 KiB head read, case-insensitive headers, token-list Connection, loopback-only Host fence, 16-byte base64 key shape check, SHA-1+base64 accept, 101 response; client: request builder + response validator). The `Sec-WebSocket-Accept` computation reproduces the RFC 6455 §1.3 known-answer vector (executed), and 25 conformance tests drive the handshake over real loopback TCP sockets (executed). It serves no documents, routes nothing, and parses no bodies — by design. The one genuine defect is documentation that lies about the failure-return shape of `read_upgrade_request` (F-001); the rest is hygiene. None of this is reachable from the shipped `lumen`/`lumen-desktop` binaries — only from the `lumen-control-server` workspace bin, which the release lane does not build.

## 2. What actually works (verified)
— verified by `cargo test -p control-protocol --lib http` → `5 passed` (incl. `accept_value_matches_rfc6455_example`: key `dGhlIHNhbXBsZSBub25jZQ==` → `s3pPLMBiTxaQ9kYGzzhZRbK+xOo=`, the RFC 6455 §1.3 vector — proves sha1.rs + base64.rs + accept_value jointly correct).
— verified by `cargo test -p control-protocol --test conformance` → `25 passed`, including `handshake_success_full_session_round_trip`, `handshake_failure_non_upgrade_http_gets_an_http_error`, `non_loopback_bind_is_refused_at_construction_conformance` — real sockets, real server loop.
— DNS-rebinding fence `is_loopback_host` (L169-183) hand-checked against 15 forms: `localhost.evil.example`, `127.0.0.1.evil.example`, `evil.example.127.0.0.1`, `localhost:`, `127.0.0.1.1`, `[::2]`, `10.0.0.1(:port)` all refused; `127.0.0.1`, `localhost(:port)`, `LOCALHOST`, `[::1](:port)`, bare `::1` accepted — fail-closed, matches the executed test `non_loopback_hosts_are_refused_and_loopback_forms_accepted`.

## 3. Findings
F-001 [S3]
Where: crates/control-protocol/src/http.rs:78-86 (doc) vs :87 (signature)
Code:
```
/// are trimmed. Failure returns the HTTP error response to write, plus
/// the typed error.
```
What is wrong: `read_upgrade_request` returns `Result<UpgradeRequest, ProtocolError>` — on failure ONLY the typed error comes back; no error-response bytes are returned, yet the doc says twice that they are ("Failure returns the HTTP error response to write, plus the typed error" L80-81; "The caller writes the returned error response bytes" L86). The real contract — visible only in the sole production caller — is that the caller maps the error to a status itself (server.rs:526-532 matches on the reason token, then calls `http::error_response`). Why it matters: an integrator following this doc will wait for bytes that never exist, or double-write an error page. What correct looks like: doc rewritten to "the caller derives and writes the error response (see server.rs)", or the signature changed to return `(Vec<u8>, ProtocolError)`. Confidence: HIGH — Verification: read (signature L87-97 + server.rs:522-536).
F-002 [S4]
Where: http.rs:101-124
Code:
```
fn read_head<R: Read>(stream: &mut R) -> Result<Option<String>, ProtocolError> {
    let mut buf: Vec<u8> = Vec::with_capacity(512);
    let mut byte = [0u8; 1];
    loop {
        match stream.read(&mut byte) {
```
What is wrong: one `read()` syscall per byte — up to 8193 syscalls for a maximal head. The design buys a real property (no over-read past the head into the first WS frame, since the server hands the raw `TcpStream` onward, server.rs:517), so it is correct — but a `recv`-with-peek or an explicitly retained buffer would serve both. Handshake-only path, so cost is bounded and small. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: http.rs:300
Code:
```
    if !status.starts_with("HTTP/1.1 101") {
```
What is wrong: `starts_with` accepts malformed status lines like `HTTP/1.1 1010 …` or `HTTP/1.1 101junk`. Test-client-only surface (validate_server_response is documented "test client"), no server exposure. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: http.rs:37-39 vs consumers
Code:
```
    /// The request target (path). v1 accepts any path — routing is a
    /// future concern (documented; not a security decision).
    pub path: String,
```
What is wrong: `UpgradeRequest.path` is populated, length-checked (≤2048) and then read by nobody in production — the only reader is http.rs's own test (L369). Dead field with an honest deferral comment. Confidence: HIGH — Verification: read + rg (`upgrade.path`/`.path` in crates/control-protocol → only http.rs:369).
F-005 [S4]
Where: http.rs:71-73
Code:
```
pub fn error_response(status: u16, reason: &str) -> Vec<u8> {
    format!("HTTP/1.1 {status} {reason}\r\nConnection: close\r\n\r\n").into_bytes()
}
```
What is wrong: `reason` is interpolated unvalidated; a future caller passing request-derived text containing `\r\n` gets HTTP response splitting. The doc (L69-70) says "the reason is a static string, never request-derived" and the sole production caller honors it (server.rs:532, literal "Bad Request") — but nothing enforces it (no assert/debug_assert on CRLF). Confidence: HIGH — Verification: read + rg (all callers).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| WEBSOCKET_GUID / WEBSOCKET_VERSION / MAX_REQUEST_HEAD consts | 23-30 | REAL | — | GUID matches RFC 6455 §1.3 verbatim; 8 KiB cap enforced at L117 |
| UpgradeRequest | 33-40 | REAL | S4 | `path` field unconsumed (F-004) |
| accept_value | 49-55 | REAL | — | RFC known-answer vector executed; re-encodes key so text/bytes can't drift (L44-48 note verified) |
| response_101 | 58-67 | REAL | — | Rust `\`-continuation strips indentation; exact bytes exercised by conformance suite |
| error_response | 71-73 | REAL | S4 | Unvalidated `reason` interpolation (F-005); static callers today |
| read_upgrade_request | 87-97 | REAL | S3 | Behavior real; doc lies about failure return (F-001) |
| read_head | 101-146 | REAL | S4 | Bounded, EOF/disconnect/timeout handled; 1-byte reads (F-002) |
| ends_with_head_terminator | 149-151 | REAL | — | `\r\n\r\n` + dirty `\n\n` |
| is_disconnect | 154-159 | REAL | — | Reset/BrokenPipe |
| is_loopback_host | 169-183 | REAL | — | Fail-closed rebinding fence; adversarially tested (executed) |
| validate_head | 189-272 | REAL | — | Duplicate headers last-wins (fail-closed direction); malformed lines skipped; key shape-checked to 16 bytes; request bytes never echoed |
| client_request | 275-287 | REAL | — | Test client builder |
| validate_server_response | 294-323 | REAL | S4 | `starts_with` leniency (F-003); last duplicate accept wins |
| read_response_head | 327-333 | REAL | — | Reuses bounded read_head |
| write_all | 336-341 | REAL | — | Typed error mapping |
| tests ×5 | 347-492 | REAL | — | All executed; honesty table below |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| accept_value_matches_rfc6455_example | YES | External known-answer vector (RFC 6455 §1.3); breaks if sha1/base64/concat order regresses |
| valid_request_parses | YES | Asserts parsed path + round-tripped key |
| dirty_requests_are_stable_reason_tokens | YES | 8 malformed requests × exact reason tokens |
| non_loopback_hosts_are_refused_and_loopback_forms_accepted | YES | 8 accept-forms + 10 hostile forms with exact refusal token |
| client_roundtrip_against_server_validation | YES* | Self-roundtrip anchors both sides to each other; anchored externally by the RFC-vector test |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "RFC 6455 §4.2 HTTP upgrade handshake — card 7.J1, in-tree" | http.rs:1 | REAL | 5 unit + 25 conformance tests executed |
| "compute `Sec-WebSocket-Accept`, write the 101" | http.rs:3-6 | REAL | RFC vector test + `handshake_success_full_session_round_trip` (executed) |
| "Failure returns the HTTP error response to write, plus the typed error" | http.rs:80-81 | FALSE | Signature L87 returns only `ProtocolError`; caller derives bytes (server.rs:526-532) |
| "the reason is a static string, never request-derived" | http.rs:69-70 | REAL-UNPROVEN | True of the sole production caller (server.rs:532) but unenforced (F-005) |
| "bounded read, dirty-world tolerant: bare-LF lines accepted" | http.rs:4 | REAL | read_head L122/L150 + dirty-tokens test |

## 7. Dead / duplicated / unreachable code
Not reachable from the shipped release lane: no `lumen-cli`/`desktop` dependency on control-protocol (both manifests verified). Live only via `lumen-control-server` (workspace member, not in `-p lumen-cli -p desktop` release lane). `UpgradeRequest.path` is dead-in-production (F-004). No fuzz target covers this boundary (scripts/quality/22_attack_surface.sh:32,32 lists control protocol as untrusted with no fuzz prefix mapping — the script itself would flag "no fuzz target").

## 8. Dependencies & flags
Std-only (`std::io::Read/Write`); in-tree `crate::sha1`, `crate::base64`, `crate::error::ProtocolError`. No feature gates, no unsafe, no external crates — consistent with deny.toml-era admission posture recorded in lib.rs.

## 9. Open questions for the next agent
- Who packages/ships `lumen-control-server` (packaging/ has zero references — rg)? If nothing ships it, the whole crate is dev/test-only surface.
- server.rs audit (sibling agent) — confirm the token handshake and failure bookkeeping around the L522 call; F-001's doc fix belongs with whoever touches http.rs next.
- `validate_server_response` is exported `pub` but documented "test client" — is it referenced outside tests/conformance.rs?
