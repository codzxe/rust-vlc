# AUDIT: crates/control-protocol/src/error.rs
| Field | Value |
|---|---|
| Auditor | W02-A08 |
| Date | 2026-11-23 |
| Lines / functions | 327 / 12 (2 enums, 4 methods, 2 Display, 1 Error, 1 From, 6 tests) |
| File verdict | REAL (used taxonomy) with 2 dead public methods whose doc claims are false |
| Reachable from shipped entry point? | YES from the `lumen-control-server` binary — apps/control-server/src/main.rs:21-31 → `serve_loop` (apps/control-server/src/run.rs:45) → `ControlServer::serve_one` → `fail_with`/`close_with` (server.rs:846-876) call `err.close_code()` (error.rs:146). NOTE: `lumen-control-server` is a workspace member built by CI (`cargo build --workspace --all-features`, ci.yml:233) but NOT in the release lane — release.yml:193 builds `-p lumen-cli -p desktop` only. Dev/local tooling, not shipped product. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `./target/debug/deps/control_protocol-30329beb10b168e9 error::` → 6 passed (binary mtime 1795457279 > source 1795455358, current); rg caller census for `to_command_error`/`is_recoverable` |

## 1. One-paragraph verdict
The taxonomy itself is real and load-bearing: every variant is constructed somewhere in the crate's server/frame/http/client code, close codes are RFC-6455-correct and test-pinned (6/6 ran), redaction discipline holds (`Unauthorized` carries no token material; `UnknownProtocol` Display pipes through `redact_name`'s 64-char cap). But the file's two bridge methods are dead code with false documentation: `to_command_error()` — the module doc's headline claim that it projects onto the command layer "so hosts answer with the same vocabulary every other host uses" — has zero callers in the tree; the server instead hand-rolls its own `ProtocolError → ProtocolErrorCode` mapping in `fail_with` (server.rs:854-863), and the control-server app surfaces errors via plain `Display` (run.rs:61). `is_recoverable()` is also dead, and its model (only `BadEnvelope` is recoverable) describes a variant the server never constructs — the server's actual recoverable errors (bad-json/invalid-request/unknown-type) are a different enum entirely.

## 2. What actually works (verified)
— `./target/debug/deps/control_protocol-30329beb10b168e9 error::` → `test result: ok. 6 passed` (close-code values, auth→permission-denied, oversized→resource-limit, version-mismatch→4402, timeout→1001/network, recoverability set).
— Caller census (rg): all 14 `ProtocolError` variants constructed in production code (frame.rs, http.rs, server.rs, client.rs); `close_code()` called at server.rs:535/540/866; `Display` used in every `error_frame`/log path.
— Redaction: `Unauthorized { locked_out: bool }` carries no payload (L120-124); `UnknownProtocol` Display routes `offered` through `crate::protocol::redact_name` (L222-227), capped at 64 chars + ellipsis (protocol.rs:265-270).

## 3. Findings
F-001 [S3]
Where: crates/control-protocol/src/error.rs:4-7 and 173-192
Code:
```
//! stringly error, never a panic. [`ProtocolError::to_command_error`]
//! projects the taxonomy onto the ONE command layer's `CommandError`
//! (stage [`Stage::Control`], doc 05 §2.13) so hosts answer with the
//! same vocabulary every other host uses.
```
What is wrong: `to_command_error` has ZERO callers outside this file's own tests (rg over the whole tree: 4 hits, all in error.rs — doc comment L4, definition L173, tests L278/290/311). No host answers with it. The actual server path is `fail_with` (server.rs:854-867), which hand-maps `ProtocolError` onto the OTHER taxonomy `ProtocolErrorCode` with a catch-all `_ => ProtocolErrorCode::BadFrame`; the control-server app then prints `source.to_string()` into a log record and exits (apps/control-server/src/run.rs:58-64). The claimed §2.13 bridge to `CommandError` is unwired scaffolding. Why it matters: the doc sells vocabulary unification ("the same vocabulary every other host uses") that does not exist at runtime; anyone relying on the projection gets `Display` strings instead. What correct looks like: either delete the method and the claim, or route `fail_with`'s error frame and the app's exit path through it. Confidence: HIGH — Verification: rg + read.
F-002 [S3]
Where: crates/control-protocol/src/error.rs:164-168
Code:
```
    /// Whether the connection can survive this error (a typed
    /// application-level error frame, no close).
    pub fn is_recoverable(&self) -> bool {
        matches!(self, ProtocolError::BadEnvelope { .. })
    }
```
What is wrong: Dead (only caller: its own test at L317-325) AND wrong if ever wired: the server never constructs `ProtocolError::BadEnvelope` — rg shows all 6 construction sites in client.rs (L223-283), and the client is documented test-support (lib.rs:32-34 "the in-tree WS conformance client used by this crate's and the app's tests"). The server's genuine recoverable failures — bad JSON, envelope shape, unknown frame type — are sent via the separate `recoverable()` helper (server.rs:813-825) using `ProtocolErrorCode::{BadJson, InvalidRequest, UnknownType}`, none of which this enum can express. Wiring `is_recoverable()` into the server as-is would turn every currently-survivable protocol error into a connection close. Confidence: HIGH — Verification: rg + read.
F-003 [S4]
Where: crates/control-protocol/src/error.rs:3-4 vs crates/control-protocol/src/protocol.rs:49-75
Code:
```
//! Every failure mode of the control plane is a variant here — never a
//! stringly error, never a panic.
```
What is wrong: The control plane's failure modes live in TWO enums: this file's `ProtocolError` (14 connection-level variants) and protocol.rs's `ProtocolErrorCode` (12 wire codes: not-hello, hello-twice, bad-json, unknown-type, bad-frame, version-mismatch, unknown-protocol, unauthenticated, non-monotonic-request-id, invalid-request, denied, internal). The claim "every failure mode … is a variant here" is false by omission, and the hand-translation in `fail_with` flattens Io/Frame/MessageTooLarge/InvalidUtf8/Closed into `bad-frame` via the `_ =>` catch-all (server.rs:862), losing machine-readable precision (a 1009-too-big close arrives wearing code `bad-frame`). Confidence: HIGH — Verification: read both enums + the catch-all.
F-004 [S4]
Where: crates/control-protocol/src/error.rs:133-139 vs server.rs:861
Code:
```
    /// A socket read/write deadline expired (the bounded-wait
    /// guarantee: an idle or stalled peer closes the session with
    /// 1001 instead of wedging the single-connection server forever).
    TimedOut {
```
What is wrong: The machine code the peer receives is `internal` ("Server-side trouble (never leaks internals)", protocol.rs:73-74) even though the failure is a normal idle disconnect — `ProtocolError::TimedOut { .. } => ProtocolErrorCode::Internal` (server.rs:861). The human message stays honest ("socket deadline expired …") and the close code is 1001, so a client keying on the code sees a lie, a client keying on the close code sees the truth. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum CloseCode` (8) | 19-38 | REAL | — | RFC 6455 + doc-05 4400-range; values test-pinned (ran). |
| `CloseCode::as_u16` | 42-53 | REAL | — | Const, correct values. |
| `Display for CloseCode` | 56-60 | REAL | — | Prints the numeric code. |
| `enum ProtocolError` (14) | 64-142 | REAL | — | All variants constructed in crate production code (rg census). |
| `ProtocolError::close_code` | 146-162 | REAL | — | Called at server.rs:535/540/866. `Normal` for Io/Closed/NonLoopbackBind is benign (no connection to close). |
| `ProtocolError::is_recoverable` | 166-168 | UNREACHABLE | S3 | Dead + model contradicts server behavior (F-002). |
| `ProtocolError::to_command_error` | 173-192 | UNREACHABLE | S3 | Dead; doc claim false (F-001). Mapping itself is sensible (Io→io, MessageTooLarge→resource-limit, Unauthorized→permission-denied, Closed→canceled). |
| `Display for ProtocolError` | 195-245 | REAL | — | Redaction holds: token never printed; `offered` via redact_name. |
| `impl Error` | 247 | REAL | — | Trivial. |
| `From<io::Error>` | 249-256 | REAL | — | Blanket `what: "control-protocol i/o"`; used by `?` in server/client I/O paths. |
| tests (6) | 262-327 | REAL | — | All pass (ran); value-pinning style, would fail on any mapping change. |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| close_codes_are_the_documented_values | YES | Exact u16 assertions for all 8 codes. |
| auth_errors_map_to_permission_denied | YES | Pins code + stage + summary string. |
| oversized_maps_to_resource_limit | YES | Pins both ErrorCode and CloseCode. |
| version_mismatch_maps_to_close_4402 | YES (weakly) | Code pinned; message check is `contains("mutually supported major")`. |
| timeouts_map_to_going_away_and_network | YES | Pins close code 1001 + ErrorCode::Network. |
| only_bad_envelope_is_recoverable | YES | Asserts the predicate — but the predicate itself is dead code, so the test pins a dead API (still honest about what the fn does). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`to_command_error` projects the taxonomy … so hosts answer with the same vocabulary every other host uses" | error.rs:4-7 | FALSE | Zero callers outside own tests (rg); server hand-maps to ProtocolErrorCode instead (server.rs:854-863). |
| "Every failure mode of the control plane is a variant here" | error.rs:3-4 | FALSE (by omission) | 12 wire codes live in protocol.rs:49-75 (F-003). |
| "token material NEVER enters an error message — the auth variants carry no payload beyond the fact of failure" | error.rs:9-10 | REAL | `Unauthorized { locked_out: bool }` (L120-124) has no token field; Display prints only "token rejected…". |
| "The inner std error is preserved for the log sink; `Display` stays redaction-safe (no paths beyond the operation name)" | error.rs:71-73 | SHALLOW | Display embeds `{source}` (L202); for this server's TCP operations the io::Errors are path-free, but nothing enforces it — an io::Error carrying a path would print it. |
| close codes 4400/4401/4402 per doc 05 §5.4 | error.rs:31-37 + test | REAL | Test-pinned (ran). |

## 7. Dead / duplicated / unreachable code
- `is_recoverable` — dead method (F-002).
- `to_command_error` — dead method (F-001).
- Recoverability decision is duplicated as call-site choice (`recoverable()` vs `fail_with()` in server.rs) instead of living in the taxonomy — the enum predicate and the behavior cannot agree because they describe different error sets.
- `ProtocolError::BadEnvelope` — constructible only from the test client (client.rs); no server path produces it.

## 8. Dependencies & flags
- Depends on `commands` (CommandError/ErrorCode/Stage) — the dead bridge method is the only use of that dependency surface in this file; the rest of the crate also uses commands types for WireRequest/Response.
- No unsafe, no allow attrs, no feature gates; part of CI workspace build (ci.yml:233-234).

## 9. Open questions for the next agent
- Does any doc/matrix row claim the control server is a shipped surface? The release lane (release.yml:193) ships only lumen-cli + desktop — if the matrix claims remote control as a product feature, that is an S0-grade claim gap (check feature-matrix rows for "control server"/"remote").
- `From<io::Error>` uses a fixed `what`; server code uses explicit `Io { what, source }` at most sites — confirm no `?`-conversion path accidentally discards context (client.rs is the likely user).
