# AUDIT: crates/control-protocol/src/server.rs
| Field | Value |
|---|---|
| Auditor | W02-A05 |
| Date | 2026-11-23 |
| Lines / functions | 987 / 33 (28 fns/methods, 4 unit tests, 1 test helper) |
| File verdict | REAL (crate real + executed; NOT reachable from shipped release lane) |
| Reachable from shipped entry point? | NO — only `apps/control-server` depends on this crate; release lane builds `-p lumen-cli -p desktop` (+ desktop-tauri) only; product.toml `control-server = false` |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 4 |
| Commands run | `./target/debug/deps/conformance-fc3f1f30343cb626` -> 25 passed, exit 0; `./target/debug/deps/control_protocol-30329beb10b168e9` -> 47 passed, exit 0; rg/sed cross-refs (see below) |

## 1. One-paragraph verdict
This is a REAL in-tree RFC 6455 control server, not a mock seam: `TcpListener::bind` (L348), `listener.accept()` (L441), a hand-written HTTP-upgrade/frame stack (`crate::http`, `crate::frame`), per-start 256-bit token from `/dev/urandom` with constant-time compare (L720, token.rs:45-53), deny-by-default policy, and a 25-case conformance suite over real loopback sockets — which I executed (25/25 pass, exit 0; plus all 47 crate unit tests, exit 0). It is deliberately NON-concurrent: strictly one connection at a time (`serve_one` blocks; the host app's `serve_loop` repeats serially, apps/control-server/src/run.rs:54), and the crate's own docs record the missing rate caps / Origin allowlist as deferred gaps (lib.rs "Honest scope"). The hostile answer: sockets real, concurrency absent by design, and the whole thing ships nowhere today — only `apps/control-server` uses it, that binary is not in the release lane (release.yml:193 builds `-p lumen-cli -p desktop`), product.toml:167 gates `control-server = false`, and the app itself refuses to start under the repo product.toml (config.rs:233-254 test `repo_product_today_refuses_the_feature_gate`). The host dispatch behind the server implements only 2 of ~65 commands (`engine.info`, `system.list-commands`); everything else returns typed `NotReady` (apps/control-server/src/dispatch.rs:54-69) — so even if it ran, the server could not control playback.

## 2. What actually works (verified)
— full conformance suite (real sockets, loopback, per-case ephemeral ports): `./target/debug/deps/conformance-fc3f1f30343cb626` -> "test result: ok. 25 passed", exit 0 (covers ping/pong, wrong-token 4401, 5-failure lockout, non-monotonic id close, DNS-rebind refusal, oversize-claim 1009, read-only subset, idle-deadline 1001).
— full unit suite: `./target/debug/deps/control_protocol-30329beb10b168e9` -> "47 passed", exit 0.
— Orchestrator baseline corroborates: audit/runs/baseline/cargo-test-workspace.txt lines 1552-1697 (control-protocol 47+25 ok, exit_code=0; control-server 11+6 ok, exit_code=0).

## 3. Findings
F-001 [S2]
Where: server.rs:651-667 (+491, 653-656)
Code:
```
Ok(ClientFrame::Subscribe { events } => {
    for kind in events {
        if !self.subscriptions.contains(&kind) {
            // Bounded: the message cap bounds the union of
            // kinds (each kind costs ≥ its JSON bytes).
            self.subscriptions.push(kind);
```
What is wrong: the "Bounded" claim is false across a session. The 1 MiB message cap bounds each message's new kinds, not the UNION over an unbounded message stream; `subscriptions` grows without limit for the connection's lifetime. In read-only mode this is reachable by an UNAUTHENTICATED peer (read-only hello sets `authenticated=false`, L739, and Subscribe needs no auth), i.e. unbounded memory growth driven purely by wire input on a multi-user machine. Why it matters: textbook unbounded-alloc-from-wire-input; the in-code safety argument is wrong. What correct looks like: cap `subscriptions.len()` (reject beyond N) or gate Subscribe on authentication. Confidence: HIGH — Verification: read (rg confirms `subscriptions` is only ever pushed/retained/cleared, never drained into any output; no `ServerFrame::Event` construction exists anywhere: `rg "ServerFrame::Event"` -> 0 hits).

F-002 [S3]
Where: server.rs:651-667, 196-199 of protocol.rs
Code:
```
Ok(ClientFrame::Unsubscribe { events } => {
    if events.is_empty() {
        self.subscriptions.clear();
```
What is wrong: Subscribe/Unsubscribe are accepted and then DO nothing observable — no ack frame, no error, and no event can ever arrive: `ServerFrame::Event` (protocol.rs:196-199) is defined but never constructed anywhere in the repo (`rg "ServerFrame::Event"` -> only the definition site). Doc 05 §4.4 (L450) specifies subscribe topics (`area.*`, `area.>`) — no syntax validation, no filtering exists. Why it matters: a wire-level feature that silently no-ops; a client that subscribes gets the connection kept alive and zero events, with no signal. What correct looks like: either a typed not-supported error, or the event push path. Mitigation: doc 05 marks subscriptions `[PROPOSAL]`. Confidence: HIGH — Verification: read + rg.

F-003 [S3]
Where: server.rs:445-446 (claim), 448-459 (mechanism)
Code:
```
// Bounded waits on both directions BEFORE any session work: a
// silent peer can never wedge the (single-connection) server.
stream
    .set_read_timeout(timeouts.read)
```
What is wrong: the deadlines are per socket OPERATION, not per session: any peer that sends ≥1 byte every <60 s (e.g. a 1-byte ping fragment, or 1 MiB pings) resets the read deadline forever and holds the strictly serial server (one connection at a time; next `accept` happens only after this session ends, run.rs:54) — excluding every other client indefinitely. The comment is true for a *silent* peer and false for a *slow* peer. Why it matters: trivial slow-loris service denial; "bounded by design (D-1)" does not bound session duration. What correct looks like: a session-total deadline (e.g. `Instant` budget in `Session::run`) or concurrent accept. Mitigation: loopback-only, app not shipped (product gate off), and any local process has easier local-DoS vectors; doc 05 §5.4 defers rate caps `[PROPOSAL]`. Confidence: HIGH — Verification: read; conformance only proves the silent-peer case (`idle_session_is_closed_by_the_read_deadline_1001` passed).

F-004 [S3]
Where: server.rs:786-794
Code:
```
None => Err(commands::CommandError::new(
    commands::error::Stage::Control,
    commands::error::ErrorCode::Internal,
    "validated request carries no catalog id",
)),
```
What is wrong: an internal invariant breach ("Internal" error code) is surfaced to the peer as `ProtocolErrorCode::Denied` with `ops_denied` incremented (L795-797) — an internal bug would be miscounted/misreported as a policy denial. Why it matters: violates the crate's own typed-error discipline; obscures invariant breaches in logs. What correct looks like: a distinct protocol error code / internal close. Confidence: MEDIUM — Verification: read.

F-005 [S4]
Where: server.rs:721-722
Code:
```
let engaged = failures.note_failure(&mut self.auth_limits);
let _ = engaged; // the summary carries the state
```
What is wrong: `note_failure`'s only return-value consumer discards it; the "fresh lockout" signal is dead API surface. Confidence: HIGH — Verification: read (rg: `note_failure` called only here and in tests).

F-006 [S4]
Where: server.rs:526-531
Code:
```
ProtocolError::HttpUpgrade {
    reason: "request-head-too-large",
} => 431,
```
What is wrong: HTTP status selection by string-matching an enum payload field — a reason-string rename silently downgrades 431 to 400; stringly-typed control flow. Confidence: HIGH — Verification: read.

F-007 [S4]
Where: server.rs:846-851
Code:
```
match self.fail_with(stream, err) {
    Flow::End(end) => end,
    Flow::Continue => unreachable!("fail_with always ends the session"),
```
What is wrong: `unreachable!` (one of the tree's 4126, audit/runs/baseline/loc-patterns.txt); locally sound today because `fail_with` unconditionally returns `Flow::End`, but it is a panic path inside a server whose crate docs claim "no unsafe/unwrap/expect/panic". A `debug_assert!` + fallback close would uphold the stated invariant. Confidence: HIGH — Verification: read.

F-008 [S4]
Where: server.rs:525-535
Code:
```
let bytes = http::error_response(status, "Bad Request");
let _ = stream.write_all(&bytes);
let _ = stream.shutdown(std::net::Shutdown::Both);
return SessionEnd::ServerClosed(err.close_code());
```
What is wrong: a pre-WebSocket HTTP failure is recorded in the summary as `ServerClosed(<WS close code>)` — no WS close frame was ever sent (the peer got an HTTP 4xx/431); the summary's close kind misdescribes the failure layer. Best-effort writes here are fine; the mislabeled summary is not. Confidence: MEDIUM — Verification: read.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Limits::documented | 88-93 | REAL | — | 1 MiB/1 MiB pinned by test L961-968 |
| Default for Limits | 96-100 | REAL | — | delegates |
| AuthLimits::documented | 116-121 | REAL | — | 5/60 s pinned by test |
| Default for AuthLimits | 124-127 | REAL | — | delegates |
| Timeouts::documented | 145-150 | REAL | — | 60 s read / 10 s write |
| Default for Timeouts | 153-156 | REAL | — | delegates |
| FailureBook::is_locked | 170-175 | REAL | — | expiry check correct |
| FailureBook::note_failure | 179-187 | REAL | S4 | return value dead (F-005); persistent count re-locks after expiry (tested L953-958) |
| ServerConfig::strict | 228-239 | REAL | — | deny-all defaults |
| `<F as Dispatch>::dispatch` | 252-259 | REAL | — | closure adapter, used by tests + run.rs UnitDispatch |
| ControlServer::bind | 316-397 | REAL | — | real TcpListener; loopback allowlist + resolved-addr re-verify (L344, L381); tests L904-933 executed |
| ControlServer::local_addr | 403-410 | REAL | — | passthrough |
| ControlServer::token | 414-416 | REAL | — | sanctioned reader; app prints to stderr (main.rs:242 — doc 05 §5.4 says stdout, app-level nit) |
| ControlServer::entropy_source | 420-422 | REAL | — | app warns on WeakFallback (main.rs:245-249) |
| ControlServer::is_locked_out | 425-427 | REAL | — | |
| ControlServer::serve_one | 437-473 | REAL | S3 | serial accept; per-op timeouts (F-003) |
| Session::new | 495-512 | REAL | — | clones policy/engine_info per session (fine, once per connection) |
| Session::run | 515-608 | REAL | — | upgrade → frame loop; ping/pong/close per RFC 6455; binary-frame rejection survives (tested) |
| Session::handle_text | 611-677 | REAL | S2/S3 | raw-value shape validation before typed parse (good); Subscribe/Unsubscribe inert (F-001/F-002) |
| Session::handle_hello | 680-758 | REAL | — | protocol/version/token/lockout ordering correct; lockout checked BEFORE compare (L716) so the lockout cannot be probed |
| Session::handle_request | 763-810 | REAL | S3 | monotonic anti-replay correct (L772-783); Internal-vs-Denied mislabel (F-004) |
| Session::recoverable | 813-825 | REAL | — | connection survives typed errors (tested) |
| Session::error_frame | 828-833 | REAL | — | redact_name applied |
| Session::send_json | 836-842 | REAL | — | |
| Session::fail_session | 846-851 | REAL | S4 | unreachable! (F-007) |
| Session::fail_with | 854-867 | REAL | — | code mapping total |
| Session::close_with | 870-875 | REAL | — | |
| err_reason | 879-891 | REAL | — | static strings |
| test non_loopback_binds_are_refused_at_construction | 904-915 | HONEST | — | would fail if allowlist widened |
| test loopback_binds_succeed_on_both_families | 917-933 | HONEST | — | real binds on 127.0.0.1/::1/localhost |
| test failure_book_engages_the_documented_lockout | 936-959 | HONEST | — | real 50 ms lockout expiry |
| test documented_limits_are_the_doc_05_numbers | 961-968 | HONEST | — | pins constants |
| test_engine_info (helper) | 971-986 | REAL | — | real catalog checksum |

## 5. Test honesty (if tests exist)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| non_loopback_binds_are_refused_at_construction | YES | enumerates 6 hostile hosts incl. "" and "[::]" |
| loopback_binds_succeed_on_both_families | YES | real OS binds; asserts resolved loopback |
| failure_book_engages_the_documented_lockout | YES | counts failures, sleeps past expiry, re-lock |
| documented_limits_are_the_doc_05_numbers | YES | pins 1 MiB/1 MiB/5/60 s |
| (conformance suite, other file) 25 cases | YES | executed by me over real sockets: 25/25 pass |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "loopback-only; deny-by-default is structural" | docs/05-command-layer.md §5.4 (L541) | REAL | bind L316-397 + tests executed |
| "Every frame: max 1 MiB" | doc 05 §5.4 (L542) | REAL | read_frame cap (frame.rs:258-260) + conformance `oversized_length_claim_closes_1009_before_allocation` passed |
| "5 failures impose a 60 s lockout" | doc 05 §5.4 (L541) | REAL | FailureBook + conformance `five_failures_lock_out_even_the_right_token` passed |
| "per-connection concurrency cap (8) and rate limit (100/s)" | doc 05 §5.4 (L542) `[PROPOSAL]` | NOT IMPLEMENTED (honestly deferred) | absent from server.rs; lib.rs "Honest scope" records the gap |
| "Origin check … rejects any origin not on the allowlist" | doc 05 §5.4 (L543) `[PROPOSAL]` | NOT IMPLEMENTED (honestly deferred) | no Origin handling in server.rs; DNS-rebind Host check exists in http.rs (conformance `dns_rebound_host_names_never_upgrade` passed) |
| subscribe/unsubscribe topic filtering | doc 05 §4.4 (L450) `[PROPOSAL]` | STUB | frames accepted, nothing filtered/sent (F-002) |
| "single-connection by design (D-1)" | lib.rs + server.rs L30-33 | REAL | serve_one is the only accept path; no threads anywhere (rg: no `thread::spawn` in crate src) |

## 7. Dead / duplicated / unreachable code
- `Session::subscriptions` — written-only state (F-002): the only reader is `contains` for dedup. Dead-in-effect.
- `note_failure` return value — dead (F-005).
- Whole crate vs shipped product: `rg control-protocol --glob **/Cargo.toml` -> only apps/control-server; release lane (release.yml:193) does not build it; product.toml:167 `control-server = false` — the crate is real but UNREACHABLE from shipped entry points today (Phase 5+ [PROPOSAL]).

## 8. Dependencies & flags
No feature flags in crates/control-protocol/Cargo.toml (deps: commands, serde, serde_json only; std-only transport — verified no tungstenite/sha1/base64 crates). No cfg-gated code in server.rs beyond `#[cfg(test)]`. No unsafe/unwrap/expect in server.rs (rg verified: only the documented `unreachable!` at L849).

## 9. Open questions for the next agent
- client.rs (file 4): does the client actually send Subscribe, and does any doc promise events to control clients? If the docs' UI-remote-control story depends on events, F-002 promotes.
- apps/control-server is NOT in my assignment: its dispatch (2 real commands + NotReady) deserves its own audit row; `e2e_full_session_through_the_app_stack` passes because it only exercises engine.info/list-commands.
- http.rs Host-header validation (DNS rebind) is conformance-tested but the Origin gap means any local browser page can reach the upgrade path (token still required) — confirm with 8.A2 deferral.
