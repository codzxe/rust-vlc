# AUDIT: crates/control-protocol/src/server.rs#part2 (lines 494-987)
| Field | Value |
|---|---|
| Auditor | W02-A09 |
| Date | 2026-11-21 |
| Chunk boundaries | part2 = lines 494-987: `impl Session` (struct declared 477-492, boundary item), `err_reason`, `enum Flow`, in-file tests. Part1 (lines 1-493: Limits/AuthLimits/Timeouts/FailureBook/ServerConfig/ControlServer::bind/serve_one) is W02-A07's assignment. |
| Lines / functions | 494 / 17 (Session struct + 11 methods + err_reason + Flow + 4 tests + 1 helper) |
| File verdict | REAL (request/handshake half) — the event-subscription half is SHALLOW bookkeeping with no egress (F-001) |
| Reachable from shipped entry point? | YES via the `lumen-control-server` binary (`apps/control-server/Cargo.toml:37 [[bin]] name = "lumen-control-server"`; crate `control-protocol` is its dependency; the binary compiled in baseline `cargo build --release --locked --workspace` → exit 0, INDEX.md #1). Not linked into the `lumen` CLI (rg `control_protocol` in apps → only apps/control-server). |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 2 |
| Commands run | `sed -n` reads of server.rs 1-987 (full file read for context); `rg "ServerFrame::Event" *.rs` repo-wide → ZERO constructors; `rg "subscribe"` conformance.rs; `sed -n '1464,1494p' audit/runs/baseline/test-workspace.log.txt` (25/25 conformance ok); `sed -n '194,199p' protocol.rs` (ServerFrame::Event defined); `rg "D-1\|D-3" docs/engineering-directives.md`; `sed -n '372,392p'/'561,594p'/'661,694p'/'976,1054p' conformance.rs` (pre-auth, garbage, ping, subscribe tests); `rg "event" apps/control-server/src` (no event pump declared) |

## 1. One-paragraph verdict
The session state machine in this chunk is genuinely engineered and genuinely tested: handshake, version negotiation, constant-time token check with a global lockout book, request-envelope validation on the RAW value (a real fix for a real flatten bug), anti-replay monotonicity, deny-by-default policy gate, ping/pong/close per RFC 6455, bounded waits — 25/25 conformance tests green in the baseline run. But the chunk carries one silent, undeclared hole at its heart: the protocol's event push does not exist. `subscribe`/`unsubscribe` are accepted and bookkept into a `Vec<String>` that nothing ever reads, `ServerFrame::Event` is defined in protocol.rs and constructed NOWHERE in the repository, and the `Dispatch` seam returns one response per request with no event channel — so over this transport a class-A command's `Accepted` can never be followed by its §1.4 C3 completion evidence. The conformance test even papering over it with a false architectural claim ("the event push is the host engine's business" — the host engine has no path to the session stream). Secondary: the subscription Vec's "Bounded" comment is mathematically wrong across frames, unknown-field rejection only covers request frames, and an unauthenticated peer can occupy the single-connection slot forever with garbage/pings while the module doc claims "frames before authentication are refused".

## 2. What actually works (verified)
— 25/25 conformance tests passed in baseline: `audit/runs/baseline/test-workspace.log.txt:1464-1493` (`test result: ok. 25 passed; 0 failed`), including `handshake_success_full_session_round_trip`, `wrong_token_closes_4401_and_counts`, `five_failures_lock_out_even_the_right_token`, `non_monotonic_request_id_closes_the_connection`, `sensitive_command_denied_by_default_and_connection_survives`, `unauthenticated_read_only_mode_serves_the_safe_subset_only`, `request_validation_rejects_dirty_payloads_and_survives`, `binary_application_frames_are_rejected_but_survive`, `invalid_utf8_text_closes_1007`, `idle_session_is_closed_by_the_read_deadline_1001`.
— Raw-value envelope validation before typed parse (handle_text 634-647) closes a real silently-dropped-unknown-field bug the comment says the conformance suite found (`"extra":true` once dispatched); pinned by `request_validation_rejects_dirty_payloads_and_survives` (baseline ok).
— Lockout is checked BEFORE the token comparison (handle_hello 716-719), so a locked-out attacker with the RIGHT token is still refused — pinned by `five_failures_lock_out_even_the_right_token` (baseline ok).
— app-level e2e through the real binary stack: `apps/control-server/tests/e2e.rs` `e2e_full_session_through_the_app_stack` (welcome → list-commands 65 rows → granted sensitive op reaches dispatch); ran in baseline workspace suite.

## 3. Findings

### F-001 [S2]
Where: crates/control-protocol/src/server.rs:651-668 (and 491, 245-249; protocol.rs:194-199)
Code:
```rust
            Ok(ClientFrame::Subscribe { events }) => {
                for kind in events {
                    if !self.subscriptions.contains(&kind) {
                        // Bounded: the message cap bounds the union of
                        // kinds (each kind costs ≥ its JSON bytes).
                        self.subscriptions.push(kind);
                    }
                }
                Flow::Continue
            }
```
What is wrong: The subscription machinery is bookkeeping for a feature that does not exist. `ServerFrame::Event` is defined in protocol.rs (lines 194-199, doc comment "An engine event push") but constructed nowhere in the repository (`rg "ServerFrame::Event" --type rust` → zero matches, tests included). `Session::subscriptions` (line 491) is written by subscribe/unsubscribe and read by nothing. The `Dispatch` trait (lines 245-249: `fn dispatch(&mut self, request: WireRequest) -> WireResponse`) has no event channel, and `Session` owns the only stream handle — the host cannot push events even if it wanted to. Doc 05 §5.4 defines the envelope with `"event"` frame types; lib.rs's "Honest scope (card 7.J1)" declares only rate caps and the Origin allowlist as gaps — this one is silent.
Why it matters: Over this transport, class-A commands (`session.open`, `playback.seek`, …) answer `Accepted` and then silence — the §1.4 C3 completion-evidence contract (state-changed/seek-completed/error events) is unenforceable; any client implementing the documented `waitFor("seek-completed")` flow (doc 05 §4.1) hangs forever. The conformance test's comment (conformance.rs:993-994) claims "the event push is the host engine's business" — false: the host engine has no seam to the session stream.
What correct looks like: Either an event pump (host-side event queue drained into `ServerFrame::Event` between requests, honoring the subscription filter and §4.3 ordering) or subscribe/unsubscribe answered with a typed "not supported" error and the gap recorded in lib.rs's honest-scope list.
Confidence: HIGH — Verification: read + repo-wide rg (zero constructors) + baseline conformance run (no event test exists).

### F-002 [S2]
Where: crates/control-protocol/src/server.rs:652-657
Code:
```rust
                    if !self.subscriptions.contains(&kind) {
                        // Bounded: the message cap bounds the union of
                        // kinds (each kind costs ≥ its JSON bytes).
                        self.subscriptions.push(kind);
                    }
```
What is wrong: The "Bounded" claim is false across frames. The message cap (1 MiB) bounds ONE Subscribe frame; nothing bounds the accumulated union over UNLIMITED frames. A client (authenticated, or unauthenticated in read-only mode) can send N distinct-kind Subscribe frames and grow `subscriptions` without limit (each frame adds up to ~1 MiB of distinct kind strings; dedup only prevents re-adding the same string).
Why it matters: Peer-controlled unbounded memory growth in a long-lived connection, contradicting D-1 memory discipline (which this very file cites at line 72: "the claim is rejected BEFORE any allocation (D-1)") and the pattern this crate follows everywhere else (FailureBook saturating counters, byte caps, bounded waits). Today the growth is also pointless — the Vec is dead state (F-001) — which makes it worse: unbounded allocation for bookkeeping nothing reads.
What correct looks like: A cap on `subscriptions.len()` (drop-oldest or reject-with-error when exceeded), or delete the Vec until events actually ship.
Confidence: HIGH — Verification: read; no test sends a second Subscribe frame (conformance.rs sends exactly one).

### F-003 [S3]
Where: crates/control-protocol/src/server.rs:640-647
Code:
```rust
        if let Some(err) = value
            .get("type")
            .and_then(serde_json::Value::as_str)
            .filter(|kind| *kind == "request")
            .and_then(|_| validate_request_shape(&value).err())
        {
            return self.recoverable(stream, ProtocolErrorCode::InvalidRequest, &err.message);
        }
```
What is wrong: Raw-value unknown-field validation runs ONLY when `type == "request"`. `hello`, `subscribe`, and `unsubscribe` frames are parsed by tagged enums without `deny_unknown_fields`, so unknown fields in those envelopes are silently dropped — exactly the class of bug the comment (lines 634-639) says was found by the conformance suite for requests. Doc 05 §5.4 requires "unknown fields/IDs rejected" for every frame, not just requests.
Why it matters: A control client with a typo'd subscribe field (e.g. `"event"` instead of `"events"`) gets silent acceptance instead of a typed error — the same silent-drop failure mode the code explicitly patched for requests, left open one enum branch over. (With `events` missing, deserialization errors — good — but extra unknown keys vanish.)
What correct looks like: The same raw-value key-allowlist check for the hello/subscribe/unsubscribe envelopes (REQUEST_KEYS-style HELLO_KEYS/SUBSCRIBE_KEYS in protocol.rs).
Confidence: HIGH — Verification: read of ClientFrame (protocol.rs:156-185, no deny_unknown_fields possible under `#[serde(tag)]`).

### F-004 [S3]
Where: crates/control-protocol/src/server.rs:618-633 (and 551-557; module doc 21-22)
Code:
```rust
        let value: serde_json::Value = match serde_json::from_str(text) {
            Ok(value) => value,
            Err(err) => {
                return self.recoverable(
                    stream,
                    ProtocolErrorCode::BadJson,
```
What is wrong: The module doc (part1, lines 21-22) claims "frames before authentication are refused", but: (a) pre-hello garbage TEXT frames are RECOVERABLE (`recoverable` → BadJson → `Flow::Continue`, lines 618-630; the connection stays un-established and open — pinned as intended behavior by `garbage_masked_payload_is_a_recoverable_json_error`, baseline ok); (b) the Ping arm (lines 551-557) answers pongs with NO `established`/`authenticated` check. lib.rs states "first frame must be `hello`". Consequence: an unauthenticated local peer can hold the single-connection server's only slot indefinitely by sending any byte sequence below the 60 s read deadline (garbage or pings) — a second legitimate client can never connect.
Why it matters: The single-connection design is declared and defensible for a local single-user product; what is not declared is that the single slot is occupiable forever WITHOUT authentication, while the docs claim pre-auth frames are refused. Availability of the control surface is one garbage-frame-per-minute away from permanent denial for everyone else.
What correct looks like: A pre-establishment frame budget (N recoverable errors or M seconds un-established ⇒ typed close), pings included; and a module doc that states the real rule.
Confidence: HIGH — Verification: read (code path clear: opcode arms before `handle_text`); conformance test pins the recoverable-garbage half.

### F-005 [S4]
Where: crates/control-protocol/src/server.rs:861
Code:
```rust
            ProtocolError::TimedOut { .. } => ProtocolErrorCode::Internal,
```
What is wrong: A read/write-deadline expiry is reported to the peer as `ProtocolErrorCode::Internal` ("internal") while the message text says "deadline" (`idle_session…` conformance test asserts both). No timeout variant exists in `ProtocolErrorCode`.
Why it matters: Machine-stable codes exist so clients can branch on them; a client distinguishing "server bug" (internal) from "peer stalled" (timeout) cannot — the code lies about the class of failure.
What correct looks like: A `ProtocolErrorCode::TimedOut` variant (additive, kebab-cased "timed-out") mapped from `ProtocolError::TimedOut`.
Confidence: HIGH — Verification: read of both enums; conformance test pins current behavior.

### F-006 [S4]
Where: crates/control-protocol/src/server.rs:526-531
Code:
```rust
                let status = match &err {
                    ProtocolError::HttpUpgrade {
                        reason: "request-head-too-large",
                    } => 431,
                    _ => 400,
                };
```
What is wrong: HTTP status selection matches on the error's human-readable `reason` STRING owned by another module (crate::http, A08's file). If the reason literal in http.rs changes, oversized heads silently degrade to 400 with no compile error and no test failure (no test pins the 431 path — conformance's `handshake_failure_non_upgrade_http_gets_an_http_error` checks a different path).
Why it matters: Cross-module contract carried by an untyped string; classic silent-drift seam.
What correct looks like: A typed variant or flag on `ProtocolError::HttpUpgrade` (e.g. `head_too_large: bool`) matched here.
Confidence: MEDIUM (drift is hypothetical until http.rs changes) — Verification: read; `rg "431"` finds no test.

## 4. Per-function table (part2: lines 494-987)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct Session` (boundary: declared 477-492) | 477-492 | REAL | — | 12 fields incl. dead `subscriptions` (F-001/F-002) |
| `Session::new` | 495-512 | REAL | — | Clones policy/engine_info/token per session (bounded, not hot) |
| `Session::run` | 515-608 | REAL | — | Frame loop; upgrade error → HTTP 431/400 (F-006 string match); Ping answered pre-auth (F-004); Close echoed per RFC 6455 §5.5.1; binary message → recoverable BadFrame; invalid UTF-8 → close 1007 |
| `Session::handle_text` | 611-677 | REAL | S2/S3 | Raw-JSON parse; pre-hello garbage recoverable (F-004); request-only shape validation (F-003); subscribe/unsubscribe bookkeeping into dead state (F-001, F-002) |
| `Session::handle_hello` | 680-758 | REAL | — | protocol/version checks; lockout checked BEFORE token compare (verified good); read-only unauthenticated path gated on `policy.read_only()`; welcome with honest capabilities |
| `Session::handle_request` | 763-810 | REAL | — | Monotonicity breach → error + close AppProtocolError; policy deny → recoverable Denied (connection survives); dispatch; response. First-request id 0 accepted (doc says ids start at 1 — unenforced minimum, cosmetic) |
| `Session::recoverable` | 813-825 | REAL | — | protocol_errors saturating++; connection survives |
| `Session::error_frame` | 816-833 | REAL | — | `redact_name` applied to every message (V8) |
| `Session::send_json` | 836-842 | REAL | — | serde error wrapped as Io; single write_frame |
| `Session::fail_session` | 846-851 | REAL | — | `unreachable!` arm is sound: `fail_with` provably always returns `Flow::End` |
| `Session::fail_with` | 854-867 | REAL | S4 | TimedOut→Internal mapping (F-005) |
| `Session::close_with` | 870-875 | REAL | — | close payload + TCP shutdown; best-effort writes on the way out (`let _ =`) — acceptable on terminal paths |
| `err_reason` | 879-891 | REAL | — | Static redaction-safe strings; literal markers for lint-literals regime |
| `enum Flow` | 894-897 | REAL | — | Two-state control flow |
| test `non_loopback_binds_are_refused_at_construction` | 903-915 | REAL | — | Covers part1 `bind`; 6 bad hosts incl. DNS-rebind-shaped `example.com` |
| test `loopback_binds_succeed_on_both_families` | 917-933 | REAL | — | Covers part1 `bind`; both families + localhost |
| test `failure_book_engages_the_documented_lockout` | 935-959 | REAL | — | Covers part1 `FailureBook`; 3-failure config, expiry sleep 60 ms, re-lock after expiry |
| test `documented_limits_are_the_doc_05_numbers` | 961-968 | REAL | — | Pins 1 MiB/1 MiB/5/60 s against doc 05 §5.4 |
| helper `test_engine_info` | 970-986 | REAL | — | Uses live `catalog_checksum()` — no fixture drift possible |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| in-file 4 tests (903-968) | YES | Direct behavioral asserts on bind/failure-book/limits; mutation: swap lockout to per-connection reset → `five_failures…` style asserts fail. Note: all four target PART1 constructs; part2 has no in-file unit tests — coverage is external (conformance.rs), which is acceptable but lopsided. |
| conformance: handshake_success_full_session_round_trip | YES | Full wire session incl. welcome assertions (version, capabilities) |
| conformance: frames_before_authentication_are_refused | YES | Pre-hello REQUEST → NotHello + 4400 + requests_served==0; a regression that dispatches pre-auth fails it |
| conformance: garbage_masked_payload_is_a_recoverable_json_error | YES (pins the defect too) | Pins recoverable-BadJson as INTENDED — this is the evidence that F-004's behavior is deliberate, not accidental |
| conformance: binary_application_frames_are_rejected_but_survive | PARTIAL | Binary rejection real; the subscribe half (lines 993-1005) only proves the connection survives a subscribe — it asserts NOTHING about subscription state (none is observable) and its comment makes the false "host engine's business" claim (F-001) |
| conformance: idle_session_is_closed_by_the_read_deadline_1001 | YES | Silent-but-established client gets Internal+"deadline" then 1001; summary asserts ServerClosed(GoingAway) |
| e2e: e2e_full_session_through_the_app_stack | YES | Real bind → serve_loop thread → real client → welcome/list-commands(65)/granted-op; pins the app wiring |
| NO test anywhere | — | Sends a second Subscribe frame (F-002 unbounded growth untested); constructs `ServerFrame::Event` (F-001 untested because unimplemented); pins the 431 head-too-large path (F-006) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| Envelope includes `"event"`/`"subscribe"`/`"unsubscribe"` frame types | docs/05-command-layer.md §5.4 | SHALLOW | Types defined (protocol.rs:156-199); subscribe accepted; `ServerFrame::Event` has zero producers repo-wide (F-001) |
| "unknown fields/IDs rejected" (every frame) | docs/05-command-layer.md §5.4 | FALSE (non-request frames) | F-003: raw-value check gated on `type=="request"` only |
| "frames before authentication are refused" | server.rs:21-22 (module doc) | FALSE | Garbage pre-hello is recoverable (tested as intended); pings answered pre-auth (code path, no established check) (F-004) |
| "first frame must be `hello` naming the protocol" | lib.rs:17-18 | FALSE (for garbage) | `garbage_masked_payload_is_a_recoverable_json_error` — first frame garbage, session continues |
| "5 failures ⇒ 60 s lockout" | server.rs:21-22, doc 05 §5.4 | REAL | `five_failures_lock_out_even_the_right_token` baseline ok; lockout pre-checks token compare |
| "requestId monotonicity enforced per connection (anti-replay); a breach closes the connection" | doc 05 §5.4 | REAL | `non_monotonic_request_id_closes_the_connection` baseline ok; code 771-782 |
| "max 1 MiB" caps | doc 05 §5.4 | REAL | `documented_limits_are_the_doc_05_numbers` + `oversized_length_claim_closes_1009_before_allocation` baseline ok |
| read-only mode "admits unauthenticated clients to a fixed safe subset" | doc 05 §5.4 | REAL | `unauthenticated_read_only_mode_serves_the_safe_subset_only` baseline ok (policy enforcement verified in policy.rs audit, next file) |
| rate limit 100 req/s burst 200 / 8 in-flight / Origin allowlist | doc 05 §5.4 `[PROPOSAL]` | PAPER-ONLY (declared gap) | Not implemented; lib.rs "Honest scope" declares it a recorded gap for 8.A2 — honest |

## 7. Dead / duplicated / unreachable code
- `Session::subscriptions` (491) + the Subscribe/Unsubscribe arms (651-668): dead state — written, never read (F-001). The whole subscription feature is unreachable end-to-end: no event egress exists.
- `ServerFrame::Event` (protocol.rs:194-199): dead wire vocabulary, zero constructors repo-wide.
- Not unreachable: the request/handshake half ships in `lumen-control-server` (baseline build #1).

## 8. Dependencies & flags
- Uses only in-crate modules + `commands` wire types + std net — consistent with the crate's zero-deps in-tree philosophy (lib.rs). No feature flags. No unsafe/unwrap/expect in this chunk (workspace lints; verified by reading — the only `unreachable!` is the proven-sound fail_session arm and test unreachable!s).

## 9. Open questions for the next agent
- policy.rs (W02-A09, next): confirm `READ_ONLY_SAFE` matches doc 05 §5.4's named subset (`media.get-info`, `system.get-engine-info`, `tracks.list`, snapshot) and that `session.open` stays privileged.
- http.rs auditor (A08): does `read_upgrade_request` emit the exact `"request-head-too-large"` reason string this file matches (F-006)?
- Whoever owns card 8.A2: the event-push gap (F-001) is NOT in the recorded-gaps list — it must be added or implemented.
