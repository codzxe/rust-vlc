# AUDIT: crates/control-protocol/src/client.rs
| Field | Value |
|---|---|
| Auditor | W02-A03 |
| Date | 2026-11-21 |
| Lines / functions | 311 / 18 (const, struct, 14 methods, helper fn, Frame struct) |
| File verdict | UNREACHABLE from shipped entry points (by design, declared) — REAL as test infrastructure, EXECUTED via baseline conformance (25 PASS) + e2e (6 PASS) |
| Reachable from shipped entry point? | NO (only `apps/control-server` links control-protocol; only its tests and this crate's tests construct `TestClient`; the `lumen` CLI does not depend on control-protocol — verified via Cargo.toml grep) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:4 |
| Commands run | `rg "TestClient"` whole tree → only tests/conformance.rs + apps/control-server/tests/e2e.rs (test-only consumers); `rg "Opcode::Ping" server.rs` → line 552 (server only ANSWERS pings, never sends — client needs no pong-send); `rg "\.key"` in both test files → 0 matches (dead pub field); sed -n verification of every quoted range; baseline test-workspace.log.txt:1411-1419 (control_protocol 47 passed), :1413-1417 (conformance 25 passed), :1525-1529 (e2e 6 passed); crate Cargo.toml read (serde/serde_json/commands only — no external WS/SHA-1 crates) |

## 1. One-paragraph verdict
A clean, honest, tightly-scoped RFC 6455 test client that does exactly what its module doc promises: masked writes, unmasked reads, raw-byte injection for dirty-world conformance, deadlines on both socket directions, typed errors on every path, and zero `unsafe`/`unwrap`/`expect`/`panic` (verified by full read). It is executed evidence, not decoration: the baseline conformance suite (25 tests, PASS) and the control-server e2e suite (6 tests, PASS) drive the real server through this client over real loopback sockets, including truncated headers, unmasked frames, a 4-GiB length claim, fragmentation with interleaved pings, and binary-frame rejection. It is unreachable from every shipped binary (test-only consumers; the `lumen` CLI does not even link the crate) — which the module doc states outright. All four findings are hygiene: a dead `pub key` field whose documented purpose no test exercises, a method named `send_text_masked` that sends any opcode (and is USED for Continuation frames), a patterned 8-byte-entropy nonce behind a "16 varied bytes" comment, and unconditional compilation of test-only code into the production lib.

## 2. What actually works (verified)
— The client executed against the real server: baseline test-workspace.log.txt:1413-1417 `Running tests/conformance.rs … running 25 tests … test result: ok. 25 passed` — the suite's connect/send/expect helpers are this file's methods (`TestClient::connect` at conformance.rs:169, `read_server_frame` via `next_frame`:183, `expect_close` via `close_code`:191).
— End-to-end through the app binary: baseline:1525-1529 `Running tests/e2e.rs … 6 passed` (apps/control-server/tests/e2e.rs:292 constructs `TestClient::connect`).
— Dirty-world paths executed: conformance.rs:506 `send_raw(&[0x81, 0x80 | 126, 0x01])` (truncated header → server 1002), :547 4-GiB length claim → 1009, :984 `send_binary` → rejected-but-survives, :625-637 fragmented hello with continuation frames.
— No panic paths: full read — all fallible ops return `ProtocolError`; the only `unwrap_or_default()` (75, 94) is on `SystemTime::duration_since`, which cannot fail for UNIX_EPOCH-forward clocks and degrades to 0 harmlessly.

## 3. Findings

### F-001 [S4] — `pub key` field is dead: no test reads it
Where: crates/control-protocol/src/client.rs:34-39
Code:
```rust
pub struct TestClient {
    stream: TcpStream,
    /// The nonce this client offered in the upgrade (kept so tests can
    /// verify the accept computation was truly key-derived).
    pub key: [u8; 16],
}
```
What is wrong: `rg "\.key"` over both consumer test files (tests/conformance.rs, apps/control-server/tests/e2e.rs) returns zero matches. The accept-derivation check the comment promises already happens inside `connect` (client.rs:83 `http::validate_server_response(&head, &key)?`); no test ever re-derives the accept from the exposed field.
Why it matters: the documented justification for a `pub` field on an otherwise private-fields struct is false in practice — dead API surface on a test type.
What correct looks like: drop the field (keep the local in `connect`), or add the conformance test the comment describes (re-derive Sec-WebSocket-Accept from `client.key` and compare).
Confidence: HIGH — Verification: read + rg over both test files.

### F-002 [S4] — `send_text_masked` is named for TEXT but sends any opcode — and is used for Continuation
Where: crates/control-protocol/src/client.rs:186-199
Code:
```rust
    /// Send one masked TEXT frame with an EXPLICIT mask (wire-byte
    /// reproducibility for fragmentation tests).
    ///
    /// # Errors
    /// [`ProtocolError::Io`] on write failure.
    pub fn send_text_masked(
        &mut self,
        fin: bool,
        opcode: Opcode,
        payload: &[u8],
        mask: [u8; 4],
    ) -> Result<(), ProtocolError> {
        write_masked_frame(&mut self.stream, fin, opcode, payload, mask)
    }
```
What is wrong: the doc says "one masked TEXT frame" but the signature takes an arbitrary `Opcode`, and the crate's own conformance suite sends `Opcode::Continuation` through it (conformance.rs:632 `client.send_text_masked(false, Opcode::Continuation, &payload[5..40], [2, 2, 2, 2])`).
Why it matters: a name that lies on a conformance tool — a reader auditing "text frame" paths would miss that fragmentation legs ride the same method.
What correct looks like: rename to `send_masked_frame` (matching `write_masked_frame` underneath) and fix the doc line.
Confidence: HIGH — Verification: read + conformance.rs:630-634 sed.

### F-003 [S4] — Patterned nonce: 8 distinct bytes, each appearing twice, behind a "16 varied bytes" comment
Where: crates/control-protocol/src/client.rs:68-78 (and 90-102 fresh_mask)
Code:
```rust
        // A locally-generated nonce (this is a TEST client; the value
        // only needs to be 16 varied bytes — the accept check pins the
        // derivation).
        let mut key = [0u8; 16];
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or_default();
        for (i, byte) in key.iter_mut().enumerate() {
            *byte = (nanos >> ((i % 8) * 8)) as u8 ^ (0x5a_u8.rotate_left(i as u32 % 8));
        }
```
What is wrong: for `i` in 0..16, `i % 8` repeats — `key[i] == key[i + 8]` for all i. The 16-byte nonce carries exactly 8 distinct patterned bytes derived from one clock reading. Harmless for a loopback test client (RFC 6455 accepts any 16 bytes; the accept check pins the derivation), but the comment's "16 varied bytes" oversells 8 bytes of clock entropy. Similarly `fresh_mask` (90-102) mixes one `subsec_nanos` reading with shifts and ORs an unexplained `| 1` into byte 2 (its only real effect: the mask can never be all-zero — undocumented).
Why it matters: test-client crypto-adjacent code that looks random but is patterned invites copy-paste into places where it matters; the comments describe intent the code does not deliver.
What correct looks like: comment says "8 bytes of clock entropy, mirrored" (or use a real PRNG seeded per test), and one line documenting the `| 1` non-zero-mask guarantee.
Confidence: HIGH — Verification: read + arithmetic traced (i and i+8 produce identical expressions).

### F-004 [S4] — Test-only module compiled unconditionally into the production lib
Where: crates/control-protocol/src/client.rs:14-15 + crates/control-protocol/src/lib.rs:61
Code:
```rust
//! Not part of any production path: `pub` because integration tests
//! in the app crate drive the same server through it.
```
lib.rs:60-61:
```rust
pub mod base64;
pub mod client;
```
What is wrong: `pub mod client;` is not cfg-gated, so the test client compiles into `libcontrol_protocol` and links into `lumen-control-server` (the only app depending on the crate) where nothing calls it. The doc comment's "Not part of any production path" is true of execution, false of linkage — the binary carries the dead code.
Why it matters: dead weight + surface in a network-facing server binary; a reviewer trusting "not part of any production path" would not look for it there.
What correct looks like: a `test-util` cargo feature gating the module (integration tests enable it), or accept and reword ("not executed on any production path; compiled in for cross-crate test access").
Confidence: HIGH — Verification: read lib.rs + `rg "control-protocol"` over apps/*/Cargo.toml → only apps/control-server.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `TEST_CLIENT_DEADLINE` (const) | 27-31 | REAL | — | 10 s both directions; maps to typed TimedOut via frame.rs:109-115/http.rs:129-131 (read, cross-file). |
| `TestClient` (struct) | 33-39 | REAL | S4 (F-001) | `pub key` dead. |
| `TestClient::connect` | 41-85 | REAL (executed) | S4 (F-003) | Connect itself has no deadline (loopback-refused is instant; acceptable); handshake validated against our nonce (:83). Executed by all 31 baseline tests. |
| `TestClient::fresh_mask` | 87-102 | REAL | S4 (F-003) | `nanos << 11` on u32 silently truncates (shift, not arithmetic — no panic); `| 1` undocumented non-zero guarantee. |
| `TestClient::send_text` | 104-116 | REAL (executed) | — | Masked text; used by every request-sending conformance leg. |
| `TestClient::send_binary` | 118-130 | REAL (executed) | — | conformance.rs:984 binary-rejection leg. |
| `TestClient::send_raw` | 132-139 | REAL (executed) | — | The dirty-world injector: truncated header (506), unmasked frame (528), 4-GiB claim (547). |
| `TestClient::shutdown_write` | 141-155 | REAL (executed) | — | conformance.rs:509 EOF-mid-frame leg. |
| `TestClient::ping` | 157-170 | REAL (executed) | — | conformance.rs:628, 667. |
| `TestClient::close` | 172-184 | REAL | — | 2-byte code payload, §5.5.1-legal. |
| `TestClient::send_text_masked` | 186-199 | REAL (executed) | S4 (F-002) | Name lies; fragmentation legs use it with Continuation. |
| `TestClient::read_frame` | 201-208 | REAL (executed) | — | Unmasked-read strictness via read_client_side_frame. |
| `TestClient::expect_text` | 210-233 | REAL | — | Requires FIN+Text+exact payload; typed BadEnvelope/Closed otherwise. |
| `TestClient::expect_pong` | 235-247 | REAL (executed) | — | conformance.rs:642, 670. |
| `TestClient::expect_close` | 249-265 | REAL (executed) | — | Delegates payload check to frame::parse_close_payload; conformance close_code helper:191. |
| `TestClient::read_server_frame` | 267-287 | REAL (executed) | — | FIN+Text+UTF-8+serde_json; conformance next_frame:183. |
| `Frame` (struct) | 289-298 | REAL | — | Client-side mirror of frame::Frame. |
| `read_client_side_frame` | 300-311 | REAL (executed) | — | 1 MiB frame cap (1 << 20) hard-coded — matches test needs; server-side limit is server.rs's business. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| No tests in this file (client is the harness, not the subject) | — | The subject is the server; but a broken client fails every conformance leg loudly (connect/expect_* are on the assertion path of all 25+6 tests, all executed PASS in baseline). |
| Mutation check (reasoned): make `read_client_side_frame` accept masked frames | Conformance still passes (server never masks) — but that is the CLIENT being lax about a violation only a hostile server would make; out of the suite's scope, documented as "mirrored strictness". | No gap: the suite tests the server, not itself. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "in-tree WebSocket test client … driving the real server over a real loopback socket (doc 10: no synthetic echo chambers)" | client.rs:1-12 | REAL | 31 executed tests over real TCP loopback (baseline conformance 25 + e2e 6 PASS). |
| "a broken server must FAIL a test with a typed ProtocolError::TimedOut, never hang the suite" | client.rs:27-31 | REAL | Deadlines set at 56-67; WouldBlock/TimedOut mapped to ProtocolError::TimedOut in frame.rs:109-115, http.rs:129-131 (read). Not directly exercised by a baseline test (no hung-server leg) — REAL by construction, execution UNPROVEN. |
| "kept so tests can verify the accept computation was truly key-derived" | client.rs:36-38 | FALSE (in practice) | Zero `.key` readers (F-001). |
| "Not part of any production path" | client.rs:14-15 | REAL (execution) / misleading (linkage) | F-004. |

## 7. Dead / duplicated / unreachable code
- `TestClient.key` (38): dead pub field (F-001).
- Whole module: unreachable from shipped binaries by design (test infrastructure, executed only by test targets).

## 8. Dependencies & flags
std-only transport (TcpStream, Shutdown); serde_json for `read_server_frame`; no external WS/SHA-1/base64 crates (in-tree per Cargo.toml's ADR-011 note — verified Cargo.toml: deps = commands, serde, serde_json only). No feature gates. No `unsafe`/`unwrap`/`expect`/`panic`/`todo!` in the file (full read).

## 9. Open questions for the next agent
- Server auditor (W02-A0x, crates/control-protocol/src/server.rs): confirm the server's max-frame limit matches or exceeds the client's 1 MiB read cap (client.rs:304) — a server sending larger legit frames would deadlock tests at the typed error instead of hanging; also confirm no server-initiated Ping exists (rg says only the handler at server.rs:552) so the client's missing pong-send is safe forever.
- The "typed TimedOut, never hang" claim (client.rs:27-31) has no executed hung-server leg — a conformance test that never reads (server stalls) would pin it. REQUEST-ORCH-RUN not needed (test-authoring, not a run gap).
