# AUDIT: crates/control-protocol/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W02-A01 |
| Date | 2026-11-23 |
| Lines / functions | 81 / 0 (pure facade: 11 `pub mod`, 4 `pub use` blocks; no functions) |
| File verdict | REAL (every substantive doc claim verified against sibling code and executed tests) |
| Reachable from shipped entry point? | NO — sole consumer is apps/control-server (`lumen-control-server` bin), product-gated OFF (product.toml:167), not in the release lane (`-p lumen-cli -p desktop`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p control-protocol` (exit 0; 47 + 25 passed, 0 failed, 0 ignored); rg sweeps cited below; baseline: PACKAGE control-protocol section of audit/runs/baseline/cargo-test-workspace.txt |

## 1. One-paragraph verdict
This lib.rs is a doc-heavy facade with zero executable code, and — unusually for this repo — its
documentation tells the truth. I verified all ten substantive claims against the sibling
modules: the loopback bind gate is real structural code (literal allowlist + post-bind
re-verification), the token/lockout numbers (256-bit, 4401, 5×60 s) exist as code constants,
the deny-by-default policy is enforced, the handshake reuses `commands::WireRequest/
WireResponse` by import rather than reimplementation, SHA-1/base64 are pinned by external RFC
vectors, and the no-`unsafe`/`unwrap`/`expect`/`panic` claim survives a strict rg sweep of
production code. I ran the crate's tests: 72 pass, 0 fail. The two real problems: the entire
4522-line crate is unreachable from any shipped build (only `apps/control-server` consumes it
and the product config ships that feature off), and one doc pointer ("see [`server::Limits`]")
points at byte caps where the rate-limit/Origin gap record actually lives elsewhere.

## 2. What actually works (verified)
— `cargo test -p control-protocol` -> 47 passed (unit) + 25 passed (conformance), 0 failed,
0.06s/0.16s. Baseline agrees (audit/runs/baseline/cargo-test-workspace.txt, PACKAGE
control-protocol, exit 0).
— Loopback gate: crates/control-protocol/src/server.rs:316-338 — literal match on
`"127.0.0.1" | "::1" | "[::1]" | "localhost"`, else `ProtocolError::NonLoopbackBind`; after
resolve, every candidate is re-checked `socket.ip().is_loopback()` before `TcpListener::bind`.
Structural, not advisory — the doc claim (lib.rs:11-13) is REAL.
— Auth policy numbers: server.rs:115-120 `max_failures: 5, lockout: Duration::from_secs(60)`;
close 4401 wired through `error.rs`/`fail_with`; failure book server-global (server.rs:102-104,
164-180). lib.rs:20-23 claim REAL.
— `constant_time_eq` (token.rs:18-32): length-public, accumulate-then-decide — the standard
construction, no early exit. REAL.
— Policy: policy.rs:58 `is_sensitive(CommandId)`, `sensitive_allow` deny-by-default list,
`deny_unknown_fields` on the config parse. lib.rs:24-27 claim REAL.
— Command-layer reuse: protocol.rs:29-30 `use commands::WireRequest; use commands::WireResponse;`
— no re-implementation. lib.rs:17-19 claim REAL.
— SHA-1/base64 in-tree: sha1.rs:160-188 external vectors (RFC 3174 §7.3, NIST million-`a`,
RFC 6455 §1.3 worked handshake); files are 210/220 lines with the "~100 lines each" (lib.rs:39)
being the implementation proper after doc comments and test modules. REAL.
— Malformed input survives: server.rs:618-671 typed `serde_json::from_str` rejection path,
784 "the connection survives", 827-853 typed `protocol-error` + close. lib.rs:28-31 claim REAL.
— Serial serving: server.rs:429-441 "Serve exactly one connection to completion" (accept →
upgrade → run). lib.rs:50-51 claim REAL.
— No-panic claim: rg for `todo!|unimplemented!|unreachable!|\.unwrap\(|\.expect\(|panic!|unsafe`
over src/ → all hits are either doc comments (token.rs:97) or inside `#[cfg(test)]` modules
(cfg(test) offsets: server 899, http 343, frame 571, protocol 288, token 109) — except one
PRODUCTION `unreachable!` at server.rs:849 guarding a same-function invariant (`fail_with`
always returns `Flow::End`). The lib.rs claim lists unsafe/unwrap/expect/panic only, so it is
accurate as written; the 849 invariant assertion is not input-reachable.

## 3. Findings
F-001 [S3]
Where: crates/control-protocol/src/lib.rs:3-5 (whole-crate reachability), cross-ref
product.toml:167, apps/control-server/Cargo.toml
Code (product.toml:166-167):
```
wasm-extensions = false         # Phase 7
control-server = false          # Phase 5+ [PROPOSAL]
```
What is wrong: Not a code defect — a reachability fact the facade doc omits. The only consumer
of this crate is apps/control-server (rg over all Cargo.tomls: one hit). The product config
ships `control-server = false`, and the release lane builds only `-p lumen-cli -p desktop`
(audit/runs/baseline/cargo-build-release-lumen-cli.txt). So all 4522 lines of
security-sensitive surface (a WebSocket server, SHA-1, token auth, a policy engine) plus 72
passing tests are maintained for a binary that no shipped lane enables. Why it matters: the
lib.rs presents itself as "the remote-control protocol behind apps/control-server" with no
statement that the feature is product-off; a reader auditing "what ships" would mis-prioritize.
What correct looks like: one line in the crate doc — "product-gated: control-server = false
until Phase 5+".
Confidence: HIGH — Verification: read (Cargo graph, product.toml, baseline) + ran tests.

F-002 [S4]
Where: crates/control-protocol/src/lib.rs:47-49 vs server.rs:74-92, server.rs:30-32
Code (lib.rs:47-49):
```
//! `[PROPOSAL]`-numbered rate limits of doc 05 §5.4 (100 req/s burst
//! 200, 8 in-flight requests) and the Origin allowlist are recorded
//! gaps, not silent omissions — see [`server::Limits`].
```
What is wrong: `server::Limits` documents only byte caps (`max_frame_bytes` /
`max_message_bytes`, 1 MiB defaults). The rate-limit/in-flight/Origin gaps are recorded in the
server module docs (server.rs:30-32 "Concurrency/rate caps and the Origin allowlist are
recorded [gaps]"), not in `Limits`. The pointer sends an auditor to the wrong struct. The gaps
themselves are honestly declared — this is a doc-pointer defect only.
Confidence: HIGH — Verification: read both sites.

## 4. Per-function table
Not required (81 LOC, no functions — facade of `pub mod`/`pub use` only). Module inventory:
base64, client, error, frame, http, policy, protocol, server, sha1, token (all exist with
non-trivial implementations; sizes 153-987 lines each).

## 5. Test honesty
No tests in this file. The crate's tests (ran, 72 pass) live in the sibling modules and
tests/conformance.rs; conformance.rs drives the real server through the in-tree client
(`TestClient`) — the client claim (lib.rs:32-34 "used by this crate's and the app's tests") is
REAL: apps/control-server/tests/e2e.rs:19 `use control_protocol::client::TestClient;`.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| bind refuses non-loopback with typed error | lib.rs:11-13 | REAL | server.rs:316-338 code; conformance tests pass |
| lumen-control/1 handshake, mismatch = typed error + close, never UB | lib.rs:14-17 | REAL | protocol.rs:37-44, 216; error codes; tests |
| requests/responses are commands::WireRequest/WireResponse, never re-implemented | lib.rs:17-19 | REAL | protocol.rs:29-30 imports |
| 256-bit token, constant-time compare, 4401, 5 failures ⇒ 60 s lockout | lib.rs:20-23 | REAL | token.rs:44-56, server.rs:115-120,164-180 |
| deny-by-default sensitive ops unless config grants | lib.rs:24-27 | REAL | policy.rs:58,109-164 |
| malformed input = typed protocol-error, connection survives | lib.rs:28-31 | REAL | server.rs:618-671,784,827-853; tests |
| client used by this crate's and the app's tests | lib.rs:32-34 | REAL | conformance.rs:29-33, apps/control-server/tests/e2e.rs:19-23 |
| SHA-1/base64 in-crate ~100 lines, pinned by external vectors | lib.rs:38-42 | REAL | sha1.rs:160-188 (RFC 3174 §7.3, NIST, RFC 6455 §1.3) |
| rate limits + Origin allowlist are recorded gaps — "see server::Limits" | lib.rs:46-49 | PARTIAL (pointer wrong, gaps real) | F-002 |
| server serves connections serially (one at a time) | lib.rs:50-51 | REAL | server.rs:429-441 |
| no unsafe/unwrap/expect/panic | lib.rs:57-58 | REAL | rg sweep: only test-module/comment hits; one production invariant unreachable! at server.rs:849 (not input-reachable, not in the claimed list) |
| (implicit) the crate ships | lib.rs:1-5 | FALSE-by-omission | F-001: product.toml:167 control-server = false; release lane excludes it |

## 7. Dead / duplicated / unreachable code
The crate as a whole is production-unreachable in the shipped lanes (F-001). No internal dead
code found in this facade. No duplication: wire types are imported from `commands`, not copied.

## 8. Dependencies & flags
Cargo.toml: deps = commands (path), serde 1.0.229 (derive), serde_json 1.0.154 — matches the
"one serde path" claim; no tungstenite/sha1/base64 crates (ADR-011 in-tree, verified by the
dependency list). No features defined. `[lints] workspace = true`.

## 9. Open questions for the next agent
- token.rs entropy: `/dev/urandom` is Unix-only; on Windows every generated token takes the
  WEAK fallback (documented `[VERIFY]` at token.rs:40-47, typed `EntropySource::WeakFallback`).
  When control-server actually ships on Windows this becomes a real weakness — re-audit then.
- apps/control-server dispatch.rs: does it forward to a real engine host or a stub? (Next wave
  should audit apps/control-server/src/{dispatch,run}.rs for "handlers forwarding to nothing".)
- The 100 req/s / 8 in-flight caps of doc 05 §5.4 are still unimplemented anywhere — confirm
  they stay on the gap ledger when Phase 5 lands.
