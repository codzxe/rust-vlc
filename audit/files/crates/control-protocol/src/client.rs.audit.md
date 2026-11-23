# AUDIT: crates/control-protocol/src/client.rs
| Field | Value |
|---|---|
| Auditor | W02-A05 |
| Date | 2026-11-23 |
| Lines / functions | 311 / 14 (1 ctor + 12 methods, 1 free fn, no in-file tests) |
| File verdict | REAL (test infrastructure — real sockets; by design NOT a production client) |
| Reachable from shipped entry point? | NO — test-only: used by crates/control-protocol/tests/conformance.rs and apps/control-server/tests/e2e.rs; no production caller exists (rg verified) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | conformance suite (uses this client) -> 25 passed exit 0 (run in File 1); `rg`/`sed` cross-refs incl. the SEC-07 baseline cross-ref |

## 1. One-paragraph verdict
This is a REAL minimal RFC 6455 client over a real `TcpStream` — masked writes, unmasked reads with mirrored strictness (a masked server frame is a typed violation), 10 s per-op deadlines, and a `send_raw` seam that puts genuinely dirty bytes on the wire (truncated headers, bad masks, oversize claims). It is honest about being test infrastructure ("Not part of any production path", L14-15) and the conformance suite that drives the real server through it passes 25/25 (executed in File 1's audit). The hostile answer for the "protocol client" domain: there IS no production control client anywhere in the tree — the only client is this loopback test tool, which is exactly what a pre-Phase-5 control feature should have. The baseline cross-ref is CONFIRMED and pinned down (see §6): docs/11 SEC-07 documents a `LUMEN_DEV_INSECURE_TLS=1` dev override that exists in NO code anywhere; the `tls_config` the doc cites is in `crates/streaming/src/client.rs:91-96` (a different file from this one) and reads no environment — and this file has no TLS at all.

## 2. What actually works (verified)
— The conformance suite that drives the real server exclusively through `TestClient` (rg: `TestClient::connect` at conformance.rs:169, `send_raw` at :506/:528/:547, `expect_close` at :192/:1043) -> `./target/debug/deps/conformance-fc3f1f30343cb626` -> "25 passed", exit 0 (run this session, File 1).
— Real handshake: `TcpStream::connect` (L52), `http::client_request`/`read_response_head`/`validate_server_response` (L80-83) — the client verifies the server's `Sec-WebSocket-Accept` against its own nonce (key is kept `pub` for that, L36-38).

## 3. Findings
F-001 [S4]
Where: client.rs:76-78
Code:
```
for (i, byte) in key.iter_mut().enumerate() {
    *byte = (nanos >> ((i % 8) * 8)) as u8 ^ (0x5a_u8.rotate_left(i as u32 % 8));
}
```
What is wrong: both `i % 8` expressions have period 8, so `key[8..16] == key[0..8]` — the "16-byte" nonce is 8 bytes of clock entropy duplicated, and those 8 bytes are public wall-clock nanos, not random. Documented as acceptable for a TEST client ("only needs to be 16 varied bytes", L68-70) and the WS nonce is not a secret; still, the code does not do what "16-byte nonce" implies to an RFC reader. Confidence: HIGH — Verification: read (period arithmetic).
F-002 [S4]
Where: client.rs:303-305
Code:
```
fn read_client_side_frame<R: Read>(stream: &mut R) -> Result<Option<Frame>, ProtocolError> {
    crate::frame::read_frame_unmasked(stream, 1 << 20)
```
What is wrong: the client's read cap is hard-coded 1 MiB while the server's caps are configurable (`ServerConfig.limits`); a test that configures a larger server frame cap and expects a >1 MiB server frame would get a typed `MessageTooLarge` from the CLIENT, not the behavior under test. Latent test-harness limitation only. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: client.rs:72-75 (and 91-94)
Code:
```
let nanos = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_nanos() as u64)
    .unwrap_or_default();
```
What is wrong: swallowed error (clock before epoch → silent 0) — in a test client the consequence is a degenerate nonce/mask, not a wrong verdict; recorded for the swallowed-errors ledger. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| TEST_CLIENT_DEADLINE (const) | 31 | REAL | — | 10 s; anti-hang guarantee, honest history note |
| TestClient::connect | 51-85 | REAL | — | real connect + deadlines + full RFC 6455 handshake validation; weak nonce (F-001) |
| TestClient::fresh_mask | 90-102 | REAL | — | time-derived mask; documented justification (masking is not a security feature) |
| TestClient::send_text | 108-116 | REAL | — | masked write |
| TestClient::send_binary | 122-130 | REAL | — | dirty-world binary frames |
| TestClient::send_raw | 137-139 | REAL | — | THE honest-seam: raw hostile bytes on a real socket |
| TestClient::shutdown_write | 148-155 | REAL | — | FIN for truncated-frame cases |
| TestClient::ping | 162-170 | REAL | — | |
| TestClient::close | 176-184 | REAL | — | |
| TestClient::send_text_masked | 191-199 | REAL | — | explicit mask for byte-reproducible fragmentation tests |
| TestClient::read_frame | 206-208 | REAL | S4 | 1 MiB hard cap (F-002) |
| TestClient::expect_text | 215-233 | REAL | — | exact-payload assertion |
| TestClient::expect_pong | 239-247 | REAL | — | |
| TestClient::expect_close | 254-265 | REAL | — | validates §5.5.1 payload shape via parse_close_payload |
| TestClient::read_server_frame | 271-286 | REAL | — | requires final text frame; typed JSON errors |
| read_client_side_frame | 303-311 | REAL | — | enforces unmasked server frames (mirrored strictness) |

## 5. Test honesty (if tests exist)
No `#[cfg(test)]` in this file — it IS test infrastructure. Its own honesty is demonstrated externally: the 25 conformance cases that use it include genuinely adversarial inputs (`send_raw` truncated headers, oversize claims, bad masks) and pass against the real server (executed, exit 0).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "client — the in-tree WS conformance client used by this crate's and the app's tests (doc 01 §2.3 (e))" | crates/control-protocol/src/lib.rs | REAL | conformance.rs + apps/control-server/tests/e2e.rs import it; suite executed 25/25 |
| "deliberately NOT a general client: no TLS, no extensions" | client.rs:8-9 | REAL | no TLS code in file; loopback test client |
| "Not part of any production path" | client.rs:14-15 | REAL | rg: no non-test caller |
| SEC-07: "Dev-only override via `LUMEN_DEV_INSECURE_TLS=1`, refused in release builds (`debug_assertions`-gated compile-out)" + acceptance "env override no-ops in release" | docs/11-dependency-security-compliance.md L494 (and L737: "`LUMEN_DEV_INSECURE_TLS` mechanism (debug-only, env var)") | PAPER-ONLY (the override mechanism) | `rg LUMEN_DEV` across all non-audit files: ZERO code hits — no env read, no debug_assertions gate, no insecure path exists anywhere. `crates/streaming/src/client.rs:91-96` `tls_config()` is a 6-line builder (Rustls provider + WebPki roots) that never calls `disable_verification` and reads no env — verified by sed. The doc's "verification never off — crates/streaming/src/client.rs::tls_config" is TRUE; the documented override is implemented nowhere, so "no-ops in release" is vacuously satisfied and untestable as written. NOTE: docs/11's `client.rs::tls_config` refers to crates/streaming/src/client.rs — NOT this file. Wave-1 cross-ref CONFIRMED with that disambiguation. |

## 7. Dead / duplicated / unreachable code
None dead. Whole file is UNREACHABLE from shipped entry points by design (test infrastructure). The duplicated-nonce half (F-001) is the only redundancy.

## 8. Dependencies & flags
No external deps beyond the crate's own http/frame/protocol/error modules and std. No feature flags, no cfg gates, no unsafe, no unwrap/expect (unwrap_or_default noted in F-003).

## 9. Open questions for the next agent
- docs/11 SEC-07 owner: the `LUMEN_DEV_INSECURE_TLS=1` `[PROPOSAL]` override has no implementation; either implement the debug-gated env override or delete the acceptance criterion "env override no-ops in release" (vacuous today). Also L737 references "the `LUMEN_DEV_INSECURE_TLS` mechanism (debug-only, env var)" as if it existed.
- crates/streaming/src/client.rs (999 LOC, not my file) owns `tls_config`/`agent_config`/redirect policy — worth its own audit row for SEC-07/SEC-08 acceptance (`sec_tls_*` tests are the [VERIFY: networked CI lane] gap the doc itself records).
- When Phase 5 ships a production control client (UI/remote), this file must NOT be it — confirm the plan (doc 01 §2.3 (e)) keeps production clients out of this test crate.
